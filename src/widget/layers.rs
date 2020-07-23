use crate::draw::Primitive;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::widget::{Context, IntoNode, Node, Widget};

/// Stack child widgets on top of each other, while only the topmost receives events.
pub struct Layers<'a, T, Id> {
    state: &'a mut State<Id>,
    layers: Vec<Layer<'a, T, Id>>,
    background: Option<Node<'a, T>>,
}

struct Layer<'a, T, Id> {
    node: Node<'a, T>,
    id: Id,
}

/// State for [`Layers`](struct.Layers.html)
#[derive(Default)]
pub struct State<Id> {
    cursor_x: f32,
    cursor_y: f32,
    order: Vec<Id>,
    background_focused: bool,
}

impl<'a, T: 'a, Id: 'a + Eq + Clone> Layers<'a, T, Id> {
    /// Construct new `Layers`
    pub fn new(state: &'a mut State<Id>) -> Self {
        Self {
            state,
            layers: Vec::new(),
            background: None,
        }
    }

    /// Construct new `Layers` with a background layer
    pub fn with_background(state: &'a mut State<Id>, background: impl IntoNode<'a, T>) -> Self {
        Self {
            state,
            layers: Vec::new(),
            background: Some(background.into_node()),
        }
    }

    /// Adds a widget
    pub fn push(mut self, id: Id, layer: impl IntoNode<'a, T>) -> Self {
        self.layers.push(Layer {
            node: layer.into_node(),
            id,
        });
        self
    }
}

impl<'a, T: 'a, Id: 'a> Widget<'a, T> for Layers<'a, T, Id> {
    fn widget(&self) -> &'static str {
        "layers"
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, T>)) {
        for background in self.background.iter_mut() {
            visitor(background);
        }
        for layer in self.layers.iter_mut() {
            visitor(&mut layer.node);
        }
    }

    fn focused(&self) -> bool {
        self.layers.iter().find(|layer| layer.node.focused()).is_some()
            || self.background.as_ref().map(|bg| bg.focused()).unwrap_or(false)
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        (style.width, style.height)
    }

    fn event(&mut self, layout: Rectangle, clip: Rectangle, _: &Stylesheet, event: Event, context: &mut Context<T>) {
        if self.background.as_ref().map(|bg| bg.focused()).unwrap_or(false) {
            self.background.as_mut().unwrap().event(layout, clip, event, context);
            return;
        }

        for layer in self.layers.iter_mut() {
            if layer.node.focused() {
                layer.node.event(layout, clip, event, context);
                return;
            }
        }

        match event {
            Event::Cursor(mut x, mut y) => {
                self.state.cursor_x = x;
                self.state.cursor_y = y;
                // make sure that hovering always works regardless of the active layer
                for layer in self.layers.iter_mut() {
                    layer.node.event(layout, clip, Event::Cursor(x, y), context);
                    if layer.node.hit(layout, clip, x, y) {
                        // I hate this hack, but this will stop layers hidden behind the current from being hovered
                        x = std::f32::INFINITY;
                        y = std::f32::INFINITY;
                    }
                }
                self.background.as_mut().map(|bg| bg.event(layout, clip, Event::Cursor(x, y), context));
                return;
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
                    if hit_index != 0 || self.state.background_focused {
                        self.state.background_focused = false;
                        if hit_index != 0 {
                            self.layers[0].node.event(layout, clip, event, context);
                        }
                        let rm = self.layers.remove(hit_index);
                        self.layers.insert(0, rm);
                        self.layers[0].node.event(layout, clip, Event::Cursor(x, y), context);
                    }
                } else if !self.state.background_focused {
                    self.state.background_focused = true;
                    if self.layers.len() > 0 {
                        self.layers[0].node.event(layout, clip, event, context);
                    }
                    self.background
                        .as_mut()
                        .map(|bg| bg.event(layout, clip, Event::Cursor(x, y), context));
                }
            }
            _ => (),
        }

        if self.state.background_focused {
            self.background
                .as_mut()
                .map(|bg| bg.event(layout, clip, event, context));
        } else if self.layers.len() > 0 {
            self.layers[0].node.event(layout, clip, event, context);
        }
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, _: &Stylesheet) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();
        if let Some(bg) = self.background.as_mut() {
            result.extend(bg.draw(layout, clip));
        }
        self.layers.iter_mut().rev().fold(result, |mut result, layer| {
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
