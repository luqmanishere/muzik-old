use std::sync::Arc;

use iced::{
    alignment,
    widget::{
        checkbox, column, container, horizontal_rule, image::Handle, row, scrollable, text, Button,
        Column, Image, Text,
    },
    Command, Element, Length,
};
use iced_aw::{Split, TabLabel};
use tracing::{debug, error, info, trace};

use crate::config::Config;
use muzik_common::{
    data::{self, load_songs, Song},
    database::DbConnection,
    entities::{album::AlbumModel, artist::ArtistModel, genre::GenreModel},
    tags::{self, write_tags_song},
};

use super::{
    multi_input::{MultiStringInput, MultiStringInputMessage},
    Msg, Tab,
};

#[derive(Debug, Clone)]
pub enum EditorMessage {
    ReloadButton,
    LoadSongs(Vec<Song>),
    DbVisibleToggle(bool),
    DividerResize(u16),
    SongButton(Song),
    LoadSongImage(Vec<u8>),

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

    SubmitChanges(),
    WriteAfterInsertSong((bool, Song)),
    AfterTagWrite(bool),
}

pub struct EditorTab {
    config: Config,
    db: Arc<DbConnection>,
    db_songs_visibility: bool,
    songs_vec: Option<Vec<Song>>,
    hor_divider_pos: Option<u16>,

    current_app_song: Option<Song>,
    current_app_song_image: Option<Vec<u8>>,

    title_text_input: Option<String>,
    artist_text_input: Option<Vec<MultiStringInput<Msg>>>,
    album_text_input: Option<Vec<MultiStringInput<Msg>>>,
    genre_text_input: Option<Vec<MultiStringInput<Msg>>>,
}

impl EditorTab {
    pub fn new_with_command(config: Config, db: Arc<DbConnection>) -> (Self, Command<Msg>) {
        let music_dir = config.get_music_dir();
        let db_conn = db.clone();
        (
            Self {
                config,
                db,
                db_songs_visibility: false,
                songs_vec: None,
                hor_divider_pos: None,

                current_app_song: None,
                current_app_song_image: None,

                title_text_input: None,
                artist_text_input: None,
                album_text_input: None,
                genre_text_input: None,
            },
            Command::perform(async { load_songs(music_dir, db_conn).await }, |result| {
                Msg::Editor(EditorMessage::LoadSongs(result))
            }),
        )
    }

    pub fn reset_input_fields(&mut self) {
        self.title_text_input = Some(if let Some(song) = self.current_app_song.as_ref() {
            song.get_title_string()
        } else {
            String::new()
        });

        if let Some(song) = self.current_app_song.as_ref() {
            // reset all song related text fields

            // reset artist text input
            self.artist_text_input = {
                let artists = song.get_artists_vec();
                if !artists.is_empty() {
                    Some(
                        artists
                            .iter()
                            .map(|a| MultiStringInput::new(a.clone()))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            };

            // reset album text input
            self.album_text_input = {
                let albums = song.get_albums_vec();
                if !albums.is_empty() {
                    Some(
                        albums
                            .iter()
                            .map(|a| MultiStringInput::new(a.clone()))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            };

            // reset genre text input
            self.genre_text_input = {
                let genres = song.get_genres_vec();
                if !genres.is_empty() {
                    Some(
                        genres
                            .iter()
                            .map(|a| MultiStringInput::new(a.clone()))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            };
        };
    }
}

impl Tab for EditorTab {
    type Message = Msg;

    fn title(&self) -> String {
        "editor".to_string()
    }

    fn tab_label(&self) -> TabLabel {
        TabLabel::Text(self.title())
    }

    fn content(&self) -> Element<'_, Self::Message> {
        trace!("content render start!");
        let reload_button = Button::new("Reload")
            .on_press(Self::Message::Editor(EditorMessage::ReloadButton))
            .into();
        let songs: Element<_> = {
            let mut songs = vec![];

            if let Some(local_songs) = self.songs_vec.as_ref() {
                if self.db_songs_visibility {
                    for item in local_songs.into_iter().map(|msongs| msongs.view()) {
                        songs.push(item);
                    }
                } else {
                    for item in local_songs.into_iter().filter_map(|msongs| {
                        if msongs.is_database_only() {
                            None
                        } else {
                            Some(msongs.view())
                        }
                    }) {
                        songs.push(item);
                    }
                }
            } else {
                return text("loading...").into();
            }

            if !songs.is_empty() {
                scrollable(column(songs)).into()
            } else {
                text("no songs! try toggeling DB view or adding new entries").into()
            }
        };

        let second_panel: Element<_> = if let Some(song) = self.current_app_song.as_ref() {
            let mut sp_col = Column::new().spacing(10);

            // render image if available
            let img_header = text("Image");
            let img: Element<_> = if let Some(pic_vec) = self.current_app_song_image.as_ref() {
                if !pic_vec.is_empty() {
                    // todo: add frame around image display
                    // todo: make image display resizable
                    Image::new(Handle::from_memory(pic_vec.clone()))
                        .width(200)
                        .height(200)
                        .into()
                } else {
                    text("no image found!").into()
                }
            } else {
                text("loading").into()
            };
            sp_col = sp_col.push(img_header).push(img);

            let title = Text::new("Title");
            let title_input = iced::widget::TextInput::new(
                &song.get_title_string(),
                self.title_text_input.as_ref().unwrap_or(&String::new()),
            )
            .on_input(|input| Msg::Editor(EditorMessage::TitleTextInput(input)));
            sp_col = sp_col
                .push(title)
                .push(title_input)
                .push(horizontal_rule(1));

            let path = {
                let path = if let Some(path) = song.path.as_ref() {
                    let conv = path.display().to_string();
                    if !conv.is_empty() && path.exists() {
                        conv
                    } else if !conv.is_empty() && !path.exists() {
                        format!("{} - Not on disk", conv)
                    } else {
                        "No path provided".to_string()
                    }
                } else {
                    "No path provided".to_string()
                };

                text(format!("Local path: {}", path))
            };
            sp_col = sp_col.push(path).push(horizontal_rule(1));

            let artist = text("Artists");
            let add_artist_button = Button::new("Add Artist")
                .on_press(Msg::Editor(EditorMessage::AddArtistButton))
                .into();
            let remove_artist_button = Button::new("Remove Last Artist")
                .on_press(Msg::Editor(EditorMessage::RemoveLastArtistButton))
                .into();
            let mut artist_col = Column::new().spacing(5);
            if let Some(artist_edits) = self.artist_text_input.as_ref() {
                for (id, val) in artist_edits.iter().enumerate() {
                    let txt = val.view(id, "artist", |res| {
                        Msg::Editor(EditorMessage::ArtistTextInput(res))
                    });
                    artist_col = artist_col.push(txt);
                }
            }
            sp_col = sp_col
                .push(
                    container(
                        row(vec![artist.into(), add_artist_button, remove_artist_button])
                            .spacing(10),
                    )
                    .align_y(alignment::Vertical::Center),
                )
                .push(artist_col)
                .push(horizontal_rule(1));

            let album_header = Text::new("Albums");
            let add_album_button = Button::new("Add Album")
                .on_press(Msg::Editor(EditorMessage::AddAlbumButton))
                .into();
            let remove_album_button = Button::new("Remove Album")
                .on_press(Msg::Editor(EditorMessage::RemoveLastAlbumButton))
                .into();
            let mut album_col = Column::new().spacing(5);
            if let Some(album_edits) = self.album_text_input.as_ref() {
                for (id, val) in album_edits.iter().enumerate() {
                    let text_input = val.view(id, "Album", |res| {
                        Msg::Editor(EditorMessage::AlbumTextInput(res))
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
                    .align_y(alignment::Vertical::Bottom),
                )
                .push(album_col)
                .push(horizontal_rule(1));

            let genre_header = Text::new("Genre");
            let add_genre_button = Button::new("Add Genre")
                .on_press(Msg::Editor(EditorMessage::AddGenreButton))
                .into();
            let remove_genre_button = Button::new("Remove Genre")
                .on_press(Msg::Editor(EditorMessage::RemoveLastGenreButton))
                .into();
            let mut genre_col = Column::new().spacing(5);
            if let Some(genre_edits) = self.genre_text_input.as_ref() {
                for (id, val) in genre_edits.iter().enumerate() {
                    let text_input = val.view(id, "Genres", |res| {
                        Msg::Editor(EditorMessage::GenreTextInput(res))
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
                    .align_y(alignment::Vertical::Bottom),
                )
                .push(genre_col)
                .push(horizontal_rule(1));

            let source_disp = text(format!("Source: {}", song.source));
            sp_col = sp_col.push(source_disp);

            // only show this if the source is Youtube
            if song.source == data::Source::Youtube {
                let youtube_id = {
                    let id = if let Some(youtube_id) = song.youtube_id.as_ref() {
                        format!("Youtube ID: {}", youtube_id)
                    } else {
                        "Youtube ID: None".to_string()
                    };
                    text(id)
                };
                sp_col = sp_col.push(youtube_id).push(horizontal_rule(1));
                // TODO: allow editing yt playlists
            }

            let submit_button =
                Button::new("Submit").on_press(Msg::Editor(EditorMessage::SubmitChanges()));
            sp_col = sp_col.push(submit_button);

            scrollable(sp_col).into()
        } else {
            text("select a song!").into()
        };

        let row = column(vec![
            checkbox("Show items from database", self.db_songs_visibility, |b| {
                Self::Message::Editor(EditorMessage::DbVisibleToggle(b))
            })
            .into(),
            reload_button,
            container(Split::new(
                songs,
                second_panel,
                self.hor_divider_pos,
                iced_aw::split::Axis::Vertical,
                |resize| Self::Message::Editor(EditorMessage::DividerResize(resize)),
            ))
            .height(Length::Fill)
            .width(Length::Fill)
            .into(),
        ])
        .padding(5)
        .spacing(10);

        // return the main container
        container(row)
            .center_x()
            .center_y()
            .padding(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        if let Self::Message::Editor(msg) = message {
            match msg {
                EditorMessage::LoadSongs(songs) => self.songs_vec = Some(songs),
                EditorMessage::DbVisibleToggle(b) => self.db_songs_visibility = b,
                EditorMessage::DividerResize(size) => self.hor_divider_pos = Some(size),
                EditorMessage::SongButton(song) => {
                    self.current_app_song = Some(song.clone());
                    self.current_app_song_image = None;
                    self.reset_input_fields();
                    if let Some(path) = song.path.clone() {
                        return Command::perform(
                            async {
                                match tags::read_picture(path).await {
                                    Ok(pic) => pic,
                                    Err(e) => {
                                        error!("{e}");
                                        vec![]
                                    }
                                }
                            },
                            |res| Msg::Editor(EditorMessage::LoadSongImage(res)),
                        );
                    }
                }
                EditorMessage::LoadSongImage(pic) => {
                    self.current_app_song_image = Some(pic);
                }
                EditorMessage::ReloadButton => {
                    let db_action2 = self.db.clone();
                    let music_dir2 = self.config.get_music_dir();
                    return Command::batch(vec![Command::perform(
                        async { load_songs(music_dir2, db_action2).await },
                        |result| Self::Message::Editor(EditorMessage::LoadSongs(result)),
                    )]);
                }
                EditorMessage::TitleTextInput(input) => self.title_text_input = Some(input),
                EditorMessage::ArtistTextInput((id, val)) => {
                    if let Some(artist_input) = self.artist_text_input.as_mut() {
                        artist_input[id].value = val.get_data();
                    }
                }
                EditorMessage::AddArtistButton => {
                    if let Some(artists_input) = self.artist_text_input.as_mut() {
                        artists_input.push(MultiStringInput::new(String::new()));
                    }
                }
                EditorMessage::RemoveLastArtistButton => {
                    if let Some(artist_inputs) = self.artist_text_input.as_mut() {
                        artist_inputs.pop();
                    }
                }
                EditorMessage::AlbumTextInput((id, val)) => {
                    if let Some(album_input) = self.artist_text_input.as_mut() {
                        album_input[id].value = val.get_data();
                    }
                }
                EditorMessage::AddAlbumButton => {
                    if let Some(albums_input) = self.album_text_input.as_mut() {
                        albums_input.push(MultiStringInput::new(String::new()));
                    }
                }
                EditorMessage::RemoveLastAlbumButton => {
                    if let Some(albums_input) = self.album_text_input.as_mut() {
                        albums_input.pop();
                    }
                }
                EditorMessage::GenreTextInput((id, val)) => {
                    if let Some(genre_input) = self.genre_text_input.as_mut() {
                        genre_input[id].value = val.get_data();
                    }
                }
                EditorMessage::AddGenreButton => {
                    if let Some(genre_input) = self.genre_text_input.as_mut() {
                        genre_input.push(MultiStringInput::new(String::new()));
                    }
                }
                EditorMessage::RemoveLastGenreButton => {
                    if let Some(genre_input) = self.genre_text_input.as_mut() {
                        genre_input.pop();
                    }
                }
                EditorMessage::SubmitChanges() => {
                    // todo: if in database, update. if not in database, add new
                    if let Some(current_song) = self.current_app_song.as_ref() {
                        match current_song.id {
                            Some(_id) => {
                                // is in database
                                // TODO: update song based on given inputs
                                let mut song = current_song.clone();

                                if let Some(artist_text_inputs) = self.artist_text_input.as_ref() {
                                    let artists_vec: Vec<_> = artist_text_inputs
                                        .iter()
                                        .map(|s| ArtistModel {
                                            name: s.value.clone(),
                                            ..Default::default()
                                        })
                                        .collect();
                                    song.set_artists(artists_vec);
                                    // todo: get other values
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
                                debug!("{:?}", &song);
                                let db = self.db.clone();
                                return Command::perform(
                                    async move {
                                        match db.update_all_from_gui_song(song.clone()).await {
                                            Ok(_) => {
                                                info!("update database entries successfully");
                                                (true, song)
                                            }
                                            Err(e) => {
                                                error!("error updating database: {e}");
                                                (false, song)
                                            }
                                        }
                                    },
                                    |res| Msg::Editor(EditorMessage::WriteAfterInsertSong(res)),
                                );
                            }
                            None => {
                                // is not in a database
                                let mut song = current_song.clone();
                                if let Some(artist_text_inputs) = self.artist_text_input.as_ref() {
                                    let artists_vec: Vec<_> = artist_text_inputs
                                        .iter()
                                        .map(|s| ArtistModel {
                                            name: s.value.clone(),
                                            ..Default::default()
                                        })
                                        .collect();
                                    song.set_artists(artists_vec);
                                    // todo: get other values
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
                                dbg!(&song);
                                let db = self.db.clone();
                                let song_clone = song.clone();
                                return Command::perform(
                                    async move {
                                        match db.insert_from_gui_song(song).await {
                                            Ok(new_song) => {
                                                info!("insert into database successfully");
                                                (true, new_song)
                                            }
                                            Err(e) => {
                                                error!("error inserting song into database: {e}");
                                                (false, song_clone)
                                            }
                                        }
                                    },
                                    |res| Msg::Editor(EditorMessage::WriteAfterInsertSong(res)),
                                );
                            }
                        }
                    }
                }
                EditorMessage::WriteAfterInsertSong((res, song)) => match res {
                    true => {
                        let path = song.path.clone().expect("inserted song has path");
                        return Command::perform(
                            async move {
                                match write_tags_song(path, &song).await {
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
                            |res| Msg::Editor(EditorMessage::AfterTagWrite(res)),
                        );
                    }
                    false => {}
                },
                EditorMessage::AfterTagWrite(_res) => {
                    return Command::perform(async {}, |_| Msg::Editor(EditorMessage::ReloadButton))
                }
            }
            Command::none()
        } else {
            Command::none()
        }
    }
}

trait Disp {
    type Message;

    fn view(&self) -> Element<Self::Message>;
}

impl Disp for Song {
    type Message = Msg;

    fn view(&self) -> Element<Self::Message> {
        // TODO: display picture
        let t = format!(
            "{} - {} [{}]",
            self.get_title_string(),
            self.get_artists_string(),
            self.identify()
        );
        let row = row(vec![text(t).into()]);
        if self.in_database {
            Button::new(row)
                .on_press(Msg::Editor(EditorMessage::SongButton(self.clone())))
                .width(Length::Fill)
                .style(iced::theme::Button::Custom(Box::new(
                    super::theme::DbButton,
                )))
                .into()
        } else {
            Button::new(row)
                .on_press(Msg::Editor(EditorMessage::SongButton(self.clone())))
                .width(Length::Fill)
                .style(iced::theme::Button::Custom(Box::new(
                    super::theme::LocalButton,
                )))
                .into()
        }
    }
}
