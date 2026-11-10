use fern_git::Change;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;

use super::files::mark;
use super::list::{self, aligned, clip_end, clip_start, width};
use super::{dim, pane, title};
use crate::app::{App, Focus, Mode};

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.mode == Mode::Changes && app.focus == Focus::List;
    let room = list::room(area);
    let rows = app
        .status
        .changes
        .iter()
        .map(|change| row(change, room))
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
    let heading = format!(
        "[2] Changes ({})  {}{ahead}{behind}",
        rows.len(),
        app.status.branch
    );
    let block = pane(title(&heading, area.width), focused);
    list::draw(frame, area, block, rows, &mut app.changes_state, focused);
}

fn row(change: &Change, room: usize) -> Vec<Span<'static>> {
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
    let mut left = Vec::new();
    if !dir.is_empty() && dir_space > 0 {
        left.push(Span::styled(
            clip_start(&format!("{dir}/"), dir_space),
            dim(),
        ));
    }
    left.push(Span::raw(file));

    aligned(left, right, room)
}
