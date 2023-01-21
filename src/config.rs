use std::path::PathBuf;

use crate::database::Database;

pub struct Config {
    pub music_dir: PathBuf,
    pub db: Database,
}

impl Config {
    pub fn get_music_dir(&self) -> PathBuf {
        self.music_dir.clone()
    }
}
