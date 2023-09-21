use iced::{
    color,
    widget::{button, container, row},
    BorderRadius, Color, Theme,
};

pub struct LocalButton;

impl button::StyleSheet for LocalButton {
    type Style = Theme;

    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(color!(120, 255, 120).into()),
            border_radius: 5.0.into(),
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
            border_radius: 5.0.into(),
            ..Default::default()
        }
    }
}

pub struct StatusBarContainer;
impl container::StyleSheet for StatusBarContainer {
    type Style = Theme;

    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(color!(211, 211, 211).into()),
            border_radius: 10.0.into(),
            border_color: Color::BLACK,
            border_width: 1.0,
            ..Default::default()
        }
    }
}
