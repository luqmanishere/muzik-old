use std::{path::PathBuf, sync::Arc};

use strum::Display;
use tracing::debug;

use crate::{
    database::DbConnection,
    entities::{
        album::AlbumModel, artist::ArtistModel, genre::GenreModel,
        youtube_playlist_id::YoutubePlaylistIdModel,
    },
    tags,
};

/// Source of the song file. Other than local is supported download source
#[derive(Debug, Clone, Default, PartialEq, Eq, Display)]
pub enum Source {
    /// Download from Youtube / Youtube Music
    Youtube,
    /// Local file
    #[default]
    Local,
}

/// Song data. Use setters to set data
#[derive(Default, Clone, Debug)]
pub struct Song {
    pub music_dir: PathBuf,
    /// Path to song on the filesystem
    pub path: Option<PathBuf>,
    /// Database id if available
    pub id: Option<i32>,
    /// Title of the song
    pub title: Option<String>,
    /// List of artists of the song
    pub artists: Option<Vec<ArtistModel>>,
    /// List of albums of the song
    pub albums: Option<Vec<AlbumModel>>,
    /// List of genres for the song
    pub genres: Option<Vec<GenreModel>>,

    /// youtube id of song from youtube
    pub youtube_id: Option<String>,
    /// Youtube playlists associated with the song
    youtube_playlists: Option<Vec<YoutubePlaylistIdModel>>,
    /// Online thumbnail source
    pub thumbnail_url: Option<String>,
    /// Local thumbnail
    pub thumbnail: Option<Vec<u8>>,
    /// Source of the file
    pub source: Source,
    pub update_required: bool,
    pub in_database: bool,
}

impl Song {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_path(&mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self.clone()
    }

    pub fn set_id(&mut self, id: i32) -> Self {
        self.id = Some(id);
        self.clone()
    }

    pub fn set_title(&mut self, title: String) -> Self {
        if !title.is_empty() {
            self.title = Some(title);
        }
        self.clone()
    }

    pub fn set_artists(&mut self, artists: Vec<ArtistModel>) -> Self {
        if !artists.is_empty() {
            self.artists = Some(artists);
        }
        self.clone()
    }

    pub fn set_albums(&mut self, albums: Vec<AlbumModel>) -> Self {
        if albums.len() > 0 {
            self.albums = Some(albums);
        }
        self.clone()
    }

    pub fn set_genres(&mut self, genres: Vec<GenreModel>) -> Self {
        if genres.len() > 0 {
            self.genres = Some(genres);
        }
        self.clone()
    }

    pub fn set_youtube_id(&mut self, youtube_id: String) -> Self {
        if !youtube_id.is_empty() {
            self.youtube_id = Some(youtube_id);
        }
        self.clone()
    }

    pub fn set_youtube_playlists(
        &mut self,
        youtube_playlists: Vec<YoutubePlaylistIdModel>,
    ) -> Self {
        if !youtube_playlists.is_empty() {
            self.youtube_playlists = Some(youtube_playlists);
        }
        self.clone()
    }

    pub fn set_thumbnail_url(&mut self, thumbnail_url: String) -> Self {
        if !thumbnail_url.is_empty() {
            self.thumbnail_url = Some(thumbnail_url);
        }
        self.clone()
    }

    pub fn set_source(&mut self, source: Source) -> Self {
        self.source = source;
        self.clone()
    }

    /// Returns database id if in database
    pub fn identify(&self) -> String {
        if let Some(path) = self.path.as_ref() {
            if self.in_database && path.exists() {
                format!(
                    "Database : {} | Local Exists",
                    self.id.expect("in database have id")
                )
            } else if self.in_database && !path.exists() {
                format!(
                    "Database : {} | Local Not Exists",
                    self.id.expect("in database have id")
                )
            } else if path.exists() {
                format!("Not in database | Local Exists")
            } else {
                format!("Not in database | Local Not Exists")
            }
        } else {
            if self.in_database {
                format!(
                    "Database : {} | Local Not Exists",
                    self.id.expect("in database have id")
                )
            } else {
                format!("Not in database | Local Not Exists")
            }
        }
    }

    /// Returns the title, or Unknown if none
    pub fn get_title_string(&self) -> String {
        if let Some(title) = self.title.as_ref() {
            title.clone()
        } else {
            "Unknown".to_string()
        }
    }

    pub fn get_artists_string(&self) -> String {
        if let Some(artists) = self.artists.as_ref() {
            artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<_>>()
                .join("; ")
        } else {
            "Unknown".to_string()
        }
    }

    pub fn get_artists_vec(&self) -> Vec<String> {
        if let Some(artists) = self.artists.as_ref() {
            artists.iter().map(|a| a.name.clone()).collect()
        } else {
            vec!["Unknown".to_string()]
        }
    }
    pub fn get_albums_vec(&self) -> Vec<String> {
        if let Some(albums) = self.albums.as_ref() {
            albums.iter().map(|a| a.name.clone()).collect()
        } else {
            vec!["Unknown".to_string()]
        }
    }

    pub fn get_genres_vec(&self) -> Vec<String> {
        if let Some(genres) = self.genres.as_ref() {
            genres.iter().map(|a| a.genre.clone()).collect()
        } else {
            vec!["Unknown".to_string()]
        }
    }

    pub fn is_database_only(&self) -> bool {
        if let Some(path) = self.path.as_ref() {
            if path.exists() {
                false
            } else {
                true
            }
        } else {
            true
        }
    }

    /// Returns the path without music dir
    ///
    /// Ensure portability between OS
    pub fn get_database_path(&self) -> String {
        if let Some(path) = self.path.as_ref() {
            let mut music_compo = self.music_dir.components();
            let mut path_compo = path.components();
            while music_compo.next().is_some() {
                path_compo.next();
            }
            debug!("{:?}", &path_compo.as_path());
            path_compo
                .as_path()
                .to_str()
                .expect("no file name errors")
                .to_string()
        } else {
            String::new()
        }
    }
}

pub async fn load_songs(music_dir: PathBuf, db: Arc<DbConnection>) -> Vec<Song> {
    let files = walkdir::WalkDir::new(music_dir.clone())
        // only 1 dir deep for now
        .max_depth(1)
        .into_iter()
        .filter(|f| -> bool {
            // only audio files
            mime_guess::from_path(f.as_ref().expect("path success").path())
                .iter()
                .any(|ev| ev.type_() == "audio")
        })
        .map(|e| e.expect("can unwrap path").path().to_owned())
        .collect::<Vec<_>>();

    let (mut svec, id_present) = {
        let mut svec = vec![];
        let mut id_present = vec![];
        for file in files {
            let mut song = tags::read_tags_to_gui_song(file)
                .await
                .expect("can read tags");
            song.music_dir = music_dir.clone();

            // check if song is in database
            let in_database = db.check_song_in_database(&song).await;
            debug!(
                "{} database check: {}",
                song.title.as_ref().unwrap_or(&String::new()),
                in_database
            );
            if in_database {
                // TODO: verify correctness against all values in Song model
                id_present.push(song.id.expect("has id if true in database"));
            }
            song.in_database = in_database;
            song.get_database_path();
            svec.push(song)
        }
        (svec, id_present)
    };

    for sonsgs in db
        .get_remaining_entries(id_present)
        .await
        .expect("not fail")
    {
        svec.push(sonsgs);
    }

    svec
}
