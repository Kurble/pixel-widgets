use std::borrow::Cow;

use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::node::{IntoNode, Node};
use crate::stylesheet::Stylesheet;
use crate::text;
use crate::widget::*;

/// Widget that renders a paragraph of text.
#[derive(Default)]
pub struct Text {
    text: String,
}

impl Text {
    /// Constructs a new `Text`
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self { text: text.into() }
    }

    /// Sets the text value.
    pub fn val(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }
}

impl<'a, T> Widget<'a, T> for Text {
    type State = ();

    fn mount(&self) {}

    fn widget(&self) -> &'static str {
        "text"
    }

    fn len(&self) -> usize {
        0
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {}

    fn size(&self, _: &(), style: &Stylesheet) -> (Size, Size) {
        let width = style.width;
        let height = style.height;
        let text = text::Text {
            text: Cow::Borrowed(self.text.as_str()),
            font: style.font.clone(),
            size: style.text_size,
            wrap: style.text_wrap,
            color: style.color,
        };
        let content = match (width, height) {
            (Size::Shrink, Size::Shrink) => {
                let measured = text.measure(None);
                (Size::Exact(measured.width()), Size::Exact(measured.height()))
            }
            (Size::Shrink, height) => {
                let measured = text.measure(None);
                (Size::Exact(measured.width()), height)
            }
            (Size::Exact(size), Size::Shrink) => {
                let measured = text.measure(Some(Rectangle::from_wh(size, std::f32::INFINITY)));
                (Size::Exact(size), Size::Exact(measured.height()))
            }
            (width, height) => (width, height),
        };
        style
            .background
            .resolve_size((style.width, style.height), content, style.padding)
    }

    fn event(&mut self, _: &mut (), _: Rectangle, _: Rectangle, _: &Stylesheet, _: Event, _: &mut Context<T>) {}

    fn draw(&mut self, _: &mut (), layout: Rectangle, _: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();
        result.extend(style.background.render(layout));
        result.push(Primitive::DrawText(
            text::Text {
                text: Cow::Owned(self.text.clone()),
                font: style.font.clone(),
                size: style.text_size,
                wrap: style.text_wrap,
                color: style.color,
            },
            style.background.content_rect(layout, style.padding),
        ));
        result
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Text {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

impl<'a, T: 'a, S: 'a + Into<String>> IntoNode<'a, T> for S {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(Text::new(self))
    }
}
