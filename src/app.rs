use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{Frame, style::Style, widgets::Block};

use crate::{editor, explorer, home, settings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Home,
    Settings,
    Explorer,
    Editor,
}

pub struct App {
    screen: Screen,
    home: home::logic::Home,
    settings: settings::logic::Settings,
    explorer: explorer::logic::Explorer,
    editor: Option<(editor::logic::Editor, Screen)>,
    config: settings::config::Config,
    config_path: PathBuf,
    pub should_quit: bool,
}

impl App {
    pub fn new(
        directory: PathBuf,
        config: settings::config::Config,
        config_path: PathBuf,
        startup_message: Option<String>,
    ) -> Self {
        let mut home = home::logic::Home::new(directory);
        home.message = startup_message;
        Self {
            screen: Screen::Home,
            home,
            settings: settings::logic::Settings::default(),
            explorer: explorer::logic::Explorer::default(),
            editor: None,
            config,
            config_path,
            should_quit: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return;
        }

        if self.screen == Screen::Editor {
            let action = self
                .editor
                .as_mut()
                .expect("editor is open")
                .0
                .handle_key(key);
            match action {
                editor::logic::Action::None => {}
                editor::logic::Action::Close => {
                    let (_, origin) = self.editor.take().expect("editor is open");
                    self.screen = origin;
                    if origin == Screen::Explorer {
                        self.explorer.refresh();
                    }
                }
                editor::logic::Action::Quit => self.should_quit = true,
            }
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                    return;
                }
                KeyCode::Char('s') => {
                    self.screen = Screen::Settings;
                    return;
                }
                _ => {}
            }
        }

        match self.screen {
            Screen::Home => match self.home.handle_key(key) {
                home::logic::Action::OpenSettings => self.screen = Screen::Settings,
                home::logic::Action::OpenExplorer => {
                    self.explorer.open(&self.home.directory, &self.config);
                    self.screen = Screen::Explorer;
                }
                home::logic::Action::OpenEditor(path) => {
                    self.open_editor(path, Screen::Home, true);
                }
                home::logic::Action::None => {}
            },
            Screen::Explorer => {
                match self
                    .explorer
                    .handle_key(key, &mut self.config, &self.config_path)
                {
                    explorer::logic::Action::None => {}
                    explorer::logic::Action::Close => self.screen = Screen::Home,
                    explorer::logic::Action::OpenFile(path) => {
                        self.open_editor(path, Screen::Explorer, false);
                    }
                }
            }
            Screen::Settings => {
                if self.settings.handle_key(
                    key,
                    &mut self.config,
                    &self.config_path,
                    &self.home.directory,
                ) {
                    self.screen = Screen::Home;
                }
            }
            Screen::Editor => unreachable!("editor keys are handled above"),
        }
    }

    fn open_editor(&mut self, path: PathBuf, origin: Screen, create: bool) {
        match editor::logic::Editor::open(&path, create) {
            Ok(editor) => {
                self.editor = Some((editor, origin));
                self.screen = Screen::Editor;
            }
            Err(error) => match origin {
                Screen::Home => self.home.history.push(format!("pof: {error}")),
                Screen::Explorer => self.explorer.message = Some(error),
                _ => unreachable!("editor opens from home or explorer"),
            },
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let config = if self.screen == Screen::Settings {
            self.settings.appearance.preview_config(&self.config)
        } else {
            self.config.clone()
        };
        frame.render_widget(
            Block::new().style(
                Style::default()
                    .fg(config.line_color())
                    .bg(config.background_color()),
            ),
            frame.area(),
        );
        match self.screen {
            Screen::Home => home::layout::render(frame, &mut self.home),
            Screen::Explorer => explorer::layout::render(frame, &mut self.explorer, &self.config),
            Screen::Editor => {
                editor::layout::render(frame, &mut self.editor.as_mut().expect("editor is open").0)
            }
            Screen::Settings => {
                settings::layout::render(frame, &self.settings, &config, &self.home.directory)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, style::Color};

    fn app() -> App {
        App::new(
            PathBuf::from("example"),
            settings::config::Config::default(),
            PathBuf::from("unused-settings.json"),
            None,
        )
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn command_opens_settings_and_escape_returns_home() {
        let mut app = app();
        for ch in "settings.opof".chars() {
            app.handle_key(key(KeyCode::Char(ch)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.screen, Screen::Settings);
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.screen, Screen::Home);
        assert!(app.home.input.is_empty());
    }

    #[test]
    fn shortcuts_work_and_key_releases_are_ignored() {
        let mut app = app();
        let mut shortcut = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
        shortcut.kind = KeyEventKind::Release;
        app.handle_key(shortcut);
        assert_eq!(app.screen, Screen::Home);
        shortcut.kind = KeyEventKind::Press;
        app.handle_key(shortcut);
        assert_eq!(app.screen, Screen::Settings);
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
        assert!(app.should_quit);
    }

    #[test]
    fn screens_render_with_white_background_and_black_borders() {
        let mut app = app();
        let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, 0)].fg, Color::Rgb(0, 0, 0));
        assert_eq!(buffer[(0, 0)].bg, Color::Rgb(255, 255, 255));
        assert_eq!(buffer[(0, 9)].symbol(), "┌");
        assert_eq!(buffer[(1, 10)].symbol(), ">");
        assert_eq!(buffer[(2, 0)].symbol(), "e");
        assert_eq!(buffer[(1, 1)].symbol(), " ");

        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        terminal.draw(|frame| app.render(frame)).unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, 0)].symbol(), "┌");
        assert_eq!(buffer[(15, 0)].symbol(), "┌");
        assert_eq!(buffer[(15, 0)].fg, Color::Rgb(0, 0, 0));
        assert_eq!(buffer[(15, 0)].bg, Color::Rgb(255, 255, 255));
        assert_eq!(buffer[(2, 2)].bg, Color::Rgb(216, 216, 216));
    }

    #[test]
    fn screens_fill_terminal_width_after_resize() {
        let mut app = app();
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        for (width, height) in [(80, 20), (61, 12), (119, 30), (40, 10)] {
            terminal.backend_mut().resize(width, height);
            for screen in [Screen::Home, Screen::Settings] {
                app.screen = screen;
                terminal.draw(|frame| app.render(frame)).unwrap();
                let buffer = terminal.backend().buffer();
                assert_eq!(buffer[(width - 1, 0)].symbol(), "┐");
                assert_eq!(buffer[(width - 1, height - 1)].symbol(), "┘");
                assert_eq!(buffer[(width - 1, 1)].symbol(), "│");
                assert_eq!(buffer[(width - 1, 1)].bg, Color::Rgb(255, 255, 255));
            }
        }
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_key(key(code));
    }

    fn text(app: &mut App, value: &str) {
        for ch in value.chars() {
            press(app, KeyCode::Char(ch));
        }
    }

    fn clear_input(app: &mut App) {
        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    }

    fn row(terminal: &Terminal<TestBackend>, y: u16) -> String {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    #[test]
    fn directory_header_and_listing_follow_cd() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("item.txt"), "").unwrap();
        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            temp.path().join("settings.json"),
            None,
        );
        let mut terminal = Terminal::new(TestBackend::new(200, 10)).unwrap();
        text(&mut app, "cd nested");
        press(&mut app, KeyCode::Enter);
        text(&mut app, "ls");
        press(&mut app, KeyCode::Enter);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 0).contains("nested"));
        assert!(!row(&terminal, 0).contains(r"\\?\"));
        assert!(row(&terminal, 1).contains("> cd nested"));
        assert!(row(&terminal, 2).contains("> ls"));
        assert!(row(&terminal, 3).contains("item.txt"));
    }

    #[test]
    fn pof_creates_edits_and_reopens_a_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("draft file.txt");
        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            temp.path().join("settings.json"),
            None,
        );
        text(&mut app, "pof \"draft file.txt\"");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Editor);
        assert_eq!(std::fs::read(&path).unwrap(), b"");
        text(&mut app, "hello");
        press(&mut app, KeyCode::Enter);
        text(&mut app, "world");
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(app.screen, Screen::Editor);
        assert_eq!(std::fs::read(&path).unwrap(), b"hello\nworld");
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.screen, Screen::Home);
        text(&mut app, "pof draft file.txt");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.editor.as_ref().unwrap().0.text, "hello\nworld");
        assert_eq!(app.home.directory, temp.path());
    }

    #[test]
    fn explorer_opens_files_and_reports_failed_open_without_creating() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("a.txt");
        let binary = temp.path().join("z.bin");
        std::fs::write(&path, "abc").unwrap();
        std::fs::write(&binary, [0xff]).unwrap();
        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            temp.path().join("settings.json"),
            None,
        );
        text(&mut app, "fe");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Editor);
        press(&mut app, KeyCode::End);
        text(&mut app, "d");
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.screen, Screen::Explorer);
        assert_eq!(std::fs::read(&path).unwrap(), b"abcd");
        assert_eq!(app.explorer.entries[0].size, "4");
        assert_eq!(app.home.directory, temp.path());

        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Explorer);
        assert!(
            app.explorer
                .message
                .as_deref()
                .unwrap()
                .contains("not UTF-8")
        );
        assert_eq!(std::fs::read(&binary).unwrap(), [0xff]);

        std::fs::remove_file(&path).unwrap();
        press(&mut app, KeyCode::Up);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Explorer);
        assert!(
            app.explorer
                .message
                .as_deref()
                .unwrap()
                .contains("Could not open")
        );
        assert!(!path.exists());
    }

    #[test]
    fn editor_guards_unsaved_changes_and_external_modifications() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.txt");
        std::fs::write(&path, "original").unwrap();
        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            temp.path().join("settings.json"),
            None,
        );
        text(&mut app, "pof note.txt");
        press(&mut app, KeyCode::Enter);
        text(&mut app, "edit");
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
        assert!(!app.should_quit);
        assert!(app.editor.as_ref().unwrap().0.exit_prompt().is_some());
        std::fs::write(&path, "external").unwrap();
        press(&mut app, KeyCode::Char('s'));
        assert!(!app.should_quit);
        assert!(
            app.editor
                .as_ref()
                .unwrap()
                .0
                .message
                .as_deref()
                .unwrap()
                .contains("changed on disk")
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"external");
        press(&mut app, KeyCode::Esc);
        assert!(app.editor.as_ref().unwrap().0.exit_prompt().is_none());
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('d'));
        assert_eq!(app.screen, Screen::Home);
        assert_eq!(std::fs::read(&path).unwrap(), b"external");
    }

    #[test]
    fn editor_save_on_exit_and_input_errors_are_visible() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            temp.path().join("settings.json"),
            None,
        );
        for command in [
            "pof".to_string(),
            "pof \"\"".to_string(),
            format!(
                "pof {}",
                temp.path().join("missing").join("file.txt").display()
            ),
        ] {
            text(&mut app, &command);
            press(&mut app, KeyCode::Enter);
            assert_eq!(app.screen, Screen::Home);
            assert!(app.home.history.last().unwrap().starts_with("pof:"));
        }
        text(&mut app, "pof saved.txt");
        press(&mut app, KeyCode::Enter);
        text(&mut app, "saved");
        press(&mut app, KeyCode::Esc);
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.screen, Screen::Home);
        assert_eq!(
            std::fs::read(temp.path().join("saved.txt")).unwrap(),
            b"saved"
        );
    }

    #[test]
    fn editor_quit_requires_a_decision_for_unsaved_work() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("quit.txt");
        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            temp.path().join("settings.json"),
            None,
        );
        text(&mut app, "pof quit.txt");
        press(&mut app, KeyCode::Enter);
        text(&mut app, "kept");
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
        assert!(!app.should_quit);
        press(&mut app, KeyCode::Char('s'));
        assert!(app.should_quit);
        assert_eq!(std::fs::read(&path).unwrap(), b"kept");

        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            temp.path().join("settings.json"),
            None,
        );
        text(&mut app, "pof quit.txt");
        press(&mut app, KeyCode::Enter);
        text(&mut app, "discarded");
        app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
        press(&mut app, KeyCode::Char('d'));
        assert!(app.should_quit);
        assert_eq!(std::fs::read(&path).unwrap(), b"kept");
    }

    #[test]
    fn editor_scrolls_to_cursor_after_resize() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            temp.path().join("settings.json"),
            None,
        );
        text(&mut app, "pof long.txt");
        press(&mut app, KeyCode::Enter);
        text(&mut app, "012345678901234567890123456789xyz");
        let mut terminal = Terminal::new(TestBackend::new(20, 7)).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 1).contains("xyz"));
        for _ in 0..10 {
            press(&mut app, KeyCode::Enter);
        }
        text(&mut app, "last");
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!((1..6).any(|y| row(&terminal, y).contains("last")));
        for (width, height) in [(1, 1), (12, 5), (40, 10)] {
            terminal.backend_mut().resize(width, height);
            terminal.draw(|frame| app.render(frame)).unwrap();
        }
        assert!((1..9).any(|y| row(&terminal, y).contains("last")));
    }

    #[test]
    fn explorer_navigates_highlights_and_persists_favorites() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("folder");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(temp.path().join("note.txt"), "abc").unwrap();
        let config_path = temp.path().join("config").join("settings.json");
        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            config_path.clone(),
            None,
        );
        text(&mut app, "fe");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Explorer);

        let mut terminal = Terminal::new(TestBackend::new(120, 12)).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 1).contains("Modified (UTC)"));
        assert!(row(&terminal, 1).contains("Size (bytes)"));
        assert!(row(&terminal, 2).contains("📁"));
        assert!(row(&terminal, 2).contains("folder"));
        assert!(row(&terminal, 3).contains("📄"));
        assert!(row(&terminal, 3).contains("note.txt"));
        assert!(row(&terminal, 3).contains("TXT file"));
        assert!(row(&terminal, 3).contains('3'));
        assert_eq!(
            terminal.backend().buffer()[(31, 2)].bg,
            Color::Rgb(216, 216, 216)
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(
            app.config.favorites,
            [std::fs::canonicalize(temp.path()).unwrap()]
        );
        assert_eq!(
            settings::config::Config::load(&config_path).unwrap(),
            app.config
        );
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.explorer.directory,
            std::fs::canonicalize(&nested).unwrap()
        );
        assert_eq!(app.home.directory, temp.path());
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        press(&mut app, KeyCode::Tab);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert_eq!(
            terminal.backend().buffer()[(1, 1)].bg,
            Color::Rgb(216, 216, 216)
        );
        assert_eq!(terminal.backend().buffer()[(1, 1)].symbol(), "📁");
        assert_eq!(terminal.backend().buffer()[(1, 2)].symbol(), "📁");
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.explorer.directory,
            std::fs::canonicalize(temp.path()).unwrap()
        );
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.explorer.directory,
            std::fs::canonicalize(&nested).unwrap()
        );
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(app.config.favorites.len(), 1);
        assert_eq!(
            settings::config::Config::load(&config_path).unwrap(),
            app.config
        );
        let mut reopened = App::new(
            nested.clone(),
            settings::config::Config::load(&config_path).unwrap(),
            config_path.clone(),
            None,
        );
        text(&mut reopened, "fe");
        press(&mut reopened, KeyCode::Enter);
        press(&mut reopened, KeyCode::Tab);
        press(&mut reopened, KeyCode::Enter);
        assert_eq!(
            reopened.explorer.directory,
            std::fs::canonicalize(temp.path()).unwrap()
        );
        assert_eq!(reopened.home.directory, nested);

        press(&mut app, KeyCode::Backspace);
        assert_eq!(
            app.explorer.directory,
            std::fs::canonicalize(temp.path()).unwrap()
        );
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.screen, Screen::Home);
        assert_eq!(app.home.directory, temp.path());
        assert!(app.home.history.contains(&"> fe".to_string()));
    }

    #[test]
    fn explorer_directory_is_independent_and_restarts_from_home() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("nested");
        let inner = nested.join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            temp.path().join("settings.json"),
            None,
        );

        text(&mut app, "fe");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.explorer.directory, temp.path());
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.explorer.directory,
            std::fs::canonicalize(&nested).unwrap()
        );
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.home.directory, temp.path());

        text(&mut app, "cd nested");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.home.directory, std::fs::canonicalize(&nested).unwrap());
        text(&mut app, "fe");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.explorer.directory, app.home.directory);
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.explorer.directory,
            std::fs::canonicalize(&inner).unwrap()
        );
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.home.directory, std::fs::canonicalize(&nested).unwrap());
        text(&mut app, "fe");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.explorer.directory, app.home.directory);
    }

    #[test]
    fn explorer_reports_favorite_and_navigation_errors() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");
        let config = settings::config::Config {
            favorites: vec![missing],
            ..settings::config::Config::default()
        };
        let mut app = App::new(
            temp.path().to_path_buf(),
            config.clone(),
            temp.path().to_path_buf(),
            None,
        );
        text(&mut app, "fe");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Enter);
        assert!(
            app.explorer
                .message
                .as_deref()
                .unwrap()
                .contains("Could not open")
        );
        assert_eq!(app.home.directory, temp.path());
        assert_eq!(app.explorer.directory, temp.path());
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(
            app.explorer
                .message
                .as_deref()
                .unwrap()
                .contains("Could not save favorites")
        );
        assert_eq!(app.config, config);
    }

    #[test]
    fn existing_settings_without_favorites_still_load() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"background":[1,2,3],"lines":[4,5,6],"home_dir":null}"#,
        )
        .unwrap();
        let config = settings::config::Config::load(&path).unwrap();
        assert_eq!(config.background, [1, 2, 3]);
        assert!(config.favorites.is_empty());
    }

    #[test]
    fn explorer_scrolls_to_selection_and_survives_resize() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..20 {
            std::fs::write(temp.path().join(format!("file{index:02}.txt")), "").unwrap();
        }
        let mut app = App::new(
            temp.path().to_path_buf(),
            settings::config::Config::default(),
            temp.path().join("settings.json"),
            None,
        );
        text(&mut app, "fe");
        press(&mut app, KeyCode::Enter);
        let mut terminal = Terminal::new(TestBackend::new(100, 8)).unwrap();
        press(&mut app, KeyCode::End);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!((1..7).any(|y| row(&terminal, y).contains("file19.txt")));
        assert!(app.explorer.files.offset() > 0);
        for (width, height) in [(60, 10), (40, 6), (1, 1), (100, 12)] {
            terminal.backend_mut().resize(width, height);
            terminal.draw(|frame| app.render(frame)).unwrap();
        }
        assert!((1..11).any(|y| row(&terminal, y).contains("file19.txt")));
    }

    #[test]
    fn command_history_scrolls_and_follows_new_entries() {
        let mut app = app();
        let mut terminal = Terminal::new(TestBackend::new(32, 9)).unwrap();
        for command in ["first", "second", "third"] {
            text(&mut app, command);
            press(&mut app, KeyCode::Enter);
        }
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 4).contains("Unknown command: third"));
        assert!(app.home.max_scroll > 0);
        assert!(row(&terminal, 1).ends_with('║') || row(&terminal, 1).ends_with('█'));

        press(&mut app, KeyCode::PageUp);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 0).contains("example"));
        assert!(row(&terminal, 1).contains("> first"));
        press(&mut app, KeyCode::Down);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 1).contains("Unknown command: first"));

        text(&mut app, "settings.opof");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 4).contains("> settings.opof"));
        assert_eq!(app.home.scroll_from_bottom, 0);
        let history_len = app.home.history.len();
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.home.history.len(), history_len);
    }

    #[test]
    fn wrapped_history_and_long_input_remain_visible_after_resize() {
        let mut app = app();
        let mut terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
        text(&mut app, "abcdefghijklmnopqrstuv");
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 6).contains("qrstuv"));
        press(&mut app, KeyCode::Enter);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 2).contains("abcdefghijklmnopqr"));
        assert!(row(&terminal, 3).contains("stuv"));
        assert!(app.home.max_scroll > 0);

        press(&mut app, KeyCode::PageUp);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 1).contains("> "));
        press(&mut app, KeyCode::PageDown);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(row(&terminal, 3).contains("stuv"));

        terminal.backend_mut().resize(80, 12);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert_eq!(app.home.max_scroll, 0);
        assert!(row(&terminal, 1).contains("> abcdefghijklmnopqrstuv"));
        assert_eq!(terminal.backend().buffer()[(79, 1)].symbol(), "│");
    }

    #[test]
    fn focus_controls_navigation_and_appearance_is_saved_and_rendered() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut app = App::new(
            dir.path().to_path_buf(),
            settings::config::Config::default(),
            path.clone(),
            None,
        );
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.settings.active, Some(settings::logic::Page::Appearance));
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Down);
        assert_eq!(app.settings.selected, settings::logic::Page::Appearance);
        assert_eq!(app.settings.appearance.selected, 1);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::End);
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            settings::config::Config::load(&path).unwrap().lines,
            [255, 0, 0]
        );

        press(&mut app, KeyCode::Up);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Home);
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Home);
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Left);
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            settings::config::Config::load(&path).unwrap().background,
            [1, 16, 254]
        );
        let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert_eq!(
            terminal.backend().buffer()[(15, 0)].fg,
            Color::Rgb(255, 0, 0)
        );
        assert_eq!(
            terminal.backend().buffer()[(15, 0)].bg,
            Color::Rgb(1, 16, 254)
        );
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Down);
        assert_eq!(app.settings.appearance.selected, 0);
        assert_eq!(app.settings.selected, settings::logic::Page::HomeDir);
        assert_eq!(app.settings.active, Some(settings::logic::Page::Appearance));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.settings.active, Some(settings::logic::Page::HomeDir));
    }

    #[test]
    fn home_dir_validation_and_save_errors_are_visible() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut app = App::new(
            dir.path().to_path_buf(),
            settings::config::Config::default(),
            path.clone(),
            None,
        );
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Enter);
        clear_input(&mut app);
        text(&mut app, "relative");
        press(&mut app, KeyCode::Enter);
        assert!(
            app.settings
                .message
                .as_deref()
                .unwrap()
                .contains("absolute")
        );
        assert!(!path.exists());
        clear_input(&mut app);
        text(&mut app, &dir.path().join("missing").to_string_lossy());
        press(&mut app, KeyCode::Enter);
        assert!(
            app.settings
                .message
                .as_deref()
                .unwrap()
                .contains("must exist")
        );
        clear_input(&mut app);
        text(&mut app, &dir.path().to_string_lossy());
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            settings::config::Config::load(&path)
                .unwrap()
                .home_dir
                .as_deref(),
            Some(dir.path())
        );
        assert_eq!(app.home.directory, dir.path());

        app.config_path = dir.path().to_path_buf();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Enter);
        assert!(
            app.settings
                .message
                .as_deref()
                .unwrap()
                .contains("Could not save")
        );
        assert!(app.settings.home_dir.input.is_some());
        assert_eq!(settings::config::Config::load(&path).unwrap(), app.config);
    }

    #[test]
    fn slider_preview_and_cancel_do_not_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let mut app = App::new(
            dir.path().to_path_buf(),
            settings::config::Config::default(),
            path.clone(),
            None,
        );
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Home);
        let mut terminal = Terminal::new(TestBackend::new(80, 14)).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert_eq!(
            terminal.backend().buffer()[(79, 0)].bg,
            Color::Rgb(0, 255, 255)
        );
        assert_eq!(terminal.backend().buffer()[(23, 4)].symbol(), "R");
        assert_eq!(terminal.backend().buffer()[(23, 5)].symbol(), "G");
        assert_eq!(terminal.backend().buffer()[(23, 6)].symbol(), "B");
        assert_eq!(app.config.background, [255, 255, 255]);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Home);
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Home);
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert_eq!(terminal.backend().buffer()[(21, 1)].fg, Color::White);
        assert_eq!(terminal.backend().buffer()[(21, 1)].bg, Color::Rgb(0, 0, 0));
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.screen, Screen::Settings);
        assert!(app.settings.appearance.edit.is_none());
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert_eq!(
            terminal.backend().buffer()[(79, 0)].bg,
            Color::Rgb(255, 255, 255)
        );
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Home);
        press(&mut app, KeyCode::Tab);
        assert!(app.settings.appearance.edit.is_none());
        assert_eq!(app.config.background, [255, 255, 255]);
        assert!(!path.exists());
    }
}
