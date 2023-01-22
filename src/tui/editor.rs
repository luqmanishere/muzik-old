use std::sync::mpsc::Sender;

use cursive::{
    view::{Nameable, Resizable, Scrollable},
    views::{Dialog, EditView, LinearLayout, Panel, SelectView, TextView},
    Cursive,
};

use crate::database::Song;

use super::{event_runner::Event, State};

pub fn draw_database_editor(siv: &mut Cursive, tx: Sender<Event>) -> LinearLayout {
    let user_data: &mut State = siv.user_data().unwrap();
    let song_list = match &user_data.song_list {
        Some(sl) => sl.clone(),
        None => {
            if let Some(db) = &user_data.db {
                let song_list = db.get_all(user_data.music_dir.clone()).unwrap();
                siv.with_user_data(|f: &mut State| f.song_list = Some(song_list.clone()));
                song_list
            } else {
                vec![]
            }
        }
    };

    let select_song_list = song_list.iter().enumerate().map(|(ind, f)| {
        (
            format!("{} - {}", f.title.clone().unwrap(), f.get_artists_string()),
            ind,
        )
    });
    let mut select_song = SelectView::new();
    select_song.add_all(select_song_list.into_iter());

    let select_song = select_song.on_submit(|siv, item| {
        let user_data: &mut State = siv.user_data().unwrap();
        user_data.song_index = Some(*item);
        let mut song_list = user_data.song_list.clone().unwrap();
        let song = song_list.get_mut(*item).unwrap();
        user_data.current_selected_song = Some(song.clone());
        siv.call_on_name("select_metadata", |view: &mut SelectView<String>| {
            view.clear();
            let title = song.title.as_ref().unwrap();
            let artist = song.get_artists_string();
            let album = song.get_albums_string();
            view.add_item(title.clone(), title.clone());
            view.add_item(artist.clone(), artist);
            view.add_item(album.clone(), album);
        });
    });
    let select_song = select_song
        .with_name("select_song")
        .scrollable()
        .min_size((20, 10));
    let select_song = Panel::new(select_song).title("Songs");

    let mut select_metadata = SelectView::new().item("Empty".to_string(), "Empty".to_string());
    let tx = tx;
    select_metadata.set_on_submit(move |siv: &mut Cursive, _item: &String| {
        let user_data: &mut State = siv.user_data().unwrap();
        let song = user_data.current_selected_song.as_ref().unwrap().clone();
        // TODO: show editing prompt
        let editor = editor_layer(siv, song, tx.clone());
        siv.add_layer(editor);
    });
    let select_metadata = select_metadata.with_name("select_metadata");
    let select_metadata = select_metadata.scrollable().min_size((20, 10));
    let select_metadata = Panel::new(select_metadata).title("Metadata");

    let hlayout = LinearLayout::horizontal()
        .child(select_song)
        .child(select_metadata);

    LinearLayout::vertical()
        .child(TextView::new("Database Editor").h_align(cursive::align::HAlign::Center))
        .child(hlayout)
        .child(TextView::new(
            "d - Delete | u - Update list | V - verify all | R - download all missing",
        ))
        .child(
            Panel::new(TextView::new("Standby").with_name("statusbar"))
                .title("Status Bar")
                .title_position(cursive::align::HAlign::Left),
        )
}

fn editor_layer(_siv: &mut Cursive, song: Song, tx: Sender<Event>) -> Dialog {
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

        song.title = Some(title);
        song.set_artists(artist);
        song.set_albums(album);
        tx.send(Event::UpdateSongDatabase(song)).unwrap();
        siv.pop_layer();
    })
}

pub fn update_database(siv: &mut Cursive) {
    let user_data: &mut State = siv.user_data().unwrap();
    let db = user_data.db.as_ref().unwrap();
    let song_list = db.get_all(user_data.music_dir.clone()).unwrap();
    siv.with_user_data(|f: &mut State| f.song_list = Some(song_list.clone()));

    let select_song_list = song_list.iter().enumerate().map(|(ind, f)| {
        (
            format!("{} - {}", f.title.clone().unwrap(), f.get_artists_string()),
            ind,
        )
    });

    siv.call_on_name("select_song", |view: &mut SelectView<usize>| {
        view.clear();
        view.add_all(select_song_list.into_iter());
    });
}

pub fn delete_from_database(siv: &mut Cursive) {
    let user_data: &mut State = siv.user_data().unwrap();
    let mut song_list = user_data.song_list.clone().unwrap();
    let tx = user_data.tx.clone();
    siv.call_on_name("select_song", |view: &mut SelectView<usize>| {
        let item = view.selection().unwrap();
        let song = song_list.get_mut(*item).unwrap().clone();
        tx.send(Event::DeleteSongDatabase(song)).unwrap();
    });
}

pub fn verify_all_song_integrity(siv: &mut Cursive) {
    let user_data: &mut State = siv.user_data().unwrap();
    let tx = user_data.tx.clone();
    tx.send(Event::VerifyAllSongIntegrity()).unwrap();
}

pub fn download_all_missing(siv: &mut Cursive) {
    let user_data: &mut State = siv.user_data().unwrap();
    let tx = user_data.tx.clone();
    tx.send(Event::DownloadAllMissingFromDatabase).unwrap();
}
