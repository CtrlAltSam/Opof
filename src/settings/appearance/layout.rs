use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use super::{super::config::Config, logic::Appearance};

pub fn render(frame: &mut Frame, area: Rect, state: &Appearance, config: &Config, focused: bool) {
    let values = [config.background, config.lines];
    let labels = ["Background", "Box lines"];
    let mut lines = vec![Line::from(if state.edit.is_some() {
        "Up/Down: channel | Left/Right: adjust"
    } else {
        "Up/Down: select | Enter: edit"
    })];
    for (index, label) in labels.iter().enumerate() {
        let marker = if focused && index == state.selected {
            ">"
        } else {
            " "
        };
        let [r, g, b] = values[index];
        lines.push(Line::from(vec![
            Span::raw(format!("{marker} {label}: {r}, {g}, {b} ")),
            Span::styled("   ", Style::default().bg(Color::Rgb(r, g, b))),
        ]));
    }
    if let Some(edit) = &state.edit {
        for (index, label) in ["R", "G", "B"].iter().enumerate() {
            let value = edit.rgb[index];
            let filled = usize::from(value) * 16 / 255;
            let marker = if index == edit.channel { ">" } else { " " };
            lines.push(Line::from(format!(
                "{marker} {label} [{}{}] {value:3}",
                "=".repeat(filled),
                "-".repeat(16 - filled),
            )));
        }
        lines.push(Line::from("Shift: +/-16 | Home/End: min/max"));
        lines.push(Line::from("Enter: save | Esc: cancel"));
    }
    let style = if state.edit.is_some() {
        let [r, g, b] = config.background;
        let luminance = u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114;
        Style::default().fg(if luminance > 128_000 {
            Color::Black
        } else {
            Color::White
        })
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(lines)
            .style(style)
            .wrap(Wrap { trim: false }),
        area,
    );
}
