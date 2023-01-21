use std::path::PathBuf;

use clap::{Parser, Subcommand};
use database::Database;
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input};
use eyre::{eyre, Result};
use tracing::{debug, error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{filter, fmt, prelude::__tracing_subscriber_SubscriberExt, Layer};
use youtube_dl::{SearchOptions, YoutubeDl};

mod config;
mod database;
mod tags;
mod tui;

use crate::{config::Config, database::Song};

#[derive(Debug, Parser)]
#[command(name = "muzik")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Download {
        #[arg(num_args = .., trailing_var_arg = true)]
        query: Vec<String>,
    },
    List,
    Delete,
    Tui,
}

#[tokio::main]
async fn main() -> Result<()> {
    info!("log started");
    let args = Cli::parse();

    if let Some(command) = args.command {
        match command {
            Commands::Download { query } => {
                // construct a subscriber that prints formatted traces to stdout
                let subscriber = tracing_subscriber::FmtSubscriber::new();
                // use that subscriber to process traces emitted after this point
                tracing::subscriber::set_global_default(subscriber)?;
                download_command(query).await.unwrap();
            }
            Commands::List => list_command().await.unwrap(),
            Commands::Delete => delete_command().await.unwrap(),
            Commands::Tui => {
                let mut _guards = start_tui_log();
                tui_command().await.unwrap()
            }
        }
    } else {
        // When launched without a subcommand
        //MuzikGui::run(Settings::default())?;
    };

    // Return gracefully
    Ok(())
}

async fn tui_command() -> Result<()> {
    match tui::run_tui() {
        Ok(_) => (),
        Err(e) => {
            error!("Fatal error: {}", e);
        }
    }
    Ok(())
}

async fn download_command(query: Vec<String>) -> Result<()> {
    let music_dir = directories_next::UserDirs::new()
        .unwrap()
        .audio_dir()
        .unwrap()
        .to_path_buf();
    debug!("music dir is : {}", music_dir.display());
    let db = Database::new(music_dir.join("database.sqlite"))?;
    let config = Config { music_dir, db };
    let name: String = query.join(" ");

    println!("search: {}", name);
    let search_options = SearchOptions::youtube(name).with_count(5);
    let search = YoutubeDl::search_for(&search_options)
        .youtube_dl_path("yt-dlp")
        .run_async()
        .await
        .unwrap();

    match search {
        youtube_dl::YoutubeDlOutput::Playlist(playlist) => {
            let entries = playlist.entries.unwrap();
            let items = entries
                .iter()
                .map(|entry| entry.title.to_string())
                .collect::<Vec<String>>();
            let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
                .items(&items)
                .default(0)
                .interact();

            match selection {
                Ok(index) => {
                    println!("User selected: [{}] : {}", index, items[index]);

                    let video = entries[index].clone();
                    let id = video.id;

                    println!("Enter details");

                    // TODO: Implement completion based on existing database entries
                    let title: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Track Title")
                        .default(video.title.clone())
                        .interact()?;

                    let artist: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Track Artist")
                        .default(video.channel.clone().unwrap())
                        .interact()?;

                    let album: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Track Album")
                        .default({
                            match video.album {
                                Some(album) => album,
                                None => video.channel.clone().unwrap(),
                            }
                        })
                        .interact()?;
                    /*
                    let genre = match video.genre {
                        Some(genre) => genre,
                        None => "Unknown".to_string(),
                    };
                    */
                    let filename_format = format!("{} - {}.%(ext)s", title.clone(), artist.clone());
                    let filename = format!("{} - {}.opus", title.clone(), artist.clone());
                    let filename = config.get_music_dir().join(filename);
                    let _youtube = YoutubeDl::new(id.clone())
                        .youtube_dl_path("yt-dlp")
                        .extra_arg("--audio-format")
                        .extra_arg("opus")
                        .extra_arg("--downloader")
                        .extra_arg("aria2c")
                        .extra_arg("--sponsorblock-remove")
                        .extra_arg("all")
                        .extra_arg("-P")
                        .extra_arg(config.get_music_dir().to_str().unwrap())
                        .extra_arg("-o")
                        .extra_arg(filename_format)
                        .download(true)
                        .extract_audio(true)
                        .run_async()
                        .await
                        .unwrap();

                    println!("Expected filename: {}", filename.display());
                    if filename.exists() {
                        println!("Download finished!")
                    } else {
                        println!("File not found after downloading");
                    }

                    let song = Song::new(
                        config.get_music_dir(),
                        None,
                        Some(filename.display().to_string()),
                        Some(title),
                        Some(album),
                        Some(artist),
                        Some(id),
                        video.thumbnail,
                    );

                    match tags::write_tags_async(filename.clone(), &song).await {
                        Ok(_) => {
                            config.db.insert_entry(&song)?;
                            info!("database updated")
                        }
                        Err(e) => {
                            error!("error writing tags: {}", e);
                            std::fs::rename(
                                filename.clone(),
                                PathBuf::from("/tmp").join(filename.file_name().unwrap()),
                            )?;
                            println!("moved file to temp for inspection");
                        }
                    };

                    Ok(())
                }
                Err(_) => Err(eyre!("User canceled selection")),
            }
        }
        youtube_dl::YoutubeDlOutput::SingleVideo(_) => {
            // TODO: not a major todo, because multi search always return multiple
            Ok(())
        }
    }
}

async fn list_command() -> Result<()> {
    let db = Database::new("/home/luqman/Music/database.sqlite".into())?;
    let e = db.get_all("/home/luqman/Music".into())?;

    if e.is_empty() {
        println!("no songs in database");
    } else {
        for s in e {
            println!(
                "{} - {}",
                s.title.clone().unwrap_or_else(|| "Unknown".to_string()),
                s.get_artists_string()
            );
        }
    }
    Ok(())
}

async fn delete_command() -> Result<()> {
    let mut db = Database::new("/home/luqman/Music/database.sqlite".into())?;
    let music_dir = PathBuf::from("/home/luqman/Music");

    if let Ok(songs) = db.get_all(music_dir.clone()) {
        let lel = songs
            .iter()
            .map(|s| format!("{} [{}]", s.title.clone().unwrap(), s.id.unwrap()))
            .collect::<Vec<_>>();
        let selection =
            dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .items(&lel)
                .default(0)
                .interact_opt()?;

        match selection {
            Some(sel) => {
                let song = songs.get(sel).unwrap();
                if let Some(song_id) = song.id {
                    db.delete_entry_by_id(song_id)?;
                    println!("deleted: {}", lel[sel]);
                    if Confirm::new()
                        .with_prompt("want to delete file?")
                        .interact()?
                    {
                        let fpath = music_dir.join(song.path.clone().unwrap());
                        std::fs::remove_file(fpath)?;
                        println!("file removed");
                    }
                } else {
                    println!("no got id");
                }
            }
            None => todo!(),
        }
    };

    Ok(())
}

fn start_tui_log() -> Vec<WorkerGuard> {
    let mut guards = vec![];

    let file_appender = tracing_appender::rolling::daily("/tmp/muzik", "log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    guards.push(guard);

    let subs = tracing_subscriber::registry()
        .with(
            fmt::Layer::new()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_timer(tracing_subscriber::fmt::time::time())
                .with_filter(
                    tracing_subscriber::filter::EnvFilter::builder()
                        .with_default_directive(filter::LevelFilter::INFO.into())
                        .from_env_lossy(),
                ),
        )
        .with(
            fmt::layer()
                .with_ansi(true)
                .with_timer(tracing_subscriber::fmt::time::time())
                .with_filter(filter::LevelFilter::INFO),
        );
    tracing::subscriber::set_global_default(subs).expect("setting default subscriber failed");
    info!("logger started!");
    guards
}
