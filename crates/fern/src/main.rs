mod app;
mod entries;
mod highlight;
mod preview;
mod ui;
mod watch;

use std::io::{self, ErrorKind, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use fern_git::Status;
use preview::Preview;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;

use app::App;

pub enum Msg {
    Input(Event),
    Changed,
    Status(Result<Status, fern_git::Error>),
    Preview(u64, Preview),
}

fn main() -> io::Result<()> {
    let Some(start) = start()? else {
        return Ok(());
    };
    let (root, cwd, in_repo) = match fern_git::locate(&start) {
        Some(repo) => (repo.root, repo.cwd, true),
        None => (start.clone(), start, false),
    };

    let (messages, inbox) = mpsc::channel();
    let (requests, worker) = mpsc::channel();
    preview::spawn(worker, messages.clone());

    let input = messages.clone();
    thread::spawn(move || {
        while let Ok(event) = event::read() {
            if input.send(Msg::Input(event)).is_err() {
                break;
            }
        }
    });
    let _watcher = watch::start(&root, messages.clone());

    let mut app = App::new(root, cwd, in_repo, requests, messages);
    let mut terminal = ratatui::try_init()?;
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

    let path = std::path::absolute(path)?;
    if !path.is_dir() {
        let message = format!("{} is not a directory", path.display());
        return Err(io::Error::new(ErrorKind::NotADirectory, message));
    }

    Ok(Some(path))
}

fn run(terminal: &mut DefaultTerminal, app: &mut App, inbox: &Receiver<Msg>) -> io::Result<()> {
    while !app.quit {
        terminal.draw(|frame| ui::draw(frame, app))?;

        let Ok(message) = inbox.recv() else {
            break;
        };
        app.handle(message);
        while let Ok(message) = inbox.try_recv() {
            app.handle(message);
        }
    }

    Ok(())
}
