use std::{path::PathBuf, sync::mpsc::Sender};

use cursive::{
    view::{Nameable, Resizable},
    views::{Dialog, OnEventView, Panel},
    Cursive, CursiveExt,
};
use cursive_tabs::TabPanel;
use directories::UserDirs;
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
    // TODO: only use 1 config
    let music_dir = if let Ok(_termux_ver) = std::env::var("TERMUX_VERSION") {
        PathBuf::from(std::env::var("HOME").unwrap()).join("storage/music")
    } else {
        UserDirs::new().unwrap().audio_dir().unwrap().to_path_buf()
    };
    let db = match Database::new(music_dir.join("database.sqlite")) {
        Ok(db) => Some(db),
        Err(e) => {
            error!("Error connecting to database: {}", e);
            None
        }
    };
    let conf = Config::default();
    let mut ev_man = event_runner::EventRunner::new(siv.cb_sink().clone(), conf);
    let tx = ev_man.get_tx();
    let tx_us = ev_man.get_tx();
    std::thread::spawn(move || loop {
        match ev_man.process() {
            Ok(_) => {}
            Err(e) => {
                error!("Error occurs in event loop: {}", e);
                ev_man
                    .cb_sink
                    .send(Box::new(move |siv: &mut Cursive| {
                        let text = format!("Error occured in event loop: {}", e);
                        let dialog = Dialog::text(text).dismiss_button("Close");
                        siv.add_layer(dialog);
                    }))
                    .unwrap();
            }
        };
    });

    siv.load_toml(include_str!("theme.toml")).unwrap();

    let update_tx = tx.clone();
    let delete_tx = tx.clone();
    let verify_tx = tx.clone();
    let missing_tx = tx.clone();

    let mut tab_panel = TabPanel::new();
    tab_panel.set_bar_alignment(cursive_tabs::Align::Center);
    tab_panel.add_tab(
        OnEventView::new(editor::draw_database_editor(tx.clone()))
            .on_event('u', move |_| {
                update_tx.send(Event::UpdateLocalDatabase).unwrap()
            })
            .on_event('d', move |_| delete_tx.send(Event::OnDeleteKey).unwrap())
            .on_event('V', move |_| {
                verify_tx.send(Event::VerifyAllSongIntegrity()).unwrap()
            })
            .on_event('R', move |_| {
                missing_tx
                    .send(Event::DownloadAllMissingFromDatabase)
                    .unwrap()
            })
            .with_name("Editor"),
    );
    tab_panel.add_tab(download::draw_download_tab(&mut siv, tx).with_name("Download"));
    tab_panel.set_active_tab("Editor")?;
    let panel = Panel::new(
        OnEventView::new(tab_panel.with_name("tab_panel"))
            .on_event('1', |siv: &mut Cursive| {
                siv.call_on_name("tab_panel", |v: &mut TabPanel| {
                    v.set_active_tab("Editor").unwrap()
                })
                .unwrap();
            })
            .on_event('2', |siv: &mut Cursive| {
                siv.call_on_name("tab_panel", |v: &mut TabPanel| {
                    v.set_active_tab("Download").unwrap()
                })
                .unwrap();
            })
            .on_event(cursive::event::Key::Tab, |siv: &mut Cursive| {
                siv.call_on_name("tab_panel", |v: &mut TabPanel| {
                    v.next();
                })
                .unwrap();
            }),
    )
    .title("muziktui");
    siv.add_fullscreen_layer(panel.full_screen());

    siv.add_global_callback('~', Cursive::toggle_debug_console);
    siv.add_global_callback('q', |s| s.quit());
    siv.run();
    Ok(())
}
