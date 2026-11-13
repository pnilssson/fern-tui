use std::path::Path;
use std::thread;

use fern_git::{Error, Status};
use ratatui_image::errors::Errors;
use ratatui_image::thread::ResizeResponse;

use super::{App, Mode, Refresh};
use crate::Msg;
use crate::preview::{Preview, Request, Target};

fn ancestors(path: &str) -> impl Iterator<Item = String> + '_ {
    path.match_indices('/')
        .map(|(index, _)| path[..index].to_owned())
}

impl App {
    pub(super) fn refresh_all(&mut self) {
        self.reload();
        self.refresh_git();
    }

    pub(super) fn refresh_git(&mut self) {
        if !self.in_repo {
            return;
        }
        if self.refresh != Refresh::Idle {
            self.refresh = Refresh::Queued;
            return;
        }

        self.refresh = Refresh::Running;
        let root = self.root.clone();
        let messages = self.messages.clone();
        thread::spawn(move || {
            let _ = messages.send(Msg::Status(root.clone(), fern_git::status(&root)));
        });
    }

    pub(super) fn apply_status(&mut self, root: &Path, result: Result<Status, Error>) {
        let queued = self.refresh == Refresh::Queued;
        self.refresh = Refresh::Idle;
        if !self.in_repo || root != self.root {
            if queued {
                self.refresh_git();
            }

            return;
        }

        match result {
            Ok(status) => self.set_status(status),
            Err(error) => self.git_error = Some(error.to_string()),
        }

        self.request_preview();
        if queued {
            self.refresh_git();
        }
    }

    fn set_status(&mut self, status: Status) {
        let kept = self.selected_change().map(|change| change.path.clone());
        let previous = self.changes_state.selected();
        self.marks = status
            .changes
            .iter()
            .map(|change| (change.path.clone(), change.clone()))
            .collect();
        self.dirty_dirs = status
            .changes
            .iter()
            .flat_map(|change| ancestors(&change.path))
            .collect();
        self.status = status;
        self.git_error = None;
        self.changes = self.visible_changes();

        let len = self.changes.len();
        let index = kept
            .and_then(|path| {
                self.changes
                    .iter()
                    .position(|(change, _)| change.path == path)
            })
            .or_else(|| previous.map(|index| index.min(len.saturating_sub(1))));
        self.changes_state.select(if len == 0 {
            None
        } else {
            Some(index.unwrap_or(0))
        });
    }

    pub(super) fn apply_preview(&mut self, id: u64, preview: Preview) {
        if id != self.latest {
            return;
        }

        let fresh = self.wanted_key != self.shown_key;
        if fresh {
            self.selection = None;
            self.shown_key.clone_from(&self.wanted_key);
        }
        self.preview = preview;
        if fresh {
            self.preview_scroll = match &self.preview {
                Preview::Diff { diff, .. } => diff.hunks.first().copied().unwrap_or(0),
                _ => 0,
            };
        }
        self.preview_scroll = self
            .preview_scroll
            .min(self.preview.len().saturating_sub(1));
    }

    pub(super) fn apply_resized(&mut self, id: u64, result: Result<ResizeResponse, Errors>) {
        if id != self.latest {
            return;
        }

        if let (Preview::Image(protocol), Ok(done)) = (&mut self.preview, result) {
            protocol.update_resized_protocol(done);
        }
    }

    pub(super) fn request_preview(&mut self) {
        let (target, key) = match self.mode {
            Mode::Files => {
                let Some(entry) = self.selected_entry() else {
                    self.show(Preview::Message("Empty folder".to_owned()));
                    return;
                };
                let path = self.cwd.join(&entry.name);
                let key = path.to_string_lossy().into_owned();
                let target = if entry.dir {
                    Target::Dir(path, self.hidden)
                } else {
                    Target::File(path)
                };

                (target, key)
            }
            Mode::Changes => {
                let Some(change) = self.selected_change() else {
                    self.show(Preview::Message("No changes".to_owned()));
                    return;
                };

                (
                    Target::Change(self.root.clone(), change.clone(), self.context),
                    format!("change:{}:{:?}", change.path, self.context),
                )
            }
        };

        self.latest += 1;
        self.wanted_key = key;
        let _ = self.requests.send(Request {
            id: self.latest,
            target,
        });
    }

    fn show(&mut self, preview: Preview) {
        self.latest += 1;
        self.preview = preview;
        self.preview_scroll = 0;
        self.selection = None;
        self.shown_key.clear();
    }
}
