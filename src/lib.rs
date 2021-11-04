#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::task::Waker;

use node::GenericNode;
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

/// Entry point for pixel-widgets.
///
/// `Ui` manages a root [`Component`](component/trait.Component.html) and processes it to a
/// [`DrawList`](draw/struct.DrawList.html) that can be rendered using your own renderer implementation.
/// Alternatively, you can use one of the following included wrappers:
/// - [`wgpu::Ui`](backend/wgpu/struct.Ui.html) Renders using [wgpu](https://github.com/gfx-rs/wgpu).
pub struct Ui<M: 'static + Component> {
    root_node: ComponentNode<'static, M>,
    _state: ManagedState,
    viewport: Rectangle,
    redraw: bool,
    cursor: (f32, f32),
    style: Arc<Style>,
    hidpi_scale: f32,
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
            root_node,
            _state: state,
            viewport: Rectangle {
                left: viewport.left / hidpi_scale,
                top: viewport.top / hidpi_scale,
                right: viewport.right / hidpi_scale,
                bottom: viewport.bottom / hidpi_scale,
            },
            redraw: true,
            cursor: (0.0, 0.0),
            style,
            hidpi_scale,
        })
    }

    /// Resizes the viewport.
    /// This forces the view to be rerendered.
    pub fn resize(&mut self, viewport: Rectangle) {
        self.root_node.set_dirty();
        self.redraw = true;
        self.viewport = Rectangle {
            left: viewport.left / self.hidpi_scale,
            top: viewport.top / self.hidpi_scale,
            right: viewport.right / self.hidpi_scale,
            bottom: viewport.bottom / self.hidpi_scale,
        };
    }

    /// Returns true if the ui needs to be redrawn. If the ui doesn't need to be redrawn the
    /// [`Command`s](draw/struct.Command.html) from the last [`draw`](#method.draw) may be used again.
    pub fn needs_redraw(&self) -> bool {
        self.redraw || self.root_node.dirty()
    }

    /// Updates the model with a message.
    /// This forces the view to be rerendered.
    pub fn update_poll(&mut self, message: C::Message, waker: Waker) -> Vec<C::Output> {
        let mut context = Context::new(self.needs_redraw(), self.cursor, waker);

        self.root_node.update(message, &mut context);
        while self.root_node.needs_poll() {
            self.root_node.poll(&mut context);
        }

        self.redraw = context.redraw_requested();
        context.into_vec()
    }

    /// Handles an [`Event`](event/struct.Event.html).
    pub fn event(&mut self, mut event: Event, waker: Waker) -> Vec<C::Output> {
        if let Event::Cursor(x, y) = event {
            event = Event::Cursor(x / self.hidpi_scale, y / self.hidpi_scale);
            self.cursor = (x / self.hidpi_scale, y / self.hidpi_scale);
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

        let mut outer_context = Context::new(self.needs_redraw(), self.cursor, waker);

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
    pub fn poll(&mut self, waker: Waker) -> Vec<C::Output> {
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

        let scale = self.hidpi_scale;
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

impl<C: Component> Deref for Ui<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        self.root_node.props()
    }
}

impl<C: Component> DerefMut for Ui<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.root_node.props_mut()
    }
}
