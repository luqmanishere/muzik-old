use crossbeam_channel::Sender;
use cursive::{
    event::{Callback, EventResult, Key},
    view::{Nameable, Resizable, Scrollable},
    views::{
        Dialog, DummyView, EditView, FocusTracker, LinearLayout, OnEventView, Panel, SelectView,
        TextView,
    },
    Cursive,
};
use muzik_common::{
    database::AppSong,
    entities::{album, artist, genre},
};
use tracing::debug;

use super::event_runner::Event;

pub fn draw_database_editor(tx: Sender<Event>) -> LinearLayout {
    // Default before data is loaded in
    let select_song = song_selector_view(tx.clone());

    // Metadata SelectView entries are dynamically updated
    let select_metadata = metadata_selector_view(tx);

    let hlayout = LinearLayout::horizontal()
        .child(select_song)
        .child(select_metadata)
        .scrollable()
        .scroll_x(true)
        .show_scrollbars(false);

    LinearLayout::vertical()
        .child(TextView::new("Database Editor").h_align(cursive::align::HAlign::Center))
        .child(hlayout)
        .child(
            TextView::new("d - Delete | u - Update list | V - verify all | R - download all missing | S - yt sync")
                .h_align(cursive::align::HAlign::Center)
                .with_name("help"),
        )
        .child(
            Panel::new(TextView::new("Standby").with_name("statusbar"))
                .title("Status Bar")
                .title_position(cursive::align::HAlign::Left),
        )
}

fn song_selector_view(tx: Sender<Event>) -> impl cursive::View {
    let list = vec![AppSong::default()];
    let select_song_list = list.iter().enumerate().map(|(ind, f)| {
        (
            format!("{} - {}", f.get_title_string(), f.get_artists_string()),
            ind,
        )
    });
    let mut select_song = SelectView::new();
    select_song.add_all(select_song_list.into_iter());
    tx.send(Event::UpdateEditorSongSelectView).unwrap();

    let ttx1 = tx.clone();
    let ttx2 = tx;
    let select_song = select_song
        .on_submit(move |_, index| {
            ttx1.send(Event::UpdateEditorMetadataSelectView(*index))
                .unwrap();
        })
        .on_select(move |_, index| {
            ttx2.send(Event::UpdateEditorMetadataSelectView(*index))
                .unwrap()
        });
    let select_song = select_song
        .with_name("select_song")
        .scrollable()
        .min_width(20)
        .full_width()
        .full_height();
    let select_song = Panel::new(select_song).title("Songs");

    FocusTracker::new(select_song).on_focus(|_view| {
        EventResult::Consumed(Some(Callback::from_fn_mut(|siv: &mut Cursive| {
            siv.call_on_name("help", |view: &mut TextView| 
                view.set_content("d - Delete | u - Update list | V - verify all | R - download all missing | S - yt sync" ));
        })))
    })
}

fn metadata_selector_view(tx: Sender<Event>) -> impl cursive::View {
    let artist_add_tx = tx.clone();
    let artist_edit_tx = tx.clone();
    let album_edit_tx = tx.clone();
    let album_add_tx = tx;
    let title = TextView::new("Unknown").with_name("metadata_title");

    let artist_select = OnEventView::new(
        FocusTracker::new(
            SelectView::new()
                .item("Empty".to_string(), artist::Model::default())
                .with_name("metadata_artist_select_view"),
        )
        .on_focus(|_| {
            EventResult::Consumed(Some(Callback::from_fn_mut(|siv: &mut Cursive| {
                siv.call_on_name("help", |view: &mut TextView| {
                    view.set_content("a - add artist | e - edit artist | Enter - view info")
                });
            })))
        }),
    )
    .on_event(Key::Enter, |s| {
        on_artist_show_event(s);
    })
    // add artist event
    .on_event('a', move |s| {
        on_artist_add_command(s, artist_add_tx.clone());
    })
    // edit artist event
    .on_event('e', move |s| {
        on_artist_edit_command(s, artist_edit_tx.clone());
    });
    // TODO: delete entry

    let album_select = OnEventView::new(
        FocusTracker::new(
            SelectView::new()
                .item(
                    "Empty".to_string(),
                    album::Model {
                        id: 0,
                        name: "Empty".to_string(),
                    },
                )
                .with_name("metadata_album_select_view"),
        )
        .on_focus(|_| {
            EventResult::Consumed(Some(Callback::from_fn_mut(|siv: &mut Cursive| {
                siv.call_on_name("help", |view: &mut TextView| {
                    view.set_content("a - add album | e - edit album | Enter - view info")
                });
            })))
        }),
    )
    .on_event(Key::Enter, |s| {
        on_album_show_command(s);
    })
    .on_event('e', move |s| {
        on_album_edit_command(s, album_edit_tx.clone());
    })
    .on_event('a', move |s| {
        on_album_add_command(s, album_add_tx.clone());
    });

    let genre_select = SelectView::new()
        .item(
            "Empty".to_string(),
            genre::Model {
                id: 0,
                genre: "Empty".to_string(),
            },
        )
        .with_name("metadata_genre_select_view");

    // layouts
    let artist_layout = LinearLayout::vertical()
        .child(artist_select)
        .with_name("metadata_artist_layout")
        .full_width();
    let album_layout = LinearLayout::vertical()
        .child(album_select)
        .with_name("metadata_album_layout")
        .full_width();

    Panel::new(
        LinearLayout::vertical()
            .child(DummyView.full_width().full_height())
            .child(Panel::new(title).title("Title"))
            .child(
                LinearLayout::horizontal()
                    .child(Panel::new(artist_layout).title("Artists"))
                    .child(Panel::new(album_layout).title("Albums"))
                    .full_width(),
            )
            .child(Panel::new(genre_select).title("Genres"))
            .child(DummyView.full_width().full_height()),
    )
    .title("Metadata")
    .full_width()
}

fn on_artist_show_event(s: &mut Cursive) {
    let selection = s
        .call_on_name(
            "metadata_artist_select_view",
            |view: &mut SelectView<artist::Model>| view.selection(),
        )
        .unwrap()
        .unwrap();
    let dia = Dialog::around(
        LinearLayout::vertical()
            .child(TextView::new(format!("id: {}", selection.id)))
            .child(TextView::new(format!("name: {}", selection.name))),
    )
    .title("Artist Info")
    .dismiss_button("Dismiss");
    s.add_layer(dia);
}

fn on_artist_add_command(s: &mut Cursive, tx: Sender<Event>) {
    let dialog = Dialog::around(
        LinearLayout::vertical().child(
            LinearLayout::horizontal()
                .child(TextView::new("new artist name: "))
                .child(
                    EditView::new()
                        .with_name("artist_name_edit_view")
                        .min_width(10),
                ),
        ), // TODO: implement some kind of autocompletion
    )
    .dismiss_button("Dismiss")
    .button("Ok", move |s| {
        let new = s
            .call_on_name("artist_name_edit_view", |view: &mut EditView| {
                view.get_content()
            })
            .unwrap();

        debug!("new: {}", new);
        tx.send(Event::MetadataEditorAddArtist(new.to_string()))
            .unwrap();
        s.pop_layer();
    })
    .title("Edit Artist");
    s.add_layer(dialog);
}

fn on_artist_edit_command(s: &mut Cursive, tx: Sender<Event>) {
    let selection = s
        .call_on_name(
            "metadata_artist_select_view",
            |view: &mut SelectView<artist::Model>| view.selection().unwrap(),
        )
        .unwrap();
    let dialog = Dialog::around(
        LinearLayout::vertical()
            .child(TextView::new(format!(
                "old artist name: {}",
                selection.name
            )))
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("new artist name: "))
                    .child(
                        EditView::new()
                            .with_name("artist_name_edit_view")
                            .min_width(10),
                    ),
            ), // TODO: implement some kind of autocompletion
    )
    .dismiss_button("Dismiss")
    .button("Ok", move |s| {
        let selection = s
            .call_on_name(
                "metadata_artist_select_view",
                |view: &mut SelectView<artist::Model>| view.selection(),
            )
            .unwrap()
            .unwrap();
        debug!("selection: {}", selection.name);

        let new = s
            .call_on_name("artist_name_edit_view", |view: &mut EditView| {
                view.get_content()
            })
            .unwrap();

        debug!("new: {}", new);
        tx.send(Event::MetadataEditorEditArtist((
            selection.name.to_string(),
            new.to_string(),
        )))
        .unwrap();
        s.pop_layer();
    })
    .title("Edit Artist");
    s.add_layer(dialog);
}

fn on_album_show_command(s: &mut Cursive) {
    let selection = s
        .call_on_name(
            "metadata_album_select_view",
            |view: &mut SelectView<album::Model>| view.selection(),
        )
        .unwrap()
        .unwrap();
    let dia = Dialog::around(
        LinearLayout::vertical()
            .child(TextView::new(format!("id: {}", selection.id)))
            .child(TextView::new(format!("name: {}", selection.name))),
    )
    .title("Album Info")
    .dismiss_button("Dismiss");
    s.add_layer(dia);
}

fn on_album_edit_command(s: &mut Cursive, tx: Sender<Event>) {
    let selection = s
        .call_on_name(
            "metadata_album_select_view",
            |view: &mut SelectView<album::Model>| view.selection().unwrap(),
        )
        .unwrap();
    let dialog = Dialog::around(
        LinearLayout::vertical()
            .child(TextView::new(format!("old album name: {}", selection.name)))
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("new album name: "))
                    .child(
                        EditView::new()
                            .with_name("album_name_edit_view")
                            .min_width(10),
                    ),
            ), // TODO: implement some kind of autocompletion
    )
    .dismiss_button("Dismiss")
    .button("Ok", move |s| {
        let selection = s
            .call_on_name(
                "metadata_album_select_view",
                |view: &mut SelectView<album::Model>| view.selection(),
            )
            .unwrap()
            .unwrap();
        debug!("selection: {}", selection.name);

        let new = s
            .call_on_name("album_name_edit_view", |view: &mut EditView| {
                view.get_content()
            })
            .unwrap();

        debug!("new: {}", new);
        tx.send(Event::MetadataEditorEditAlbum((
            selection.name.to_string(),
            new.to_string(),
        )))
        .unwrap();
        s.pop_layer();
    })
    .title("Edit Artist");
    s.add_layer(dialog);
}

fn on_album_add_command(s: &mut Cursive, tx: Sender<Event>) {
    let dialog = Dialog::around(
        LinearLayout::vertical().child(
            LinearLayout::horizontal()
                .child(TextView::new("new album name: "))
                .child(
                    EditView::new()
                        .with_name("album_name_edit_view")
                        .min_width(10),
                ),
        ), // TODO: implement some kind of autocompletion
    )
    .dismiss_button("Dismiss")
    .button("Ok", move |s| {
        let new = s
            .call_on_name("album_name_edit_view", |view: &mut EditView| {
                view.get_content()
            })
            .unwrap();

        debug!("new: {}", new);
        tx.send(Event::MetadataEditorAddAlbum(new.to_string()))
            .unwrap();
        s.pop_layer();
    })
    .title("Edit Album");
    s.add_layer(dialog);
}
