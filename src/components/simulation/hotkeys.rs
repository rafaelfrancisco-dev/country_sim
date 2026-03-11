use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) const GLOBAL_RUNNING_HINT: &str = "[Tab/Shift+Tab] switch tab  [Space] pause/resume  [n] step day (paused)  [m] toggle sim mode  [Shift+S] save  [q] quit";

pub(super) fn is_char(key: KeyEvent, expected: char) -> bool {
    match key.code {
        KeyCode::Char(c) => key.modifiers.is_empty() && c.eq_ignore_ascii_case(&expected),
        _ => false,
    }
}

pub(super) fn is_country_prev_key(key: KeyEvent) -> bool {
    is_char(key, ',')
}

pub(super) fn is_country_next_key(key: KeyEvent) -> bool {
    is_char(key, '.')
}

pub(super) fn is_page_prev_key(key: KeyEvent) -> bool {
    is_char(key, 'u')
}

pub(super) fn is_page_next_key(key: KeyEvent) -> bool {
    is_char(key, 'i')
}

pub(super) fn is_ctrl_u(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('u')) && key.modifiers.contains(KeyModifiers::CONTROL)
}
