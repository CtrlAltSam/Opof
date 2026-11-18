use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use super::logic::Home;
use std::path::Path;

pub fn display_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    #[cfg(windows)]
    if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{path}");
    } else if let Some(path) = path.strip_prefix(r"\\?\") {
        return path.to_string();
    }
    path.into_owned()
}

pub fn render(frame: &mut Frame, home: &mut Home) {
    let [main, prompt] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(frame.area());

    let mut lines = Vec::new();
    if let Some(message) = &home.message {
        lines.push(Line::from(message.as_str()));
    }
    lines.extend(home.history.iter().map(|line| Line::from(line.as_str())));
    let directory = display_path(&home.directory);
    let block = Block::bordered()
        .title_top(format!(" {directory} "))
        .title_bottom(" Ctrl+S: settings | Ctrl+Q: quit ");
    let inner = block.inner(main);
    frame.render_widget(block, main);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    let total_lines = paragraph.line_count(inner.width);
    home.max_scroll = total_lines.saturating_sub(usize::from(inner.height));
    home.scroll_from_bottom = home.scroll_from_bottom.min(home.max_scroll);
    let offset = home.max_scroll - home.scroll_from_bottom;
    frame.render_widget(
        paragraph.scroll((offset.min(u16::MAX as usize) as u16, 0)),
        inner,
    );
    if home.max_scroll > 0 && inner.height > 0 {
        let mut state = ScrollbarState::new(home.max_scroll).position(offset);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            Rect::new(main.x, inner.y, main.width, inner.height),
            &mut state,
        );
    }
    let prompt_inner = Block::bordered().inner(prompt);
    let input = format!("> {}", home.input);
    let input_width = Line::from(input.as_str()).width();
    let horizontal_scroll = input_width.saturating_sub(usize::from(prompt_inner.width));
    frame.render_widget(
        Paragraph::new(input)
            .scroll((0, horizontal_scroll.min(u16::MAX as usize) as u16))
            .block(Block::bordered()),
        prompt,
    );
}
