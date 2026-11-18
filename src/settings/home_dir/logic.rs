use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::config::Config;

#[derive(Default)]
pub struct HomeDir {
    pub input: Option<String>,
}

impl HomeDir {
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        config: &Config,
        current_dir: &std::path::Path,
    ) -> Result<Option<Config>, String> {
        if let Some(input) = &mut self.input {
            match key.code {
                KeyCode::Enter => {
                    let path = PathBuf::from(input.as_str());
                    if !path.is_absolute() {
                        return Err("Home directory must be an absolute path".into());
                    }
                    if !path.is_dir() {
                        return Err("Home directory must exist and be a directory".into());
                    }
                    let mut updated = config.clone();
                    updated.home_dir = Some(path);
                    return Ok(Some(updated));
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    input.clear()
                }
                KeyCode::Char(ch)
                    if !key.modifiers.intersects(
                        KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                    ) =>
                {
                    input.push(ch);
                }
                _ => {}
            }
        } else if key.code == KeyCode::Enter {
            self.input = Some(
                config
                    .home_dir
                    .as_deref()
                    .unwrap_or(current_dir)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        Ok(None)
    }
}
