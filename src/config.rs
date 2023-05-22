use std::path::PathBuf;

use directories::{ProjectDirs, UserDirs};

use crate::database::Database;

pub struct Config {
    pub music_dir: PathBuf,
    pub db: Database,
    pub cookies: Option<PathBuf>,
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
        // TODO: cookies default path
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
        }
    }
}

impl Config {
    pub fn get_music_dir(&self) -> PathBuf {
        self.music_dir.clone()
    }
}
