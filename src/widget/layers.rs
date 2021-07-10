use std::hash::Hash;

use crate::draw::Primitive;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::stylesheet::Stylesheet;
use crate::widget::{Context, Widget};

/// Stack child widgets on top of each other, while only the topmost receives events.
pub struct Layers<'a, T, Id> {
    layers: Vec<Layer<'a, T, Id>>,
    background: Option<Node<'a, T>>,
}

struct Layer<'a, T, Id> {
    node: Node<'a, T>,
    id: Id,
}

/// State for [`Layers`](struct.Layers.html)
pub struct State<Id> {
    cursor_x: f32,
    cursor_y: f32,
    order: Vec<Id>,
    background_focused: bool,
}

impl<'a, T: 'a, Id: 'a + Hash + Eq + Clone> Layers<'a, T, Id> {
    /// Construct new `Layers`
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            background: None,
        }
    }

    /// Construct new `Layers` with a background layer
    pub fn with_background(background: impl IntoNode<'a, T>) -> Self {
        Self {
            layers: Vec::new(),
            background: Some(background.into_node()),
        }
    }

    /// Adds a widget
    pub fn push(mut self, id: Id, layer: impl IntoNode<'a, T>) -> Self {
        self.layers.push(Layer {
            node: layer.into_node().with_key(&id),
            id,
        });
        self
    }

    /// Adds child widgets using an iterator
    pub fn extend<I: IntoIterator<Item = (Id, N)>, N: IntoNode<'a, T> + 'a>(mut self, iter: I) -> Self {
        self.layers.extend(iter.into_iter().map(|(id, layer)| Layer {
            node: layer.into_node().with_key(&id),
            id,
        }));
        self
    }

    fn ordered_layers<'b>(
        layers: &'b mut Vec<Layer<'a, T, Id>>,
        state: &mut State<Id>,
    ) -> Vec<&'b mut Layer<'a, T, Id>> {
        let mut result = layers.iter_mut().collect::<Vec<_>>();

        let mut index = 0;
        for order_id in state.order.iter() {
            if let Some(pos) = result.iter().position(|layer| layer.id.eq(order_id)) {
                result.swap(pos, index);
                index += 1;
            }
        }

        state.order.clear();
        state.order.extend(result.iter().map(|l| l.id.clone()));

        result
    }
}

impl<'a, T: 'a + Send, Id: 'static + Send + Sync + Hash + Eq + Clone> Widget<'a, T> for Layers<'a, T, Id> {
    type State = State<Id>;

    fn mount(&self) -> Self::State {
        State::default()
    }

    fn widget(&self) -> &'static str {
        "layers"
    }

    fn len(&self) -> usize {
        self.layers.len() + if self.background.is_some() { 1 } else { 0 }
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {
        for background in self.background.iter_mut() {
            visitor(&mut **background);
        }
        for layer in self.layers.iter_mut() {
            visitor(&mut *layer.node);
        }
    }

    fn size(&self, _: &State<Id>, style: &Stylesheet) -> (Size, Size) {
        (style.width, style.height)
    }

    fn focused(&self, _: &State<Id>) -> bool {
        self.layers.iter().any(|layer| layer.node.focused())
            || self.background.as_ref().map(|bg| bg.focused()).unwrap_or(false)
    }

    fn event(
        &mut self,
        state: &mut State<Id>,
        layout: Rectangle,
        clip: Rectangle,
        _: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        let mut ordered_layers = Layers::ordered_layers(&mut self.layers, &mut *state);

        if self.background.as_ref().map(|bg| bg.focused()).unwrap_or(false) {
            self.background.as_mut().unwrap().event(layout, clip, event, context);
            return;
        }

        for layer in ordered_layers.iter_mut() {
            if layer.node.focused() {
                layer.node.event(layout, clip, event, context);
                return;
            }
        }

        match event {
            Event::Cursor(mut x, mut y) => {
                state.cursor_x = x;
                state.cursor_y = y;
                // make sure that hovering always works regardless of the active layer
                for layer in ordered_layers.iter_mut() {
                    layer.node.event(layout, clip, Event::Cursor(x, y), context);
                    if layer.node.hit(layout, clip, x, y) {
                        // I hate this hack, but this will stop layers hidden behind the current from being hovered
                        x = f32::INFINITY;
                        y = f32::INFINITY;
                    }
                }
                if let Some(bg) = self.background.as_mut() {
                    bg.event(layout, clip, Event::Cursor(x, y), context)
                }
                return;
            }
            Event::Press(Key::LeftMouseButton) => {
                let x = state.cursor_x;
                let y = state.cursor_y;
                if let Some(hit_index) = ordered_layers.iter_mut().enumerate().find_map(move |(i, l)| {
                    if l.node.hit(layout, clip, x, y) {
                        Some(i)
                    } else {
                        None
                    }
                }) {
                    if hit_index != 0 || state.background_focused {
                        state.background_focused = false;
                        if hit_index != 0 {
                            ordered_layers[0].node.event(layout, clip, event, context);
                        }
                        let rm = ordered_layers.remove(hit_index);
                        ordered_layers.insert(0, rm);
                        ordered_layers[0].node.event(layout, clip, Event::Cursor(x, y), context);
                    }
                } else if !state.background_focused {
                    state.background_focused = true;
                    if !ordered_layers.is_empty() {
                        ordered_layers[0].node.event(layout, clip, event, context);
                    }
                    if let Some(bg) = self.background.as_mut() {
                        bg.event(layout, clip, Event::Cursor(x, y), context)
                    }
                }
            }
            _ => (),
        }

        if let Some(bg) = self.background.as_mut() {
            bg.event(layout, clip, event, context)
        }
        for layer in ordered_layers.iter_mut() {
            layer.node.event(layout, clip, event, context);
        }

        state.order.clear();
        state.order.extend(ordered_layers.into_iter().map(|l| l.id.clone()));
    }

    fn draw(
        &mut self,
        state: &mut State<Id>,
        layout: Rectangle,
        clip: Rectangle,
        _: &Stylesheet,
    ) -> Vec<Primitive<'a>> {
        let mut result = Vec::new();
        if let Some(bg) = self.background.as_mut() {
            result.extend(bg.draw(layout, clip));
        }
        Self::ordered_layers(&mut self.layers, &mut *state)
            .into_iter()
            .rev()
            .fold(result, |mut result, layer| {
                result.extend(layer.node.draw(layout, clip));
                result
            })
    }
}

impl<'a, T: 'a + Send, Id: 'static + Send + Sync + Hash + Eq + Clone> IntoNode<'a, T> for Layers<'a, T, Id> {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

impl<Id> Default for State<Id> {
    fn default() -> Self {
        Self {
            cursor_x: 0.0,
            cursor_y: 0.0,
            order: Vec::new(),
            background_focused: true,
        }
    }
}
