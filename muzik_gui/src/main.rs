use std::path::PathBuf;

use clap::Parser;
use iced::{window::icon, Application, Settings};
use miette::{IntoDiagnostic, Result};
use muzik_common::config::ReadConfig;
use tracing::info;
use tracing_subscriber::{filter, fmt, prelude::__tracing_subscriber_SubscriberExt, Layer};

use crate::log::StatusLayer;

mod gui;
mod log;

fn main() -> Result<()> {
    let _args = Cli::parse();

    let mut guards = vec![];

    let tmp = PathBuf::from("/tmp/muzik");
    let file_appender = tracing_appender::rolling::daily(tmp, "gui-log");
    let (tx, events_rx) = crossbeam_channel::unbounded();
    let status_layer = StatusLayer::new(tx);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    guards.push(guard);
    let subscriber = tracing_subscriber::registry()
        .with(
            fmt::layer().with_ansi(true).with_filter(
                filter::EnvFilter::builder()
                    .with_default_directive(filter::LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
        )
        .with(
            status_layer.with_filter(
                filter::EnvFilter::builder()
                    .with_default_directive(filter::LevelFilter::INFO.into())
                    .from_env_lossy(),
            ),
        )
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
        );
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;
    info!("logger started!");
    // let mut _guards = start_tui_log(PathBuf::from("/tmp"));

    // FIXME: this is an ugly hax pls fix
    let config = {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { ReadConfig::read_config(None).await.expect("config exists") })
    };
    // staticly load icon
    let icon = include_bytes!("./gui/icon.png");
    let icon = match icon::from_file_data(icon, None) {
        Ok(ok) => Some(ok),
        Err(_) => None,
    };
    gui::GuiMain::run(Settings {
        window: iced::window::Settings {
            size: (1280, 720),
            icon,
            decorations: true,
            ..Default::default()
        },
        flags: (config, Some(events_rx)),
        // default_font: Font::MONOSPACE,
        ..Default::default()
    })
    .into_diagnostic()
}

#[derive(Debug, Parser)]
#[command(name = "muzik-gui")]
struct Cli {
    config: Option<String>,
}
