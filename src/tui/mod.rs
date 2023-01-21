use std::{path::PathBuf, sync::mpsc::Sender};

use cursive::{
    view::Nameable,
    views::{OnEventView, Panel},
    Cursive, CursiveExt, View,
};
use cursive_tabs::TabPanel;
use directories_next::UserDirs;
use eyre::Result;
use tracing::error;

use crate::{
    config::Config,
    database::{Database, Song},
};

use self::event_runner::Event;

mod download;
mod editor;
mod event_runner;

pub fn run_tui() -> Result<()> {
    let mut siv = Cursive::new();
    let music_dir = UserDirs::new().unwrap().audio_dir().unwrap().to_path_buf();
    let db = match Database::new(music_dir.join("database.sqlite")) {
        Ok(db) => Some(db),
        Err(e) => {
            error!("Error connecting to database: {}", e);
            None
        }
    };
    let conf = Config {
        db: Database::new(music_dir.join("database.sqlite")).unwrap(),
        music_dir: music_dir.clone(),
    };
    let ev_man = event_runner::EventRunner::new(siv.cb_sink().clone(), conf);
    let tx = ev_man.get_tx();
    let tx_us = ev_man.get_tx();
    std::thread::spawn(move || loop {
        ev_man.process();
    });

    siv.set_user_data(State {
        db,
        music_dir,
        song_list: None,
        edit_state: EditState::None,
        song_index: None,
        tx: tx_us,
        current_selected_song: None,
    });
    siv.load_toml(include_str!("theme.toml")).unwrap();

    let mut panel = TabPanel::new();
    panel.set_bar_alignment(cursive_tabs::Align::Center);
    panel.add_tab(
        OnEventView::new(editor::draw_database_editor(&mut siv, tx.clone()))
            .on_event('u', editor::update_database)
            .on_event('d', editor::delete_from_database)
            .with_name("Editor"),
    );
    panel.add_tab(download::draw_download_tab(&mut siv, tx).with_name("Download"));
    panel.set_active_tab("Editor")?;
    let panel = Panel::new(panel.with_name("tab_panel")).title("muziktui");
    siv.add_layer(panel);

    siv.add_global_callback('~', Cursive::toggle_debug_console);
    siv.add_global_callback('q', |s| s.quit());
    siv.run();
    Ok(())
}

struct State {
    db: Option<Database>,
    music_dir: PathBuf,
    song_list: Option<Vec<Song>>,
    edit_state: EditState,
    song_index: Option<usize>,
    tx: Sender<Event>,
    current_selected_song: Option<Song>,
}

#[derive(Clone, Copy)]
enum EditState {
    Title,
    Artist,
    Album,
    None,
}