use std::collections::HashMap;
use std::path::Path;

use crate::Error;
use crate::run::git;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub path: String,
    pub orig: Option<String>,
    pub x: char,
    pub y: char,
    pub adds: u32,
    pub dels: u32,
}

impl Change {
    pub fn untracked(&self) -> bool {
        self.x == '?'
    }

    pub fn conflict(&self) -> bool {
        self.x == 'U'
            || self.y == 'U'
            || (self.x == 'A' && self.y == 'A')
            || (self.x == 'D' && self.y == 'D')
    }
}

#[derive(Clone, Debug, Default)]
pub struct Status {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub changes: Vec<Change>,
}

pub fn status(root: &Path) -> Result<Status, Error> {
    let output = git(
        root,
        &[
            "status",
            "--porcelain=v2",
            "-z",
            "--branch",
            "--untracked-files=all",
        ],
    )?;

    let mut status = parse(&String::from_utf8_lossy(&output));
    let counts = git(root, &["diff", "HEAD", "--numstat", "-z", "--no-renames"])
        .map(|output| numstat(&String::from_utf8_lossy(&output)))
        .unwrap_or_default();
    for change in &mut status.changes {
        if let Some(&(adds, dels)) = counts.get(&change.path) {
            change.adds = adds;
            change.dels = dels;
        }
    }

    Ok(status)
}

fn numstat(text: &str) -> HashMap<String, (u32, u32)> {
    text.split('\0')
        .filter_map(|record| {
            let mut parts = record.splitn(3, '\t');
            let adds = parts.next()?.parse().unwrap_or(0);
            let dels = parts.next()?.parse().unwrap_or(0);

            Some((parts.next()?.to_owned(), (adds, dels)))
        })
        .collect()
}

fn parse(text: &str) -> Status {
    let mut status = Status::default();
    let mut records = text.split('\0');
    while let Some(record) = records.next() {
        if let Some(header) = record.strip_prefix("# ") {
            header_line(&mut status, header);
        } else if let Some(rest) = record.strip_prefix("1 ") {
            push(&mut status, rest, 8, None);
        } else if let Some(rest) = record.strip_prefix("2 ") {
            push(&mut status, rest, 9, records.next());
        } else if let Some(rest) = record.strip_prefix("u ") {
            push(&mut status, rest, 10, None);
        } else if let Some(path) = record.strip_prefix("? ") {
            status.changes.push(Change {
                path: path.to_owned(),
                orig: None,
                x: '?',
                y: '?',
                adds: 0,
                dels: 0,
            });
        }
    }
    status
        .changes
        .sort_by_cached_key(|change| change.path.to_lowercase());

    status
}

fn header_line(status: &mut Status, header: &str) {
    if let Some(branch) = header.strip_prefix("branch.head ") {
        branch.clone_into(&mut status.branch);
    } else if let Some(counts) = header.strip_prefix("branch.ab ") {
        let mut parts = counts.split(' ');
        status.ahead = count(parts.next(), '+');
        status.behind = count(parts.next(), '-');
    }
}

fn count(part: Option<&str>, sign: char) -> u32 {
    part.and_then(|part| part.strip_prefix(sign))
        .and_then(|part| part.parse().ok())
        .unwrap_or(0)
}

fn push(status: &mut Status, rest: &str, count: usize, orig: Option<&str>) {
    let fields: Vec<&str> = rest.splitn(count, ' ').collect();
    let (Some(code), Some(path)) = (fields.first(), fields.get(count - 1)) else {
        return;
    };
    let mut chars = code.chars().map(|c| if c == '.' { ' ' } else { c });
    let x = chars.next().unwrap_or(' ');
    let y = chars.next().unwrap_or(' ');
    status.changes.push(Change {
        path: (*path).to_owned(),
        orig: orig.map(str::to_owned),
        x,
        y,
        adds: 0,
        dels: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_branch_changes_renames_and_untracked_files() {
        let text = "# branch.oid abc\0# branch.head main\0# branch.ab +2 -1\0\
        1 .M N... 100644 100644 100644 a b src/main.rs\0\
        1 A. N... 000000 100644 100644 a b notes file.txt\0\
        2 R. N... 100644 100644 100644 a b R100 docs/new.md\0docs/old.md\0\
        u UU N... 100644 100644 100644 100644 a b c src/lib.rs\0\
        ? scratch.txt\0";

        let status = parse(text);

        assert_eq!(
            (status.branch.as_str(), status.ahead, status.behind),
            ("main", 2, 1)
        );
        let paths: Vec<&str> = status
            .changes
            .iter()
            .map(|change| change.path.as_str())
            .collect();
        assert_eq!(
            paths,
            [
                "docs/new.md",
                "notes file.txt",
                "scratch.txt",
                "src/lib.rs",
                "src/main.rs"
            ]
        );
        assert_eq!(status.changes[0].orig.as_deref(), Some("docs/old.md"));
        assert_eq!((status.changes[4].x, status.changes[4].y), (' ', 'M'));
        assert!(
            status.changes[1].x == 'A'
                && status.changes[2].untracked()
                && status.changes[3].conflict()
        );
    }

    #[test]
    fn reads_line_counts_from_real_numstat_output() {
        let text = "-\t-\ta.bin\x001\t1\tnotes file.txt\x003\t1\tsrc/main.rs\0";

        let counts = numstat(text);

        assert_eq!(counts["a.bin"], (0, 0));
        assert_eq!(counts["notes file.txt"], (1, 1));
        assert_eq!(counts["src/main.rs"], (3, 1));
    }
}
