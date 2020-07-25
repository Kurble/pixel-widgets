//!
//! Widgets in pixel-widgets are defined using the [`Widget`](trait.Widget.html) trait.
//! You can choose to implement widgets yourself, or you can use the built in widgets that come with pixel-widgets:
//! - [`Button`](button/struct.Button.html)
//! - [`Toggle`](toggle/struct.Toggle.html)
//! - [`Column`](column/struct.Column.html)
//! - [`Row`](row/struct.Row.html)
//! - [`Text`](text/struct.Text.html)
//! - [`Space`](space/struct.Space.html)
//! - [`Input`](input/struct.Input.html)
//! - [`Scroll`](scroll/struct.Scroll.html)
//! - [`Layers`](layers/struct.Layers.html)
//! - [`Window`](window/struct.Window.html)
//!
//! Since pixel-widgets rebuilds the whole ui every time the [`Model`](../trait.Model.html) is modified,
//! most widgets need to keep track of some kind of state across rebuilds. You can manually supply these state
//! objects in your [`view`](../trait.Model.html#tymethod.view) implementation, or you can use a
//! [`ManagedState`](../tracker/struct.ManagedState.html), which tracks state for your widgets using user defined ids.
//!
//! When implementing custom widgets, you need to make sure that the custom widgets do not remember absolute layouts.
//! Widgets like [`Scroll`](scroll/struct.Scroll.html) can change the layout without needing a rebuild of the ui.
use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

use crate::bitset::BitSet;
use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::*;
use crate::stylesheet::*;

pub use self::button::Button;
pub use self::column::Column;
pub use self::dropdown::Dropdown;
pub use self::input::Input;
pub use self::layers::Layers;
pub use self::row::Row;
pub use self::scroll::Scroll;
pub use self::space::Space;
pub use self::text::Text;
pub use self::toggle::Toggle;
pub use self::window::Window;

/// A clickable button
pub mod button;
/// Layout child widgets vertically
pub mod column;
/// Pick an item from a dropdown box
pub mod dropdown;
/// Editable text input
pub mod input;
/// Stack child widgets on top of each other, while only the topmost receives events.
pub mod layers;
/// Layout child widgets horizontally
pub mod row;
/// View a small section of larger widget, with scrollbars.
pub mod scroll;
/// Empty widget
pub mod space;
/// Widget that renders a paragraph of text.
pub mod text;
/// A clickable button that toggles some `bool`.
pub mod toggle;
/// A window with a title and a content widget that can be moved by dragging the title.
pub mod window;

/// A user interface widget.
pub trait Widget<'a, Message> {
    /// The name of this widget, used to identify widgets of this type in stylesheets.
    fn widget(&self) -> &'static str;

    /// The state of this widget, used for computing the style. Usually an empty string, "hover", "pressed", etc.
    fn state(&self) -> &'static str {
        ""
    }

    /// Applies a visitor to all childs of the widget. If an widget fails to visit it's children, the children won't
    /// be able to resolve their stylesheet, resulting in a panic when calling [`size`](struct.Node.html#method.size),
    /// [`hit`](struct.Node.html#method.hit), [`event`](struct.Node.html#method.event) or
    /// [`draw`](struct.Node.html#method.draw).
    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, Message>));

    /// Returns the `(width, height)` of this widget.
    /// The extents are defined as a [`Size`](../layout/struct.Size.html),
    /// which will later be resolved to actual dimensions.
    fn size(&self, style: &Stylesheet) -> (Size, Size);

    /// Perform a hit detect on the widget. Most widgets are fine with the default implementation, but some
    /// widgets (like [`Window`](window/struct.Window.html) need to report a _miss_ (`false`) even when the queried
    /// position is within their layout.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the widget
    /// - `clip`: a clipping rect for mouse events. Mouse events outside of this rect should be considered invalid,
    /// such as with [`Scroll`](scroll/struct.Scroll.html), where the widget would not be visible outside of the
    /// currently visible rect.
    /// - `x`: x mouse coordinate being queried
    /// - `y`: y mouse coordinate being queried
    fn hit(&self, layout: Rectangle, clip: Rectangle, _style: &Stylesheet, x: f32, y: f32) -> bool {
        layout.point_inside(x, y) && clip.point_inside(x, y)
    }

    /// Test the widget for focus exclusivity.
    /// If the widget or one of it's descendants is in an exclusive focus state, this function should return `true`.
    /// In all other cases, it should return `false`. When a widget is in an exclusive focus state it is
    /// the only widget that is allowed to receive events in [`event`](#tymethod.event).
    /// Widgets that intended to use this behaviour are modal dialogs, dropdown boxes, context menu's, etc.
    fn focused(&self) -> bool {
        false
    }

    /// Handle an event. If an event changes the graphical appearance of an `Widget`,
    /// [`redraw`](struct.Context.html#method.redraw) should be called to let the [`Ui`](../struct.Ui.html) know that
    /// the ui should be redrawn.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the widget
    /// - `clip`: a clipping rect for mouse events. Mouse events outside of this rect should be considered invalid,
    /// such as with [`Scroll`](scroll/struct.Scroll.html), where the widget would not be visible outside of the
    /// currently visible rect.
    /// - `event`: the event that needs to be handled
    /// - `context`: context for submitting messages and requesting redraws of the ui.
    fn event(
        &mut self,
        _layout: Rectangle,
        _clip: Rectangle,
        _style: &Stylesheet,
        _event: Event,
        _context: &mut Context<Message>,
    ) {
    }

    /// Draw the widget. Returns a list of [`Primitive`s](../draw/enum.Primitive.html) that should be drawn.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the widget
    /// - `clip`: a clipping rect for use with [`Primitive::PushClip`](../draw/enum.Primitive.html#variant.PushClip).
    fn draw(&mut self, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>>;
}

/// Convert to a generic widget. All widgets should implement this trait. It is also implemented by `Node` itself,
/// which simply returns self.
pub trait IntoNode<'a, Message: 'a>: 'a + Sized {
    /// Perform the conversion.
    fn into_node(self) -> Node<'a, Message>;

    /// Convenience function that converts to a node and then adds a style class to the `Node`.
    fn class(self, class: &'a str) -> Node<'a, Message> {
        self.into_node().class(class)
    }
}

/// Generic ui widget.
pub struct Node<'a, Message> {
    widget: Box<dyn Widget<'a, Message> + 'a>,
    size: Cell<Option<(Size, Size)>>,
    focused: Cell<Option<bool>>,
    style: Option<Rc<Style>>,
    selector_matches: BitSet,
    stylesheet: Option<Rc<Stylesheet>>,
    class: Option<&'a str>,
    state: Option<&'static str>,
}

/// Context for posting messages and requesting redraws of the ui.
pub struct Context<Message> {
    redraw: bool,
    messages: Vec<Message>,
}

impl<'a, Message> Node<'a, Message> {
    /// Construct a new `Node` from an [`Widget`](trait.Widget.html).
    pub fn new<T: 'a + Widget<'a, Message>>(widget: T) -> Self {
        Node {
            widget: Box::new(widget),
            size: Cell::new(None),
            focused: Cell::new(None),
            style: None,
            selector_matches: BitSet::new(),
            stylesheet: None,
            class: None,
            state: None,
        }
    }

    /// Sets the style class
    pub fn class(mut self, class: &'a str) -> Self {
        self.class = Some(class);
        self
    }

    pub(crate) fn style(&mut self, query: &mut Query) {
        // remember style
        self.style = Some(query.style.clone());

        // resolve own stylesheet
        self.selector_matches = query.match_widget(
            self.widget.widget(),
            self.class.unwrap_or(""),
            self.widget.state(),
            false,
        );
        self.stylesheet.replace(query.style.get(&self.selector_matches));
        self.state = Some(self.widget.state());

        // resolve children style
        query.ancestors.push(self.selector_matches.clone());
        let own_siblings = std::mem::replace(&mut query.siblings, Vec::new());
        self.widget.visit_children(&mut |child| child.style(&mut *query));
        std::mem::replace(&mut query.siblings, own_siblings);
        query.siblings.push(query.ancestors.pop().unwrap());
    }

    /// Returns the `(width, height)` of this widget.
    /// The extents are defined as a [`Size`](../layout/struct.Size.html),
    /// which will later be resolved to actual dimensions.
    pub fn size(&self) -> (Size, Size) {
        if self.size.get().is_none() {
            let stylesheet = self.stylesheet.as_ref().unwrap().deref();
            self.size.replace(Some(self.widget.size(stylesheet)));
        }
        self.size.get().unwrap()
    }

    /// Perform a hit detect on the widget. Most widgets are fine with the default implementation, but some
    /// widgets (like [`Window`](window/struct.Window.html) need to report a _miss_ (`false`) even when the queried
    /// position is within their layout.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the widget
    /// - `clip`: a clipping rect for mouse events. Mouse events outside of this rect should be considered invalid,
    /// such as with [`Scroll`](scroll/struct.Scroll.html), where the widget would not be visible outside of the
    /// currently visible rect.
    /// - `x`: x mouse coordinate being queried
    /// - `y`: y mouse coordinate being queried
    pub fn hit(&self, layout: Rectangle, clip: Rectangle, x: f32, y: f32) -> bool {
        let stylesheet = self.stylesheet.as_ref().unwrap().deref();
        self.widget.hit(layout, clip, stylesheet, x, y)
    }

    /// Test the widget for focus exclusivity.
    /// If the widget or one of it's descendants is in an exclusive focus state, this function should return `true`.
    /// In all other cases, it should return `false`. When a widget is in an exclusive focus state it is
    /// the only widget that is allowed to receive events in [`event`](#tymethod.event).
    /// Widgets that intended to use this behaviour are modal dialogs, dropdown boxes, context menu's, etc.
    pub fn focused(&self) -> bool {
        if self.focused.get().is_none() {
            self.focused.replace(Some(self.widget.focused()));
        }
        self.focused.get().unwrap()
    }

    /// Handle an event.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the widget
    /// - `clip`: a clipping rect for mouse events. Mouse events outside of this rect should be considered invalid,
    /// such as with [`Scroll`](scroll/struct.Scroll.html), where the widget would not be visible outside of the
    /// currently visible rect.
    /// - `event`: the event that needs to be handled
    /// - `context`: context for submitting messages and requesting redraws of the ui.
    pub fn event(&mut self, layout: Rectangle, clip: Rectangle, event: Event, context: &mut Context<Message>) {
        let stylesheet = self.stylesheet.as_ref().unwrap().deref();
        self.widget.event(layout, clip, stylesheet, event, context);
        self.focused.replace(Some(self.widget.focused()));

        if Some(self.widget.state()) != self.state {
            self.state = Some(self.widget.state());

            // trigger a restyle for this widget and it's children
            let new_style = self.style.as_ref().unwrap().restyle(
                &self.selector_matches,
                self.widget.widget(),
                self.class.unwrap_or(""),
                self.widget.state(),
            );

            if new_style != self.selector_matches {
                self.stylesheet.replace(self.style.as_ref().unwrap().get(&self.selector_matches));

                // todo: restyle children here

                self.selector_matches = new_style;
            }
        }
    }

    /// Draw the widget. Returns a list of [`Primitive`s](../draw/enum.Primitive.html) that should be drawn.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the widget
    /// - `clip`: a clipping rect for use with [`Primitive::PushClip`](../draw/enum.Primitive.html#variant.PushClip).
    pub fn draw(&mut self, layout: Rectangle, clip: Rectangle) -> Vec<Primitive<'a>> {
        let stylesheet = self.stylesheet.as_ref().unwrap().deref();
        self.widget.draw(layout, clip, stylesheet)
    }
}

impl<'a, Message: 'a> IntoNode<'a, Message> for Node<'a, Message> {
    fn into_node(self) -> Node<'a, Message> {
        self
    }
}

impl<Message> Context<Message> {
    pub(crate) fn new(redraw: bool) -> Self {
        Self {
            redraw,
            messages: Vec::new(),
        }
    }

    /// Push a message to the current [`Model`].
    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Push multiple messages to the current [`Model`] using an iterator.
    pub fn extend<I: IntoIterator<Item = Message>>(&mut self, iter: I) {
        self.messages.extend(iter);
    }

    /// Request a redraw of the ui.
    pub fn redraw(&mut self) {
        self.redraw = true;
    }

    /// Returns the redraw flag.
    pub fn redraw_requested(&self) -> bool {
        self.redraw
    }
}

impl<Message> IntoIterator for Context<Message> {
    type Item = Message;
    type IntoIter = std::vec::IntoIter<Message>;

    fn into_iter(self) -> Self::IntoIter {
        self.messages.into_iter()
    }
}
