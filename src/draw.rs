use crate::layout::Rectangle;
use crate::text::Text;
use smallvec::SmallVec;
use zerocopy::AsBytes;

#[derive(Clone)]
pub enum Primitive<'a> {
    PushClip(Rectangle),
    PopClip,
    DrawRect(Rectangle, Color),
    DrawText(Text<'a>, Rectangle),
    Draw9(Patch, Rectangle, Color),
    DrawImage(Image, Rectangle, Color),
}

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// An image in an im9 atlas texture.
#[derive(Clone, Debug)]
pub struct Image {
    /// The texture atlas identifier that this image resides in.
    pub texture: usize,
    /// The texcoords within the atlas that the image spans.
    pub texcoords: Rectangle,
    /// The physical size in pixels of the image.
    pub size: Rectangle,
}

/// 9 patch data on top of an `Image`, which is used to create dynamically stretchable images.
#[derive(Clone, Debug)]
pub struct Patch {
    /// The `Image` this `Patch` operates on.
    pub image: Image,
    /// Horizontally stretchable regions in the 9 patch image.
    /// Every element is a pair of begin and end of the stretchable region.
    /// Defined in relative coordinates: 0.0 is the left side of the image,
    /// 1.0 is the right side of the image.
    pub h_stretch: SmallVec<[(f32, f32); 2]>,
    /// Vertically stretchable regions in the 9 patch image.
    /// Every element is a pair of begin and end of the stretchable region.
    /// Defined in relative coordinates: 0.0 is the top side of the image,
    /// 1.0 is the bottom side of the image.
    pub v_stretch: SmallVec<[(f32, f32); 2]>,
    /// Horizontal content area in the 9 patch image. Content can be placed
    /// in the region defined here.
    /// Defined in relative coordinates: 0.0 is the left side of the image,
    /// 1.0 is the right side of the image.
    pub h_content: (f32, f32),
    /// Vertical content area in the 9 patch image. Content can be placed
    ///  in the region defined here.
    /// Defined in relative coordinates: 0.0 is the top side of the image,
    /// 1.0 is the bottom side of the image.
    pub v_content: (f32, f32),
}

#[derive(Clone)]
pub enum Background {
    None,
    Color(Color),
    Image(Image, Color),
    Patch(Patch, Color),
}

/// A collection of data needed to render the `Ui` once.
pub struct DrawList {
    /// A list of texture updates that need to be uploaded before rendering.
    pub updates: Vec<Update>,
    /// The vertex buffer used for this frame.
    pub vertices: Vec<Vertex>,
    /// A list of draw commands that use the `vertices` buffer.
    pub commands: Vec<Command>,
}

/// An update of the available texture data. The backend is responsible for uploading the provided
/// data to the GPU.
pub enum Update {
    /// A subresource of an existing texture is updated. This happens f.e. when new glyphs are
    /// loaded or when a new 9 patch is used.
    TextureSubresource {
        /// The id of the texture that needs to be updated
        id: usize,
        /// Offset from the left top corner of the texture.
        offset: [u32; 2],
        /// Size of the rect described by `data`
        size: [u32; 2],
        /// The texel data of the updated rect. 4 elements per pixel.
        data: Vec<u8>,
    },
    /// A new texture is introduced. This happens when f.e. a background image was loaded, or when
    /// the `Ui` is used for the first time.
    Texture {
        /// The id for the new texture. This is the id that will later be used to identify which
        /// texture the backend has to use whenever applicable.
        id: usize,
        /// Size of the texture
        size: [u32; 2],
        /// The texel data of the texture. 4 elements per pixel
        data: Vec<u8>,
        /// Whether the texture will be used as atlas. `true` means the texture might be updated
        /// later with `Update::TextureSubresource`, while `false` means the texture is immutable.
        atlas: bool,
    },
}

/// The `Vertex` type passed to the vertex shader.
#[derive(Debug, Clone, Copy, AsBytes)]
#[repr(packed)]
pub struct Vertex {
    /// The position of the vertex within device coordinates.
    /// [-1.0, -1.0] is the left top position of the display.
    pub pos: [f32; 2],
    /// The coordinates of the texture used by this `Vertex`.
    /// [0.0, 0.0] is the left top position of the texture.
    pub uv: [f32; 2],
    /// A color associated with the `Vertex`.
    /// The color is multiplied by the end result of the fragment shader.
    /// When `mode` is not 1, the default value is white ([1.0; 4])
    pub color: [f32; 4],
    /// The mode with which the `Vertex` will be drawn within the fragment shader.
    ///
    /// `0` for rendering text.
    /// `1` for rendering an image.
    /// `2` for rendering non-textured 2D geometry.
    ///
    /// If any other value is given, the fragment shader will not output any color.
    pub mode: u32,
}

/// A draw `Command` that is to be translated to a draw command specific to the backend
#[derive(Debug, Clone, Copy)]
pub enum Command {
    /// Do nothing. Appending a `Nop` to another command will flush the other command.
    Nop,
    /// Sets a new scissor rect, which is used to confine geometry to a certain area on screen.
    Clip { scissor: Rectangle },
    /// Draw a list of vertices without an active texture
    Colored { offset: usize, count: usize },
    /// Draw a list of vertices with the active texture denoted by it's index
    Textured {
        texture: usize,
        offset: usize,
        count: usize,
    },
}

impl Color {
    pub fn white() -> Color {
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
    pub fn black() -> Color {
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }
    pub fn red() -> Color {
        Color {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }
    pub fn green() -> Color {
        Color {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        }
    }
    pub fn blue() -> Color {
        Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        }
    }
    pub fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }
}

impl Patch {
    pub fn measure_with_content(&self, measured_content: Rectangle) -> Rectangle {
        let patch_content = self.image.size.sub(Rectangle {
            left: self.h_content.0,
            right: self.h_content.1,
            top: self.v_content.0,
            bottom: self.v_content.1,
        });

        let grow_x = (measured_content.width() - patch_content.width()).max(0.0);
        let grow_y = (measured_content.height() - patch_content.height()).max(0.0);

        let result = Rectangle {
            left: 0.0,
            top: 0.0,
            right: self.image.size.width() + grow_x,
            bottom: self.image.size.height() + grow_y,
        };

        result
    }
    pub fn margin(&self) -> Rectangle {
        let patch_content = self.image.size.sub(Rectangle {
            left: self.h_content.0,
            right: self.h_content.1,
            top: self.v_content.0,
            bottom: self.v_content.1,
        });

        Rectangle {
            left: patch_content.left,
            right: self.image.size.right - patch_content.right,
            top: patch_content.top,
            bottom: self.image.size.bottom - patch_content.bottom,
        }
    }
    pub fn minimum_size(&self) -> (f32, f32) {
        let margin = self.margin();
        (
            self.image.size.width() - margin.left - margin.right,
            self.image.size.height() - margin.top - margin.bottom,
        )
    }
    pub fn content_rect(&self, span: Rectangle) -> Rectangle {
        let mut result = span;

        let blend = |(a, b), x| a + (b - a) * x;
        let unblend = |x, (a, b)| (x - a) / (b - a);

        self.iterate_sections(false, span.width(), |x, u| {
            if self.h_content.0 >= u.0 && self.h_content.0 < u.1 {
                result.left = span.left + blend(x, unblend(self.h_content.0, u));
            }
            if self.h_content.1 > u.0 && self.h_content.1 <= u.1 {
                result.right = span.left + blend(x, unblend(self.h_content.1, u));
            }
        });
        self.iterate_sections(true, span.height(), |y, v| {
            if self.v_content.0 >= v.0 && self.v_content.0 < v.1 {
                result.top = span.top + blend(y, unblend(self.v_content.0, v));
            }
            if self.v_content.1 > v.0 && self.v_content.1 <= v.1 {
                result.bottom = span.top + blend(y, unblend(self.v_content.1, v));
            }
        });

        result
    }

    pub fn iterate_sections<F: FnMut((f32, f32), (f32, f32))>(&self, vertical: bool, length: f32, mut callback: F) {
        let stretches = if vertical { &self.v_stretch } else { &self.h_stretch };

        let total = stretches.iter().fold(0.0, |t, &(a, b)| t + (b - a));

        let mut cursor = 0.0;
        let mut grow = 0.0;

        let base = if vertical {
            (0.0, self.image.size.height())
        } else {
            (0.0, self.image.size.width())
        };

        let sub = |x| base.0 + (base.1 - base.0) * x;

        let space = length - base.1;

        for s in stretches.iter() {
            if s.0 > 0.0 {
                callback((sub(cursor) + grow, sub(s.0) + grow), (cursor, s.0));
            }

            let stretch = (s.1 - s.0) / total * space;

            callback((sub(s.0) + grow, sub(s.1) + grow + stretch), (s.0, s.1));
            cursor = s.1;
            grow += stretch;
        }
        if cursor < 1.0 {
            callback((sub(cursor) + grow, sub(1.0) + grow), (cursor, 1.0));
        }
    }
}

impl Background {
    pub fn content_rect(&self, span: Rectangle) -> Rectangle {
        match self {
            &Background::Patch(ref patch, _) => patch.content_rect(span),
            &_ => span,
        }
    }

    pub fn padding(&self) -> Rectangle {
        match self {
            &Background::Patch(ref patch, _) => patch.margin(),
            &_ => Rectangle::zero(),
        }
    }

    pub fn is_solid(&self) -> bool {
        match self {
            &Background::None => false,
            &_ => true,
        }
    }

    pub fn render(&self, rectangle: Rectangle) -> Option<Primitive<'static>> {
        match self {
            &Background::Color(color) => Some(Primitive::DrawRect(rectangle, color)),
            &Background::Image(ref image, color) => Some(Primitive::DrawImage(
                image.clone(),
                rectangle,
                color,
            )),
            &Background::Patch(ref patch, color) => Some(Primitive::Draw9(
                patch.clone(),
                rectangle,
                color,
            )),
            &Background::None => None,
        }
    }
}

impl Command {
    /// Append another `Command` to this `Command`. If the `Command`s can be chained together
    /// the `Command` is extended and `None` is returned, but if the `Command`s can not be chained
    /// `self` is replaced by the new `Command` and the old `Command` is returned.
    pub fn append(&mut self, command: Command) -> Option<Command> {
        match *self {
            Command::Nop => {
                *self = command;
                None
            }

            Command::Clip { scissor } => match command {
                Command::Nop => None,
                Command::Clip { scissor: new_scissor } => {
                    *self = Command::Clip { scissor: new_scissor };
                    None
                }
                Command::Colored { offset, count } => {
                    *self = Command::Colored { offset, count };
                    Some(Command::Clip { scissor })
                }
                Command::Textured { texture, offset, count } => {
                    *self = Command::Textured { texture, offset, count };
                    Some(Command::Clip { scissor })
                }
            },

            Command::Colored { offset, count } => match command {
                Command::Nop => {
                    *self = Command::Nop;
                    Some(Command::Colored { offset, count })
                }
                Command::Clip { scissor } => {
                    *self = Command::Clip { scissor: scissor };
                    Some(Command::Colored { offset, count })
                }
                Command::Colored {
                    offset: new_offset,
                    count: new_count,
                } => {
                    if new_offset == offset + count {
                        *self = Command::Colored {
                            offset: offset,
                            count: count + new_count,
                        };
                        None
                    } else {
                        *self = Command::Colored {
                            offset: new_offset,
                            count: new_count,
                        };
                        Some(Command::Colored { offset, count })
                    }
                }
                Command::Textured {
                    texture,
                    offset: new_offset,
                    count: new_count,
                } => {
                    if new_offset == offset + count {
                        *self = Command::Textured {
                            texture: texture,
                            offset: offset,
                            count: count + new_count,
                        };
                        None
                    } else {
                        *self = Command::Textured {
                            texture: texture,
                            offset: new_offset,
                            count: new_count,
                        };
                        Some(Command::Colored { offset, count })
                    }
                }
            },

            Command::Textured { texture, offset, count } => match command {
                Command::Nop => {
                    *self = Command::Nop;
                    Some(Command::Textured { texture, offset, count })
                }
                Command::Clip { scissor } => {
                    *self = Command::Clip { scissor: scissor };
                    Some(Command::Textured { texture, offset, count })
                }
                Command::Colored {
                    offset: new_offset,
                    count: new_count,
                } => {
                    if new_offset == offset + count {
                        *self = Command::Textured {
                            texture: texture,
                            offset: offset,
                            count: count + new_count,
                        };
                        None
                    } else {
                        *self = Command::Colored {
                            offset: new_offset,
                            count: new_count,
                        };
                        Some(Command::Textured { texture, offset, count })
                    }
                }
                Command::Textured {
                    texture: new_texture,
                    offset: new_offset,
                    count: new_count,
                } => {
                    if texture == new_texture && new_offset == offset + count {
                        *self = Command::Textured {
                            texture: texture,
                            offset: offset,
                            count: count + new_count,
                        };
                        None
                    } else {
                        *self = Command::Textured {
                            texture: new_texture,
                            offset: new_offset,
                            count: new_count,
                        };
                        Some(Command::Textured { texture, offset, count })
                    }
                }
            },
        }
    }

    /// Return any draw command that is still being built by the `Command`.
    /// This function is the same as `append(Command::Nop)`.
    pub fn flush(&mut self) -> Option<Command> {
        self.append(Command::Nop)
    }
}
