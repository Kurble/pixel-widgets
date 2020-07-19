//!
//! Elements in maple are defined using the [`Element`](trait.Element.html) trait.
//! You can choose to implement elements yourself, or you can use the built in elements that come with maple:
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
//! Since maple rebuilds the whole ui every time the [`Model`](../trait.Model.html) is modified,
//! most elements need to keep track of some kind of state across rebuilds. You can manually supply these state
//! objects in your [`view`](../trait.Model.html#tymethod.view) implementation, or you can use a
//! [`ManagedState`](../tracker/struct.ManagedState.html), which tracks state for your elements using user defined ids.
//!
//! When implementing custom elements, you need to make sure that the custom elements do not remember absolute layouts.
//! Elements like [`Scroll`](scroll/struct.Scroll.html) can change the layout without needing a rebuild of the ui.

use std::borrow::Cow;
use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::*;
use crate::stylesheet::*;

pub use self::button::Button;
pub use self::column::Column;
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
/// Layout child elements vertically
pub mod column;
/// Editable text input
pub mod input;
/// Stack child elements on top of each other, while only the topmost receives events.
pub mod layers;
/// Layout child elements horizontally
pub mod row;
/// View a small section of larger element, with scrollbars.
pub mod scroll;
/// Empty element
pub mod space;
/// Element that renders a paragraph of text.
pub mod text;
/// A clickable button that toggles some `bool`.
pub mod toggle;
/// A window with a title and a content element that can be moved by dragging the title.
pub mod window;

/// A user interface element.
pub trait Element<'a, Message> {
    /// The name of this element, used to identify elements of this type in stylesheets.
    fn element(&self) -> &'static str;

    /// Applies a visitor to all childs of the element. If an element fails to visit it's children, the children won't
    /// be able to resolve their stylesheet, resulting in a panic when calling [`size`](struct.Node.html#method.size),
    /// [`hit`](struct.Node.html#method.hit), [`event`](struct.Node.html#method.event) or
    /// [`draw`](struct.Node.html#method.draw).
    fn visit_children(&mut self, visitor: &mut dyn FnMut(&mut Node<'a, Message>));

    /// Returns the `(width, height)` of this element.
    /// The extents are defined as a [`Size`](../layout/struct.Size.html),
    /// which will later be resolved to actual dimensions.
    fn size(&self, style: &Stylesheet) -> (Size, Size);

    /// Perform a hit detect on the element. Most elements are fine with the default implementation, but some
    /// elements (like [`Window`](window/struct.Window.html) need to report a _miss_ (`false`) even when the queried
    /// position is within their layout.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the element
    /// - `clip`: a clipping rect for mouse events. Mouse events outside of this rect should be considered invalid,
    /// such as with [`Scroll`](scroll/struct.Scroll.html), where the element would not be visible outside of the
    /// currently visible rect.
    /// - `x`: x mouse coordinate being queried
    /// - `y`: y mouse coordinate being queried
    fn hit(&self, layout: Rectangle, clip: Rectangle, _style: &Stylesheet, x: f32, y: f32) -> bool {
        layout.point_inside(x, y) && clip.point_inside(x, y)
    }

    /// Handle an event. If an event changes the graphical appearance of an `Element`,
    /// [`redraw`](struct.Context.html#method.redraw) should be called to let the [`Ui`](../struct.Ui.html) know that
    /// the ui should be redrawn.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the element
    /// - `clip`: a clipping rect for mouse events. Mouse events outside of this rect should be considered invalid,
    /// such as with [`Scroll`](scroll/struct.Scroll.html), where the element would not be visible outside of the
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

    /// Draw the element. Returns a list of [`Primitive`s](../draw/enum.Primitive.html) that should be drawn.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the element
    /// - `clip`: a clipping rect for use with [`Primitive::PushClip`](../draw/enum.Primitive.html#variant.PushClip).
    fn draw(&mut self, layout: Rectangle, clip: Rectangle, style: &Stylesheet) -> Vec<Primitive<'a>>;
}

/// Convert to a generic element. All elements should implement this trait. It is also implemented by `Node` itself,
/// which simply returns self.
pub trait IntoNode<'a, Message: 'a>: 'a + Sized {
    /// Perform the conversion.
    fn into_node(self) -> Node<'a, Message>;

    /// Convenience function that converts to a node and then adds a style class to the `Node`.
    fn class(self, class: &'a str) -> Node<'a, Message> {
        self.into_node().class(class)
    }
}

/// Generic ui element.
pub struct Node<'a, Message> {
    element: Box<dyn Element<'a, Message> + 'a>,
    size_cache: Cell<Option<(Size, Size)>>,
    style: Option<Rc<Stylesheet>>,
    class: Option<&'a str>,
}

/// Context for posting messages and requesting redraws of the ui.
pub struct Context<Message> {
    redraw: bool,
    messages: Vec<Message>,
}

impl<'a, Message> Node<'a, Message> {
    /// Construct a new `Node` from an [`Element`](trait.Element.html).
    pub fn new<T: 'a + Element<'a, Message>>(element: T) -> Self {
        Node {
            element: Box::new(element),
            size_cache: Cell::new(None),
            style: None,
            class: None,
        }
    }

    /// Sets the style class
    pub fn class(mut self, class: &'a str) -> Self {
        self.class = Some(class);
        self
    }

    pub(crate) fn style(&mut self, engine: &mut Style, query: &mut Query<'a>) {
        query.elements.push(self.element.element());
        if let Some(class) = self.class {
            query.classes.push(Cow::Borrowed(class));
        }

        self.style.replace(engine.get(query));
        self.element
            .visit_children(&mut |child| child.style(&mut *engine, &mut *query));

        query.elements.pop();
        if self.class.is_some() {
            query.classes.pop();
        }
    }

    /// Returns the `(width, height)` of this element.
    /// The extents are defined as a [`Size`](../layout/struct.Size.html),
    /// which will later be resolved to actual dimensions.
    pub fn size(&self) -> (Size, Size) {
        if self.size_cache.get().is_none() {
            let stylesheet = self.style.as_ref().unwrap().deref();
            self.size_cache.replace(Some(self.element.size(stylesheet)));
        }
        self.size_cache.get().unwrap()
    }

    /// Perform a hit detect on the element. Most elements are fine with the default implementation, but some
    /// elements (like [`Window`](window/struct.Window.html) need to report a _miss_ (`false`) even when the queried
    /// position is within their layout.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the element
    /// - `clip`: a clipping rect for mouse events. Mouse events outside of this rect should be considered invalid,
    /// such as with [`Scroll`](scroll/struct.Scroll.html), where the element would not be visible outside of the
    /// currently visible rect.
    /// - `x`: x mouse coordinate being queried
    /// - `y`: y mouse coordinate being queried
    pub fn hit(&self, layout: Rectangle, clip: Rectangle, x: f32, y: f32) -> bool {
        let stylesheet = self.style.as_ref().unwrap().deref();
        self.element.hit(layout, clip, stylesheet, x, y)
    }

    /// Handle an event.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the element
    /// - `clip`: a clipping rect for mouse events. Mouse events outside of this rect should be considered invalid,
    /// such as with [`Scroll`](scroll/struct.Scroll.html), where the element would not be visible outside of the
    /// currently visible rect.
    /// - `event`: the event that needs to be handled
    /// - `context`: context for submitting messages and requesting redraws of the ui.
    pub fn event(&mut self, layout: Rectangle, clip: Rectangle, event: Event, context: &mut Context<Message>) {
        let stylesheet = self.style.as_ref().unwrap().deref();
        self.element.event(layout, clip, stylesheet, event, context);
    }

    /// Draw the element. Returns a list of [`Primitive`s](../draw/enum.Primitive.html) that should be drawn.
    ///
    /// Arguments:
    /// - `layout`: the layout assigned to the element
    /// - `clip`: a clipping rect for use with [`Primitive::PushClip`](../draw/enum.Primitive.html#variant.PushClip).
    pub fn draw(&mut self, layout: Rectangle, clip: Rectangle) -> Vec<Primitive<'a>> {
        let stylesheet = self.style.as_ref().unwrap().deref();
        self.element.draw(layout, clip, stylesheet)
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
    pub fn redraw_requested(&self) -> bool { self.redraw }
}

impl<Message> IntoIterator for Context<Message> {
    type Item = Message;
    type IntoIter = std::vec::IntoIter<Message>;

    fn into_iter(self) -> Self::IntoIter {
        self.messages.into_iter()
    }
}