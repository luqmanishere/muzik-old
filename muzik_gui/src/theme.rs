use iced::{color, widget::button, Theme};

pub struct LocalButton;

impl button::StyleSheet for LocalButton {
    type Style = Theme;

    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(color!(120, 255, 120).into()),
            ..Default::default()
        }
    }
}

pub struct DbButton;
impl button::StyleSheet for DbButton {
    type Style = Theme;

    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(color!(95, 158, 160).into()),
            ..Default::default()
        }
    }
}
