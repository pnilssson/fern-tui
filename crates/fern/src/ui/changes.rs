use fern_git::Change;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;

use super::files::{mark, name_spans, query};
use super::list::{self, aligned, clip_end, clip_start, width};
use super::{dim, pane, title};
use crate::app::{App, Focus, Mode};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.mode == Mode::Changes && app.focus == Focus::List;
    let room = list::room(area);
    let rows = app
        .changes
        .iter()
        .map(|(change, hits)| row(change, hits, room))
        .collect::<Vec<_>>();
    let ahead = if app.status.ahead > 0 {
        format!(" ↑{}", app.status.ahead)
    } else {
        String::new()
    };
    let behind = if app.status.behind > 0 {
        format!(" ↓{}", app.status.behind)
    } else {
        String::new()
    };
    let count = if app.changes_filter.is_some() {
        format!("{}/{}", rows.len(), app.status.changes.len())
    } else {
        rows.len().to_string()
    };
    let heading = format!("[2] ({count}) {}{ahead}{behind}", app.status.branch);
    let mut block = pane(title(&heading, area.width), focused);
    if let Some(filter) = &app.changes_filter {
        block = block.title_bottom(query(filter));
    }
    list::draw(frame, area, block, rows, &mut app.changes_state, focused);
}

fn row(change: &Change, hits: &[u32], room: usize) -> Vec<Span<'static>> {
    let mut right = Vec::new();
    if change.adds > 0 {
        right.push(Span::styled(
            format!("+{}", change.adds),
            Style::new().fg(Color::Green),
        ));
    }
    if change.dels > 0 {
        if !right.is_empty() {
            right.push(Span::raw(" "));
        }
        right.push(Span::styled(
            format!("−{}", change.dels),
            Style::new().fg(Color::Red),
        ));
    }
    if !right.is_empty() {
        right.push(Span::raw(" "));
    }
    right.push(mark(change));

    let (dir, file) = change
        .path
        .rsplit_once('/')
        .map_or(("", change.path.as_str()), |(dir, file)| (dir, file));
    let space = room.saturating_sub(width(&right) + 1);
    let file = clip_end(file, space);
    let dir_space = space.saturating_sub(file.chars().count());
    let dir_len = if dir.is_empty() {
        0
    } else {
        dir.chars().count() + 1
    };
    let mut left = Vec::new();
    if dir_len > 0 && dir_space > 0 {
        let shown = clip_start(&format!("{dir}/"), dir_space);
        let clipped = shown.chars().count() < dir_len;
        let dropped = if clipped {
            dir_len + 1 - shown.chars().count()
        } else {
            0
        };
        let dir_hits: Vec<u32> = hits
            .iter()
            .filter(|&&hit| (hit as usize) < dir_len)
            .filter_map(|&hit| {
                let moved = (hit as usize).checked_sub(dropped)?;

                u32::try_from(moved + usize::from(clipped)).ok()
            })
            .collect();
        left.extend(name_spans(&shown, &dir_hits, dim()));
    }
    let file_hits: Vec<u32> = hits
        .iter()
        .filter_map(|&hit| hit.checked_sub(u32::try_from(dir_len).ok()?))
        .collect();
    left.extend(name_spans(&file, &file_hits, Style::new()));

    aligned(left, right, room)
}
