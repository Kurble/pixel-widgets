use std::mem;
use std::sync::{Arc, Weak};

use anyhow::*;
use image::{Pixel, RgbaImage};
use smallvec::SmallVec;

use crate::atlas::*;
use crate::draw::*;
use crate::layout::Rectangle;
use crate::text::Font;

/// A cache for textures and text
pub struct Cache {
    #[allow(unused)]
    size: usize,
    textures: Vec<TextureSlot>,
    updates: Vec<Update>,
    image_id_counter: usize,
}

enum TextureSlot {
    Atlas(Atlas<Weak<usize>>),
    Big,
}

impl Cache {
    /// Create a new cache. Size is the width and height of textures in pixels.
    /// Offset is the offset to apply to texture ids
    pub fn new(size: usize) -> Cache {
        let atlas = Atlas::new(size);

        Cache {
            size,
            textures: vec![
                // glyph cache
                TextureSlot::Big,
                // atlas for textures
                TextureSlot::Atlas(atlas),
            ],
            updates: vec![
                // glyph cache
                Update::Texture {
                    id: 0,
                    size: [size as u32, size as u32],
                    data: Vec::new(),
                    atlas: true,
                },
                // atlas for textures
                Update::Texture {
                    id: 1,
                    size: [size as u32, size as u32],
                    data: Vec::new(),
                    atlas: true,
                },
            ],
            image_id_counter: 1,
        }
    }

    /// Take updates for the texture system from the cache
    pub fn take_updates(&mut self) -> Vec<Update> {
        mem::take(&mut self.updates)
    }

    pub(crate) fn load_image(&mut self, image: RgbaImage) -> ImageData {
        let size = Rectangle {
            left: 0.0,
            top: 0.0,
            right: image.width() as f32,
            bottom: image.height() as f32,
        };
        let (texture, cache_id, texcoords) = self.insert_image(image);
        ImageData {
            texture,
            _cache_id: cache_id,
            texcoords,
            size,
        }
    }

    pub(crate) fn load_patch(&mut self, mut image: RgbaImage) -> Patch {
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
            if image[(x, 0)].channels()[3] > 128 {
                h_current_stretch = Some(h_current_stretch.map_or_else(|| (h_begin, h_end), |(s, _)| (s, h_end)));
            } else if let Some(s) = h_current_stretch.take() {
                h_stretch.push(s);
            }

            // check content pixel
            if image[(x, image.height() - 1)].channels()[3] > 128 {
                h_content.0 = h_begin.min(h_content.0);
                h_content.1 = h_end.max(h_content.1);
            }
        }

        // scan vertical stretch and content bars
        for y in 1..image.height() - 1 {
            let v_begin = (y - 1) as f32 / (image.height() - 2) as f32;
            let v_end = (y) as f32 / (image.height() - 2) as f32;

            // check stretch pixel
            if image[(0, y)].channels()[3] > 128 {
                v_current_stretch = Some(v_current_stretch.map_or_else(|| (v_begin, v_end), |(s, _)| (s, v_end)));
            } else if let Some(s) = v_current_stretch.take() {
                v_stretch.push(s);
            }

            // check content pixel
            if image[(image.width() - 1, y)].channels()[3] > 128 {
                v_content.0 = v_begin.min(v_content.0);
                v_content.1 = v_end.max(v_content.1);
            }
        }

        if let Some(s) = h_current_stretch.take() {
            h_stretch.push(s);
        }
        if let Some(s) = v_current_stretch.take() {
            v_stretch.push(s);
        }

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
            image: ImageData {
                texture,
                _cache_id: cache_id,
                texcoords,
                size,
            },
            h_stretch,
            v_stretch,
            h_content,
            v_content,
        }
    }

    pub(crate) fn load_font<D: AsRef<[u8]>>(&mut self, data: D, image: RgbaImage) -> Result<crate::text::Font> {
        let atlas = self.load_image(image);
        Font::from_data(data, atlas)
    }

    fn insert_image(&mut self, image: image::RgbaImage) -> (usize, Arc<usize>, Rectangle) {
        for slot in self.textures.iter_mut() {
            if let TextureSlot::Atlas(atlas) = slot {
                atlas.remove_expired();
            }
        }

        let image_id = Arc::new(self.image_id_counter);
        self.image_id_counter += 1;

        let slot = self
            .textures
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| match slot {
                TextureSlot::Atlas(atlas) => {
                    let image_size = image.width().max(image.height()) as usize;
                    atlas
                        .insert(Arc::downgrade(&image_id), image_size)
                        .ok()
                        .map(|area| (area, atlas.size() as f32, index))
                }
                TextureSlot::Big => None,
            })
            .next();

        if let Some((mut area, atlas_size, tex_id)) = slot {
            area.right = area.left + image.width() as usize;
            area.bottom = area.top + image.height() as usize;

            let update = Update::TextureSubresource {
                id: tex_id,
                offset: [area.left as u32, area.top as u32],
                size: [image.width(), image.height()],
                data: image.to_vec(),
            };
            self.updates.push(update);

            (
                tex_id,
                image_id,
                Rectangle {
                    left: area.left as f32 / atlas_size,
                    top: area.top as f32 / atlas_size,
                    right: area.right as f32 / atlas_size,
                    bottom: area.bottom as f32 / atlas_size,
                },
            )
        } else {
            let tex_id = self.textures.len();

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
}
