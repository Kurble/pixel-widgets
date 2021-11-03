use std::marker::PhantomData;

use crate::draw::Primitive;
use crate::event::{Event, Key};
use crate::layout::{Rectangle, Size};
use crate::node::{GenericNode, IntoNode, Node};
use crate::style::Stylesheet;
use crate::widget::{Context, Widget};

/// A (context) menu with nestable items
pub struct Menu<'a, T: 'a, S: AsMut<[MenuItem<'a, T>]>> {
    items: S,
    x: f32,
    y: f32,
    marker: PhantomData<&'a ()>,
    on_close: Option<T>,
}

/// State for `Menu`
pub struct MenuState {
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
    HoverSubMenu { index: usize, sub_state: Box<MenuState> },
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
    pub fn new(x: f32, y: f32, on_close: T) -> Self {
        Self {
            items: Vec::new(),
            x,
            y,
            marker: PhantomData,
            on_close: on_close.into(),
        }
    }

    /// Sets the (x, y) coordinates of the menu relative to it's parent.
    pub fn position(mut self, (x, y): (f32, f32)) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Sets the message to be posted when the menu is closed without selecting an item.
    pub fn on_close(mut self, on_close: T) -> Self {
        self.on_close = Some(on_close);
        self
    }

    /// Sets all of the items of the menu
    pub fn items(mut self, items: Vec<MenuItem<'a, T>>) -> Self {
        self.items = items;
        self
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

impl<'a, T: 'a> Default for Menu<'a, T, Vec<MenuItem<'a, T>>> {
    fn default() -> Self {
        Self {
            items: vec![],
            x: 0.0,
            y: 0.0,
            marker: PhantomData,
            on_close: None,
        }
    }
}

impl<'a, T: 'a + Send, S: Send + AsRef<[MenuItem<'a, T>]> + AsMut<[MenuItem<'a, T>]>> Menu<'a, T, S> {
    fn layout(&self, state: &MenuState, viewport: Rectangle, style: &Stylesheet) -> Rectangle {
        let (width, height) = self.size(state, style);
        let width = match width {
            Size::Exact(width) => width,
            Size::Fill(_) => viewport.width() - state.right,
            Size::Shrink => 0.0,
        };
        let height = match height {
            Size::Exact(height) => height,
            Size::Fill(_) => viewport.height() - state.top,
            Size::Shrink => 0.0,
        };

        let (left, right) = if ((state.right + width) - viewport.width()).max(0.0) <= (-(state.left - width)).max(0.0) {
            (state.right, state.right + width)
        } else {
            (state.left - width, state.left)
        };

        let (top, bottom) =
            if ((state.top + height) - viewport.height()).max(0.0) <= (-(state.bottom - height)).max(0.0) {
                (state.top, state.top + height)
            } else {
                (state.bottom - height, state.bottom)
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

    #[allow(clippy::too_many_arguments)]
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
                        MenuItem::Item { .. } => InnerState::HoverItem { index },
                        MenuItem::Menu { .. } => InnerState::HoverSubMenu {
                            index,
                            sub_state: Box::new(MenuState {
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

fn visit<'a, T>(items: &mut [MenuItem<'a, T>], visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {
    for item in items.iter_mut() {
        match item {
            MenuItem::Item { ref mut content, .. } => visitor(&mut **content),
            MenuItem::Menu {
                ref mut content,
                ref mut items,
            } => {
                visitor(&mut **content);
                visit(items.as_mut_slice(), visitor);
            }
        }
    }
}

impl<'a, T: 'a + Send, S: Send + AsRef<[MenuItem<'a, T>]> + AsMut<[MenuItem<'a, T>]>> Widget<'a, T> for Menu<'a, T, S> {
    type State = MenuState;

    fn mount(&self) -> Self::State {
        MenuState {
            inner: InnerState::Idle,
            left: self.x,
            right: self.x,
            top: self.y,
            bottom: self.y,
        }
    }

    fn widget(&self) -> &'static str {
        "menu"
    }

    fn len(&self) -> usize {
        self.items.as_ref().len()
    }

    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, T>)) {
        visit(self.items.as_mut(), visitor);
    }

    fn size(&self, _: &MenuState, style: &Stylesheet) -> (Size, Size) {
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

    fn hit(&self, state: &MenuState, layout: Rectangle, clip: Rectangle, _style: &Stylesheet, x: f32, y: f32) -> bool {
        self.focused(state) && layout.point_inside(x, y) && clip.point_inside(x, y)
    }

    fn focused(&self, state: &MenuState) -> bool {
        !matches!(state.inner, InnerState::Closed)
    }

    fn event(
        &mut self,
        state: &mut MenuState,
        viewport: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
        event: Event,
        context: &mut Context<T>,
    ) {
        if let InnerState::Closed = state.inner {
            return;
        }

        let layout = self.layout(state, viewport, style);

        state.inner = match (event, std::mem::replace(&mut state.inner, InnerState::Idle)) {
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
                context.extend(self.on_close.take());
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
                context.extend(self.on_close.take());
                InnerState::Closed
            }

            (_, unhandled) => unhandled,
        };

        let mut close = false;

        if let InnerState::HoverSubMenu {
            index,
            ref mut sub_state,
        } = state.inner
        {
            if let Some(&mut MenuItem::Menu { ref mut items, .. }) = self.items.as_mut().get_mut(index) {
                let mut sub_menu = Menu {
                    items: items.as_mut_slice(),
                    x: 0.0,
                    y: 0.0,
                    marker: PhantomData,
                    on_close: None,
                };
                sub_menu.event(&mut *sub_state, viewport, clip, style, event, context);
            }

            if let InnerState::Closed = sub_state.as_mut().inner {
                close = true;
            }
        }

        if close {
            context.redraw();
            state.inner = InnerState::Closed;
            context.extend(self.on_close.take());
        }
    }

    fn draw(
        &mut self,
        state: &mut MenuState,
        viewport: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
    ) -> Vec<Primitive<'a>> {
        if let InnerState::Closed = state.inner {
            return Vec::new();
        }

        let mut result = vec![Primitive::LayerUp];

        let layout = self.layout(state, viewport, style);

        result.extend(style.background.render(layout));

        let hover_index = match state.inner {
            InnerState::Closed => None,
            InnerState::Idle => None,
            InnerState::HoverItem { index } => Some(index),
            InnerState::HoverSubMenu {
                index,
                ref mut sub_state,
            } => {
                if let Some(&mut MenuItem::Menu { ref mut items, .. }) = self.items.as_mut().get_mut(index) {
                    let mut sub_menu = Menu {
                        items: items.as_mut_slice(),
                        x: 0.0,
                        y: 0.0,
                        marker: PhantomData,
                        on_close: None,
                    };

                    result.extend(sub_menu.draw(&mut *sub_state, viewport, clip, style));
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

impl<'a, T: 'a + Send, S: 'a + Send + AsRef<[MenuItem<'a, T>]> + AsMut<[MenuItem<'a, T>]>> IntoNode<'a, T>
    for Menu<'a, T, S>
{
    fn into_node(self) -> Node<'a, T> {
        Node::from_widget(self)
    }
}

impl MenuState {
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

impl Default for MenuState {
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
    /// Construct a new `MenuItem` of the item type,
    ///  with a content node and a message to be posted when this item is selected.
    pub fn item(content: impl IntoNode<'a, T>, on_select: impl Into<Option<T>>) -> Self {
        Self::Item {
            content: content.into_node(),
            on_select: on_select.into(),
        }
    }

    /// Construct a new `MenuItem` of the sub menu type,
    ///  with a content node. Sub menu items can be added to the returned value.
    pub fn menu(content: impl IntoNode<'a, T>) -> Self {
        Self::Menu {
            content: content.into_node(),
            items: Vec::new(),
        }
    }

    /// Adds a sub `MenuItem` to this menu.
    /// Will panic if this is an item instead of a submenu.
    pub fn push(self, item: Self) -> Self {
        if let Self::Menu { content, mut items } = self {
            items.push(item);
            Self::Menu { content, items }
        } else {
            panic!("push may only be called on menu items")
        }
    }

    /// Adds multiple sub `MenuItem`s to this menu.
    /// Will panic if this is an item instead of a submenu.
    pub fn extend(self, new_items: impl IntoIterator<Item = Self>) -> Self {
        if let Self::Menu { content, mut items } = self {
            items.extend(new_items.into_iter());
            Self::Menu { content, items }
        } else {
            panic!("extend may only be called on menu items")
        }
    }

    fn content(&self) -> &Node<'a, T> {
        match self {
            MenuItem::Item { ref content, .. } => content,
            MenuItem::Menu { ref content, .. } => content,
        }
    }

    fn content_mut(&mut self) -> &mut Node<'a, T> {
        match self {
            MenuItem::Item { ref mut content, .. } => content,
            MenuItem::Menu { ref mut content, .. } => content,
        }
    }
}
