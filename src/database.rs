use std::path::PathBuf;

use eyre::Result;
use rusqlite::{params, Connection, Transaction};
use tracing::info;

#[allow(dead_code)]
pub struct Database {
    pub conn: Connection,
    path: PathBuf,
}

#[allow(dead_code)]
impl Database {
    pub fn new(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS songs (
                id  INTEGER PRIMARY KEY,
                path    TEXT NOT NULL,
                title   TEXT NOT NULL,
                album   TEXT,
                artist  TEXT,
                yt_id   TEXT,
                tb_url  TEXT,
                genre   TEXT,
                yt_playlist_id TEXT
)",
            (),
        )?;
        Ok(Self { conn, path })
    }

    pub fn get_all(&self, music_dir: PathBuf) -> Result<Vec<Song>> {
        let mut stmt = self.conn.prepare(
            "
SELECT id,path,title,album,artist,genre,yt_id,tb_url,yt_playlist_id FROM songs
",
        )?;

        let s_iter = stmt.query_map([], |row| {
            Ok(Song::new()
                .music_dir(Some(music_dir.clone()))
                .id(row.get(0).ok())
                .fname(row.get(1).ok())
                .title(row.get(2).ok())
                .albums(row.get(3).ok())
                .artists(row.get(4).ok())
                .genre(row.get(5).ok())
                .yt_id(row.get(6).ok())
                .tb_url(row.get(7).ok())
                .yt_playlist_id(row.get(8).ok()))
        })?;
        let s_vec = s_iter.map(|s| s.unwrap()).collect::<Vec<Song>>();
        Ok(s_vec)
    }

    pub fn insert_entry(&self, song: &Song) -> Result<()> {
        match song.id {
            Some(_id) => {
                info!("song already exists in database");
            }
            None => {
                let sql = "
INSERT INTO songs (
                path,
                title,
                album,
                artist,
                genre,
                yt_id,
                tb_url,
                yt_playlist_id
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
";
                let fname = if let Some(path) = &song.path {
                    path.file_name().unwrap().to_str().unwrap().to_string()
                } else {
                    "Unknown".to_string()
                };
                self.conn.execute(
                    sql,
                    params![
                        fname,
                        song.title.clone().unwrap_or_else(|| "Unknown".to_string()),
                        song.get_albums_string(),
                        song.get_artists_string(),
                        song.get_genre_string(),
                        song.yt_id.clone().unwrap_or_else(|| "None".to_string()),
                        song.tb_url.clone().unwrap_or_else(|| "None".to_string()),
                        song.yt_playlist
                            .clone()
                            .unwrap_or_else(|| "None".to_string())
                    ],
                )?;
            }
        }

        Ok(())
    }

    pub fn update_song(&self, song: &Song) -> Result<()> {
        let sql = "
            UPDATE songs SET
                path = ?2,
                title = ?3,
                album = ?4,
                artist = ?5,
                genre = ?6,
                yt_id = ?7,
                tb_url = ?8,
                yt_playlist_id = ?9
            WHERE id = ?1
        ";
        self.conn.execute(
            sql,
            params![
                song.id,
                if song.npath.is_none() {
                    song.get_filename()
                } else {
                    song.get_new_filename()
                },
                song.title,
                song.get_albums_string(),
                song.get_artists_string(),
                song.get_genre_string(),
                song.yt_id,
                song.tb_url,
                song.yt_playlist
            ],
        )?;
        Ok(())
    }
    pub fn delete_entry_by_id(&self, id: usize) -> Result<()> {
        let sql = "DELETE FROM songs WHERE id = ?";
        self.conn.execute(sql, params![id])?;
        Ok(())
    }

    pub fn get_database_path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn get_transaction(&mut self) -> Result<Transaction> {
        Ok(self.conn.transaction()?)
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum PlaylistStatus {
    Download,
    NoDownload,
}

#[derive(Clone)]
pub struct Song {
    pub id: Option<usize>,
    pub music_dir: Option<PathBuf>,
    pub path: Option<PathBuf>,
    pub title: Option<String>,
    pub album: Option<Vec<String>>,
    pub artist: Option<Vec<String>>,
    pub genre: Option<Vec<String>>,
    pub yt_id: Option<String>,
    pub tb_url: Option<String>,
    pub npath: Option<PathBuf>,
    // youtube playlist
    pub yt_playlist: Option<String>,
}

impl Song {
    /// Create a new instance of `Song`
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, id: Option<usize>) -> Self {
        self.id = id;
        self
    }

    pub fn fname(mut self, fname: Option<String>) -> Self {
        assert!(self.music_dir.is_some());
        let music_dir = self.music_dir.clone().unwrap();
        let path = music_dir.join(fname.unwrap());
        self.path = Some(path);
        self
    }

    pub fn music_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.music_dir = dir;
        self
    }

    pub fn title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    pub fn artists(mut self, artists: Option<String>) -> Self {
        let artists = artists.unwrap_or("Unknown".to_string());
        let artists_vec = artists
            .split(';')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();

        self.artist = Some(artists_vec);
        self.compute_filename();
        self
    }

    pub fn albums(mut self, albums: Option<String>) -> Self {
        let albums = albums.unwrap_or("Unknown".to_string());
        let albums_vec = albums
            .split(';')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();

        // here lies a dreadful mistake caused by copy-paste
        // self.artist = Some(albums_vec);
        self.album = Some(albums_vec);
        self.compute_filename();
        self
    }

    pub fn genre(mut self, genre: Option<String>) -> Self {
        let genre = genre.unwrap_or("Unknown".to_string());
        let genre_vec = genre
            .split(';')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();

        self.genre = Some(genre_vec);
        self
    }

    pub fn yt_id(mut self, yt_id: Option<String>) -> Self {
        self.yt_id = yt_id;
        self
    }

    pub fn tb_url(mut self, tb_url: Option<String>) -> Self {
        self.tb_url = tb_url;
        self
    }

    pub fn yt_playlist_id(mut self, yt_playlist_id: Option<String>) -> Self {
        self.yt_playlist = yt_playlist_id;
        self
    }

    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
        self.compute_filename();
    }

    pub fn set_artists(&mut self, artists: String) {
        let artist = artists
            .split(';')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();

        self.artist = Some(artist);
        self.compute_filename();
    }

    pub fn set_albums(&mut self, albums: String) {
        let album = albums
            .split(';')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();

        self.album = Some(album);
        self.compute_filename();
    }

    pub fn set_genre(&mut self, genre: String) {
        let genre = genre
            .split(';')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();

        self.genre = Some(genre);
        self.compute_filename();
    }

    pub fn get_music_dir(&self) -> PathBuf {
        self.music_dir.clone().unwrap_or_default()
    }

    pub fn get_yt_id(&self) -> Option<String> {
        self.yt_id.as_ref().cloned()
    }

    pub fn get_title_string(&self) -> String {
        if let Some(title) = &self.title {
            title.clone()
        } else {
            "Unknonw".to_string()
        }
    }
    pub fn get_artists_string(&self) -> String {
        if let Some(artist) = &self.artist {
            artist.join("; ")
        } else {
            "Unknown".to_string()
        }
    }

    pub fn get_albums_string(&self) -> String {
        if let Some(album) = &self.album {
            album.join("; ")
        } else {
            "Unknown".to_string()
        }
    }

    pub fn get_genre_string(&self) -> String {
        if let Some(genre) = &self.genre {
            genre.join("; ")
        } else {
            "Unknown".to_string()
        }
    }

    pub fn get_filename(&self) -> String {
        self.path
            .clone()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    pub fn get_new_filename(&self) -> String {
        self.npath
            .clone()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    pub fn compute_filename(&mut self) {
        let fname = format!(
            "{} - {}.opus",
            self.title.clone().unwrap_or_else(|| "Unknown".to_string()),
            self.get_artists_string()
        );

        if let Some(path) = self.path.clone() {
            let new_path = path.with_file_name(fname);
            self.npath = Some(new_path);
        }
    }
}

/// This `Default` impl is only meant to be used as a placeholder
impl Default for Song {
    fn default() -> Self {
        Self {
            id: None,
            music_dir: None,
            path: None,
            title: None,
            album: None,
            artist: None,
            genre: None,
            yt_id: None,
            tb_url: None,
            npath: None,
            yt_playlist: None,
        }
    }
}
