use crate::draw::*;
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::widget::*;

/// Empty widget
pub struct Space;

impl<'a, T> Widget<'a, T> for Space {
    fn widget(&self) -> &'static str {
        "space"
    }

    fn len(&self) -> usize { 0 }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut Node<'a, T>)) {}

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        style.background.resolve_size(
            (style.width, style.height),
            (Size::Exact(0.0), Size::Exact(0.0)),
            style.padding,
        )
    }

    fn draw(&mut self, layout: Rectangle, _clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        style.background.render(layout).into_iter().collect()
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Space {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}
