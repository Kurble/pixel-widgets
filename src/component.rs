use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::node::component_node::{DetectMut, Runtime};
use crate::node::Node;
use crate::style::builder::StyleBuilder;
use crate::widget::Context;

/// A re-usable component for defining a fragment of a user interface.
/// Components are the main building block for user interfaces in pixel-widgets.
///
/// The examples in this repository all implement some kind of `Component`,
/// check them out if you just want to read some code.
pub trait Component {
    /// Mutable state associated with this `Component`.
    type State: 'static + Any + Send + Sync;

    /// The message type this `Component` will receive from it's view.
    type Message: 'static;

    /// The message type this `Component` submits to its parent.
    type Output: 'static;

    /// Create a new `State` for the `Component`.
    /// This will be called only once when the `Component` is first created.
    fn mount(&self, runtime: &mut Runtime<Self::Message>) -> Self::State;

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
        _state: DetectMut<Self::State>,
        _runtime: &mut Runtime<Self::Message>,
        _context: &mut Context<Self::Output>,
    ) {
    }

    /// Returns a `StyleBuilder` with styling information scoped to this component.
    /// This method will be called when you call
    /// [`StyleBuilder::component()`](../style/builder/struct.StyleBuilder.html#method.component)
    /// when building your style.
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
