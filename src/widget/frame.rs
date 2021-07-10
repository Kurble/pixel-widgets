use crate::draw::Primitive;
use crate::layout::{Rectangle, Size};
use crate::node::{IntoNode, Node};
use crate::stylesheet::Stylesheet;
use crate::widget::*;

/// A widget that wraps around a content widget
pub struct Frame<'a, T> {
    content: Node<'a, T>,
}

impl<'a, T: 'a> Frame<'a, T> {
    /// Construct a new `Frame` with content
    pub fn new(content: impl IntoNode<'a, T>) -> Self {
        Self {
            content: content.into_node(),
        }
    }
}

impl<'a, T: 'a> Widget<'a, T> for Frame<'a, T> {
    type State = ();

    fn mount(&self) -> Self::State {
        ()
    }

    fn widget(&self) -> &'static str {
        "frame"
    }

    fn len(&self) -> usize {
        1
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {
        visitor(&mut *self.content);
    }

    fn size(&self, _: &(), style: &Stylesheet) -> (Size, Size) {
        style
            .background
            .resolve_size((style.width, style.height), self.content.size(), style.padding)
    }

    fn event(
        &mut self,
        _: &mut (),
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        self.content.event(
            style.background.content_rect(layout, style.padding),
            clip,
            event,
            context,
        );
    }

    fn draw(&mut self, _: &mut (), layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let content_rect = style.background.content_rect(layout, style.padding);

        style
            .background
            .render(layout)
            .into_iter()
            .chain(self.content.draw(content_rect, clip).into_iter())
            .collect()
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Frame<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}
