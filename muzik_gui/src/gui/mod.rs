use std::sync::Arc;

use crossbeam_channel::Receiver;
use iced::{
    futures::SinkExt,
    keyboard::{self, KeyCode, Modifiers},
    theme::Theme,
    widget::{
        column, container, horizontal_rule, scrollable as scrollablefn,
        scrollable::{self, RelativeOffset},
        text, vertical_space, Column,
    },
    Application, Command, Element, Event, Length, Subscription,
};
use iced_aw::{TabLabel, Tabs};
use muzik_common::{config::Config, database::DbConnection};
use tokio::task::block_in_place;
use tracing::error;

use crate::log::GuiEvent;

use self::{
    downloader::{DownloaderMsg, DownloaderTab},
    editor::{EditorMessage, EditorTab},
    theme::StatusBarContainer,
};

mod downloader;
mod editor;
mod multi_input;
mod theme;

struct StatusScroll;

/// GUI start point
#[allow(dead_code)]
pub struct GuiMain {
    config: Config,
    db: Arc<DbConnection>,

    active_tab: TabId,
    editor_state: EditorTab,
    downloader_state: DownloaderTab,

    action_log: Vec<Actions>,
    events_log_receiver: Receiver<GuiEvent>,
    events_log: Vec<GuiEvent>,
}

impl Application for GuiMain {
    type Message = Msg;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = (Config, Option<Receiver<GuiEvent>>);

    fn new(flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let (flags, events_log) = flags;
        let events_log = events_log.expect("provided");
        let mut commands = vec![];
        let db = Arc::new(flags.db_new.clone());

        let (editor_state, e_comms) = EditorTab::new_with_command(flags.clone(), db.clone());
        commands.push(e_comms);

        let (downloader_state, d_comms) = DownloaderTab::new(flags.clone(), db.clone());
        commands.push(d_comms);

        (
            Self {
                config: flags.clone(),
                db: db.clone(),
                active_tab: TabId::Editor,
                editor_state,
                downloader_state,
                action_log: vec![],
                events_log_receiver: events_log,
                events_log: vec![],
            },
            Command::batch(commands),
        )
    }
    fn title(&self) -> String {
        "muzik - music manager".to_string()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let rx = self.events_log_receiver.clone();
        let rx2 = rx.clone();
        struct Id;
        let test = iced::subscription::unfold(std::any::TypeId::of::<Id>(), rx2, |rx| async move {
            block_in_place(|| async {
                match rx.recv() {
                    Ok(res) => {
                        println!("got log, sending log");
                        (Msg::Log(res.clone()), rx)
                    }
                    Err(_) => (Msg::None, rx),
                }
            })
            .await
        });
        Subscription::batch(vec![test, iced::subscription::events().map(Msg::IcedEvent)])
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        const NO_MODIFIER: Modifiers = Modifiers::empty();
        match message {
            Msg::Editor(msg) => return self.editor_state.update(Msg::Editor(msg)),
            Msg::Downloader(msg) => return self.downloader_state.update(Msg::Downloader(msg)),
            Msg::TabSelected(tab_id) => self.active_tab = tab_id,
            Msg::IcedEvent(event) => {
                // TODO: handle keyboard events not input here
                match event {
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key_code,
                        modifiers,
                    }) => match modifiers {
                        NO_MODIFIER => match key_code {
                            KeyCode::Tab => return iced::widget::focus_next(),
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                }
            }
            Msg::PushAction(action) => self.action_log.push(action),
            Msg::None => {}
            Msg::Log(event) => {
                println!("got event");
                self.events_log.push(dbg!(event));
                return scrollable::snap_to(
                    scrollable::Id::new("status_scroll"),
                    scrollable::RelativeOffset { x: 0.0, y: 1.0 },
                );
            }
        };
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        let title = text(self.title())
            .width(Length::Fill)
            .horizontal_alignment(iced::alignment::Horizontal::Center);
        let tabs = Tabs::new(Self::Message::TabSelected)
            .push(
                TabId::Editor,
                self.editor_state.tab_label(),
                self.editor_state.view(),
            )
            .push(
                TabId::Downloader,
                self.downloader_state.tab_label(),
                self.downloader_state.view(),
            )
            .set_active_tab(&self.active_tab);

        let status_bar = {
            let events: Element<'_, Msg> = if !self.events_log.is_empty() {
                let mut events_column = Column::new().spacing(3);
                for event in &self.events_log {
                    events_column = events_column.push(event.display());
                }
                events_column.padding(10).into()
            } else {
                text("no logs to show").width(Length::Fill).into()
            };
            container(
                scrollablefn(events)
                    .width(Length::Fill)
                    .id(scrollable::Id::new("status_scroll")),
            )
        }
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(StatusBarContainer)));

        container(
            column(vec![
                title.into(),
                container(tabs)
                    .padding(10)
                    .width(Length::Fill)
                    .height(Length::FillPortion(3))
                    .into(),
                horizontal_rule(10).into(),
                status_bar.height(Length::FillPortion(1)).into(),
            ])
            .spacing(10),
        )
        .padding(5)
        .into()
    }
}

impl GuiMain {}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TabId {
    Editor,
    Downloader,
}

#[derive(Debug, Clone)]
pub enum Msg {
    TabSelected(TabId),
    Editor(EditorMessage),
    Downloader(DownloaderMsg),
    IcedEvent(Event),

    PushAction(Actions),
    Log(GuiEvent),

    // ? what is the use of this anyways
    #[allow(dead_code)]
    None,
}

#[derive(Debug, Clone)]
pub enum Actions {
    // Downloader
    /// Takes search keyword as arg
    SearchYoutubeStart(String),
    StartDownloadFromYoutube(String),
    WriteTagsToFile(String),
    DoneInsertIntoDatabase(String),
    Done,
}

impl std::fmt::Display for Actions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Actions::SearchYoutubeStart(keyword) => {
                write!(f, "Searching youtube for: {keyword}")
            }
            Actions::Done => write!(f, "Done!"),
            Actions::StartDownloadFromYoutube(id) => {
                write!(f, "Downloaded from YouTube: {id}")
            }
            Actions::WriteTagsToFile(path) => write!(f, "Wrote tags to file {path}"),
            Actions::DoneInsertIntoDatabase(id) => {
                write!(f, "Inserting entries into database, song id is {id}")
            }
        }
    }
}

trait EventsDisplay {
    type Message;

    fn display(&self) -> Element<Msg>;
}

impl EventsDisplay for GuiEvent {
    type Message = Msg;

    fn display(&self) -> Element<Msg> {
        // LEVEL TIMESTAMP IDENTIFIER MESSAGE
        let form = format!("{} | {}", self.level.to_string(), self.get_message());
        text(form).into()
    }
}

pub trait Tab {
    /// Messages for this Tab
    type Message;

    fn title(&self) -> String;

    /// Label of the tab
    fn tab_label(&self) -> TabLabel;

    /// View to be rendered
    fn view(&self) -> Element<'_, Self::Message> {
        let column = Column::new()
            .push(
                text(self.title())
                    .size(30)
                    .width(Length::Fill)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .push(self.content());

        container(column)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .padding(10)
            .into()
    }

    /// Content to be rendered
    fn content(&self) -> Element<'_, Self::Message>;

    /// Update commands
    fn update(&mut self, message: Self::Message) -> Command<Self::Message>;
}
