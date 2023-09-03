use std::marker::PhantomData;

use iced::{widget::TextInput, Element};

#[derive(Debug, Default)]
pub struct MultiStringInput<M> {
    phantom_data: PhantomData<M>,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiStringInputMessage {
    Change(String),
}

impl MultiStringInputMessage {
    pub fn get_data(&self) -> String {
        let MultiStringInputMessage::Change(data) = self;
        data.clone()
    }
}

impl<M> MultiStringInput<M> {
    pub fn new(value: String) -> MultiStringInput<M> {
        MultiStringInput {
            phantom_data: PhantomData,
            value,
        }
    }

    pub fn view<F>(&self, id: usize, placeholder: &str, on_change: F) -> Element<M>
    where
        F: 'static + Fn((usize, MultiStringInputMessage)) -> M + Copy,
        M: 'static,
    {
        let text_i =
            TextInput::new(placeholder, &self.value).on_input(MultiStringInputMessage::Change);
        Element::new(text_i).map(move |i| on_change((id, i)))
    }
}
