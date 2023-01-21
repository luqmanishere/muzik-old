use std::path::PathBuf;

use eyre::{eyre, Result};
use image::ImageFormat;
use tracing::{debug, error, trace, warn};

// use super::song::Song;

#[derive(Debug, Clone, Copy)]
pub enum Kid3Op {
    SetMetadata,
}

#[derive(Debug)]
pub struct Kid3Runner {
    file_path: PathBuf,
    commands: Vec<String>,
    op: Option<Kid3Op>,
    set_photo: bool,

    title: Option<Vec<String>>,
    artists: Option<Vec<String>>,
    album: Option<Vec<String>>,
    genre: Option<Vec<String>>,
    picture: Option<String>,
}

impl Kid3Runner {
    pub fn new(
        file_path: PathBuf,
        title: Option<Vec<String>>,
        artists: Option<Vec<String>>,
        album: Option<Vec<String>>,
        genre: Option<Vec<String>>,
    ) -> Kid3Runner {
        Self {
            file_path,
            commands: vec![],
            op: None,
            set_photo: false,
            title,
            artists,
            album,
            genre,
            picture: None,
        }
    }
    pub fn op(self, op: Kid3Op) -> Kid3Runner {
        Self {
            file_path: self.file_path,
            commands: self.commands,
            op: Some(op),
            set_photo: self.set_photo,
            title: self.title,
            artists: self.artists,
            album: self.album,
            genre: self.genre,
            picture: self.picture,
        }
    }

    async fn generate_commands(&mut self) -> Result<()> {
        if let Some(op) = self.op {
            match op {
                Kid3Op::SetMetadata => {
                    self.gen_title();
                    self.gen_artists();
                    self.gen_albums();
                    if self.set_photo {
                        match self.gen_pictures().await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("{}", e)
                            }
                        };
                    }
                }
            }
        } else {
            return Err(eyre!("No operation is set. Please set one!"));
        }
        Ok(())
    }

    pub async fn execute(mut self) -> Result<()> {
        self.generate_commands().await?;
        self.commands
            .push(self.file_path.to_str().unwrap().to_string());
        trace!("{:?}", self.commands);
        let kid3 = tokio::process::Command::new("kid3-cli")
            .args(self.commands)
            .status()
            .await;
        match kid3 {
            Ok(code) => {
                debug!("kid3-cli executed, returned status code: {}", code);
                match std::fs::remove_file(self.file_path.with_extension("png")) {
                    Ok(_) => debug!("Removed temporary picture file"),
                    Err(_) => debug!("No temporary picture file"),
                };
                Ok(())
            }
            Err(e) => {
                error!("kid3-cli failed with error: {}", e);
                return Err(eyre!("kid3-cli failed with error: {}", e));
            }
        }
    }

    pub fn no_photo(self) -> Kid3Runner {
        Self {
            file_path: self.file_path,
            commands: self.commands,
            op: self.op,
            set_photo: false,
            title: self.title,
            artists: self.artists,
            album: self.album,
            genre: self.genre,
            picture: self.picture,
        }
    }

    pub fn with_photo(self, picture: Option<String>) -> Kid3Runner {
        Self {
            file_path: self.file_path,
            commands: self.commands,
            op: self.op,
            set_photo: true,
            title: self.title,
            artists: self.artists,
            album: self.album,
            genre: self.genre,
            picture,
        }
    }
    fn gen_title(&mut self) {
        let titles = match self.title.clone() {
            Some(title) => title,
            None => vec!["Unknown".to_string()],
        };

        for (i, title) in titles.iter().enumerate() {
            let format = format!("set title[{}] \"{}\"", i, title);
            self.commands.append(&mut vec!["-c".to_string(), format]);
        }

        debug!("Appended command to add title");
    }

    fn gen_artists(&mut self) {
        let artists = match self.artists.clone() {
            Some(artists) => artists,
            None => vec!["Unknown".to_string()],
        };

        for (i, artist) in artists.iter().enumerate() {
            let format = format!("set artist[{}] \"{}\"", i, artist);
            self.commands.append(&mut vec!["-c".to_string(), format]);
        }

        debug!("Appended command to add artists");
    }

    fn gen_albums(&mut self) {
        let albums = match self.album.clone() {
            Some(albums) => albums,
            None => vec!["Unknown".to_string()],
        };

        for (i, album) in albums.iter().enumerate() {
            let format = format!("set album[{}] \"{}\"", i, album);
            self.commands.append(&mut vec!["-c".to_string(), format]);
        }

        debug!("Appended command to add albums");
    }

    async fn gen_pictures(&mut self) -> Result<()> {
        let picture = match self.picture.clone() {
            Some(title) => title,
            None => "".to_string(),
        };

        if picture.contains("http") {
            let picture = reqwest::get(picture).await;
            match picture {
                Ok(request) => {
                    let picture = image::load_from_memory(&request.bytes().await?.to_vec())?;
                    trace!("{:?}", picture.color());
                    picture
                        .save_with_format(self.file_path.with_extension("png"), ImageFormat::Png)?;
                }
                Err(e) => {
                    return Err(eyre!("Unable to request picture: {}", e));
                }
            }
            let format = format!(
                "set picture:\"{}\" ''",
                self.file_path.with_extension("png").display()
            );
            self.commands.append(&mut vec!["-c".to_string(), format]);
        } else {
            warn!("No URL found to set albumart");
        }

        debug!("Appended command to add title");
        Ok(())
    }
}
/*
impl From<Song> for Kid3Runner {
    fn from(song: Song) -> Self {
        Self {
            file_path: song.file_path,
            commands: vec![],
            op: None,
            set_photo: { song.thumbnail_url.is_some() },
            title: { song.title.map(|title| vec![title]) },
            artists: song.artists,
            album: { song.album.map(|album| vec![album]) },
            genre: { song.genre.map(|genre| vec![genre]) },
            picture: song.thumbnail_url,
        }
    }
}
*/
