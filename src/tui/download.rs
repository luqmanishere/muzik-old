use crossbeam_channel::Sender;

use cursive::{
    view::{Nameable, Resizable, Scrollable},
    views::{Dialog, EditView, LinearLayout, NamedView, Panel, SelectView, TextView},
    Cursive,
};
use youtube_dl::SingleVideo;

use super::event_runner::{DownloadMetadataInput, Event};

pub fn draw_download_tab(_siv: &mut Cursive, tx: Sender<Event>) -> NamedView<LinearLayout> {
    let search_box_tx = tx.clone();
    let search_box = EditView::new().on_submit(move |siv: &mut Cursive, text: &str| {
        if !text.is_empty() {
            search_box_tx
                .send(Event::YoutubeSearch(text.to_string()))
                .unwrap();
        } else {
            siv.add_layer(Dialog::text("Can't search for nothingness").dismiss_button("Dismiss"));
        }
    });

    let video_select_tx = tx;
    LinearLayout::vertical()
        .child(TextView::new("Search:"))
        .child(search_box)
        .child(Dialog::around(
            SelectView::<SingleVideo>::new()
                .on_submit(move |_, video| {
                    // send selection to the event thread
                    video_select_tx
                        .send(Event::OnDownloadVideoSelect(video.clone()))
                        .unwrap();
                })
                .with_name("result_selectview")
                .scrollable(),
        ))
        .child(
            Panel::new(TextView::new("Standby").with_name("statusbar"))
                .title("Status Bar")
                .title_position(cursive::align::HAlign::Left),
        )
        .with_name("download_v_layout")
}

pub fn draw_metadata_editor(siv: &mut Cursive, song: SingleVideo, tx: Sender<Event>) {
    let id = song.id.clone();
    let title = song.title.clone();
    let artist = {
        if let Some(artist) = song.artist.clone() {
            artist
        } else if let Some(channel) = song.channel.clone() {
            channel
        } else {
            "Unknown".to_string()
        }
    };
    let album = song.album.clone().unwrap_or_else(|| "Unknown".to_string());
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
        .child(
            EditView::new()
                .content(album)
                .with_name("album_input")
                .min_width(30),
        );

    let hlayout = LinearLayout::horizontal().child(left).child(right);

    siv.add_layer(Dialog::around(hlayout).dismiss_button("Cancel").button(
        "Ok",
        move |siv: &mut Cursive| {
            // Get the inputs, then send them to event runner
            //
            //
            let video = song.clone();
            let id = id.clone();
            let genre = song.genre.clone().unwrap_or_else(|| "Unknown".to_string());
            let title = siv.call_on_name("title_input", |v: &mut EditView| {
                v.get_content().to_string()
            });

            let artist = siv.call_on_name("artist_input", |v: &mut EditView| {
                v.get_content().to_string()
            });
            let album = siv.call_on_name("album_input", |v: &mut EditView| {
                v.get_content().to_string()
            });

            let met = DownloadMetadataInput {
                id,
                title,
                artist,
                album,
                genre: Some(genre),
                video,
            };

            tx.send(Event::OnDownloadMetadataSubmit(met)).unwrap();
            siv.pop_layer();
        },
    ));
}
