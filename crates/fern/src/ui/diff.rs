use fern_git::Row;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use similar::{ChangeTag, TextDiff};

use super::paint;
use super::wrap::wrap;
use super::{clean, dim};
use crate::highlight::Colors;

const ADD: Color = Color::Indexed(22);
const ADD_STRONG: Color = Color::Indexed(28);
const DEL: Color = Color::Indexed(52);
const DEL_STRONG: Color = Color::Indexed(88);
const SIMILAR: f32 = 0.5;

#[derive(Clone, Copy)]
struct Look {
    line: Style,
    strong: Style,
    sign: Style,
}

#[derive(Clone, Copy)]
struct Gutter {
    digits: usize,
    old: Option<u32>,
    new: Option<u32>,
}

pub fn lines(
    rows: &[Row],
    colors: &[Colors],
    index: usize,
    digits: usize,
    cols: usize,
) -> Vec<Line<'static>> {
    let Some(row) = rows.get(index) else {
        return Vec::new();
    };
    let tint = colors.get(index).map_or(&[][..], Vec::as_slice);
    let context = Look {
        line: Style::new(),
        strong: Style::new(),
        sign: Style::new(),
    };
    let added = Look {
        line: Style::new().bg(ADD),
        strong: Style::new().bg(ADD_STRONG),
        sign: Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
    };
    let removed = Look {
        line: Style::new().bg(DEL),
        strong: Style::new().bg(DEL_STRONG),
        sign: Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    match row {
        Row::Meta(text) => notice(
            text,
            cols,
            Style::new()
                .fg(Color::Yellow)
                .add_modifier(Modifier::ITALIC),
        ),
        Row::Hunk(text) => notice(
            text,
            cols,
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Row::Context(old, new, text) => {
            let gutter = Gutter {
                digits,
                old: Some(*old),
                new: Some(*new),
            };
            numbered(gutter, ' ', text, (None, tint), cols, context)
        }
        Row::Add(number, text) => {
            let gutter = Gutter {
                digits,
                old: None,
                new: Some(*number),
            };
            numbered(
                gutter,
                '+',
                text,
                (emphasis(rows, index).as_deref(), tint),
                cols,
                added,
            )
        }
        Row::Del(number, text) => {
            let gutter = Gutter {
                digits,
                old: Some(*number),
                new: None,
            };
            numbered(
                gutter,
                '-',
                text,
                (emphasis(rows, index).as_deref(), tint),
                cols,
                removed,
            )
        }
    }
}

fn notice(text: &str, cols: usize, style: Style) -> Vec<Line<'static>> {
    wrap(&clean(text), cols)
        .into_iter()
        .map(|part| Line::styled(part, style))
        .collect()
}

fn numbered(
    gutter: Gutter,
    sign: char,
    text: &str,
    (marks, tint): (Option<&[bool]>, &[Option<Color>]),
    cols: usize,
    look: Look,
) -> Vec<Line<'static>> {
    let digits = gutter.digits;
    let cell = |number: Option<u32>| {
        number.map_or_else(|| " ".repeat(digits), |n| format!("{n:>digits$}"))
    };
    let numbers = format!("{} {} ", cell(gutter.old), cell(gutter.new));
    let indent = numbers.chars().count();
    let text = clean(text);

    let mut offset = 0;
    wrap(&text, cols.saturating_sub(indent + 2))
        .into_iter()
        .enumerate()
        .map(|(index, part)| {
            let (head, lead) = if index == 0 {
                (numbers.clone(), format!("{sign} "))
            } else {
                (" ".repeat(indent), "  ".to_owned())
            };
            let length = part.chars().count();
            let mut spans = vec![Span::styled(head, dim()), Span::styled(lead, look.sign)];
            spans.extend(paint::spans(&part, offset, marks, tint, look.strong));
            offset += length;

            Line::from(spans).style(look.line)
        })
        .collect()
}

fn run(rows: &[Row], index: usize) -> (usize, usize) {
    let kind = rows.get(index).map(std::mem::discriminant);
    let same = |at: usize| rows.get(at).map(std::mem::discriminant) == kind;
    let mut start = index;
    while start > 0 && same(start - 1) {
        start -= 1;
    }
    let mut end = index + 1;
    while same(end) {
        end += 1;
    }

    (start, end)
}

fn partner(rows: &[Row], index: usize) -> Option<usize> {
    let (start, end) = run(rows, index);
    match rows.get(index)? {
        Row::Del(..) => {
            if !matches!(rows.get(end), Some(Row::Add(..))) {
                return None;
            }

            let (_, adds_end) = run(rows, end);
            let target = end + (index - start);

            (target < adds_end).then_some(target)
        }
        Row::Add(..) => {
            if start == 0 || !matches!(rows.get(start - 1), Some(Row::Del(..))) {
                return None;
            }

            let (dels_start, _) = run(rows, start - 1);
            let target = dels_start + (index - start);

            (target < start).then_some(target)
        }
        _ => None,
    }
}

fn emphasis(rows: &[Row], index: usize) -> Option<Vec<bool>> {
    let other = partner(rows, index)?;
    let (mine, theirs, adding) = match (rows.get(index)?, rows.get(other)?) {
        (Row::Add(_, added), Row::Del(_, removed)) => (clean(added), clean(removed), true),
        (Row::Del(_, removed), Row::Add(_, added)) => (clean(removed), clean(added), false),
        _ => return None,
    };
    let diff = if adding {
        TextDiff::from_words(theirs.as_str(), mine.as_str())
    } else {
        TextDiff::from_words(mine.as_str(), theirs.as_str())
    };
    if diff.ratio() < SIMILAR {
        return None;
    }

    let wanted = if adding {
        ChangeTag::Insert
    } else {
        ChangeTag::Delete
    };
    let mut marks = Vec::new();
    for change in diff.iter_all_changes() {
        let tag = change.tag();
        if tag == ChangeTag::Equal || tag == wanted {
            let on = tag == wanted;
            marks.extend(change.value().chars().map(|_| on));
        }
    }

    Some(marks)
}
