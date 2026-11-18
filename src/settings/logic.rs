use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent};

use super::{appearance::logic::Appearance, config::Config, home_dir::logic::HomeDir};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Appearance,
    HomeDir,
}

pub struct Settings {
    pub selected: Page,
    pub active: Option<Page>,
    pub right_focused: bool,
    pub appearance: Appearance,
    pub home_dir: HomeDir,
    pub message: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            selected: Page::Appearance,
            active: None,
            right_focused: false,
            appearance: Appearance::default(),
            home_dir: HomeDir::default(),
            message: None,
        }
    }
}

impl Settings {
    fn cancel_edit(&mut self) -> bool {
        self.appearance.edit.take().is_some() | self.home_dir.input.take().is_some()
    }

    // Returns true when the settings screen should close.
    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        config: &mut Config,
        path: &Path,
        current_dir: &Path,
    ) -> bool {
        match key.code {
            KeyCode::Esc => {
                if self.cancel_edit() {
                    self.message = None;
                    return false;
                }
                return true;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.cancel_edit();
                self.right_focused = !self.right_focused;
                self.message = None;
                return false;
            }
            _ => {}
        }

        if !self.right_focused {
            match key.code {
                KeyCode::Up => self.selected = Page::Appearance,
                KeyCode::Down => self.selected = Page::HomeDir,
                KeyCode::Enter => {
                    self.active = Some(self.selected);
                    self.message = None;
                }
                _ => {}
            }
            return false;
        }

        let update = match self.active {
            Some(Page::Appearance) => Ok(self.appearance.handle_key(key, config)),
            Some(Page::HomeDir) => self.home_dir.handle_key(key, config, current_dir),
            None => return false,
        };
        match update {
            Ok(Some(updated)) => match updated.save(path) {
                Ok(()) => {
                    *config = updated;
                    self.cancel_edit();
                    self.message = Some("Saved.".into());
                }
                Err(error) => self.message = Some(format!("Could not save settings: {error:#}")),
            },
            Ok(None) => {}
            Err(error) => self.message = Some(error),
        }
        false
    }
}
