//! Geometric functions, algorithms and data structures.

pub mod angle;
pub mod bbox;
pub mod convex_hull;
pub mod delaunay;
pub mod kdtree;
pub mod kmeans;
pub mod line;
pub mod point;
pub mod polygon;
pub mod triangle;
pub mod utils;

pub use self::angle::{angle_orientation, polar_angle, AngleOrientation};
pub use self::bbox::BoundingBox;
pub use self::line::LineEquation;
pub use self::point::{Point, PointF64, PointI32, PointU32};
pub use self::polygon::Polygon;
pub use self::triangle::Triangle;
