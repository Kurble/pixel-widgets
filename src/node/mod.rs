use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};

use crate::draw::Primitive;
use crate::event::Event;
use crate::layout::{Rectangle, Size};
use crate::style::tree::Query;
use crate::tracker::ManagedStateTracker;
use crate::widget::{Context, Widget};
use crate::Component;

pub(crate) mod component_node;
pub(crate) mod widget_node;

/// A node in a user interface element tree.
pub struct Node<'a, Message>(Box<dyn GenericNode<'a, Message> + 'a>);

#[doc(hidden)]
pub trait GenericNode<'a, Message>: Send {
    fn get_key(&self) -> u64;

    fn set_key(&mut self, key: u64);

    fn set_class(&mut self, class: &'a str);

    fn acquire_state(&mut self, tracker: &mut ManagedStateTracker<'a>);

    fn size(&self) -> (Size, Size);

    fn hit(&self, layout: Rectangle, clip: Rectangle, x: f32, y: f32, recursive: bool) -> bool;

    fn focused(&self) -> bool;

    fn draw(&mut self, layout: Rectangle, clip: Rectangle) -> Vec<Primitive<'a>>;

    fn style(&mut self, query: &mut Query, position: (usize, usize));

    fn add_matches(&mut self, query: &mut Query);

    fn remove_matches(&mut self, query: &mut Query);

    fn event(&mut self, layout: Rectangle, clip: Rectangle, event: Event, context: &mut Context<Message>);

    fn acquire_waker(&mut self, waker: &std::task::Waker);

    fn poll(&mut self, context: &mut Context<Message>, task_context: &mut std::task::Context);
}

/// Convert widget to a [`Node`](struct.Node.html).
/// All widgets should implement this trait.
/// It is also implemented by [`Node`](struct.Node.html) itself, which simply returns self.
pub trait IntoNode<'a, Message: 'a>: 'a + Sized {
    /// Perform the conversion.
    fn into_node(self) -> Node<'a, Message>;

    /// Convenience function that converts to a node and then adds a style class to the resulting [`Node`](struct.Node.html).
    fn class(self, class: &'a str) -> Node<'a, Message> {
        let mut node = self.into_node();
        node.set_class(class);
        node
    }

    /// Convenience function that converts to a node and then sets a custom id to the resulting [`Node`](struct.Node.html).
    fn key<K: Hash>(self, key: K) -> Node<'a, Message> {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let mut node = self.into_node();
        node.set_key(hasher.finish());
        node
    }
}

impl<'a, Message: 'a> Node<'a, Message> {
    /// Create a new [`Node`](struct.Node.html) from a [`Widget`](../widget/trait.Widget.html).
    pub fn from_widget<W: 'a + Widget<'a, Message>>(widget: W) -> Self {
        Self(Box::new(widget_node::WidgetNode::new(widget)) as Box<_>)
    }

    /// Create a new [`Node`](struct.Node.html) from a [`Component`](../component/trait.Component.html).
    pub fn from_component<C: 'a + Component<Output = Message>>(component: C) -> Self {
        Self(Box::new(component_node::ComponentNode::new(component)) as Box<_>)
    }
}

impl<'a, Message> Deref for Node<'a, Message> {
    type Target = dyn GenericNode<'a, Message> + 'a;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a, Message> DerefMut for Node<'a, Message> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl<'a, Message: 'a> IntoNode<'a, Message> for Node<'a, Message> {
    fn into_node(self) -> Node<'a, Message> {
        self
    }
}
