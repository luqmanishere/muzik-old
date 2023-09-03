use cursive::{
    view::{Nameable, Resizable},
    views::{Dialog, OnEventView, Panel},
    Cursive, CursiveExt,
};
use cursive_tabs::TabPanel;
use eyre::Result;
use tracing::error;

use crate::{config::ReadConfig, download, editor, event_runner};

use crate::event_runner::Event;

pub async fn run_tui() -> Result<()> {
    let mut siv = Cursive::new();
    let conf = ReadConfig::read_config(None).await?;
    let mut ev_man = event_runner::EventRunner::new(siv.cb_sink().clone(), conf).await;
    let tx = ev_man.get_tx();
    let ev_loop = tokio::spawn(async move {
        loop {
            match ev_man.process().await {
                Ok(action) => match action {
                    event_runner::EventLoopAction::Continue => {}
                    event_runner::EventLoopAction::Quit => break,
                },
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
        }
    });

    siv.load_toml(include_str!("theme.toml")).unwrap();

    let update_tx = tx.clone();
    let delete_tx = tx.clone();
    let verify_tx = tx.clone();
    let missing_tx = tx.clone();
    let sync_tx = tx.clone();

    let tab_panel_tx = tx.clone();
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
            .on_event('S', move |_| sync_tx.send(Event::SyncWithYoutube).unwrap())
            .with_name("Editor"),
    );
    tab_panel.add_tab(download::draw_download_tab(&mut siv, tab_panel_tx).with_name("Download"));
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

    let quit_tx = tx;
    siv.add_global_callback('~', Cursive::toggle_debug_console);
    siv.add_global_callback('q', move |s| {
        s.quit();
        quit_tx.send(Event::QuitEventLoop).unwrap();
    });
    siv.run();
    ev_loop.abort();
    Ok(())
}
