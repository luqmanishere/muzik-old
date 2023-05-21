use std::{io::Cursor, path::PathBuf};

use eyre::{eyre, Result};
use lofty::{ItemKey, ItemValue, Picture, Probe, Tag, TagExt, TagItem, TaggedFileExt};
use tracing::error;

use crate::database::Song;

pub async fn write_tags_async(path: PathBuf, song: &Song) -> Result<()> {
    write_tags(path, song)
}

pub fn write_tags(path: PathBuf, song: &Song) -> Result<()> {
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
                        lofty::ItemValue::Text(artist.to_string()),
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
                        lofty::ItemValue::Text(album.to_string()),
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
                        ItemValue::Text(it.to_string()),
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

            if let Some(picture_url) = &song.tb_url {
                tag.remove_picture_type(lofty::PictureType::CoverFront);
                if picture_url.contains("http") {
                    let picture = reqwest::blocking::get(picture_url);
                    match picture {
                        Ok(request) => {
                            let mut pict: Vec<u8> = vec![];
                            let pic = image::load_from_memory(&request.bytes()?)?;
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
