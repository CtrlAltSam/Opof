use std::{
    env, fs, io,
    io::Write,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{Context, Result, eyre};
use ratatui::style::{Color, Style};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub background: [u8; 3],
    pub lines: [u8; 3],
    pub home_dir: Option<PathBuf>,
    pub favorites: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            background: [255, 255, 255],
            lines: [0, 0, 0],
            home_dir: None,
            favorites: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read(path) {
            Ok(data) => {
                let config: Self = serde_json::from_slice(&data)
                    .wrap_err_with(|| format!("Invalid settings file {}", path.display()))?;
                if config
                    .home_dir
                    .as_ref()
                    .is_some_and(|home| !home.is_absolute())
                {
                    return Err(eyre!(
                        "Home directory in {} must be absolute",
                        path.display()
                    ));
                }
                if config.favorites.iter().any(|path| !path.is_absolute()) {
                    return Err(eyre!(
                        "Favorite directories in {} must be absolute",
                        path.display()
                    ));
                }
                Ok(config)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error).wrap_err_with(|| format!("Reading {}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| eyre!("Settings path has no parent"))?;
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("Creating settings directory {}", parent.display()))?;
        let data = serde_json::to_vec_pretty(self)?;
        let mut file = tempfile::NamedTempFile::new_in(parent).wrap_err_with(|| {
            format!("Creating temporary settings file in {}", parent.display())
        })?;
        file.write_all(&data)
            .wrap_err_with(|| format!("Writing {}", path.display()))?;
        file.persist(path)
            .wrap_err_with(|| format!("Replacing {}", path.display()))?;
        Ok(())
    }

    pub fn background_color(&self) -> Color {
        let [r, g, b] = self.background;
        Color::Rgb(r, g, b)
    }

    pub fn line_color(&self) -> Color {
        let [r, g, b] = self.lines;
        Color::Rgb(r, g, b)
    }

    pub fn selection_style(&self) -> Style {
        let [r, g, b] = self.background;
        let darker = |v: u8| (u16::from(v) * 85 / 100) as u8;
        let color = if [r, g, b] == [0, 0, 0] {
            Color::Rgb(24, 24, 24)
        } else {
            Color::Rgb(darker(r), darker(g), darker(b))
        };
        let foreground = if u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114 > 128_000 {
            Color::Black
        } else {
            Color::White
        };
        Style::default().fg(foreground).bg(color)
    }
}

pub fn config_path() -> Result<PathBuf> {
    #[cfg(windows)]
    let base = env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| eyre!("APPDATA is not set; cannot locate settings"))?;

    #[cfg(not(windows))]
    let base = match env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        Some(path) if path.is_absolute() => path,
        _ => PathBuf::from(
            env::var_os("HOME").ok_or_else(|| eyre!("HOME is not set; cannot locate settings"))?,
        )
        .join(".config"),
    };

    Ok(base.join("opof").join("settings.json"))
}
