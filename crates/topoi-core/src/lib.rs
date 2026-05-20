//! Topoi — Pure-Rust computational geometry engine.
//!
//! Boolean operations, buffering, Voronoi diagrams, Delaunay triangulation,
//! and topological predicates (DE-9IM) for 2D geometries.

mod algorithms;
mod buffer;
mod clipping;
mod delaunay;
mod envelope;
mod error;
pub mod geojson;
mod geometry;
mod predicates;
mod rtree;

pub use algorithms::{convex_hull, segment_intersection, simplify};
pub use buffer::buffer_polygon;
pub use clipping::{
    clip_polygon, clip_polygon_rect, intersection_area, polygon_intersection, union_area,
};
pub use delaunay::{Triangle, Triangulation, delaunay};
pub use envelope::Envelope;
pub use error::Error;
pub use geometry::{Coord, LineString, MultiPolygon, Point, Polygon, Ring};
pub use predicates::{contains, intersects};
pub use rtree::RTree;
