use std::path::PathBuf;

use crate::audio_file_name::{AudioFileNameComponents, Enclose};

use self::error::ConfigError;

use super::database::DbConnection;
use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use miette::Result;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Deserialize, Serialize)]
pub struct ReadConfig {
    version: usize,
    music_dir: PathBuf,
    cookies: Option<PathBuf>,
    yt_playlist_sync: Option<Vec<String>>,
    db_path: Option<String>,
    audio_file_name_format: Vec<AudioFileNameComponents>,
}

impl ReadConfig {
    /// Read config from provided path
    pub async fn read_config(path: Option<PathBuf>) -> Result<Config, ConfigError> {
        let strategy = choose_app_strategy(AppStrategyArgs {
            top_level_domain: "".to_string(),
            author: "".to_string(),
            app_name: "muzik".to_string(),
        })?;

        let config_path = {
            if let Some(path) = path {
                path
            } else {
                strategy.config_dir().join("config.toml")
            }
        };

        let conf: ReadConfig = if !config_path.exists() {
            let music_dir = {
                let home = strategy.home_dir();
                if let Ok(_termux_ver) = std::env::var("TERMUX_VERSION") {
                    PathBuf::from(std::env::var("HOME").unwrap()).join("storage/music")
                } else {
                    home.join("Music")
                }
            };

            // Example starting config
            let starter_read_config = Self {
                version: 1,
                music_dir,
                cookies: None,
                yt_playlist_sync: None,
                db_path: None,
                audio_file_name_format: vec![
                    AudioFileNameComponents::Title(Enclose::None),
                    AudioFileNameComponents::Custom("-".to_string()),
                    AudioFileNameComponents::Artist(Enclose::None),
                    AudioFileNameComponents::DatabaseId(Enclose::Bracket),
                ],
            };

            // Create the dir
            std::fs::create_dir_all(config_path.parent().unwrap())?;
            // write version to file
            std::fs::write(&config_path, toml::to_string_pretty(&starter_read_config)?)?;
            starter_read_config
        } else {
            toml::from_str(&std::fs::read_to_string(config_path)?)?
        };

        // conf is now version 1
        match conf.version {
            1 => {
                info!("configuration version 1 detected, this is the latest version");
            }
            _ => {
                error!(
                    "unsupported configuration version {} detected, please fix to proper schema!",
                    conf.version
                );
                error!("halting...");
                return Err(ConfigError::ConfigVersion(conf.version));
            }
        }

        let music_dir = conf.music_dir;

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
            audio_file_name_format: conf.audio_file_name_format,
        })
    }
}

#[derive(Clone)]
pub struct Config {
    pub music_dir: PathBuf,
    pub db_new: DbConnection,
    pub cookies: Option<PathBuf>,
    pub yt_playlist_sync: Option<Vec<String>>,
    pub audio_file_name_format: Vec<AudioFileNameComponents>,
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
            audio_file_name_format: Default::default(),
        }
    }
}

mod error {
    use miette::Diagnostic;
    use thiserror::Error;

    #[derive(Error, Diagnostic, Debug)]
    pub enum ConfigError {
        #[error("config version {0} provided is unsupported")]
        ConfigVersion(usize),
        #[error(transparent)]
        EtceteraError(#[from] etcetera::HomeDirError),
        #[error(transparent)]
        IoError(#[from] std::io::Error),
        #[error(transparent)]
        DatabaseError(#[from] crate::database::error::DatabaseError),
        #[error(transparent)]
        TomlDeError(#[from] toml::de::Error),
        #[error(transparent)]
        TomlSerError(#[from] toml::ser::Error),
    }
}
