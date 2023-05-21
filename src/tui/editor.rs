use std::sync::mpsc::Sender;

use cursive::{
    view::{Nameable, Resizable, Scrollable},
    views::{Dialog, EditView, LinearLayout, Panel, SelectView, TextView},
    Cursive,
};

use crate::database::Song;

use super::event_runner::Event;

pub fn draw_database_editor(tx: Sender<Event>) -> LinearLayout {
    // Default before data is loaded in
    let list = vec![Song::default()];
    let select_song_list = list.iter().enumerate().map(|(ind, f)| {
        (
            format!("{} - {}", f.get_title_string(), f.get_artists_string()),
            ind,
        )
    });
    let mut select_song = SelectView::new();
    select_song.add_all(select_song_list.into_iter());
    tx.send(Event::UpdateEditorSongSelectView).unwrap();

    let ttx = tx.clone();
    let select_song = select_song.on_submit(move |_, index| {
        ttx.send(Event::UpdateEditorMetadataSelectView(*index))
            .unwrap();
    });
    let select_song = select_song
        .with_name("select_song")
        .scrollable()
        .min_width(20)
        .full_width()
        .full_height();
    let select_song = Panel::new(select_song).title("Songs");

    // Metadata SelectView entries are dynamically updated
    let mut select_metadata = SelectView::new().item("Empty".to_string(), "Empty".to_string());
    let ttx = tx.clone();
    select_metadata.set_on_submit(move |_siv: &mut Cursive, _item: &String| {
        ttx.send(Event::OnMetadataSelect).unwrap();
    });
    let select_metadata = select_metadata.with_name("select_metadata");
    let select_metadata = select_metadata.scrollable().min_width(20);
    let select_metadata = Panel::new(select_metadata).title("Metadata").full_width();

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
            TextView::new(
                "d - Delete | u - Update list | V - verify all | R - download all missing",
            )
            .h_align(cursive::align::HAlign::Center),
        )
        .child(
            Panel::new(TextView::new("Standby").with_name("statusbar"))
                .title("Status Bar")
                .title_position(cursive::align::HAlign::Left),
        )
}

pub fn editor_layer(_siv: &mut Cursive, song: Song, tx: Sender<Event>) -> Dialog {
    let left_layout = LinearLayout::vertical()
        .child(TextView::new("Title:"))
        .child(TextView::new("Artist"))
        .child(TextView::new("Album"));

    let right_layout = LinearLayout::vertical()
        .child(
            EditView::new()
                .content(song.title.as_ref().unwrap().clone())
                .with_name("editor_title")
                .min_width(30),
        )
        .child(
            EditView::new()
                .content(song.get_artists_string())
                .with_name("editor_artist")
                .min_width(30),
        )
        .child(
            EditView::new()
                .content(song.get_albums_string())
                .with_name("editor_album")
                .min_width(30),
        );

    Dialog::around(
        LinearLayout::horizontal()
            .child(left_layout)
            .child(right_layout),
    )
    .dismiss_button("Cancel")
    .button("Ok", move |siv: &mut Cursive| {
        let mut song = song.clone();
        let title = siv
            .call_on_name("editor_title", |view: &mut EditView| view.get_content())
            .unwrap()
            .to_string();
        let artist = siv
            .call_on_name("editor_artist", |view: &mut EditView| view.get_content())
            .unwrap()
            .to_string();
        let album = siv
            .call_on_name("editor_album", |view: &mut EditView| view.get_content())
            .unwrap()
            .to_string();

        song.set_title(Some(title));
        song.set_artists(artist);
        song.set_albums(album);
        // TODO: actually set genre
        song.set_genre(String::from("Unknown"));
        tx.send(Event::UpdateSongDatabase(song)).unwrap();
        siv.pop_layer();
    })
}
