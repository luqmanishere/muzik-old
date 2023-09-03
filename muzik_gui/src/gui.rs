use std::sync::Arc;

use iced::{
    keyboard::{self, KeyCode, Modifiers},
    theme::Theme,
    widget::{container, text, Column},
    Application, Command, Element, Event, Length,
};
use iced_aw::{TabLabel, Tabs};
use muzik_common::database::DbConnection;

use crate::{
    config::Config,
    editor::{EditorMessage, EditorTab},
};

/// GUI start point
#[allow(dead_code)]
pub struct GuiMain {
    config: Config,
    db: Arc<DbConnection>,

    active_tab: TabId,
    editor_state: EditorTab,
}

impl Application for GuiMain {
    type Message = Msg;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = Config;

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let db = Arc::new(_flags.db_new.clone());
        let (editor_state, e_comms) = EditorTab::new_with_command(_flags.clone(), db.clone());
        let comms = vec![e_comms];
        (
            Self {
                config: _flags.clone(),
                db: db.clone(),
                active_tab: TabId::Editor,
                editor_state,
            },
            Command::batch(comms),
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
            Msg::Editor(msg) => return self.editor_state.update(msg),
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
            Msg::None => {}
        };
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        Tabs::new(Self::Message::TabSelected)
            .push(
                TabId::Editor,
                self.editor_state.tab_label(),
                self.editor_state.view(),
            )
            .set_active_tab(&self.active_tab)
            .into()
    }
}

impl GuiMain {}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TabId {
    Editor,
}

#[derive(Debug, Clone)]
pub enum Msg {
    TabSelected(TabId),
    Editor(EditorMessage),
    IcedEvent(Event),

    // ? what is the use of this anyways
    #[allow(dead_code)]
    None,
}

pub trait Tab {
    /// Messages for this Tab
    type Message;
    /// Messages from the Tab Manager
    type ReturnMessage;

    fn title(&self) -> String;

    /// Label of the tab
    fn tab_label(&self) -> TabLabel;

    /// View to be rendered
    fn view(&self) -> Element<'_, Self::ReturnMessage> {
        let column = Column::new()
            .spacing(20)
            .push(text(self.title()).size(30))
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
    fn content(&self) -> Element<'_, Self::ReturnMessage>;

    /// Update commands
    fn update(&mut self, message: Self::Message) -> Command<Self::ReturnMessage>;
}
