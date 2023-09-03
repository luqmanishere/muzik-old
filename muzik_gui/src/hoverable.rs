use iced_native::{
    event,
    widget::{tree, Tree},
    Clipboard, Element, Event, Layout, Length, Padding, Shell, Widget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    is_hovered: bool,
}

pub struct Hoverable<'a, Message, Renderer> {
    content: Element<'a, Message, Renderer>,
    on_hover: Message,
    on_unhover: Message,
    padding: Padding,
}

impl<'a, Message, Renderer> Hoverable<'a, Message, Renderer>
where
    Renderer: iced_native::Renderer,
{
    const WIDTH: Length = Length::Shrink;
    const HEIGHT: Length = Length::Shrink;

    pub fn new(
        content: Element<'a, Message, Renderer>,
        on_hover: Message,
        on_unhover: Message,
    ) -> Self {
        Self {
            content,
            on_hover,
            on_unhover,
            padding: Padding::ZERO,
        }
    }

    pub fn padding<P>(mut self, padding: P) -> Self
    where
        P: Into<Padding>,
    {
        self.padding = padding.into();
        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for Hoverable<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: iced_native::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<iced_native::widget::Tree> {
        vec![iced_native::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor_position: iced_native::Point,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        if let event::Status::Captured = self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            cursor_position,
            renderer,
            clipboard,
            shell,
        ) {
            return event::Status::Captured;
        }

        let state = tree.state.downcast_mut::<State>();
        let was_hovered = state.is_hovered;
        let now_hovered = layout.bounds().contains(cursor_position);

        match (was_hovered, now_hovered) {
            (true, true) => {}
            (false, false) => {}
            (true, false) => {
                // exited hover
                state.is_hovered = now_hovered;
                shell.publish(self.on_unhover.clone());
            }
            (false, true) => {
                // entered hover
                state.is_hovered = now_hovered;
                shell.publish(self.on_hover.clone());
            }
        }

        event::Status::Ignored
    }

    fn width(&self) -> Length {
        Self::WIDTH
    }

    fn height(&self) -> Length {
        Self::HEIGHT
    }

    fn layout(
        &self,
        _renderer: &Renderer,
        _limits: &iced_native::layout::Limits,
    ) -> iced_native::layout::Node {
        todo!()
    }

    fn draw(
        &self,
        _state: &Tree,
        _renderer: &mut Renderer,
        _theme: &<Renderer as iced_native::Renderer>::Theme,
        _style: &iced_native::renderer::Style,
        _layout: Layout<'_>,
        _cursor_position: iced_native::Point,
        _viewport: &iced_native::Rectangle,
    ) {
        todo!()
    }
}
