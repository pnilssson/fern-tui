use std::io::ErrorKind;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::Error;

pub fn git(dir: &Path, args: &[&str]) -> Result<Vec<u8>, Error> {
    let output = Command::new("git")
        .current_dir(dir)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["-c", "core.quotepath=false", "-c", "color.ui=never"])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => Error::Missing,
            _ => Error::Git(error.to_string()),
        })?;
    if !output.status.success() {
        return Err(failure(&output.stderr));
    }

    Ok(output.stdout)
}

fn failure(stderr: &[u8]) -> Error {
    let text = String::from_utf8_lossy(stderr);
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("git failed");
    let line = line
        .strip_prefix("fatal: ")
        .or_else(|| line.strip_prefix("error: "))
        .unwrap_or(line);

    Error::Git(line.to_owned())
}
