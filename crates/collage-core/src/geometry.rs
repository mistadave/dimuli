//! Basic geometry types (`f32`, collage units unless stated otherwise).

/// A point or a vector.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    /// Horizontal component.
    pub x: f32,
    /// Vertical component.
    pub y: f32,
}

impl Vec2 {
    /// Creates a vector.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A width/height pair.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    /// Horizontal extent.
    pub width: f32,
    /// Vertical extent.
    pub height: f32,
}

impl Size {
    /// Creates a size.
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// `true` if either side is zero, negative, NaN or infinite.
    pub fn is_empty(&self) -> bool {
        !(self.width.is_finite()
            && self.width > 0.0
            && self.height.is_finite()
            && self.height > 0.0)
    }
}

/// An axis-aligned rectangle with its origin at the top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Horizontal extent.
    pub width: f32,
    /// Vertical extent.
    pub height: f32,
}

impl Rect {
    /// Creates a rectangle.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Rectangle at the origin with the given size.
    pub const fn from_size(size: Size) -> Self {
        Self::new(0.0, 0.0, size.width, size.height)
    }

    /// Width and height.
    pub const fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Right edge (`x + width`).
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge (`y + height`).
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Center point.
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Half-open containment test: the left/top edges belong to the rectangle, the right/bottom edges don't,
    /// so adjacent rectangles never both contain a point.
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x && point.x < self.right() && point.y >= self.y && point.y < self.bottom()
    }

    /// Shrinks the rectangle by `amount` on every side. Never produces a negative size;
    /// negative amounts are treated as zero.
    pub fn inset(&self, amount: f32) -> Rect {
        let dx = amount.max(0.0).min(self.width / 2.0);
        let dy = amount.max(0.0).min(self.height / 2.0);
        Rect::new(
            self.x + dx,
            self.y + dy,
            self.width - 2.0 * dx,
            self.height - 2.0 * dy,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_is_half_open() {
        let r = Rect::new(0.0, 0.0, 1.0, 1.0);
        assert!(r.contains(Vec2::new(0.0, 0.0)));
        assert!(r.contains(Vec2::new(0.5, 0.999)));
        assert!(!r.contains(Vec2::new(1.0, 0.5)));
        assert!(!r.contains(Vec2::new(0.5, 1.0)));
        assert!(!r.contains(Vec2::new(-0.1, 0.5)));
    }

    #[test]
    fn inset_shrinks_on_all_sides() {
        let r = Rect::new(1.0, 2.0, 4.0, 6.0).inset(0.5);
        assert_eq!(r, Rect::new(1.5, 2.5, 3.0, 5.0));
    }

    #[test]
    fn inset_never_produces_negative_size() {
        let r = Rect::new(0.0, 0.0, 1.0, 2.0).inset(5.0);
        assert_eq!(r.width, 0.0);
        assert_eq!(r.height, 0.0);
        assert_eq!(r.center(), Vec2::new(0.5, 1.0));
    }

    #[test]
    fn negative_inset_is_ignored() {
        let r = Rect::new(0.0, 0.0, 1.0, 1.0);
        assert_eq!(r.inset(-1.0), r);
    }

    #[test]
    fn empty_size_detection() {
        assert!(Size::new(0.0, 1.0).is_empty());
        assert!(Size::new(1.0, f32::NAN).is_empty());
        assert!(Size::new(f32::INFINITY, 1.0).is_empty());
        assert!(!Size::new(0.1, 0.1).is_empty());
    }
}
