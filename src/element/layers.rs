use crate::draw::Primitive;
use crate::element::{Element, IntoNode, Node, Stylable};
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::Context;

pub struct Layers<'a, T, Id> {
    state: &'a mut State<Id>,
    layers: Vec<Layer<'a, T, Id>>,
}

struct Layer<'a, T, Id> {
    node: Node<'a, T>,
    id: Id,
}

#[derive(Default)]
pub struct State<Id> {
    cursor_x: f32,
    cursor_y: f32,
    order: Vec<Id>,
}

impl<'a, T: 'a, Id: 'a + Eq + Clone> Layers<'a, T, Id> {
    pub fn new(state: &'a mut State<Id>) -> Self {
        Self {
            state,
            layers: Vec::new(),
        }
    }

    pub fn push(mut self, id: Id, layer: impl IntoNode<'a, T>) -> Self {
        self.layers.push(Layer {
            node: layer.into_node(),
            id,
        });
        self
    }
}

impl<'a, T: 'a, Id: 'a> Element<'a, T> for Layers<'a, T, Id> {
    fn element(&self) -> &'static str {
        "layers"
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn Stylable<'a>)) {
        for layer in self.layers.iter_mut() {
            visitor(&mut layer.node);
        }
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        (style.width, style.height)
    }

    fn event(&mut self, layout: Rectangle, clip: Rectangle, _: &Stylesheet, event: Event, context: &mut Context<T>) {
        match event {
            Event::Cursor(x, y) => {
                self.state.cursor_x = x;
                self.state.cursor_y = y;
            }
            Event::Press(Key::LeftMouseButton) => {
                let x = self.state.cursor_x;
                let y = self.state.cursor_y;
                if let Some(hit_index) =
                    self.layers
                        .iter()
                        .enumerate()
                        .find_map(move |(i, l)| if l.node.hit(layout, clip, x, y) { Some(i) } else { None })
                {
                    if hit_index != 0 {
                        let rm = self.layers.remove(hit_index);
                        self.layers[0].node.event(layout, clip, event, context);
                        self.layers.insert(0, rm);
                        self.layers[0].node.event(layout, clip, Event::Cursor(x, y), context);
                    }
                }
            }
            _ => (),
        }

        self.layers[0].node.event(layout, clip, event, context);
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, _: &Stylesheet) -> Vec<Primitive<'a>> {
        self.layers.iter_mut().rev().fold(Vec::new(), |mut result, layer| {
            result.extend(layer.node.draw(layout, clip));
            result
        })
    }
}

impl<'a, T: 'a, Id: 'a + Eq + Clone> IntoNode<'a, T> for Layers<'a, T, Id> {
    fn into_node(mut self) -> Node<'a, T> {
        let mut index = 0;
        for order_id in self.state.order.iter() {
            if let Some(pos) = self.layers.iter().position(|layer| layer.id.eq(order_id)) {
                self.layers.swap(pos, index);
                index += 1;
            }
        }

        Node::new(self)
    }
}

impl<'a, T, Id> Drop for Layers<'a, T, Id> {
    fn drop(&mut self) {
        self.state.order.clear();
        self.state.order.extend(self.layers.drain(..).map(|layer| layer.id));
    }
}
