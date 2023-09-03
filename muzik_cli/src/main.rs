use std::path::PathBuf;

use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use eyre::{eyre, Result};
use tracing::{debug, error, info};
use tracing_subscriber::{
    filter, fmt, prelude::__tracing_subscriber_SubscriberExt, EnvFilter, Layer,
};
use youtube_dl::{SearchOptions, YoutubeDl};

use muzik_common::{
    database::{self, AppSong},
    tags,
};

use crate::config::ReadConfig;

mod config;

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
    DbTest,
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
                // TODO: switch to new backend
                download_command(query).await.unwrap();
            }
            // TODO: switch to new backend
            Commands::List => list_command().await.unwrap(),
            // TODO: switch to new backend
            Commands::Delete => delete_command().await.unwrap(),
            Commands::DbTest => {
                // construct a subscriber that prints formatted traces to stdout
                let _subscriber = tracing_subscriber::registry().with(
                    fmt::Layer::new()
                        .with_timer(tracing_subscriber::fmt::time::time())
                        .with_filter(
                            tracing_subscriber::filter::EnvFilter::builder()
                                .with_default_directive(filter::LevelFilter::TRACE.into())
                                .from_env_lossy(),
                        ),
                );
                tracing_subscriber::fmt()
                    .with_env_filter(EnvFilter::from_default_env())
                    .with_test_writer()
                    .init();
                // use that subscriber to process traces emitted after this point
                // tracing::subscriber::set_global_default(subscriber)?;
                // let test = database::DbConnection::new("./test.db".into()).await?;
                database::DbConnection::in_memory_test().await?;
            }
        }
    } else {
        // When launched without a subcommand
        //MuzikGui::run(Settings::default())?;
        info!("ran without a subcommand");
    };

    // Return gracefully
    Ok(())
}

async fn download_command(query: Vec<String>) -> Result<()> {
    let config = ReadConfig::read_config(None).await?;
    debug!("music dir is : {}", config.music_dir.display());
    let name: String = query.join(" ");

    // TODO: cleanup duplicate code
    println!("searching for: {}", name);
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

                    let _artists_present = config.db_new.get_all_artists().await?;
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

                    let song = AppSong::new()
                        .with_music_dir(Some(config.get_music_dir()))
                        .with_title(Some(title))
                        .with_albums(album)
                        .with_artists_string(artist)
                        .with_genre(video.genre.unwrap_or_else(|| "Unknown".to_string()))
                        .with_yt_id(Some(id))
                        .with_tb_url(video.thumbnail)
                        .compute_new_filename();

                    match tags::write_tags_async(filename.clone(), &song).await {
                        Ok(_) => {
                            config.db_new.insert_from_app_song(song).await?;
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
    let _config = ReadConfig::read_config(None).await?;
    // TODO: implement new db
    // let e = db.get_all("/home/luqman/Music".into())?;
    //
    // if e.is_empty() {
    //     println!("no songs in database");
    // } else {
    //     for s in e {
    //         println!(
    //             "{} - {}",
    //             s.title.clone().unwrap_or_else(|| "Unknown".to_string()),
    //             s.get_artists_string()
    //         );
    //     }
    // }
    Ok(())
}

async fn delete_command() -> Result<()> {
    // let db = Database::new("/home/luqman/Music/database.sqlite".into())?;
    // TODO: implement new db
    // let music_dir = PathBuf::from("/home/luqman/Music");
    //
    // if let Ok(songs) = db.get_all(music_dir.clone()) {
    //     let lel = songs
    //         .iter()
    //         .map(|s| format!("{} [{}]", s.title.clone().unwrap(), s.id.unwrap()))
    //         .collect::<Vec<_>>();
    //     let selection =
    //         dialoguer::FuzzySelect::with_theme(&dialoguer::theme::ColorfulTheme::default())
    //             .items(&lel)
    //             .default(0)
    //             .interact_opt()?;
    //
    //     match selection {
    //         Some(sel) => {
    //             let song = songs.get(sel).unwrap();
    //             if let Some(song_id) = song.id {
    //                 db.delete_entry_by_id(song_id)?;
    //                 println!("deleted: {}", lel[sel]);
    //                 if Confirm::new()
    //                     .with_prompt("want to delete file?")
    //                     .interact()?
    //                 {
    //                     let fpath = music_dir.join(song.path.clone().unwrap());
    //                     std::fs::remove_file(fpath)?;
    //                     println!("file removed");
    //                 }
    //             } else {
    //                 println!("no got id");
    //             }
    //         }
    //         None => todo!(),
    //     }
    // };

    Ok(())
}
