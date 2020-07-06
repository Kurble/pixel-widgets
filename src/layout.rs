#[derive(Clone, Copy)]
pub enum Size {
    Shrink,
    Exact(u32),
    Fill(u32),
}

#[derive(Debug, Clone, Copy)]
pub struct Rectangle {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Size {
    pub fn resolve(self, available_space: u32, available_parts: u32) -> u32 {
        match self {
            Size::Shrink => 0,
            Size::Exact(wanted) => available_space.min(wanted),
            Size::Fill(parts) => (available_space * parts) / available_parts
        }
    }

    pub fn parts(&self) -> u32 {
        match self {
            Size::Fill(parts) => *parts,
            _ => 0,
        }
    }
}

impl Rectangle {
    pub fn to_device_coordinates(self, viewport: Rectangle) -> Rectangle {
        let center = (
            (viewport.left + viewport.right) * 0.5,
            (viewport.top + viewport.bottom) * 0.5,
        );
        let size = (
            (viewport.right - viewport.left) * 0.5,
            (viewport.top - viewport.bottom) * -0.5,
        );
        Rectangle {
            left: (self.left - center.0) / size.0,
            top: (self.top - center.1) / size.1,
            right: (self.right - center.0) / size.0,
            bottom: (self.bottom - center.1) / size.1,
        }
    }

    pub fn zero() -> Rectangle {
        Rectangle {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }

    pub fn from_wh(w: f32, h: f32) -> Rectangle {
        Rectangle {
            left: 0.0,
            right: w,
            top: 0.0,
            bottom: h,
        }
    }

    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
        Rectangle {
            left: x,
            right: x + w,
            top: y,
            bottom: y + h,
        }
    }

    pub fn intersect(&self, other: &Rectangle) -> Option<Rectangle> {
        let result = Rectangle {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };
        if result.left < result.right && result.top < result.bottom {
            Some(result)
        } else {
            None
        }
    }

    pub fn pt(&self, x: f32, y: f32) -> [f32; 2] {
        [
            self.left + (self.right - self.left) * x,
            self.top + (self.bottom - self.top) * y,
        ]
    }

    pub fn round(self) -> Rectangle {
        Rectangle {
            left: self.left.round(),
            top: self.top.round(),
            right: self.right.round(),
            bottom: self.bottom.round(),
        }
    }

    pub fn sub(&self, lerps: Rectangle) -> Rectangle {
        Rectangle {
            left: self.left + (self.right - self.left) * lerps.left,
            right: self.left + (self.right - self.left) * lerps.right,
            top: self.top + (self.bottom - self.top) * lerps.top,
            bottom: self.top + (self.bottom - self.top) * lerps.bottom,
        }
    }

    pub fn translate(&self, x: f32, y: f32) -> Rectangle {
        Rectangle {
            left: self.left + x,
            top: self.top + y,
            right: self.right + x,
            bottom: self.bottom + y,
        }
    }

    pub fn grow(&self, w: f32, h: f32) -> Rectangle {
        Rectangle {
            left: self.left,
            top: self.top,
            right: self.right + w,
            bottom: self.bottom + h,
        }
    }

    pub fn inset(&self, x: f32, y: f32) -> Option<Rectangle> {
        if self.width() > y * 2.0 && self.height() > x * 2.0 {
            Some(Rectangle {
                left: self.left + x,
                top: self.top + y,
                right: self.right - x,
                bottom: self.bottom - y,
            })
        } else {
            None
        }
    }

    pub fn outset(&self, x: f32, y: f32) -> Rectangle {
        Rectangle {
            left: self.left - x,
            top: self.top - y,
            right: self.right + x,
            bottom: self.bottom + y,
        }
    }

    pub fn size(&self) -> Rectangle {
        Rectangle {
            left: 0.0,
            top: 0.0,
            right: self.width(),
            bottom: self.height(),
        }
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub fn after_margin(self, margin: Rectangle) -> Rectangle {
        Rectangle {
            left: self.left - margin.left,
            top: self.top - margin.top,
            right: self.right + margin.right,
            bottom: self.bottom + margin.bottom,
        }
    }

    pub fn after_padding(self, padding: Rectangle) -> Rectangle {
        Rectangle {
            left: self.left + padding.left,
            top: self.top + padding.top,
            right: self.right - padding.right,
            bottom: self.bottom - padding.bottom,
        }
    }

    pub fn union(self, other: Rectangle) -> Rectangle {
        Rectangle {
            left: self.left.min(other.left),
            right: self.right.max(other.right),
            top: self.top.min(other.top),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

impl Into<Rectangle> for [f32; 4] {
    fn into(self) -> Rectangle {
        Rectangle {
            left: self[0],
            top: self[1],
            right: self[2],
            bottom: self[3],
        }
    }
}
