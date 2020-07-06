use crate::draw::*;
use crate::element::Element;
use crate::layout::{Size, Rectangle};
use crate::event::Event;

pub struct Space {
    size: (Size, Size),
    background: Background,
}

impl Space {
    pub fn new(width: Size, height: Size) -> Self {
        Self {
            size: (width, height),
            background: Background::None,
        }
    }

    pub fn background(mut self, background: Background) -> Self {
        self.background = background;
        self
    }
}

impl<'a, T> Element<'a, T> for Space {
    fn size(&self) -> (Size, Size) {
        self.size
    }

    fn event(&mut self, _layout: Rectangle, _event: Event) -> Option<T> {
        None
    }

    fn render(&mut self, layout: Rectangle) -> Vec<Primitive> {
        match &self.background {
            &Background::Color(color) => vec![Primitive::DrawRect(layout, color)],
            &Background::Image(ref image, alpha) => vec![Primitive::DrawImage(image.clone(), layout, Color::white().with_alpha(alpha))],
            &Background::Patch(ref patch, alpha) => vec![Primitive::Draw9(patch.clone(), layout, Color::white().with_alpha(alpha))],
            &Background::None => vec![]
        }
    }
}
