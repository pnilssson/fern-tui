use fern_git::Context;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{List, ListItem, Paragraph};

use super::wrap::wrap;
use super::{center, clean, diff, dim, files, pane, title};
use super::{paint, ruler};
use crate::app::{App, Focus};
use crate::highlight::Colors;
use crate::preview::Preview;

fn max_scroll(preview: &Preview, height: usize, cols: usize) -> usize {
    let rows = preview.len();
    let (truncated, row_height): (bool, Box<dyn Fn(usize) -> usize>) = match preview {
        Preview::Text {
            lines,
            colors,
            truncated,
        } => (
            *truncated,
            Box::new(|index| plain(&lines[index], colors.get(index), cols).len()),
        ),
        Preview::Diff { diff, colors } => (
            diff.truncated,
            Box::new(|index| diff::lines(&diff.rows, colors, index, diff.width, cols).len()),
        ),
        _ => return 0,
    };

    let budget = height.saturating_sub(usize::from(truncated));
    let mut used = 0;
    let mut start = rows;
    while start > 0 {
        let next = row_height(start - 1);
        if used + next > budget {
            break;
        }

        used += next;
        start -= 1;
    }

    start.min(rows.saturating_sub(1))
}

pub fn draw(frame: &mut Frame, app: &mut App, area: Rect, name: &str) {
    let room = usize::from(area.height.saturating_sub(2));
    let max = max_scroll(
        &app.preview,
        room,
        usize::from(area.width.saturating_sub(2)),
    );
    app.preview_scroll = app.preview_scroll.min(max);

    let heading = match &app.preview {
        Preview::Diff { diff, .. } if !name.is_empty() => {
            let view = match app.context {
                Context::Full => "full",
                Context::Hunks => "diff",
            };
            format!("{name}  +{} −{}  {view}", diff.adds, diff.dels)
        }
        _ => name.to_owned(),
    };
    let heading = format!(
        "{heading}{}",
        ruler::summary(&app.preview, app.preview_scroll, max)
    );
    let block = pane(title(&heading, area.width), app.focus == Focus::Preview);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.preview_height = usize::from(inner.height);

    ruler::draw(
        frame,
        area,
        inner,
        &app.preview,
        app.preview_scroll,
        max,
        app.focus == Focus::Preview,
    );

    let skip = app.preview_scroll;
    let height = usize::from(inner.height);
    let cols = usize::from(inner.width);
    match &app.preview {
        Preview::Empty => {}
        Preview::Message(text) => center(frame, inner, text),
        Preview::Text {
            lines,
            colors,
            truncated,
        } => {
            let shown = fill((0..lines.len()).skip(skip), height, *truncated, |index| {
                plain(&lines[index], colors.get(index), cols)
            });
            frame.render_widget(Paragraph::new(shown), inner);
        }
        Preview::Dir(entries) => {
            let base = if app.cwd_rel.is_empty() {
                name.to_owned()
            } else {
                format!("{}/{name}", app.cwd_rel)
            };
            let items: Vec<ListItem> = files::rows(app, entries, &base, cols)
                .into_iter()
                .map(|spans| ListItem::new(Line::from(spans)))
                .collect();
            frame.render_widget(List::new(items), inner);
        }
        Preview::Diff { diff, colors } => {
            let digits = diff.width;
            let shown = fill(
                (0..diff.rows.len()).skip(skip),
                height,
                diff.truncated,
                |index| diff::lines(&diff.rows, colors, index, digits, cols),
            );
            frame.render_widget(Paragraph::new(shown), inner);
        }
    }
}

fn fill<T>(
    items: impl Iterator<Item = T>,
    height: usize,
    truncated: bool,
    render: impl Fn(T) -> Vec<Line<'static>>,
) -> Vec<Line<'static>> {
    let mut shown = Vec::new();
    for item in items {
        if shown.len() >= height {
            break;
        }

        shown.extend(render(item));
    }

    if shown.len() < height && truncated {
        shown.push(Line::styled("… truncated", dim()));
    }
    shown.truncate(height);

    shown
}

fn plain(text: &str, colors: Option<&Colors>, cols: usize) -> Vec<Line<'static>> {
    let colors = colors.map_or(&[][..], Vec::as_slice);
    let mut offset = 0;

    wrap(&clean(text), cols)
        .into_iter()
        .map(|part| {
            let length = part.chars().count();
            let spans = paint::spans(&part, offset, None, colors, Style::new());
            offset += length;

            Line::from(spans)
        })
        .collect()
}
