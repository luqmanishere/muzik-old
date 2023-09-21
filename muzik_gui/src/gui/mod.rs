use std::sync::Arc;

use iced::{
    keyboard::{self, KeyCode, Modifiers},
    theme::Theme,
    widget::{column, container, horizontal_rule, row, scrollable, text, vertical_rule, Column},
    Application, Command, Element, Event, Length,
};
use iced_aw::{TabLabel, Tabs};
use muzik_common::database::DbConnection;

use crate::config::Config;

use self::{
    downloader::{DownloaderMsg, DownloaderTab},
    editor::{EditorMessage, EditorTab},
    theme::StatusBarContainer,
};

mod downloader;
mod editor;
mod hoverable;
mod multi_input;
mod theme;

/// GUI start point
#[allow(dead_code)]
pub struct GuiMain {
    config: Config,
    db: Arc<DbConnection>,

    active_tab: TabId,
    editor_state: EditorTab,
    downloader_state: DownloaderTab,

    action_log: Vec<Actions>,
}

impl Application for GuiMain {
    type Message = Msg;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = Config;

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let mut commands = vec![];

        let db = Arc::new(_flags.db_new.clone());

        let (editor_state, e_comms) = EditorTab::new_with_command(_flags.clone(), db.clone());
        commands.push(e_comms);

        let (downloader_state, d_comms) = DownloaderTab::new(_flags.clone(), db.clone());
        commands.push(d_comms);

        (
            Self {
                config: _flags.clone(),
                db: db.clone(),
                active_tab: TabId::Editor,
                editor_state,
                downloader_state,
                action_log: vec![],
            },
            Command::batch(commands),
        )
    }
    fn title(&self) -> String {
        "muzik - music manager".to_string()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced::subscription::events().map(Msg::IcedEvent)
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
        };
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        // TODO: align the title to the center
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
            .set_active_tab(&self.active_tab)
            .height(Length::FillPortion(3));

        let status_bar = {
            let actions: Element<'_, Msg> = if !self.action_log.is_empty() {
                let mut log_column = Column::new().spacing(3);
                for action in &self.action_log {
                    let action_text = text(action.to_string());
                    log_column = log_column.push(action_text);
                }
                log_column.into()
            } else {
                text("No actions yet!").into()
            };
            container(scrollable(actions))
        }
        .height(Length::FillPortion(1))
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(StatusBarContainer)))
        .padding(10);

        container(column(vec![
            title.into(),
            tabs.into(),
            horizontal_rule(10).into(),
            status_bar.into(),
            horizontal_rule(10).into(),
        ]))
        .center_x()
        .center_y()
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

    // ? what is the use of this anyways
    #[allow(dead_code)]
    None,
}

#[derive(Debug, Clone)]
pub enum Actions {
    // Downloader
    /// Takes search keyword as arg
    SearchYoutube(String),
    Done,
}

impl std::fmt::Display for Actions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Actions::SearchYoutube(keyword) => {
                write!(f, "Searching youtube for: {keyword}")
            }
            Actions::Done => write!(f, "Done!"),
        }
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
            .spacing(20)
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
