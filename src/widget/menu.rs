use crate::draw::Primitive;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::stylesheet::Stylesheet;
use crate::widget::{Context, IntoNode, Node, Widget};

use std::marker::PhantomData;

/// A (context) menu with nestable items
pub struct Menu<'a, T: 'a, S: AsMut<[MenuItem<'a, T>]>> {
    state: &'a mut State,
    items: S,
    marker: PhantomData<T>,
}

/// State for `Menu`
pub struct State {
    inner: InnerState,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

enum InnerState {
    Closed,
    Idle,
    HoverItem { index: usize },
    HoverSubMenu { index: usize, sub_state: Box<State> },
    Pressed { index: usize },
}

/// An item in `Menu`.
pub enum MenuItem<'a, T> {
    /// Item
    Item {
        /// The content of the item
        content: Node<'a, T>,
        /// Message to send when the item is clicked
        on_select: Option<T>,
    },
    /// Sub menu
    Menu {
        /// The content of the item
        content: Node<'a, T>,
        /// MenuItems to show when this item is hovered
        items: Vec<MenuItem<'a, T>>,
    },
}

impl<'a, T: 'a> Menu<'a, T, Vec<MenuItem<'a, T>>> {
    /// Construct a new `Menu`
    pub fn new(state: &'a mut State) -> Self {
        Self {
            items: Vec::new(),
            state,
            marker: PhantomData,
        }
    }

    /// Adds an item to the menu
    pub fn push(mut self, item: MenuItem<'a, T>) -> Self {
        self.items.push(item);
        self
    }

    /// Adds items using an iterator
    pub fn extend<I: IntoIterator<Item = MenuItem<'a, T>>>(mut self, iter: I) -> Self {
        self.items.extend(iter);
        self
    }
}

impl<'a, T: 'a, S: AsRef<[MenuItem<'a, T>]> + AsMut<[MenuItem<'a, T>]>> Menu<'a, T, S> {
    fn layout(
        &self,
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        viewport: Rectangle,
        style: &Stylesheet,
    ) -> Rectangle {
        let (width, height) = self.size(style);
        let width = match width {
            Size::Exact(width) => width,
            Size::Fill(_) => viewport.width() - right,
            Size::Shrink => 0.0,
        };
        let height = match height {
            Size::Exact(height) => height,
            Size::Fill(_) => viewport.height() - top,
            Size::Shrink => 0.0,
        };

        let (left, right) = if ((right + width) - viewport.width()).max(0.0) <= (-(left - width)).max(0.0) {
            (right, right + width)
        } else {
            (left - width, left)
        };

        let (top, bottom) = if ((top + height) - viewport.height()).max(0.0) <= (-(bottom - height)).max(0.0) {
            (top, top + height)
        } else {
            (bottom - height, bottom)
        };

        Rectangle {
            left,
            top,
            right,
            bottom,
        }
    }

    fn item_layouts(
        &mut self,
        layout: Rectangle,
        style: &Stylesheet,
    ) -> impl Iterator<Item = (&mut MenuItem<'a, T>, Rectangle)> {
        let layout = style.background.content_rect(layout, style.padding);
        let align = style.align_horizontal;
        let available_parts = self.items.as_mut().iter().map(|i| i.content().size().1.parts()).sum();
        let available_space = layout.height()
            - self
                .items
                .as_mut()
                .iter()
                .map(|i| i.content().size().1.min_size())
                .sum::<f32>();
        let mut cursor = 0.0;
        self.items.as_mut().iter_mut().map(move |item| {
            let (w, h) = item.content().size();
            let w = w.resolve(layout.width(), w.parts());
            let h = h
                .resolve(available_space, available_parts)
                .min(layout.height() - cursor);
            let x = align.resolve_start(w, layout.width());
            let y = cursor;

            cursor += h;
            (
                item,
                Rectangle::from_xywh(x, y, w, h).translate(layout.left, layout.top),
            )
        })
    }

    fn hover(
        &mut self,
        current: InnerState,
        x: f32,
        y: f32,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        context: &mut Context<T>,
    ) -> InnerState {
        context.redraw();
        if clip.point_inside(x, y) {
            let mut result = current;
            for (index, (item, item_layout)) in self.item_layouts(layout, style).enumerate() {
                let hover_rect = Rectangle {
                    left: layout.left + style.padding.left,
                    right: layout.right - style.padding.right,
                    top: item_layout.top,
                    bottom: item_layout.bottom,
                };
                if hover_rect.point_inside(x, y) {
                    result = match item {
                        &mut MenuItem::Item { .. } => InnerState::HoverItem { index },
                        &mut MenuItem::Menu { .. } => InnerState::HoverSubMenu {
                            index,
                            sub_state: Box::new(State {
                                inner: InnerState::Idle,
                                right: layout.right - style.padding.right - style.padding.left,
                                left: layout.left + style.padding.left + style.padding.right,
                                top: item_layout.top - style.padding.top,
                                bottom: item_layout.bottom + style.padding.bottom,
                            }),
                        },
                    };
                }
            }
            result
        } else {
            current
        }
    }
}

fn visit<'a, T>(items: &mut [MenuItem<'a, T>], visitor: &mut dyn FnMut(&mut Node<'a, T>)) {
    for item in items.iter_mut() {
        match item {
            &mut MenuItem::Item { ref mut content, .. } => visitor(content),
            &mut MenuItem::Menu {
                ref mut content,
                ref mut items,
            } => {
                visitor(content);
                visit(items.as_mut_slice(), visitor);
            }
        }
    }
}

impl<'a, T: 'a, S: AsRef<[MenuItem<'a, T>]> + AsMut<[MenuItem<'a, T>]>> Widget<'a, T> for Menu<'a, T, S> {
    fn widget(&self) -> &'static str {
        "menu"
    }

    fn len(&self) -> usize {
        self.items.as_ref().len()
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, T>)) {
        visit(self.items.as_mut(), visitor);
    }

    fn size(&self, style: &Stylesheet) -> (Size, Size) {
        let width = match style.width {
            Size::Shrink => {
                Size::Exact(
                    self.items
                        .as_ref()
                        .iter()
                        .fold(0.0, |size, child| match child.content().size().0 {
                            Size::Exact(child_size) => size.max(child_size),
                            _ => size,
                        }),
                )
            }
            other => other,
        };
        let height = match style.height {
            Size::Shrink => {
                Size::Exact(
                    self.items
                        .as_ref()
                        .iter()
                        .fold(0.0, |size, child| match child.content().size().1 {
                            Size::Exact(child_size) => size + child_size,
                            _ => size,
                        }),
                )
            }
            other => other,
        };

        style
            .background
            .resolve_size((style.width, style.height), (width, height), style.padding)
    }

    fn hit(&self, layout: Rectangle, clip: Rectangle, _style: &Stylesheet, x: f32, y: f32) -> bool {
        self.focused() && layout.point_inside(x, y) && clip.point_inside(x, y)
    }

    fn focused(&self) -> bool {
        if let InnerState::Closed = self.state.inner {
            false
        } else {
            true
        }
    }

    fn event(
        &mut self,
        viewport: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        if let InnerState::Closed = self.state.inner {
            return;
        }

        let layout = self.layout(
            self.state.left,
            self.state.right,
            self.state.top,
            self.state.bottom,
            viewport,
            style,
        );

        self.state.inner = match (event, std::mem::replace(&mut self.state.inner, InnerState::Idle)) {
            (Event::Cursor(x, y), InnerState::HoverSubMenu { index, sub_state }) => self.hover(
                InnerState::HoverSubMenu { index, sub_state },
                x,
                y,
                layout,
                clip,
                style,
                context,
            ),

            (Event::Cursor(x, y), InnerState::Pressed { index }) => {
                match self.hover(InnerState::Idle, x, y, layout, clip, style, context) {
                    InnerState::HoverItem { index: hover_index } if hover_index == index => {
                        InnerState::Pressed { index }
                    }
                    other => other,
                }
            }

            (Event::Cursor(x, y), _) => self.hover(InnerState::Idle, x, y, layout, clip, style, context),

            (Event::Press(Key::LeftMouseButton), InnerState::Idle) => {
                context.redraw();
                InnerState::Closed
            }

            (Event::Press(Key::LeftMouseButton), InnerState::HoverItem { index }) => {
                context.redraw();
                InnerState::Pressed { index }
            }

            (Event::Release(Key::LeftMouseButton), InnerState::Pressed { index }) => {
                context.redraw();
                if let Some(MenuItem::Item { on_select, .. }) = self.items.as_mut().get_mut(index) {
                    context.extend(on_select.take());
                }
                InnerState::Closed
            }

            (_, unhandled) => unhandled,
        };

        let mut close = false;

        if let InnerState::HoverSubMenu {
            index,
            ref mut sub_state,
        } = self.state.inner
        {
            if let Some(&mut MenuItem::Menu { ref mut items, .. }) = self.items.as_mut().get_mut(index) {
                unsafe {
                    let mut sub_menu = Menu {
                        items: items.as_mut_slice(),
                        state: (sub_state.as_mut() as *mut State).as_mut().unwrap(),
                        marker: PhantomData,
                    };
                    sub_menu.event(viewport, clip, style, event, context);
                }
            }

            if let InnerState::Closed = sub_state.as_mut().inner {
                close = true;
            }
        }

        if close {
            context.redraw();
            self.state.inner = InnerState::Closed;
        }
    }

    fn draw(&mut self, viewport: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>> {
        if let InnerState::Closed = self.state.inner {
            return Vec::new();
        }

        let mut result = Vec::new();
        result.push(Primitive::LayerUp);

        let layout = self.layout(
            self.state.left,
            self.state.right,
            self.state.top,
            self.state.bottom,
            viewport,
            style,
        );

        result.extend(style.background.render(layout));

        let hover_index = match self.state.inner {
            InnerState::Closed => None,
            InnerState::Idle => None,
            InnerState::HoverItem { index } => Some(index),
            InnerState::HoverSubMenu {
                index,
                ref mut sub_state,
            } => {
                if let Some(&mut MenuItem::Menu { ref mut items, .. }) = self.items.as_mut().get_mut(index) {
                    unsafe {
                        let mut sub_menu = Menu {
                            items: items.as_mut_slice(),
                            state: (sub_state.as_mut() as *mut State).as_mut().unwrap(),
                            marker: PhantomData,
                        };

                        result.extend(sub_menu.draw(viewport, clip, style));
                    }
                }
                Some(index)
            }
            InnerState::Pressed { index } => Some(index),
        };

        result =
            self.item_layouts(layout, style)
                .enumerate()
                .fold(result, |mut result, (index, (item, item_layout))| {
                    if hover_index == Some(index) {
                        result.push(Primitive::DrawRect(
                            Rectangle {
                                left: layout.left + style.padding.left,
                                right: layout.right - style.padding.right,
                                top: item_layout.top,
                                bottom: item_layout.bottom,
                            },
                            style.color,
                        ));
                    }
                    result.extend(item.content_mut().draw(item_layout, clip));
                    result
                });

        result.push(Primitive::LayerDown);
        result
    }
}

impl<'a, T: 'a, S: 'a + AsRef<[MenuItem<'a, T>]> + AsMut<[MenuItem<'a, T>]>> IntoNode<'a, T> for Menu<'a, T, S> {
    fn into_node(self) -> Node<'a, T> {
        Node::new(self)
    }
}

impl State {
    /// Opens the context menu if it's closed and `open == true`. The context menu will be positioned at (x,y) inside
    /// of it's layout rect.
    pub fn open(&mut self, x: f32, y: f32) {
        self.inner = match std::mem::replace(&mut self.inner, InnerState::Idle) {
            InnerState::Closed => {
                self.left = x;
                self.right = x;
                self.top = y;
                self.bottom = y;
                InnerState::Idle
            }
            open => open,
        };
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            inner: InnerState::Closed,
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }
}

impl<'a, T: 'a> MenuItem<'a, T> {
    fn content(&self) -> &Node<'a, T> {
        match self {
            &MenuItem::Item { ref content, .. } => content,
            &MenuItem::Menu { ref content, .. } => content,
        }
    }

    fn content_mut(&mut self) -> &mut Node<'a, T> {
        match self {
            &mut MenuItem::Item { ref mut content, .. } => content,
            &mut MenuItem::Menu { ref mut content, .. } => content,
        }
    }
}
