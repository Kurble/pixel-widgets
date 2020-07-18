//! Documentation here
#![deny(missing_docs)]

use std::future::Future;
use std::ops::{Deref, DerefMut};

use draw::DrawList;

use crate::element::{Context, Node};
use crate::event::Event;
use crate::layout::Rectangle;
use crate::model_view::ModelView;
use crate::stylesheet::Style;

/// Backend specific code
pub mod backend;
mod cache;
/// Primitives used for drawing
pub mod draw;
/// User interface elements
pub mod element;
/// User input events
pub mod event;
/// Primitives used for layouts
pub mod layout;
mod model_view;
mod qtree;
/// Styling system
#[allow(missing_docs)]
pub mod stylesheet;
/// Primitives for rendering text
pub mod text;
/// Utility for tracking state conveniently.
pub mod tracker;

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
    fn update(&mut self, message: Self::Message);

    /// Called after [`update`](#tymethod.update) or after the model has been accessed mutably from the [`Ui`](struct.Ui.html).
    /// This is where you should build all of your ui elements based on the current gui state.
    /// The returned ui elements produce messages of the type `Self::Message`.
    fn view(&mut self) -> Node<Self::Message>;
}

/// A way to load URLs from a data source.
///
/// Two implementations are included:
/// - `PathBuf`, loads data from disk using the `PathBuf` as working directory.
/// - `Url`, loads data over HTTP using reqwest using the `Url` as base.
pub trait Loader: Send + Sync {
    /// A future returned when calling `load`.
    type Load: Future<Output = Result<Vec<u8>, Self::Error>> + Send + Sync;
    /// Error returned by the loader when the request failed.
    type Error: std::error::Error;

    /// Asynchronously load a resource located at the given url
    fn load(&self, url: impl AsRef<str>) -> Self::Load;
}

/// Entry point for maple. Manages a [`Model`](trait.Model.html) and processes it to a [`DrawList`](draw/struct.DrawList.html) that can be rendered using your
///  own renderer implementation. Alternatively, you can use one of the following included wrappers:
/// - [`WgpuUi`](backend/wgpu/struct.WgpuUi.html) Renders using [wgpu-rs](https://github.com/gfx-rs/wgpu-rs).
/// - [`GlowUi`](backend/glow/struct.GlowUi.html) Renders using [glow](https://github.com/grovesNL/glow).
pub struct Ui<I: Model> {
    model_view: ModelView<I>,
    style: Style,
    cache: self::cache::Cache,
    viewport: Rectangle,
    redraw: bool,
}

impl<I: Model> Ui<I> {
    /// Constructs a new `Ui` using the default style.
    /// This is not recommended as the default style is very empty and only renders white text.
    pub fn new(model: I, viewport: Rectangle) -> Self {
        let mut cache = self::cache::Cache::new(512, 0);

        let style = Style::new(&mut cache);

        Self {
            model_view: ModelView::new(model),
            style,
            cache,
            viewport,
            redraw: true,
        }
    }

    /// Constructs a new `Ui` asynchronously by first fetching a stylesheet for a [.mss] data source.
    pub async fn with_stylesheet<L: Loader, U: AsRef<str>>(
        model: I,
        loader: L,
        url: U,
        viewport: Rectangle,
    ) -> Result<Self, stylesheet::Error<L::Error>> {
        let mut cache = self::cache::Cache::new(512, 0);

        let style = Style::load(&loader, url, &mut cache).await?;

        Ok(Self {
            model_view: ModelView::new(model),
            style,
            cache,
            viewport,
            redraw: true,
        })
    }

    /// Resizes the viewport.
    /// This forces the view to be rerendered.
    pub fn resize(&mut self, viewport: Rectangle) {
        self.model_view.set_dirty();
        self.viewport = viewport;
    }

    /// Updates the model with a message.
    /// This forces the view to be rerendered.
    pub fn update(&mut self, message: I::Message) {
        self.model_view.model_mut().update(message);
    }

    /// Handles an [`Event`](event/struct.Event.html).
    pub fn event(&mut self, event: Event) {
        let mut context = Context::new(self.redraw);

        {
            let view = self.model_view.view(&mut self.style);
            let (w, h) = view.size();
            let layout = Rectangle::from_wh(
                w.resolve(self.viewport.width(), w.parts()),
                h.resolve(self.viewport.height(), h.parts()),
            );
            view.event(layout, self.viewport, event, &mut context);
        }

        self.redraw = context.redraw_requested();

        for message in context {
            self.model_view.model_mut().update(message);
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
            let view = self.model_view.view(&mut self.style);
            let (w, h) = view.size();
            let layout = Rectangle::from_wh(
                w.resolve(viewport.width(), w.parts()),
                h.resolve(viewport.height(), h.parts()),
            );
            view.draw(layout, viewport)
        };
        self.redraw = false;

        let mut vtx = Vec::new();
        let mut cmd = Vec::new();

        let mut scissors = Vec::new();
        scissors.push(viewport);

        let mut current_command = Command::Nop;

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
                        current_command
                            .append(Command::Clip { scissor: s })
                            .and_then(|c| Some(cmd.push(c)));

                        true
                    });
                }

                Primitive::PopClip => {
                    scissors.pop();
                    let scissor = scissors[scissors.len() - 1];

                    draw_enabled = validate_clip(scissor).map_or(false, |s| {
                        current_command
                            .append(Command::Clip { scissor: s })
                            .and_then(|c| Some(cmd.push(c)));

                        true
                    });
                }

                Primitive::DrawRect(r, color) => {
                    if draw_enabled {
                        let r = r.to_device_coordinates(viewport);
                        let color = [color.r, color.g, color.b, color.a];
                        let mode = 2;
                        let offset = vtx.len();
                        vtx.push(Vertex {
                            pos: [r.left, r.top],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        vtx.push(Vertex {
                            pos: [r.right, r.top],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        vtx.push(Vertex {
                            pos: [r.right, r.bottom],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        vtx.push(Vertex {
                            pos: [r.left, r.top],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        vtx.push(Vertex {
                            pos: [r.right, r.bottom],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });
                        vtx.push(Vertex {
                            pos: [r.left, r.bottom],
                            uv: [0.0; 2],
                            color,
                            mode,
                        });

                        current_command
                            .append(Command::Colored { offset, count: 6 })
                            .and_then(|c| Some(cmd.push(c)));
                    }
                }

                Primitive::DrawText(text, rect) => {
                    if draw_enabled {
                        let color = [text.color.r, text.color.g, text.color.b, text.color.a];
                        let mode = 0;
                        let offset = vtx.len();

                        self.cache.draw_text(&text, rect, |uv, pos| {
                            let rc = Rectangle {
                                left: pos.left,
                                top: pos.top,
                                right: pos.right,
                                bottom: pos.bottom,
                            }
                            .to_device_coordinates(viewport);

                            vtx.push(Vertex {
                                pos: [rc.left, rc.top],
                                uv: uv.pt(0.0, 0.0),
                                color,
                                mode,
                            });
                            vtx.push(Vertex {
                                pos: [rc.right, rc.top],
                                uv: uv.pt(1.0, 0.0),
                                color,
                                mode,
                            });
                            vtx.push(Vertex {
                                pos: [rc.right, rc.bottom],
                                uv: uv.pt(1.0, 1.0),
                                color,
                                mode,
                            });
                            vtx.push(Vertex {
                                pos: [rc.left, rc.top],
                                uv: uv.pt(0.0, 0.0),
                                color,
                                mode,
                            });
                            vtx.push(Vertex {
                                pos: [rc.right, rc.bottom],
                                uv: uv.pt(1.0, 1.0),
                                color,
                                mode,
                            });
                            vtx.push(Vertex {
                                pos: [rc.left, rc.bottom],
                                uv: uv.pt(0.0, 1.0),
                                color,
                                mode,
                            });
                        });

                        current_command
                            .append(Command::Textured {
                                texture: text.font.tex_slot,
                                offset,
                                count: vtx.len() - offset,
                            })
                            .and_then(|c| Some(cmd.push(c)));
                    }
                }

                Primitive::Draw9(patch, rect, color) => {
                    if draw_enabled {
                        let uv = patch.image.texcoords;
                        let color = [color.r, color.g, color.b, color.a];
                        let mode = 1;
                        let offset = vtx.len();

                        patch.iterate_sections(false, rect.width(), |x, u| {
                            patch.iterate_sections(true, rect.height(), |y, v| {
                                let rc = Rectangle {
                                    left: x.0 + rect.left,
                                    right: x.1 + rect.left,
                                    top: y.0 + rect.top,
                                    bottom: y.1 + rect.top,
                                }
                                .to_device_coordinates(viewport);

                                vtx.push(Vertex {
                                    pos: [rc.left, rc.top],
                                    uv: uv.pt(u.0, v.0),
                                    color,
                                    mode,
                                });
                                vtx.push(Vertex {
                                    pos: [rc.right, rc.top],
                                    uv: uv.pt(u.1, v.0),
                                    color,
                                    mode,
                                });
                                vtx.push(Vertex {
                                    pos: [rc.right, rc.bottom],
                                    uv: uv.pt(u.1, v.1),
                                    color,
                                    mode,
                                });
                                vtx.push(Vertex {
                                    pos: [rc.left, rc.top],
                                    uv: uv.pt(u.0, v.0),
                                    color,
                                    mode,
                                });
                                vtx.push(Vertex {
                                    pos: [rc.right, rc.bottom],
                                    uv: uv.pt(u.1, v.1),
                                    color,
                                    mode,
                                });
                                vtx.push(Vertex {
                                    pos: [rc.left, rc.bottom],
                                    uv: uv.pt(u.0, v.1),
                                    color,
                                    mode,
                                });
                            });
                        });

                        current_command
                            .append(Command::Textured {
                                texture: patch.image.texture,
                                offset,
                                count: vtx.len() - offset,
                            })
                            .and_then(|c| Some(cmd.push(c)));
                    }
                }

                Primitive::DrawImage(image, r, color) => {
                    if draw_enabled {
                        let r = r.to_device_coordinates(viewport);
                        let uv = image.texcoords;
                        let color = [color.r, color.g, color.b, color.a];
                        let mode = 1;
                        let offset = vtx.len();

                        vtx.push(Vertex {
                            pos: [r.left, r.top],
                            uv: [uv.left, uv.top],
                            color,
                            mode,
                        });
                        vtx.push(Vertex {
                            pos: [r.right, r.top],
                            uv: [uv.right, uv.top],
                            color,
                            mode,
                        });
                        vtx.push(Vertex {
                            pos: [r.right, r.bottom],
                            uv: [uv.right, uv.bottom],
                            color,
                            mode,
                        });
                        vtx.push(Vertex {
                            pos: [r.left, r.top],
                            uv: [uv.left, uv.top],
                            color,
                            mode,
                        });
                        vtx.push(Vertex {
                            pos: [r.right, r.bottom],
                            uv: [uv.right, uv.bottom],
                            color,
                            mode,
                        });
                        vtx.push(Vertex {
                            pos: [r.left, r.bottom],
                            uv: [uv.left, uv.bottom],
                            color,
                            mode,
                        });

                        current_command
                            .append(Command::Textured {
                                texture: image.texture,
                                offset,
                                count: 6,
                            })
                            .and_then(|c| Some(cmd.push(c)));
                    }
                }
            }
        }

        // Flush any commands that are not finalized
        current_command.flush().and_then(|c| Some(cmd.push(c)));

        DrawList {
            updates: self.cache.take_updates(),
            vertices: vtx,
            commands: cmd,
        }
    }
}

impl<I: Model> Deref for Ui<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.model_view.model()
    }
}

impl<I: Model> DerefMut for Ui<I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.model_view.model_mut()
    }
}

impl Loader for std::path::PathBuf {
    type Load = futures::future::Ready<Result<Vec<u8>, Self::Error>>;
    type Error = std::io::Error;

    fn load(&self, url: impl AsRef<str>) -> Self::Load {
        let path = self.join(std::path::Path::new(url.as_ref()));
        futures::future::ready(std::fs::read(path))
    }
}

/// prelude module for convenience
pub mod prelude {
    pub use crate::{
        backend::{wgpu::WgpuUi, winit::convert_event},
        element::*,
        layout::Rectangle,
        stylesheet::Style,
        tracker::ManagedState,
        Model, Ui,
    };
}
