mod copy;
mod filter;
mod help;
mod keys;
mod nav;
mod select;
mod sync;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use fern_git::{Change, Context, Status};
use nucleo_matcher::{Config, Matcher};
use ratatui::crossterm::event::Event;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;

use crate::Msg;
use crate::entries::Entry;
use crate::preview::{Preview, Request};
use crate::watch::{self, Watcher};

const NOTICE: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Files,
    Changes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Preview,
}

pub use select::{Selection, Visual};

struct Notice {
    text: String,
    expires: Instant,
}

pub type Menu = (&'static str, Vec<(String, &'static str)>);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Chord {
    Copy,
    Help,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Refresh {
    Idle,
    Running,
    Queued,
}

pub struct App {
    pub root: PathBuf,
    pub in_repo: bool,
    pub cwd: PathBuf,
    pub cwd_rel: String,
    pub mode: Mode,
    pub focus: Focus,
    pub context: Context,
    pub quit: bool,
    pub update: Option<String>,

    pub entries: Vec<Entry>,
    pub filter: Option<String>,
    pub files_state: ListState,

    pub status: Status,
    pub changes: Vec<(Change, Vec<u32>)>,
    pub changes_filter: Option<String>,
    pub git_error: Option<String>,
    pub marks: HashMap<String, Change>,
    pub dirty_dirs: HashSet<String>,
    pub changes_state: ListState,

    pub preview: Preview,
    pub preview_scroll: usize,
    pub preview_height: usize,
    pub preview_area: Rect,
    pub preview_inner: Rect,
    pub preview_rows: Vec<Visual>,
    pub selection: Option<Selection>,

    listing: Vec<Entry>,
    matcher: Matcher,
    notice: Option<Notice>,
    hidden: bool,
    chord: Option<Chord>,
    watcher: Option<Watcher>,
    memory: HashMap<PathBuf, String>,
    refresh: Refresh,
    shown_key: String,
    wanted_key: String,
    latest: u64,
    requests: Sender<Request>,
    messages: Sender<Msg>,
}

pub fn rel(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);

    rel.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

impl App {
    pub fn new(
        root: PathBuf,
        cwd: PathBuf,
        in_repo: bool,
        requests: Sender<Request>,
        messages: Sender<Msg>,
    ) -> Self {
        let watcher = watch::start(&root, in_repo, messages.clone());
        let mut app = Self {
            cwd_rel: rel(&root, &cwd),
            root,
            in_repo,
            cwd,
            mode: Mode::Files,
            focus: Focus::List,
            context: Context::Full,
            quit: false,
            update: None,
            entries: Vec::new(),
            filter: None,
            files_state: ListState::default(),
            status: Status::default(),
            changes: Vec::new(),
            changes_filter: None,
            git_error: None,
            marks: HashMap::new(),
            dirty_dirs: HashSet::new(),
            changes_state: ListState::default(),
            preview: Preview::Empty,
            preview_scroll: 0,
            preview_height: 20,
            preview_area: Rect::default(),
            preview_inner: Rect::default(),
            preview_rows: Vec::new(),
            selection: None,
            listing: Vec::new(),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            notice: None,
            hidden: false,
            chord: None,
            watcher,
            memory: HashMap::new(),
            refresh: Refresh::Idle,
            shown_key: String::new(),
            wanted_key: String::new(),
            latest: 0,
            requests,
            messages,
        };
        app.load_dir(None);
        app.refresh_git();

        app
    }

    pub fn say(&mut self, text: String) {
        self.notice = Some(Notice {
            text,
            expires: Instant::now() + NOTICE,
        });
    }

    pub fn notice(&self) -> Option<&str> {
        self.notice.as_ref().map(|notice| notice.text.as_str())
    }

    pub fn notice_wait(&self) -> Option<Duration> {
        let notice = self.notice.as_ref()?;

        Some(notice.expires.saturating_duration_since(Instant::now()))
    }

    pub fn expire_notice(&mut self) {
        if self.notice_wait() == Some(Duration::ZERO) {
            self.notice = None;
        }
    }

    pub fn menu(&self) -> Option<Menu> {
        let rows = match self.chord? {
            Chord::Copy => (
                "Copy",
                copy::OPTIONS
                    .iter()
                    .map(|(key, _, help)| (key.to_string(), *help))
                    .collect(),
            ),
            Chord::Help => (
                "Keys",
                help::bindings(self.mode, self.focus)
                    .iter()
                    .map(|(keys, help)| ((*keys).to_owned(), *help))
                    .collect(),
            ),
        };

        Some(rows)
    }

    pub fn handle(&mut self, message: Msg) {
        match message {
            Msg::Input(Event::Key(key)) => self.key(key),
            Msg::Input(Event::Mouse(mouse)) => self.mouse(mouse),
            Msg::Input(_) => {}
            Msg::Changed => self.refresh_all(),
            Msg::Status(root, result) => self.apply_status(&root, result),
            Msg::Preview(id, preview) => self.apply_preview(id, preview),
            Msg::Resized(id, result) => self.apply_resized(id, *result),
            Msg::Update(version) => self.update = Some(version),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::App;

    #[test]
    fn notice_disappears_once_it_expires() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().canonicalize().unwrap();
        let (requests, _worker) = mpsc::channel();
        let (messages, _inbox) = mpsc::channel();
        let mut app = App::new(path.clone(), path, false, requests, messages);

        app.say("Copied".to_owned());
        app.expire_notice();
        assert_eq!(app.notice(), Some("Copied"));

        if let Some(notice) = &mut app.notice {
            notice.expires = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
        }
        app.expire_notice();
        assert_eq!(app.notice(), None);
    }
}
