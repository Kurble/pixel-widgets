#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use std::collections::VecDeque;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard};

use futures::future::poll_fn;
use graphics::Graphics;
use node::GenericNode;
use owning_ref::{MutexGuardRef, MutexGuardRefMut};
use widget::Context;

use crate::component::Component;
use crate::draw::DrawList;
use crate::event::Event;
use crate::layout::Rectangle;
use crate::node::component_node::ComponentNode;
use crate::style::tree::Query;
use crate::style::Style;
use crate::tracker::ManagedState;

mod atlas;
/// Backend specific code
pub mod backend;
mod bitset;
/// Texture cache for styles and text
pub mod cache;
/// The component trait.
pub mod component;
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
/// Prelude module for pixel-widgets.
pub mod prelude;
/// Simple windowing system for those who want to render _just_ widgets.
#[cfg(feature = "winit")]
#[cfg(feature = "wgpu")]
pub mod sandbox;
/// Styling system
pub mod style;
/// Primitives for rendering text
pub mod text;
/// Utility for tracking state conveniently.
pub mod tracker;
/// User interface widgets
pub mod widget;

/// Entry point for the user interface.
///
/// `Ui` manages a root [`Component`](component/trait.Component.html) and processes it to a
/// [`DrawList`](draw/struct.DrawList.html) that can be rendered using your own renderer implementation.
/// Alternatively, you can use one of the following included wrappers:
/// - [`wgpu::Ui`](backend/wgpu/struct.Ui.html) Renders using [wgpu](https://github.com/gfx-rs/wgpu).
///
/// # Async support
/// Components can submit futures to the [`Context`](component/struct.Context.html) using
/// [`wait()`](component/struct.Context.html#method.wait) and
/// [`stream()`](component/struct.Context.html#method.stream). These futures will update
/// those components when they complete or yield messages.
/// To support this, you must make sure that the poll method on `Ui` is called appropriately
/// since the `Ui` can't be submitted to a typical executor.
///
/// The [`Sandbox`](sandbox/struct.Sandbox.html) can serve as an example since it
/// provides such a runtime for you through the winit event loop.
pub struct Ui<C: 'static + Component> {
    data: Arc<Mutex<Data<C>>>,
    style: Arc<Style>,
    task_created: bool,
    viewport: Rectangle,
    hidpi_scale: f32,
}

struct Data<C: 'static + Component> {
    #[allow(unused)]
    state: ManagedState,
    root_node: ComponentNode<'static, C>,
    viewport: Rectangle,
    redraw: bool,
    cursor: (f32, f32),
    hidpi_scale: f32,
    output: VecDeque<C::Output>,
}

impl<C: 'static + Component> Ui<C> {
    /// Constructs a new `Ui`. Returns an error if the style fails to load.
    pub fn new<S, E>(root: C, viewport: Rectangle, hidpi_scale: f32, style: S) -> anyhow::Result<Self>
    where
        S: TryInto<Style, Error = E>,
        anyhow::Error: From<E>,
    {
        let mut state = ManagedState::default();
        let mut root_node = ComponentNode::new(root);
        root_node.acquire_state(&mut unsafe { (&mut state as *mut ManagedState).as_mut() }.unwrap().tracker());

        let style = Arc::new(style.try_into()?);
        root_node.set_dirty();
        root_node.style(&mut Query::from_style(style.clone()), (0, 1));

        Ok(Self {
            data: Arc::new(Mutex::new(Data {
                root_node,
                state,
                viewport: Rectangle {
                    left: viewport.left / hidpi_scale,
                    top: viewport.top / hidpi_scale,
                    right: viewport.right / hidpi_scale,
                    bottom: viewport.bottom / hidpi_scale,
                },
                redraw: true,
                cursor: (0.0, 0.0),
                hidpi_scale,
                output: Default::default(),
            })),
            style,
            task_created: false,
            viewport: Rectangle {
                left: viewport.left / hidpi_scale,
                top: viewport.top / hidpi_scale,
                right: viewport.right / hidpi_scale,
                bottom: viewport.bottom / hidpi_scale,
            },
            hidpi_scale,
        })
    }

    /// Retrieve a `Graphics` loader that can be used to load images
    pub fn graphics(&self) -> Graphics {
        self.style.graphics()
    }

    /// Create a task that will drive all ui futures.
    /// Takes an `on_redraw` closure that will be called to wake up the main thread for redrawing the ui when required.
    /// This method will panic if it's called a second time.
    pub fn task(&mut self, mut on_redraw: impl FnMut()) -> impl Future<Output = ()> {
        assert!(!self.task_created);
        self.task_created = true;

        let data = self.data.clone();
        poll_fn(move |cx| {
            if let Ok(mut data) = data.lock() {
                let mut context = Context::new(false, data.cursor);
                data.root_node.poll(&mut context, cx);
                if context.redraw_requested() {
                    (on_redraw)();
                    data.redraw = true;
                }
                data.output.extend(context);

                std::task::Poll::Pending
            } else {
                std::task::Poll::Ready(())
            }
        })
    }

    /// Updates the root component with a message.
    pub fn update(&mut self, message: C::Message) {
        let mut data = self.data.lock().unwrap();
        let mut context = Context::new(data.redraw, data.cursor);
        data.root_node.update(message, &mut context);
        data.redraw |= context.redraw_requested();
        data.output.extend(context);
    }

    /// Handles a ui [`Event`](event/struct.Event.html).
    /// If the ui has any pending futures internally, they are polled using the waker.
    /// It's up to the user to make sure that the `waker` will schedule a call to [`poll()`](#method.poll) on this `Ui`.
    ///
    /// Returns `true` if the event was handled in a way that it's captured by the ui.
    pub fn handle_event(&mut self, mut event: Event) -> bool {
        let mut data = self.data.lock().unwrap();

        if let Event::Cursor(x, y) = event {
            event = Event::Cursor(x / data.hidpi_scale, y / data.hidpi_scale);
            data.cursor = (x / data.hidpi_scale, y / data.hidpi_scale);
        }

        let mut context = Context::new(data.redraw, data.cursor);

        let result = {
            let mut view = data.root_node.view();
            let (w, h) = view.size();
            let layout = Rectangle::from_wh(
                w.resolve(data.viewport.width(), w.parts()),
                h.resolve(data.viewport.height(), h.parts()),
            );
            view.event(layout, data.viewport, event, &mut context);
            view.focused()
        };

        data.redraw |= context.redraw_requested();

        let mut outer_context = Context::new(data.redraw, data.cursor);

        for message in context {
            data.root_node.update(message, &mut outer_context);
        }

        data.redraw |= outer_context.redraw_requested();
        data.output.extend(outer_context);

        result
    }

    /// Resizes the viewport.
    /// This forces the view to be rerendered, but only if the size actually changed.
    pub fn resize(&mut self, viewport: Rectangle, hidpi_scale: f32) {
        let viewport = Rectangle {
            left: viewport.left / hidpi_scale,
            top: viewport.top / hidpi_scale,
            right: viewport.right / hidpi_scale,
            bottom: viewport.bottom / hidpi_scale,
        };
        if self.viewport != viewport || self.hidpi_scale != hidpi_scale {
            self.viewport = viewport;
            self.hidpi_scale = hidpi_scale;
            let mut data = self.data.lock().unwrap();
            data.root_node.set_dirty();
            data.redraw = true;
            data.hidpi_scale = hidpi_scale;
            data.viewport = viewport;
        }
    }

    /// Check whether any widget in the ui has input focus
    pub fn focused(&self) -> bool {
        let data = self.data.lock().unwrap();
        let view = data.root_node.view();
        view.focused()
    }

    /// Perform a hitdetect on the root component,
    ///  to see if a future pointer event would be handled
    pub fn hit(&self, x: f32, y: f32) -> bool {
        let data = self.data.lock().unwrap();
        let view = data.root_node.view();
        let (w, h) = view.size();
        let layout = Rectangle::from_wh(
            w.resolve(data.viewport.width(), w.parts()),
            h.resolve(data.viewport.height(), h.parts()),
        );
        view.hit(layout, data.viewport, x, y, true)
    }

    /// Return an immutable reference to the root component
    pub fn props(&self) -> impl '_ + Deref<Target = C> {
        MutexGuardRef::new(self.data.lock().unwrap()).map(|d| d.root_node.props())
    }

    /// Return a mutable reference to the root component
    pub fn props_mut(&mut self) -> impl '_ + DerefMut<Target = C> {
        let mut lock = self.data.lock().unwrap();
        lock.redraw = true;
        MutexGuardRefMut::new(lock).map_mut(|d| d.root_node.props_mut())
    }

    /// Returns an iterator over the output messages produced by the root component.
    pub fn output(&mut self) -> impl '_ + Iterator<Item = C::Output> {
        Output(self.data.lock().unwrap())
    }

    /// Returns true if the ui needs to be redrawn. If the ui doesn't need to be redrawn the
    /// [`Command`s](draw/struct.Command.html) from the last [`draw`](#method.draw) may be used again.
    pub fn needs_redraw(&self) -> bool {
        let data = self.data.lock().unwrap();
        data.redraw || data.root_node.dirty()
    }

    /// Generate a [`DrawList`](draw/struct.DrawList.html) for the view.
    pub fn draw(&mut self) -> DrawList {
        use self::draw::*;

        let mut data = self.data.lock().unwrap();

        let viewport = data.viewport;
        let viewport_center = (
            (viewport.left + viewport.right) * 0.5,
            (viewport.top + viewport.bottom) * 0.5,
        );
        let viewport_inverse_size = (
            ((viewport.right - viewport.left) * 0.5).recip(),
            ((viewport.top - viewport.bottom) * -0.5).recip(),
        );

        let primitives = {
            let mut view = data.root_node.view();
            let (w, h) = view.size();
            let layout = Rectangle::from_wh(
                w.resolve(viewport.width(), w.parts()),
                h.resolve(viewport.height(), h.parts()),
            );
            view.draw(layout, viewport)
        };
        data.redraw = false;

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

        let scale = data.hidpi_scale;
        let validate_clip = move |clip: Rectangle| {
            let v = Rectangle {
                left: clip.left.max(0.0).min(viewport.right) * scale,
                top: clip.top.max(0.0).min(viewport.bottom) * scale,
                right: clip.right.max(0.0).min(viewport.right) * scale,
                bottom: clip.bottom.max(0.0).min(viewport.bottom) * scale,
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

                Primitive::DrawTriangle(vtx, color) => {
                    if draw_enabled {
                        let color = [color.r, color.g, color.b, color.a];
                        let mode = 1.0;
                        let offset = layers[layer].vtx.len();
                        layers[layer].vtx.extend(vtx.map(|[x, y]| Vertex {
                            pos: [
                                (x - viewport_center.0) * viewport_inverse_size.0,
                                (y - viewport_center.1) * viewport_inverse_size.1,
                            ],
                            uv: [0.0; 2],
                            color,
                            mode,
                        }));
                        layers[layer].append(Command::Colored { offset, count: 3 });
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

        drop(data);
        self.handle_event(Event::Animate);

        DrawList {
            updates: self.style.cache().lock().unwrap().take_updates(),
            vertices,
            commands,
        }
    }
}

struct Output<'a, C: 'static + Component>(MutexGuard<'a, Data<C>>);

impl<'a, C: 'static + Component> Iterator for Output<'a, C> {
    type Item = C::Output;

    fn next(&mut self) -> Option<C::Output> {
        self.0.output.pop_front()
    }
}
