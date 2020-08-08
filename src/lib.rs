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
//! use pixel_widgets::prelude::*;
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
//! impl Model for Counter {
//!     // define our message type
//!     type Message = Message;
//!
//!     fn update(&mut self, message: Message) {
//!         match message {
//!             Message::UpClicked => self.count += 1,
//!             Message::DownClicked => self.count -= 1,
//!         }
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
//! fn main() {
//!     let model = Counter {
//!         state: ManagedState::default(),
//!         count: 0,
//!     };
//!
//!     let window = winit::window::WindowBuilder::new()
//!         .with_title("Counter")
//!         .with_inner_size(winit::dpi::LogicalSize::new(240, 240));
//!
//!     pixel_widgets::sandbox::run(model, PathBuf::from("."), "counter.pwss", window);
//! }
//! ```
//!
#![deny(missing_docs)]

use std::future::Future;
use std::ops::{Deref, DerefMut};

use draw::DrawList;

use crate::event::Event;
use crate::layout::Rectangle;
use crate::model_view::ModelView;
use crate::stylesheet::Style;
use crate::widget::{Context, Node};
use crate::loader::Loader;
use std::rc::Rc;
use std::task::{Poll, Waker};
use std::sync::{Arc, Mutex};
use futures::Stream;
use futures::task::ArcWake;

mod atlas;
/// Backend specific code
pub mod backend;
mod bitset;
mod cache;
/// Primitives used for drawing
pub mod draw;
/// User input events
pub mod event;
/// Primitives used for layouts
pub mod layout;
/// Asynchronous resource loading
pub mod loader;
mod model_view;
/// Simple windowing system for those who want to render _just_ widgets.
#[cfg(feature="winit")]
#[cfg(feature="wgpu")]
pub mod sandbox;
/// Styling system
pub mod stylesheet;
/// Primitives for rendering text
pub mod text;
/// Utility for tracking state conveniently.
pub mod tracker;
/// User interface widgets
pub mod widget;

/// A model that keeps track of the state of your GUI. Serves to control the behaviour and DOM of your GUI.
/// Styling is handled separately. Once you implemented a model, you can run your GUI using a [`Ui`](struct.Ui.html).
///
/// # Examples
/// The examples in this repository all implement some kind of [`Model`](trait.Model.html), check them out if you just want to read
/// some code.
pub trait Model: 'static {
    /// The type of message your GUI will produce.
    type Message;

    /// Called when a message is fired from the view or some other source.
    /// This is where you should update your gui state.
    fn update(&mut self, message: Self::Message) -> Vec<Command<Self::Message>>;

    /// Called after [`update`](#tymethod.update) or after the model has been accessed mutably from the [`Ui`](struct.Ui.html).
    /// This is where you should build all of your ui widgets based on the current gui state.
    /// The returned ui widgets produce messages of the type `Self::Message`.
    fn view(&mut self) -> Node<Self::Message>;
}

/// Trait for sending custom events to the event loop of the application
pub trait EventLoop<T: Send>: Clone + Send {
    /// The error returned when send_event fails
    type Error;

    /// Sends custom event to the event loop.
    /// Returns an `Err` if sending failed, for example when the event loop doesn't exist anymore.
    fn send_event(&self, event: T) -> Result<(), Self::Error>;
}

/// Entry point for pixel-widgets.
///
/// `Ui` manages a [`Model`](trait.Model.html) and processes it to a [`DrawList`](draw/struct.DrawList.html) that can be rendered using your
///  own renderer implementation. Alternatively, you can use one of the following included wrappers:
/// - [`WgpuUi`](backend/wgpu/struct.WgpuUi.html) Renders using [wgpu-rs](https://github.com/gfx-rs/wgpu-rs).
pub struct Ui<M: Model, E: EventLoop<Command<M::Message>>, L: Loader> {
    model_view: ModelView<M>,
    style: Rc<Style>,
    cache: Arc<Mutex<self::cache::Cache>>,
    viewport: Rectangle,
    redraw: bool,
    cursor: (f32, f32),
    event_loop: E,
    loader: Arc<L>,
    hot_reload_style: Option<String>,
}

/// Some asynchronous work that will update the ui later.
pub enum Command<Message> {
    /// Wait for a future to resolve
    Await(Box<dyn Future<Output = Message> + Send>),
    /// Handle all messages coming out of stream
    Subscribe(Box<dyn Stream<Item = Message> + Send>),
    /// Swap the stylesheet
    Stylesheet(Box<dyn Future<Output = Result<Style, stylesheet::Error>> + Send>),
}

impl<M: Model, E: EventLoop<Command<M::Message>>, L: Loader> Ui<M, E, L> {
    /// Constructs a new `Ui` using the default style.
    /// This is not recommended as the default style is very empty and only renders white text.
    pub fn new(model: M, event_loop: E, loader: L, viewport: Rectangle) -> Self {
        let cache = Arc::new(Mutex::new(self::cache::Cache::new(512, 0)));

        let style = Rc::new(Style::new(cache.clone()));

        Self {
            model_view: ModelView::new(model),
            style,
            cache,
            viewport,
            redraw: true,
            cursor: (0.0, 0.0),
            event_loop,
            hot_reload_style: None,
            loader: Arc::new(loader),
        }
    }

    /// Constructs a new `Ui` asynchronously by first fetching a stylesheet from a
    /// [.pwss](stylesheet/index.html) data source.
    pub async fn with_stylesheet<U: AsRef<str>>(
        model: M,
        event_loop: E,
        loader: L,
        url: U,
        viewport: Rectangle,
    ) -> Result<Self, stylesheet::Error> {
        let cache = Arc::new(Mutex::new(self::cache::Cache::new(512, 0)));

        let loader = Arc::new(loader);

        let style = Rc::new(Style::load(loader.clone(), url.as_ref(), cache.clone()).await?);

        let mut result = Self {
            model_view: ModelView::new(model),
            style,
            cache: cache.clone(),
            viewport,
            event_loop,
            redraw: true,
            cursor: (0.0, 0.0),
            hot_reload_style: Some(url.as_ref().to_string()),
            loader: loader.clone(),
        };

        let loader = loader.clone();
        let url = url.as_ref().to_string();
        result.command(Command::from_future_style(async move {
            loader.wait(&url).await.map_err(|e| stylesheet::Error::Io(Box::new(e)))?;
            Ok(Style::load(loader, url, cache).await?)
        }));

        Ok(result)
    }

    /// Replace the current stylesheet with a new one
    pub fn reload_stylesheet<U: 'static + AsRef<str> + Send>(&mut self, url: U) {
        let cache = self.cache.clone();
        let loader = self.loader.clone();
        self.hot_reload_style = Some(url.as_ref().to_string());
        self.command(Command::from_future_style(async move {
            Style::load(loader, url, cache).await
        }));
    }

    /// Resizes the viewport.
    /// This forces the view to be rerendered.
    pub fn resize(&mut self, viewport: Rectangle) {
        self.model_view.set_dirty();
        self.redraw = true;
        self.viewport = viewport;
    }

    /// Updates the model after a `Command` has resolved.
    pub fn command(&mut self, command: Command<M::Message>) {
        match command {
            Command::Await(mut fut) => {
                let ptr = fut.deref_mut() as *mut dyn Future<Output = M::Message>;
                let pin = unsafe { std::pin::Pin::new_unchecked(ptr.as_mut().unwrap()) };
                let waker = EventLoopWaker::new(self.event_loop.clone(), Command::Await(fut));
                match pin.poll(&mut std::task::Context::from_waker(&waker)) {
                    Poll::Ready(message) => {
                        self.update(message);
                    }
                    _ => (),
                }
            }

            Command::Subscribe(mut stream) => {
                let ptr = stream.deref_mut() as *mut dyn Stream<Item = M::Message>;
                let pin = unsafe { std::pin::Pin::new_unchecked(ptr.as_mut().unwrap()) };
                let waker = EventLoopWaker::new(self.event_loop.clone(), Command::Subscribe(stream));
                match pin.poll_next(&mut std::task::Context::from_waker(&waker)) {
                    Poll::Ready(Some(message)) => {
                        self.update(message);
                        waker.wake();
                    }
                    _ => (),
                }
            }

            Command::Stylesheet(mut fut) => {
                let ptr = fut.deref_mut() as *mut dyn Future<Output = Result<Style, stylesheet::Error>>;
                let pin = unsafe { std::pin::Pin::new_unchecked(ptr.as_mut().unwrap()) };
                let waker = EventLoopWaker::new(self.event_loop.clone(), Command::Stylesheet(fut));
                match pin.poll(&mut std::task::Context::from_waker(&waker)) {
                    Poll::Ready(style) => {
                        match style {
                            Ok(style) => {
                                self.style = Rc::new(style);
                                self.model_view.set_dirty();
                            }
                            Err(error) => {
                                eprintln!("Unable to load stylesheet: {}", error);
                            }
                        }

                        if let Some(url) = self.hot_reload_style.clone() {
                            let loader = self.loader.clone();
                            let cache = self.cache.clone();
                            let url = url.clone();
                            self.command(Command::from_future_style(async move {
                                loader.wait(&url).await.map_err(|e| stylesheet::Error::Io(Box::new(e)))?;
                                Ok(Style::load(loader, url, cache).await?)
                            }));
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    /// Updates the model with a message.
    /// This forces the view to be rerendered.
    pub fn update(&mut self, message: M::Message) {
        for command in self.model_view.model_mut().update(message) {
            self.command(command);
        }
    }

    /// Handles an [`Event`](event/struct.Event.html).
    pub fn event(&mut self, event: Event) {
        if let Event::Cursor(x, y) = event {
            self.cursor = (x, y);
        }

        let mut context = Context::new(self.redraw, self.cursor);

        {
            let view = self.model_view.view(self.style.clone());
            let (w, h) = view.size();
            let layout = Rectangle::from_wh(
                w.resolve(self.viewport.width(), w.parts()),
                h.resolve(self.viewport.height(), h.parts()),
            );
            view.event(layout, self.viewport, event, &mut context);
        }

        self.redraw = context.redraw_requested();

        for message in context {
            for command in self.model_view.model_mut().update(message) {
                self.command(command);
            }
        }
    }

    /// Returns true if the ui needs to be redrawn. If the ui doesn't need to be redrawn the
    /// [`Command`s](draw/struct.Command.html) from the last [`draw`](#method.draw) may be used again.
    pub fn needs_redraw(&self) -> bool {
        self.redraw || self.model_view.dirty()
    }

    /// Generate a [`DrawList`](draw/struct.DrawList.html) for the view.
    pub fn draw(&mut self) -> DrawList {
        use self::draw::*;

        let viewport = self.viewport;
        let primitives = {
            let view = self.model_view.view(self.style.clone());
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
        };

        impl Layer {
            fn append(&mut self, command: Command) {
                if let Some(next) = self.cmd.last_mut().unwrap().append(command) {
                    self.cmd.push(next);
                }
            }
        }

        let mut layers = vec![Layer { vtx: Vec::new(), cmd: vec![Command::Nop] }];
        let mut layer: usize = 0;

        let mut scissors = Vec::new();
        scissors.push(viewport);

        let validate_clip = move |clip: Rectangle| {
            let v = Rectangle {
                left: clip.left.max(0.0).min(viewport.right),
                top: clip.top.max(0.0).min(viewport.bottom),
                right: clip.right.max(0.0).min(viewport.right),
                bottom: clip.bottom.max(0.0).min(viewport.bottom),
            };
            if v.width() > 0.0 && v.height() > 0.0 {
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
                        layers.push(Layer { vtx: Vec::new(), cmd: vec![Command::Nop] });
                    }
                }

                Primitive::LayerDown => {
                    layer -= 1;
                }

                Primitive::DrawRect(r, color) => {
                    if draw_enabled {
                        let r = r.to_device_coordinates(viewport);
                        let color = [color.r, color.g, color.b, color.a];
                        let mode = 2;
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
                        let mode = 0;
                        let offset = layers[layer].vtx.len();

                        self.cache.lock().unwrap().draw_text(&text, rect, |uv, pos| {
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
                        let mode = 1;
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
                        let mode = 1;
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

        let (vertices, commands) = layers.into_iter().fold((Vec::new(), Vec::new()), |(mut vtx, mut cmd), mut layer| {
            let layer_offset = vtx.len();
            vtx.append(&mut layer.vtx);
            cmd.extend(layer.cmd.into_iter().map(|command| match command {
                Command::Textured { texture, offset, count } => {
                    Command::Textured { texture, offset: offset + layer_offset, count }
                }
                Command::Colored { offset, count } => {
                    Command::Colored { offset: offset + layer_offset, count }
                }
                other => other,
            }));
            (vtx, cmd)
        });

        DrawList {
            updates: self.cache.lock().unwrap().take_updates(),
            vertices,
            commands,
        }
    }
}

struct EventLoopWaker<T: Send, E: EventLoop<T>> {
    message: Mutex<(E, Option<T>)>,
}

impl<T: Send, E: EventLoop<T>> EventLoopWaker<T, E> {
    fn new(event_loop: E, message: T) -> Waker {
        futures::task::waker(Arc::new(Self {
            message: Mutex::new((event_loop, Some(message))),
        }))
    }
}

impl<T: Send, E: EventLoop<T>> ArcWake for EventLoopWaker<T, E> {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let mut guard = arc_self.message.lock().unwrap();
        if let Some(message) = guard.1.take() {
            guard.0.send_event(message).ok();
        }
    }
}

impl<M: Model, E: EventLoop<Command<M::Message>>, L: Loader> Deref for Ui<M, E, L> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.model_view.model()
    }
}

impl<M: Model, E: EventLoop<Command<M::Message>>, L: Loader> DerefMut for Ui<M, E, L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.model_view.model_mut()
    }
}

impl<Message> Command<Message> {
    /// Construct a new command from a future that generates a message.
    /// The message will be handled as soon as it is available.
    pub fn from_future_message<F: 'static + Future<Output=Message> + Send>(fut: F) -> Self {
        Command::Await(Box::new(fut))
    }

    /// Construct a new command that will replace the style of the Ui after it completes.
    pub fn from_future_style<F: 'static + Future<Output=Result<Style, stylesheet::Error>> + Send>(fut: F) -> Self {
        Command::Stylesheet(Box::new(fut))
    }

    /// Construct a new command from a stream of messages. Each message will be handled as soon as it's available.
    pub fn from_stream<S: 'static + Stream<Item=Message> + Send>(stream: S) -> Self {
        Command::Subscribe(Box::new(stream))
    }
}

/// prelude module for convenience
pub mod prelude {
    pub use crate::{layout::Rectangle, stylesheet::Style, tracker::ManagedState, widget::*, Model, Command, Ui};
}
