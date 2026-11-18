use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    widgets::{Block, List, ListItem, Paragraph, Row, Table, Wrap},
};

use super::logic::Explorer;
use crate::{home::layout::display_path, settings::config::Config};

pub fn render(frame: &mut Frame, explorer: &mut Explorer, config: &Config) {
    let [sidebar, content] =
        Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
            .areas(frame.area());
    let left = Block::bordered().title(if explorer.files_focused {
        " Favorites "
    } else {
        " Favorites * "
    });
    let left_inner = left.inner(sidebar);
    frame.render_widget(left, sidebar);
    let favorites = config.favorites.iter().map(|path| {
        let name = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy();
        ListItem::new(format!("📁 {name}"))
    });
    let list = List::new(favorites).highlight_style(if explorer.files_focused {
        Style::default()
    } else {
        config.selection_style()
    });
    frame.render_stateful_widget(list, left_inner, &mut explorer.favorites);

    let title = format!(
        " {}{} ",
        display_path(&explorer.directory),
        if explorer.files_focused { " *" } else { "" }
    );
    let right = Block::bordered()
        .title_top(title)
        .title_bottom(" Tab: focus | Enter: open | Backspace: up | Ctrl+D: favorite | Esc: back ");
    let inner = right.inner(content);
    frame.render_widget(right, content);
    let [body, message] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(2)]).areas(inner);
    let rows = explorer.entries.iter().map(|entry| {
        let icon = if entry.directory { "📁" } else { "📄" };
        Row::new(vec![
            format!("{icon} {}", entry.name),
            entry.modified.clone(),
            entry.kind.clone(),
            entry.size.clone(),
        ])
    });
    let header = Row::new(["Name", "Modified (UTC)", "Type", "Size (bytes)"])
        .style(Style::default().add_modifier(Modifier::BOLD));
    let table = Table::new(
        rows,
        [
            Constraint::Min(12),
            Constraint::Length(16),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .column_spacing(1)
    .row_highlight_style(if explorer.files_focused {
        config.selection_style()
    } else {
        Style::default()
    });
    frame.render_stateful_widget(table, body, &mut explorer.files);
    if let Some(text) = &explorer.message {
        frame.render_widget(
            Paragraph::new(text.as_str()).wrap(Wrap { trim: false }),
            message,
        );
    }
}
