use super::{Diff, LIMIT, Row};

const META: [&str; 4] = ["Binary", "rename ", "old mode", "new mode"];

pub fn parse(text: &str) -> Diff {
    let mut rows = Vec::new();
    let (mut old, mut new) = (0, 0);
    let mut in_hunk = false;
    let mut truncated = false;
    for line in text.lines() {
        if rows.len() >= LIMIT {
            truncated = true;
            break;
        }

        if line.starts_with("diff --git ") {
            in_hunk = false;
        } else if line.starts_with("@@") {
            in_hunk = true;
            (old, new) = starts(line);
            rows.push(Row::Hunk(line.to_owned()));
        } else if !in_hunk {
            if META.iter().any(|prefix| line.starts_with(prefix)) {
                rows.push(Row::Meta(line.to_owned()));
            }
        } else {
            let text = line.get(1..).unwrap_or_default().to_owned();
            match line.as_bytes().first() {
                Some(b'+') => {
                    rows.push(Row::Add(new, text));
                    new += 1;
                }
                Some(b'-') => {
                    rows.push(Row::Del(old, text));
                    old += 1;
                }
                Some(b'\\') => rows.push(Row::Meta("No newline at end of file".to_owned())),
                _ => {
                    rows.push(Row::Context(old, new, text));
                    old += 1;
                    new += 1;
                }
            }
        }
    }

    Diff::new(rows, truncated)
}

fn starts(header: &str) -> (u32, u32) {
    let mut parts = header.split(' ').skip(1);
    let old = number(parts.next(), '-');
    let new = number(parts.next(), '+');

    (old, new)
}

fn number(part: Option<&str>, sign: char) -> u32 {
    part.and_then(|part| part.strip_prefix(sign))
        .and_then(|part| part.split(',').next())
        .and_then(|part| part.parse().ok())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_rows_from_a_real_diff() {
        let diff = parse(include_str!("fixtures/working.diff"));

        assert_eq!((diff.adds, diff.dels, diff.hunks.clone()), (3, 1, vec![0]));
        assert_eq!(diff.rows[1], Row::Context(1, 1, "fn main() {".to_owned()));
        assert_eq!(
            diff.rows[3],
            Row::Del(3, "    println!(\"{name}\");".to_owned())
        );
        assert_eq!(
            diff.rows[4],
            Row::Add(3, "    println!(\"hello {name}\");".to_owned())
        );
        assert_eq!(diff.rows[6], Row::Add(5, "    ++ a marker".to_owned()));
    }

    #[test]
    fn removed_and_added_lines_that_look_like_headers_stay_in_the_hunk() {
        let text =
            "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n--- removed line\n+++ added line\n";

        let diff = parse(text);

        assert_eq!((diff.adds, diff.dels), (1, 1));
    }

    #[test]
    fn keeps_binary_and_rename_notices() {
        let text = "diff --git a/a.bin b/a.bin\nindex 1..2 100644\nBinary files a/a.bin and b/a.bin differ\n";

        let diff = parse(text);

        assert_eq!(
            diff.rows,
            vec![Row::Meta(
                "Binary files a/a.bin and b/a.bin differ".to_owned()
            )]
        );
    }

    #[test]
    fn stops_at_the_row_limit() {
        let body = "+line\n".repeat(LIMIT + 10);
        let text = format!("diff --git a/f b/f\n@@ -0,0 +1,{} @@\n{body}", LIMIT + 10);

        let diff = parse(&text);

        assert!(diff.truncated);
        assert_eq!(diff.rows.len(), LIMIT);
    }
}
