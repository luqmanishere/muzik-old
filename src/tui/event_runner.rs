use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread::JoinHandle,
};

use tracing::{debug, error, info};
use youtube_dl::{SearchOptions, SingleVideo, YoutubeDl};

#[allow(dead_code)]
pub struct EventRunner {
    thread_handle: Option<JoinHandle<()>>,
    cb_sink: CbSink,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    config: Config,
}
use cursive::{
    view::{Nameable, Scrollable},
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

    pub fn process(&self) {
        match self.rx.recv().unwrap() {
            Event::YoutubeSearch(kw) => {
                // TODO: add a searching notification
                self.cb_sink
                    .send(Box::new(|siv: &mut Cursive| {
                        let text = Dialog::text("Searching").with_name("search");
                        siv.add_layer(text);
                    }))
                    .unwrap();

                match search_youtube(kw) {
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
                                siv.call_on_name("search", |view: &mut Dialog| {
                                    view.set_content(TextView::new("Done"));
                                    view.add_button("Dismiss", |siv: &mut Cursive| {
                                        siv.pop_layer();
                                    });
                                    view.set_focus(cursive::views::DialogFocus::Button(0));
                                });
                            }))
                            .unwrap();
                    }
                    Err(_) => todo!(),
                }
            }
            Event::YoutubeDownload(dl_options) => {
                debug!(
                    "got id: {}, title: {}, artist: {}, album: {}",
                    &dl_options.id, &dl_options.title, &dl_options.artist, &dl_options.album
                );
                let title = dl_options.title.clone();
                let artist = dl_options.artist.clone();
                self.cb_sink
                    .send(Box::new(move |siv: &mut Cursive| {
                        let popup = Dialog::text(format!(
                            "Downloading: {}: {}",
                            title,
                            artist
                        ))
                        .dismiss_button("Dismiss");
                        siv.add_layer(popup);
                    }))
                    .unwrap();

                let filename_format =
                    format!("{} - {}.%(ext)s", dl_options.title, dl_options.artist);
                let filename = format!("{} - {}.opus", dl_options.title, dl_options.artist);
                let filename = dl_options.music_dir.join(filename);
                let _youtube = YoutubeDl::new(dl_options.id.clone())
                    .youtube_dl_path("yt-dlp")
                    .extra_arg("--audio-format")
                    .extra_arg("opus")
                    .extra_arg("--downloader")
                    .extra_arg("aria2c")
                    .extra_arg("--sponsorblock-remove")
                    .extra_arg("all")
                    .extra_arg("-P")
                    .extra_arg(dl_options.music_dir.to_str().unwrap())
                    .extra_arg("-o")
                    .extra_arg(filename_format)
                    .download(true)
                    .extract_audio(true)
                    .run()
                    .unwrap();

                if filename.exists() {
                    let title = dl_options.title.clone();
                    let artist = dl_options.artist.clone();
                    self.cb_sink
                        .send(Box::new(move |siv: &mut Cursive| {
                            siv.add_layer(
                                Dialog::text(format!(
                                    "Download finished! for: {} - {}",
                                    title, artist
                                ))
                                .dismiss_button("Ok"),
                            );
                        }))
                        .unwrap();
                } else {
                    println!("File not found after downloading");
                }

                let song = Song::new(
                    dl_options.music_dir,
                    None,
                    Some(filename.display().to_string()),
                    Some(dl_options.title),
                    Some(dl_options.album),
                    Some(dl_options.artist),
                    Some(dl_options.id),
                    dl_options.song.thumbnail,
                );
                self.tx.send(Event::InsertTags(song)).unwrap();
            }
            Event::InsertTags(song) => {
                let filename = song.path.as_ref().unwrap();
                match tags::write_tags(filename.into(), &song) {
                    Ok(_) => {
                        info!("wrote tags to file successfully");
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
                Ok(_) => info!("inserted into database successfully"),
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
        }
    }

    pub fn get_tx(&self) -> Sender<Event> {
        self.tx.clone()
    }
}

pub enum Event {
    YoutubeSearch(String),
    YoutubeDownload(YoutubeDownloadOptions),
    InsertTags(Song),
    InsertSongDatabase(Song),
    UpdateSongDatabase(Song),
    UpdateTags(Song),
    DeleteSongDatabase(Song),
}

pub struct YoutubeDownloadOptions {
    pub id: String,
    pub title: String,
    pub album: String,
    pub artist: String,
    pub song: SingleVideo,
    pub music_dir: PathBuf,
}

use eyre::Result;

use crate::{config::Config, database::Song, tags, tui::editor};

fn search_youtube(kw: String) -> Result<Vec<SingleVideo>> {
    let search_options = SearchOptions::youtube(kw).with_count(5);
    let yt_search = YoutubeDl::search_for(&search_options)
        .youtube_dl_path("yt-dlp")
        .run()
        .unwrap();

    match yt_search {
        youtube_dl::YoutubeDlOutput::Playlist(playlist) => {
            let entries = playlist.entries.unwrap();
            Ok(entries)
        }
        youtube_dl::YoutubeDlOutput::SingleVideo(video) => Ok(vec![*video]),
    }
}
