use std::cmp::Reverse;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Matcher, Utf32Str};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::nav::Step;
use super::{App, Focus, Mode};
use fern_git::Change;

use crate::entries::Entry;

fn rank<T: Clone>(
    matcher: &mut Matcher,
    items: &[T],
    text: impl Fn(&T) -> &str,
    query: &str,
) -> Vec<(T, Vec<u32>)> {
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut buffer = Vec::new();
    let mut found = Vec::new();
    for item in items {
        let mut hits = Vec::new();
        let haystack = Utf32Str::new(text(item), &mut buffer);
        let Some(score) = pattern.indices(haystack, matcher, &mut hits) else {
            continue;
        };

        hits.sort_unstable();
        hits.dedup();
        found.push((score, item.clone(), hits));
    }
    found.sort_by_key(|(score, ..)| Reverse(*score));

    found
        .into_iter()
        .map(|(_, item, hits)| (item, hits))
        .collect()
}

fn text(filter: Option<&String>) -> Option<String> {
    filter.filter(|query| !query.is_empty()).cloned()
}

fn delete_word(query: &mut String) {
    query.truncate(query.trim_end().len());
    query.truncate(query.rfind(' ').map_or(0, |space| space + 1));
}

impl App {
    fn slot(&mut self) -> &mut Option<String> {
        match self.mode {
            Mode::Files => &mut self.filter,
            Mode::Changes => &mut self.changes_filter,
        }
    }

    pub(super) fn filtering(&self) -> bool {
        match self.mode {
            Mode::Files => self.filter.is_some(),
            Mode::Changes => self.changes_filter.is_some(),
        }
    }

    pub(super) fn visible(&mut self) -> Vec<Entry> {
        let Some(query) = text(self.filter.as_ref()) else {
            return self.listing.clone();
        };

        rank(
            &mut self.matcher,
            &self.listing,
            |entry| &entry.name,
            &query,
        )
        .into_iter()
        .map(|(entry, hits)| Entry { hits, ..entry })
        .collect()
    }

    pub(super) fn visible_changes(&mut self) -> Vec<(Change, Vec<u32>)> {
        let all = &self.status.changes;
        let Some(query) = text(self.changes_filter.as_ref()) else {
            return all
                .iter()
                .map(|change| (change.clone(), Vec::new()))
                .collect();
        };

        rank(&mut self.matcher, all, |change| &change.path, &query)
    }

    fn refilter(&mut self, wanted: Option<String>) {
        match self.mode {
            Mode::Files => self.apply(wanted),
            Mode::Changes => self.apply_changes(wanted),
        }
    }

    fn selected_name(&self) -> Option<String> {
        match self.mode {
            Mode::Files => self.selected_entry().map(|entry| entry.name.clone()),
            Mode::Changes => self.selected_change().map(|change| change.path.clone()),
        }
    }

    pub(super) fn start_filter(&mut self) {
        self.focus = Focus::List;
        self.slot().get_or_insert_default();
    }

    pub(super) fn clear_filter(&mut self) {
        let name = self.selected_name();
        *self.slot() = None;
        self.refilter(name);
    }

    pub(super) fn filter_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => self.clear_filter(),
            KeyCode::Right => {
                self.clear_filter();
                self.open();
            }
            KeyCode::Enter => {
                self.clear_filter();
                if self.mode == Mode::Changes {
                    self.reveal();
                } else {
                    self.open();
                }
            }
            KeyCode::Up => self.move_sel(Step::Up(1)),
            KeyCode::Down => self.move_sel(Step::Down(1)),
            KeyCode::Char('p' | 'k') if ctrl => self.move_sel(Step::Up(1)),
            KeyCode::Char('n' | 'j') if ctrl => self.move_sel(Step::Down(1)),
            KeyCode::Char('u') if ctrl => self.edit(String::clear),
            KeyCode::Char('w') if ctrl => self.edit(delete_word),
            KeyCode::Backspace => {
                if self.slot().as_ref().is_some_and(String::is_empty) {
                    self.clear_filter();
                } else {
                    self.edit(|query| {
                        query.pop();
                    });
                }
            }
            KeyCode::Char(ch) if !ctrl => self.edit(|query| query.push(ch)),
            _ => {}
        }
    }

    fn edit(&mut self, change: impl FnOnce(&mut String)) {
        let Some(query) = self.slot() else {
            return;
        };

        change(query);
        self.refilter(None);
    }
}

#[cfg(test)]
mod tests {
    use nucleo_matcher::{Config, Matcher};

    use super::rank;

    fn names<'a>(found: &[(&'a str, Vec<u32>)]) -> Vec<&'a str> {
        found.iter().map(|(name, _)| *name).collect()
    }

    fn run<'a>(all: &[&'a str], query: &str) -> Vec<(&'a str, Vec<u32>)> {
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());

        rank(&mut matcher, all, |name| name, query)
    }

    #[test]
    fn matches_fuzzily_and_ranks_the_closest_first() {
        let found = run(
            &["README.md", "src_main.rs", "main.rs", "Cargo.toml"],
            "mnrs",
        );

        assert_eq!(names(&found), ["main.rs", "src_main.rs"]);
        assert_eq!(found[0].1, [0, 3, 5, 6]);
    }

    #[test]
    fn lowercase_ignores_case_and_uppercase_does_not() {
        let all = ["readme.md", "README.md"];

        assert_eq!(run(&all, "readme").len(), 2);
        assert_eq!(names(&run(&all, "README")), ["README.md"]);
    }

    #[test]
    fn matches_across_the_folders_of_a_path() {
        let found = run(&["src/app/main.rs", "docs/main.md"], "appmain");

        assert_eq!(names(&found), ["src/app/main.rs"]);
    }

    #[test]
    fn right_arrow_ends_the_filter_and_enters_the_folder() {
        use std::sync::mpsc;

        use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        use crate::app::App;

        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        std::fs::create_dir_all(base.join("docs")).unwrap();
        std::fs::create_dir_all(base.join("src")).unwrap();
        let (requests, _worker) = mpsc::channel();
        let (messages, _inbox) = mpsc::channel();
        let mut app = App::new(base.clone(), base.clone(), false, requests, messages);

        for code in [KeyCode::Char('f'), KeyCode::Char('s'), KeyCode::Char('r')] {
            app.key(KeyEvent::new(code, KeyModifiers::NONE));
        }
        assert_eq!(app.entries.len(), 1);
        app.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

        assert_eq!(app.cwd, base.join("src"));
        assert!(app.filter.is_none());
    }
}
