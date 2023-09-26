use std::path::PathBuf;

use super::database::DbConnection;
use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use miette::{IntoDiagnostic, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ReadConfig {
    version: usize,
    music_dir: Option<PathBuf>,
    cookies: Option<PathBuf>,
    yt_playlist_sync: Option<Vec<String>>,
}

impl ReadConfig {
    /// Read config from provided path
    pub async fn read_config(path: Option<PathBuf>) -> Result<Config> {
        let strategy = choose_app_strategy(AppStrategyArgs {
            top_level_domain: "".to_string(),
            author: "".to_string(),
            app_name: "muzik".to_string(),
        })
        .into_diagnostic()?;

        let config_path = {
            if let Some(path) = path {
                path
            } else {
                strategy.config_dir().join("config.toml")
            }
        };

        if !config_path.exists() {
            // Create the dir
            std::fs::create_dir_all(config_path.parent().unwrap()).into_diagnostic()?;
            // write version to file
            std::fs::write(&config_path, "version = 1").into_diagnostic()?;
        }

        let conf: ReadConfig =
            toml::from_str(&std::fs::read_to_string(config_path).into_diagnostic()?)
                .into_diagnostic()?;

        // conf is now version 1
        assert!(conf.version == 1);

        let music_dir = {
            if let Some(dir) = conf.music_dir {
                dir
            } else {
                let home = strategy.home_dir();
                if let Ok(_termux_ver) = std::env::var("TERMUX_VERSION") {
                    PathBuf::from(std::env::var("HOME").unwrap()).join("storage/music")
                } else {
                    home.join("Music")
                }
            }
        };

        let db_new = DbConnection::m_new(music_dir.join("database.sqlite")).await?;

        let cookies = {
            if let Some(cookies_path) = conf.cookies {
                cookies_path
            } else {
                strategy.data_dir().join("cookies.txt")
            }
        };
        Ok(Config {
            music_dir,
            db_new,
            cookies: if cookies.exists() {
                Some(cookies)
            } else {
                None
            },
            yt_playlist_sync: conf.yt_playlist_sync,
        })
    }
}

#[derive(Clone)]
pub struct Config {
    pub music_dir: PathBuf,
    pub db_new: DbConnection,
    pub cookies: Option<PathBuf>,
    pub yt_playlist_sync: Option<Vec<String>>,
}

impl Config {
    pub fn get_music_dir(&self) -> PathBuf {
        self.music_dir.clone()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            music_dir: Default::default(),
            db_new: DbConnection::default(),
            cookies: Default::default(),
            yt_playlist_sync: Default::default(),
        }
    }
}
