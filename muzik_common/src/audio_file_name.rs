use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::data::Song;

#[derive(Serialize, Deserialize, Clone)]
pub enum Enclose {
    CurlyBracket,
    Bracket,
    Ellipsis,
    None,
}

impl Enclose {
    pub fn enclose(&self, text: &str) -> String {
        match self {
            Enclose::CurlyBracket => format!("{{{}}}", text),
            Enclose::Bracket => format!("[{}]", text),
            Enclose::Ellipsis => format!("({})", text),
            Enclose::None => text.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum AudioFileNameComponents {
    Title(Enclose),
    Artist(Enclose),
    Album(Enclose),
    DatabaseId(Enclose),
    YoutubeId(Enclose),
    Custom(String),
}

impl Display for AudioFileNameComponents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            _ => {
                write!(f, "Unknown")
            }
        }
    }
}

pub fn get_audio_file_name_from_song(
    audio_file_name_components: &[AudioFileNameComponents],
    song: &Song,
    ext: Option<String>,
) -> String {
    let mut form = String::new();
    for components in audio_file_name_components {
        match components {
            AudioFileNameComponents::Title(enclose) => {
                form.push_str(&enclose.enclose(&song.get_title_string()))
            }
            AudioFileNameComponents::Artist(enclose) => {
                form.push_str(&enclose.enclose(&song.get_artists_string()))
            }
            AudioFileNameComponents::Album(enclose) => {
                form.push_str(&enclose.enclose(&song.get_albums_string()))
            }
            AudioFileNameComponents::DatabaseId(enclose) => {
                form.push_str(&enclose.enclose(&song.get_id_string()))
            }
            AudioFileNameComponents::YoutubeId(enclose) => {
                form.push_str(&enclose.enclose(&song.get_youtube_id_string()))
            }
            AudioFileNameComponents::Custom(custom) => form.push_str(custom),
        }
        form.push(' ');
    }
    if let Some(ext) = ext {
        // push a dot and the extension
        let mut form = form.trim().to_string();
        form.push('.');
        form.push_str(&ext);
        form.trim().to_string()
    } else {
        form.trim().to_string()
    }
}

pub fn get_audio_file_name_from_song_predownload(
    audio_file_name_components: &[AudioFileNameComponents],
    song: &Song,
    ext: Option<String>,
) -> String {
    let mut audio_file_name_components = audio_file_name_components.to_vec();
    audio_file_name_components.retain(|comp| match comp {
        AudioFileNameComponents::Title(_) => true,
        AudioFileNameComponents::Artist(_) => true,
        AudioFileNameComponents::Album(_) => true,
        AudioFileNameComponents::DatabaseId(_) => false,
        AudioFileNameComponents::YoutubeId(_) => true,
        AudioFileNameComponents::Custom(_) => true,
    });
    get_audio_file_name_from_song(&audio_file_name_components, song, ext)
}

pub fn get_audio_file_name_from_song_ytdl_template(
    audio_file_name_components: &[AudioFileNameComponents],
    song: &Song,
) -> String {
    get_audio_file_name_from_song(
        audio_file_name_components,
        song,
        Some("%(ext)s".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        audio_file_name::{get_audio_file_name_from_song_predownload, Enclose},
        data::Song,
        entities::{album::AlbumModel, artist::ArtistModel},
    };

    use super::{get_audio_file_name_from_song, AudioFileNameComponents};

    #[test]
    fn test_get_audio_file_name_from_song() {
        let song = Song::new()
            .set_id(1)
            .set_title("Crossing Field".to_string())
            .set_artists(vec![ArtistModel {
                name: "LiSA".to_string(),
                ..Default::default()
            }])
            .set_albums(vec![AlbumModel {
                name: "Sword Art Online".to_string(),
                ..Default::default()
            }])
            .set_youtube_id("kajflajefljeafe".to_string());
        let format_fn = get_audio_file_name_from_song(
            &vec![
                AudioFileNameComponents::Title(super::Enclose::None),
                AudioFileNameComponents::Custom("-".to_string()),
                AudioFileNameComponents::Artist(Enclose::None),
                AudioFileNameComponents::DatabaseId(Enclose::Bracket),
            ],
            &song,
            None,
        );
        let format_manual = format!(
            "{} - {} [{}]",
            song.get_title_string(),
            song.get_artists_string(),
            song.get_id_string()
        );
        assert_eq!(format_manual, format_fn, "format generated: {}", format_fn)
    }
    fn test_get_audio_file_name_from_song_predownload() {
        let song = Song::new()
            .set_id(1)
            .set_title("Crossing Field".to_string())
            .set_artists(vec![ArtistModel {
                name: "LiSA".to_string(),
                ..Default::default()
            }])
            .set_albums(vec![AlbumModel {
                name: "Sword Art Online".to_string(),
                ..Default::default()
            }])
            .set_youtube_id("kajflajefljeafe".to_string());
        let format_fn = get_audio_file_name_from_song_predownload(
            &vec![
                AudioFileNameComponents::Title(super::Enclose::None),
                AudioFileNameComponents::Custom("-".to_string()),
                AudioFileNameComponents::Artist(Enclose::None),
                AudioFileNameComponents::DatabaseId(Enclose::Bracket),
            ],
            &song,
            None,
        );
        let format_manual = format!(
            "{} - {}",
            song.get_title_string(),
            song.get_artists_string(),
        );
        assert_eq!(format_manual, format_fn, "format generated: {}", format_fn)
    }

    #[test]
    fn test_get_audio_file_name_from_song_with_ext() {
        let song = Song::new()
            .set_id(1)
            .set_title("Crossing Field".to_string())
            .set_artists(vec![ArtistModel {
                name: "LiSA".to_string(),
                ..Default::default()
            }])
            .set_albums(vec![AlbumModel {
                name: "Sword Art Online".to_string(),
                ..Default::default()
            }])
            .set_youtube_id("kajflajefljeafe".to_string());
        let format_fn = get_audio_file_name_from_song(
            &vec![
                AudioFileNameComponents::Title(super::Enclose::None),
                AudioFileNameComponents::Custom("-".to_string()),
                AudioFileNameComponents::Artist(Enclose::None),
                AudioFileNameComponents::DatabaseId(Enclose::Bracket),
            ],
            &song,
            Some("opus".to_string()),
        );
        let format_manual = format!(
            "{} - {} [{}].opus",
            song.get_title_string(),
            song.get_artists_string(),
            song.get_id_string()
        );
        assert_eq!(format_manual, format_fn, "format generated: {}", format_fn)
    }
}
