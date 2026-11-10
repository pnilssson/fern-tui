use std::path::{Component, Path};
use std::sync::mpsc::Sender;
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebounceEventResult, Debouncer, new_debouncer};

use crate::Msg;

pub type Watcher = Debouncer<RecommendedWatcher>;

pub fn start(root: &Path, recursive: bool, messages: Sender<Msg>) -> Option<Watcher> {
    let mut debouncer = new_debouncer(
        Duration::from_millis(250),
        move |result: DebounceEventResult| {
            let Ok(events) = result else {
                return;
            };
            if events.iter().any(|event| relevant(&event.path)) {
                let _ = messages.send(Msg::Changed);
            }
        },
    )
    .ok()?;
    let mode = if recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    debouncer.watcher().watch(root, mode).ok()?;

    Some(debouncer)
}

fn relevant(path: &Path) -> bool {
    let parts: Vec<_> = path
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect();
    let Some(git) = parts.iter().position(|part| *part == ".git") else {
        return true;
    };

    parts.len() == git + 2 && matches!(parts[git + 1], "index" | "HEAD")
}

#[cfg(test)]
mod tests {
    use super::relevant;
    use std::path::Path;

    #[test]
    fn working_tree_files_are_relevant() {
        assert!(relevant(Path::new("/repos/app/src/main.rs")));
    }

    #[test]
    fn only_the_index_and_head_inside_git_are_relevant() {
        assert!(relevant(Path::new("/repos/app/.git/index")));
        assert!(relevant(Path::new("/repos/app/.git/HEAD")));
        assert!(!relevant(Path::new("/repos/app/.git/index.lock")));
        assert!(!relevant(Path::new("/repos/app/.git/objects/ab/cdef")));
    }
}
