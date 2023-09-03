use std::{println, thread::JoinHandle};

use crossbeam_channel::{self, Receiver, Sender};
use cursive::{
    views::{Dialog, SelectView, TextView},
    CbSink, Cursive,
};
use eyre::{Context, Result};
use muzik_common::{
    entities::*,
    tags,
    util::{download_from_youtube, search_youtube, search_youtube_playlist}, database::AppSong,
};
use tracing::{debug, error, info, instrument, warn};
use youtube_dl::SingleVideo;

use crate::download::draw_metadata_editor;

use super::config::Config;
use super::metadata::draw_list_confirm_box;
use super::metadata::draw_metadata_yt_sync;

#[derive(Default)]
struct AppState {
    song_list: Option<Vec<AppSong>>,
    song_index: Option<usize>,
    current_selected_song: Option<AppSong>,
}

#[allow(dead_code)]
pub struct EventRunner {
    thread_handle: Option<JoinHandle<()>>,
    pub cb_sink: CbSink,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    config: Config,
    state: AppState,
}
impl EventRunner {
    pub async fn new(cb: CbSink, config: Config) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded::<Event>();

        Self {
            thread_handle: None,
            cb_sink: cb,
            tx,
            rx,
            config,
            state: Default::default(),
        }
    }

    #[instrument(skip(self))]
    async fn youtube_search(&self, kw: String) -> Result<EventLoopAction> {
        let text = format!("Searching for: {}", kw);
        self.notify_ui(text);

        match search_youtube(kw.clone(), self.config.cookies.clone()) {
            Ok(entries) => {
                // IDK how this works but ok
                self.cb_sink
                    .send(Box::new(move |siv: &mut Cursive| {
                        let items = entries
                            .iter()
                            .enumerate()
                            .map(|(_ind, e)| (e.title.to_string(), e.to_owned()));
                        siv.call_on_name(
                            "result_selectview",
                            |view: &mut SelectView<SingleVideo>| {
                                view.clear();
                                view.add_all(items);
                            },
                        );
                    }))
                    .unwrap();
                self.notify_ui(format!("Done searching for: {}", kw));
            }
            Err(e) => return Err(e.wrap_err("error while searching youtube")),
        }
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all, fields(song.yt_id))]
    async fn youtube_download(&self, song: AppSong) -> Result<EventLoopAction> {
        debug!(
            "downloading: {} - {} ({})",
            song.get_title_string(),
            song.get_artists_string(),
            song.get_yt_id().unwrap()
        );
        let id = song.yt_id.clone().unwrap();
        let title = song.get_title_string();
        let artist = song.get_artists_string();
        let status_text = format!("Downloading: {}: {}", title, artist);
        self.notify_ui(status_text);

        let filename_format = format!("{} - {} {}.%(ext)s", title, artist, id);
        let filename = format!("{} - {} {}.opus", title, artist, id);
        let filename = song.get_music_dir().join(filename);
        let _youtube = download_from_youtube(
            song.get_yt_id().unwrap(),
            song.get_music_dir().display().to_string(),
            filename_format,
            self.config.cookies.clone(),
        )?;

        if filename.exists() {
            let title = song.get_title_string();
            let artist = song.get_artists_string();
            let status_text = format!("Download finished for: {} - {}", title, artist);
            self.notify_ui(status_text);
        } else {
            println!("File not found after downloading");
        }

        self.tx.send(Event::InsertTags(song)).unwrap();
        Ok(EventLoopAction::Continue)
    }

    /// this runs last
    #[instrument(skip_all, fields(song.yt_id))]
    async fn insert_tags(&self, song: AppSong) -> Result<EventLoopAction> {
        let filename = song.path.as_ref().unwrap();
        debug!("{}", &filename.display());
        let title = song.title.clone().unwrap_or_default();
        let artist = song.get_artists_string();
        let status_text = format!("Inserting tags for {} - {}", title, artist);
        self.notify_ui(status_text);
        match tags::write_tags(filename.into(), &song).await {
            Ok(_) => {
                info!("wrote tags to file successfully");
                let title = song.title.clone().unwrap_or_default();
                let artist = song.get_artists_string();
                let status_text = format!("Done inserting tags for {} - {}", title, artist);
                self.notify_ui(status_text);
            }
            Err(e) => {
                error!("error writing tags: {}", e);
            }
        }
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all, fields(song.yt_id))]
    async fn update_tags(&self, song: AppSong) -> Result<EventLoopAction> {
        let filename = song.path.as_ref().unwrap();
        match tags::write_tags(filename.into(), &song).await {
            Ok(_) => {
                info!("wrote tags to file successfully");
                self.tx.send(Event::ChangeFilename(song))?;
            }
            Err(e) => {
                if let Some(npath) = song.npath.clone() {
                    match tags::write_tags(npath, &song).await {
                        Ok(_) => {
                            info!("wrote tags to file successfully");
                            self.tx.send(Event::ChangeFilename(song))?;
                        }
                        Err(e) => {
                            error!("error writing tags: {}", e);
                        }
                    }
                } else {
                    error!("error writing tags: {}", e);
                }
            }
        }
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all, fields(song.yt_id))]
    async fn insert_song_database(&self, song: AppSong) -> Result<EventLoopAction> {
        self.config
            .db_new
            .insert_from_app_song(song.clone())
            .await?;

        self.tx.send(Event::YoutubeDownload(song))?;
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all, fields(song.yt_id))]
    async fn update_song_database(&self, song: AppSong) -> Result<EventLoopAction> {
        match self
            .config
            .db_new
            .update_all_from_app_song(song.clone())
            .await
        {
            Ok(_) => {
                info!("updated song in database");
                self.tx.send(Event::UpdateTags(song))?;
                self.tx.send(Event::UpdateLocalDatabase)?;
            }
            Err(e) => {
                error!("failed to update song in database: {}", e);
            }
        };

        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all, fields(song.yt_id))]
    async fn delete_song_database(&self, song: AppSong) -> Result<EventLoopAction> {
        // TODO: db_new
        match self
            .config
            .db_new
            .delete_song_from_app_song(song.clone())
            .await
        {
            Ok(_) => {
                info!("deleted song from database");
                std::fs::remove_file(song.path.clone().unwrap()).unwrap_or_default();
                self.tx.send(Event::UpdateLocalDatabase)?;
            }
            Err(e) => {
                error!("can't delete record from db: {}", e);
            }
        };
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all, fields(song.yt_id))]
    async fn change_filename(&self, song: AppSong) -> Result<EventLoopAction> {
        if let Some(npath) = song.npath {
            std::fs::rename(song.path.as_ref().unwrap(), npath)?;
        } else {
            debug!("no path changes needed");
        }
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn sync_with_youtube(&self) -> Result<EventLoopAction> {
        debug!("starting yt playlist sync process");

        if let Some(playlist_list) = self.config.yt_playlist_sync.clone() {
            debug!("found playlists in config");

            let mut vid_vec = vec![];
            for playlist_id in playlist_list {
                self.notify_ui(format!("Syncing with playlist: {}", playlist_id));
                debug!("playlist: {}", playlist_id);
                // get the list of videos here
                let videos =
                    search_youtube_playlist(playlist_id.clone(), self.config.cookies.clone());

                if let Ok(videos) = videos {
                    for vid in videos {
                        debug!("got {} - {}", vid.title, vid.channel.clone().unwrap());
                        debug!(
                            "found in database: {}",
                            self.check_yt_duplicate(vid.id.clone())
                        );
                        // check for dupes
                        if let Some(song) = self.find_yt_duplicate(vid.id.clone()) {
                            debug!("yt id {} exists in database", vid.id.clone());
                            // if playlist value is empty then update, else ignore
                            if song.yt_playlist.is_none() {
                                debug!(
                                    "yt id {} has no playlist, updating value to {}",
                                    vid.id.clone(),
                                    &playlist_id
                                );
                                let song = song.with_yt_playlist_id(Some(playlist_id.clone()));
                                self.tx.send(Event::UpdateSongDatabase(song))?;
                            } else {
                                debug!("yt id {} has playlist {}", vid.id.clone(), playlist_id);
                            }
                        } else {
                            // if song was not downloaded prior
                            // draw widget to confirm list and integrate into database
                            vid_vec.push(vid.clone());
                        }
                    }
                } else {
                    debug!("fail");
                };
            }
            let ttx = self.get_tx();
            self.cb_sink
                .send(Box::new(|siv: &mut Cursive| {
                    draw_list_confirm_box(siv, vid_vec, ttx);
                }))
                .unwrap();
            self.notify_ui("Standby".to_string());
        } else {
            debug!("no playlist to be sync");
        }
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn verify_all_song_integrity(&self) -> Result<EventLoopAction> {
        match std::process::Command::new("opusinfo")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
        {
            Ok(_status) => {
                let status_text = "Verifying integrity of all songs".to_string();
                self.notify_ui(status_text);
                if let Ok(song_list) = self
                    .config
                    .db_new
                    .get_all_songs(self.config.music_dir.clone())
                    .await
                {
                    for song in song_list {
                        let path = song.path.as_ref().unwrap().clone();
                        if path.exists() {
                            info!(
                                "file for {} - {} [{}] exists",
                                song.title.as_ref().unwrap().clone(),
                                song.get_artists_string(),
                                song.id.unwrap()
                            );
                        } else {
                            warn!(
                                "file for {} - {} [{}] does not exist",
                                song.title.as_ref().unwrap().clone(),
                                song.get_artists_string(),
                                song.id.unwrap()
                            );
                        }
                        let ext: String = path
                            .extension()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap()
                            .to_string();
                        if ext.contains("opus") {
                            let opusinfo = std::process::Command::new("opusinfo")
                                .arg(path)
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .status()
                                .wrap_err("failed to execute command opusinfo")?;
                            match opusinfo.code() {
                                Some(code) => {
                                    if code != 0 {
                                        error!(
                                            "opusinfo returned code {} for song: {} - {} [{}]",
                                            code,
                                            song.title.as_ref().unwrap().clone(),
                                            song.get_artists_string(),
                                            song.id.unwrap()
                                        );
                                    } else {
                                        info!(
                                            "opusinfo returned code {} for song: {} - {} [{}]",
                                            code,
                                            song.title.as_ref().unwrap().clone(),
                                            song.get_artists_string(),
                                            song.id.unwrap()
                                        );
                                    }
                                }
                                None => {
                                    error!("process opusinfo was killed");
                                }
                            }
                        } else {
                        }
                    }
                }
                let status_text =
                    "done verifying integrity of all songs, check logs for info".to_string();
                self.notify_ui(status_text);
            }
            Err(e) => {
                self.notify_ui("cant verify integrity: opustools not installed".to_string());
                error!("cant execute opusinfo: {}", e);
            }
        };
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn download_all_missing_from_db(&self) -> Result<EventLoopAction> {
        let song_list = self
            .config
            .db_new
            .get_all_songs(self.config.music_dir.clone())
            .await
            .wrap_err("failed to get song list")?;

        for song in song_list {
            let path = song.path.as_ref().unwrap().clone();
            if !path.exists() {
                self.tx.send(Event::YoutubeDownload(song))?;
            }
        }
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn update_local_database(&mut self) -> Result<EventLoopAction> {
        // TODO: db_new
        let song_list = self
            .config
            .db_new
            .get_all_songs(self.config.music_dir.clone())
            .await?;
        self.state.song_list = Some(song_list);
        self.tx.send(Event::UpdateEditorSongSelectView)?;
        self.tx.send(Event::UpdateEditorMetadataSelectView(
            self.state.song_index.unwrap_or(0),
        ))?;
        self.notify_ui("updated local database".to_string());
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn update_editor_song_select_view(&mut self) -> Result<EventLoopAction> {
        let song_list = self
            .config
            .db_new
            .get_all_songs(self.config.music_dir.clone())
            .await?;
        self.state.song_list = Some(song_list);
        let index = self.state.song_index.unwrap_or(0);

        let song_list = self.state.song_list.clone().unwrap();
        self.cb_sink
            .send(Box::new(move |siv: &mut Cursive| {
                let select_song_list = song_list.iter().enumerate().map(|(ind, f)| {
                    (
                        format!("{} - {}", f.get_title_string(), f.get_artists_string()),
                        ind,
                    )
                });
                siv.call_on_name("select_song", |view: &mut SelectView<usize>| {
                    view.clear();
                    view.add_all(select_song_list);
                    view.set_selection(index);
                });
            }))
            .unwrap();
        // TODO: db_new
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn update_editor_metadata_select_view(
        &mut self,
        index: usize,
    ) -> Result<EventLoopAction> {
        self.state.song_index = Some(index);
        let mut song_list = self.state.song_list.clone().unwrap();
        let song = song_list.get_mut(index).unwrap();
        self.state.current_selected_song = Some(song.clone());
        let song = song.clone();
        self.cb_sink
            .send(Box::new(move |siv: &mut Cursive| {
                //siv.call_on_name("ar", callback)
                let song1 = song.clone();
                siv.call_on_name("metadata_title", |view: &mut TextView| {
                    view.set_content(song1.get_title_string());
                });
                siv.call_on_name(
                    "metadata_artist_select_view",
                    |view: &mut SelectView<artist::Model>| {
                        view.clear();
                        if let Some(artists_model_vec) = song1.artist {
                            for artist in artists_model_vec {
                                view.add_item(artist.name.clone(), artist.clone());
                            }
                        }
                    },
                );
                siv.call_on_name(
                    "metadata_album_select_view",
                    |view: &mut SelectView<album::Model>| {
                        view.clear();
                        if let Some(albums_model_vec) = song1.album {
                            for album in albums_model_vec {
                                view.add_item(album.name.clone(), album.clone());
                            }
                        }
                    },
                );
                siv.call_on_name(
                    "metadata_genre_select_view",
                    |view: &mut SelectView<genre::Model>| {
                        view.clear();
                        if let Some(genre_model_vec) = song1.genre {
                            for genre in genre_model_vec {
                                view.add_item(genre.genre.clone(), genre.clone());
                            }
                        }
                    },
                );
                siv.call_on_name("select_metadata", |view: &mut SelectView<String>| {
                    view.clear();
                    let title = song.get_title_string();
                    let artist = song.get_artists_string();
                    let album = song.get_albums_string();
                    view.add_item(title.clone(), title);
                    view.add_item(artist.clone(), artist);
                    view.add_item(album.clone(), album);
                });
            }))
            .unwrap();
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn on_delete_key(&self) -> Result<EventLoopAction> {
        let mut song_list = self.state.song_list.clone().unwrap();
        let tx = self.get_tx();
        self.cb_sink
            .send(Box::new(move |siv: &mut Cursive| {
                siv.call_on_name("select_song", |view: &mut SelectView<usize>| {
                    let item = view.selection().unwrap();
                    let song = song_list.get_mut(*item).unwrap().clone();
                    tx.send(Event::DeleteSongDatabase(song)).unwrap();
                });
            }))
            .unwrap();
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn on_download_metadata_submit(
        &self,
        metadata: DownloadMetadataInput,
    ) -> Result<EventLoopAction> {
        let music_dir = self.config.music_dir.clone();
        let genre = metadata.genre.unwrap_or("Unknown".to_string());

        let mut song = AppSong::new()
            .with_music_dir(Some(music_dir))
            .with_title(metadata.title)
            .with_albums(metadata.album.unwrap_or("Unknown".to_string()))
            .with_artists_string(metadata.artist.unwrap_or("Unknown".to_string()))
            .with_genre(genre)
            .with_yt_id(Some(metadata.id))
            .with_tb_url(metadata.video.thumbnail)
            .compute_new_filename();

        song.compute_filename();
        self.tx.send(Event::InsertSongDatabase(song)).unwrap();
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn on_download_video_select(&self, video: SingleVideo) -> Result<EventLoopAction> {
        // Show popup to confirm
        let is_existing = self.check_yt_duplicate(video.id.clone());
        let title = video.title.clone();
        let channel = video
            .channel
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());
        let song2 = video;

        let ttx = self.get_tx();
        self.cb_sink
            .send(Box::new(move |siv: &mut Cursive| {
                let text = if !is_existing {
                    format!("Title: {}\nChannel:{}\nConfirm to edit?", title, channel)
                } else {
                    format!(
                        "Title: {}\nChannel:{}\nIs Existing: Yes\nConfirm to edit?",
                        title, channel
                    )
                };
                let confirm = Dialog::text(text).dismiss_button("Cancel").button(
                    "Edit",
                    move |siv: &mut Cursive| {
                        siv.pop_layer();
                        draw_metadata_editor(siv, song2.clone(), ttx.clone());
                    },
                );
                siv.add_layer(confirm);
            }))
            .unwrap();
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn on_sync_confirm(&self, video_vec: Vec<SingleVideo>) -> Result<EventLoopAction> {
        // calmly draw metadata editors for all of this
        // add to database first, dont download yet
        for video in video_vec {
            let ttx = self.get_tx();
            self.cb_sink
                .send(Box::new(|siv: &mut Cursive| {
                    draw_metadata_yt_sync(siv, video, ttx);
                }))
                .unwrap();
        }
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip(self), fields(metadata))]
    async fn on_sync_metadata_submit(
        &self,
        metadata: DownloadMetadataInput,
    ) -> Result<EventLoopAction> {
        if let Some(playlist_id) = metadata.video.playlist_id.clone() {
            debug!("got playlist_id {} for {}", playlist_id, metadata.id);
        } else {
            debug!("got no playlist_id for {}", metadata.id);
        }

        let mut song = AppSong::new()
            .with_music_dir(Some(self.config.music_dir.clone()))
            .with_yt_id(Some(metadata.id))
            .with_title(metadata.title)
            .with_artists_string(metadata.artist.unwrap_or("Unknown".to_string()))
            .with_albums(metadata.album.unwrap_or("Unknown".to_string()))
            .with_genre(metadata.genre.unwrap_or("Unknown".to_string()))
            .with_tb_url(metadata.video.thumbnail)
            .with_yt_playlist_id(metadata.video.playlist_id);

        // as a precaution
        song.compute_filename();

        self.tx.send(Event::InsertSongDatabase(song))?;
        Ok(EventLoopAction::Continue)
    }

    async fn metadata_editor_add_artist(&self, artist: String) -> Result<EventLoopAction> {
        let artist_id = self.config.db_new.insert_artist(artist).await?;
        let song_id = self
            .state
            .current_selected_song
            .clone()
            .expect("current_selected_song is not empty")
            .id
            .expect("id is not empty");

        let _song_artist = self
            .config
            .db_new
            .insert_song_artist(artist_id, song_id)
            .await?;
        self.tx.send(Event::UpdateLocalDatabase)?;
        Ok(EventLoopAction::Continue)
    }

    #[tracing::instrument(skip_all)]
    async fn metadata_editor_edit_artist(
        &self,
        old: String,
        new: String,
    ) -> Result<EventLoopAction> {
        let artist_id = self.config.db_new.insert_artist(new).await?;

        let old_artist_id = self.config.db_new.insert_artist(old).await?;

        if old_artist_id != artist_id {
            let song_id = self
                .state
                .current_selected_song
                .clone()
                .expect("current_selected_song is not empty")
                .id
                .expect("id is not empty");
            let _song_artist = self
                .config
                .db_new
                .insert_song_artist(artist_id, song_id)
                .await?;

            self.config.db_new.delete_song_artist(old_artist_id).await?;
        }
        self.tx.send(Event::UpdateLocalDatabase)?;
        Ok(EventLoopAction::Continue)
    }

    #[tracing::instrument(skip_all)]
    async fn metadata_editor_edit_album(
        &self,
        old: String,
        new: String,
    ) -> Result<EventLoopAction> {
        let album_id = self.config.db_new.insert_album(new).await?;

        let old_album_id = self.config.db_new.insert_album(old).await?;

        if old_album_id != album_id {
            let song_id = self
                .state
                .current_selected_song
                .clone()
                .expect("current_selected_song is not empty")
                .id
                .expect("id is not empty");
            let _song_album = self
                .config
                .db_new
                .insert_song_album(album_id, song_id)
                .await?;

            self.config.db_new.delete_song_album(old_album_id).await?;
        }
        self.tx.send(Event::UpdateLocalDatabase)?;
        Ok(EventLoopAction::Continue)
    }

    #[tracing::instrument(skip_all)]
    async fn metadata_editor_add_album(&self, album: String) -> Result<EventLoopAction> {
        let album_id = self.config.db_new.insert_album(album).await?;
        let song_id = self
            .state
            .current_selected_song
            .clone()
            .expect("current_selected_song is not empty")
            .id
            .expect("id is not empty");

        let _song_album = self
            .config
            .db_new
            .insert_song_album(album_id, song_id)
            .await?;
        self.tx.send(Event::UpdateLocalDatabase)?;
        Ok(EventLoopAction::Continue)
    }

    #[instrument(skip_all)]
    async fn quit_event_loop(&self) -> Result<EventLoopAction> {
        Ok(EventLoopAction::Quit)
    }

    #[instrument(skip_all)]
    pub async fn process(&mut self) -> Result<EventLoopAction> {
        // TODO: remove unwraps
        // TODO: add option to edit entries directly
        let recv = self.rx.recv();
        let action = match recv.unwrap() {
            Event::YoutubeSearch(kw) => self.youtube_search(kw).await,
            Event::YoutubeDownload(song) => self.youtube_download(song).await,
            Event::InsertTags(song) => self.insert_tags(song).await,
            Event::UpdateTags(song) => self.update_tags(song).await,
            Event::InsertSongDatabase(song) => self.insert_song_database(song).await,
            Event::UpdateSongDatabase(song) => self.update_song_database(song).await,
            Event::DeleteSongDatabase(song) => self.delete_song_database(song).await,
            Event::ChangeFilename(song) => self.change_filename(song).await,
            Event::SyncWithYoutube => self.sync_with_youtube().await,
            Event::VerifyAllSongIntegrity() => self.verify_all_song_integrity().await,
            Event::DownloadAllMissingFromDatabase => self.download_all_missing_from_db().await,
            Event::UpdateLocalDatabase => self.update_local_database().await,
            Event::UpdateEditorSongSelectView => self.update_editor_song_select_view().await,
            Event::UpdateEditorMetadataSelectView(index) => {
                self.update_editor_metadata_select_view(index).await
            }
            Event::OnDeleteKey => self.on_delete_key().await,
            Event::OnDownloadMetadataSubmit(metadata) => {
                self.on_download_metadata_submit(metadata).await
            }
            Event::OnDownloadVideoSelect(video) => self.on_download_video_select(video).await,
            Event::OnSyncConfirm(video_vec) => self.on_sync_confirm(video_vec).await,
            Event::OnSyncMetadataSubmit(metadata) => self.on_sync_metadata_submit(metadata).await,

            Event::MetadataEditorAddArtist(artist) => self.metadata_editor_add_artist(artist).await,
            Event::MetadataEditorEditArtist((old, new)) => {
                self.metadata_editor_edit_artist(old, new).await
            }
            Event::MetadataEditorEditAlbum((old, new)) => {
                self.metadata_editor_edit_album(old, new).await
            }
            Event::MetadataEditorAddAlbum(album) => self.metadata_editor_add_album(album).await,
            Event::QuitEventLoop => self.quit_event_loop().await,
        }?;
        Ok(action)
    }

    pub fn get_tx(&self) -> Sender<Event> {
        self.tx.clone()
    }

    pub fn notify_ui(&self, msg: String) {
        self.cb_sink
            .send(Box::new(move |siv: &mut Cursive| {
                siv.call_on_all_named("statusbar", |view: &mut TextView| view.set_content(&msg));
            }))
            .unwrap();
    }

    fn check_yt_duplicate(&self, id: String) -> bool {
        debug!("got id: {}", id);
        if let Some(song_list) = &self.state.song_list {
            for s in song_list {
                if let Some(ytid) = s.get_yt_id() {
                    if id == ytid {
                        debug!(
                            "comparing ytid:{} to id: {} = result {}",
                            ytid,
                            id,
                            id == ytid
                        );
                        return id == ytid;
                    };
                }
            }
            false
        } else {
            false
        }
    }
    fn find_yt_duplicate(&self, id: String) -> Option<AppSong> {
        debug!("got id: {}", id);
        if let Some(song_list) = &self.state.song_list {
            for s in song_list {
                if let Some(ytid) = s.get_yt_id() {
                    if id == ytid {
                        debug!(
                            "comparing ytid:{} to id: {} = result {}",
                            ytid,
                            id,
                            id == ytid
                        );
                        return Some(s.clone());
                    };
                }
            }
            None
        } else {
            None
        }
    }
}

pub enum Event {
    YoutubeSearch(String),
    YoutubeDownload(AppSong),
    InsertTags(AppSong),
    InsertSongDatabase(AppSong),
    UpdateSongDatabase(AppSong),
    UpdateTags(AppSong),
    DeleteSongDatabase(AppSong),
    VerifyAllSongIntegrity(),
    DownloadAllMissingFromDatabase,
    ChangeFilename(AppSong),
    SyncWithYoutube,
    QuitEventLoop,

    UpdateLocalDatabase,
    UpdateEditorSongSelectView,
    UpdateEditorMetadataSelectView(usize),
    OnDeleteKey,
    OnDownloadVideoSelect(SingleVideo),
    OnDownloadMetadataSubmit(DownloadMetadataInput),
    OnSyncConfirm(Vec<SingleVideo>),
    OnSyncMetadataSubmit(DownloadMetadataInput),
    MetadataEditorAddArtist(String),
    /// (old, new)
    MetadataEditorEditArtist((String, String)),
    MetadataEditorEditAlbum((String, String)),
    MetadataEditorAddAlbum(String),
}

pub struct DownloadMetadataInput {
    pub id: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub video: SingleVideo,
}

pub enum EventLoopAction {
    Continue,
    Quit,
}
