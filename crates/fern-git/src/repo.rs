use std::path::{Path, PathBuf};

use crate::run::git;

pub struct Repo {
    pub root: PathBuf,
    pub cwd: PathBuf,
}

pub fn locate(start: &Path) -> Option<Repo> {
    let output = git(start, &["rev-parse", "--show-toplevel", "--show-prefix"]).ok()?;
    let text = String::from_utf8_lossy(&output);
    let mut lines = text.lines();
    let root = PathBuf::from(lines.next()?);
    let cwd = root.join(lines.next().unwrap_or_default());

    Some(Repo { root, cwd })
}
