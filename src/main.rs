mod app;
mod editor;
mod explorer;
mod home;
mod settings;

use color_eyre::Result;
use crossterm::event::{self, Event};
use settings::config::Config;
use std::path::PathBuf;

fn main() -> Result<()> {
    color_eyre::install()?;
    let path = settings::config::config_path()?;
    let config = Config::load(&path)?;
    let (directory, startup_message) = startup_directory(&config)?;
    let mut app = app::App::new(directory, config, path, startup_message);

    ratatui::run(|terminal| {
        while !app.should_quit {
            terminal.draw(|frame| app.render(frame))?;
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }
        Ok(())
    })
}

fn startup_directory(config: &Config) -> Result<(PathBuf, Option<String>)> {
    let mut directory = std::env::current_dir()?;
    let mut startup_message = None;
    if let Some(home) = &config.home_dir {
        match std::env::set_current_dir(home) {
            Ok(()) => directory = std::env::current_dir()?,
            Err(error) => {
                startup_message = Some(format!(
                    "Could not open saved home directory {}: {error}",
                    home.display()
                ));
            }
        }
    }
    Ok((directory, startup_message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_home_is_used_on_startup_and_missing_home_is_reported() {
        const CHILD_CONFIG: &str = "OPOF_TEST_STARTUP_CONFIG_PATH";
        if let Some(path) = std::env::var_os(CHILD_CONFIG) {
            let config = Config::load(&PathBuf::from(path)).unwrap();
            let (directory, message) = startup_directory(&config).unwrap();
            assert_eq!(directory, config.home_dir.unwrap());
            assert_eq!(std::env::current_dir().unwrap(), directory);
            assert!(message.is_none());

            let missing = Config {
                home_dir: Some(directory.join("missing")),
                ..Config::default()
            };
            let (fallback, message) = startup_directory(&missing).unwrap();
            assert_eq!(fallback, directory);
            assert!(
                message
                    .unwrap()
                    .contains("Could not open saved home directory")
            );
            return;
        }

        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        std::fs::create_dir(&home).unwrap();
        let path = temp.path().join("settings.json");
        Config {
            home_dir: Some(home),
            ..Config::default()
        }
        .save(&path)
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::saved_home_is_used_on_startup_and_missing_home_is_reported",
            ])
            .env(CHILD_CONFIG, &path)
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child test failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
