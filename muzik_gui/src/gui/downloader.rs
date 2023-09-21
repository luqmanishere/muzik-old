use std::sync::Arc;

use iced::{
    widget::{
        container, horizontal_rule, image::Handle, row, scrollable, text, text_input, Button,
        Column, Image, Row, Text,
    },
    Command, Element, Length,
};
use iced_aw::{card, modal, Split, TabLabel};
use muzik_common::{
    data::{Song, Source},
    database::DbConnection,
    entities::{album::AlbumModel, artist::ArtistModel, genre::GenreModel},
    tags::write_tags_song,
    util::{download_video, load_image, search_youtube_async, youtube_dl::SingleVideo},
};
use strum::{Display, EnumIter, IntoEnumIterator};
use tracing::{debug, error, info};

use crate::config::Config;

use super::{
    multi_input::{MultiStringInput, MultiStringInputMessage},
    Actions, Msg, Tab,
};

#[derive(Debug, Clone)]
pub enum DownloaderMsg {
    SourcePick(DownloadSource),
    SearchBarInput(String),
    SearchSubmit,
    YoutubeSearchResult(Vec<SingleVideo>),
    SearchThumbnailLoad(Vec<u8>),
    ResultButton(SingleVideo),
    DownloadThisVideoButton,
    CancelMetadataInput,

    TitleTextInput(String),

    ArtistTextInput((usize, MultiStringInputMessage)),
    AddArtistButton,
    RemoveLastArtistButton,

    AlbumTextInput((usize, MultiStringInputMessage)),
    AddAlbumButton,
    RemoveLastAlbumButton,

    GenreTextInput((usize, MultiStringInputMessage)),
    AddGenreButton,
    RemoveLastGenreButton,

    SubmitChanges,
    DownloadAfterInsert((bool, Song)),
    TagAfterDownload((bool, Song)),
    AfterTagWrite(bool),
    PathUpdateAfterDownload(bool),
}

#[derive(Display, EnumIter, Clone, Eq, PartialEq, Debug, Copy)]
pub enum DownloadSource {
    Youtube,
    Spotify,
}

pub struct DownloaderTab {
    config: Config,
    db: Arc<DbConnection>,

    source_picklist: Option<DownloadSource>,
    search_bar: String,
    search_result: Option<Vec<SingleVideo>>,
    /// If search undergoing, its false. We will lock search submissions
    search_lock: bool,
    selected_result: Option<SingleVideo>,
    selected_result_thumbnail: Option<Vec<u8>>,

    show_metadata_input_modal: bool,
    title_text_input: Option<String>,
    artist_text_input: Option<Vec<MultiStringInput<Msg>>>,
    album_text_input: Option<Vec<MultiStringInput<Msg>>>,
    genre_text_input: Option<Vec<MultiStringInput<Msg>>>,
}

impl DownloaderTab {
    pub fn new(config: Config, db: Arc<DbConnection>) -> (Self, Command<Msg>) {
        let tab = Self {
            config,
            db,
            source_picklist: Some(DownloadSource::Youtube),
            search_bar: String::new(),
            search_result: None,
            search_lock: false,

            selected_result: None,
            selected_result_thumbnail: None,

            show_metadata_input_modal: false,
            title_text_input: None,
            artist_text_input: None,
            album_text_input: None,
            genre_text_input: None,
        };
        (tab, Command::none())
    }

    fn metadata_popup(&self, video: &SingleVideo) -> Element<'_, Msg> {
        let mut sp_col = Column::new().spacing(10);

        let title = Text::new("Title");
        let title_input = iced::widget::TextInput::new(
            &video.title.clone().unwrap_or("Unknown".to_string()),
            self.title_text_input.as_ref().unwrap_or(&String::new()),
        )
        .on_input(|input| Msg::Downloader(DownloaderMsg::TitleTextInput(input)));
        sp_col = sp_col
            .push(title)
            .push(title_input)
            .push(horizontal_rule(1));

        let artist = text("Artists");
        let add_artist_button = Button::new("+")
            .on_press(Msg::Downloader(DownloaderMsg::AddArtistButton))
            .into();
        let remove_artist_button = Button::new("-")
            .on_press(Msg::Downloader(DownloaderMsg::RemoveLastArtistButton))
            .into();
        let mut artist_col = Column::new().spacing(5);
        if let Some(artist_edits) = self.artist_text_input.as_ref() {
            for (id, val) in artist_edits.iter().enumerate() {
                let txt = val.view(id, "artist", |res| {
                    Msg::Downloader(DownloaderMsg::ArtistTextInput(res))
                });
                artist_col = artist_col.push(txt);
            }
        }
        sp_col = sp_col
            .push(
                container(
                    row(vec![artist.into(), add_artist_button, remove_artist_button]).spacing(10),
                )
                .align_y(iced::alignment::Vertical::Center),
            )
            .push(artist_col)
            .push(horizontal_rule(1));

        let album_header = Text::new("Albums");
        let add_album_button = Button::new("+")
            .on_press(Msg::Downloader(DownloaderMsg::AddAlbumButton))
            .into();
        let remove_album_button = Button::new("-")
            .on_press(Msg::Downloader(DownloaderMsg::RemoveLastAlbumButton))
            .into();
        let mut album_col = Column::new().spacing(5);
        if let Some(album_edits) = self.album_text_input.as_ref() {
            for (id, val) in album_edits.iter().enumerate() {
                let text_input = val.view(id, "Album", |res| {
                    Msg::Downloader(DownloaderMsg::AlbumTextInput(res))
                });
                album_col = album_col.push(text_input);
            }
        }
        sp_col = sp_col
            .push(
                container(
                    row(vec![
                        album_header.into(),
                        add_album_button,
                        remove_album_button,
                    ])
                    .spacing(10),
                )
                .align_y(iced::alignment::Vertical::Bottom),
            )
            .push(album_col)
            .push(horizontal_rule(1));

        let genre_header = Text::new("Genre");
        let add_genre_button = Button::new("+")
            .on_press(Msg::Downloader(DownloaderMsg::AddGenreButton))
            .into();
        let remove_genre_button = Button::new("-")
            .on_press(Msg::Downloader(DownloaderMsg::RemoveLastGenreButton))
            .into();
        let mut genre_col = Column::new().spacing(5);
        if let Some(genre_edits) = self.genre_text_input.as_ref() {
            for (id, val) in genre_edits.iter().enumerate() {
                let text_input = val.view(id, "Genres", |res| {
                    Msg::Downloader(DownloaderMsg::GenreTextInput(res))
                });
                genre_col = genre_col.push(text_input);
            }
        }
        sp_col = sp_col
            .push(
                container(
                    row(vec![
                        genre_header.into(),
                        add_genre_button,
                        remove_genre_button,
                    ])
                    .spacing(10),
                )
                .align_y(iced::alignment::Vertical::Bottom),
            )
            .push(genre_col)
            .push(horizontal_rule(1));

        let submit_button = Row::new()
            .push(Button::new("Submit").on_press(Msg::Downloader(DownloaderMsg::SubmitChanges)));
        sp_col = sp_col.push(submit_button);

        scrollable(sp_col).into()
    }

    fn update_source_pick(&mut self, sp: DownloadSource) {
        self.source_picklist = Some(sp)
    }

    fn update_search_bar_input(&mut self, input: String) {
        self.search_bar = input
    }

    fn update_search_submit(&mut self) -> Option<Command<Msg>> {
        match self.source_picklist.expect("exists") {
            DownloadSource::Youtube => {
                let search = self.search_bar.clone();
                let search1 = self.search_bar.clone();
                let cookies = self.config.cookies.clone();
                self.search_lock = true;
                return Some(Command::batch(vec![
                    Command::perform(async {}, |_| {
                        Msg::PushAction(Actions::SearchYoutube(search1))
                    }),
                    Command::perform(
                        async {
                            match search_youtube_async(search, cookies, None).await {
                                Ok(k) => {
                                    info!("youtube search succeded");
                                    k
                                }
                                Err(e) => {
                                    error!("Error searching youtube: {e}");
                                    vec![]
                                }
                            }
                        },
                        |res| Msg::Downloader(DownloaderMsg::YoutubeSearchResult(res)),
                    ),
                ]));
            }
            DownloadSource::Spotify => todo!(),
        }
    }

    fn update_youtube_search_result(&mut self, res: Vec<SingleVideo>) -> Option<Command<Msg>> {
        self.search_result = Some(res);
        self.search_lock = false;
        return Some(Command::perform(async {}, |_| {
            Msg::PushAction(Actions::Done)
        }));
    }

    fn update_result_button(&mut self, video: SingleVideo) -> Option<Command<Msg>> {
        let url = video.thumbnail.clone();
        self.selected_result = Some(video);
        self.selected_result_thumbnail = None;
        return Some(Command::perform(
            async {
                match load_image(url).await {
                    Ok(i) => i,
                    Err(e) => {
                        error!("error loading thumbnail image: {e}");
                        vec![]
                    }
                }
            },
            |res| Msg::Downloader(DownloaderMsg::SearchThumbnailLoad(res)),
        ));
    }
}

impl Tab for DownloaderTab {
    type Message = Msg;

    fn title(&self) -> String {
        "Downloader".into()
    }

    fn tab_label(&self) -> iced_aw::TabLabel {
        TabLabel::Text(self.title())
    }

    fn content(&self) -> iced::Element<'_, Self::Message> {
        let mut main_column: Column<Msg> = Column::new().spacing(10);

        // source, search box and submit button
        let search_text = text("Search").horizontal_alignment(iced::alignment::Horizontal::Center);
        let search_source = iced::widget::pick_list(
            DownloadSource::iter().collect::<Vec<_>>(),
            self.source_picklist,
            |sel| Msg::Downloader(DownloaderMsg::SourcePick(sel)),
        );
        let mut search_bar = text_input("Search me", &self.search_bar)
            .on_input(|inp| Msg::Downloader(DownloaderMsg::SearchBarInput(inp)));
        let mut search_submit_button = Button::new(text("Submit"));
        if !self.search_lock {
            search_bar = search_bar.on_submit(Msg::Downloader(DownloaderMsg::SearchSubmit));
            search_submit_button =
                search_submit_button.on_press(Msg::Downloader(DownloaderMsg::SearchSubmit));
        }
        main_column = main_column
            .push(search_text)
            .push(
                row(vec![
                    search_source.into(),
                    search_bar.into(),
                    search_submit_button.into(),
                ])
                .spacing(5),
            )
            .push(horizontal_rule(1));

        // results area
        let list: Element<'_, Msg> = if let Some(video_list) = self.search_result.as_ref() {
            if !video_list.is_empty() {
                let mut col = Column::new().spacing(5);
                for video in video_list {
                    col = col.push(video.button());
                }
                scrollable(col).into()
            } else {
                text("No search results!").into()
            }
        } else {
            text("Search to view results").into()
        };

        let details: Element<'_, Msg> = if let Some(selected_result) = self.selected_result.as_ref()
        {
            selected_result.details(self.selected_result_thumbnail.as_ref())
        } else {
            text("select a result to view details").into()
        };

        let split = Split::new(list, details, None, iced_aw::split::Axis::Vertical, |_| {
            Msg::None
        });
        main_column = main_column.push(split);

        let overlay = {
            if self.show_metadata_input_modal {
                // construct overlay here
                let fields: Element<'_, Msg> = if let Some(video) = self.selected_result.as_ref() {
                    // TODO: more popup friendly UI
                    self.metadata_popup(video)
                } else {
                    text("test").into()
                };
                Some(
                    card(text("Input Metadata"), fields)
                        .max_width(500.0)
                        .on_close(Msg::Downloader(DownloaderMsg::CancelMetadataInput)),
                )
            } else {
                None
            }
        };

        modal(main_column, overlay).into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if let Msg::Downloader(msg) = message {
            match msg {
                DownloaderMsg::SourcePick(sp) => self.update_source_pick(sp),
                DownloaderMsg::SearchBarInput(input) => self.update_search_bar_input(input),
                DownloaderMsg::SearchSubmit => {
                    if let Some(value) = self.update_search_submit() {
                        return value;
                    }
                }
                DownloaderMsg::YoutubeSearchResult(res) => {
                    if let Some(value) = self.update_youtube_search_result(res) {
                        return value;
                    }
                }
                DownloaderMsg::ResultButton(video) => {
                    if let Some(value) = self.update_result_button(video) {
                        return value;
                    }
                }
                DownloaderMsg::SearchThumbnailLoad(thumbnail) => {
                    self.selected_result_thumbnail = Some(thumbnail)
                }
                DownloaderMsg::DownloadThisVideoButton => {
                    // show overlay to get metadata
                    self.show_metadata_input_modal = true;
                    // reset first
                    self.artist_text_input = None;
                    self.album_text_input = None;
                    self.genre_text_input = None;
                    if let Some(video) = self.selected_result.as_ref() {
                        let title = video.title.clone().unwrap_or("Unknown".to_string());
                        self.title_text_input = Some(title);

                        if let Some(artist) = video.artist.clone() {
                            self.artist_text_input = Some(vec![MultiStringInput::new(artist)]);
                        }

                        if let Some(album) = video.album.clone() {
                            self.album_text_input = Some(vec![MultiStringInput::new(album)]);
                        }

                        if let Some(genre) = video.genre.clone() {
                            self.genre_text_input = Some(vec![MultiStringInput::new(genre)]);
                        }
                    }
                }
                DownloaderMsg::CancelMetadataInput => {
                    self.show_metadata_input_modal = false;
                    self.artist_text_input = None;
                    self.album_text_input = None;
                    self.genre_text_input = None;
                }
                DownloaderMsg::TitleTextInput(input) => self.title_text_input = Some(input),
                DownloaderMsg::ArtistTextInput((id, val)) => {
                    if let Some(artist_input) = self.artist_text_input.as_mut() {
                        artist_input[id].value = val.get_data();
                    }
                }
                DownloaderMsg::AddArtistButton => {
                    debug!("add artist field");
                    if let Some(artists_input) = self.artist_text_input.as_mut() {
                        artists_input.push(MultiStringInput::new(String::new()));
                    } else {
                        self.artist_text_input = Some(vec![MultiStringInput::new(String::new())]);
                    }
                }
                DownloaderMsg::RemoveLastArtistButton => {
                    if let Some(artist_inputs) = self.artist_text_input.as_mut() {
                        artist_inputs.pop();
                    }
                }
                DownloaderMsg::AlbumTextInput((id, val)) => {
                    if let Some(album_input) = self.album_text_input.as_mut() {
                        album_input[id].value = val.get_data();
                    }
                }
                DownloaderMsg::AddAlbumButton => {
                    if let Some(albums_input) = self.album_text_input.as_mut() {
                        albums_input.push(MultiStringInput::new(String::new()));
                    } else {
                        self.album_text_input = Some(vec![MultiStringInput::new(String::new())]);
                    }
                }
                DownloaderMsg::RemoveLastAlbumButton => {
                    if let Some(albums_input) = self.album_text_input.as_mut() {
                        albums_input.pop();
                    }
                }
                DownloaderMsg::GenreTextInput((id, val)) => {
                    if let Some(genre_input) = self.genre_text_input.as_mut() {
                        genre_input[id].value = val.get_data();
                    }
                }
                DownloaderMsg::AddGenreButton => {
                    if let Some(genre_input) = self.genre_text_input.as_mut() {
                        genre_input.push(MultiStringInput::new(String::new()));
                    } else {
                        self.genre_text_input = Some(vec![MultiStringInput::new(String::new())]);
                    }
                }
                DownloaderMsg::RemoveLastGenreButton => {
                    if let Some(genre_input) = self.genre_text_input.as_mut() {
                        genre_input.pop();
                    }
                }
                DownloaderMsg::SubmitChanges => {
                    self.show_metadata_input_modal = false;
                    if let Some(video) = self.selected_result.as_ref() {
                        let mut song = Song::new();
                        song.music_dir = self.config.get_music_dir();

                        let title = self
                            .title_text_input
                            .clone()
                            .unwrap_or("Unknown".to_string());
                        song.set_title(title);

                        if let Some(artist_text_inputs) = self.artist_text_input.as_ref() {
                            let artists_vec: Vec<_> = artist_text_inputs
                                .iter()
                                .map(|s| ArtistModel {
                                    name: s.value.clone(),
                                    ..Default::default()
                                })
                                .collect();
                            song.set_artists(artists_vec);
                        }
                        if let Some(album_text_inputs) = self.album_text_input.as_ref() {
                            let albums_vec: Vec<_> = album_text_inputs
                                .iter()
                                .map(|s| AlbumModel {
                                    name: s.value.clone(),
                                    ..Default::default()
                                })
                                .collect();
                            song.set_albums(albums_vec);
                        }
                        if let Some(genre_text_inputs) = self.genre_text_input.as_ref() {
                            let genres_vec: Vec<_> = genre_text_inputs
                                .iter()
                                .map(|s| GenreModel {
                                    genre: s.value.clone(),
                                    ..Default::default()
                                })
                                .collect();
                            song.set_genres(genres_vec);
                        }
                        song.youtube_id = Some(video.id.clone());
                        song.set_source(Source::Youtube);
                        if let Some(thumbnail) = video.thumbnail.clone() {
                            song.set_thumbnail_url(thumbnail);
                        };

                        debug!("{:?}", &song);
                        let db = self.db.clone();
                        return Command::perform(
                            async move {
                                match db.insert_from_gui_song(song.clone()).await {
                                    Ok(song_with_id) => {
                                        info!("insert database entries successfully");
                                        (true, song_with_id)
                                    }
                                    Err(e) => {
                                        error!("error updating database: {e}");
                                        (false, song)
                                    }
                                }
                            },
                            |res| Msg::Downloader(DownloaderMsg::DownloadAfterInsert(res)),
                        );
                    }
                }
                DownloaderMsg::DownloadAfterInsert((res, song)) => {
                    if res {
                        match song.source {
                            Source::Youtube => {
                                let db_id = song.id.expect("youtube_id exists");
                                let youtube_id = song.youtube_id.clone().expect("exists");
                                let title = song.get_title_string();
                                let artists = song.get_artists_string();
                                let music_dir = self.config.get_music_dir();
                                let cookies = self.config.cookies.clone();

                                let filename_format =
                                    format!("{} - {} [{}].%(ext)s", title, artists, db_id);

                                let song = song.clone();
                                return Command::perform(
                                    async move {
                                        match download_video(
                                            youtube_id,
                                            music_dir,
                                            filename_format,
                                            cookies,
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                info!("download success!");
                                                return (true, song);
                                            }
                                            Err(e) => {
                                                error!("unable to download from youtube: {e}");
                                                return (false, song);
                                            }
                                        }
                                    },
                                    |res| Msg::Downloader(DownloaderMsg::TagAfterDownload(res)),
                                );
                            }
                            _ => {}
                        }
                    } else {
                    }
                }
                DownloaderMsg::TagAfterDownload((result, song)) => {
                    if result {
                        debug!("previous download succeeded, tagging");
                        let mut song = song.clone();
                        let db_id = song.id.expect("youtube_id exists");
                        let title = song.get_title_string();
                        let artists = song.get_artists_string();
                        let path = self
                            .config
                            .get_music_dir()
                            .join(format!("{} - {} [{}].opus", title, artists, db_id));
                        let db = self.db.clone();

                        // properly set the path of the song
                        song.set_path(path.clone());

                        let song_write = song.clone();
                        return Command::batch(vec![
                            Command::perform(
                                async move {
                                    match write_tags_song(path, &song_write).await {
                                        Ok(_) => {
                                            info!("successfully wrote tags to file");
                                            true
                                        }
                                        Err(e) => {
                                            error!("failed to write tags to file: {e}");
                                            false
                                        }
                                    }
                                },
                                |res| Msg::Downloader(DownloaderMsg::AfterTagWrite(res)),
                            ),
                            Command::perform(
                                async move {
                                    match db.update_song_from_gui_song(song).await {
                                        Ok(_) => {
                                            info!("updated database entries post download");
                                            return true;
                                        }
                                        Err(e) => {
                                            error!(
                                            "unable to update database entries post download: {e}"
                                        );
                                            return false;
                                        }
                                    }
                                },
                                |res| Msg::Downloader(DownloaderMsg::PathUpdateAfterDownload(res)),
                            ),
                        ]);
                    } else {
                        error!("previous download failed, no tagging to be done");
                    }
                }
                DownloaderMsg::AfterTagWrite(_res) => {
                    return Command::perform(async {}, |_| {
                        Msg::Editor(super::editor::EditorMessage::ReloadButton)
                    })
                }
                DownloaderMsg::PathUpdateAfterDownload(result) => match result {
                    true => {}
                    false => {}
                },
            }
        }
        Command::none()
    }
}

trait SearchDisplay {
    type Message;

    fn button(&self) -> Element<Self::Message>;

    fn details(&self, thumbnail: Option<&Vec<u8>>) -> Element<Self::Message>;
}

impl SearchDisplay for SingleVideo {
    type Message = Msg;

    fn button(&self) -> Element<Self::Message> {
        let t = text(format!(
            "{}",
            self.title.clone().unwrap_or("Unknown".to_string())
        ));
        // TODO: better styling
        Button::new(t)
            .on_press(Msg::Downloader(DownloaderMsg::ResultButton(self.clone())))
            .width(Length::Fill)
            .style(iced::theme::Button::Custom(Box::new(
                super::theme::DbButton,
            )))
            .into()
    }

    fn details(&self, thumbnail: Option<&Vec<u8>>) -> Element<Self::Message> {
        let mut main_column = Column::new()
            .spacing(5)
            .padding(10)
            .align_items(iced::Alignment::Center);

        // TODO: implement image caching for current session
        let img: Element<_> = if let Some(pic_vec) = thumbnail {
            if !pic_vec.is_empty() {
                // todo: add frame around image display
                // todo: make image display resizable
                Image::new(Handle::from_memory(pic_vec.clone()))
                    .width(200)
                    .height(200)
                    .into()
            } else {
                text("no thumbnail is found!").into()
            }
        } else {
            text("thumbnail is loading").into()
        };
        main_column = main_column.push(img).push(horizontal_rule(1));

        let mut title_row = Row::new()
            .spacing(5)
            .width(Length::Fill)
            .align_items(iced::Alignment::Center);
        let mut channel_row = Row::new()
            .spacing(5)
            .width(Length::Fill)
            .align_items(iced::Alignment::Center);
        let mut upload_date_row = Row::new()
            .spacing(5)
            .width(Length::Fill)
            .align_items(iced::Alignment::Center);

        let title_header = text("Title")
            .horizontal_alignment(iced::alignment::Horizontal::Center)
            .vertical_alignment(iced::alignment::Vertical::Center)
            .width(Length::Fill);
        let title = text(self.title.clone().unwrap_or("Unknown".to_string()))
            .horizontal_alignment(iced::alignment::Horizontal::Center)
            .vertical_alignment(iced::alignment::Vertical::Center)
            .width(Length::Fill)
            .shaping(text::Shaping::Advanced);

        let channel_header = text("Channel")
            .horizontal_alignment(iced::alignment::Horizontal::Center)
            .vertical_alignment(iced::alignment::Vertical::Center)
            .width(Length::Fill);
        let channel = text(self.channel.clone().unwrap_or("Unknown".to_string()))
            .horizontal_alignment(iced::alignment::Horizontal::Center)
            .vertical_alignment(iced::alignment::Vertical::Center)
            .width(Length::Fill);

        let upload_date_header = text("Upload date")
            .horizontal_alignment(iced::alignment::Horizontal::Center)
            .vertical_alignment(iced::alignment::Vertical::Center)
            .width(Length::Fill);
        let upload_date = text(self.upload_date.clone().unwrap_or("Unknown".to_string()))
            .horizontal_alignment(iced::alignment::Horizontal::Center)
            .vertical_alignment(iced::alignment::Vertical::Center)
            .width(Length::Fill);

        let mut select_download_button = Button::new("Download this");
        select_download_button = select_download_button
            .on_press(Msg::Downloader(DownloaderMsg::DownloadThisVideoButton));

        title_row = title_row.push(title_header).push(title);
        channel_row = channel_row.push(channel_header).push(channel);
        upload_date_row = upload_date_row.push(upload_date_header).push(upload_date);

        main_column = main_column
            .push(title_row)
            .push(horizontal_rule(1))
            .push(channel_row)
            .push(horizontal_rule(1))
            .push(upload_date_row)
            .push(horizontal_rule(1))
            .push(select_download_button);

        container(scrollable(main_column))
            .align_x(iced::alignment::Horizontal::Center)
            .into()
    }
}
