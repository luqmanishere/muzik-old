use crossbeam_channel::Sender;

use cursive::{
    view::{Nameable, Resizable, Scrollable},
    views::{Dialog, EditView, LinearLayout, SelectView, TextView},
    Cursive,
};
use muzik_common::util::youtube_dl::SingleVideo;

use super::event_runner::{DownloadMetadataInput, Event};

pub fn draw_list_confirm_box(siv: &mut Cursive, video_list: Vec<SingleVideo>, tx: Sender<Event>) {
    let iter = video_list.iter().enumerate().map(|(ind, f)| {
        (
            format!(
                "{} - {}",
                f.title.clone().unwrap_or("Unknown".to_string()),
                f.channel.clone().unwrap()
            ),
            ind,
        )
    });
    let mut list = SelectView::new();
    list.add_all(iter);
    let layout = LinearLayout::vertical()
        .child(TextView::new("New songs to add:"))
        .child(list.scrollable().show_scrollbars(true));

    let dialog = Dialog::around(layout)
        .button("Ok", move |siv: &mut Cursive| {
            tx.send(Event::OnSyncConfirm(video_list.clone())).unwrap();
            siv.pop_layer();
        })
        .dismiss_button("Cancel")
        .title("Youtube Sync");

    siv.add_layer(dialog);
}

pub fn draw_metadata_yt_sync(siv: &mut Cursive, video: SingleVideo, tx: Sender<Event>) {
    let yt_id = video.id.clone();
    let title = video.title.clone();
    let artist = {
        if let Some(artist) = video.artist.clone() {
            artist
        } else if let Some(channel) = video.channel.clone() {
            channel
        } else {
            "Unknown".to_string()
        }
    };
    let title_box_name = format!("title_{}", yt_id);
    let artist_box_name = format!("artist_{}", yt_id);
    let album_box_name = format!("album_{}", yt_id);

    let album = video.album.clone().unwrap_or_else(|| "Unknown".to_string());
    let left = LinearLayout::vertical()
        .child(TextView::new("Title"))
        .child(TextView::new("Artist"))
        .child(TextView::new("Album"));

    let right = LinearLayout::vertical()
        .child(
            EditView::new()
                .content(title.unwrap_or("Unknown".to_string()))
                .with_name(title_box_name.clone())
                .min_width(30),
        )
        .child(
            EditView::new()
                .content(artist)
                .with_name(artist_box_name.clone())
                .min_width(30),
        )
        .child(
            EditView::new()
                .content(album)
                .with_name(album_box_name.clone())
                .min_width(30),
        );

    let hlayout = LinearLayout::horizontal().child(left).child(right);

    siv.add_layer(Dialog::around(hlayout).dismiss_button("Cancel").button(
        "Ok",
        move |siv: &mut Cursive| {
            // Get the inputs, then send them to event runner
            //
            //
            let video = video.clone();
            let yt_id = yt_id.clone();
            let genre = video.genre.clone().unwrap_or_else(|| "Unknown".to_string());
            let title = siv.call_on_name(&title_box_name, |v: &mut EditView| {
                v.get_content().to_string()
            });

            let artist = siv.call_on_name(&artist_box_name, |v: &mut EditView| {
                v.get_content().to_string()
            });
            let album = siv.call_on_name(&album_box_name, |v: &mut EditView| {
                v.get_content().to_string()
            });

            let met = DownloadMetadataInput {
                id: yt_id,
                title,
                artist,
                album,
                genre: Some(genre),
                video,
            };

            tx.send(Event::OnSyncMetadataSubmit(met)).unwrap();
            siv.pop_layer();
        },
    ));
}
