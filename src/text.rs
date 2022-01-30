use crate::draw::Color;
use crate::layout::Rectangle;
use crate::widget::image::ImageData;
use anyhow::*;
use serde::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::iter::Peekable;
use std::ops::Deref;
use std::sync::Arc;

/// How to wrap text
#[derive(Clone, Copy, Debug)]
pub enum TextWrap {
    /// Don't wrap text at all (fastest)
    NoWrap,
    /// Wrap text per character
    Wrap,
    /// Try to keep words on the same line (slowest)
    WordWrap,
}

/// A multi + true signed distance field font.
#[derive(Clone, Debug)]
pub struct Font {
    atlas: ImageData,
    data: Arc<FontData>,
}

#[allow(missing_docs)]
#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct FontDataSerialized {
    atlas: AtlasProperties,
    metrics: VerticalMetrics,
    glyphs: Vec<Glyph>,
    kerning: Vec<KerningPair>,
}

#[allow(missing_docs)]
#[derive(Deserialize, Debug)]
#[serde(from = "FontDataSerialized")]
pub struct FontData {
    pub atlas: AtlasProperties,
    pub metrics: VerticalMetrics,
    pub glyphs: HashMap<u32, Glyph>,
    pub kerning: HashMap<(u32, u32), f32>,
    pub default_glyph: Glyph,
}

/// MSDF font atlas descriptor
#[allow(missing_docs)]
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AtlasProperties {
    pub distance_range: f32,
    pub size: f32,
    pub width: u32,
    pub height: u32,
    pub y_origin: String,
}

/// Vertical metrics for an MSDF font.
#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct VerticalMetrics {
    /// The size of the em square in pixels
    pub em_size: f32,
    /// The height in em units of a single line
    pub line_height: f32,
    /// The amount of ascent
    pub ascender: f32,
    /// The amount of descent
    pub descender: f32,
    /// Y coordinate in em units of underline effects
    pub underline_y: f32,
    /// Thickness in em units of underline effects
    pub underline_thickness: f32,
}

/// A single glyph in an MSDF font.
#[derive(Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct Glyph {
    /// The unicode character for this glyph
    pub unicode: u32,
    /// The amount of space to advance to the right after this glyph.
    pub advance: f32,
    /// Plane bounds
    pub plane_bounds: Option<Rectangle>,
    /// Atlas bounds
    pub atlas_bounds: Option<Rectangle>,
}

/// A kerning pair in an MSDF font.
#[derive(Deserialize, Debug)]
struct KerningPair {
    unicode1: u32,
    unicode2: u32,
    advance: f32,
}

/// A styled paragraph of text
#[derive(Clone)]
pub struct Text<'a> {
    /// The text
    pub text: Cow<'a, str>,
    /// Font to render the text with
    pub font: Font,
    /// Font size to render the text with
    pub size: f32,
    /// Wrapping style to use
    pub wrap: TextWrap,
    /// Color to render the text with
    pub color: Color,
}

/// Iterator over characters that have been layout by the rusttype engine.
pub struct CharPositionIter<'a, 'b: 'a> {
    font: &'b FontData,
    scale_x: f32,
    scale_y: f32,
    x: f32,
    base: Peekable<std::str::Chars<'a>>,
}

impl<'a, 'b> Iterator for CharPositionIter<'a, 'b> {
    type Item = (Glyph, f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
        let c = self.base.next()? as u32;
        let n = self.base.peek().map(|&c| c as u32);
        let g = self.font.glyphs.get(&c).unwrap_or(&self.font.default_glyph);
        let w = (g.advance + n.and_then(|n| self.font.kerning.get(&(c, n)).copied()).unwrap_or(0.0)) * self.scale_x;
        let elem = (g.scale(self.scale_x, self.scale_y), self.x, self.x + w);
        self.x += w;
        Some(elem)
    }
}

struct WordWrapper<'a, 'b: 'a> {
    x: f32,
    y: f32,
    final_x: f32,
    final_y: f32,
    width: f32,
    height: f32,
    iter: CharPositionIter<'a, 'b>,
    f: &'a mut dyn FnMut(Glyph, f32, f32, f32),
}

impl Font {
    /// Load mtsdf font from a json file and an atlas texture
    pub fn from_data(data: impl AsRef<[u8]>, atlas: ImageData) -> Result<Self> {
        let mut data: FontData = serde_json::from_slice(data.as_ref())?;
        for (_, g) in data.glyphs.iter_mut() {
            g.atlas_bounds = g.atlas_bounds.map(|b| match data.atlas.y_origin.as_str() {
                "bottom" => atlas.texcoords.sub(Rectangle {
                    left: b.left / data.atlas.width as f32,
                    top: 1.0 - b.top / data.atlas.height as f32,
                    right: b.right / data.atlas.width as f32,
                    bottom: 1.0 - b.bottom / data.atlas.height as f32,
                }),
                _ => atlas.texcoords.sub(Rectangle {
                    left: b.left / data.atlas.width as f32,
                    top: b.top / data.atlas.height as f32,
                    right: b.right / data.atlas.width as f32,
                    bottom: b.bottom / data.atlas.height as f32,
                }),
            });
            g.plane_bounds = g.plane_bounds.map(|b| Rectangle {
                top: -b.top,
                bottom: -b.bottom,
                ..b
            });
        }
        Ok(Self {
            atlas,
            data: Arc::new(data),
        })
    }

    pub(crate) fn texture(&self) -> usize {
        self.atlas.texture
    }
}

impl Deref for Font {
    type Target = FontData;

    fn deref(&self) -> &FontData {
        &*self.data
    }
}

impl From<FontDataSerialized> for FontData {
    fn from(val: FontDataSerialized) -> Self {
        let default_glyph = val.glyphs[0].clone();
        Self {
            atlas: val.atlas,
            metrics: val.metrics,
            glyphs: val.glyphs.into_iter().map(|g| (g.unicode, g)).collect(),
            kerning: val
                .kerning
                .into_iter()
                .map(|k| ((k.unicode1, k.unicode2), k.advance))
                .collect(),
            default_glyph,
        }
    }
}

impl VerticalMetrics {
    /// Scale the font metrics to the desired font size
    pub fn scale(&self, y: f32) -> Self {
        Self {
            em_size: self.em_size * y,
            line_height: self.line_height * y,
            ascender: self.ascender * y,
            descender: self.descender * y,
            underline_y: self.underline_y * y,
            underline_thickness: self.underline_thickness * y,
        }
    }
}

impl Glyph {
    /// Scale the glyph to the desired font size
    pub fn scale(&self, x: f32, y: f32) -> Self {
        Self {
            unicode: self.unicode,
            advance: self.advance * x,
            atlas_bounds: self.atlas_bounds.clone(),
            plane_bounds: self.plane_bounds.clone().map(|b| Rectangle {
                left: b.left * x,
                top: b.top * y,
                right: b.right * x,
                bottom: b.bottom * y,
            }),
        }
    }
}

impl<'a, 'b: 'a> WordWrapper<'a, 'b> {
    fn layout_word(&mut self, glyph: Glyph, a: f32, b: f32, c: f32, mut word: bool) {
        if word {
            self.x = self.final_x;
            self.y = self.final_y;

            if let Some((glyph, b, c)) = self.iter.next() {
                let ch = unsafe { char::from_u32_unchecked(glyph.unicode) };
                if ch.is_alphanumeric() {
                    if c - self.x > self.width {
                        self.x = a;
                        self.y += self.height;
                        word = false;
                    }
                    self.layout_word(glyph, a, b, c, word);
                }
            }

            (self.f)(glyph, b - self.x, c - self.x, self.y);
        } else {
            self.final_x = self.x;
            self.final_y = self.y;

            if c - self.final_x > self.width {
                self.final_x = b;
                self.final_y += self.height;
            }
            (self.f)(glyph, b - self.final_x, c - self.final_x, self.final_y);

            for (glyph, b, c) in &mut self.iter {
                let ch = unsafe { char::from_u32_unchecked(glyph.unicode) };

                if c - self.final_x > self.width {
                    self.final_x = b;
                    self.final_y += self.height;
                }

                (self.f)(glyph, b - self.final_x, c - self.final_x, self.final_y);

                if !ch.is_alphanumeric() {
                    break;
                }
            }
        }
    }
}

impl<'t> Text<'t> {
    pub(crate) fn char_positions<'a, 'b>(&'b self) -> CharPositionIter<'a, 'b> {
        CharPositionIter {
            font: &*self.font.data,
            scale_x: self.size,
            scale_y: self.size,
            x: 0.0,
            base: self.text.chars().peekable(),
        }
    }

    pub(crate) fn layout<F: FnMut(Glyph, f32, f32, f32)>(&self, rect: Rectangle, mut f: F) {
        let line = self.font.data.metrics.scale(self.size);

        let width = rect.width();
        let height = -line.descender + line.line_height + line.ascender;

        match self.wrap {
            TextWrap::NoWrap => {
                for (g, a, b) in self.char_positions() {
                    f(g, a, b, line.ascender);
                }
            }

            TextWrap::Wrap => {
                let mut x = 0.0;
                let mut y = line.ascender;

                for (g, a, b) in self.char_positions() {
                    if b - x > width {
                        x = a;
                        y += height;
                    }

                    f(g, a - x, b - x, y);
                }
            }

            TextWrap::WordWrap => {
                let mut wrapper = WordWrapper {
                    x: 0.0,
                    y: line.ascender,
                    final_x: 0.0,
                    final_y: line.ascender,
                    width,
                    height,
                    iter: self.char_positions(),
                    f: &mut f,
                };

                while let Some((glyph, a, b)) = wrapper.iter.next() {
                    let ch = unsafe { char::from_u32_unchecked(glyph.unicode) };
                    wrapper.layout_word(glyph, a, a, b, ch.is_alphanumeric());
                }
            }
        }
    }

    /// Measure the size of the text. If a rectangle is supplied and the text wraps,
    /// the layout will stay within the width of the given rectangle.
    pub fn measure(&self, rect: Option<Rectangle>) -> Rectangle {
        let line = self.font.data.metrics.scale(self.size);

        match rect {
            None => {
                let mut w = 0.0;
                self.layout(Rectangle::from_wh(f32::INFINITY, 0.0), |_, _, new_w, _| w = new_w);

                Rectangle::from_wh(w.ceil(), (line.ascender - line.descender).ceil())
            }
            Some(r) => {
                let mut w = 0.0;
                let mut h = line.ascender;
                match self.wrap {
                    TextWrap::NoWrap => self.layout(r, |_, _, new_w, _| w = new_w),
                    TextWrap::Wrap | TextWrap::WordWrap => {
                        w = rect.map_or(0.0, |r| r.width());
                        self.layout(r, |_, _, _, new_h| h = new_h);
                    }
                }

                Rectangle::from_xywh(r.left, r.top, w.ceil(), (h - line.descender).ceil())
            }
        }
    }

    /// Measure the start and end coordinates of some selected glyphs
    pub fn measure_range(&self, from: usize, to: usize, rect: Rectangle) -> ((f32, f32), (f32, f32)) {
        let mut from_result = (0.0, 0.0);
        let mut to_result = (0.0, 0.0);

        let mut index = 0;
        self.layout(rect, |_, begin, end, y| {
            if index == from {
                from_result = (begin, y)
            }
            if index == to {
                to_result = (begin, y)
            }
            if index + 1 == from {
                from_result = (end, y)
            }
            if index + 1 == to {
                to_result = (end, y)
            }
            index += 1;
        });

        (from_result, to_result)
    }

    /// Find out the index of a character where the mouse is.
    pub fn hitdetect(&self, cursor: (f32, f32), rect: Rectangle) -> usize {
        let dist = |(x, y)| x * x + y * y;

        let mut nearest = (dist(cursor), 0);
        let mut index = 0;

        self.layout(rect, |_, begin, end, y| {
            if dist((begin - cursor.0, y - cursor.1)) < nearest.0 {
                nearest.0 = dist((begin - cursor.0, y - cursor.1));
                nearest.1 = index;
            }
            if dist((end - cursor.0, y - cursor.1)) < nearest.0 {
                nearest.0 = dist((end - cursor.0, y - cursor.1));
                nearest.1 = index + 1;
            }

            index += 1;
        });

        nearest.1
    }

    pub(crate) fn draw<F: FnMut(Rectangle, Rectangle)>(&self, rect: Rectangle, mut place_glyph: F) {
        self.layout(rect, |g, x, _, y| {
            if let (Some(atlas), Some(plane)) = (g.atlas_bounds, g.plane_bounds) {
                place_glyph(atlas, plane.translate(rect.left + x, rect.top + y));
            }
        });
    }
}

impl<'a> Text<'a> {
    /// Convert a borrowed to text to owned text.
    pub fn to_owned(&self) -> Text<'static> {
        Text {
            text: Cow::Owned(self.text.clone().into_owned()),
            font: self.font.clone(),
            size: self.size,
            wrap: self.wrap,
            color: self.color,
        }
    }
}
