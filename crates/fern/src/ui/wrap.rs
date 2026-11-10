use unicode_width::UnicodeWidthChar;

pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut row = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let cell = ch.width().unwrap_or(0);
        if used + cell > width && !row.is_empty() {
            rows.push(std::mem::take(&mut row));
            used = 0;
        }

        row.push(ch);
        used += cell;
    }
    rows.push(row);

    rows
}
