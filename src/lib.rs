//! pixel-widgets is a user interface library focused on use for games. It's architecture is inspired by elm, since it's very
//! fitting for rusts mutability patterns.
//!
//! # Features
//! - Very compact and easy API
//! - Render agnostic rendering
//! - [wgpu](https://github.com/gfx-rs/wgpu-rs) based renderer included
//! - Styling using [stylesheets](stylesheet/index.html)
//! - Built in [widgets](widget/index.html)
//!
//! Check out the [examples](https://github.com/Kurble/pixel-widgets/tree/master/examples) to get started quickly.
//!
//! # Overview
//! User interfaces in pixel-widgets are all defined by implementing a [`Model`](trait.Model.html), serving as the data model
//! for your user interface. The model then has to implement some methods:
//! - [`view`](trait.Model.html#tymethod.view) - for generating a tree of ui widgets. These are retained for as long as
//! the model is not mutated. Ui widgets generate _messages_ when they are interacted with, which leads us to the next
//! method:
//! - [`update`](trait.Model.html#tymethod.update) - modifies the model based on a message that was generated
//! by the view
//!
//! Other ways of updating the ui, such as futures and subscriptions will be be coming in the future.
//!
//! # Quick start
//! Setting up a ui with pixel-widgets is easy. You start with defining a model.
//! ```
//! use pixel_widgets::prelude::*;
//!
//! pub struct Counter {
//!     // a state manager, used for remembering the state of our buttons
//!     state: ManagedState<String>,
//!     // the counter value
//!     count: i32,
//! }
//! ```
//!
//! Then, we have to define a message type. The message type should be able to tell us what happend in the ui.
//! ```
//! pub enum Message {
//!     UpClicked,
//!     DownClicked,
//! }
//! ```
//!
//! And finally, we must implement [`Model`](trait.Model.html) on our state
//! ```
//! use pixel_widgets::node::Node;
//! use pixel_widgets::prelude::*;
//!
//!
//! pub struct Counter {
//!     state: ManagedState<String>,
//!     count: i32,
//! }
//!
//! pub enum Message {
//!     UpClicked,
//!     DownClicked,
//! }
//!
//! impl UpdateComponent for Counter {
//!     // define our message type
//!     type Message = Message;
//!
//!     fn update(&mut self, message: Message) -> Vec<Command<Message>> {
//!         match message {
//!             Message::UpClicked => self.count += 1,
//!             Message::DownClicked => self.count -= 1,
//!         }
//!         Vec::new()
//!     }
//!
//!     // Note that the view is allowed to keep mutable references to the model.
//!     // As soon as the model is accessed mutably, the `Ui` will destroy the existing view.
//!     fn view(&mut self) -> Node<Message> {
//!         let mut state = self.state.tracker();
//!         Column::new()
//!             .push(Button::new(state.get("up"), Text::new("Up"))
//!                 .on_clicked(Message::UpClicked)
//!             )
//!             .push(Text::new(format!("Count: {}", self.count)))
//!             .push(Button::new(state.get("down"), Text::new("Down"))
//!                 .on_clicked(Message::DownClicked)
//!             )
//!             .into_node()
//!     }
//! }
//!
//! // Now that we have a model that can be used with pixel-widgets,
//! // we can pass it to the sandbox to quickly see some results!
//! #[tokio::main]
//! async fn main() {
//!     let model = Counter {
//!         state: ManagedState::default(),
//!         count: 0,
//!     };
//!
//!     let window = winit::window::WindowBuilder::new()
//!         .with_title("Counter")
//!         .with_inner_size(winit::dpi::LogicalSize::new(240, 240));
//!
//!    let loader = pixel_widgets::loader::FsLoader::new("./examples".into()).unwrap();
//!
//!     let mut sandbox = Sandbox::new(model, loader, window).await;
//!     sandbox.ui.set_stylesheet("counter.pwss").await.unwrap();
//!     sandbox.run().await;
//! }
//! ```
//!
//#![deny(missing_docs)]

use std::any::Any;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::task::{Poll, Waker};

use futures::{FutureExt, Stream, StreamExt};

use node::{GenericNode, Node};
use widget::Context;

use crate::draw::DrawList;
use crate::event::Event;
use crate::layout::Rectangle;
use crate::node::component_node::ComponentNode;
use crate::stylesheet::tree::Query;
use crate::stylesheet::Style;
use crate::tracker::ManagedState;

mod atlas;
/// Backend specific code
pub mod backend;
mod bitset;
/// Texture cache for styles and text
pub mod cache;
/// Primitives used for drawing
pub mod draw;
/// User input events
pub mod event;
/// Graphics loader
pub mod graphics;
/// Primitives used for layouts
pub mod layout;
mod macros;
/// User interface building blocks
pub mod node;
/// Simple windowing system for those who want to render _just_ widgets.
#[cfg(feature = "winit")]
#[cfg(feature = "wgpu")]
pub mod sandbox;
/// Styling system
pub mod stylesheet;
/// Primitives for rendering text
pub mod text;
/// Utility for tracking state conveniently.
pub mod tracker;
/// User interface widgets
pub mod widget;

/// A re-usable component for defining a fragment of a user interface.
///
/// # Examples
/// The examples in this repository all implement some kind of [`Component`](trait.Component.html), 
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
    fn mount(&self) -> Self::State;

    /// Generate the view for the `Component`. 
    /// This will be called just in time before ui rendering. 
    /// When the `Component` is updated, 
    ///  the view will be invalidated and the runtime will have to call this function again.
    fn view<'a>(&'a self, state: &'a Self::State) -> Node<'a, Self::Message>;

    /// Update the `Component` state in response to the `message`.
    /// Asynchronous operations can be submitted to the `runtime`, 
    ///  which will result in more `update` calls in the future.
    /// Messages for the parent `Component` or root can be submitted through the `context`.
    fn update(
        &self,
        _message: Self::Message,
        _state: &mut Self::State,
        _runtime: &mut Runtime<Self::Message>,
        _context: &mut Context<Self::Output>,
    ) {}
}

pub struct Runtime<Message> {
    futures: Vec<Box<dyn Future<Output = Message> + Send + Sync + Unpin>>,
    streams: Vec<Box<dyn Stream<Item = Message> + Send + Sync + Unpin>>,
    modified: bool,
}

impl<Message> Default for Runtime<Message> {
    fn default() -> Self {
        Self {
            futures: Vec::new(),
            streams: Vec::new(),
            modified: false,
        }
    }
}

impl<Message> Runtime<Message> {
    pub fn wait<F: 'static + Future<Output = Message> + Send + Sync + Unpin>(&mut self, fut: F) {
        self.futures.push(Box::new(fut));
        self.modified = true;
    }

    pub fn stream<S: 'static + Stream<Item = Message> + Send + Sync + Unpin>(&mut self, stream: S) {
        self.streams.push(Box::new(stream));
        self.modified = true;
    }

    pub(crate) fn poll(&mut self, cx: &mut std::task::Context) -> Vec<Message> {
        self.modified = false;

        let mut result = Vec::new();

        let mut i = 0;
        while i < self.futures.len() {
            match self.futures[i].poll_unpin(&mut *cx) {
                Poll::Ready(message) => {
                    result.push(message);
                    drop(self.futures.remove(i));
                }
                Poll::Pending => {
                    i += 1;
                }
            }
        }

        let mut i = 0;
        while i < self.streams.len() {
            match self.streams[i].poll_next_unpin(&mut *cx) {
                Poll::Ready(Some(message)) => {
                    result.push(message);
                }
                Poll::Ready(None) => {
                    drop(self.streams.remove(i));
                }
                Poll::Pending => {
                    i += 1;
                }
            }
        }

        result
    }
}

/// Entry point for pixel-widgets.
///
/// `Ui` manages a [`Model`](trait.Model.html) and processes it to a [`DrawList`](draw/struct.DrawList.html) that can be rendered using your
///  own renderer implementation. Alternatively, you can use one of the following included wrappers:
/// - [`WgpuUi`](backend/wgpu/struct.WgpuUi.html) Renders using [wgpu-rs](https://github.com/gfx-rs/wgpu-rs).
pub struct Ui<M: 'static + Component> {
    root_node: ComponentNode<'static, M>,
    _state: ManagedState,
    viewport: Rectangle,
    redraw: bool,
    cursor: (f32, f32),
    style: Arc<Style>,
}

impl<M: 'static + Component> Ui<M> {
    /// Constructs a new `Ui` using the default style.
    /// This is not recommended as the default style is very empty and only renders white text.
    pub fn new(model: M, viewport: Rectangle) -> Self {
        let mut state = ManagedState::default();
        let mut root_node = ComponentNode::new(model);
        root_node.acquire_state(&mut unsafe { (&mut state as *mut ManagedState).as_mut() }.unwrap().tracker());

        let style = Arc::new(Style::new(512, 0));

        let mut result = Self {
            root_node,
            _state: state,
            viewport,
            redraw: true,
            cursor: (0.0, 0.0),
            style: style.clone(),
        };
        result.set_style(style);
        result
    }

    pub fn set_style(&mut self, style: Arc<Style>) {
        if !Arc::ptr_eq(&self.style, &style) {
            self.root_node.set_dirty();
            self.style = style.clone();
            self.root_node.style(&mut Query::from_style(style), (0, 1));
        }
    }

    /// Resizes the viewport.
    /// This forces the view to be rerendered.
    pub fn resize(&mut self, viewport: Rectangle) {
        self.root_node.set_dirty();
        self.redraw = true;
        self.viewport = viewport;
    }

    /// Returns true if the ui needs to be redrawn. If the ui doesn't need to be redrawn the
    /// [`Command`s](draw/struct.Command.html) from the last [`draw`](#method.draw) may be used again.
    pub fn needs_redraw(&self) -> bool {
        self.redraw || self.root_node.dirty()
    }

    /// Updates the model with a message.
    /// This forces the view to be rerendered.
    pub fn update_poll(&mut self, message: M::Message, waker: Waker) -> Vec<M::Output> {
        let mut context = Context::new(self.needs_redraw(), self.cursor, waker);

        self.root_node.update(message, &mut context);
        while self.root_node.needs_poll() {
            self.root_node.poll(&mut context);
        }

        self.redraw = context.redraw_requested();
        context.into_vec()
    }

    /// Handles an [`Event`](event/struct.Event.html).
    pub fn event(&mut self, event: Event, waker: Waker) -> Vec<M::Output> {
        if let Event::Cursor(x, y) = event {
            self.cursor = (x, y);
        }

        let mut context = Context::new(self.needs_redraw(), self.cursor, waker.clone());

        {
            let mut view = self.root_node.view();
            let (w, h) = view.size();
            let layout = Rectangle::from_wh(
                w.resolve(self.viewport.width(), w.parts()),
                h.resolve(self.viewport.height(), h.parts()),
            );
            view.event(layout, self.viewport, event, &mut context);
        }

        self.redraw = context.redraw_requested();

        let mut outer_context = Context::new(self.needs_redraw(), self.cursor, waker.clone());

        for message in context {
            self.root_node.update(message, &mut outer_context);
        }
        while self.root_node.needs_poll() {
            self.root_node.poll(&mut outer_context);
        }

        self.redraw = outer_context.redraw_requested();
        outer_context.into_vec()
    }

    /// Should be called when the waker wakes :)
    pub fn poll(&mut self, waker: Waker) -> Vec<M::Output> {
        let mut context = Context::new(self.needs_redraw(), self.cursor, waker);
        loop {
            self.root_node.poll(&mut context);
            self.redraw = context.redraw_requested();

            if self.root_node.needs_poll() {
                continue;
            } else {
                break;
            }
        }
        context.into_vec()
    }

    /// Generate a [`DrawList`](draw/struct.DrawList.html) for the view.
    pub fn draw(&mut self) -> DrawList {
        use self::draw::*;

        let viewport = self.viewport;
        let primitives = {
            let mut view = self.root_node.view();
            let (w, h) = view.size();
            let layout = Rectangle::from_wh(
                w.resolve(viewport.width(), w.parts()),
                h.resolve(viewport.height(), h.parts()),
            );
            view.draw(layout, viewport)
        };
        self.redraw = false;

        struct Layer {
            vtx: Vec<Vertex>,
            cmd: Vec<Command>,
        }

        impl Layer {
            fn append(&mut self, command: Command) {
                if let Some(next) = self.cmd.last_mut().unwrap().append(command) {
                    self.cmd.push(next);
                }
            }
        }

        let mut layers = vec![Layer {
            vtx: Vec::new(),
            cmd: vec![Command::Nop],
        }];
        let mut layer: usize = 0;

        let mut scissors = vec![viewport];

        let validate_clip = move |clip: Rectangle| {
            let v = Rectangle {
                left: clip.left.max(0.0).min(viewport.right),
                top: clip.top.max(0.0).min(viewport.bottom),
                right: clip.right.max(0.0).min(viewport.right),
                bottom: clip.bottom.max(0.0).min(viewport.bottom),
            };
            if v.right as u32 - v.left as u32 > 0 && v.bottom as u32 - v.top as u32 > 0 {
                Some(v)
            } else {
                None
            }
        };

        let mut draw_enabled = true;

        for primitive in primitives.into_iter() {
            match primitive {
                Primitive::PushClip(scissor) => {
                    scissors.push(scissor);

                    draw_enabled = validate_clip(scissor).map_or(false, |s| {
                        layers[layer].append(Command::Clip { scissor: s });
                        true
                    });
                }

                Primitive::PopClip => {
                    scissors.pop();
                    let scissor = scissors[scissors.len() - 1];

                    draw_enabled = validate_clip(scissor).map_or(false, |s| {
                        layers[layer].append(Command::Clip { scissor: s });
                        true
                    });
                }

                Primitive::LayerUp => {
                    layer += 1;
                    while layer >= layers.len() {
                        layers.push(Layer {
                            vtx: Vec::new(),
                            cmd: vec![Command::Nop],
                        });
                    }
                }

                Primitive::LayerDown => {
                    layer -= 1;
                }

                Primitive::DrawRect(r, color) => {
                    if draw_enabled {
                        let r = r.to_device_coordinates(viewport);
                        let color = [color.r, color.g, color.b, color.a];
                        let mode = 1.0;
                        let offset = layers[layer].vtx.len();
                        layers[layer].vtx.push(Vertex {
                            pos: [r.left, r.top],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        layers[layer].vtx.push(Vertex {
                            pos: [r.right, r.top],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        layers[layer].vtx.push(Vertex {
                            pos: [r.right, r.bottom],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        layers[layer].vtx.push(Vertex {
                            pos: [r.left, r.top],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        layers[layer].vtx.push(Vertex {
                            pos: [r.right, r.bottom],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        layers[layer].vtx.push(Vertex {
                            pos: [r.left, r.bottom],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        layers[layer].append(Command::Colored { offset, count: 6 });
                    }
                }

                Primitive::DrawText(text, rect) => {
                    if draw_enabled {
                        let color = [text.color.r, text.color.g, text.color.b, text.color.a];
                        let mode = 0.0;
                        let offset = layers[layer].vtx.len();

                        self.style.cache().lock().unwrap().draw_text(&text, rect, |uv, pos| {
                            let rc = Rectangle {
                                left: pos.left,
                                top: pos.top,
                                right: pos.right,
                                bottom: pos.bottom,
                            }
                            .to_device_coordinates(viewport);

                            layers[layer].vtx.push(Vertex {
                                pos: [rc.left, rc.top],
                                uv: uv.pt(0.0, 0.0),
                                color,
                                mode,
                            });
                            layers[layer].vtx.push(Vertex {
                                pos: [rc.right, rc.top],
                                uv: uv.pt(1.0, 0.0),
                                color,
                                mode,
                            });
                            layers[layer].vtx.push(Vertex {
                                pos: [rc.right, rc.bottom],
                                uv: uv.pt(1.0, 1.0),
                                color,
                                mode,
                            });
                            layers[layer].vtx.push(Vertex {
                                pos: [rc.left, rc.top],
                                uv: uv.pt(0.0, 0.0),
                                color,
                                mode,
                            });
                            layers[layer].vtx.push(Vertex {
                                pos: [rc.right, rc.bottom],
                                uv: uv.pt(1.0, 1.0),
                                color,
                                mode,
                            });
                            layers[layer].vtx.push(Vertex {
                                pos: [rc.left, rc.bottom],
                                uv: uv.pt(0.0, 1.0),
                                color,
                                mode,
                            });
                        });

                        let count = layers[layer].vtx.len() - offset;
                        layers[layer].append(Command::Textured {
                            texture: text.font.tex_slot,
                            offset,
                            count,
                        });
                    }
                }

                Primitive::Draw9(patch, rect, color) => {
                    if draw_enabled {
                        let uv = patch.image.texcoords;
                        let color = [color.r, color.g, color.b, color.a];
                        let mode = 0.0;
                        let offset = layers[layer].vtx.len();

                        patch.iterate_sections(false, rect.width(), |x, u| {
                            patch.iterate_sections(true, rect.height(), |y, v| {
                                let rc = Rectangle {
                                    left: x.0 + rect.left,
                                    right: x.1 + rect.left,
                                    top: y.0 + rect.top,
                                    bottom: y.1 + rect.top,
                                }
                                .to_device_coordinates(viewport);

                                layers[layer].vtx.push(Vertex {
                                    pos: [rc.left, rc.top],
                                    uv: uv.pt(u.0, v.0),
                                    color,
                                    mode,
                                });
                                layers[layer].vtx.push(Vertex {
                                    pos: [rc.right, rc.top],
                                    uv: uv.pt(u.1, v.0),
                                    color,
                                    mode,
                                });
                                layers[layer].vtx.push(Vertex {
                                    pos: [rc.right, rc.bottom],
                                    uv: uv.pt(u.1, v.1),
                                    color,
                                    mode,
                                });
                                layers[layer].vtx.push(Vertex {
                                    pos: [rc.left, rc.top],
                                    uv: uv.pt(u.0, v.0),
                                    color,
                                    mode,
                                });
                                layers[layer].vtx.push(Vertex {
                                    pos: [rc.right, rc.bottom],
                                    uv: uv.pt(u.1, v.1),
                                    color,
                                    mode,
                                });
                                layers[layer].vtx.push(Vertex {
                                    pos: [rc.left, rc.bottom],
                                    uv: uv.pt(u.0, v.1),
                                    color,
                                    mode,
                                });
                            });
                        });

                        let count = layers[layer].vtx.len() - offset;
                        layers[layer].append(Command::Textured {
                            texture: patch.image.texture,
                            offset,
                            count,
                        });
                    }
                }

                Primitive::DrawImage(image, r, color) => {
                    if draw_enabled {
                        let r = r.to_device_coordinates(viewport);
                        let uv = image.texcoords;
                        let color = [color.r, color.g, color.b, color.a];
                        let mode = 0.0;
                        let offset = layers[layer].vtx.len();

                        layers[layer].vtx.push(Vertex {
                            pos: [r.left, r.top],
                            uv: [uv.left, uv.top],
                            color,
                            mode,
                        });
                        layers[layer].vtx.push(Vertex {
                            pos: [r.right, r.top],
                            uv: [uv.right, uv.top],
                            color,
                            mode,
                        });
                        layers[layer].vtx.push(Vertex {
                            pos: [r.right, r.bottom],
                            uv: [uv.right, uv.bottom],
                            color,
                            mode,
                        });
                        layers[layer].vtx.push(Vertex {
                            pos: [r.left, r.top],
                            uv: [uv.left, uv.top],
                            color,
                            mode,
                        });
                        layers[layer].vtx.push(Vertex {
                            pos: [r.right, r.bottom],
                            uv: [uv.right, uv.bottom],
                            color,
                            mode,
                        });
                        layers[layer].vtx.push(Vertex {
                            pos: [r.left, r.bottom],
                            uv: [uv.left, uv.bottom],
                            color,
                            mode,
                        });

                        layers[layer].append(Command::Textured {
                            texture: image.texture,
                            offset,
                            count: 6,
                        });
                    }
                }
            }
        }

        let (vertices, commands) =
            layers
                .into_iter()
                .fold((Vec::new(), Vec::new()), |(mut vtx, mut cmd), mut layer| {
                    let layer_offset = vtx.len();
                    vtx.append(&mut layer.vtx);
                    cmd.extend(layer.cmd.into_iter().map(|command| match command {
                        Command::Textured { texture, offset, count } => Command::Textured {
                            texture,
                            offset: offset + layer_offset,
                            count,
                        },
                        Command::Colored { offset, count } => Command::Colored {
                            offset: offset + layer_offset,
                            count,
                        },
                        other => other,
                    }));
                    (vtx, cmd)
                });

        DrawList {
            updates: self.style.cache().lock().unwrap().take_updates(),
            vertices,
            commands,
        }
    }
}

impl<M: Component> Deref for Ui<M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.root_node.props()
    }
}

impl<M: Component> DerefMut for Ui<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.root_node.props_mut()
    }
}

/// prelude module for convenience
pub mod prelude {
    #[cfg(feature = "winit")]
    #[cfg(feature = "wgpu")]
    pub use crate::sandbox::Sandbox;
    pub use crate::{declare_view, layout::Rectangle, node::*, stylesheet::Style, widget::*, Component, Runtime, Ui};
}
