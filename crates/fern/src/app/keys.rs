use ratatui::crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Position;

use super::copy::OPTIONS;
use super::nav::Step;
use super::{App, Chord, Focus, Mode};

const STEP: usize = 3;
const SEEK: usize = 5;

impl App {
    pub(super) fn key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }

        self.notice = None;
        self.selection = None;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            self.quit = true;

            return;
        }

        if self.typing() {
            self.filter_key(key);

            return;
        }

        match self.chord.take() {
            Some(Chord::Copy) => {
                if !ctrl {
                    self.copy_key(key.code);
                }

                return;
            }
            Some(Chord::Help) => return,
            None => {}
        }

        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('c') => self.chord = Some(Chord::Copy),
            KeyCode::Char('?') => self.chord = Some(Chord::Help),
            KeyCode::Char('1') => self.set_mode(Mode::Files),
            KeyCode::Char('2') => self.set_mode(Mode::Changes),
            KeyCode::Char('0') => self.read(),
            KeyCode::Tab => self.toggle_mode(),
            KeyCode::Char('f') if !ctrl => self.start_filter(),
            KeyCode::Char('d') if !ctrl => self.toggle_context(),
            KeyCode::Esc if self.focus == Focus::List && self.filtering() => self.clear_filter(),
            KeyCode::Char('R') => self.refresh_all(),
            KeyCode::Char('.') => self.toggle_hidden(),
            KeyCode::Char('n') => self.jump_hunk(true),
            KeyCode::Char('N') => self.jump_hunk(false),
            KeyCode::Char('g') | KeyCode::Home => self.top(),
            KeyCode::Char('G') | KeyCode::End => self.bottom(),
            _ if self.focus == Focus::Preview => self.read_key(key.code, ctrl),
            _ => self.list_key(key.code, ctrl),
        }
    }

    fn copy_key(&mut self, code: KeyCode) {
        let KeyCode::Char(key) = code else {
            return;
        };
        if let Some((_, what, _)) = OPTIONS.iter().find(|(option, ..)| *option == key) {
            self.copy(*what);
        }
    }

    fn top(&mut self) {
        if self.focus == Focus::Preview {
            self.scroll_up(usize::MAX);
        } else {
            self.move_sel(Step::Top);
        }
    }

    fn bottom(&mut self) {
        if self.focus == Focus::Preview {
            self.scroll_down(usize::MAX);
        } else {
            self.move_sel(Step::Bottom);
        }
    }

    fn list_key(&mut self, code: KeyCode, ctrl: bool) {
        let page = self.page();
        match code {
            KeyCode::Up | KeyCode::Char('k') => self.move_sel(Step::Up(1)),
            KeyCode::Down | KeyCode::Char('j') => self.move_sel(Step::Down(1)),
            KeyCode::Char('u') if ctrl => self.move_sel(Step::Up(page / 2)),
            KeyCode::Char('d') if ctrl => self.move_sel(Step::Down(page / 2)),
            KeyCode::PageUp => self.move_sel(Step::Up(page)),
            KeyCode::Char('b') if ctrl => self.move_sel(Step::Up(page)),
            KeyCode::PageDown => self.move_sel(Step::Down(page)),
            KeyCode::Char('f') if ctrl => self.move_sel(Step::Down(page)),
            KeyCode::Char('K') => self.scroll_up(SEEK),
            KeyCode::Char('J') => self.scroll_down(SEEK),
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => self.go_up(),
            KeyCode::Enter if self.mode == Mode::Changes => self.reveal(),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => self.open(),
            _ => {}
        }
    }

    fn read_key(&mut self, code: KeyCode, ctrl: bool) {
        let page = self.page();
        match code {
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(STEP),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(STEP),
            KeyCode::Char('u') if ctrl => self.scroll_up(page / 2),
            KeyCode::Char('d') if ctrl => self.scroll_down(page / 2),
            KeyCode::PageUp => self.scroll_up(page),
            KeyCode::Char('b') if ctrl => self.scroll_up(page),
            KeyCode::PageDown => self.scroll_down(page),
            KeyCode::Char('f') if ctrl => self.scroll_down(page),
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                self.focus = Focus::List;
            }
            _ => {}
        }
    }

    pub(super) fn mouse(&mut self, mouse: MouseEvent) {
        let over_preview = self
            .preview_area
            .contains(Position::new(mouse.column, mouse.row));
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => self.select_press(mouse.column, mouse.row),
            MouseEventKind::Drag(MouseButton::Left) => self.select_drag(mouse.column, mouse.row),
            MouseEventKind::Up(MouseButton::Left) => self.select_release(),
            MouseEventKind::ScrollUp if over_preview => self.scroll_up(STEP),
            MouseEventKind::ScrollDown if over_preview => self.scroll_down(STEP),
            MouseEventKind::ScrollUp => self.move_sel(Step::Up(1)),
            MouseEventKind::ScrollDown => self.move_sel(Step::Down(1)),
            _ => {}
        }
    }
}
