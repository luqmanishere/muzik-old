use std::path::PathBuf;

use miette::Result;
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{filter, fmt, prelude::__tracing_subscriber_SubscriberExt, Layer};

mod download;
mod editor;
mod event_runner;
mod metadata;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    let mut _guards = start_tui_logger(PathBuf::from("/tmp"));
    tui_command().await
}
async fn tui_command() -> Result<()> {
    match tui::run_tui().await {
        Ok(_) => (),
        Err(e) => {
            error!("Fatal error: {}", e);
        }
    }
    Ok(())
}

fn start_tui_logger(tmp: PathBuf) -> Vec<WorkerGuard> {
    let mut guards = vec![];

    let tmp = tmp.join("muzik");
    let file_appender = tracing_appender::rolling::daily(tmp, "log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    guards.push(guard);

    // Only write logs to log file
    let subs = tracing_subscriber::registry().with(
        fmt::Layer::new()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_timer(tracing_subscriber::fmt::time::time())
            .with_filter(
                tracing_subscriber::filter::EnvFilter::builder()
                    .with_default_directive(filter::LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
    );
    tracing::subscriber::set_global_default(subs).expect("setting default subscriber failed");
    info!("logger started!");
    guards
}
