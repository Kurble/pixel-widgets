use std::future::Future;
use std::ops::{Deref, DerefMut};

use draw::DrawList;

use crate::element::Node;
use crate::event::Event;
use crate::layout::Rectangle;
use crate::stylesheet::{Query, Style};

pub mod backend;
pub mod cache;
pub mod draw;
pub mod element;
pub mod event;
pub mod layout;
pub mod qtree;
pub mod stylesheet;
pub mod text;

pub trait Model {
    type Message;

    fn update(&mut self, message: Self::Message);

    fn view(&mut self) -> Node<Self::Message>;
}

pub trait Loader: Send + Sync {
    type Load: Future<Output = Result<Vec<u8>, Self::Error>> + Send + Sync;
    type Error: std::error::Error;

    fn load(&self, url: impl AsRef<str>) -> Self::Load;
}

pub struct Ui<I> {
    model: I,
    cache: self::cache::Cache,
    events: Vec<Event>,
    stylesheet: Style,
}

impl<I: Model> Ui<I> {
    pub fn new(model: I) -> Self {
        let mut cache = self::cache::Cache::new(512, 0);

        let stylesheet = Style::new(&mut cache);

        Self {
            model,
            cache,
            events: Vec::new(),
            stylesheet,
        }
    }

    pub async fn with_stylesheet(model: I, loader: impl Loader, url: impl AsRef<str>) -> Self {
        let mut cache = self::cache::Cache::new(512, 0);

        let stylesheet = Style::load(&loader, url, &mut cache).await.unwrap();

        Self {
            model,
            cache,
            events: Vec::new(),
            stylesheet,
        }
    }

    pub fn update(&mut self, message: I::Message) {
        self.model.update(message);
    }

    pub fn event(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn render(&mut self, viewport: Rectangle) -> DrawList {
        use self::draw::*;

        let mut root = self.model.view();
        root.style(&mut self.stylesheet, &mut Query::default());
        let (w, h) = root.size();
        let layout = Rectangle::from_wh(
            w.resolve(viewport.width(), w.parts()),
            h.resolve(viewport.height(), h.parts()),
        );
        let messages = self
            .events
            .drain(..)
            .filter_map(|event| root.event(layout, event, viewport))
            .collect::<Vec<_>>();
        let primitives = root.render(layout);

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

        drop(root);
        for message in messages {
            self.model.update(message);
        }

        DrawList {
            updates: self.cache.take_updates(),
            vertices: vtx,
            commands: cmd,
        }
    }
}

impl<I> Deref for Ui<I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.model
    }
}

impl<I> DerefMut for Ui<I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.model
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
