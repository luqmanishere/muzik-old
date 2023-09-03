use clap::Parser;
use iced::{Application, Settings};
use miette::{IntoDiagnostic, Result};
use tracing::info;
use tracing_subscriber::{filter, fmt, prelude::__tracing_subscriber_SubscriberExt, Layer};

use crate::config::ReadConfig;

mod config;
mod editor;
mod gui;
mod hoverable;
mod multi_input;
mod theme;

fn main() -> Result<()> {
    let _args = Cli::parse();

    let subscriber = tracing_subscriber::registry().with(
        fmt::layer().with_ansi(true).with_filter(
            filter::EnvFilter::builder()
                .with_default_directive(filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        ),
    );
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
    info!("logger started!");
    // let mut _guards = start_tui_log(PathBuf::from("/tmp"));

    let config = {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { ReadConfig::read_config(None).await.expect("config exists") })
    };
    gui::GuiMain::run(Settings {
        window: iced::window::Settings {
            size: (1280, 720),
            ..Default::default()
        },
        flags: config,
        ..Default::default()
    })
    .into_diagnostic()
}

#[derive(Debug, Parser)]
#[command(name = "muzik-gui")]
struct Cli {
    config: Option<String>,
}
