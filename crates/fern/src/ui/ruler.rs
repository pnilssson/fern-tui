use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use super::ACCENT;
use crate::preview::Preview;
use fern_git::Row;

const THUMB: &str = "┃";
const TICK: &str = "─";

fn offset(index: usize, len: usize, height: usize) -> u16 {
    let y = (index * height / len).min(height.saturating_sub(1));

    u16::try_from(y).unwrap_or(u16::MAX)
}

pub fn summary(preview: &Preview, scroll: usize, max: usize) -> String {
    if max == 0 {
        return String::new();
    }

    let percent = scroll * 100 / max;
    match preview {
        Preview::Diff { diff, .. } if !diff.hunks.is_empty() => {
            let seen = diff.hunks.iter().filter(|&&start| start <= scroll).count();

            format!("  change {seen}/{}  {percent}%", diff.hunks.len())
        }
        _ => format!("  {percent}%"),
    }
}

pub fn draw(
    frame: &mut Frame,
    area: Rect,
    inner: Rect,
    preview: &Preview,
    scroll: usize,
    max: usize,
    reading: bool,
) {
    let len = preview.len();
    let height = usize::from(inner.height);
    if max == 0 || height == 0 {
        return;
    }

    let x = area.right().saturating_sub(1);
    let buffer = frame.buffer_mut();
    let thumb = Style::new().fg(if reading { ACCENT } else { Color::Gray });
    let first = offset(scroll, len, height);
    let last = if scroll >= max {
        offset(len - 1, len, height)
    } else {
        offset((scroll + height).min(len) - 1, len, height)
    };
    for y in first..=last {
        buffer[(x, inner.y + y)].set_symbol(THUMB).set_style(thumb);
    }

    let Preview::Diff { diff, .. } = preview else {
        return;
    };

    for (index, row) in diff.rows.iter().enumerate() {
        let color = match row {
            Row::Add(..) => Color::Green,
            Row::Del(..) => Color::Red,
            _ => continue,
        };
        let y = inner.y + offset(index, len, height);
        buffer[(x, y)]
            .set_symbol(TICK)
            .set_style(Style::new().fg(color));
    }
}
