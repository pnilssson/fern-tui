use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::Msg;

const URL: &str = "https://api.github.com/repos/pnilssson/fern-tui/releases/latest";
const MAX_AGE: u64 = 24 * 60 * 60;
const RECHECK: Duration = Duration::from_hours(1);

pub fn spawn(messages: Sender<Msg>) {
    if std::env::var_os("FERN_NO_UPDATE_CHECK").is_some() {
        return;
    }

    thread::spawn(move || {
        loop {
            if let Some(latest) = latest()
                && newer(env!("CARGO_PKG_VERSION"), &latest)
            {
                let _ = messages.send(Msg::Update(latest));

                return;
            }

            thread::sleep(RECHECK);
        }
    });
}

fn latest() -> Option<String> {
    let cache = cache_file();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    if let Some((checked, version)) = cache.as_deref().and_then(read)
        && now.saturating_sub(checked) < MAX_AGE
    {
        return Some(version);
    }

    let version = fetch()?;
    if let Some(path) = &cache {
        write(path, now, &version);
    }

    Some(version)
}

fn cache_file() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
    } else if let Some(dir) = std::env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(dir)
    } else {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        if cfg!(target_os = "macos") {
            home.join("Library/Caches")
        } else {
            home.join(".cache")
        }
    };

    Some(base.join("fern").join("latest-release"))
}

fn read(path: &Path) -> Option<(u64, String)> {
    let text = std::fs::read_to_string(path).ok()?;
    let (checked, version) = text.split_once('\n')?;

    Some((checked.parse().ok()?, version.trim().to_owned()))
}

fn write(path: &Path, now: u64, version: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, format!("{now}\n{version}\n"));
}

fn fetch() -> Option<String> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "5",
            "-H",
            "Accept: application/vnd.github+json",
            URL,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    tag(&String::from_utf8_lossy(&output.stdout)).map(str::to_owned)
}

fn tag(json: &str) -> Option<&str> {
    let rest = json.split_once("\"tag_name\"")?.1;
    let rest = rest
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .strip_prefix('"')?;
    let name = rest.split_once('"')?.0;

    Some(name.strip_prefix('v').unwrap_or(name))
}

fn parse(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.').map(|part| part.parse().ok());

    Some((parts.next()??, parts.next()??, parts.next()??))
}

fn newer(current: &str, latest: &str) -> bool {
    match (parse(current), parse(latest)) {
        (Some(current), Some(latest)) => latest > current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{newer, tag};

    #[test]
    fn reads_the_tag_from_a_release() {
        let json = r#"{"url":"x","tag_name": "v0.1.7","name":"v0.1.7"}"#;

        assert_eq!(tag(json), Some("0.1.7"));
        assert_eq!(tag("{}"), None);
    }

    #[test]
    fn only_a_higher_version_is_newer() {
        assert!(newer("0.1.0", "0.1.1"));
        assert!(newer("0.9.9", "0.10.0"));
        assert!(!newer("0.1.1", "0.1.1"));
        assert!(!newer("0.2.0", "0.1.9"));
        assert!(!newer("0.1.0", "0.2.0-rc.1"));
    }
}
