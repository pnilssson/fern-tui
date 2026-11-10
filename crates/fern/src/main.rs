mod app;
mod clipboard;
mod entries;
mod highlight;
mod preview;
mod ui;
mod update;
mod watch;

use std::io::{self, ErrorKind, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;

use fern_git::Status;
use preview::Preview;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;
use ratatui_image::errors::Errors;
use ratatui_image::picker::Picker;
use ratatui_image::thread::ResizeResponse;

use app::App;

pub enum Msg {
    Input(Event),
    Changed,
    Status(PathBuf, Result<Status, fern_git::Error>),
    Preview(u64, Preview),
    Resized(u64, Box<Result<ResizeResponse, Errors>>),
    Update(String),
}

fn main() -> io::Result<()> {
    let Some(start) = start()? else {
        return Ok(());
    };
    let (root, cwd, in_repo) = match fern_git::locate(&start) {
        Some(repo) => (repo.root, repo.cwd, true),
        None => (start.clone(), start, false),
    };

    let mut terminal = ratatui::try_init()?;
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

    let (messages, inbox) = mpsc::channel();
    let (requests, worker) = mpsc::channel();
    preview::spawn(worker, messages.clone(), picker);

    let input = messages.clone();
    thread::spawn(move || {
        while let Ok(event) = event::read() {
            if input.send(Msg::Input(event)).is_err() {
                break;
            }
        }
    });
    update::spawn(messages.clone());

    let mut app = App::new(root, cwd, in_repo, requests, messages);
    execute!(io::stdout(), EnableMouseCapture)?;
    let result = run(&mut terminal, &mut app, &inbox);
    execute!(io::stdout(), DisableMouseCapture)?;
    ratatui::restore();

    result
}

fn start() -> io::Result<Option<PathBuf>> {
    let version = env!("CARGO_PKG_VERSION");
    let mut path = PathBuf::from(".");
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                writeln!(io::stdout(), "fern {version}\n\nUsage: fern [PATH]")?;
                return Ok(None);
            }
            "-V" | "--version" => {
                writeln!(io::stdout(), "fern {version}")?;
                return Ok(None);
            }
            _ => path = PathBuf::from(arg),
        }
    }

    let path = normalize(&std::path::absolute(path)?);
    if !path.is_dir() {
        let message = format!("{} is not a directory", path.display());
        return Err(io::Error::new(ErrorKind::NotADirectory, message));
    }

    Ok(Some(path))
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }

    out
}

fn run(terminal: &mut DefaultTerminal, app: &mut App, inbox: &Receiver<Msg>) -> io::Result<()> {
    while !app.quit {
        app.expire_notice();
        terminal.draw(|frame| ui::draw(frame, app))?;

        let received = match app.notice_wait() {
            Some(wait) => inbox.recv_timeout(wait),
            None => inbox.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let message = match received {
            Ok(message) => message,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        app.handle(message);
        while let Ok(message) = inbox.try_recv() {
            app.handle(message);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::normalize;

    #[test]
    fn normalize_resolves_parent_and_current_components() {
        let path = Path::new("/a/b/../c/./d/..");

        assert_eq!(normalize(path), PathBuf::from("/a/c"));
    }
}
