use crate::draw::Primitive;
use crate::element::*;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::{text, Context};
use std::borrow::Cow;

pub struct Text {
    text: String,
}

impl Text {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self { text: text.into() }
    }
}

impl<'a, T> Element<'a, T> for Text {
    fn element(&self) -> &'static str {
        "text"
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn Stylable<'a>)) {}

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        let width = style.width;
        let height = style.height;
        let text = text::Text {
            text: Cow::Borrowed(self.text.as_str()),
            font: style.font.clone(),
            size: style.text_size.clone(),
            wrap: style.text_wrap.clone(),
            color: style.color.clone(),
        };
        match (width, height) {
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
        }
    }

    fn event(&mut self, _: Rectangle, _: Rectangle, _: &Stylesheet, _: Event, _: &mut Context<T>) {}

    fn draw(&mut self, layout: Rectangle, _: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        vec![Primitive::DrawText(
            text::Text {
                text: Cow::Owned(self.text.clone()),
                font: style.font.clone(),
                size: style.text_size.clone(),
                wrap: style.text_wrap.clone(),
                color: style.color.clone(),
            },
            layout,
        )]
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Text {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}
