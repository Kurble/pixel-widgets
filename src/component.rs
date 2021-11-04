use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::{Hash, Hasher};

use futures::Stream;

use crate::node::component_node::{Runtime, State};
use crate::node::Node;
use crate::style::builder::StyleBuilder;
use crate::widget::Context as WidgetContext;

/// A re-usable component for defining a fragment of a user interface.
/// Components are the main building block for user interfaces in pixel-widgets.
///
/// The examples in this repository all implement some kind of `Component`,
/// check them out if you just want to read some code.
pub trait Component: Default {
    /// Mutable state associated with this `Component`.
    type State: 'static + Any + Send + Sync;

    /// The message type this `Component` will receive from it's view.
    type Message: 'static;

    /// The message type this `Component` submits to its parent.
    type Output: 'static;

    /// Create a new `State` for the `Component`.
    /// This will be called only once when the `Component` is first created.
    fn mount(&self) -> Self::State;

    /// Generate the view for the `Component`.
    /// This will be called just in time before ui rendering.
    /// When the `Component` is updated,
    ///  the view will be invalidated and the runtime will have to call this function again.
    fn view<'a>(&'a self, state: &'a Self::State) -> Node<'a, Self::Message>;

    /// Update the `Component` state in response to the `message`.
    /// Asynchronous operations can be submitted to the `context`,
    ///  which will result in more `update` calls in the future.
    /// Messages for the parent `Component` or root can also be submitted through the `context`.
    fn update(
        &self,
        _message: Self::Message,
        _state: State<Self::State>,
        _context: Context<Self::Message, Self::Output>,
    ) {
    }

    /// Returns a `StyleBuilder` with styling information scoped to this component.
    fn style() -> StyleBuilder {
        StyleBuilder::default()
    }

    /// Converts the component into a `Node`. This is used by the library to
    ///  instantiate the component in a user interface.
    fn into_node<'a>(self) -> Node<'a, Self::Output>
    where
        Self: 'a + Sized,
    {
        Node::from_component(self)
    }

    /// Converts the component into a `Node` and sets a style class to it.
    fn class<'a>(self, class: &'a str) -> Node<'a, Self::Output>
    where
        Self: 'a + Sized,
    {
        let mut node = self.into_node();
        node.set_class(class);
        node
    }

    /// Converts the component into a `Node` and sets a custom key to it.
    fn key<'a, K>(self, key: K) -> Node<'a, Self::Output>
    where
        Self: 'a + Sized,
        K: Hash,
    {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let mut node = self.into_node();
        node.set_key(hasher.finish());
        node
    }
}

/// Allows for message passing between components
pub struct Context<'a, Message, Output> {
    widget_context: &'a mut WidgetContext<Output>,
    runtime: &'a mut Runtime<Message>,
}

impl<'a, Message, Output> Context<'a, Message, Output> {
    pub(crate) fn new(widget_context: &'a mut WidgetContext<Output>, runtime: &'a mut Runtime<Message>) -> Self {
        Self {
            widget_context,
            runtime,
        }
    }

    /// Push a message to the parent.
    pub fn push(&mut self, message: Output) {
        self.widget_context.push(message);
    }

    /// Push multiple messages to the parent using an iterator.
    pub fn extend<I: IntoIterator<Item = Output>>(&mut self, iter: I) {
        self.widget_context.extend(iter);
    }

    /// Returns the cursor position
    pub fn cursor(&self) -> (f32, f32) {
        self.widget_context.cursor()
    }

    /// Submits a messsage to self in the future.
    pub fn wait<F: 'static + Future<Output = Message> + Send + Sync + Unpin>(&mut self, fut: F) {
        self.runtime.wait(fut);
    }

    /// Submits a stream of messages to self in the future.
    pub fn stream<S: 'static + Stream<Item = Message> + Send + Sync + Unpin>(&mut self, stream: S) {
        self.runtime.stream(stream);
    }
}
