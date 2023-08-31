use std::{io::Cursor, path::PathBuf};

use eyre::{eyre, Result};
use iced::futures::future::ErrInto;
use lofty::{Accessor, ItemKey, ItemValue, Picture, Probe, Tag, TagExt, TagItem, TaggedFileExt};
use tracing::error;

use crate::{
    database::AppSong,
    entities::{
        album::AlbumModel,
        artist::ArtistModel,
        genre::{self, GenreModel},
    },
    gui::data::Song,
};

pub async fn write_tags_async(path: PathBuf, song: &AppSong) -> Result<()> {
    write_tags(path, song).await
}

pub async fn write_tags(path: PathBuf, song: &AppSong) -> Result<()> {
    match Probe::open(path.clone())?.read() {
        Ok(mut tagged_file) => {
            let tag = match tagged_file.primary_tag_mut() {
                Some(primary_tag) => primary_tag,
                None => {
                    if let Some(first_tag) = tagged_file.first_tag_mut() {
                        first_tag
                    } else {
                        let tag_type = tagged_file.primary_tag_type();

                        println!("no tags found, creating new tags of type {:?}", tag_type);
                        tagged_file.insert_tag(Tag::new(tag_type));
                        tagged_file.primary_tag_mut().unwrap()
                    }
                }
            };

            let mut tag_items = vec![];

            if let Some(title) = &song.title {
                tag.remove_key(&ItemKey::TrackTitle);
                let tag_item = TagItem::new_checked(
                    tag.tag_type(),
                    ItemKey::TrackTitle,
                    lofty::ItemValue::Text(title.to_string()),
                )
                .unwrap();
                tag_items.push(tag_item);
            }

            if let Some(artists) = &song.artist {
                tag.remove_key(&ItemKey::TrackArtist);
                for artist in artists {
                    let tag_item = TagItem::new_checked(
                        tag.tag_type(),
                        ItemKey::TrackArtist,
                        lofty::ItemValue::Text(artist.name.clone()),
                    )
                    .unwrap();

                    tag_items.push(tag_item);
                }
            }

            if let Some(albums) = &song.album {
                tag.remove_key(&ItemKey::AlbumTitle);
                for album in albums {
                    let tag_item = TagItem::new_checked(
                        tag.tag_type(),
                        ItemKey::AlbumTitle,
                        lofty::ItemValue::Text(album.name.clone()),
                    )
                    .unwrap();

                    tag_items.push(tag_item);
                }
            }

            if let Some(genre) = &song.genre {
                tag.remove_key(&ItemKey::Genre);
                for it in genre {
                    let tag_item = TagItem::new_checked(
                        tag.tag_type(),
                        ItemKey::Genre,
                        ItemValue::Text(it.genre.clone()),
                    )
                    .unwrap();

                    tag_items.push(tag_item);
                }
            }

            if let Some(yt_id) = &song.yt_id {
                tag.remove_key(&ItemKey::Unknown("YTID".to_string()));
                let tag_item = TagItem::new(
                    ItemKey::Unknown("YTID".to_string()),
                    ItemValue::Text(yt_id.to_string()),
                );
                tag.insert_unchecked(tag_item);
            }

            if let Some(id) = &song.id {
                tag.remove_key(&ItemKey::Unknown("ID".to_string()));
                let tag_item = TagItem::new(
                    ItemKey::Unknown("ID".to_string()),
                    ItemValue::Text(id.to_string()),
                );
                tag.insert_unchecked(tag_item);
            }

            // TODO: write database id

            if let Some(picture_url) = &song.tb_url {
                tag.remove_picture_type(lofty::PictureType::CoverFront);
                if picture_url.contains("http") {
                    let picture = reqwest::get(picture_url);
                    match picture.await {
                        Ok(request) => {
                            let mut pict: Vec<u8> = vec![];
                            let pic = image::load_from_memory(&request.bytes().await?)?;
                            pic.write_to(&mut Cursor::new(&mut pict), image::ImageFormat::Png)?;

                            let lofty_pic = Picture::new_unchecked(
                                lofty::PictureType::CoverFront,
                                lofty::MimeType::Png,
                                None,
                                pict,
                            );
                            tag.push_picture(lofty_pic);
                        }
                        Err(e) => {
                            error!("unable to get image: {}", e);
                        }
                    }
                }
            }

            for tag_item in tag_items {
                tag.push(tag_item);
            }

            tag.save_to_path(path)?;
            Ok(())
        }
        Err(e) => Err(eyre!(e)),
    }
}

pub async fn write_tags_song(path: PathBuf, song: &Song) -> Result<()> {
    match Probe::open(path.clone())?.read() {
        Ok(mut tagged_file) => {
            let tag = match tagged_file.primary_tag_mut() {
                Some(primary_tag) => primary_tag,
                None => {
                    if let Some(first_tag) = tagged_file.first_tag_mut() {
                        first_tag
                    } else {
                        let tag_type = tagged_file.primary_tag_type();

                        println!("no tags found, creating new tags of type {:?}", tag_type);
                        tagged_file.insert_tag(Tag::new(tag_type));
                        tagged_file.primary_tag_mut().unwrap()
                    }
                }
            };

            let mut tag_items = vec![];

            if let Some(title) = &song.title {
                tag.remove_key(&ItemKey::TrackTitle);
                let tag_item = TagItem::new_checked(
                    tag.tag_type(),
                    ItemKey::TrackTitle,
                    lofty::ItemValue::Text(title.to_string()),
                )
                .unwrap();
                tag_items.push(tag_item);
            }

            if let Some(artists) = &song.artists {
                tag.remove_key(&ItemKey::TrackArtist);
                for artist in artists {
                    let tag_item = TagItem::new_checked(
                        tag.tag_type(),
                        ItemKey::TrackArtist,
                        lofty::ItemValue::Text(artist.name.clone()),
                    )
                    .unwrap();

                    tag_items.push(tag_item);
                }
            }

            if let Some(albums) = &song.albums {
                tag.remove_key(&ItemKey::AlbumTitle);
                for album in albums {
                    let tag_item = TagItem::new_checked(
                        tag.tag_type(),
                        ItemKey::AlbumTitle,
                        lofty::ItemValue::Text(album.name.clone()),
                    )
                    .unwrap();

                    tag_items.push(tag_item);
                }
            }

            if let Some(genre) = &song.genres {
                tag.remove_key(&ItemKey::Genre);
                for it in genre {
                    let tag_item = TagItem::new_checked(
                        tag.tag_type(),
                        ItemKey::Genre,
                        ItemValue::Text(it.genre.clone()),
                    )
                    .unwrap();

                    tag_items.push(tag_item);
                }
            }

            if let Some(yt_id) = &song.youtube_id {
                tag.remove_key(&ItemKey::Unknown("YTID".to_string()));
                let tag_item = TagItem::new(
                    ItemKey::Unknown("YTID".to_string()),
                    ItemValue::Text(yt_id.to_string()),
                );
                tag.insert_unchecked(tag_item);
            }

            if let Some(id) = &song.id {
                tag.remove_key(&ItemKey::Unknown("DBID".to_string()));
                let tag_item = TagItem::new(
                    ItemKey::Unknown("DBID".to_string()),
                    ItemValue::Text(id.to_string()),
                );
                tag.insert_unchecked(tag_item);
            }

            if let Some(picture_url) = &song.thumbnail_url {
                tag.remove_picture_type(lofty::PictureType::CoverFront);
                if picture_url.contains("http") {
                    let picture = reqwest::get(picture_url);
                    match picture.await {
                        Ok(request) => {
                            let mut pict: Vec<u8> = vec![];
                            let pic = image::load_from_memory(&request.bytes().await?)?;
                            pic.write_to(&mut Cursor::new(&mut pict), image::ImageFormat::Png)?;

                            let lofty_pic = Picture::new_unchecked(
                                lofty::PictureType::CoverFront,
                                lofty::MimeType::Png,
                                None,
                                pict,
                            );
                            tag.push_picture(lofty_pic);
                        }
                        Err(e) => {
                            error!("unable to get image: {}", e);
                        }
                    }
                }
            }

            for tag_item in tag_items {
                tag.push(tag_item);
            }

            tag.save_to_path(path)?;
            Ok(())
        }
        Err(e) => Err(eyre!(e)),
    }
}
/// Reads the tags from the given path into an `AppSong`
pub async fn read_tags_to_gui_song(path: PathBuf) -> Result<Song> {
    match Probe::open(path.clone())?.read() {
        Ok(mut tagged_file) => {
            let tag = match tagged_file.primary_tag_mut() {
                Some(primary_tag) => primary_tag,
                None => {
                    if let Some(first_tag) = tagged_file.first_tag_mut() {
                        first_tag
                    } else {
                        let tag_type = tagged_file.primary_tag_type();

                        println!("no tags found, creating new tags of type {:?}", tag_type);
                        tagged_file.insert_tag(Tag::new(tag_type));
                        tagged_file.primary_tag_mut().unwrap()
                    }
                }
            };

            let title = tag.title().as_deref().unwrap_or("Unknown").to_string();
            let artists = tag
                .get_strings(&ItemKey::TrackArtist)
                .map(|s| ArtistModel {
                    name: s.to_string(),
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            let albums = tag
                .get_strings(&ItemKey::AlbumTitle)
                .map(|a| AlbumModel {
                    name: a.to_string(),
                    ..Default::default()
                })
                .collect::<Vec<_>>();
            let genres = tag
                .get_strings(&ItemKey::Genre)
                .map(|s| GenreModel {
                    genre: s.to_string(),
                    ..Default::default()
                })
                .collect::<Vec<_>>();

            let mut song = Song::new()
                .set_path(path)
                .set_title(title)
                .set_artists(artists)
                .set_albums(albums)
                .set_genres(genres);
            if let Some(id) = tag.get_string(&ItemKey::Unknown("DBID".to_string())) {
                song.set_id(id.parse::<i32>().expect("no fail"));
            }
                // TODO: handle artists and albums and genres and custom tags
;

            Ok(song)
        }
        Err(e) => Err(eyre!(e)),
    }
}
