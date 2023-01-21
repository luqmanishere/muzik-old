use std::path::PathBuf;

use eyre::Result;
use rusqlite::{params, Connection, Transaction};

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
                tb_url  TEXT
)",
            (),
        )?;
        Ok(Self { conn, path })
    }

    pub fn get_all(&self, music_dir: PathBuf) -> Result<Vec<Song>> {
        let mut stmt = self.conn.prepare(
            "
SELECT id,path,title,album,artist,yt_id,tb_url FROM songs
",
        )?;

        let s_iter = stmt.query_map([], |row| {
            Ok(Song::new(
                music_dir.clone(),
                row.get(0).ok(),
                row.get(1).ok(),
                row.get(2).ok(),
                row.get(3).ok(),
                row.get(4).ok(),
                row.get(5).ok(),
                row.get(6).ok(),
            ))
        })?;
        let s_vec = s_iter.map(|s| s.unwrap()).collect::<Vec<Song>>();
        Ok(s_vec)
    }

    pub fn insert_entry(&self, song: &Song) -> Result<()> {
        let sql = "
INSERT INTO songs (
                path,
                title,
                album,
                artist,
                yt_id,
                tb_url
) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
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
                song.yt_id.clone().unwrap_or_else(|| "None".to_string()),
                song.tb_url.clone().unwrap_or_else(|| "None".to_string())
            ],
        )?;

        Ok(())
    }

    pub fn update_song(&self, song: &Song) -> Result<()> {
        let sql = "
            UPDATE songs SET
                path = ?2,
                title = ?3,
                album = ?4,
                artist = ?5,
                yt_id = ?6,
                tb_url = ?7
            WHERE id = ?1
        ";
        self.conn.execute(
            sql,
            params![
                song.id,
                song.get_filename(),
                song.title,
                song.get_albums_string(),
                song.get_artists_string(),
                song.yt_id,
                song.tb_url
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
pub struct Song {
    pub id: Option<usize>,
    pub path: Option<PathBuf>,
    pub title: Option<String>,
    pub album: Option<Vec<String>>,
    pub artist: Option<Vec<String>>,
    pub yt_id: Option<String>,
    pub tb_url: Option<String>,
}

impl Song {
    pub fn new(
        music_dir: PathBuf,
        id: Option<usize>,
        fname: Option<String>,
        title: Option<String>,
        album: Option<String>,
        artist: Option<String>,
        yt_id: Option<String>,
        tb_url: Option<String>,
    ) -> Self {
        let path = fname.map(|fname| music_dir.join(fname));

        let album = album.map(|album| {
            album
                .split(';')
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>()
        });

        let artist = artist.map(|artist| {
            artist
                .split(';')
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>()
        });

        Self {
            id,
            path,
            title,
            album,
            artist,
            yt_id,
            tb_url,
        }
    }

    pub fn set_artists(&mut self, artists: String) {
        let artist = artists
            .split(';')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();

        self.artist = Some(artist);
    }

    pub fn set_albums(&mut self, albums: String) {
        let album = albums
            .split(';')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>();

        self.album = Some(album);
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
}
