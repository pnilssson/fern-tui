use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

type Key = (bool, Option<Color>);

pub fn spans(
    part: &str,
    offset: usize,
    marks: Option<&[bool]>,
    colors: &[Option<Color>],
    strong: Style,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut key: Key = (false, None);
    for (index, ch) in part.chars().enumerate() {
        let at = offset + index;
        let next: Key = (
            marks
                .and_then(|marks| marks.get(at))
                .copied()
                .unwrap_or(false),
            colors.get(at).copied().flatten(),
        );
        if next != key && !current.is_empty() {
            spans.push(chunk(std::mem::take(&mut current), key, strong));
        }

        key = next;
        current.push(ch);
    }

    if !current.is_empty() {
        spans.push(chunk(current, key, strong));
    }

    spans
}

fn chunk(text: String, (on, color): Key, strong: Style) -> Span<'static> {
    let mut style = if on { strong } else { Style::new() };
    if let Some(color) = color {
        style = style.fg(color);
    }

    Span::styled(text, style)
}

pub fn reverse(line: Line<'static>, lead: usize, from: usize, to: usize) -> Line<'static> {
    let style = line.style;
    let mut spans = Vec::new();
    let mut at = 0;
    for (index, span) in line.spans.into_iter().enumerate() {
        if index < lead {
            spans.push(span);

            continue;
        }

        let chars: Vec<char> = span.content.chars().collect();
        let first = from.saturating_sub(at).min(chars.len());
        let last = to.saturating_sub(at).min(chars.len());
        at += chars.len();
        if first >= last {
            spans.push(span);

            continue;
        }

        let part = |range: std::ops::Range<usize>| chars[range].iter().collect::<String>();
        let pieces = [
            (part(0..first), span.style),
            (
                part(first..last),
                span.style.add_modifier(Modifier::REVERSED),
            ),
            (part(last..chars.len()), span.style),
        ];
        spans.extend(
            pieces
                .into_iter()
                .filter(|(text, _)| !text.is_empty())
                .map(|(text, style)| Span::styled(text, style)),
        );
    }

    Line::from(spans).style(style)
}
