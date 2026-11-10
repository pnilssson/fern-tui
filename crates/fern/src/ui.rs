mod changes;
mod diff;
mod files;
mod list;
mod paint;
mod preview;
mod ruler;
mod wrap;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use crate::app::{App, Mode};

const ACCENT: Color = Color::Cyan;
const MAX_CELL: usize = 2000;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let [body, footer] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());
    frame.render_widget(footer_line(app), footer);

    let [side, main] =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)]).areas(body);
    if app.in_repo {
        let [top, bottom] =
            Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)]).areas(side);
        files::draw(frame, app, top);
        changes::draw(frame, app, bottom);
    } else {
        files::draw(frame, app, side);
    }

    let name = match app.mode {
        Mode::Files => app.selected_entry().map(|entry| entry.name.clone()),
        Mode::Changes => app.selected_change().map(|change| change.path.clone()),
    };
    app.preview_area = main;
    preview::draw(frame, app, main, &name.unwrap_or_default());

    if let Some((title, rows)) = app.menu() {
        menu(frame, body, title, &rows);
    }
}

fn menu(frame: &mut Frame, body: Rect, title_text: &str, rows: &[(String, &str)]) {
    let width = rows
        .iter()
        .map(|(keys, _)| keys.chars().count())
        .max()
        .unwrap_or_default();
    let lines: Vec<Line> = rows
        .iter()
        .map(|(keys, help)| {
            Line::from(vec![
                Span::styled(
                    format!(" {keys:width$}  "),
                    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::raw((*help).to_owned()),
            ])
        })
        .collect();
    let height = u16::try_from(lines.len() + 2).unwrap_or(u16::MAX);
    let area = Rect {
        y: body.bottom().saturating_sub(height).max(body.y),
        height: height.min(body.height),
        ..body
    };

    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).block(pane(title(title_text, area.width), true)),
        area,
    );
}

fn footer_line(app: &App) -> Paragraph<'static> {
    if let Some(error) = &app.git_error {
        return Paragraph::new(Span::styled(
            format!(" {error}"),
            Style::new().fg(Color::Red),
        ));
    }

    if let Some(notice) = app.notice() {
        return Paragraph::new(Span::styled(
            format!(" {notice}"),
            Style::new().fg(Color::Green),
        ));
    }

    let hints = "? keys  q quit";

    let mut spans = vec![Span::styled(
        format!(" {hints}"),
        Style::new().fg(Color::DarkGray),
    )];
    if let Some(version) = &app.update {
        spans.push(Span::styled(
            format!("  v{version} available, run fern-update"),
            Style::new().fg(Color::Yellow),
        ));
    }

    Paragraph::new(Line::from(spans))
}

fn pane(title: Line<'static>, focused: bool) -> Block<'static> {
    let border = if focused {
        Style::new().fg(ACCENT)
    } else {
        Style::new().fg(Color::DarkGray)
    };

    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(title)
}

fn title(text: &str, width: u16) -> Line<'static> {
    let room = usize::from(width).saturating_sub(4);
    let shown: String = if text.chars().count() > room {
        let tail: Vec<char> = text.chars().rev().take(room.saturating_sub(1)).collect();
        std::iter::once('…').chain(tail.into_iter().rev()).collect()
    } else {
        text.to_owned()
    };

    Line::from(Span::styled(
        format!(" {shown} "),
        Style::new().add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Left)
}

pub(crate) fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars().take(MAX_CELL) {
        match ch {
            '\t' => out.push_str("    "),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }

    out
}

fn dim() -> Style {
    Style::new().fg(Color::DarkGray)
}

fn center(frame: &mut Frame, area: Rect, text: &str) {
    frame.render_widget(
        Paragraph::new(Span::styled(text.to_owned(), dim())).alignment(Alignment::Center),
        area,
    );
}
