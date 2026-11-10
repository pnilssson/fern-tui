mod keys;
mod nav;
mod sync;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use fern_git::{Change, Context, Status};
use ratatui::crossterm::event::Event;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;

use crate::Msg;
use crate::entries::Entry;
use crate::preview::{Preview, Request};

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
    pub files_state: ListState,

    pub status: Status,
    pub git_error: Option<String>,
    pub marks: HashMap<String, Change>,
    pub dirty_dirs: HashSet<String>,
    pub changes_state: ListState,

    pub preview: Preview,
    pub preview_scroll: usize,
    pub preview_height: usize,
    pub preview_area: Rect,

    hidden: bool,
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
            files_state: ListState::default(),
            status: Status::default(),
            git_error: None,
            marks: HashMap::new(),
            dirty_dirs: HashSet::new(),
            changes_state: ListState::default(),
            preview: Preview::Empty,
            preview_scroll: 0,
            preview_height: 20,
            preview_area: Rect::default(),
            hidden: false,
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

    pub fn handle(&mut self, message: Msg) {
        match message {
            Msg::Input(Event::Key(key)) => self.key(key),
            Msg::Input(Event::Mouse(mouse)) => self.mouse(mouse),
            Msg::Input(_) => {}
            Msg::Changed => self.refresh_all(),
            Msg::Status(result) => self.apply_status(result),
            Msg::Preview(id, preview) => self.apply_preview(id, preview),
            Msg::Update(version) => self.update = Some(version),
        }
    }
}
