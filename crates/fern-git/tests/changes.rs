mod common;

use common::{Result, repo, run, write};
use fern_git::{Context, Row, diff, locate, status};

#[test]
fn reports_staged_unstaged_and_untracked_changes() -> Result<()> {
    let dir = repo()?;
    write(dir.path(), "src/main.rs", "fn main() {\n    run();\n}\n")?;
    write(dir.path(), "docs/guide.md", "# Guide\nMore\n")?;
    run(dir.path(), &["add", "docs/guide.md"])?;
    write(dir.path(), "scratch/todo.txt", "later\n")?;

    let status = status(dir.path())?;

    assert_eq!(status.branch, "main");
    let counts: Vec<(u32, u32)> = status.changes.iter().map(|c| (c.adds, c.dels)).collect();
    assert_eq!(counts, [(1, 0), (0, 0), (3, 1)]);
    let codes: Vec<(&str, char, char)> = status
        .changes
        .iter()
        .map(|c| (c.path.as_str(), c.x, c.y))
        .collect();
    assert_eq!(
        codes,
        [
            ("docs/guide.md", 'M', ' '),
            ("scratch/todo.txt", '?', '?'),
            ("src/main.rs", ' ', 'M')
        ]
    );

    Ok(())
}

#[test]
fn diff_covers_staged_and_unstaged_edits_together() -> Result<()> {
    let dir = repo()?;
    write(dir.path(), "src/main.rs", "fn main() {\n    run();\n}\n")?;
    run(dir.path(), &["add", "."])?;
    write(
        dir.path(),
        "src/main.rs",
        "fn main() {\n    run();\n    stop();\n}\n",
    )?;
    let status = status(dir.path())?;

    let diff = diff(dir.path(), &status.changes[0], Context::Hunks)?;

    assert_eq!((diff.adds, diff.dels), (4, 1));
    assert!(diff.rows.contains(&Row::Add(3, "    stop();".to_owned())));

    Ok(())
}

#[test]
fn full_context_shows_every_line_and_jumps_to_changes() -> Result<()> {
    let dir = repo()?;
    let lines: Vec<String> = (1..=40).map(|number| format!("line {number}")).collect();
    write(dir.path(), "long.txt", &(lines.join("\n") + "\n"))?;
    run(dir.path(), &["add", "."])?;
    run(dir.path(), &["commit", "-q", "-m", "long"])?;
    let edited = lines
        .join("\n")
        .replace("line 3\n", "changed\n")
        .replace("line 38", "other");
    write(dir.path(), "long.txt", &(edited + "\n"))?;
    let status = status(dir.path())?;

    let diff = diff(dir.path(), &status.changes[0], Context::Full)?;

    assert!(!diff.rows.iter().any(|row| matches!(row, Row::Hunk(_))));
    assert!(diff.rows.contains(&Row::Context(1, 1, "line 1".to_owned())));
    assert!(
        diff.rows
            .contains(&Row::Context(40, 40, "line 40".to_owned()))
    );
    assert_eq!(diff.hunks.len(), 2);

    Ok(())
}

#[test]
fn follows_renames() -> Result<()> {
    let dir = repo()?;
    run(dir.path(), &["mv", "docs/guide.md", "docs/manual.md"])?;
    let status = status(dir.path())?;

    let change = &status.changes[0];
    let diff = diff(dir.path(), change, Context::Hunks)?;

    assert_eq!(
        (change.path.as_str(), change.orig.as_deref()),
        ("docs/manual.md", Some("docs/guide.md"))
    );
    assert!(
        diff.rows
            .iter()
            .any(|row| matches!(row, Row::Meta(text) if text.starts_with("rename ")))
    );

    Ok(())
}

#[test]
fn locates_the_root_and_the_folder_inside_it() -> Result<()> {
    let dir = repo()?;
    let root = std::fs::canonicalize(dir.path())?;

    let found = locate(&root.join("src")).ok_or("not found")?;

    assert_eq!(found.cwd, found.root.join("src"));
    assert_eq!(std::fs::canonicalize(&found.root)?, root);
    assert!(locate(&std::env::temp_dir().join("fern-not-a-repo-xyz")).is_none());

    Ok(())
}
