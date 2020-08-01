use std::mem;

use image;
use image::{Rgba, RgbaImage};
use rusttype;
use rusttype::{point, vector};
use smallvec::SmallVec;

use crate::draw::*;
use crate::layout::Rectangle;
use crate::atlas::*;
use crate::text::Text;

type GlyphCache = rusttype::gpu_cache::Cache<'static>;
pub type Font = rusttype::Font<'static>;
pub type FontId = usize;

#[allow(dead_code)]
pub struct Cache {
    size: usize,
    glyphs: GlyphCache,
    textures: Vec<TextureSlot>,
    textures_offset: usize,
    updates: Vec<Update>,
    font_id_counter: usize,
    image_id_counter: usize,
}

enum TextureSlot {
    Atlas(Atlas<usize>),
    Big,
}

impl Cache {
    pub fn new(size: usize, offset: usize) -> Cache {
        let glyphs = GlyphCache::builder().dimensions(size as u32, size as u32).build();

        let mut atlas = Atlas::new(size);
        atlas.insert(0, size / 2).unwrap();

        let atlas_create = Update::Texture {
            id: offset,
            size: [size as u32, size as u32],
            data: Vec::new(),
            atlas: true,
        };

        Cache {
            size: size,
            glyphs: glyphs,
            textures: vec![TextureSlot::Atlas(atlas)],
            textures_offset: offset,
            updates: vec![atlas_create],
            font_id_counter: 0,
            image_id_counter: 1,
        }
    }

    pub fn take_updates(&mut self) -> Vec<Update> {
        mem::replace(&mut self.updates, Vec::new())
    }

    pub fn draw_text<F: FnMut(Rectangle, Rectangle)>(&mut self, text: &Text, rect: Rectangle, mut place_glyph: F) {
        let start = point(rect.left, rect.top);

        let mut placed_glyphs = Vec::with_capacity(text.text.len());
        text.layout(rect, |g, x, _, y| {
            placed_glyphs.push(g.positioned(start + vector(x, y)));
        });

        for g in placed_glyphs.iter() {
            self.glyphs.queue_glyph(text.font.id as usize, g.clone());
        }

        let textures_offset = self.textures_offset;

        let updates = &mut self.updates;
        self.glyphs
            .cache_queued(|rect, data| {
                let mut new_data = Vec::with_capacity(data.len() * 4);
                for x in data {
                    new_data.push(255);
                    new_data.push(255);
                    new_data.push(255);
                    new_data.push(*x);
                }

                let update = Update::TextureSubresource {
                    id: textures_offset,
                    offset: [rect.min.x, rect.min.y],
                    size: [rect.width(), rect.height()],
                    data: new_data,
                };

                updates.push(update);
            })
            .unwrap();

        for g in placed_glyphs.iter() {
            self.glyphs
                .rect_for(text.font.id as usize, g)
                .unwrap()
                .map(|(uv, pos)| {
                    place_glyph(
                        Rectangle {
                            left: uv.min.x,
                            top: uv.min.y,
                            right: uv.max.x,
                            bottom: uv.max.y,
                        },
                        Rectangle {
                            left: pos.min.x as f32,
                            top: pos.min.y as f32,
                            right: pos.max.x as f32,
                            bottom: pos.max.y as f32,
                        },
                    );
                });
        }
    }

    fn insert_image(&mut self, image: image::RgbaImage) -> (usize, usize, Rectangle) {
        let tex_id = 0;
        let image_id = self.image_id_counter;
        self.image_id_counter += 1;
        let (area, atlas_size) = if let TextureSlot::Atlas(ref mut atlas) = self.textures[tex_id] {
            let image_size = image.width().max(image.height()) as usize;
            (atlas.insert(image_id, image_size).ok(), atlas.size() as f32)
        } else {
            (None, 0.0)
        };

        if area.is_some() {
            let mut area = area.unwrap();

            area.right = area.left + image.width() as usize;
            area.bottom = area.top + image.height() as usize;

            let update = Update::TextureSubresource {
                id: tex_id + self.textures_offset,
                offset: [area.left as u32, area.top as u32],
                size: [image.width(), image.height()],
                data: image.to_vec(),
            };
            self.updates.push(update);

            (
                self.textures_offset + tex_id,
                image_id,
                Rectangle {
                    left: area.left as f32 / atlas_size,
                    top: area.top as f32 / atlas_size,
                    right: area.right as f32 / atlas_size,
                    bottom: area.bottom as f32 / atlas_size,
                },
            )
        } else {
            let tex_id = self.textures_offset + self.textures.len();

            let update = Update::Texture {
                id: tex_id,
                size: [image.width(), image.height()],
                data: image.to_vec(),
                atlas: false,
            };

            self.updates.push(update);
            self.textures.push(TextureSlot::Big);

            (tex_id, image_id, Rectangle::from_wh(1.0, 1.0))
        }
    }

    pub fn load_image(&mut self, image: RgbaImage) -> Image {
        let size = Rectangle {
            left: 0.0,
            top: 0.0,
            right: image.width() as f32,
            bottom: image.height() as f32,
        };
        let (texture, cache_id, texcoords) = self.insert_image(image);
        Image {
            texture,
            cache_id,
            texcoords,
            size,
        }
    }

    pub fn load_patch(&mut self, mut image: RgbaImage) -> Patch {
        // find 9 patch borders in image data
        let black = Rgba([0u8, 0u8, 0u8, 255u8]);

        let mut h_stretch = SmallVec::<[(f32, f32); 2]>::new();
        let mut h_content = (1.0, 0.0);
        let mut v_stretch = SmallVec::<[(f32, f32); 2]>::new();
        let mut v_content = (1.0, 0.0);
        let mut h_current_stretch = None;
        let mut v_current_stretch = None;

        // scan horizontal stretch and content bars
        for x in 1..image.width() - 1 {
            let h_begin = (x - 1) as f32 / (image.width() - 2) as f32;
            let h_end = (x) as f32 / (image.width() - 2) as f32;

            // check stretch pixel
            if image[(x, 0)] == black {
                h_current_stretch = Some(h_current_stretch.map_or_else(|| (h_begin, h_end), |(s, _)| (s, h_end)));
            } else {
                h_current_stretch.take().map(|s| h_stretch.push(s));
            }

            // check content pixel
            if image[(x, image.height() - 1)] == black {
                h_content.0 = h_begin.min(h_content.0);
                h_content.1 = h_end.max(h_content.1);
            }
        }

        // scan vertical stretch and content bars
        for y in 1..image.height() - 1 {
            let v_begin = (y - 1) as f32 / (image.height() - 2) as f32;
            let v_end = (y) as f32 / (image.height() - 2) as f32;

            // check stretch pixel
            if image[(0, y)] == black {
                v_current_stretch = Some(v_current_stretch.map_or_else(|| (v_begin, v_end), |(s, _)| (s, v_end)));
            } else {
                v_current_stretch.take().map(|s| v_stretch.push(s));
            }

            // check content pixel
            if image[(image.width() - 1, y)] == black {
                v_content.0 = v_begin.min(v_content.0);
                v_content.1 = v_end.max(v_content.1);
            }
        }

        h_current_stretch.take().map(|s| h_stretch.push(s));
        v_current_stretch.take().map(|s| v_stretch.push(s));

        // strip stretch and content bars from the image
        let patch_width = image.width() - 2;
        let patch_height = image.height() - 2;
        let image = image::imageops::crop(&mut image, 1, 1, patch_width, patch_height).to_image();
        let size = Rectangle {
            left: 0.0,
            top: 0.0,
            right: image.width() as f32,
            bottom: image.height() as f32,
        };
        let (texture, cache_id, texcoords) = self.insert_image(image);

        Patch {
            image: Image {
                texture,
                cache_id,
                texcoords,
                size,
            },
            h_stretch,
            v_stretch,
            h_content,
            v_content,
        }
    }

    pub fn unload_image(&mut self, image: Image) {
        if let Some(tex) = self.textures.get_mut(image.texture) {
            match tex {
                &mut TextureSlot::Atlas(ref mut atlas) => {
                    atlas.remove_by_value(&image.cache_id);
                },
                &mut TextureSlot::Big => (),
            }
        }
    }

    pub fn load_font<D: Into<Vec<u8>>>(&mut self, data: D) -> crate::text::Font {
        let inner = Font::try_from_vec(data.into()).expect("error loading font");

        let id = self.font_id_counter;
        self.font_id_counter += 1;

        crate::text::Font {
            inner,
            id,
            tex_slot: self.textures_offset,
        }
    }
}
