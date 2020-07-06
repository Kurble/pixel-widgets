use crate::draw::Color;
use crate::layout::Rectangle;

#[derive(Clone, Copy, Debug)]
pub enum TextWrap {
    NoWrap,
    Wrap,
    WordWrap,
}

#[derive(Clone)]
pub struct Font {
    pub inner: super::cache::Font,
    pub id: super::cache::FontId,
    pub tex_slot: usize,
}

#[derive(Clone)]
pub struct Text {
    pub text: String,
    pub font: Font,
    pub size: f32,
    pub wrap: TextWrap,
    pub color: Color,
    pub border: Option<Color>,
    pub padding: Rectangle,
}

pub struct CharPositionIter<'a, 'b: 'a> {
    font: &'b rusttype::Font<'static>,
    scale: rusttype::Scale,
    last: Option<rusttype::GlyphId>,
    x: f32,
    base: std::str::Chars<'a>,
}

impl<'a, 'b> Iterator for CharPositionIter<'a, 'b> {
    type Item = (char, rusttype::ScaledGlyph<'static>, f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
        self.base.next().and_then(|c| {
            let g = self.font.glyph(c);
            let g = g.scaled(self.scale);
            let w = g.h_metrics().advance_width
                + self
                    .last
                    .map(|last| self.font.pair_kerning(self.scale, last, g.id()))
                    .unwrap_or(0.0);

            self.last = Some(g.id());

            let elem = (c, g, self.x, self.x + w);
            self.x += w;
            Some(elem)
        })
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
    f: &'a mut dyn FnMut(rusttype::ScaledGlyph<'static>, f32, f32, f32),
}

impl<'a, 'b: 'a> WordWrapper<'a, 'b> {
    fn layout_word(&mut self, glyph: rusttype::ScaledGlyph<'static>, a: f32, b: f32, c: f32, mut word: bool) {
        if word {
            self.x = self.final_x;
            self.y = self.final_y;

            if let Some((ch, glyph, b, c)) = self.iter.next() {
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

            while let Some((ch, glyph, b, c)) = self.iter.next() {
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

impl Text {
    pub fn char_positions<'a, 'b>(&'b self) -> CharPositionIter<'a, 'b> {
        let scale = rusttype::Scale {
            x: self.size,
            y: self.size,
        };
        CharPositionIter {
            font: &self.font.inner,
            scale: scale,
            last: None,
            x: 0.0,
            base: self.text.chars(),
        }
    }

    pub fn layout<F: FnMut(rusttype::ScaledGlyph<'static>, f32, f32, f32)>(&self, rect: Rectangle, mut f: F) {
        let line = self.font.inner.v_metrics(rusttype::Scale {
            x: self.size,
            y: self.size,
        });

        let width = rect.width();
        let height = -line.descent + line.line_gap + line.ascent;

        match self.wrap {
            TextWrap::NoWrap => {
                for (_, g, a, b) in self.char_positions() {
                    f(g, a, b, line.ascent);
                }
            }

            TextWrap::Wrap => {
                let mut x = 0.0;
                let mut y = line.ascent;

                for (_, g, a, b) in self.char_positions() {
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
                    y: line.ascent,
                    final_x: 0.0,
                    final_y: line.ascent,
                    width: width,
                    height: height,
                    iter: self.char_positions(),
                    f: &mut f,
                };

                while let Some((ch, glyph, a, b)) = wrapper.iter.next() {
                    wrapper.layout_word(glyph, a, a, b, ch.is_alphanumeric());
                }
            }
        }
    }

    pub fn measure(&self, rect: Option<Rectangle>) -> Rectangle {
        let line = self.font.inner.v_metrics(rusttype::Scale {
            x: self.size,
            y: self.size,
        });

        rect.map_or_else(
            || {
                let mut w = 0.0;
                self.layout(Rectangle::from_wh(f32::INFINITY, 0.0), |_, _, new_w, _| w = new_w);

                Rectangle::from_wh(w.ceil(), (line.ascent - line.descent).ceil())
            },
            |r| {
                let mut w = 0.0;
                let mut h = line.ascent;
                match self.wrap {
                    TextWrap::NoWrap => self.layout(r, |_, _, new_w, _| w = new_w),
                    TextWrap::Wrap | TextWrap::WordWrap => {
                        w = rect.map_or(0.0, |r| r.width());
                        self.layout(r, |_, _, _, new_h| h = new_h);
                    }
                }

                Rectangle::from_xywh(r.left, r.top, w.ceil(), (h - line.descent).ceil())
            },
        )
    }

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
}
