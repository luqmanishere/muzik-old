use std::path::PathBuf;

use directories::{ProjectDirs, UserDirs};
use eyre::{eyre, Context, Result};
use serde::Deserialize;

use crate::database::Database;

#[derive(Deserialize)]
pub struct ReadConfig {
    version: usize,
    music_dir: Option<PathBuf>,
    cookies: Option<PathBuf>,
    yt_playlist_sync: Option<Vec<String>>,
}

impl ReadConfig {
    /// Read config from provided path
    pub fn read_config(path: Option<PathBuf>) -> Result<Config> {
        let config_path = {
            if let Some(path) = path {
                path
            } else {
                ProjectDirs::from("", "", "muzik")
                    .unwrap()
                    .config_local_dir()
                    .to_owned()
                    .join("config.toml")
            }
        };

        if !config_path.exists() {
            // Create the dir
            std::fs::create_dir_all(config_path.parent().unwrap())?;
            // write version to file
            std::fs::write(&config_path, "version = 1")?;
        }

        let conf: ReadConfig = toml::from_str(&std::fs::read_to_string(config_path)?)
            .wrap_err_with(|| eyre!("Failed to Deserialize config"))?;

        // conf is now version 1
        assert!(conf.version == 1);

        let music_dir = {
            if let Some(dir) = conf.music_dir {
                dir
            } else {
                match UserDirs::new() {
                    Some(user_dirs) => match user_dirs.audio_dir() {
                        Some(audio_dir) => audio_dir.to_path_buf(),
                        None => {
                            if let Ok(_termux_ver) = std::env::var("TERMUX_VERSION") {
                                PathBuf::from(std::env::var("HOME").unwrap()).join("storage/music")
                            } else {
                                PathBuf::from(std::env::var("HOME").unwrap()).join("Music")
                            }
                        }
                    },
                    None => {
                        if let Ok(_termux_ver) = std::env::var("TERMUX_VERSION") {
                            PathBuf::from(std::env::var("HOME").unwrap()).join("storage/music")
                        } else {
                            PathBuf::from(std::env::var("HOME").unwrap()).join("Music")
                        }
                    }
                }
            }
        };

        let db = Database::new(music_dir.join("database.sqlite")).unwrap();

        let cookies = {
            if let Some(cookies_path) = conf.cookies {
                cookies_path
            } else if let Some(project_dir) = ProjectDirs::from("", "", "muzik") {
                println!("{}", project_dir.data_dir().display());
                project_dir.data_dir().join("cookies.txt")
            } else {
                PathBuf::from(std::env::var("HOME").unwrap())
                    .join(".local/share/muzik/cookies.txt")
            }
        };
        Ok(Config {
            music_dir,
            db,
            cookies: if cookies.exists() {
                Some(cookies)
            } else {
                None
            },
            yt_playlist_sync: conf.yt_playlist_sync,
        })
    }
}

pub struct Config {
    pub music_dir: PathBuf,
    pub db: Database,
    pub cookies: Option<PathBuf>,
    pub yt_playlist_sync: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        let music_dir = match UserDirs::new() {
            Some(user_dirs) => match user_dirs.audio_dir() {
                Some(audio_dir) => audio_dir.to_path_buf(),
                None => {
                    if let Ok(_termux_ver) = std::env::var("TERMUX_VERSION") {
                        PathBuf::from(std::env::var("HOME").unwrap()).join("storage/music")
                    } else {
                        PathBuf::from(std::env::var("HOME").unwrap()).join("Music")
                    }
                }
            },
            None => {
                if let Ok(_termux_ver) = std::env::var("TERMUX_VERSION") {
                    PathBuf::from(std::env::var("HOME").unwrap()).join("storage/music")
                } else {
                    PathBuf::from(std::env::var("HOME").unwrap()).join("Music")
                }
            }
        };
        let db = Database::new(music_dir.join("database.sqlite")).unwrap();
        let cookies = if let Some(project_dir) = ProjectDirs::from("", "", "muzik") {
            println!("{}", project_dir.data_dir().display());
            project_dir.data_dir().join("cookies.txt")
        } else {
            PathBuf::from(std::env::var("HOME").unwrap()).join(".local/share/muzik/cookies.txt")
        };
        Self {
            music_dir,
            db,
            cookies: if cookies.exists() {
                Some(cookies)
            } else {
                None
            },
            yt_playlist_sync: None,
        }
    }
}

impl Config {
    pub fn get_music_dir(&self) -> PathBuf {
        self.music_dir.clone()
    }
}
