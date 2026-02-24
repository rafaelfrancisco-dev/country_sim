use ratatui::style::{Color, Style};

pub(super) const TEXT_PRIMARY: Color = Color::Rgb(230, 234, 242);
pub(super) const TEXT_MUTED: Color = Color::Rgb(166, 177, 196);
pub(super) const BORDER_NEUTRAL: Color = Color::Rgb(91, 105, 129);
pub(super) const BORDER_ACTIVE: Color = Color::Rgb(88, 166, 255);
pub(super) const PANEL_BG: Color = Color::Rgb(18, 24, 38);
pub(super) const HIGHLIGHT_BG: Color = Color::Rgb(47, 102, 186);

pub(super) fn primary_text() -> Style {
    Style::default().fg(TEXT_PRIMARY)
}

pub(super) fn muted_text() -> Style {
    Style::default().fg(TEXT_MUTED)
}

pub(super) fn neutral_border() -> Style {
    Style::default().fg(BORDER_NEUTRAL)
}

pub(super) fn active_border() -> Style {
    Style::default().fg(BORDER_ACTIVE)
}
