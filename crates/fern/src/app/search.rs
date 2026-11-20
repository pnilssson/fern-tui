use fern_git::Row;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::App;
use super::filter::delete_word;
use crate::preview::Preview;
use crate::ui::clean;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub row: usize,
    pub from: usize,
    pub to: usize,
}

pub struct Search {
    pub query: String,
    pub hits: Vec<Hit>,
    pub current: usize,
    from: usize,
}

impl Search {
    pub fn row(&self, row: usize) -> &[Hit] {
        let start = self.hits.partition_point(|hit| hit.row < row);
        let end = self.hits.partition_point(|hit| hit.row <= row);

        &self.hits[start..end]
    }
}

fn fold(ch: char) -> char {
    ch.to_lowercase().next().unwrap_or(ch)
}

fn find(text: &str, needle: &[char], ignore_case: bool) -> Vec<(usize, usize)> {
    let hay: Vec<char> = clean(text)
        .chars()
        .map(|ch| if ignore_case { fold(ch) } else { ch })
        .collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at + needle.len() <= hay.len() {
        if hay[at..at + needle.len()] == *needle {
            found.push((at, at + needle.len()));
            at += needle.len();
        } else {
            at += 1;
        }
    }

    found
}

fn texts(preview: &Preview) -> Vec<&str> {
    match preview {
        Preview::Text { lines, .. } => lines.iter().map(String::as_str).collect(),
        Preview::Diff { diff, .. } => diff
            .rows
            .iter()
            .map(|row| match row {
                Row::Meta(text)
                | Row::Hunk(text)
                | Row::Context(_, _, text)
                | Row::Add(_, text)
                | Row::Del(_, text) => text.as_str(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn hits(preview: &Preview, query: &str) -> Vec<Hit> {
    if query.is_empty() {
        return Vec::new();
    }

    let ignore_case = !query.chars().any(char::is_uppercase);
    let needle: Vec<char> = query
        .chars()
        .map(|ch| if ignore_case { fold(ch) } else { ch })
        .collect();

    texts(preview)
        .into_iter()
        .enumerate()
        .flat_map(|(row, text)| {
            find(text, &needle, ignore_case)
                .into_iter()
                .map(move |(from, to)| Hit { row, from, to })
        })
        .collect()
}

impl App {
    pub(super) fn start_search(&mut self) {
        if !matches!(self.preview, Preview::Text { .. } | Preview::Diff { .. }) {
            return;
        }

        self.search = Some(Search {
            query: String::new(),
            hits: Vec::new(),
            current: 0,
            from: self.preview_scroll,
        });
    }

    pub(super) fn search_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Esc => self.search = None,
            KeyCode::Enter if shift => self.seek(false),
            KeyCode::Enter | KeyCode::Down => self.seek(true),
            KeyCode::Up => self.seek(false),
            KeyCode::Char('n') if ctrl => self.seek(true),
            KeyCode::Char('p') if ctrl => self.seek(false),
            KeyCode::Char('u') if ctrl => self.retype(String::clear),
            KeyCode::Char('w') if ctrl => self.retype(delete_word),
            KeyCode::Backspace => {
                if self.search.as_ref().is_some_and(|s| s.query.is_empty()) {
                    self.search = None;
                } else {
                    self.retype(|query| {
                        query.pop();
                    });
                }
            }
            KeyCode::Char(ch) if !ctrl => self.retype(|query| query.push(ch)),
            _ => {}
        }
    }

    pub(super) fn rescan(&mut self) {
        let Some(search) = &mut self.search else {
            return;
        };

        search.hits = hits(&self.preview, &search.query);
        search.current = search.current.min(search.hits.len().saturating_sub(1));
    }

    fn retype(&mut self, change: impl FnOnce(&mut String)) {
        let Some(search) = &mut self.search else {
            return;
        };

        change(&mut search.query);
        search.hits = hits(&self.preview, &search.query);
        search.current = search
            .hits
            .iter()
            .position(|hit| hit.row >= search.from)
            .unwrap_or(0);
        self.show_hit();
    }

    fn seek(&mut self, forward: bool) {
        let Some(search) = &mut self.search else {
            return;
        };
        let len = search.hits.len();
        if len == 0 {
            return;
        }

        search.current = if forward {
            (search.current + 1) % len
        } else {
            (search.current + len - 1) % len
        };
        self.show_hit();
    }

    fn show_hit(&mut self) {
        let Some(hit) = self
            .search
            .as_ref()
            .and_then(|search| search.hits.get(search.current))
        else {
            return;
        };

        let window = (self.preview_height * 2 / 3).max(1);
        if hit.row < self.preview_scroll || hit.row >= self.preview_scroll + window {
            self.preview_scroll = hit.row.saturating_sub(self.preview_height / 3);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::find;
    use crate::app::{App, Focus};
    use crate::preview::Preview;

    fn run(text: &str, query: &str) -> Vec<(usize, usize)> {
        let ignore_case = !query.chars().any(char::is_uppercase);
        let needle: Vec<char> = if ignore_case {
            query.to_lowercase().chars().collect()
        } else {
            query.chars().collect()
        };

        find(text, &needle, ignore_case)
    }

    #[test]
    fn lowercase_ignores_case_and_uppercase_does_not() {
        assert_eq!(run("Main main", "main"), [(0, 4), (5, 9)]);
        assert_eq!(run("Main main", "Main"), [(0, 4)]);
    }

    #[test]
    fn positions_count_characters_not_bytes() {
        assert_eq!(run("åäö main", "main"), [(4, 8)]);
    }

    #[test]
    fn matches_do_not_overlap() {
        assert_eq!(run("aaaa", "aa"), [(0, 2), (2, 4)]);
    }

    #[test]
    fn typing_jumps_to_the_match_and_enter_cycles_through_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap();
        let (requests, _worker) = mpsc::channel();
        let (messages, _inbox) = mpsc::channel();
        let mut app = App::new(path.clone(), path, false, requests, messages);
        let lines: Vec<String> = (0..100)
            .map(|index| {
                if index % 30 == 20 {
                    "needle".to_owned()
                } else {
                    "hay".to_owned()
                }
            })
            .collect();
        app.preview = Preview::Text {
            lines,
            colors: Vec::new(),
            truncated: false,
        };
        app.focus = Focus::Preview;

        let press = |app: &mut App, code| app.key(KeyEvent::new(code, KeyModifiers::NONE));
        press(&mut app, KeyCode::Char('f'));
        for ch in "needle".chars() {
            press(&mut app, KeyCode::Char(ch));
        }
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.hits.len(), 3);
        assert_eq!(search.hits[search.current].row, 20);
        assert!(app.preview_scroll > 0 && app.preview_scroll <= 20);

        press(&mut app, KeyCode::Enter);
        assert_eq!(app.search.as_ref().unwrap().current, 1);
        press(&mut app, KeyCode::Up);
        press(&mut app, KeyCode::Up);
        assert_eq!(app.search.as_ref().unwrap().current, 2);

        press(&mut app, KeyCode::Esc);
        assert!(app.search.is_none());
    }
}
