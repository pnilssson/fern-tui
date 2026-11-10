use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use fern_git::{Change, Context, Diff};
use image::ImageReader;
use ratatui_image::picker::Picker;
use ratatui_image::thread::{ResizeRequest, ThreadProtocol};

use crate::Msg;
use crate::entries::{self, Entry};
use crate::highlight::{self, Colors};

const MAX_BYTES: u64 = 256 * 1024;
const MAX_LINES: usize = 3000;
const MAX_ENTRIES: usize = 1000;
const MAX_IMAGE_BYTES: u64 = 32 << 20;
const IMAGES: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];

pub enum Preview {
    Empty,
    Message(String),
    Text {
        lines: Vec<String>,
        colors: Vec<Colors>,
        truncated: bool,
    },
    Dir(Vec<Entry>),
    Image(Box<ThreadProtocol>),
    Diff {
        diff: Diff,
        colors: Vec<Colors>,
    },
}

impl Preview {
    pub fn len(&self) -> usize {
        match self {
            Preview::Text { lines, .. } => lines.len(),
            Preview::Diff { diff, .. } => diff.rows.len(),
            Preview::Dir(entries) => entries.len(),
            Preview::Empty | Preview::Message(_) | Preview::Image(_) => 0,
        }
    }
}

pub enum Target {
    File(PathBuf),
    Dir(PathBuf, bool),
    Change(PathBuf, Change, Context),
}

pub struct Request {
    pub id: u64,
    pub target: Target,
}

struct Pictures {
    picker: Picker,
    messages: Sender<Msg>,
}

impl Pictures {
    fn load(&self, id: u64, path: &Path) -> Preview {
        let size = fs::metadata(path).map_or(0, |meta| meta.len());
        if size > MAX_IMAGE_BYTES {
            return Preview::Message(format!("Image too large, {}", size_text(size)));
        }

        let decoded = ImageReader::open(path)
            .map_err(|error| error.to_string())
            .and_then(|reader| reader.decode().map_err(|error| error.to_string()));
        let image = match decoded {
            Ok(image) => image,
            Err(error) => return Preview::Message(error),
        };

        let (resizes, encoder) = mpsc::channel::<ResizeRequest>();
        let messages = self.messages.clone();
        thread::spawn(move || {
            while let Ok(request) = encoder.recv() {
                let done = Box::new(request.resize_encode());
                if messages.send(Msg::Resized(id, done)).is_err() {
                    break;
                }
            }
        });
        let protocol = self.picker.new_resize_protocol(image);

        Preview::Image(Box::new(ThreadProtocol::new(resizes, Some(protocol))))
    }
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| IMAGES.iter().any(|image| image.eq_ignore_ascii_case(ext)))
}

pub fn spawn(requests: Receiver<Request>, messages: Sender<Msg>, picker: Picker) {
    let pictures = Pictures {
        picker,
        messages: messages.clone(),
    };
    thread::spawn(move || {
        while let Ok(mut request) = requests.recv() {
            while let Ok(newer) = requests.try_recv() {
                request = newer;
            }

            let preview = match &request.target {
                Target::File(path) if is_image(path) => pictures.load(request.id, path),
                Target::File(path) => file(path),
                Target::Dir(path, hidden) => dir(path, *hidden),
                Target::Change(root, change, context) => {
                    let path = root.join(&change.path);
                    if is_image(&path) && path.is_file() {
                        pictures.load(request.id, &path)
                    } else {
                        change_diff(root, change, *context)
                    }
                }
            };
            if messages.send(Msg::Preview(request.id, preview)).is_err() {
                break;
            }
        }
    });
}

fn dir(path: &Path, hidden: bool) -> Preview {
    let mut entries = entries::read(path, hidden);
    entries.truncate(MAX_ENTRIES);
    if entries.is_empty() {
        return Preview::Message("Empty folder".to_owned());
    }

    Preview::Dir(entries)
}

fn size_text(bytes: u64) -> String {
    let tenths = |unit: u64| bytes % unit * 10 / unit;
    match bytes {
        b if b >= 1 << 20 => format!("{}.{} MB", b >> 20, tenths(1 << 20)),
        b if b >= 1 << 10 => format!("{}.{} KB", b >> 10, tenths(1 << 10)),
        b => format!("{b} B"),
    }
}

enum Loaded {
    Text { lines: Vec<String>, truncated: bool },
    Message(String),
}

fn file(path: &Path) -> Preview {
    match read(path) {
        Loaded::Text { lines, truncated } => {
            let colors = highlight::text(&path.to_string_lossy(), &lines);

            Preview::Text {
                lines,
                colors,
                truncated,
            }
        }
        Loaded::Message(text) => Preview::Message(text),
    }
}

fn read(path: &Path) -> Loaded {
    let opened = File::open(path).and_then(|file| {
        let size = file.metadata()?.len();
        let mut bytes = Vec::new();
        file.take(MAX_BYTES).read_to_end(&mut bytes)?;

        Ok((size, bytes))
    });
    let (size, bytes) = match opened {
        Ok(read) => read,
        Err(error) => return Loaded::Message(error.to_string()),
    };
    if bytes.is_empty() {
        return Loaded::Message("Empty file".to_owned());
    }
    if bytes.iter().take(8192).any(|&byte| byte == 0) {
        return Loaded::Message(format!("Binary file, {}", size_text(size)));
    }

    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text.lines().take(MAX_LINES).map(str::to_owned).collect();
    let truncated = size > MAX_BYTES || text.lines().nth(MAX_LINES).is_some();

    Loaded::Text { lines, truncated }
}

fn change_diff(root: &Path, change: &Change, context: Context) -> Preview {
    if change.untracked() {
        return match read(&root.join(&change.path)) {
            Loaded::Text { lines, truncated } => shown(&change.path, Diff::added(lines, truncated)),
            Loaded::Message(text) => Preview::Message(text),
        };
    }

    match fern_git::diff(root, change, context) {
        Ok(diff) if diff.rows.is_empty() => Preview::Message("No changes".to_owned()),
        Ok(diff) => shown(&change.path, diff),
        Err(error) => Preview::Message(error.to_string()),
    }
}

fn shown(name: &str, diff: Diff) -> Preview {
    let colors = highlight::rows(name, &diff.rows);

    Preview::Diff { diff, colors }
}
