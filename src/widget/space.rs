use crate::draw::*;
use crate::widget::*;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;

/// Empty widget
pub struct Space;

impl<'a, T> Widget<'a, T> for Space {
    fn widget(&self) -> &'static str {
        "space"
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut Node<'a, T>)) {}

    fn size(&self, stylesheet: &Stylesheet) -> (Size, Size) {
        (stylesheet.width, stylesheet.height)
    }

    fn draw(&mut self, layout: Rectangle, _clip: Rectangle, stylesheet: &Stylesheet) -> Vec<Primitive<'a>> {
        stylesheet.background.render(layout).into_iter().collect()
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Space {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}
