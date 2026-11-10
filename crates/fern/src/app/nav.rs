use std::path::{Path, PathBuf};

use fern_git::Context;
use ratatui::widgets::ListState;

use super::{App, Focus, Mode, rel};
use crate::entries::{self, Entry};
use crate::preview::Preview;

#[derive(Clone, Copy)]
pub(super) enum Step {
    Up(usize),
    Down(usize),
    Top,
    Bottom,
}

impl App {
    pub(super) fn page(&self) -> usize {
        self.preview_height.max(1)
    }

    pub(super) fn set_mode(&mut self, mode: Mode) {
        if mode == Mode::Changes && !self.in_repo {
            return;
        }

        self.mode = mode;
        self.focus = Focus::List;
        self.request_preview();
    }

    pub(super) fn toggle_mode(&mut self) {
        let next = if self.mode == Mode::Files {
            Mode::Changes
        } else {
            Mode::Files
        };
        self.set_mode(next);
    }

    pub(super) fn toggle_context(&mut self) {
        self.context = match self.context {
            Context::Full => Context::Hunks,
            Context::Hunks => Context::Full,
        };
        self.request_preview();
    }

    pub(super) fn toggle_hidden(&mut self) {
        self.hidden = !self.hidden;
        self.reload();
    }

    pub(super) fn move_sel(&mut self, step: Step) {
        let (len, state) = match self.mode {
            Mode::Files => (self.entries.len(), &mut self.files_state),
            Mode::Changes => (self.status.changes.len(), &mut self.changes_state),
        };
        if len == 0 {
            return;
        }

        let current = state.selected().unwrap_or(0);
        let next = match step {
            Step::Up(n) => current.saturating_sub(n),
            Step::Down(n) => current.saturating_add(n).min(len - 1),
            Step::Top => 0,
            Step::Bottom => len - 1,
        };
        if state.selected() == Some(next) {
            return;
        }

        state.select(Some(next));
        self.remember();
        self.request_preview();
    }

    pub(super) fn open(&mut self) {
        match self.mode {
            Mode::Files => self.enter(),
            Mode::Changes => self.read(),
        }
    }

    pub(super) fn read(&mut self) {
        if matches!(self.preview, Preview::Text { .. } | Preview::Diff { .. }) {
            self.focus = Focus::Preview;
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.files_state
            .selected()
            .and_then(|index| self.entries.get(index))
    }

    pub fn selected_change(&self) -> Option<&fern_git::Change> {
        self.changes_state
            .selected()
            .and_then(|index| self.status.changes.get(index))
    }

    pub(super) fn remember(&mut self) {
        if self.mode != Mode::Files {
            return;
        }

        if let Some(entry) = self.selected_entry() {
            let name = entry.name.clone();
            self.memory.insert(self.cwd.clone(), name);
        }
    }

    pub(super) fn load_dir(&mut self, select: Option<&str>) {
        self.entries = entries::read(&self.cwd, self.hidden);
        let wanted = select
            .map(str::to_owned)
            .or_else(|| self.memory.get(&self.cwd).cloned());
        let index =
            wanted.and_then(|name| self.entries.iter().position(|entry| entry.name == name));
        self.files_state = ListState::default();
        if !self.entries.is_empty() {
            self.files_state.select(Some(index.unwrap_or(0)));
        }

        self.remember();
        self.request_preview();
    }

    pub(super) fn reload(&mut self) {
        let name = self.selected_entry().map(|entry| entry.name.clone());
        self.load_dir(name.as_deref());
    }

    fn set_cwd(&mut self, cwd: PathBuf) {
        self.cwd_rel = rel(&self.root, &cwd);
        self.cwd = cwd;
    }

    pub(super) fn go_up(&mut self) {
        if self.mode != Mode::Files || self.cwd == self.root {
            return;
        }

        let Some(parent) = self.cwd.parent().map(Path::to_path_buf) else {
            return;
        };
        let name = self
            .cwd
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        self.set_cwd(parent);
        self.load_dir(name.as_deref());
    }

    fn enter(&mut self) {
        let Some(entry) = self.selected_entry() else {
            return;
        };
        if !entry.dir {
            self.read();

            return;
        }

        let next = self.cwd.join(&entry.name);
        self.set_cwd(next);
        self.load_dir(None);
    }

    pub(super) fn reveal(&mut self) {
        let Some(change) = self.selected_change() else {
            return;
        };
        let path = change.path.clone();
        let full = self.root.join(&path);
        let mut dir = full.parent().unwrap_or(&self.root).to_path_buf();
        while dir != self.root && !dir.is_dir() {
            dir = dir.parent().unwrap_or(&self.root).to_path_buf();
        }

        if path.split('/').any(|part| part.starts_with('.')) {
            self.hidden = true;
        }
        let name = full
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        self.set_cwd(dir);
        self.mode = Mode::Files;
        self.focus = Focus::List;
        self.load_dir(name.as_deref());
    }

    pub(super) fn scroll_down(&mut self, amount: usize) {
        let last = self.preview.len().saturating_sub(1);
        self.preview_scroll = self.preview_scroll.saturating_add(amount).min(last);
    }

    pub(super) fn scroll_up(&mut self, amount: usize) {
        self.preview_scroll = self.preview_scroll.saturating_sub(amount);
    }

    pub(super) fn jump_hunk(&mut self, forward: bool) {
        let Preview::Diff { diff, .. } = &self.preview else {
            return;
        };

        let at = self.preview_scroll;
        let target = if forward {
            diff.hunks.iter().copied().find(|&hunk| hunk > at)
        } else {
            diff.hunks.iter().copied().rev().find(|&hunk| hunk < at)
        };
        if let Some(target) = target {
            self.preview_scroll = target;
        }
    }
}
