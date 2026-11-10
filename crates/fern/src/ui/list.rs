use std::sync::OnceLock;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState};
use unicode_width::UnicodeWidthStr;

use super::ACCENT;

const CAP_LEFT: &str = "\u{e0b6}";
const CAP_RIGHT: &str = "\u{e0b4}";

fn rounded() -> bool {
    static ROUNDED: OnceLock<bool> = OnceLock::new();

    *ROUNDED.get_or_init(|| std::env::var_os("FERN_ROUNDED").is_some_and(|value| value == "1"))
}

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
    let selected = state.selected();
    let width = room(area);
    let items: Vec<ListItem> = rows
        .into_iter()
        .enumerate()
        .map(|(index, spans)| {
            if focused && Some(index) == selected {
                return ListItem::new(pill(spans, width));
            }

            let mut line = vec![Span::raw(" ")];
            line.extend(spans);

            ListItem::new(Line::from(line))
        })
        .collect();
    let dimmed = Style::new().add_modifier(Modifier::BOLD).fg(ACCENT);
    let list =
        List::new(items)
            .block(block)
            .highlight_style(if focused { Style::new() } else { dimmed });
    frame.render_stateful_widget(list, area, state);
}

fn pill(spans: Vec<Span<'static>>, width: usize) -> Line<'static> {
    let selected = Style::new().fg(Color::Black).bg(ACCENT);
    let used: usize = spans.iter().map(|span| span.content.width()).sum();

    let (left, right) = if rounded() {
        (
            Span::styled(CAP_LEFT, Style::new().fg(ACCENT)),
            Span::styled(CAP_RIGHT, Style::new().fg(ACCENT)),
        )
    } else {
        (Span::styled(" ", selected), Span::styled(" ", selected))
    };

    let mut line = vec![left];
    line.extend(
        spans
            .into_iter()
            .map(|span| Span::styled(span.content, span.style.patch(selected))),
    );
    line.push(Span::styled(
        " ".repeat(width.saturating_sub(used)),
        selected,
    ));
    line.push(right);

    Line::from(line)
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
