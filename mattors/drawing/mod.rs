//! Simple module to draw basic shapes on an image. Most of this stuff is
//! already implemented in the imageprocs crate, but the best way to learn is by
//! reimplementing, so...

pub mod line;
pub mod triangle;

use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use image::Pixel;

use geo::polygon::Polygon;
use geo::{BoundingBox, LineEquation, Point, PointU32};

use crate::drawing::line::BresenhamLineIter;
use crate::drawing::triangle::FlatTriangleIter;

/// The `Blender` is the function that decides how to merge two pixels together.
pub trait Blender<P: image::Pixel> {
    /// The first param is the old value of the pixel and it's meant to be modified
    /// with the blended value. The second parameter is the new pixel.
    fn blend(dst: &mut P, src: &P);
}

/// Simple struct to easily write common geometric primitives onto a given image
/// using the given `Blender`.
pub struct Drawer<'a, I: 'a, B>
where
    I: image::GenericImage,
    I::Pixel: Debug,
    B: Blender<I::Pixel>,
{
    img: &'a mut I,
    _blender: PhantomData<B>,
}

impl<'a, I> Drawer<'a, I, NoopBlender>
where
    I: image::GenericImage,
    I::Pixel: Debug,
{
    /// Create a new `Drawer` that does not perform any blending, but just
    /// copies the new pixel.
    pub fn new_with_no_blending(img: &'a mut I) -> Self {
        Drawer::new(img)
    }
}

impl<'a, I> Drawer<'a, I, DefaultBlender>
where
    I: image::GenericImage,
    I::Pixel: Debug,
{
    /// Create a new `Drawer` that performs pixel blending.
    pub fn new_with_default_blending(img: &'a mut I) -> Self {
        Drawer::new(img)
    }
}

impl<'a, P: 'a, C, B> Drawer<'a, image::ImageBuffer<P, C>, B>
where
    P: image::Pixel + Debug + 'static,
    B: Blender<P>,
    C: Deref<Target = [P::Subpixel]> + DerefMut,
{
    /// Fills the image with the given pixel.
    pub fn fill(&mut self, pix: &P) {
        for p in self.img.pixels_mut() {
            *p = *pix;
        }
    }
}

impl<'a, I, B> Drawer<'a, I, B>
where
    I: image::GenericImage,
    I::Pixel: Debug,
    B: Blender<I::Pixel>,
{
    /// Create a new `Drawer` on the given `img` with the given `blender`. The
    /// `blender` is a function that takes the current pixel on the image and
    /// the new one and can change the current pixel. It is meant for pixel
    /// blending.
    pub fn new(img: &'a mut I) -> Self {
        Drawer {
            img,
            _blender: PhantomData,
        }
    }

    /// Returns the inner image dimensions as (width, height).
    pub fn dimensions(&self) -> (u32, u32) {
        self.img.dimensions()
    }

    /// Draw the given `pix`el at `x` and `y`. It does nothing if the
    /// coordinates are out of bounds.
    pub fn draw_pixel(&mut self, x: u32, y: u32, pix: &I::Pixel) {
        if x >= self.img.width() || y >= self.img.height() {
            return;
        }

        let old_pix = self.img.get_pixel_mut(x, y);
        B::blend(old_pix, pix);
    }

    /// Draw a line on the given image using [Bresenham's line
    /// algorithm](https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm).
    pub fn line(&mut self, start: PointU32, end: PointU32, pix: &I::Pixel) {
        let it = BresenhamLineIter::new(start, end);
        for pt in it {
            self.draw_pixel(pt.x, pt.y, pix);
        }
    }

    /// Draw a hollow triangle on the given image.
    pub fn hollow_triangle(&mut self, p1: PointU32, p2: PointU32, p3: PointU32, pix: &I::Pixel) {
        self.line(p1, p2, pix);
        self.line(p1, p3, pix);
        self.line(p2, p3, pix);
    }

    /// Draw a triangle on the given image filled with the given `pix`.
    pub fn triangle(&mut self, p1: PointU32, p2: PointU32, p3: PointU32, pix: &I::Pixel) {
        // the idea here is pretty simple: divide the triangle in an upper and
        // bottom flat triangles. At that point draw horizontal lines between the
        // edge points of the triangle.
        //
        //          /\
        // _______ /__\_____________  separating line
        //         \   \
        //           \  \
        //             \ \
        //               \

        let (tl, mid, br) = {
            // ugly as hell, but easier than hand written comparisons...
            let mut tmp = [p1, p2, p3];
            tmp.sort_by_key(|p| (p.y, p.x));

            (tmp[0], tmp[1], tmp[2])
        };

        let mid_y = f64::from(mid.y);
        let tl_y = f64::from(tl.y);
        let br_y = f64::from(br.y);
        let br_x = f64::from(br.x);
        let tl_x = f64::from(tl.x);

        let break_point = Point::new(
            (tl_x + (mid_y - tl_y) / (br_y - tl_y) * (br_x - tl_x)) as u32,
            mid.y,
        );

        let upper_triangle = FlatTriangleIter::new(tl, mid, break_point);
        for (start, end) in upper_triangle {
            self.line(start, end, pix);
        }

        let mut bottom_triangle = FlatTriangleIter::new(br, break_point, mid).peekable();
        loop {
            let mpoints = bottom_triangle.next();

            match mpoints {
                Some((start, end)) => {
                    // make sure to do not draw the line between the last points because
                    // it's the line that separates the upper_triangle and bottom_triangle
                    // and we've already drawn it in the upper_triangle loop. This is
                    // because we don't want to blend the pixels twice.
                    let are_last_points = bottom_triangle.peek().is_none();

                    if !are_last_points {
                        self.line(start, end, pix);
                    }
                }
                _ => break,
            }
        }
    }

    /// Draw a rectangle filled with the given pixel.
    pub fn rect(&mut self, rect: &BoundingBox<u32>, pix: &I::Pixel) {
        if rect.is_empty() {
            return;
        }

        for y in rect.min().y..=rect.max().y {
            self.line(
                PointU32::new(rect.min().x, y),
                PointU32::new(rect.max().x, y),
                pix,
            );
        }
    }

    /// Draw a closed path formed by the given set of points. Note that the line
    /// between the first and the last point is automatically drawn.
    pub fn closed_path<P: IntoIterator<Item = PointU32>>(&mut self, points: P, pix: &I::Pixel) {
        let mut points = points.into_iter();

        if let Some(first) = points.next() {
            let last = points.fold(first, |prev, cur| {
                self.line(prev, cur, pix);

                cur
            });

            self.line(last, first, pix);
        }
    }

    /// Draw a polygon filled with the given pixel using a simplified version of
    /// the polygon fill algorithm. Doesn't work with self intersecting polygons
    /// and it does no checks to prevent that.
    pub fn polygon(&mut self, polygon: &Polygon<u32>, pix: &I::Pixel) {
        let (ymin, ymax) = polygon
            .points()
            .iter()
            .fold((u32::max_value(), u32::min_value()), |(ymin, ymax), pt| {
                (ymin.min(pt.y), ymax.max(pt.y))
            });

        for y in ymin..=ymax {
            let intersected_segments = polygon.edges().filter(|(p0, p1)| {
                let (edge_min_y, edge_max_y) = if p0.y < p1.y {
                    (p0.y, p1.y)
                } else {
                    (p1.y, p0.y)
                };

                edge_min_y <= y && edge_max_y > y
            });

            let mut xs = intersected_segments
                .map(|(p0, p1)| {
                    let line = LineEquation::between(&p0.cast::<f64>(), &p1.cast::<f64>());

                    let x = line.x_at(f64::from(y)).unwrap();
                    debug_assert!(x >= 0.0 && x <= f64::from(u32::max_value()));

                    x as u32
                })
                .collect::<Vec<_>>();

            xs.sort_unstable();

            for pixs in xs.chunks(2) {
                if pixs.len() < 2 {
                    break;
                }

                for x in pixs[0]..=pixs[1] {
                    self.draw_pixel(x, y, pix);
                }
            }
        }
    }

    // TODO: implement a version of polygon that works with self-intersecting
    // polygons. Example points for a self-intersecting polygon:
    //
    // let points = vec![
    //     matto::geo::Point { x: 392, y: 23 },
    //     matto::geo::Point { x: 134, y: 59 },
    //     matto::geo::Point { x: 251, y: 127 },
    //     matto::geo::Point { x: 266, y: 143 },
    //     matto::geo::Point { x: 380, y: 183 },
    //     matto::geo::Point { x: 337, y: 44 },
    //     matto::geo::Point { x: 229, y: 20 },
    //     matto::geo::Point { x: 378, y: 496 },
    //     matto::geo::Point { x: 392, y: 23 },
    // ];
    //
    // pub fn polygon_flood_fill()
}

impl<'a, I, B> Drawer<'a, I, B>
where
    I: image::GenericImage,
    I::Pixel: Debug,
    B: Blender<I::Pixel>,
    f64: From<<I::Pixel as image::Pixel>::Subpixel>,
{
    /// Draw an antialiased line using a variation of [`Xiaolin Wu's line
    /// algorithm`](https://en.wikipedia.org/wiki/Xiaolin_Wu%27s_line_algorithm).
    pub fn antialiased_line(&mut self, mut start: PointU32, mut end: PointU32, pix: &I::Pixel) {
        use std::mem;

        let mut dx = (<i64 as From<u32>>::from(end.x) - <i64 as From<u32>>::from(start.x)).abs();
        let mut dy = (<i64 as From<u32>>::from(end.y) - <i64 as From<u32>>::from(start.y)).abs();

        let is_steep = dy > dx;

        // the `antialised_line_impl` assumes non steep lines, therefore we swap
        // x and y to preserve this invariant. We'll use the `coord_selector`
        // parameter to swap the coordinates again just before writing onto the
        // image.
        if is_steep {
            mem::swap(&mut start.x, &mut start.y);
            mem::swap(&mut end.x, &mut end.y);
            mem::swap(&mut dx, &mut dy);
        }

        if start.x > end.x {
            mem::swap(&mut start, &mut end);
        }

        if is_steep {
            self.antialised_line_impl(start, end, pix, dx, dy, |x, y| (y, x));
        } else {
            self.antialised_line_impl(start, end, pix, dx, dy, |x, y| (x, y));
        }
    }

    /// heavily based on
    /// https://en.wikipedia.org/wiki/Xiaolin_Wu%27s_line_algorithm#Algorithm.
    /// Assumes the line is _not_ steep and `start.x <= end.x`, if unsure call `antialised_line`.
    /// `coord_selector` is used in order to restore the proper x and y
    /// coordinates before drawing onto the image because in the case of a steep
    /// line x and y were swapped.
    fn antialised_line_impl(
        &mut self,
        start: PointU32,
        end: PointU32,
        pix: &I::Pixel,
        dx: i64,
        dy: i64,
        coord_selector: impl Fn(u32, u32) -> (u32, u32),
    ) {
        // local import because otherwise using convert::From in other parts
        // will be a pain
        use num::traits::cast::NumCast;

        debug_assert!(dx >= dy);
        debug_assert!(start.x <= end.x);

        // since the points are u32 there is no fractional part and so we don't
        // need to draw the second point for each of the endpoints like in the
        // wikipedia pseudocode.
        for pt in &[start, end] {
            let (x, y) = coord_selector(pt.x, pt.y);
            self.draw_pixel(x, y, pix);
        }

        let gradient = if dx == 0 { 1.0 } else { dy as f64 / dx as f64 };
        let gradient = if start.y > end.y { -gradient } else { gradient };
        let mut intery = <f64 as From<u32>>::from(start.y) + gradient;

        for x in (start.x + 1)..end.x {
            let pts = [
                (intery.floor(), 1.0 - intery.fract()),
                (intery.floor() + 1.0, intery.fract()),
            ];

            for (y, weight) in &pts {
                // linear interpolation of the channels, might want to fancier
                // in the future and/or allow custom interpolation functions,
                // but kiss for now.
                let pix = pix.map(|c| {
                    <<I::Pixel as image::Pixel>::Subpixel as NumCast>::from(
                        <f64 as From<_>>::from(c) * weight,
                    )
                    .unwrap()
                });

                let (x, y) = coord_selector(x, *y as u32);
                self.draw_pixel(x, y, &pix);
            }

            intery += gradient;
        }
    }
}

/// Noop Blender
pub struct NoopBlender;

impl<P: image::Pixel> Blender<P> for NoopBlender {
    fn blend(dst: &mut P, src: &P) {
        *dst = *src;
    }
}

/// Default Blender
pub struct DefaultBlender;

impl<P: image::Pixel> Blender<P> for DefaultBlender {
    fn blend(dst: &mut P, src: &P) {
        dst.blend(src);
    }
}
