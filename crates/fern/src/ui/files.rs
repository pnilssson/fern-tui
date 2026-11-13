use std::path::Path;

use fern_git::Change;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::list::{self, aligned, clip_end};
use super::{pane, title};
use crate::app::{App, Focus, Mode};
use crate::entries::Entry;

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.mode == Mode::Files && app.focus == Focus::List;
    let cwd = shown(&app.cwd, std::env::var_os("HOME").as_deref().map(Path::new));
    let rows = rows(app, &app.entries, &app.cwd_rel, list::room(area));
    let mut block = pane(title(&format!("[1] {cwd}"), area.width), focused);
    if let Some(filter) = &app.filter {
        block = block.title_bottom(query(filter));
    }
    list::draw(frame, area, block, rows, &mut app.files_state, focused);
}

fn shown(cwd: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| cwd.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => cwd.display().to_string(),
    }
}

pub fn rows(app: &App, entries: &[Entry], base: &str, width: usize) -> Vec<Vec<Span<'static>>> {
    entries
        .iter()
        .map(|entry| {
            let rel = if base.is_empty() {
                entry.name.clone()
            } else {
                format!("{base}/{}", entry.name)
            };
            let (label, style) = if entry.dir {
                (
                    format!("{}/", entry.name),
                    Style::new().fg(Color::Blue).add_modifier(Modifier::BOLD),
                )
            } else {
                (entry.name.clone(), Style::new())
            };
            let mark = if entry.dir {
                app.dirty_dirs
                    .contains(&rel)
                    .then(|| Span::styled("●", Style::new().fg(Color::Yellow)))
            } else {
                app.marks.get(&rel).map(mark)
            };

            let reserved = if mark.is_some() { 2 } else { 0 };
            let name = clip_end(&label, width.saturating_sub(reserved));

            aligned(
                name_spans(&name, &entry.hits, style),
                mark.into_iter().collect(),
                width,
            )
        })
        .collect()
}

pub fn query(query: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!(" /{query}"),
            Style::new().add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::new().add_modifier(Modifier::REVERSED)),
        Span::raw(" "),
    ])
}

pub fn name_spans(text: &str, hits: &[u32], style: Style) -> Vec<Span<'static>> {
    let hit = style
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    let mut spans = Vec::new();
    let mut run = String::new();
    let mut lit = false;
    for (index, ch) in text.chars().enumerate() {
        let now = u32::try_from(index).is_ok_and(|index| hits.binary_search(&index).is_ok());
        if now != lit && !run.is_empty() {
            spans.push(Span::styled(
                std::mem::take(&mut run),
                if lit { hit } else { style },
            ));
        }

        lit = now;
        run.push(ch);
    }
    if !run.is_empty() {
        spans.push(Span::styled(run, if lit { hit } else { style }));
    }

    spans
}

pub fn mark(change: &Change) -> Span<'static> {
    let (text, color) = if change.conflict() {
        ("!", Color::Red)
    } else if change.untracked() {
        ("?", Color::Magenta)
    } else if change.y != ' ' {
        (letter(change.y), Color::Yellow)
    } else {
        (letter(change.x), Color::Green)
    };
    let style = if change.y == ' ' && !change.untracked() {
        Style::new().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(color)
    };

    Span::styled(text, style)
}

fn letter(code: char) -> &'static str {
    match code {
        'A' => "A",
        'D' => "D",
        'R' => "R",
        'C' => "C",
        _ => "M",
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::shown;

    #[test]
    fn the_home_folder_is_abbreviated() {
        let home = Some(Path::new("/Users/me"));

        assert_eq!(shown(Path::new("/Users/me"), home), "~");
        assert_eq!(shown(Path::new("/Users/me/repos/app"), home), "~/repos/app");
        assert_eq!(shown(Path::new("/etc"), home), "/etc");
        assert_eq!(shown(Path::new("/etc"), None), "/etc");
    }
}
