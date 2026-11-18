use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Exit {
    Close,
    Quit,
}

pub enum Action {
    None,
    Close,
    Quit,
}

pub struct Editor {
    pub path: PathBuf,
    pub text: String,
    saved: String,
    pub cursor: usize,
    pub scroll_row: usize,
    pub scroll_column: usize,
    pub message: Option<String>,
    pending: Option<Exit>,
}

impl Editor {
    pub fn open(path: &Path, create: bool) -> Result<Self, String> {
        match fs::metadata(path) {
            Ok(metadata) if !metadata.is_file() => {
                return Err(format!("{} is not a regular file", path.display()));
            }

            Ok(_) => {}
            Err(error) if create && error.kind() == io::ErrorKind::NotFound => {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                    .map_err(|error| format!("Could not create {}: {error}", path.display()))?;
            }
            Err(error) => return Err(format!("Could not open {}: {error}", path.display())),
        }
        let path = fs::canonicalize(path)
            .map_err(|error| format!("Could not open {}: {error}", path.display()))?;
        let bytes = fs::read(&path)
            .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
        let text = String::from_utf8(bytes)
            .map_err(|error| format!("{} is not UTF-8 text: {error}", path.display()))?;
        if text
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
        {
            return Err(format!(
                "{} contains unsupported control characters",
                path.display()
            ));
        }
        Ok(Self {
            path,
            saved: text.clone(),
            text,
            cursor: 0,
            scroll_row: 0,
            scroll_column: 0,
            message: None,
            pending: None,
        })
    }

    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }

    pub fn exit_prompt(&self) -> Option<&'static str> {
        self.pending
            .map(|_| "Unsaved changes: S save | D discard | Esc cancel")
    }

    fn save(&mut self) -> Result<(), String> {
        if !self.dirty() {
            self.message = Some("No changes to save.".into());
            return Ok(());
        }
        let metadata = fs::metadata(&self.path)
            .map_err(|error| format!("Could not save {}: {error}", self.path.display()))?;
        if !metadata.is_file() {
            return Err(format!(
                "Could not save {}: not a regular file",
                self.path.display()
            ));
        }
        let current = fs::read(&self.path)
            .map_err(|error| format!("Could not save {}: {error}", self.path.display()))?;
        if current != self.saved.as_bytes() {
            return Err("File changed on disk; save cancelled to avoid overwriting it.".into());
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "File has no parent directory".to_string())?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)
            .map_err(|error| format!("Could not save {}: {error}", self.path.display()))?;
        temp.write_all(self.text.as_bytes())
            .and_then(|_| temp.as_file().sync_all())
            .and_then(|_| fs::set_permissions(temp.path(), metadata.permissions()))
            .map_err(|error| format!("Could not save {}: {error}", self.path.display()))?;
        temp.persist(&self.path)
            .map_err(|error| format!("Could not save {}: {}", self.path.display(), error.error))?;
        self.saved.clone_from(&self.text);
        self.message = Some("Saved.".into());
        Ok(())
    }

    fn request_exit(&mut self, exit: Exit) -> Action {
        if self.dirty() {
            self.pending = Some(exit);
            self.message = None;
            Action::None
        } else {
            match exit {
                Exit::Close => Action::Close,
                Exit::Quit => Action::Quit,
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if let Some(exit) = self.pending {
            match key.code {
                KeyCode::Esc => {
                    self.pending = None;
                    self.message = None;
                }
                KeyCode::Char('d') | KeyCode::Char('D')
                    if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
                {
                    self.pending = None;
                    return match exit {
                        Exit::Close => Action::Close,
                        Exit::Quit => Action::Quit,
                    };
                }
                KeyCode::Char('s') | KeyCode::Char('S')
                    if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
                {
                    match self.save() {
                        Ok(()) => {
                            self.pending = None;
                            return match exit {
                                Exit::Close => Action::Close,
                                Exit::Quit => Action::Quit,
                            };
                        }
                        Err(error) => self.message = Some(error),
                    }
                }
                _ => {}
            }
            return Action::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') => {
                    if let Err(error) = self.save() {
                        self.message = Some(error);
                    }
                }
                KeyCode::Char('q') => return self.request_exit(Exit::Quit),
                _ => {}
            }
            return Action::None;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            return Action::None;
        }
        match key.code {
            KeyCode::Esc => return self.request_exit(Exit::Close),
            KeyCode::Char(ch) if !ch.is_control() => {
                self.text.insert(self.cursor, ch);
                self.cursor += ch.len_utf8();
                self.message = None;
            }
            KeyCode::Tab => {
                self.text.insert(self.cursor, '\t');
                self.cursor += 1;
                self.message = None;
            }
            KeyCode::Enter => {
                let eol = self.preferred_eol();
                self.text.insert_str(self.cursor, eol);
                self.cursor += eol.len();
                self.message = None;
            }
            KeyCode::Left => self.cursor = self.previous_cursor(),
            KeyCode::Right => self.cursor = self.next_cursor(),
            KeyCode::Backspace => {
                let previous = self.previous_cursor();
                if previous != self.cursor {
                    self.text.drain(previous..self.cursor);
                    self.cursor = previous;
                    self.message = None;
                }
            }
            KeyCode::Delete => {
                let next = self.next_cursor();
                if next != self.cursor {
                    self.text.drain(self.cursor..next);
                    self.message = None;
                }
            }
            KeyCode::Home | KeyCode::End | KeyCode::Up | KeyCode::Down => {
                self.move_cursor(key.code)
            }
            _ => {}
        }
        Action::None
    }

    pub fn lines(&self) -> Vec<(usize, usize)> {
        let bytes = self.text.as_bytes();
        let mut lines = Vec::new();
        let mut start = 0;
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'\r' || bytes[index] == b'\n' {
                lines.push((start, index));
                if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                    index += 1;
                }
                start = index + 1;
            }
            index += 1;
        }
        lines.push((start, bytes.len()));
        lines
    }

    fn preferred_eol(&self) -> &'static str {
        let bytes = self.text.as_bytes();
        match bytes
            .iter()
            .position(|byte| *byte == b'\r' || *byte == b'\n')
        {
            Some(index) if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') => "\r\n",
            Some(index) if bytes[index] == b'\r' => "\r",
            _ => "\n",
        }
    }

    fn previous_cursor(&self) -> usize {
        let Some((index, ch)) = self.text[..self.cursor].char_indices().next_back() else {
            return self.cursor;
        };
        if ch == '\n' && index > 0 && self.text.as_bytes()[index - 1] == b'\r' {
            index - 1
        } else {
            index
        }
    }

    fn next_cursor(&self) -> usize {
        let Some(ch) = self.text[self.cursor..].chars().next() else {
            return self.cursor;
        };
        if ch == '\r' && self.text.as_bytes().get(self.cursor + 1) == Some(&b'\n') {
            self.cursor + 2
        } else {
            self.cursor + ch.len_utf8()
        }
    }

    fn move_cursor(&mut self, code: KeyCode) {
        let lines = self.lines();
        let row = lines
            .iter()
            .position(|&(start, end)| self.cursor >= start && self.cursor <= end)
            .expect("cursor must be on a line");
        let (start, end) = lines[row];
        match code {
            KeyCode::Home => self.cursor = start,
            KeyCode::End => self.cursor = end,
            KeyCode::Up | KeyCode::Down => {
                let target = if code == KeyCode::Up {
                    row.saturating_sub(1)
                } else {
                    (row + 1).min(lines.len() - 1)
                };
                let column = self.text[start..self.cursor].chars().count();
                let (target_start, target_end) = lines[target];
                self.cursor = self.text[target_start..target_end]
                    .char_indices()
                    .nth(column)
                    .map_or(target_end, |(offset, _)| target_start + offset);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(editor: &mut Editor, code: KeyCode) {
        editor.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn editing_preserves_unicode_and_existing_line_endings() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("text.txt");
        fs::write(&path, "aé\r\nb\nc\r").unwrap();
        let mut editor = Editor::open(&path, false).unwrap();
        press(&mut editor, KeyCode::End);
        press(&mut editor, KeyCode::Char('!'));
        press(&mut editor, KeyCode::Enter);
        assert_eq!(editor.text, "aé!\r\n\r\nb\nc\r");
        press(&mut editor, KeyCode::Backspace);
        assert_eq!(editor.text, "aé!\r\nb\nc\r");
        press(&mut editor, KeyCode::Down);
        press(&mut editor, KeyCode::Right);
        press(&mut editor, KeyCode::Backspace);
        assert_eq!(editor.text, "aé!\r\nbc\r");
        press(&mut editor, KeyCode::Up);
        press(&mut editor, KeyCode::End);
        press(&mut editor, KeyCode::Backspace);
        press(&mut editor, KeyCode::Backspace);
        assert_eq!(editor.text, "a\r\nbc\r");
        editor.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(fs::read(&path).unwrap(), "a\r\nbc\r".as_bytes());
    }

    #[test]
    fn rejects_binary_and_does_not_create_on_explorer_open() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("binary");
        fs::write(&path, b"text\0binary").unwrap();
        assert!(
            Editor::open(&path, false)
                .err()
                .unwrap()
                .contains("control characters")
        );
        assert_eq!(fs::read(&path).unwrap(), b"text\0binary");
        let missing = temp.path().join("missing");
        assert!(Editor::open(&missing, false).is_err());
        assert!(!missing.exists());
        assert!(Editor::open(temp.path(), false).is_err());
    }
}
