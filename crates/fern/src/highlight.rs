use std::path::Path;
use std::sync::LazyLock;

use fern_git::Row;
use ratatui::style::Color;
use two_face::re_exports::syntect::easy::HighlightLines;
use two_face::re_exports::syntect::highlighting::{Color as Tint, Theme};
use two_face::re_exports::syntect::parsing::{SyntaxReference, SyntaxSet};
use two_face::theme::EmbeddedThemeName;

use crate::ui::clean;

pub type Colors = Vec<Option<Color>>;

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);
static THEME: LazyLock<Theme> = LazyLock::new(|| {
    two_face::theme::extra()
        .get(EmbeddedThemeName::Ansi)
        .clone()
});

fn syntax(name: &str) -> Option<&'static SyntaxReference> {
    let path = Path::new(name);
    let file = path.file_name()?.to_str()?;
    let extension = path.extension().and_then(|extension| extension.to_str());

    SYNTAXES
        .find_syntax_by_extension(file)
        .or_else(|| extension.and_then(|extension| SYNTAXES.find_syntax_by_extension(extension)))
}

fn color(tint: Tint) -> Option<Color> {
    if tint.a != 0 {
        return None;
    }

    Some(match tint.r {
        0 => return None,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        index => Color::Indexed(index),
    })
}

fn paint(state: &mut HighlightLines<'static>, text: &str) -> Colors {
    let line = format!("{text}\n");
    let Ok(tokens) = state.highlight_line(&line, &SYNTAXES) else {
        return vec![None; text.chars().count()];
    };

    let mut colors = Colors::with_capacity(text.len());
    for (style, token) in tokens {
        let fg = color(style.foreground);
        colors.extend(token.chars().filter(|&ch| ch != '\n').map(|_| fg));
    }

    colors
}

pub fn text(name: &str, lines: &[String]) -> Vec<Colors> {
    let Some(syntax) = syntax(name) else {
        return Vec::new();
    };

    let mut state = HighlightLines::new(syntax, &THEME);

    lines
        .iter()
        .map(|line| paint(&mut state, &clean(line)))
        .collect()
}

pub fn rows(name: &str, rows: &[Row]) -> Vec<Colors> {
    let Some(syntax) = syntax(name) else {
        return Vec::new();
    };

    let mut old = HighlightLines::new(syntax, &THEME);
    let mut new = HighlightLines::new(syntax, &THEME);

    rows.iter()
        .map(|row| match row {
            Row::Context(_, _, text) => {
                let text = clean(text);
                paint(&mut old, &text);

                paint(&mut new, &text)
            }
            Row::Add(_, text) => paint(&mut new, &clean(text)),
            Row::Del(_, text) => paint(&mut old, &clean(text)),
            Row::Meta(_) | Row::Hunk(_) => Vec::new(),
        })
        .collect()
}
