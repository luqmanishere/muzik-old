use std::{
    println,
    process::Stdio,
    sync::mpsc::{self, Receiver, Sender},
    thread::JoinHandle,
};

use eyre::{Context, Result};
use tracing::{debug, error, info, warn};
use youtube_dl::SingleVideo;

use super::{editor::editor_layer, metadata::draw_metadata_yt_sync};
use crate::{
    config::Config,
    tags,
    tui::metadata::draw_list_confirm_box,
    util::{download_from_youtube, search_youtube, search_youtube_playlist},
};
use crate::{database::Song, tui::download::draw_metadata_editor};

#[derive(Default)]
struct AppState {
    song_list: Option<Vec<Song>>,
    song_index: Option<usize>,
    current_selected_song: Option<Song>,
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
use cursive::{
    views::{Dialog, SelectView, TextView},
    CbSink, Cursive,
};
impl EventRunner {
    pub fn new(cb: CbSink, config: Config) -> Self {
        let (tx, rx) = mpsc::channel::<Event>();

        Self {
            thread_handle: None,
            cb_sink: cb,
            tx,
            rx,
            config,
            state: Default::default(),
        }
    }

    // TODO: split the events into seperate functions
    pub fn process(&mut self) -> Result<()> {
        // TODO: remove unwraps
        match self.rx.recv().unwrap() {
            Event::YoutubeSearch(kw) => {
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
                                // Notify on finish loading
                                let text = format!("Done searching for: {}", kw);
                                siv.call_on_all_named("statusbar", |view: &mut TextView| {
                                    view.set_content(&text);
                                });
                            }))
                            .unwrap();
                    }
                    Err(e) => return Err(e.wrap_err("error while searching youtube")),
                }
            }
            Event::YoutubeDownload(song) => {
                debug!(
                    "downloading: {} - {} ({})",
                    song.get_title_string(),
                    song.get_artists_string(),
                    song.id.unwrap()
                );
                let title = song.get_title_string();
                let artist = song.get_artists_string();
                let status_text = format!("Downloading: {}: {}", title, artist);
                self.notify_ui(status_text);

                let filename_format = format!("{} - {}.%(ext)s", title, artist);
                let filename = format!("{} - {}.opus", title, artist);
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
            }
            Event::InsertTags(song) => {
                let filename = song.path.as_ref().unwrap();
                debug!("{}", &filename.display());
                let title = song.title.clone().unwrap_or_default();
                let artist = song.get_artists_string();
                let status_text = format!("Inserting tags for {} - {}", title, artist);
                self.notify_ui(status_text);
                match tags::write_tags(filename.into(), &song) {
                    Ok(_) => {
                        info!("wrote tags to file successfully");
                        let title = song.title.clone().unwrap_or_default();
                        let artist = song.get_artists_string();
                        let status_text = format!("Done inserting tags for {} - {}", title, artist);
                        self.notify_ui(status_text);
                        self.tx.send(Event::InsertSongDatabase(song)).unwrap();
                    }
                    Err(e) => {
                        error!("error writing tags: {}", e);
                        self.tx.send(Event::InsertSongDatabase(song)).unwrap();
                    }
                }
            }
            Event::UpdateTags(song) => {
                let filename = song.path.as_ref().unwrap();
                match tags::write_tags(filename.into(), &song) {
                    Ok(_) => {
                        info!("wrote tags to file successfully");
                        self.tx.send(Event::ChangeFilename(song))?;
                    }
                    Err(e) => {
                        if let Some(npath) = song.npath.clone() {
                            match tags::write_tags(npath, &song) {
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
            }
            Event::InsertSongDatabase(song) => match self.config.db.insert_entry(&song) {
                Ok(_) => {
                    let title = song.title.clone().unwrap_or_default();
                    let artist = song.get_artists_string();
                    let status_text =
                        format!("Done inserting into database for {} - {}", title, artist);
                    self.notify_ui(status_text);
                    self.tx.send(Event::UpdateLocalDatabase)?;
                    info!("inserted into database successfully");
                }
                Err(e) => {
                    error!("failed to insert into database: {}", e);
                }
            },
            Event::UpdateSongDatabase(song) => match self.config.db.update_song(&song) {
                Ok(_) => {
                    info!("updated song in database");
                    self.tx.send(Event::UpdateTags(song))?;
                    self.tx.send(Event::UpdateLocalDatabase)?;
                }
                Err(e) => {
                    error!("failed to update song in database: {}", e);
                }
            },
            Event::DeleteSongDatabase(song) => {
                match self.config.db.delete_entry_by_id(song.id.unwrap()) {
                    Ok(_) => {
                        info!("deleted song from database");
                        std::fs::remove_file(song.path.unwrap()).unwrap_or_default();
                        self.tx.send(Event::UpdateLocalDatabase)?;
                    }
                    Err(e) => {
                        error!("can't delete record from db: {}", e);
                    }
                }
            }
            Event::ChangeFilename(song) => {
                if let Some(npath) = song.npath {
                    std::fs::rename(song.path.as_ref().unwrap(), npath)?;
                } else {
                    debug!("no path changes needed");
                }
            }
            Event::SyncWithYoutube => {
                debug!("starting yt playlist sync process");

                if let Some(playlist_list) = self.config.yt_playlist_sync.clone() {
                    debug!("found playlists in config");

                    let mut vid_vec = vec![];
                    for playlist_id in playlist_list {
                        self.notify_ui(format!("Syncing with playlist: {}", playlist_id));
                        debug!("playlist: {}", playlist_id);
                        // get the list of videos here
                        let videos = search_youtube_playlist(
                            playlist_id.clone(),
                            self.config.cookies.clone(),
                        );

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
                                        let song = song.yt_playlist_id(Some(playlist_id.clone()));
                                        self.tx.send(Event::UpdateSongDatabase(song))?;
                                    } else {
                                        debug!(
                                            "yt id {} has playlist {}",
                                            vid.id.clone(),
                                            playlist_id
                                        );
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
            }
            Event::VerifyAllSongIntegrity() => {
                match std::process::Command::new("opusinfo")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                {
                    Ok(_status) => {
                        let status_text = "Verifying integrity of all songs".to_string();
                        self.notify_ui(status_text);
                        if let Ok(song_list) = self.config.db.get_all(self.config.music_dir.clone())
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
                                        .stdout(Stdio::null())
                                        .stderr(Stdio::null())
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
                            "done verifying integrity of all songs, check logs for info"
                                .to_string();
                        self.notify_ui(status_text);
                    }
                    Err(e) => {
                        self.notify_ui(
                            "cant verify integrity: opustools not installed".to_string(),
                        );
                        error!("cant execute opusinfo: {}", e);
                    }
                };
            }
            Event::DownloadAllMissingFromDatabase => {
                let song_list = self
                    .config
                    .db
                    .get_all(self.config.music_dir.clone())
                    .wrap_err("failed to get song list")?;

                for song in song_list {
                    let path = song.path.as_ref().unwrap().clone();
                    if !path.exists() {
                        self.tx.send(Event::YoutubeDownload(song))?;
                    }
                }
            }
            Event::UpdateLocalDatabase => {
                let song_list = self.config.db.get_all(self.config.music_dir.clone())?;
                self.state.song_list = Some(song_list);
                self.tx.send(Event::UpdateEditorSongSelectView)?;
                self.tx.send(Event::UpdateEditorMetadataSelectView(
                    self.state.song_index.unwrap_or(0),
                ))?;
                self.notify_ui("updated local database".to_string());
            }
            Event::UpdateEditorSongSelectView => {
                let song_list = self.config.db.get_all(self.config.music_dir.clone())?;
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
            }
            Event::UpdateEditorMetadataSelectView(index) => {
                self.state.song_index = Some(index);
                let mut song_list = self.state.song_list.clone().unwrap();
                let song = song_list.get_mut(index).unwrap();
                self.state.current_selected_song = Some(song.clone());
                let song = song.clone();
                self.cb_sink
                    .send(Box::new(move |siv: &mut Cursive| {
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
            }
            Event::OnMetadataSelect => {
                let song = self.state.current_selected_song.as_ref().unwrap().clone();
                let ttx = self.tx.clone();
                self.cb_sink
                    .send(Box::new(move |siv: &mut Cursive| {
                        let editor = editor_layer(siv, song, ttx);
                        siv.add_layer(editor);
                    }))
                    .unwrap();
            }

            Event::OnDeleteKey => {
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
            }
            Event::OnDownloadMetadataSubmit(metadata) => {
                let music_dir = self.config.music_dir.clone();
                let genre = metadata.genre.unwrap_or("Unknown".to_string());

                let fname = format!(
                    "{} - {}.opus",
                    metadata.title.clone().unwrap_or_default(),
                    metadata.artist.clone().unwrap_or_default()
                );
                let song = crate::database::Song::new()
                    .music_dir(Some(music_dir))
                    .fname(Some(fname))
                    .title(metadata.title)
                    .albums(metadata.album)
                    .artists(metadata.artist)
                    .genre(Some(genre))
                    .yt_id(Some(metadata.id))
                    .tb_url(metadata.video.thumbnail);
                self.tx.send(Event::YoutubeDownload(song)).unwrap();
            }
            Event::OnDownloadVideoSelect(video) => {
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
            }
            Event::OnSyncConfirm(video_vec) => {
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
            }

            Event::OnSyncMetadataSubmit(metadata) => {
                //
                let fname = format!(
                    "{} - {}.opus",
                    metadata
                        .title
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string()),
                    metadata
                        .artist
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string())
                );

                if let Some(playlist_id) = metadata.video.playlist_id.clone() {
                    debug!("got playlist_id {} for {}", playlist_id, metadata.id);
                } else {
                    debug!("got no playlist_id for {}", metadata.id);
                }

                let mut song = Song::new()
                    .music_dir(Some(self.config.music_dir.clone()))
                    .fname(Some(fname))
                    .yt_id(Some(metadata.id))
                    .title(metadata.title)
                    .artists(metadata.artist)
                    .albums(metadata.album)
                    .genre(metadata.genre)
                    .tb_url(metadata.video.thumbnail)
                    .yt_playlist_id(metadata.video.playlist_id);

                // as a precaution
                song.compute_filename();

                self.tx.send(Event::InsertSongDatabase(song))?;
            }
        }
        Ok(())
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
    fn find_yt_duplicate(&self, id: String) -> Option<Song> {
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
    YoutubeDownload(Song),
    InsertTags(Song),
    InsertSongDatabase(Song),
    UpdateSongDatabase(Song),
    UpdateTags(Song),
    DeleteSongDatabase(Song),
    VerifyAllSongIntegrity(),
    DownloadAllMissingFromDatabase,
    ChangeFilename(Song),
    SyncWithYoutube,

    UpdateLocalDatabase,
    UpdateEditorSongSelectView,
    UpdateEditorMetadataSelectView(usize),
    OnMetadataSelect,
    OnDeleteKey,
    OnDownloadVideoSelect(SingleVideo),
    OnDownloadMetadataSubmit(DownloadMetadataInput),
    OnSyncConfirm(Vec<SingleVideo>),
    OnSyncMetadataSubmit(DownloadMetadataInput),
}

pub struct DownloadMetadataInput {
    pub id: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub video: SingleVideo,
}
