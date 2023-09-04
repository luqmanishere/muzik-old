use youtube_dl::{SearchOptions, SingleVideo, YoutubeDl, YoutubeDlOutput};

use std::path::PathBuf;

use self::error::YoutubeError;

pub fn search_youtube(
    kw: String,
    cookies: Option<PathBuf>,
) -> Result<Vec<SingleVideo>, YoutubeError> {
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
        Err(err) => Err(YoutubeError::YoutubeDl(err)),
    }
}

pub fn search_youtube_playlist(
    playlist_id: String,
    cookies: Option<PathBuf>,
) -> Result<Vec<SingleVideo>, YoutubeError> {
    let yt_search = {
        let link = format!("https://www.youtube.com/playlist?list={}", playlist_id);
        if let Some(cookie) = cookies {
            YoutubeDl::new(link)
                .download(false)
                .cookies(cookie.display().to_string())
                .run()
        } else {
            YoutubeDl::new(link).download(false).run()
        }
    };

    match yt_search {
        Ok(output) => match output {
            youtube_dl::YoutubeDlOutput::Playlist(playlist) => {
                let entries = playlist.entries.unwrap_or_default();
                Ok(entries)
            }
            youtube_dl::YoutubeDlOutput::SingleVideo(video) => Ok(vec![*video]),
        },
        Err(err) => Err(YoutubeError::YoutubeDl(err)),
    }
}

pub fn download_from_youtube(
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

pub mod error {
    use miette::Diagnostic;
    use thiserror::Error;

    #[derive(Error, Diagnostic, Debug)]
    pub enum YoutubeError {
        #[error(transparent)]
        YoutubeDl(#[from] youtube_dl::Error),
    }
}
