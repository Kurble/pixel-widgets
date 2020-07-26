use crate::draw::Primitive;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::widget::{Context, IntoNode, Node, Widget};

/// Pick an item from a dropdown box
pub struct Dropdown<'a, T> {
    state: &'a mut State,
    items: Vec<Item<'a, T>>,
}

struct Item<'a, T> {
    node: Node<'a, T>,
    on_select: Option<T>,
}

/// State for [`Dropdown`](struct.Dropdown.html).
pub struct State {
    selected_item: Option<usize>,
    hovered: bool,
    inner: InnerState,
}

enum InnerState {
    Idle,
    Open { scroll: f32, hover_item: usize },
    Pressed { scroll: f32, hover_item: usize },
}

impl<'a, T: 'a> Dropdown<'a, T> {
    /// Construct a new dropdown
    pub fn new(state: &'a mut State) -> Self {
        Self {
            state,
            items: Vec::new(),
        }
    }

    /// Add an item to the dropdown.
    pub fn push(mut self, item: impl IntoNode<'a, T>, on_select: T) -> Self {
        self.items.push(Item {
            node: item.into_node(),
            on_select: Some(on_select),
        });
        self
    }

    /// Add multiple items to the dropdown.
    pub fn extend(mut self, items: impl IntoIterator<Item = (impl IntoNode<'a, T>, T)>) -> Self {
        self.items.extend(items.into_iter().map(|(item, on_select)| Item {
            node: item.into_node(),
            on_select: Some(on_select),
        }));
        self
    }
}

impl<'a, T: 'a> Widget<'a, T> for Dropdown<'a, T> {
    fn widget(&self) -> &'static str {
        "dropdown"
    }

    fn state(&self) -> &'static str {
        match self.state.inner {
            InnerState::Open { .. } | InnerState::Pressed { .. } => "open",
            InnerState::Idle if self.state.hovered => "hover",
            InnerState::Idle => "",
        }
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, T>)) {
        for item in self.items.iter_mut() {
            visitor(&mut item.node);
        }
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        let width = match style.width {
            Size::Shrink => Size::Exact(self.items.iter().fold(0.0f32, |size, item| match item.node.size().0 {
                Size::Exact(item_size) => size.max(item_size),
                _ => size,
            })),
            other => other,
        };
        let height = match style.height {
            Size::Shrink => Size::Exact(self.items.iter().fold(0.0f32, |size, item| match item.node.size().1 {
                Size::Exact(item_size) => size.max(item_size),
                _ => size,
            })),
            other => other,
        };

        style.background.resolve_size((style.width, style.height), (width, height), style.padding)
    }

    fn hit(&self, layout: Rectangle, clip: Rectangle, _: &Stylesheet, x: f32, y: f32) -> bool {
        self.focused() || (layout.point_inside(x, y) && clip.point_inside(x, y))
    }

    fn focused(&self) -> bool {
        if let InnerState::Open { .. } | InnerState::Pressed { .. } = self.state.inner {
            true
        } else {
            false
        }
    }

    fn event(&mut self, layout: Rectangle, clip: Rectangle, _: &Stylesheet, event: Event, context: &mut Context<T>) {
        self.state.inner = match (event, std::mem::replace(&mut self.state.inner, InnerState::Idle)) {
            (Event::Cursor(x, y), InnerState::Idle) => {
                let hovered = layout.point_inside(x, y) && clip.point_inside(x, y);
                if hovered != self.state.hovered {
                    context.redraw();
                    self.state.hovered = hovered;
                }
                InnerState::Idle
            }

            (Event::Cursor(x, y), InnerState::Open { scroll, hover_item }) => {
                let hovered = x >= layout.left
                    && x < layout.right
                    && y >= layout.bottom
                    && y < layout.bottom + self.items.len() as f32 * layout.height();
                if hovered != self.state.hovered {
                    context.redraw();
                    self.state.hovered = hovered;
                }

                let new_hover_item =
                    (((y - layout.bottom) / layout.height()).floor().max(0.0) as usize).min(self.items.len() - 1);

                if new_hover_item != hover_item {
                    context.redraw();
                    InnerState::Open {
                        scroll,
                        hover_item: new_hover_item,
                    }
                } else {
                    InnerState::Open { scroll, hover_item }
                }
            }

            (Event::Cursor(x, y), InnerState::Pressed { scroll, hover_item }) => {
                let hovered = x >= layout.left
                    && x < layout.right
                    && y >= layout.bottom
                    && y < layout.bottom + self.items.len() as f32 * layout.height();
                if hovered != self.state.hovered {
                    context.redraw();
                    self.state.hovered = hovered;
                }

                let new_hover_item =
                    (((y - layout.bottom) / layout.height()).floor().max(0.0) as usize).min(self.items.len() - 1);

                if new_hover_item != hover_item || !self.state.hovered
                {
                    context.redraw();
                    InnerState::Open { scroll, hover_item: new_hover_item }
                } else {
                    InnerState::Pressed { scroll, hover_item }
                }
            }

            (Event::Press(Key::LeftMouseButton), InnerState::Idle) => {
                if self.state.hovered {
                    context.redraw();
                    InnerState::Open {
                        scroll: 0.0,
                        hover_item: 0,
                    }
                } else {
                    InnerState::Idle
                }
            }

            (Event::Press(Key::LeftMouseButton), InnerState::Open { scroll, hover_item }) => {
                context.redraw();
                if self.state.hovered {
                    InnerState::Pressed { scroll, hover_item }
                } else {
                    InnerState::Idle
                }
            }

            (Event::Release(Key::LeftMouseButton), InnerState::Pressed { hover_item, .. }) => {
                context.redraw();
                self.state.selected_item.replace(hover_item);
                context.extend(self.items[hover_item].on_select.take());
                InnerState::Idle
            }

            (_, state) => state,
        };
    }

    fn draw(&mut self, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        let content = style.background.content_rect(layout, style.padding);
        let focused = self.focused();

        let mut result = Vec::new();
        if focused {
            result.push(Primitive::LayerUp);
        }
        match self.state.inner {
            InnerState::Idle => {
                result.extend(style.background.render(layout));
                if let Some(selected) = self.state.selected_item {
                    result.extend(self.items[selected].node.draw(content, clip));
                }
            }
            InnerState::Open { hover_item, .. } | InnerState::Pressed { hover_item, .. } => {
                let expanded = Rectangle {
                    left: layout.left,
                    top: layout.top,
                    right: layout.right,
                    bottom: layout.bottom + self.items.len() as f32 * layout.height(),
                };
                result.extend(style.background.render(expanded));
                for (index, item) in self.items.iter_mut().enumerate() {
                    if index == hover_item {
                        result.push(Primitive::DrawRect(
                            Rectangle {
                                left: layout.left,
                                top: layout.top + (1 + index) as f32 * layout.height(),
                                right: layout.right,
                                bottom: layout.bottom + (1 + index) as f32 * layout.height(),
                            },
                            style.color,
                        ));
                    }

                    let layout = Rectangle {
                        left: content.left,
                        top: content.top + (1 + index) as f32 * layout.height(),
                        right: content.right,
                        bottom: content.bottom + (1 + index) as f32 * layout.height(),
                    };
                    result.extend(item.node.draw(layout, clip));
                }
            }
        }
        if focused {
            result.push(Primitive::LayerDown);
        }
        result
    }
}

impl<'a, T: 'a> IntoNode<'a, T> for Dropdown<'a, T> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            selected_item: None,
            hovered: false,
            inner: InnerState::Idle,
        }
    }
}
