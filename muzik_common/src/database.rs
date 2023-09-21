use std::path::PathBuf;

use sea_orm_migration::SchemaManager;
use tracing::{debug, info, trace, warn};

#[derive(Clone, Debug)]
pub struct AppSong {
    // id is None before insertion
    pub id: Option<i32>,
    pub music_dir: Option<PathBuf>,
    pub path: Option<PathBuf>,

    pub title: Option<String>,
    pub yt_id: Option<String>,
    pub tb_url: Option<String>,

    pub album: Option<Vec<album::Model>>,
    pub artist: Option<Vec<artist::Model>>,
    pub genre: Option<Vec<genre::Model>>,
    pub yt_playlist: Option<Vec<youtube_playlist_id::Model>>,
    pub npath: Option<PathBuf>,
}

impl AppSong {
    /// Create a new instance of `Song`
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id(mut self, id: Option<i32>) -> Self {
        self.id = id;
        self
    }

    pub fn with_music_dir(mut self, dir: Option<PathBuf>) -> Self {
        self.music_dir = dir;
        self
    }

    pub fn with_title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    /// Converts an artist name to a `Model`, with an id of 0 because it is unknown
    pub fn with_artists_string(mut self, artists: String) -> Self {
        let artists_vec = artists
            .split(';')
            .map(|s| artist::Model {
                id: 0,
                name: s.trim().to_string(),
            })
            .collect::<Vec<_>>();

        self.artist = Some(artists_vec);
        self
    }

    pub fn with_albums(mut self, albums: String) -> Self {
        let albums_vec = albums
            .split(';')
            .map(|s| album::Model {
                id: 0,
                name: s.trim().to_string(),
            })
            .collect::<Vec<_>>();

        // here lies a dreadful mistake caused by copy-paste
        // self.artist = Some(albums_vec);
        self.album = Some(albums_vec);
        self
    }

    pub fn with_genre(mut self, genre: String) -> Self {
        let genre_vec = genre
            .split(';')
            .map(|s| genre::Model {
                id: 0,
                genre: s.trim().to_string(),
            })
            .collect::<Vec<_>>();

        self.genre = Some(genre_vec);
        self
    }

    pub fn with_yt_id(mut self, yt_id: Option<String>) -> Self {
        self.yt_id = yt_id;
        self
    }

    pub fn with_tb_url(mut self, tb_url: Option<String>) -> Self {
        self.tb_url = tb_url;
        self
    }

    pub fn with_yt_playlist_id(mut self, yt_playlist_id: Option<String>) -> Self {
        let yt_playlist_id = yt_playlist_id.unwrap_or("Unknown".to_string());
        let yt_playlist_id_vec = yt_playlist_id
            .split(';')
            .map(|s| youtube_playlist_id::Model {
                id: 0,
                youtube_playlist_id: s.trim().to_string(),
            })
            .collect::<Vec<_>>();

        self.yt_playlist = Some(yt_playlist_id_vec);
        self
    }

    pub fn compute_new_filename(mut self) -> Self {
        let fname = format!(
            "{} - {}.opus",
            self.get_title_string(),
            self.get_artists_string()
        );

        let new_path = self.music_dir.clone().unwrap().join(fname);
        self.path = Some(new_path);
        self
    }

    pub fn add_artist(&mut self, artist: artist::Model) -> Self {
        if let Some(mut artists_vec) = self.artist.clone() {
            artists_vec.push(artist);
            self.artist = Some(artists_vec);
        } else {
            self.artist = Some(vec![artist]);
        }
        self.clone()
    }

    pub fn add_album(&mut self, album: album::Model) -> Self {
        if let Some(mut albums_vec) = self.album.clone() {
            albums_vec.push(album);
            self.album = Some(albums_vec);
        } else {
            self.album = Some(vec![album]);
        }
        self.clone()
    }

    pub fn add_genre(&mut self, genre: genre::Model) -> Self {
        if let Some(mut genre_vec) = self.genre.clone() {
            genre_vec.push(genre);
            self.genre = Some(genre_vec);
        } else {
            self.genre = Some(vec![genre]);
        }
        self.clone()
    }

    pub fn add_yt_playlist_id(&mut self, youtube_playlist_id: youtube_playlist_id::Model) -> Self {
        if let Some(mut yt_playlist_id_vec) = self.yt_playlist.clone() {
            yt_playlist_id_vec.push(youtube_playlist_id);
            self.yt_playlist = Some(yt_playlist_id_vec);
        } else {
            self.yt_playlist = Some(vec![youtube_playlist_id]);
        }
        self.clone()
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

    #[tracing::instrument(skip(self), fields(id=self.id))]
    #[allow(dead_code)]
    pub fn get_artists_string(&self) -> String {
        if let Some(artist) = &self.artist {
            let artist = artist
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<_>>()
                .join("; ");
            debug!("artist string: {}", artist);
            artist
        } else {
            "Unknown".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn get_albums_string(&self) -> String {
        if let Some(album) = &self.album {
            album
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<_>>()
                .join("; ")
        } else {
            "Unknown".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn get_genre_string(&self) -> String {
        if let Some(genre) = &self.genre {
            genre
                .iter()
                .map(|a| a.genre.clone())
                .collect::<Vec<_>>()
                .join("; ")
        } else {
            "Unknown".to_string()
        }
    }

    #[allow(dead_code)]
    pub fn get_youtube_playlist_id_string(&self) -> String {
        if let Some(youtube_playlist_id) = &self.yt_playlist {
            youtube_playlist_id
                .iter()
                .map(|a| a.youtube_playlist_id.clone())
                .collect::<Vec<_>>()
                .join("; ")
        } else {
            "Unknown".to_string()
        }
    }

    pub fn get_full_path(&self) -> PathBuf {
        self.path
            .as_ref()
            .expect("path should not be empty when this function is called")
            .clone()
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    /// Used to give the filename which is generated from the title, artist, and yt_id
    pub fn compute_filename(&mut self) {
        let fname = format!(
            "{} - {} {}.opus",
            self.get_title_string(),
            self.get_artists_string(),
            self.get_yt_id().unwrap()
        );

        if let Some(path) = self.path.clone() {
            let new_path = path.with_file_name(fname);
            self.path = Some(new_path);
        } else {
            let new_path = self.music_dir.clone().unwrap().join(fname);
            self.path = Some(new_path);
        }
    }

    /// trigger a change in the filename
    pub fn change_filename(&mut self) {
        let fname = format!(
            "{} - {}.opus",
            self.get_title_string(),
            self.get_artists_string()
        );

        if let Some(path) = self.path.clone() {
            let new_path = path.with_file_name(fname);
            self.npath = Some(new_path);
        } else {
            let new_path = self.music_dir.clone().unwrap().join(fname);
            self.path = Some(new_path);
        }
    }
}

/// This `Default` impl is only meant to be used as a placeholder
impl Default for AppSong {
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

// here begins all seaorm dev
#[allow(dead_code)]
#[derive(Clone)]
pub struct DbConnection {
    path: Option<PathBuf>,
    db: Option<DatabaseConnection>,
}

use crate::{
    data::{Song as GSong, Source},
    entities::{
        album::AlbumModel, artist::ArtistModel, genre::GenreModel, prelude::*, song::SongModel, *,
    },
};
use sea_orm::{prelude::*, ActiveValue, ConnectOptions, QuerySelect};
use sea_orm_migration::prelude::*;

use self::error::DatabaseError;

impl DbConnection {
    pub fn default() -> Self {
        Self {
            path: None,
            db: None,
        }
    }
    pub async fn new(path: PathBuf) -> Result<Self, DatabaseError> {
        let db_path = format!("sqlite:{}?mode=rwc", path.display());
        let mut opt = ConnectOptions::new(db_path);
        opt.sqlx_logging(true)
            .sqlx_logging_level(tracing::log::LevelFilter::Trace);
        let db = sea_orm::Database::connect(opt).await?;

        let _schema_manager = SchemaManager::new(&db);

        // ensure up to date
        crate::migrator::Migrator::up(&db, None).await?;

        Ok(Self {
            path: Some(path),
            db: Some(db),
        })
    }

    pub async fn m_new(path: PathBuf) -> Result<Self, DatabaseError> {
        let db_path = format!("sqlite:{}?mode=rwc", path.display());
        let mut opt = ConnectOptions::new(db_path);
        opt.sqlx_logging(true)
            .sqlx_logging_level(tracing::log::LevelFilter::Trace);
        let db = sea_orm::Database::connect(opt).await?;

        let _schema_manager = SchemaManager::new(&db);

        // ensure up to date
        crate::migrator::Migrator::up(&db, None).await?;

        Ok(Self {
            path: Some(path),
            db: Some(db),
        })
    }

    pub fn ref_db(&self) -> &DatabaseConnection {
        self.db.as_ref().unwrap()
    }
    pub async fn open_in_memory() -> Self {
        let mut opt = ConnectOptions::new("sqlite::memory:".to_owned());
        opt.sqlx_logging(true)
            .sqlx_logging_level(tracing::log::LevelFilter::Debug);
        let db = sea_orm::Database::connect(opt).await.expect("success");
        info!("spawned new in memory sqlite database");

        Self {
            path: None,
            db: Some(db),
        }
    }

    /// insert an entry into the `artist` table
    #[tracing::instrument(skip(self))]
    pub async fn insert_artist(&self, artist: String) -> Result<i32, DatabaseError> {
        let artist_model = Artist::find()
            .filter(artist::Column::Name.eq(artist.clone()))
            .one(self.ref_db())
            .await?;
        if let Some(artist) = artist_model {
            warn!("artist {} already exists in database", artist.name);
            Ok(artist.id)
        } else {
            let model = artist::ActiveModel {
                name: ActiveValue::Set(artist.to_owned()),
                ..Default::default()
            };
            Ok(Artist::insert(model)
                .exec(self.ref_db())
                .await?
                .last_insert_id)
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_song_artist(
        &self,
        artist_id: i32,
        song_id: i32,
    ) -> Result<i32, DatabaseError> {
        let model = song_artist_junction::ActiveModel {
            song_id: ActiveValue::Set(song_id),
            artist_id: ActiveValue::Set(artist_id),
            ..Default::default()
        };
        Ok(SongArtistJunction::insert(model)
            .exec(self.ref_db())
            .await?
            .last_insert_id)
    }

    /// insert an entry into the `album` table
    #[tracing::instrument(skip(self))]
    pub async fn insert_album(&self, album: String) -> Result<i32, DatabaseError> {
        let album_model = Album::find()
            .filter(album::Column::Name.eq(album.clone()))
            .one(self.ref_db())
            .await?;
        if let Some(album) = album_model {
            warn!("album {} exists in database", album.name);
            Ok(album.id)
        } else {
            let model = album::ActiveModel {
                name: ActiveValue::Set(album.to_owned()),
                ..Default::default()
            };
            Ok(Album::insert(model)
                .exec(self.ref_db())
                .await?
                .last_insert_id)
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_song_album(
        &self,
        album_id: i32,
        song_id: i32,
    ) -> Result<i32, DatabaseError> {
        let model = song_album_junction::ActiveModel {
            song_id: ActiveValue::Set(song_id),
            album_id: ActiveValue::Set(album_id),
            ..Default::default()
        };
        Ok(SongAlbumJunction::insert(model)
            .exec(self.ref_db())
            .await?
            .last_insert_id)
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_genre(&self, genre: String) -> Result<i32, DatabaseError> {
        let genre_model = Genre::find()
            .filter(genre::Column::Genre.eq(genre.clone()))
            .one(self.ref_db())
            .await?;
        if let Some(genre) = genre_model {
            warn!("genre {} exists in database", genre.genre);
            Ok(genre.id)
        } else {
            let model = genre::ActiveModel {
                genre: ActiveValue::Set(genre.to_owned()),
                ..Default::default()
            };
            Ok(Genre::insert(model)
                .exec(self.ref_db())
                .await?
                .last_insert_id)
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_song_genre(
        &self,
        genre_id: i32,
        song_id: i32,
    ) -> Result<i32, DatabaseError> {
        let model = song_genre_junction::ActiveModel {
            song_id: ActiveValue::Set(song_id),
            genre_id: ActiveValue::Set(genre_id),
            ..Default::default()
        };

        Ok(SongGenreJunction::insert(model)
            .exec(self.ref_db())
            .await?
            .last_insert_id)
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_youtube_playlist_id(
        &self,
        youtube_playlist_id: String,
    ) -> Result<i32, DatabaseError> {
        let youtube_playlist_id_model = YoutubePlaylistId::find()
            .filter(youtube_playlist_id::Column::YoutubePlaylistId.eq(youtube_playlist_id.clone()))
            .one(self.ref_db())
            .await?;
        if let Some(youtube_playlist_id) = youtube_playlist_id_model {
            warn!(
                "youtube playlist id {} exists in database",
                youtube_playlist_id.youtube_playlist_id
            );
            Ok(youtube_playlist_id.id)
        } else {
            let model = youtube_playlist_id::ActiveModel {
                youtube_playlist_id: ActiveValue::Set(youtube_playlist_id.to_owned()),
                ..Default::default()
            };
            Ok(YoutubePlaylistId::insert(model)
                .exec(self.ref_db())
                .await?
                .last_insert_id)
        }
    }

    pub async fn insert_song_youtube_playlist_id(
        &self,
        youtube_playlist_id_id: i32,
        song_id: i32,
    ) -> Result<i32, DatabaseError> {
        let model = song_youtube_playlist_id_junction::ActiveModel {
            song_id: ActiveValue::Set(song_id),
            youtube_playlist_id_id: ActiveValue::Set(youtube_playlist_id_id),
            ..Default::default()
        };

        Ok(SongYoutubePlaylistIdJunction::insert(model)
            .exec(self.ref_db())
            .await?
            .last_insert_id)
    }

    #[tracing::instrument(skip(self))]
    pub async fn insert_song(
        &self,
        title: String,
        youtube_id: Option<String>,
        thumbnail_url: Option<String>,
    ) -> Result<i32, DatabaseError> {
        let model = song::ActiveModel {
            title: ActiveValue::Set(title),
            youtube_id: ActiveValue::Set(youtube_id),
            thumbnail_url: ActiveValue::Set(thumbnail_url),
            ..Default::default()
        };

        Ok(SongEntity::insert(model)
            .exec(self.ref_db())
            .await?
            .last_insert_id)
    }

    pub async fn insert_song_with_path(
        &self,
        title: String,
        youtube_id: Option<String>,
        thumbnail_url: Option<String>,
        path: Option<String>,
    ) -> Result<i32, DatabaseError> {
        let model = song::ActiveModel {
            title: ActiveValue::Set(title),
            youtube_id: ActiveValue::Set(youtube_id),
            thumbnail_url: ActiveValue::Set(thumbnail_url),
            // should only be the filename to ensure crossplatform
            path: ActiveValue::Set(path),
            ..Default::default()
        };

        Ok(SongEntity::insert(model)
            .exec(self.ref_db())
            .await?
            .last_insert_id)
    }

    pub async fn get_all_songs_gui(&self, music_dir: PathBuf) -> Vec<GSong> {
        let songs = song::Entity::find()
            .all(self.ref_db())
            .await
            .unwrap_or(vec![]);
        let mut vvec = vec![];
        for s in songs {
            let mut new_song = GSong::new()
                .set_path(PathBuf::from(
                    music_dir.join(s.path.expect("must have partial path")),
                ))
                .set_id(s.id)
                .set_youtube_id(s.youtube_id.unwrap_or_default())
                .set_thumbnail_url(s.thumbnail_url.unwrap_or_default())
                .set_title(s.title);

            let artists = {
                let mut a_vec = vec![];
                for (_sa, mut artists) in song::Entity::find()
                    .find_with_related(Artist)
                    .filter(song::Column::Id.eq(s.id))
                    .all(self.ref_db())
                    .await
                    .unwrap_or(vec![])
                {
                    artists.sort_by_key(|s1| s1.id);
                    for artist in artists {
                        a_vec.push(artist);
                    }
                }
                a_vec
            };
            new_song.set_artists(artists);

            let albums = {
                let mut albums_v = vec![];
                for (_, albums) in song::Entity::find()
                    .find_with_related(Album)
                    .filter(song::Column::Id.eq(s.id))
                    .all(self.ref_db())
                    .await
                    .unwrap_or(vec![])
                {
                    for album in albums {
                        albums_v.push(album);
                    }
                }
                albums_v
            };
            new_song.set_albums(albums);

            let genres = {
                let mut genres_v = vec![];
                for (_, genres) in song::Entity::find()
                    .find_with_related(Genre)
                    .filter(song::Column::Id.eq(s.id))
                    .all(self.ref_db())
                    .await
                    .unwrap_or(vec![])
                {
                    for genre in genres {
                        genres_v.push(genre);
                    }
                }
                genres_v
            };
            new_song.set_genres(genres);

            let youtube_playlist_ids = {
                let mut yt_p_id = vec![];
                for (_, youtube_playlist_id_model) in song::Entity::find()
                    .find_with_related(YoutubePlaylistId)
                    .filter(song::Column::Id.eq(s.id))
                    .all(self.ref_db())
                    .await
                    .unwrap_or(vec![])
                {
                    for youtube_playlist_id in youtube_playlist_id_model {
                        yt_p_id.push(youtube_playlist_id);
                    }
                }
                yt_p_id
            };
            new_song.set_youtube_playlists(youtube_playlist_ids);
            vvec.push(new_song);
        }
        vvec
    }

    pub async fn get_all_songs_empty(&self, music_dir: PathBuf) -> Vec<AppSong> {
        let songs = song::Entity::find()
            .all(self.ref_db())
            .await
            .unwrap_or(vec![]);
        let mut vvec = vec![];
        for s in songs {
            let mut new_song = AppSong::new()
                .with_music_dir(Some(music_dir.clone()))
                .with_id(Some(s.id))
                .with_yt_id(s.youtube_id)
                .with_tb_url(s.thumbnail_url)
                .with_title(Some(s.title));
            for (_sa, mut artists) in song::Entity::find()
                .find_with_related(Artist)
                .filter(song::Column::Id.eq(s.id))
                .all(self.ref_db())
                .await
                .unwrap_or(vec![])
            {
                artists.sort_by_key(|s1| s1.id);
                for artist in artists {
                    new_song.add_artist(artist);
                }
            }
            for (_, albums) in song::Entity::find()
                .find_with_related(Album)
                .filter(song::Column::Id.eq(s.id))
                .all(self.ref_db())
                .await
                .unwrap_or(vec![])
            {
                for album in albums {
                    new_song.add_album(album);
                }
            }
            for (_, genres) in song::Entity::find()
                .find_with_related(Genre)
                .filter(song::Column::Id.eq(s.id))
                .all(self.ref_db())
                .await
                .unwrap_or(vec![])
            {
                for genre in genres {
                    new_song.add_genre(genre);
                }
            }

            for (_, youtube_playlist_id_model) in song::Entity::find()
                .find_with_related(YoutubePlaylistId)
                .filter(song::Column::Id.eq(s.id))
                .all(self.ref_db())
                .await
                .unwrap_or(vec![])
            {
                for youtube_playlist_id in youtube_playlist_id_model {
                    new_song.add_yt_playlist_id(youtube_playlist_id);
                }
            }
            new_song.compute_filename();
            vvec.push(new_song);
        }
        vvec
    }
    pub async fn get_all_songs(&self, music_dir: PathBuf) -> Result<Vec<AppSong>, DatabaseError> {
        let songs = song::Entity::find().all(self.ref_db()).await?;
        let mut vvec = vec![];
        for s in songs {
            let mut new_song = AppSong::new()
                .with_music_dir(Some(music_dir.clone()))
                .with_id(Some(s.id))
                .with_yt_id(s.youtube_id)
                .with_tb_url(s.thumbnail_url)
                .with_title(Some(s.title));
            for (_sa, mut artists) in song::Entity::find()
                .find_with_related(Artist)
                .filter(song::Column::Id.eq(s.id))
                .all(self.ref_db())
                .await?
            {
                artists.sort_by_key(|s1| s1.id);
                for artist in artists {
                    new_song.add_artist(artist);
                }
            }
            for (_, albums) in song::Entity::find()
                .find_with_related(Album)
                .filter(song::Column::Id.eq(s.id))
                .all(self.ref_db())
                .await?
            {
                for album in albums {
                    new_song.add_album(album);
                }
            }
            for (_, genres) in song::Entity::find()
                .find_with_related(Genre)
                .filter(song::Column::Id.eq(s.id))
                .all(self.ref_db())
                .await?
            {
                for genre in genres {
                    new_song.add_genre(genre);
                }
            }

            for (_, youtube_playlist_id_model) in song::Entity::find()
                .find_with_related(YoutubePlaylistId)
                .filter(song::Column::Id.eq(s.id))
                .all(self.ref_db())
                .await?
            {
                for youtube_playlist_id in youtube_playlist_id_model {
                    new_song.add_yt_playlist_id(youtube_playlist_id);
                }
            }
            new_song.compute_filename();
            vvec.push(new_song);
        }
        Ok(vvec)
    }

    pub async fn get_all_artists(&self) -> Result<Vec<String>, DatabaseError> {
        let artists_vec = Artist::find()
            .select_only()
            .column(artist::Column::Name)
            .all(self.ref_db())
            .await?;

        Ok(artists_vec
            .iter()
            .map(|f| f.name.clone())
            .collect::<Vec<String>>())
    }

    pub async fn get_remaining_entries(
        &self,
        present: Vec<i32>,
    ) -> Result<Vec<GSong>, DatabaseError> {
        let songs = SongEntity::find().all(self.ref_db()).await?;

        let diff: Vec<_> = songs
            .into_iter()
            .filter(|item| !present.contains(&item.id))
            .collect();

        let mut vvec = vec![];
        for s in diff {
            // TODO: wrap this in a function
            let mut new_song = GSong::new()
                .set_path(PathBuf::from(s.path.unwrap_or_default()))
                .set_id(s.id)
                // .set_youtube_id(s.youtube_id.unwrap_or_default())
                .set_thumbnail_url(s.thumbnail_url.unwrap_or_default())
                .set_title(s.title);

            if let Some(youtube_id) = s.youtube_id {
                new_song.set_youtube_id(youtube_id);
                new_song.set_source(Source::Youtube);

                // youtube playlist should only exist if source is youtube
                let youtube_playlist_ids = {
                    let mut yt_p_id = vec![];
                    for (_, youtube_playlist_id_model) in song::Entity::find()
                        .find_with_related(YoutubePlaylistId)
                        .filter(song::Column::Id.eq(s.id))
                        .all(self.ref_db())
                        .await
                        .unwrap_or(vec![])
                    {
                        for youtube_playlist_id in youtube_playlist_id_model {
                            yt_p_id.push(youtube_playlist_id);
                        }
                    }
                    yt_p_id
                };
                new_song.set_youtube_playlists(youtube_playlist_ids);
            }

            let artists = {
                let mut a_vec = vec![];
                for (_sa, mut artists) in song::Entity::find()
                    .find_with_related(Artist)
                    .filter(song::Column::Id.eq(s.id))
                    .all(self.ref_db())
                    .await
                    .unwrap_or(vec![])
                {
                    artists.sort_by_key(|s1| s1.id);
                    for artist in artists {
                        a_vec.push(artist);
                    }
                }
                a_vec
            };
            new_song.set_artists(artists);

            let albums = {
                let mut albums_v = vec![];
                for (_, albums) in song::Entity::find()
                    .find_with_related(Album)
                    .filter(song::Column::Id.eq(s.id))
                    .all(self.ref_db())
                    .await
                    .unwrap_or(vec![])
                {
                    for album in albums {
                        albums_v.push(album);
                    }
                }
                albums_v
            };
            new_song.set_albums(albums);

            let genres = {
                let mut genres_v = vec![];
                for (_, genres) in song::Entity::find()
                    .find_with_related(Genre)
                    .filter(song::Column::Id.eq(s.id))
                    .all(self.ref_db())
                    .await
                    .unwrap_or(vec![])
                {
                    for genre in genres {
                        genres_v.push(genre);
                    }
                }
                genres_v
            };
            new_song.set_genres(genres);

            new_song.in_database = true;
            vvec.push(new_song);
        }
        Ok(vvec)
    }

    pub async fn check_song_in_database(&self, song: &GSong) -> bool {
        // check if id exists
        if let Some(db_id) = song.id {
            let possible_song: Option<song::Model> = song::Entity::find_by_id(db_id)
                .one(self.ref_db())
                .await
                .expect("operation success");
            match possible_song {
                Some(possible_song) => {
                    // compare elements parsed from the file and from song only
                    let filename = song
                        .path
                        .as_ref()
                        .expect("has path")
                        .file_name()
                        .expect("has filename")
                        .to_str()
                        .expect("to_str")
                        .to_string();

                    // TODO: compare other fields as well
                    possible_song.title == song.get_title_string()
                        && possible_song.path.unwrap_or_default() == filename
                }
                None => false,
            }
        } else {
            false
        }
    }

    pub async fn insert_from_gui_song(&self, mut song: GSong) -> Result<GSong, DatabaseError> {
        let title = song.get_title_string();
        let youtube_id = song.youtube_id.clone();
        let thumbnail_url = song.thumbnail_url.clone();
        let path = song.get_database_path();
        let song_id = self
            .insert_song_with_path(title, youtube_id, thumbnail_url, Some(path))
            .await?;

        let artists_vec = song.artists.clone().unwrap_or(vec![]);
        for artist in artists_vec {
            let artist_id = self.insert_artist(artist.name).await?;
            self.insert_song_artist(artist_id, song_id).await?;
        }

        let albums_vec = song.albums.clone().unwrap_or(vec![]);
        for album in albums_vec {
            let album_id = self.insert_album(album.name).await?;
            self.insert_song_album(album_id, song_id).await?;
        }

        let genres_vec = song.genres.clone().unwrap_or(vec![]);
        for genre in genres_vec {
            let genre_id = self.insert_genre(genre.genre).await?;
            self.insert_song_genre(genre_id, song_id).await?;
        }

        Ok(song.set_id(song_id))
    }
    pub async fn insert_from_app_song(&self, song: AppSong) -> Result<(), DatabaseError> {
        let title = song.get_title_string();
        let youtube_id = song.yt_id;
        let thumbnail_url = song.tb_url;
        let song_id = self.insert_song(title, youtube_id, thumbnail_url).await?;

        let artists_vec = song.artist.clone().unwrap_or(vec![]);
        for artist in artists_vec {
            let artist_id = self.insert_artist(artist.name).await?;
            self.insert_song_artist(artist_id, song_id).await?;
        }

        let albums_vec = song.album.clone().unwrap_or(vec![]);
        for album in albums_vec {
            let album_id = self.insert_album(album.name).await?;
            self.insert_song_album(album_id, song_id).await?;
        }

        let genres_vec = song.genre.clone().unwrap_or(vec![]);
        for genre in genres_vec {
            let genre_id = self.insert_genre(genre.genre).await?;
            self.insert_song_genre(genre_id, song_id).await?;
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_song(
        &self,
        song_id: i32,
        title: String,
        youtube_id: Option<String>,
        thumbnail_url: Option<String>,
        path: Option<String>,
    ) -> Result<i32, DatabaseError> {
        let model = song::ActiveModel {
            id: ActiveValue::Set(song_id),
            title: ActiveValue::Set(title),
            youtube_id: ActiveValue::Set(youtube_id),
            thumbnail_url: ActiveValue::Set(thumbnail_url),
            path: ActiveValue::Set(path),
        };

        Ok(SongEntity::update(model).exec(self.ref_db()).await?.id)
    }

    pub async fn update_song_from_gui_song(&self, song: GSong) -> Result<i32, DatabaseError> {
        let model = song::ActiveModel {
            id: ActiveValue::Set(song.id.expect("exists")),
            title: ActiveValue::Set(song.get_title_string()),
            youtube_id: ActiveValue::Set(song.youtube_id.clone()),
            thumbnail_url: ActiveValue::Set(song.thumbnail_url.clone()),
            path: ActiveValue::Set(Some(song.get_database_path())),
        };

        Ok(SongEntity::update(model).exec(self.ref_db()).await?.id)
    }

    pub async fn update_all_from_app_song(&self, song: AppSong) -> Result<(), DatabaseError> {
        self.update_song(
            song.id.unwrap(),
            song.get_title_string(),
            song.yt_id.clone(),
            song.tb_url.clone(),
            // TODO: update path from TUI
            None,
        )
        .await?;
        // TODO: update artists, albums, etc
        Ok(())
    }

    pub async fn update_all_from_gui_song(&self, song: GSong) -> Result<(), DatabaseError> {
        if let Some(previous_model) =
            SongEntity::find_by_id(song.id.ok_or(DatabaseError::NoSongId)?)
                .one(self.ref_db())
                .await?
        {
            self.update_song(
                song.id.ok_or(DatabaseError::NoSongId)?,
                song.get_title_string(),
                song.youtube_id.clone(),
                song.thumbnail_url.clone(),
                Some(song.get_database_path()),
            )
            .await?;

            // call functions to update
            let new_artists = song.artists.unwrap_or_default();
            self.update_song_artists_links(previous_model.clone(), new_artists)
                .await?;
            let new_albums = song.albums.unwrap_or_default();
            self.update_song_albums_links(previous_model.clone(), new_albums)
                .await?;
            let new_genres = song.genres.unwrap_or_default();
            self.update_song_genres_links(previous_model.clone(), new_genres)
                .await?;

            // TODO: update youtube_playlists
        } else {
            return Err(DatabaseError::NoSongFound);
        }

        // update song

        Ok(())
    }

    /// update relation of a song's artists by deleting all relations and adding new ones
    #[tracing::instrument(skip_all, fields(id = song.id))]
    async fn update_song_artists_links(
        &self,
        song: SongModel,
        new_artists: Vec<ArtistModel>,
    ) -> Result<(), DatabaseError> {
        let rows = SongArtistJunction::delete_many()
            .filter(
                Condition::all().add(Expr::col(song_artist_junction::Column::SongId).eq(song.id)),
            )
            .exec(self.ref_db())
            .await?
            .rows_affected;
        trace!("rows affected: {rows}");

        for (i, new_artist) in new_artists.iter().enumerate() {
            // create new from name or use existing
            let artist = self.insert_artist(new_artist.name.clone()).await?;
            let insert_id = self.insert_song_artist(artist, song.id).await?;

            info!(
                "Inserted artist {} [{}], number {}",
                new_artist.name, insert_id, i
            );
        }
        Ok(())
    }

    /// update relation of a song's albums by deleting all relations and adding new ones
    #[tracing::instrument(skip_all, fields(id = song.id))]
    async fn update_song_albums_links(
        &self,
        song: SongModel,
        new_albums: Vec<AlbumModel>,
    ) -> Result<(), DatabaseError> {
        let rows = SongAlbumJunction::delete_many()
            .filter(
                Condition::all().add(Expr::col(song_album_junction::Column::SongId).eq(song.id)),
            )
            .exec(self.ref_db())
            .await?
            .rows_affected;
        trace!("rows affected: {rows}");

        for (i, new_album) in new_albums.iter().enumerate() {
            // create new from name or use existing
            let album = self.insert_album(new_album.name.clone()).await?;
            let model = song_album_junction::ActiveModel {
                song_id: ActiveValue::Set(song.id),
                album_id: ActiveValue::Set(album),
                ..Default::default()
            };
            let insert_id = SongAlbumJunction::insert(model)
                .exec(self.ref_db())
                .await?
                .last_insert_id;
            info!(
                "Inserted album {} [{}], number {}",
                new_album.name, insert_id, i
            );
        }
        Ok(())
    }

    /// update relation of a song's genres by deleting all relations and adding new ones
    #[tracing::instrument(skip_all, fields(id = song.id))]
    async fn update_song_genres_links(
        &self,
        song: SongModel,
        new_genres: Vec<GenreModel>,
    ) -> Result<(), DatabaseError> {
        let rows = SongGenreJunction::delete_many()
            .filter(
                Condition::all().add(Expr::col(song_genre_junction::Column::SongId).eq(song.id)),
            )
            .exec(self.ref_db())
            .await?
            .rows_affected;
        trace!("rows affected: {rows}");

        for (i, new_genres) in new_genres.iter().enumerate() {
            // create new from name or use existing
            let genre = self.insert_genre(new_genres.genre.clone()).await?;
            let model = song_genre_junction::ActiveModel {
                song_id: ActiveValue::Set(song.id),
                genre_id: ActiveValue::Set(genre),
                ..Default::default()
            };
            let insert_id = SongGenreJunction::insert(model)
                .exec(self.ref_db())
                .await?
                .last_insert_id;
            info!(
                "Inserted genre {} [{}], number {}",
                new_genres.genre, insert_id, i
            );
        }
        Ok(())
    }

    pub async fn delete_song_from_app_song(&self, song: AppSong) -> Result<u64, DatabaseError> {
        if let Some(song_id) = song.id {
            // delete all foreign keys
            SongArtistJunction::delete_many()
                .filter(song_artist_junction::Column::SongId.eq(song_id))
                .exec(self.ref_db())
                .await?;
            SongAlbumJunction::delete_many()
                .filter(song_album_junction::Column::SongId.eq(song_id))
                .exec(self.ref_db())
                .await?;
            SongGenreJunction::delete_many()
                .filter(song_genre_junction::Column::SongId.eq(song_id))
                .exec(self.ref_db())
                .await?;
            Ok(SongEntity::delete_by_id(song_id)
                .exec(self.ref_db())
                .await?
                .rows_affected)
        } else {
            Err(DatabaseError::NoSongId)
        }
    }

    pub async fn delete_song_artist(&self, artist_id: i32) -> Result<(), DatabaseError> {
        let model = SongArtistJunction::find()
            .filter(song_artist_junction::Column::ArtistId.eq(artist_id))
            .one(self.ref_db())
            .await?
            .expect("One SongArtistJunction model is given");
        model.delete(self.ref_db()).await?;
        Ok(())
    }

    pub async fn delete_song_album(&self, album_id: i32) -> Result<(), DatabaseError> {
        let model = SongAlbumJunction::find()
            .filter(song_album_junction::Column::AlbumId.eq(album_id))
            .one(self.ref_db())
            .await?
            .expect("One SongAlbumJunction model is given");
        model.delete(self.ref_db()).await?;
        Ok(())
    }

    pub async fn in_memory_test() -> Result<(), DatabaseError> {
        let mut opt = ConnectOptions::new("sqlite::memory:".to_owned());
        opt.sqlx_logging(true)
            .sqlx_logging_level(tracing::log::LevelFilter::Debug);
        let db = sea_orm::Database::connect(opt).await?;
        info!("spawned new in memory sqlite database");

        let strct = Self {
            path: None,
            db: Some(db),
        };
        strct.test_db().await?;
        Ok(())
    }

    pub async fn test_db(&self) -> Result<(), DatabaseError> {
        // ensure database is up to date
        crate::migrator::Migrator::refresh(self.ref_db()).await?;

        for artist in [
            "Hoshimashi Suisei",
            "Hoshimashi Suisei",
            "Comet-chan",
            "Sui-chan",
            "Minato Aqua",
        ] {
            self.insert_artist(artist.to_string()).await?;
        }

        for album in ["Still Still Stellar", "AYAYA", "Minato Aqua Originals"] {
            self.insert_album(album.to_owned()).await?;
        }

        for genre in ["Vtuber", "Jpop"] {
            self.insert_genre(genre.to_owned()).await?;
        }

        let youtube_playlist_id_res = self
            .insert_youtube_playlist_id("PLFnrkmfz7sBLzcERljKHFXfVaplrh53AY".to_string())
            .await?;

        for (title, yt_id) in [
            ("Stellar Stellar", "a51VH9BYzZA"),
            ("Aquairo Palette", "xGihoycGivE"),
        ] {
            let tb = format!("https://i.ytimg.com/vi_webp/{yt_id}/maxresdefault.webp");
            self.insert_song(
                title.to_owned(),
                Some(yt_id.to_owned()),
                Some(tb.to_owned()),
            )
            .await?;
        }

        for artist in ["Hoshimashi Suisei", "Comet-chan"] {
            if let Some(artist_id) = Artist::find()
                .filter(artist::Column::Name.eq(artist))
                .one(self.ref_db())
                .await?
            {
                self.insert_song_artist(artist_id.id, 1).await?;
            }
        }

        for artist in ["Minato Aqua"] {
            if let Some(artist_id) = Artist::find()
                .filter(artist::Column::Name.eq(artist))
                .one(self.ref_db())
                .await?
            {
                self.insert_song_artist(artist_id.id, 2).await?;
            }
        }

        for album in ["Still Still Stellar"] {
            if let Some(album_model) = Album::find()
                .filter(album::Column::Name.eq(album))
                .one(self.ref_db())
                .await?
            {
                self.insert_song_album(album_model.id, 1).await?;
            }
        }

        for album in ["Minato Aqua Originals"] {
            if let Some(album_model) = Album::find()
                .filter(album::Column::Name.eq(album))
                .one(self.ref_db())
                .await?
            {
                self.insert_song_album(album_model.id, 2).await?;
            }
        }

        for genre in ["Vtuber", "Jpop"] {
            if let Some(genre_model) = Genre::find()
                .filter(genre::Column::Genre.eq(genre))
                .one(self.ref_db())
                .await?
            {
                self.insert_song_genre(genre_model.id, 1).await?;
                self.insert_song_genre(genre_model.id, 2).await?;
            }
        }

        self.insert_song_youtube_playlist_id(youtube_playlist_id_res, 1)
            .await?;

        let vvec = self
            .get_all_songs(PathBuf::from("/home/luqman/Music"))
            .await?;
        dbg!(vvec);

        Ok(())
    }
}

pub mod error {
    use miette::Diagnostic;
    use thiserror::Error;

    #[derive(Error, Diagnostic, Debug)]
    pub enum DatabaseError {
        #[error(transparent)]
        SeaOrm(#[from] sea_orm::DbErr),
        #[error("No song id was given")]
        NoSongId,
        #[error("No song was found with the associated ID")]
        NoSongFound,
    }
}

#[cfg(test)]
mod tests {
    use crate::database::DbConnection;

    #[tokio::test]
    async fn test_db() {
        DbConnection::in_memory_test().await.unwrap();
    }
}
