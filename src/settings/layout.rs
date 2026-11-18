use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Paragraph, Wrap},
};

use super::{
    appearance,
    config::Config,
    home_dir,
    logic::{Page, Settings},
};
use std::path::Path;

pub fn render(frame: &mut Frame, state: &Settings, config: &Config, current_dir: &Path) {
    let [sidebar, content] =
        Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
            .areas(frame.area());
    let left = Block::bordered().title(if state.right_focused {
        " Settings "
    } else {
        " Settings * "
    });
    let left_inner = left.inner(sidebar);
    frame.render_widget(left, sidebar);
    let [appearance_area, home_area, _] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .areas(left_inner);
    for (page, label, area) in [
        (Page::Appearance, "Appearance", appearance_area),
        (Page::HomeDir, "Home Dir", home_area),
    ] {
        let selected = !state.right_focused && state.selected == page;
        let block = if selected {
            Block::bordered()
                .style(config.selection_style())
                .border_style(Style::default().fg(config.line_color()))
        } else {
            Block::bordered()
        };
        frame.render_widget(Paragraph::new(label).block(block), area);
    }

    let right = Block::bordered().title(if state.right_focused {
        " Details * | Tab: focus | Esc: back "
    } else {
        " Details | Tab: focus | Esc: back "
    });
    let inner = right.inner(content);
    frame.render_widget(right, content);
    let [body, message] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(2)]).areas(inner);
    match state.active {
        Some(Page::Appearance) => {
            appearance::layout::render(frame, body, &state.appearance, config, state.right_focused)
        }
        Some(Page::HomeDir) => {
            home_dir::layout::render(frame, body, &state.home_dir, config, current_dir);
        }
        None => frame.render_widget(Paragraph::new("Select an option and press Enter."), body),
    }
    if let Some(text) = &state.message {
        frame.render_widget(
            Paragraph::new(text.as_str()).wrap(Wrap { trim: false }),
            message,
        );
    }
}
