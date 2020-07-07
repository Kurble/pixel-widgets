use crate::draw::Primitive;
use crate::element::Element;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::{ElementKey, Stylesheet};
use crate::text;
use std::borrow::Cow;

pub struct Text<'a> {
    text: Cow<'a, str>,
    class: &'static str,
}

impl<'a> Text<'a> {
    pub fn borrowed(text: &'a str) -> Self {
        Self {
            text: Cow::Borrowed(text),
            class: "",
        }
    }

    pub fn owned(text: String) -> Self {
        Self {
            text: Cow::Owned(text),
            class: "",
        }
    }
}

impl<'a, T> Element<'a, T> for Text<'a> {
    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        (
            stylesheet.width(self.class, &ElementKey::Text),
            stylesheet.height(self.class, &ElementKey::Text),
        )
    }

    fn event(&mut self, _: Rectangle, _: &Stylesheet, _: Event) -> Option<T> {
        None
    }

    fn render(&mut self, layout: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        vec![Primitive::DrawText(
            text::Text {
                text: std::mem::replace(&mut self.text, Cow::Borrowed("<error>")),
                font: stylesheet.font(self.class, &ElementKey::Text).clone(),
                size: stylesheet.text_size(self.class, &ElementKey::Text).clone(),
                wrap: stylesheet.text_wrap(self.class, &ElementKey::Text).clone(),
                color: stylesheet.color(self.class, &ElementKey::Text).clone(),
                border: None,
                padding: stylesheet.padding(self.class, &ElementKey::Text).clone(),
            },
            layout,
        )]
    }
}
