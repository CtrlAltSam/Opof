use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Paragraph, Wrap},
};

use super::logic::Editor;
use crate::home::layout::display_path;

pub fn render(frame: &mut Frame, editor: &mut Editor) {
    let title = format!(
        " {}{} ",
        display_path(&editor.path),
        if editor.dirty() { " *" } else { "" }
    );
    let block = Block::bordered()
        .title_top(title)
        .title_bottom(" Ctrl+S: save | Esc: back | Ctrl+Q: quit ");
    let inner = block.inner(frame.area());
    frame.render_widget(block, frame.area());
    let [body, status] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(2)]).areas(inner);

    let lines = editor.lines();
    let row = lines
        .iter()
        .position(|&(start, end)| editor.cursor >= start && editor.cursor <= end)
        .expect("cursor must be on a line");
    let (start, _) = lines[row];
    let cursor_text = editor.text[start..editor.cursor].replace('\t', "    ");
    let column = Line::from(cursor_text).width();
    if body.height > 0 {
        editor.scroll_row = editor.scroll_row.min(row);
        editor.scroll_row = editor
            .scroll_row
            .max(row.saturating_sub(usize::from(body.height) - 1));
    }
    if body.width > 0 {
        if column < editor.scroll_column {
            editor.scroll_column = column.saturating_sub(usize::from(body.width) - 1);
        }
        editor.scroll_column = editor
            .scroll_column
            .max(column.saturating_sub(usize::from(body.width) - 1));
    }
    for (visible_row, &(start, end)) in lines
        .iter()
        .skip(editor.scroll_row)
        .take(usize::from(body.height))
        .enumerate()
    {
        let text = editor.text[start..end].replace('\t', "    ");
        frame.render_widget(
            Paragraph::new(text).scroll((0, editor.scroll_column.min(u16::MAX as usize) as u16)),
            Rect::new(body.x, body.y + visible_row as u16, body.width, 1),
        );
    }
    let mut status_lines = Vec::new();
    if let Some(prompt) = editor.exit_prompt() {
        status_lines.push(Line::from(prompt));
    }
    if let Some(message) = &editor.message {
        status_lines.push(Line::from(message.as_str()));
    }
    frame.render_widget(
        Paragraph::new(status_lines).wrap(Wrap { trim: false }),
        status,
    );
    if body.width > 0 && body.height > 0 {
        frame.set_cursor_position((
            body.x + (column - editor.scroll_column).min(usize::from(body.width - 1)) as u16,
            body.y + (row - editor.scroll_row) as u16,
        ));
    }
}
