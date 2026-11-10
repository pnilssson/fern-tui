use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState};
use unicode_width::UnicodeWidthStr;

use super::ACCENT;

pub fn room(area: Rect) -> usize {
    usize::from(area.width).saturating_sub(4)
}

pub fn draw(
    frame: &mut Frame,
    area: Rect,
    block: Block<'static>,
    rows: Vec<Vec<Span<'static>>>,
    state: &mut ListState,
    focused: bool,
) {
    let items: Vec<ListItem> = rows
        .into_iter()
        .map(|spans| {
            let mut line = vec![Span::raw(" ")];
            line.extend(spans);

            ListItem::new(Line::from(line))
        })
        .collect();
    let bold = Style::new().add_modifier(Modifier::BOLD);
    let selected = if focused { bold.fg(ACCENT) } else { bold };
    let list = List::new(items).block(block).highlight_style(selected);
    frame.render_stateful_widget(list, area, state);
}

pub fn width(spans: &[Span<'static>]) -> usize {
    spans.iter().map(|span| span.content.width()).sum()
}

pub fn aligned(
    mut left: Vec<Span<'static>>,
    right: Vec<Span<'static>>,
    room: usize,
) -> Vec<Span<'static>> {
    let gap = room.saturating_sub(width(&left) + width(&right));
    left.push(Span::raw(" ".repeat(gap)));
    left.extend(right);

    left
}

pub fn clip_end(text: &str, room: usize) -> String {
    if text.chars().count() <= room {
        return text.to_owned();
    }

    let kept: String = text.chars().take(room.saturating_sub(1)).collect();

    format!("{kept}…")
}

pub fn clip_start(text: &str, room: usize) -> String {
    let count = text.chars().count();
    if count <= room {
        return text.to_owned();
    }

    let kept: String = text.chars().skip(count - room.saturating_sub(1)).collect();

    format!("…{kept}")
}
