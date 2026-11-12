use super::{Focus, Mode};

type Binding = (&'static str, &'static str);

const FILES: [Binding; 14] = [
    ("↑ ↓  j k", "Move"),
    ("→ l ⏎", "Open folder or read file"),
    ("← h ⌫", "Go up a folder"),
    ("g G", "Top / bottom"),
    ("ctrl-u ctrl-d", "Half page up / down"),
    ("J K", "Scroll the preview"),
    ("f", "Filter this folder"),
    ("d", "Full file / diff"),
    (".", "Show hidden files"),
    ("R", "Refresh"),
    ("tab 1 2", "Switch Files / Changes"),
    ("drag", "Select text, copy on release"),
    ("c", "Copy path menu"),
    ("q", "Quit"),
];

const CHANGES: [Binding; 13] = [
    ("↑ ↓  j k", "Move"),
    ("→ l", "Read the change"),
    ("⏎", "Reveal in Files"),
    ("n N", "Next / previous change"),
    ("g G", "Top / bottom"),
    ("J K", "Scroll the preview"),
    ("f", "Filter the changes"),
    ("d", "Full file / diff"),
    ("R", "Refresh"),
    ("tab 1 2", "Switch Files / Changes"),
    ("drag", "Select text, copy on release"),
    ("c", "Copy path menu"),
    ("q", "Quit"),
];

const PREVIEW: [Binding; 9] = [
    ("↑ ↓  j k", "Scroll"),
    ("ctrl-u ctrl-d", "Half page up / down"),
    ("g G", "Top / bottom"),
    ("n N", "Next / previous change"),
    ("d", "Full file / diff"),
    ("← h esc", "Back to the list"),
    ("drag", "Select text, copy on release"),
    ("c", "Copy path menu"),
    ("q", "Quit"),
];

pub(super) fn bindings(mode: Mode, focus: Focus) -> &'static [Binding] {
    match (focus, mode) {
        (Focus::Preview, _) => &PREVIEW,
        (Focus::List, Mode::Files) => &FILES,
        (Focus::List, Mode::Changes) => &CHANGES,
    }
}
