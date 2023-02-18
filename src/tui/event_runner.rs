use std::{
    path::PathBuf,
    process::Stdio,
    sync::mpsc::{self, Receiver, Sender},
    thread::JoinHandle,
};

use eyre::{Context, Result};
use tracing::{debug, error, info, warn};
use youtube_dl::{SearchOptions, SingleVideo, YoutubeDl, YoutubeDlOutput};

#[allow(dead_code)]
pub struct EventRunner {
    thread_handle: Option<JoinHandle<()>>,
    pub cb_sink: CbSink,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    config: Config,
}
use cursive::{
    view::Scrollable,
    views::{Dialog, LinearLayout, SelectView, TextView},
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
        }
    }

    pub fn process(&self) -> Result<()> {
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
                                let mut select_entry = SelectView::new();
                                select_entry.add_all(items.into_iter());
                                select_entry.set_on_submit(super::download::start_download);
                                let select_entry = Dialog::around(select_entry.scrollable());
                                siv.call_on_name(
                                    "download_v_layout",
                                    |layout: &mut LinearLayout| {
                                        layout.remove_child(2);
                                        layout.insert_child(2, select_entry);
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
                    Err(e) => return Err(e),
                }
            }
            Event::YoutubeDownload(song) => {
                // TODO: Rework these functions to accept and use `Song`
                debug!(
                    "got id: {}, title: {}, artist: {}, album: {}",
                    &song.id.unwrap(),
                    &song.get_title_string(),
                    &song.get_artists_string(),
                    &song.get_albums_string(),
                );
                let title = song.get_title_string();
                let artist = song.get_artists_string();
                let status_text = format!("Downloading: {}: {}", title, artist);
                self.notify_ui(status_text);

                let filename_format = format!("{} - {}.%(ext)s", title, artist);
                let filename = format!("{} - {}.opus", title, artist);
                let filename = song.get_music_dir().join(filename);
                let _youtube = download_from_youtube(
                    song.get_yt_id(),
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
                    }
                    Err(e) => {
                        error!("error writing tags: {}", e);
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
                    self.cb_sink
                        .send(Box::new(editor::update_database))
                        .unwrap();
                    info!("inserted into database successfully");
                }
                Err(e) => {
                    error!("failed to insert into database: {}", e);
                }
            },
            Event::UpdateSongDatabase(song) => match self.config.db.update_song(&song) {
                Ok(_) => {
                    info!("updated song in database");
                    self.tx.send(Event::UpdateTags(song)).unwrap();
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
                        self.cb_sink
                            .send(Box::new(editor::update_database))
                            .unwrap();
                    }
                    Err(e) => {
                        error!("can't delete record from db: {}", e);
                    }
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
                        // TODO: download with metadata
                        let title = song.title.clone().unwrap();
                        let artist = song.get_artists_string();
                        let status_text = format!("Downloading: {}: {}", title, artist);
                        self.notify_ui(status_text);

                        let title = song.title.clone().unwrap();
                        let artist = song.get_artists_string();
                        let filename_format = format!("{} - {}.%(ext)s", title, artist);
                        let filename_full = path.clone();
                        let yt_id = song.yt_id.as_ref().unwrap().clone();
                        let _youtube = download_from_youtube(
                            yt_id,
                            self.config.music_dir.display().to_string(),
                            filename_format,
                            self.config.cookies.clone(),
                        )?;

                        if filename_full.exists() {
                            let title = song.title.clone().unwrap();
                            let artist = song.get_artists_string();
                            let status_text =
                                format!("Download finished for: {} - {}", title, artist);
                            self.notify_ui(status_text);
                        } else {
                            println!("File not found after downloading");
                        }
                        self.tx.send(Event::InsertTags(song)).unwrap();
                    }
                }
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
}

use crate::{config::Config, database::Song, tags, tui::editor};

use eyre::eyre;

fn search_youtube(kw: String, cookies: Option<PathBuf>) -> Result<Vec<SingleVideo>> {
    let yt_search = if !kw.contains("http") {
        let search_options = SearchOptions::youtube(kw).with_count(5);
        if let Some(cookie) = cookies {
            YoutubeDl::search_for(&search_options)
                .cookies(cookie.display().to_string())
                .run()
        } else {
            YoutubeDl::search_for(&search_options).run()
        }
    } else if let Some(cookie) = cookies {
        YoutubeDl::new(kw)
            .download(false)
            .cookies(cookie.display().to_string())
            .run()
    } else {
        YoutubeDl::new(kw).download(false).run()
    };

    match yt_search {
        Ok(output) => match output {
            youtube_dl::YoutubeDlOutput::Playlist(playlist) => {
                let entries = playlist.entries.unwrap_or_default();
                Ok(entries)
            }
            youtube_dl::YoutubeDlOutput::SingleVideo(video) => Ok(vec![*video]),
        },
        Err(err) => match err {
            youtube_dl::Error::Io(e) => return Err(eyre!("error during I/O: {}", e)),
            youtube_dl::Error::Json(e) => return Err(eyre!("error parsing JSON: {}", e)),
            youtube_dl::Error::ExitCode { code, stderr } => {
                return Err(eyre!(
                    "process returned code: {}, with stderr: {}",
                    code,
                    stderr
                ))
            }
            youtube_dl::Error::ProcessTimeout => return Err(eyre!("process timed out")),
        },
    }
}

fn download_from_youtube(
    id: String,
    output_dir: String,
    format: String,
    cookies: Option<PathBuf>,
) -> Result<YoutubeDlOutput, youtube_dl::Error> {
    if let Some(cookie) = cookies {
        println!("cookie found");
        YoutubeDl::new(&id)
            .youtube_dl_path("yt-dlp")
            .extra_arg("--audio-format")
            .extra_arg("opus")
            .format("251")
            .extra_arg("--sponsorblock-remove")
            .extra_arg("all")
            .output_directory(&output_dir)
            .output_template(&format)
            .cookies(cookie.display().to_string())
            .download(true)
            .extract_audio(true)
            .run()
    } else {
        YoutubeDl::new(id)
            .youtube_dl_path("yt-dlp")
            .extra_arg("--audio-format")
            .extra_arg("opus")
            .format("251")
            .extra_arg("--sponsorblock-remove")
            .extra_arg("all")
            .output_directory(output_dir)
            .output_template(format)
            .download(true)
            .extract_audio(true)
            .run()
    }
}
