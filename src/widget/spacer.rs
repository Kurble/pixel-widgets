use crate::draw::*;
use crate::layout::{Rectangle, Size};
use crate::node::{IntoNode, Node};
use crate::style::Stylesheet;
use crate::widget::*;

/// Empty widget. Default size is (fill(1), fill(1)).
#[derive(Default)]
pub struct Spacer;

impl<'a, T> Widget<'a, T> for Spacer {
    type State = ();

    fn mount(&self) {}

    fn widget(&self) -> &'static str {
        "spacer"
    }

    fn len(&self) -> usize {
        0
    }

    fn visit_children(&mut self, _: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {}

    fn size(&self, _: &(), style: &Stylesheet) -> (Size, Size) {
        style.background.resolve_size(
            (style.width, style.height),
            (Size::Exact(0.0), Size::Exact(0.0)),
            style.padding,
        )
    }

    fn hit(
        &self,
        _state: &Self::State,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        x: f32,
        y: f32,
        _recursive: bool,
    ) -> bool {
        layout.point_inside(x, y) && clip.point_inside(x, y) && style.background.is_solid()
    }

    fn draw(&mut self, _: &mut (), layout: Rectangle, _clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        style.background.render(layout).into_iter().collect()
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Spacer {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}
