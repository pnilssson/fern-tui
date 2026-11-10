use fern_git::Row;
use ratatui::layout::Position;
use unicode_width::UnicodeWidthChar;

use super::App;
use crate::preview::Preview;

const TAB: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Pos {
    row: usize,
    col: usize,
}

#[derive(Clone, Copy)]
pub struct Selection {
    anchor: Pos,
    head: Pos,
    held: bool,
    dragged: bool,
}

pub struct Visual {
    pub row: usize,
    pub start: usize,
    pub gutter: usize,
    pub text: String,
}

impl Selection {
    fn ends(&self) -> (Pos, Pos) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }

    pub fn range(&self, row: usize) -> Option<(usize, usize)> {
        if !self.dragged {
            return None;
        }

        let (first, last) = self.ends();
        if row < first.row || row > last.row {
            return None;
        }

        let from = if row == first.row { first.col } else { 0 };
        let to = if row == last.row {
            last.col.saturating_add(1)
        } else {
            usize::MAX
        };

        Some((from, to))
    }
}

impl Visual {
    fn locate(&self, cell: usize) -> usize {
        let mut used = 0;
        let mut index = 0;
        let inside = cell.saturating_sub(self.gutter);
        for ch in self.text.chars() {
            let width = ch.width().unwrap_or(0);
            if used + width > inside {
                break;
            }

            used += width;
            index += 1;
        }

        self.start + index.min(self.text.chars().count().saturating_sub(1))
    }
}

fn segment(text: &str, from: usize, to: usize) -> String {
    let mut out = String::new();
    let mut at = 0;
    for ch in text.chars() {
        let width = match ch {
            '\t' => TAB,
            c if c.is_control() => 0,
            _ => 1,
        };
        if width > 0 && at + width > from && at < to {
            out.push(ch);
        }

        at += width;
    }

    out
}

fn copied(preview: &Preview, selection: &Selection) -> Option<(String, usize)> {
    let (first, last) = selection.ends();
    let cut = |row: usize, text: &str| {
        let (from, to) = selection.range(row)?;

        Some(segment(text, from, to))
    };
    let picked: Vec<String> = match preview {
        Preview::Text { lines, .. } => (first.row..=last.row)
            .map(|row| cut(row, lines.get(row)?))
            .collect::<Option<_>>()?,
        Preview::Diff { diff, .. } => {
            let rows = (first.row..=last.row).zip(diff.rows.get(first.row..=last.row)?);
            let new_side: Vec<String> = rows
                .clone()
                .filter_map(|(row, kind)| match kind {
                    Row::Context(_, _, text) | Row::Add(_, text) => cut(row, text),
                    _ => None,
                })
                .collect();
            if new_side.is_empty() {
                rows.filter_map(|(row, kind)| match kind {
                    Row::Del(_, text) => cut(row, text),
                    _ => None,
                })
                .collect()
            } else {
                new_side
            }
        }
        _ => return None,
    };

    Some((picked.join("\n"), picked.len()))
}

impl App {
    fn position(&self, column: u16, row: u16) -> Option<Pos> {
        let inner = self.preview_inner;
        let visual = self
            .preview_rows
            .get(usize::from(row.checked_sub(inner.y)?))?;

        Some(Pos {
            row: visual.row,
            col: visual.locate(usize::from(column.saturating_sub(inner.x))),
        })
    }

    pub(super) fn select_press(&mut self, column: u16, row: u16) {
        self.selection = None;
        if !self.preview_inner.contains(Position::new(column, row)) {
            return;
        }

        let Some(at) = self.position(column, row) else {
            return;
        };
        self.selection = Some(Selection {
            anchor: at,
            head: at,
            held: true,
            dragged: false,
        });
    }

    pub(super) fn select_drag(&mut self, column: u16, row: u16) {
        if !self.selection.is_some_and(|selection| selection.held) {
            return;
        }

        let inner = self.preview_inner;
        let (Some(first), Some(last)) = (self.preview_rows.first(), self.preview_rows.last())
        else {
            return;
        };
        let end = self.preview.len().saturating_sub(1);
        let after = |row| Pos {
            row,
            col: usize::MAX,
        };
        let head = if row < inner.y {
            let before = Pos {
                row: first.row.saturating_sub(1),
                col: 0,
            };
            self.scroll_up(1);

            before
        } else if row >= inner.bottom() {
            let next = after((last.row + 1).min(end));
            self.scroll_down(1);

            next
        } else {
            self.position(column, row).unwrap_or(after(last.row))
        };

        if let Some(selection) = &mut self.selection {
            selection.head = head;
            selection.dragged = true;
        }
    }

    pub(super) fn select_release(&mut self) {
        let Some(selection) = &mut self.selection else {
            return;
        };
        if !selection.held {
            return;
        }

        selection.held = false;
        if !selection.dragged {
            self.selection = None;

            return;
        }

        let selection = *selection;
        match copied(&self.preview, &selection) {
            Some((text, lines)) if !text.is_empty() => {
                let chars = text.chars().count();
                let done = if lines > 1 {
                    format!("Copied {lines} lines")
                } else if chars == 1 {
                    "Copied 1 character".to_owned()
                } else {
                    format!("Copied {chars} characters")
                };
                self.put(&text, done);
            }
            _ => self.say("Nothing to copy".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use fern_git::{Diff, Row};

    use super::{Pos, Selection, Visual, copied, segment};
    use crate::preview::Preview;

    fn select(from: (usize, usize), to: (usize, usize)) -> Selection {
        Selection {
            anchor: Pos {
                row: from.0,
                col: from.1,
            },
            head: Pos {
                row: to.0,
                col: to.1,
            },
            held: false,
            dragged: true,
        }
    }

    fn text() -> Preview {
        Preview::Text {
            lines: vec!["alpha".to_owned(), "beta".to_owned(), "gamma".to_owned()],
            colors: Vec::new(),
            truncated: false,
        }
    }

    #[test]
    fn copies_part_of_a_line() {
        let selection = select((0, 1), (0, 3));

        assert_eq!(copied(&text(), &selection), Some(("lph".to_owned(), 1)));
    }

    #[test]
    fn copies_across_lines_in_either_direction() {
        let selection = select((2, 1), (0, 3));

        assert_eq!(
            copied(&text(), &selection),
            Some(("ha\nbeta\nga".to_owned(), 3))
        );
    }

    #[test]
    fn diff_copies_the_new_side_and_falls_back_to_removed_lines() {
        let rows = vec![
            Row::Hunk("@@".to_owned()),
            Row::Context(1, 1, "keep".to_owned()),
            Row::Del(2, "old".to_owned()),
            Row::Add(2, "new".to_owned()),
        ];
        let preview = Preview::Diff {
            diff: Diff::new(rows, false),
            colors: Vec::new(),
        };

        let all = select((0, 0), (3, usize::MAX - 1));
        assert_eq!(copied(&preview, &all), Some(("keep\nnew".to_owned(), 2)));
        let removed = select((2, 0), (2, 1));
        assert_eq!(copied(&preview, &removed), Some(("ol".to_owned(), 1)));
    }

    #[test]
    fn segment_maps_tab_expanded_columns_back_to_the_source() {
        assert_eq!(segment("a\tb", 0, 1), "a");
        assert_eq!(segment("a\tb", 1, 5), "\t");
        assert_eq!(segment("a\tb", 5, 6), "b");
    }

    #[test]
    fn clicks_in_the_gutter_or_past_the_end_stay_on_the_text() {
        let visual = Visual {
            row: 0,
            start: 10,
            gutter: 4,
            text: "abc".to_owned(),
        };

        assert_eq!(visual.locate(1), 10);
        assert_eq!(visual.locate(5), 11);
        assert_eq!(visual.locate(40), 12);
    }
}
