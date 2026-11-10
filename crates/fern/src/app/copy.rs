use std::path::{Path, PathBuf};

use super::{App, Mode};
use crate::clipboard;

#[derive(Clone, Copy)]
pub(super) enum Copy {
    Path,
    Dir,
    Name,
    Stem,
}

pub(super) const OPTIONS: [(char, Copy, &str); 4] = [
    ('c', Copy::Path, "Copy the file path"),
    ('d', Copy::Dir, "Copy the directory path"),
    ('f', Copy::Name, "Copy the filename"),
    ('n', Copy::Stem, "Copy the filename without extension"),
];

fn text(what: Copy, hovered: Option<&Path>, cwd: &Path) -> Option<String> {
    let path = match what {
        Copy::Path => hovered.unwrap_or(cwd),
        Copy::Dir => hovered.and_then(Path::parent).unwrap_or(cwd),
        Copy::Name => Path::new(hovered?.file_name()?),
        Copy::Stem => Path::new(hovered?.file_stem()?),
    };

    Some(path.to_string_lossy().into_owned())
}

impl App {
    fn hovered(&self) -> Option<PathBuf> {
        match self.mode {
            Mode::Files => self
                .selected_entry()
                .map(|entry| self.cwd.join(&entry.name)),
            Mode::Changes => self
                .selected_change()
                .map(|change| self.root.join(&change.path)),
        }
    }

    pub(super) fn copy(&mut self, what: Copy) {
        let hovered = self.hovered();
        let Some(text) = text(what, hovered.as_deref(), &self.cwd) else {
            self.say("Nothing to copy".to_owned());

            return;
        };

        self.put(&text, format!("Copied {text}"));
    }

    pub(super) fn put(&mut self, text: &str, done: String) {
        let result = if clipboard::copy(text) {
            done
        } else {
            "No clipboard tool found".to_owned()
        };
        self.say(result);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Copy, text};

    #[test]
    fn copies_the_hovered_item_in_each_form() {
        let hovered = Some(Path::new("/repos/app/src/main.rs"));
        let cwd = Path::new("/repos/app/src");
        let copied = |what| text(what, hovered, cwd);

        assert_eq!(
            copied(Copy::Path).as_deref(),
            Some("/repos/app/src/main.rs")
        );
        assert_eq!(copied(Copy::Dir).as_deref(), Some("/repos/app/src"));
        assert_eq!(copied(Copy::Name).as_deref(), Some("main.rs"));
        assert_eq!(copied(Copy::Stem).as_deref(), Some("main"));
    }

    #[test]
    fn an_empty_folder_copies_only_its_own_path() {
        let cwd = Path::new("/repos/app");

        assert_eq!(text(Copy::Path, None, cwd).as_deref(), Some("/repos/app"));
        assert_eq!(text(Copy::Dir, None, cwd).as_deref(), Some("/repos/app"));
        assert_eq!(text(Copy::Name, None, cwd), None);
    }
}
