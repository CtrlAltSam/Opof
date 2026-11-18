use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    None,
    OpenSettings,
    OpenExplorer,
    OpenEditor(PathBuf),
}

pub struct Home {
    pub directory: PathBuf,
    pub input: String,
    pub message: Option<String>,
    pub history: Vec<String>,
    pub scroll_from_bottom: usize,
    pub max_scroll: usize,
}

impl Home {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            input: String::new(),
            message: None,
            history: Vec::new(),
            scroll_from_bottom: 0,
            max_scroll: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char(ch)
                if !key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                ) =>
            {
                self.input.push(ch);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Enter => {
                let input = std::mem::take(&mut self.input);
                let command = input.trim();
                if !command.is_empty() {
                    self.history.push(format!("> {input}"));
                    self.scroll_from_bottom = 0;
                }
                self.message = None;
                if command == "settings.opof" {
                    return Action::OpenSettings;
                }
                if command == "fe" {
                    return Action::OpenExplorer;
                }
                if !command.is_empty() {
                    return self.run_command(command);
                }
            }
            KeyCode::Up => {
                self.scroll_from_bottom = self
                    .scroll_from_bottom
                    .saturating_add(1)
                    .min(self.max_scroll);
            }
            KeyCode::Down => self.scroll_from_bottom = self.scroll_from_bottom.saturating_sub(1),
            KeyCode::PageUp => {
                self.scroll_from_bottom = self
                    .scroll_from_bottom
                    .saturating_add(10)
                    .min(self.max_scroll);
            }
            KeyCode::PageDown => {
                self.scroll_from_bottom = self.scroll_from_bottom.saturating_sub(10)
            }
            _ => {}
        }
        Action::None
    }

    fn run_command(&mut self, command: &str) -> Action {
        let (name, argument) = command
            .split_once(char::is_whitespace)
            .map_or((command, ""), |(name, rest)| (name, rest.trim()));
        match name {
            "pof" => {
                if argument.is_empty() {
                    self.history.push("pof: expected a file path".into());
                } else {
                    match self.command_path(argument, false) {
                        Ok(path) => return Action::OpenEditor(path),
                        Err(error) => self.history.push(format!("pof: {error}")),
                    }
                }
            }
            "cd" => {
                let result = self.command_path(argument, true).and_then(|path| {
                    let path = fs::canonicalize(&path)
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                    if !fs::metadata(&path)
                        .map_err(|error| format!("{}: {error}", path.display()))?
                        .is_dir()
                    {
                        return Err(format!("{}: not a directory", path.display()));
                    }
                    Ok(path)
                });
                match result {
                    Ok(path) => self.directory = path,
                    Err(error) => self.history.push(format!("cd: {error}")),
                }
            }
            "ls" => {
                let result = self.command_path(argument, false).and_then(|path| {
                    let metadata = fs::metadata(&path)
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                    if !metadata.is_dir() {
                        return Ok(vec![
                            path.file_name()
                                .unwrap_or(path.as_os_str())
                                .to_string_lossy()
                                .into_owned(),
                        ]);
                    }
                    let entries = fs::read_dir(&path)
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                    let mut names = entries
                        .map(|entry| entry.map(|entry| entry.file_name()))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                    names.sort();
                    Ok(names
                        .into_iter()
                        .map(|name| name.to_string_lossy().into_owned())
                        .collect())
                });
                match result {
                    Ok(names) => self.history.extend(names),
                    Err(error) => self.history.push(format!("ls: {error}")),
                }
            }
            _ => self.history.push(format!("Unknown command: {command}")),
        }
        Action::None
    }

    fn command_path(&self, argument: &str, home_if_empty: bool) -> Result<PathBuf, String> {
        let quoted = argument.starts_with('"') || argument.starts_with('\'');
        let argument = if quoted {
            let quote = argument.chars().next().unwrap();
            argument
                .strip_prefix(quote)
                .and_then(|value| value.strip_suffix(quote))
                .ok_or_else(|| "unterminated quoted path".to_string())?
        } else {
            argument
        };
        if quoted && argument.is_empty() {
            return Err("path is empty".into());
        }
        if argument.is_empty() && !home_if_empty {
            return Ok(self.directory.clone());
        }
        let home_suffix = if argument.is_empty() {
            Some(Path::new(""))
        } else {
            Path::new(argument).strip_prefix("~").ok()
        };
        let path = if let Some(suffix) = home_suffix {
            #[cfg(windows)]
            let home = env::var_os("USERPROFILE");
            #[cfg(not(windows))]
            let home = env::var_os("HOME");
            let home = PathBuf::from(
                home.filter(|value| !value.is_empty())
                    .ok_or_else(|| "home directory environment variable is not set".to_string())?,
            );
            if !home.is_absolute() {
                return Err("home directory environment variable must be absolute".into());
            }
            home.join(suffix)
        } else {
            PathBuf::from(argument)
        };
        #[cfg(windows)]
        if !path.is_absolute()
            && matches!(
                path.components().next(),
                Some(std::path::Component::Prefix(_))
            )
        {
            return Err("drive-relative paths are not supported; use an absolute path".into());
        }
        Ok(if path.is_absolute() {
            path
        } else {
            self.directory.join(path)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(home: &mut Home, command: &str) {
        for ch in command.chars() {
            home.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        home.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    }

    #[test]
    fn editing_and_unknown_commands() {
        let mut home = Home::new(PathBuf::from("example"));
        for ch in "badx".chars() {
            home.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        home.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        home.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(home.history, ["> bad", "Unknown command: bad"]);
        assert!(home.input.is_empty());
    }

    #[test]
    fn cd_and_ls_use_current_directory_and_preserve_history() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("folder with spaces");
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("z.txt"), "").unwrap();
        fs::write(nested.join(".hidden"), "").unwrap();
        let mut home = Home::new(temp.path().to_path_buf());

        run(&mut home, "ls \"folder with spaces\"");
        assert_eq!(
            home.history,
            ["> ls \"folder with spaces\"", ".hidden", "z.txt"]
        );
        run(&mut home, "cd folder with spaces");
        assert_eq!(home.directory, fs::canonicalize(&nested).unwrap());
        run(&mut home, "ls");
        assert_eq!(&home.history[4..], ["> ls", ".hidden", "z.txt"]);
        run(&mut home, "cd ..");
        assert_eq!(home.directory, fs::canonicalize(temp.path()).unwrap());
        run(&mut home, &format!("cd \"{}\"", nested.display()));
        assert_eq!(home.directory, fs::canonicalize(&nested).unwrap());
        run(&mut home, "ls z.txt");
        assert_eq!(home.history.last().unwrap(), "z.txt");
    }

    #[test]
    fn command_errors_are_visible_and_do_not_change_directory() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("file"), "").unwrap();
        let mut home = Home::new(temp.path().to_path_buf());
        for command in ["cd missing", "cd file", "cd \"unfinished", "cd \"\""] {
            run(&mut home, command);
            assert!(home.history.last().unwrap().starts_with("cd: "));
            assert_eq!(home.directory, temp.path());
        }
        run(&mut home, "ls missing");
        assert!(home.history.last().unwrap().starts_with("ls: "));
        run(&mut home, "ls \"\"");
        assert_eq!(home.history.last().unwrap(), "ls: path is empty");
    }
}
