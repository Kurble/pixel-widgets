use crate::draw::Primitive;
use crate::element::*;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::text;
use std::borrow::Cow;
use std::cell::Cell;

pub struct Text<'a> {
    text: Cell<TextState<'a>>,
}

enum TextState<'a> {
    Raw(Cow<'a, str>),
    Styled(text::Text<'a>),
    Consumed,
}

impl<'a> Text<'a> {
    pub fn borrowed(text: &'a str) -> Self {
        Self {
            text: Cell::new(TextState::Raw(Cow::Borrowed(text))),
        }
    }

    pub fn owned(text: String) -> Self {
        Self {
            text: Cell::new(TextState::Raw(Cow::Owned(text))),
        }
    }

    pub fn with_inner<F: FnOnce(&text::Text) -> T, T>(&self, stylesheet: &Stylesheet, f: F) -> T {
        self.style(stylesheet);
        if let TextState::Styled(inner) = self.text.replace(TextState::Consumed) {
            let result = f(&inner);
            self.text.replace(TextState::Styled(inner));
            result
        } else {
            panic!("Text is already consumed")
        }
    }

    fn style(&self, stylesheet: &Stylesheet) {
        self.text.replace(match self.text.replace(TextState::Consumed) {
            TextState::Raw(raw) => TextState::Styled(text::Text {
                text: raw,
                font: stylesheet.font.clone(),
                size: stylesheet.text_size.clone(),
                wrap: stylesheet.text_wrap.clone(),
                color: stylesheet.color.clone(),
            }),
            styled => styled,
        });
    }
}

impl<'a, T> Element<'a, T> for Text<'a> {
    fn element(&self) -> &'static str {
        "text"
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn Stylable<'a>)) {}

    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        let width = stylesheet.width;
        let height = stylesheet.height;
        match (width, height) {
            (Size::Shrink, Size::Shrink) => self.with_inner(stylesheet, |text| {
                let measured = text.measure(None);
                (Size::Exact(measured.width()), Size::Exact(measured.height()))
            }),
            (Size::Shrink, height) => self.with_inner(stylesheet, |text| {
                let measured = text.measure(None);
                (Size::Exact(measured.width()), height)
            }),
            (Size::Exact(size), Size::Shrink) => self.with_inner(stylesheet, |text| {
                let measured = text.measure(Some(Rectangle::from_wh(size, std::f32::INFINITY)));
                (Size::Exact(size), Size::Exact(measured.height()))
            }),
            (width, height) => (width, height),
        }
    }

    fn event(&mut self, _: Rectangle, _: Rectangle, _: &Stylesheet, _: Event) -> Option<T> {
        None
    }

    fn render(&mut self, layout: Rectangle, _: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        self.style(stylesheet);
        if let TextState::Styled(inner) = self.text.replace(TextState::Consumed) {
            vec![Primitive::DrawText(inner, layout)]
        } else {
            panic!("Text already consumed")
        }
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Text<'a> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}
