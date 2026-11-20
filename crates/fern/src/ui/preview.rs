use std::collections::HashMap;

use fern_git::Context;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui_image::{Resize, StatefulImage};

use super::wrap::wrap;
use super::{center, clean, diff, dim, files, pane, title};
use super::{paint, ruler};
use crate::app::{App, Focus, Hit, Search, Selection, Visual};
use crate::highlight::Colors;
use crate::preview::Preview;

const MATCH: Style = Style::new().fg(Color::Black).bg(Color::Yellow);
const CURRENT: Style = Style::new()
    .fg(Color::Black)
    .bg(Color::Cyan)
    .add_modifier(Modifier::BOLD);

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
    let mut block = pane(title(&heading, area.width), app.focus == Focus::Preview);
    if let Some(search) = &app.search {
        block = block.title_bottom(bar(search));
    }
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

    if let Preview::Image(protocol) = &mut app.preview {
        let image = StatefulImage::default().resize(Resize::Fit(None));
        frame.render_stateful_widget(image, inner, protocol.as_mut());
        app.preview_inner = inner;
        app.preview_rows = Vec::new();

        return;
    }

    let skip = app.preview_scroll;
    let height = usize::from(inner.height);
    let cols = usize::from(inner.width);
    let selection = app.selection;
    let search = app.search.as_ref();
    let rows = match &app.preview {
        Preview::Empty | Preview::Image(_) => Vec::new(),
        Preview::Message(text) => {
            center(frame, inner, text);
            Vec::new()
        }
        Preview::Text {
            lines,
            colors,
            truncated,
        } => {
            let (shown, rows) = fill(
                (0..lines.len()).skip(skip),
                (height, *truncated),
                (selection, search),
                (|index| plain(&lines[index], colors.get(index), cols), |_| 0),
            );
            frame.render_widget(Paragraph::new(shown), inner);

            rows
        }
        Preview::Dir(entries) => {
            let base = if app.cwd_rel.is_empty() {
                name.to_owned()
            } else {
                format!("{}/{name}", app.cwd_rel)
            };
            let items: Vec<ListItem> = files::rows(app, entries, &base, &HashMap::new(), cols)
                .into_iter()
                .map(|spans| ListItem::new(Line::from(spans)))
                .collect();
            frame.render_widget(List::new(items), inner);

            Vec::new()
        }
        Preview::Diff { diff, colors } => {
            let digits = diff.width;
            let (shown, rows) = fill(
                (0..diff.rows.len()).skip(skip),
                (height, diff.truncated),
                (selection, search),
                (
                    |index| diff::lines(&diff.rows, colors, index, digits, cols),
                    |index| diff::lead(&diff.rows, index),
                ),
            );
            frame.render_widget(Paragraph::new(shown), inner);

            rows
        }
    };
    app.preview_inner = inner;
    app.preview_rows = rows;
}

fn fill(
    items: impl Iterator<Item = usize>,
    (height, truncated): (usize, bool),
    (selection, search): (Option<Selection>, Option<&Search>),
    (render, lead): (
        impl Fn(usize) -> Vec<Line<'static>>,
        impl Fn(usize) -> usize,
    ),
) -> (Vec<Line<'static>>, Vec<Visual>) {
    let mut shown = Vec::new();
    let mut visuals = Vec::new();
    for index in items {
        if shown.len() >= height {
            break;
        }

        let lead = lead(index);
        let range = selection.and_then(|selection| selection.range(index));
        let hits = search.map_or(&[][..], |search| search.row(index));
        let current = search.and_then(|search| search.hits.get(search.current));
        let mut start = 0;
        for line in render(index) {
            let text: String = line
                .spans
                .iter()
                .skip(lead)
                .map(|span| span.content.as_ref())
                .collect();
            let length = text.chars().count();
            let gutter = line.spans.iter().take(lead).map(Span::width).sum();
            visuals.push(Visual {
                row: index,
                start,
                gutter,
                text,
            });
            let line = lit(line, lead, (start, length), (hits, current));
            shown.push(match range {
                Some((from, to)) => paint::patch(
                    line,
                    lead,
                    (from.saturating_sub(start), to.saturating_sub(start)),
                    Style::new().add_modifier(Modifier::REVERSED),
                ),
                None => line,
            });
            start += length;
        }
    }

    if shown.len() < height && truncated {
        shown.push(Line::styled("… truncated", dim()));
    }
    shown.truncate(height);
    visuals.truncate(height);

    (shown, visuals)
}

fn lit(
    mut line: Line<'static>,
    lead: usize,
    (start, length): (usize, usize),
    (hits, current): (&[Hit], Option<&Hit>),
) -> Line<'static> {
    for hit in hits
        .iter()
        .filter(|hit| hit.to > start && hit.from < start + length)
    {
        let style = if Some(hit) == current { CURRENT } else { MATCH };
        line = paint::patch(
            line,
            lead,
            (hit.from.saturating_sub(start), hit.to - start),
            style,
        );
    }

    line
}

fn bar(search: &Search) -> Line<'static> {
    let mut spans = vec![
        Span::styled(
            format!(" /{}", search.query),
            Style::new().add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::new().add_modifier(Modifier::REVERSED)),
    ];
    if !search.query.is_empty() {
        spans.push(if search.hits.is_empty() {
            Span::styled("  no matches", Style::new().fg(Color::Red))
        } else {
            Span::styled(
                format!("  {}/{}", search.current + 1, search.hits.len()),
                dim(),
            )
        });
    }
    spans.push(Span::raw(" "));

    Line::from(spans)
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
