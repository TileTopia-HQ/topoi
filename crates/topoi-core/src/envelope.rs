use crate::Coord;

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Envelope {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Envelope {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn from_coords(coords: &[Coord]) -> Option<Self> {
        if coords.is_empty() {
            return None;
        }
        let mut env = Self {
            min_x: f64::MAX,
            min_y: f64::MAX,
            max_x: f64::MIN,
            max_y: f64::MIN,
        };
        for c in coords {
            env.min_x = env.min_x.min(c.x);
            env.min_y = env.min_y.min(c.y);
            env.max_x = env.max_x.max(c.x);
            env.max_y = env.max_y.max(c.y);
        }
        Some(env)
    }

    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }

    pub fn contains_coord(&self, c: &Coord) -> bool {
        c.x >= self.min_x && c.x <= self.max_x && c.y >= self.min_y && c.y <= self.max_y
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// Alias for `union` used in R-tree building.
    pub fn merge(&self, other: &Self) -> Self {
        self.union(other)
    }

    pub fn center_x(&self) -> f64 {
        (self.min_x + self.max_x) / 2.0
    }

    pub fn center_y(&self) -> f64 {
        (self.min_y + self.max_y) / 2.0
    }

    /// Minimum distance from a point to this envelope (0 if inside).
    pub fn distance_to_point(&self, p: &Coord) -> f64 {
        let dx = if p.x < self.min_x {
            self.min_x - p.x
        } else if p.x > self.max_x {
            p.x - self.max_x
        } else {
            0.0
        };
        let dy = if p.y < self.min_y {
            self.min_y - p.y
        } else if p.y > self.max_y {
            p.y - self.max_y
        } else {
            0.0
        };
        (dx * dx + dy * dy).sqrt()
    }
}
