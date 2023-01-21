use std::sync::mpsc::Sender;

use cursive::{
    view::{Nameable, Resizable},
    views::{Dialog, EditView, LinearLayout, NamedView, TextView},
    Cursive,
};
use youtube_dl::SingleVideo;

use super::{
    event_runner::{Event, YoutubeDownloadOptions},
    State,
};

pub fn draw_download_tab(_siv: &mut Cursive, tx: Sender<Event>) -> NamedView<LinearLayout> {
    let ss_tx = tx.clone();
    let search_box = EditView::new().on_submit(move |siv: &mut Cursive, text: &str| {
        if !text.is_empty() {
            ss_tx.send(Event::YoutubeSearch(text.to_string())).unwrap();
        } else {
            siv.add_layer(Dialog::text("Can't search for nothingness").dismiss_button("Dismiss"));
        }
    });

    let vlayout = LinearLayout::vertical()
        .child(TextView::new("Search:"))
        .child(search_box)
        .with_name("download_v_layout");
    vlayout
}

pub fn start_download(siv: &mut Cursive, song: &SingleVideo) {
    // Show popup to confirm
    let title = song.title.clone();
    let channel = song.channel.clone().unwrap_or("Unknown".to_string());
    let song2 = song.clone();
    let confirm = Dialog::text(format!(
        "Title: {}\nChannel:{}\nConfirm? to edit?",
        title, channel
    ))
    .dismiss_button("Cancel")
    .button("Edit", move |siv: &mut Cursive| {
        let id = song2.id.clone();
        let title = song2.title.clone();
        let artist = song2.channel.clone().unwrap();

        siv.pop_layer();
        draw_metadata_editor(siv, id, title, artist, song2.clone());
    });
    siv.add_layer(confirm);
}

fn draw_metadata_editor(
    siv: &mut Cursive,
    id: String,
    title: String,
    artist: String,
    song: SingleVideo,
) {
    let left = LinearLayout::vertical()
        .child(TextView::new("Title"))
        .child(TextView::new("Artist"))
        .child(TextView::new("Album"));

    let right = LinearLayout::vertical()
        .child(
            EditView::new()
                .content(title)
                .with_name("title_input")
                .min_width(30),
        )
        .child(
            EditView::new()
                .content(artist)
                .with_name("artist_input")
                .min_width(30),
        )
        .child(EditView::new().with_name("album_input").min_width(30));

    let hlayout = LinearLayout::horizontal().child(left).child(right);

    siv.add_layer(Dialog::around(hlayout).dismiss_button("Cancel").button(
        "Ok",
        move |siv: &mut Cursive| {
            let user_data: &mut State = siv.user_data().unwrap();
            let tx = user_data.tx.clone();

            let music_dir = user_data.music_dir.clone();
            let id = id.clone();
            let title = siv
                .call_on_name("title_input", |v: &mut EditView| {
                    v.get_content().to_string()
                })
                .unwrap();

            let artist = siv
                .call_on_name("artist_input", |v: &mut EditView| {
                    v.get_content().to_string()
                })
                .unwrap();
            let album = siv
                .call_on_name("album_input", |v: &mut EditView| {
                    v.get_content().to_string()
                })
                .unwrap();
            tx.send(Event::YoutubeDownload(YoutubeDownloadOptions {
                id,
                title: title,
                album: album,
                artist: artist,
                song: song.clone(),
                music_dir,
            }))
            .unwrap();
            siv.pop_layer();
        },
    ));
}
