use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::{ListState, TableState};
use time::OffsetDateTime;

use crate::settings::config::Config;

pub enum Action {
    None,
    Close,
    OpenFile(PathBuf),
}

pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub directory: bool,
    pub modified: String,
    pub kind: String,
    pub size: String,
}

pub struct Explorer {
    pub directory: PathBuf,
    pub entries: Vec<Entry>,
    pub files: TableState,
    pub favorites: ListState,
    pub files_focused: bool,
    pub message: Option<String>,
}

impl Default for Explorer {
    fn default() -> Self {
        Self {
            directory: PathBuf::new(),
            entries: Vec::new(),
            files: TableState::default(),
            favorites: ListState::default(),
            files_focused: true,
            message: None,
        }
    }
}

impl Explorer {
    pub fn open(&mut self, directory: &Path, config: &Config) {
        self.directory = directory.to_path_buf();
        self.files_focused = true;
        self.favorites = ListState::default();
        self.favorites
            .select((!config.favorites.is_empty()).then_some(0));
        match read_directory(directory) {
            Ok((entries, warning)) => self.set_entries(entries, warning),
            Err(error) => {
                self.entries.clear();
                self.files.select(None);
                self.message = Some(error);
            }
        }
    }

    pub fn refresh(&mut self) {
        let selected = self
            .files
            .selected()
            .and_then(|index| self.entries.get(index))
            .map(|entry| entry.path.clone());
        match read_directory(&self.directory) {
            Ok((entries, warning)) => {
                let index =
                    selected.and_then(|path| entries.iter().position(|entry| entry.path == path));
                self.entries = entries;
                self.files
                    .select(index.or_else(|| (!self.entries.is_empty()).then_some(0)));
                self.message = warning;
            }
            Err(error) => self.message = Some(error),
        }
    }

    fn set_entries(&mut self, entries: Vec<Entry>, warning: Option<String>) {
        self.entries = entries;
        self.files = TableState::default();
        self.files.select((!self.entries.is_empty()).then_some(0));
        self.message = warning;
    }

    fn navigate(&mut self, path: &Path) {
        let result = fs::canonicalize(path)
            .map_err(|error| format!("Could not open {}: {error}", path.display()))
            .and_then(|path| read_directory(&path).map(|listing| (path, listing)));
        match result {
            Ok((path, (entries, warning))) => {
                self.directory = path;
                self.set_entries(entries, warning);
            }
            Err(error) => self.message = Some(error),
        }
    }

    fn toggle_favorite(&mut self, config: &mut Config, config_path: &Path) {
        let path = match fs::canonicalize(&self.directory) {
            Ok(path) => path,
            Err(error) => {
                self.message = Some(format!(
                    "Could not favorite {}: {error}",
                    self.directory.display()
                ));
                return;
            }
        };
        let mut updated = config.clone();
        let removed = if let Some(index) = updated.favorites.iter().position(|item| item == &path) {
            updated.favorites.remove(index);
            true
        } else {
            updated.favorites.push(path);
            false
        };
        match updated.save(config_path) {
            Ok(()) => {
                *config = updated;
                self.favorites.select(if config.favorites.is_empty() {
                    None
                } else {
                    Some(
                        self.favorites
                            .selected()
                            .unwrap_or(0)
                            .min(config.favorites.len() - 1),
                    )
                });
                self.message = Some(if removed {
                    "Removed from favorites.".into()
                } else {
                    "Added to favorites.".into()
                });
            }
            Err(error) => self.message = Some(format!("Could not save favorites: {error:#}")),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, config: &mut Config, config_path: &Path) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('d') {
            self.toggle_favorite(config, config_path);
            return Action::None;
        }
        match key.code {
            KeyCode::Esc => return Action::Close,
            KeyCode::Tab | KeyCode::BackTab => self.files_focused = !self.files_focused,
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End => {
                let (selected, len) = if self.files_focused {
                    (self.files.selected(), self.entries.len())
                } else {
                    (self.favorites.selected(), config.favorites.len())
                };
                let next = if len == 0 {
                    None
                } else {
                    let current = selected.unwrap_or(0).min(len - 1);
                    Some(match key.code {
                        KeyCode::Up => current.saturating_sub(1),
                        KeyCode::Down => (current + 1).min(len - 1),
                        KeyCode::Home => 0,
                        KeyCode::End => len - 1,
                        _ => unreachable!(),
                    })
                };
                if self.files_focused {
                    self.files.select(next);
                } else {
                    self.favorites.select(next);
                }
            }
            KeyCode::Enter => {
                let target = if self.files_focused {
                    self.files
                        .selected()
                        .and_then(|index| self.entries.get(index))
                        .map(|entry| (entry.path.clone(), entry.directory))
                } else {
                    self.favorites
                        .selected()
                        .and_then(|index| config.favorites.get(index))
                        .cloned()
                        .map(|path| (path, true))
                };
                if let Some((path, directory)) = target {
                    if directory {
                        self.navigate(&path);
                    } else {
                        return Action::OpenFile(path);
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(parent) = self.directory.parent().map(Path::to_path_buf) {
                    self.navigate(&parent);
                }
            }
            _ => {}
        }
        Action::None
    }
}

fn read_directory(directory: &Path) -> Result<(Vec<Entry>, Option<String>), String> {
    let iter = fs::read_dir(directory)
        .map_err(|error| format!("Could not list {}: {error}", directory.display()))?;
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for item in iter {
        let item =
            item.map_err(|error| format!("Could not list {}: {error}", directory.display()))?;
        let path = item.path();
        let name = item.file_name().to_string_lossy().into_owned();
        let entry = match fs::metadata(&path) {
            Ok(metadata) => {
                let directory = metadata.is_dir();
                let modified = match metadata
                    .modified()
                    .map_err(|error| error.to_string())
                    .and_then(format_time)
                {
                    Ok(value) => value,
                    Err(error) => {
                        warnings.push(format!("{}: {error}", path.display()));
                        "?".into()
                    }
                };
                let kind = if directory {
                    "Folder".into()
                } else if metadata.is_file() {
                    path.extension()
                        .filter(|ext| !ext.is_empty())
                        .map(|ext| format!("{} file", ext.to_string_lossy().to_uppercase()))
                        .unwrap_or_else(|| "File".into())
                } else {
                    "Other".into()
                };
                Entry {
                    path,
                    name,
                    directory,
                    modified,
                    kind,
                    size: if directory {
                        "-".into()
                    } else {
                        format!("{}", metadata.len())
                    },
                }
            }
            Err(error) => {
                warnings.push(format!("{}: {error}", path.display()));
                Entry {
                    path,
                    name,
                    directory: false,
                    modified: "?".into(),
                    kind: "Unavailable".into(),
                    size: "?".into(),
                }
            }
        };
        entries.push(entry);
    }
    entries.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| a.name.cmp(&b.name))
    });
    let warning = warnings.first().map(|first| {
        format!(
            "Metadata unavailable for {} item(s): {first}",
            warnings.len()
        )
    });
    Ok((entries, warning))
}

fn format_time(value: SystemTime) -> Result<String, String> {
    let nanos = match value.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos() as i128,
        Err(error) => -(error.duration().as_nanos() as i128),
    };
    let date =
        OffsetDateTime::from_unix_timestamp_nanos(nanos).map_err(|error| error.to_string())?;
    Ok(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        date.year(),
        u8::from(date.month()),
        date.day(),
        date.hour(),
        date.minute()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn formats_modification_times_in_utc() {
        assert_eq!(
            format_time(SystemTime::UNIX_EPOCH).unwrap(),
            "1970-01-01 00:00"
        );
        assert_eq!(
            format_time(SystemTime::UNIX_EPOCH - Duration::from_secs(60)).unwrap(),
            "1969-12-31 23:59"
        );
    }
}
