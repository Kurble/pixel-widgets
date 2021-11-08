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
pub trait Component: Sized {
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

    /// Returns the scope for the styling information of this component returned by [`style()`](#method.style)
    fn style_scope() -> &'static str {
        std::any::type_name::<Self>()
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

/// Utility methods for components
pub trait ComponentExt: Component + Sized {
    /// Maps the output message of a component to a different type.
    fn map_message<T: 'static, F: Fn(Self::Output) -> T>(self, map_fn: F) -> MapComponent<Self, T, F> {
        MapComponent {
            component: self,
            map_fn,
        }
    }
}

impl<T: Component + Sized> ComponentExt for T {}

/// The value returned by [`ComponentExt::map_message`](trait.ComponentExt.html#method.map_message).
pub struct MapComponent<C: Component, T: 'static, F: Fn(C::Output) -> T> {
    component: C,
    map_fn: F,
}

impl<C: Component, T: 'static, F: Fn(C::Output) -> T> Component for MapComponent<C, T, F> {
    type State = C::State;

    type Message = C::Message;

    type Output = T;

    fn mount(&self, runtime: &mut Runtime<Self::Message>) -> Self::State {
        self.component.mount(runtime)
    }

    fn view<'a>(&'a self, state: &'a Self::State) -> Node<'a, Self::Message> {
        self.component.view(state)
    }

    fn update(
        &self,
        message: C::Message,
        state: DetectMut<C::State>,
        runtime: &mut Runtime<C::Message>,
        context: &mut Context<T>,
    ) {
        let mut sub_context = context.sub_context();
        self.component.update(message, state, runtime, &mut sub_context);
        if sub_context.redraw_requested() {
            context.redraw();
        }
        context.extend(sub_context.into_iter().map(|m| (self.map_fn)(m)));
    }

    fn style() -> StyleBuilder {
        C::style()
    }

    fn style_scope() -> &'static str {
        C::style_scope()
    }
}
