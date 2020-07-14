use crate::draw::*;
use crate::element::*;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;

pub struct Space;

impl<'a, T> Element<'a, T> for Space {
    fn element(&self) -> &'static str {
        "space"
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn Stylable<'a>)) {}

    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        (stylesheet.width, stylesheet.height)
    }

    fn event(&mut self, _: Rectangle, _: Rectangle, _: &Stylesheet, _: Event) -> Option<T> {
        None
    }

    fn render(&mut self, layout: Rectangle, _clip: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        stylesheet.background.render(layout).into_iter().collect()
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Space {}
