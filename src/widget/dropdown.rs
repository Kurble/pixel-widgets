use smallvec::smallvec;

use crate::draw::Primitive;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::style::{StyleState, Stylesheet};
use crate::widget::{Context, StateVec, Widget};

/// Pick an item from a dropdown box
pub struct Dropdown<'a, T, F> {
    items: Vec<Node<'a, T>>,
    default_selection: Option<usize>,
    on_select: F,
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

impl<'a, T: 'a, F> Dropdown<'a, T, F> {
    /// Set the default selected item
    pub fn default_selection(mut self, item_index: usize) -> Self {
        self.default_selection = Some(item_index);
        self
    }

    /// Sets the on_select callback for the dropdown, which is called when an item is selected
    pub fn on_select<N: Fn(usize) -> T>(self, on_select: N) -> Dropdown<'a, T, N> {
        Dropdown {
            items: self.items,
            default_selection: self.default_selection,
            on_select,
        }
    }

    /// Add an item to the dropdown.
    pub fn push(mut self, item: impl IntoNode<'a, T>) -> Self {
        self.items.push(item.into_node());
        self
    }

    /// Add multiple items to the dropdown.
    pub fn extend(mut self, items: impl IntoIterator<Item = impl IntoNode<'a, T>>) -> Self {
        self.items.extend(items.into_iter().map(IntoNode::into_node));
        self
    }
}

impl<'a, T: 'a> Default for Dropdown<'a, T, ()> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            default_selection: None,
            on_select: (),
        }
    }
}

impl<'a, T: Send + 'a, F: Send + Fn(usize) -> T> Widget<'a, T> for Dropdown<'a, T, F> {
    type State = State;

    fn mount(&self) -> Self::State {
        State {
            selected_item: self.default_selection.map(|i| i.min(self.items.len() - 1)),
            ..Default::default()
        }
    }

    fn widget(&self) -> &'static str {
        "dropdown"
    }

    fn state(&self, state: &State) -> StateVec {
        match state.inner {
            InnerState::Open { .. } | InnerState::Pressed { .. } => smallvec![StyleState::Open],
            InnerState::Idle if state.hovered => smallvec![StyleState::Hover],
            InnerState::Idle => StateVec::new(),
        }
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {
        for item in self.items.iter_mut() {
            visitor(&mut **item);
        }
    }

    fn size(&self, _: &State, style: &Stylesheet) -> (Size, Size) {
        let width = match style.width {
            Size::Shrink => Size::Exact(self.items.iter().fold(0.0f32, |size, item| match item.size().0 {
                Size::Exact(item_size) => size.max(item_size),
                _ => size,
            })),
            other => other,
        };
        let height = match style.height {
            Size::Shrink => Size::Exact(self.items.iter().fold(0.0f32, |size, item| match item.size().1 {
                Size::Exact(item_size) => size.max(item_size),
                _ => size,
            })),
            other => other,
        };

        style
            .background
            .resolve_size((style.width, style.height), (width, height), style.padding)
    }

    fn hit(&self, state: &State, layout: Rectangle, clip: Rectangle, _: &Stylesheet, x: f32, y: f32) -> bool {
        self.focused(state) || (layout.point_inside(x, y) && clip.point_inside(x, y))
    }

    fn focused(&self, state: &State) -> bool {
        matches!(state.inner, InnerState::Open { .. } | InnerState::Pressed { .. })
    }

    fn event(
        &mut self,
        state: &mut State,
        layout: Rectangle,
        clip: Rectangle,
        _: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        state.inner = match (event, std::mem::replace(&mut state.inner, InnerState::Idle)) {
            (Event::Cursor(x, y), InnerState::Idle) => {
                let hovered = layout.point_inside(x, y) && clip.point_inside(x, y);
                if hovered != state.hovered {
                    context.redraw();
                    state.hovered = hovered;
                }
                InnerState::Idle
            }

            (Event::Cursor(x, y), InnerState::Open { scroll, hover_item }) => {
                let hovered = x >= layout.left
                    && x < layout.right
                    && y >= layout.bottom
                    && y < layout.bottom + self.items.len() as f32 * layout.height();
                if hovered != state.hovered {
                    context.redraw();
                    state.hovered = hovered;
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
                if hovered != state.hovered {
                    context.redraw();
                    state.hovered = hovered;
                }

                let new_hover_item =
                    (((y - layout.bottom) / layout.height()).floor().max(0.0) as usize).min(self.items.len() - 1);

                if new_hover_item != hover_item || !state.hovered {
                    context.redraw();
                    InnerState::Open {
                        scroll,
                        hover_item: new_hover_item,
                    }
                } else {
                    InnerState::Pressed { scroll, hover_item }
                }
            }

            (Event::Press(Key::LeftMouseButton), InnerState::Idle) => {
                if state.hovered {
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
                if state.hovered {
                    InnerState::Pressed { scroll, hover_item }
                } else {
                    InnerState::Idle
                }
            }

            (Event::Release(Key::LeftMouseButton), InnerState::Pressed { hover_item, .. }) => {
                context.redraw();
                state.selected_item.replace(hover_item);
                context.push((self.on_select)(hover_item));
                InnerState::Idle
            }

            (_, state) => state,
        };
    }

    fn draw(
        &mut self,
        state: &mut State,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
    ) -> Vec<Primitive<'a>> {
        let content = style.background.content_rect(layout, style.padding);
        let focused = self.focused(state);

        let mut result = Vec::new();
        if focused {
            result.push(Primitive::LayerUp);
        }
        match state.inner {
            InnerState::Idle => {
                result.extend(style.background.render(layout));
                if let Some(selected) = state.selected_item {
                    result.extend(self.items[selected].draw(content, clip));
                }
            }
            InnerState::Open { hover_item, .. } | InnerState::Pressed { hover_item, .. } => {
                let padding = style.background.padding();
                let expanded = Rectangle {
                    left: layout.left,
                    top: layout.top,
                    right: layout.right,
                    bottom: layout.bottom + self.items.len() as f32 * layout.height() + padding.top + padding.bottom,
                };
                result.extend(style.background.render(expanded));
                for (index, item) in self.items.iter_mut().enumerate() {
                    if index == hover_item {
                        result.push(Primitive::DrawRect(
                            Rectangle {
                                left: layout.left + padding.left,
                                top: layout.top + (1 + index) as f32 * layout.height() + padding.top,
                                right: layout.right - padding.right,
                                bottom: layout.bottom + (1 + index) as f32 * layout.height() + padding.top,
                            },
                            style.color,
                        ));
                    }

                    let layout = Rectangle {
                        left: content.left + padding.left,
                        top: content.top + (1 + index) as f32 * layout.height() + padding.top,
                        right: content.right - padding.right,
                        bottom: content.bottom + (1 + index) as f32 * layout.height(),
                    };
                    result.extend(item.draw(layout, clip));
                }
            }
        }
        if focused {
            result.push(Primitive::LayerDown);
        }
        result
    }
}

impl<'a, T: 'a + Send, F: 'a + Send + Fn(usize) -> T> IntoNode<'a, T> for Dropdown<'a, T, F> {
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
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
