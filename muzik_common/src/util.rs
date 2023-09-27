pub use youtube_dl;

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
            .cookies(cookie.display().to_string())
            .run()
    } else {
        YoutubeDl::new(kw).run()
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

pub async fn search_youtube_async(
    kw: String,
    cookies: Option<PathBuf>,
    entries: Option<usize>,
) -> Result<Vec<SingleVideo>, YoutubeError> {
    let count = match entries {
        Some(count) => count,
        None => 15,
    };
    let yt_search = if !kw.contains("http") {
        let search_options = SearchOptions::youtube(kw).with_count(count);
        if let Some(cookie) = cookies {
            YoutubeDl::search_for(&search_options)
                .cookies(cookie.display().to_string())
                .run_async()
                .await
        } else {
            YoutubeDl::search_for(&search_options).run_async().await
        }
    } else if let Some(cookie) = cookies {
        YoutubeDl::new(kw)
            .cookies(cookie.display().to_string())
            .run_async()
            .await
    } else {
        YoutubeDl::new(kw).run_async().await
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
                .cookies(cookie.display().to_string())
                .run()
        } else {
            YoutubeDl::new(link).run()
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
) -> Result<YoutubeDlOutput, YoutubeError> {
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
            .extract_audio(true)
            .run()
            .map_err(|e| YoutubeError::YoutubeDl(e))
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
            .extract_audio(true)
            .run()
            .map_err(|e| YoutubeError::YoutubeDl(e))
    }
}

pub async fn load_image(url: Option<String>) -> Result<Vec<u8>, YoutubeError> {
    if let Some(url) = url {
        if url.contains("http") {
            let picture = reqwest::get(url);
            match picture.await {
                Ok(request) => {
                    let pic: Vec<_> = request.bytes().await?.to_vec();
                    Ok(pic)
                }
                Err(e) => Err(YoutubeError::ReqwestError(e)),
            }
        } else {
            Ok(vec![])
        }
    } else {
        Ok(vec![])
    }
}

pub async fn download_video(
    id: String,
    music_dir: PathBuf,
    filename_format: String,
    cookies: Option<PathBuf>,
) -> Result<(), YoutubeError> {
    if let Some(cookies) = cookies {
        YoutubeDl::new(id)
            .extract_audio(true)
            .extra_arg("--audio-format")
            .extra_arg("opus")
            .format("251")
            .extra_arg("--sponsorblock-remove")
            .extra_arg("all")
            .cookies(cookies.display().to_string())
            .output_template(filename_format)
            .download_to_async(music_dir)
            .await?
    } else {
        YoutubeDl::new(id)
            .extract_audio(true)
            .extra_arg("--audio-format")
            .extra_arg("opus")
            .format("251")
            .extra_arg("--sponsorblock-remove")
            .extra_arg("all")
            .output_template(filename_format)
            .download_to_async(music_dir)
            .await?
    }
    Ok(())
}

/// attaches the variable extension to the filename
pub fn format_add_extension(filename: String) -> String {
    let filename_format = format!("{}.%(ext)s", filename);
    filename_format
}

pub mod error {
    use miette::Diagnostic;
    use thiserror::Error;

    #[derive(Error, Diagnostic, Debug)]
    pub enum YoutubeError {
        #[error(transparent)]
        YoutubeDl(#[from] youtube_dl::Error),
        #[error(transparent)]
        ImageError(#[from] image::ImageError),

        #[error(transparent)]
        ReqwestError(#[from] reqwest::Error),
    }
}
