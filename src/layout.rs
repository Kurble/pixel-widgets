/// A sizing request
#[derive(Debug, Clone, Copy)]
pub enum Size {
    /// Try to fit all children exactly
    Shrink,
    /// An exact size in units
    Exact(f32),
    /// Fill the available space using a weight in units.
    /// The available space is divided between `Fill` sizes according to their weight.
    Fill(u32),
}

/// Alignment
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum Align {
    Begin,
    Center,
    End,
}

/// Layout direction
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    TopToBottom,
    LeftToRight,
    RightToLeft,
    BottomToTop,
}

/// A rectangle
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Size {
    /// Resolve the `Size` to an actual size
    pub fn resolve(self, available_space: f32, available_parts: u32) -> f32 {
        match self {
            Size::Shrink => 0.0,
            Size::Exact(wanted) => wanted,
            Size::Fill(parts) => (available_space * parts as f32) / available_parts as f32,
        }
    }

    /// Get the weight of this `Size`, which is 0 for non fill sizes.
    pub fn parts(&self) -> u32 {
        match self {
            Size::Fill(parts) => *parts,
            _ => 0,
        }
    }

    /// Get the minimum size of this `Size`, which is 0 for non exact sizes.
    pub fn min_size(&self) -> f32 {
        match self {
            Size::Exact(wanted) => *wanted,
            _ => 0.0,
        }
    }
}

impl Align {
    /// Align `space` units within `available_space`.
    pub fn resolve_start(self, space: f32, available_space: f32) -> f32 {
        match self {
            Align::Begin => 0.0,
            Align::Center => (available_space - space) * 0.5,
            Align::End => available_space - space,
        }
    }
}

impl Rectangle {
    /// Convert a rectangle to device coordinates (`[-1.0, 1.0]`) using a `Viewport`.
    /// (-1, -1) is the top left corner (0, 0), where (1, 1) is the bottom right
    /// corner (viewport.width(), viewport.height()).
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

    /// Return a zero size rectangle
    pub fn zero() -> Rectangle {
        Rectangle {
            left: 0.0,
            right: 0.0,
            top: 0.0,
            bottom: 0.0,
        }
    }

    /// Construct a new rectangle with (0, 0) as (left, top), and w, h as (right, bottom)
    pub fn from_wh(w: f32, h: f32) -> Rectangle {
        Rectangle {
            left: 0.0,
            right: w,
            top: 0.0,
            bottom: h,
        }
    }

    /// Construct a new rectangle from a position and a size
    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
        Rectangle {
            left: x,
            right: x + w,
            top: y,
            bottom: y + h,
        }
    }

    /// Returns `true` when the queried point is inside the rectangle
    pub fn point_inside(&self, x: f32, y: f32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    /// Returns the rectangle that is covered both by `self` and `other`.
    /// `None` is returned if the rectangles do not overlap.
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

    /// Return a point within this rectangle. The point should be in [0, 1] range.
    pub fn pt(&self, x: f32, y: f32) -> [f32; 2] {
        [
            self.left + (self.right - self.left) * x,
            self.top + (self.bottom - self.top) * y,
        ]
    }

    /// Return a rectangle with all fields rounded
    pub fn round(self) -> Rectangle {
        Rectangle {
            left: self.left.round(),
            top: self.top.round(),
            right: self.right.round(),
            bottom: self.bottom.round(),
        }
    }

    pub(crate) fn sub(&self, lerps: Rectangle) -> Rectangle {
        Rectangle {
            left: self.left + (self.right - self.left) * lerps.left,
            right: self.left + (self.right - self.left) * lerps.right,
            top: self.top + (self.bottom - self.top) * lerps.top,
            bottom: self.top + (self.bottom - self.top) * lerps.bottom,
        }
    }

    /// Apply a translation the the rectangle
    pub fn translate(&self, x: f32, y: f32) -> Rectangle {
        Rectangle {
            left: self.left + x,
            top: self.top + y,
            right: self.right + x,
            bottom: self.bottom + y,
        }
    }

    /// Increase the size of the rectangle on the right and bottom side.
    pub fn grow(&self, w: f32, h: f32) -> Rectangle {
        Rectangle {
            left: self.left,
            top: self.top,
            right: self.right + w,
            bottom: self.bottom + h,
        }
    }

    /// Decrease the size of the rectangle on all sides
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

    /// Grow the rectangle on all sides
    pub fn outset(&self, x: f32, y: f32) -> Rectangle {
        Rectangle {
            left: self.left - x,
            top: self.top - y,
            right: self.right + x,
            bottom: self.bottom + y,
        }
    }

    /// Return a rectangle with the same size, but positioned at the origin
    pub fn size(&self) -> Rectangle {
        Rectangle {
            left: 0.0,
            top: 0.0,
            right: self.width(),
            bottom: self.height(),
        }
    }

    /// The width of the rectangle
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    /// The height of the rectangle
    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    /// Apply a margin to the rectangle
    pub fn after_margin(self, margin: Rectangle) -> Rectangle {
        Rectangle {
            left: self.left - margin.left,
            top: self.top - margin.top,
            right: self.right + margin.right,
            bottom: self.bottom + margin.bottom,
        }
    }

    /// Apply padding to the rectangle
    pub fn after_padding(self, padding: Rectangle) -> Rectangle {
        Rectangle {
            left: self.left + padding.left,
            top: self.top + padding.top,
            right: self.right - padding.right,
            bottom: self.bottom - padding.bottom,
        }
    }

    /// Return the smallest rectangle that covers both `self` and `other`
    pub fn union(self, other: Rectangle) -> Rectangle {
        Rectangle {
            left: self.left.min(other.left),
            right: self.right.max(other.right),
            top: self.top.min(other.top),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

impl From<[f32; 4]> for Rectangle {
    fn from(a: [f32; 4]) -> Rectangle {
        Rectangle {
            left: a[0],
            top: a[1],
            right: a[2],
            bottom: a[3],
        }
    }
}

impl From<f32> for Size {
    fn from(value: f32) -> Size {
        Size::Exact(value)
    }
}
