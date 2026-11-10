use ratatui::style::{Color, Style};
use ratatui::text::Span;

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
