use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::config::Config;

pub struct ColorEdit {
    pub rgb: [u8; 3],
    pub channel: usize,
}

#[derive(Default)]
pub struct Appearance {
    pub selected: usize,
    pub edit: Option<ColorEdit>,
}

impl Appearance {
    pub fn preview_config(&self, config: &Config) -> Config {
        let mut updated = config.clone();
        if let Some(edit) = &self.edit {
            if self.selected == 0 {
                updated.background = edit.rgb;
            } else {
                updated.lines = edit.rgb;
            }
        }
        updated
    }

    pub fn handle_key(&mut self, key: KeyEvent, config: &Config) -> Option<Config> {
        if let Some(edit) = &mut self.edit {
            let step = if key.modifiers.contains(KeyModifiers::SHIFT) {
                16
            } else {
                1
            };
            match key.code {
                KeyCode::Up => edit.channel = edit.channel.saturating_sub(1),
                KeyCode::Down => edit.channel = (edit.channel + 1).min(2),
                KeyCode::Left => {
                    edit.rgb[edit.channel] = edit.rgb[edit.channel].saturating_sub(step)
                }
                KeyCode::Right => {
                    edit.rgb[edit.channel] = edit.rgb[edit.channel].saturating_add(step)
                }
                KeyCode::Home => edit.rgb[edit.channel] = 0,
                KeyCode::End => edit.rgb[edit.channel] = 255,
                KeyCode::Enter => return Some(self.preview_config(config)),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                KeyCode::Down => self.selected = (self.selected + 1).min(1),
                KeyCode::Enter => {
                    self.edit = Some(ColorEdit {
                        rgb: if self.selected == 0 {
                            config.background
                        } else {
                            config.lines
                        },
                        channel: 0,
                    });
                }
                _ => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_clamps_channels_and_values() {
        let config = Config::default();
        let mut appearance = Appearance::default();
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        appearance.handle_key(key(KeyCode::Enter), &config);
        appearance.handle_key(key(KeyCode::Right), &config);
        appearance.handle_key(key(KeyCode::Up), &config);
        assert_eq!(appearance.edit.as_ref().unwrap().rgb[0], 255);
        assert_eq!(appearance.edit.as_ref().unwrap().channel, 0);
        appearance.handle_key(key(KeyCode::Home), &config);
        appearance.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT), &config);
        appearance.handle_key(key(KeyCode::Down), &config);
        appearance.handle_key(key(KeyCode::Down), &config);
        appearance.handle_key(key(KeyCode::Down), &config);
        assert_eq!(appearance.edit.as_ref().unwrap().rgb[0], 0);
        assert_eq!(appearance.edit.as_ref().unwrap().channel, 2);
    }
}
