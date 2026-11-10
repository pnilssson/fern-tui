use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn run(dir: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=tester",
            "-c",
            "user.email=tester@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed").into());
    }

    Ok(())
}

pub fn repo() -> Result<TempDir> {
    let dir = tempfile::tempdir()?;
    run(dir.path(), &["init", "-q", "-b", "main"])?;
    write(dir.path(), "src/main.rs", "fn main() {}\n")?;
    write(dir.path(), "docs/guide.md", "# Guide\n")?;
    run(dir.path(), &["add", "."])?;
    run(dir.path(), &["commit", "-q", "-m", "init"])?;

    Ok(dir)
}

pub fn write(root: &Path, path: &str, text: &str) -> Result<()> {
    let file = root.join(path);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(file, text)?;

    Ok(())
}
