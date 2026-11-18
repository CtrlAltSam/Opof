use std::path::Path;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Paragraph, Wrap},
};

use super::{super::config::Config, logic::HomeDir};

pub fn render(frame: &mut Frame, area: Rect, state: &HomeDir, config: &Config, current_dir: &Path) {
    let current = config
        .home_dir
        .as_deref()
        .unwrap_or(current_dir)
        .to_string_lossy();
    let value = state.input.as_deref().unwrap_or(&current);
    let [help, input, hint] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new("Enter: edit/save | Applies on next launch"),
        help,
    );
    let line = Line::from(format!("Home directory: {value}"));
    let scroll = if state.input.is_some() {
        line.width()
            .saturating_sub(input.width as usize)
            .min(u16::MAX as usize) as u16
    } else {
        0
    };
    frame.render_widget(Paragraph::new(line).scroll((0, scroll)), input);
    frame.render_widget(
        Paragraph::new("Absolute path | Ctrl+U: clear | Esc: cancel").wrap(Wrap { trim: false }),
        hint,
    );
}
