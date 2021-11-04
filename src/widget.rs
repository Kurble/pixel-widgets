//! Widgets are defined using the [`Widget`](trait.Widget.html) trait.
//! You can choose to implement widgets yourself, or you can use the built in widgets defined in this module.
//!
//! Since the whole [`Component`](../trait.Component.html) every time it is mutated,
//! most widgets need to keep track of some kind of state across rebuilds.
//! This is managed automatically by the component, identified by the [`key`](trait.Widget.html#method.key) method.
//! When implementing custom widgets, you need to make sure that the returned `u64` key is as unique as possible.
//! You also need to make sure that the custom widgets do not remember absolute layouts.
//! Widgets like [`Scroll`](scroll/struct.Scroll.html) can change the layout without needing a rebuild of the ui.
use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::task::Waker;

use smallvec::SmallVec;

use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::*;
use crate::node::GenericNode;
use crate::style::*;

/// Prelude widgets
pub mod prelude {
    pub use super::button::Button;
    pub use super::column::Column;
    pub use super::drag_drop::{Drag, Drop};
    pub use super::dropdown::Dropdown;
    pub use super::dummy::Dummy;
    pub use super::frame::Frame;
    pub use super::image::Image;
    pub use super::input::Input;
    pub use super::layers::Layers;
    pub use super::menu::Menu;
    pub use super::panel::Panel;
    pub use super::progress::Progress;
    pub use super::row::Row;
    pub use super::scroll::Scroll;
    pub use super::slider::Slider;
    pub use super::spacer::Spacer;
    pub use super::text::Text;
    pub use super::toggle::Toggle;
    pub use super::window::Window;

    pub use super::{StateVec, Widget};
}

/// A clickable button
pub mod button;
/// Layout child widgets vertically
pub mod column;
/// Drag and drop zones
pub mod drag_drop;
/// Pick an item from a dropdown box
pub mod dropdown;
/// Dummy widget that has a custom widget name
pub mod dummy;
/// A widget that wraps around a content widget
pub mod frame;
/// Just an image
pub mod image;
/// Editable text input
pub mod input;
/// Stack child widgets on top of each other, while only the topmost receives events.
pub mod layers;
/// A context menu with nestable items
pub mod menu;
/// A panel with a fixed size and location within it's parent
pub mod panel;
/// A bar that fills up according to a value.
pub mod progress;
/// Layout child widgets horizontally
pub mod row;
/// View a small section of larger widget, with scrollbars.
pub mod scroll;
/// A slider for easily picking some number
pub mod slider;
/// Empty widget
pub mod spacer;
/// Widget that renders a paragraph of text.
pub mod text;
/// A clickable button that toggles some `bool`.
pub mod toggle;
/// A window with a title and a content widget that can be moved by dragging the title.
pub mod window;

/// A user interface widget.
pub trait Widget<'a, Message>: Send {
    /// The type of state this widget keeps track of.
    type State: Any + Send + Sync;

    /// The key of this widget, used for resolving state.
    fn key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        std::any::type_name::<Self>().hash(&mut hasher);
        hasher.finish()
    }

    /// Create a new state
    fn mount(&self) -> Self::State;

    /// The name of this widget, used to identify widgets of this type in stylesheets.
    fn widget(&self) -> &'static str;

    /// The state of this widget, used for computing the style.
    /// If `None` is returned, `Node` will automatically compute a state, such as "hover" and "pressed".
    fn state(&self, _state: &Self::State) -> StateVec {
        StateVec::new()
    }

    /// Should return the amount of children this widget has. Must be consistent with
    /// [`visit_children()`](#tymethod.visit_children).
    fn len(&self) -> usize;

    /// Returns whether this children has no children. Must be consistent with
    /// [`visit_children()`](#tymethod.visit_children).
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Applies a visitor to all childs of the widget. If an widget fails to visit it's children, the children won't
    /// be able to resolve their stylesheet, resulting in a panic when calling [`size`](struct.Node.html#method.size),
    /// [`hit`](struct.Node.html#method.hit), [`event`](struct.Node.html#method.event) or
    /// [`draw`](struct.Node.html#method.draw).
    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut dyn GenericNode<'a, Message>));

    /// Returns the `(width, height)` of this widget.
    /// The extents are defined as a [`Size`](../layout/struct.Size.html),
    /// which will later be resolved to actual dimensions.
    fn size(&self, state: &Self::State, style: &Stylesheet) -> (Size, Size);

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
    fn hit(
        &self,
        _state: &Self::State,
        layout: Rectangle,
        clip: Rectangle,
        _style: &Stylesheet,
        x: f32,
        y: f32,
    ) -> bool {
        layout.point_inside(x, y) && clip.point_inside(x, y)
    }

    /// Test the widget for focus exclusivity.
    /// If the widget or one of it's descendants is in an exclusive focus state, this function should return `true`.
    /// In all other cases, it should return `false`. When a widget is in an exclusive focus state it is
    /// the only widget that is allowed to receive events in [`event`](#tymethod.event).
    /// Widgets that intended to use this behaviour are modal dialogs, dropdown boxes, context menu's, etc.
    fn focused(&self, _state: &Self::State) -> bool {
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
        _state: &mut Self::State,
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
    fn draw(
        &mut self,
        state: &mut Self::State,
        layout: Rectangle,
        clip: Rectangle,
        style: &Stylesheet,
    ) -> Vec<Primitive<'a>>;
}

/// Storage for style states
pub type StateVec = SmallVec<[StyleState<&'static str>; 3]>;

/// Context for posting messages and requesting redraws of the ui.
pub struct Context<Message> {
    cursor: (f32, f32),
    redraw: bool,
    poll: bool,
    messages: Vec<Message>,
    waker: Waker,
}

impl<Message> Context<Message> {
    pub(crate) fn new(redraw: bool, cursor: (f32, f32), waker: Waker) -> Self {
        Context {
            cursor,
            redraw,
            poll: false,
            messages: Vec::new(),
            waker,
        }
    }

    pub(crate) fn sub_context<M>(&self) -> Context<M> {
        Context {
            cursor: self.cursor,
            redraw: self.redraw,
            poll: self.poll,
            messages: Vec::new(),
            waker: self.waker.clone(),
        }
    }

    /// Push a message to the parnet [`Component`](../component/trait.Component.html).
    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Push multiple messages to the parent [`Component`](../component/trait.Component.html) using an iterator.
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

    /// Returns the cursor position
    pub fn cursor(&self) -> (f32, f32) {
        self.cursor
    }

    pub(crate) fn task_context(&self) -> std::task::Context<'_> {
        std::task::Context::from_waker(&self.waker)
    }

    pub(crate) fn into_vec(self) -> Vec<Message> {
        self.messages
    }
}

impl<Message> IntoIterator for Context<Message> {
    type Item = Message;
    type IntoIter = std::vec::IntoIter<Message>;

    fn into_iter(self) -> Self::IntoIter {
        self.messages.into_iter()
    }
}
