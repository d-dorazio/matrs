//! Simple module to draw basic shapes on an image. Most of this stuff is
//! already implemented in the imageprocs crate, but the best way to learn is by
//! reimplementing, so...

extern crate image;

use std::iter::Iterator;
use std::mem;

use geo::{Point, PointU32};

/// Iterator that returns all the points that compose the line from start to
/// end. It uses the [Bresenham's line
/// algorithm](https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm) to
/// interpolate the points in the line. Note that the points are returned in
/// order that is if start is higher than end(i.e. start.y < end.y) then the
/// points will be returned by starting from the top falling down.
#[derive(Debug)]
pub struct BresenhamLineIter {
    // this struct is designed to work for non steep lines. In case we actually
    // want to iterate over a steep line then the `new` function swaps x with y,
    // sets `is_steep` that is then checked in `next` and swaps x with y again
    // if the flag is set. It also assumes that `start` is the more "bottom
    // left" than `end`(this invariant is also ensured by `new`).
    start: Point<i64>,
    end: PointU32,
    is_steep: bool,
    d: i64,
    dx: i64,
    dy: i64,
    xstep: i64,
    ystep: i64,
}

impl BresenhamLineIter {
    /// Creates a new `BresenhamLineIter` iterator to return all points between
    /// `start` and `end` both included.
    pub fn new(mut start: PointU32, mut end: PointU32) -> BresenhamLineIter {
        let mut dx = (i64::from(end.x) - i64::from(start.x)).abs();
        let mut dy = (i64::from(end.y) - i64::from(start.y)).abs();

        let is_steep;

        // find out whether the line is steep that is that whether it grows faster
        // in y or in x and call the appropriate implementation. The algorithms are
        // the mirrors of each other, but the main idea is the same: the bump of the
        // slowest coordinate is governed by whether the value is closer to the new
        // coord or not.
        if dx >= dy {
            is_steep = false;
        } else {
            is_steep = true;

            mem::swap(&mut start.x, &mut start.y);
            mem::swap(&mut end.x, &mut end.y);
            mem::swap(&mut dx, &mut dy);
        }

        let xstep = if start.x > end.x { -1 } else { 1 };
        let ystep = if start.y > end.y { -1 } else { 1 };

        let start = Point {
            x: i64::from(start.x),
            y: i64::from(start.y),
        };

        BresenhamLineIter {
            start,
            end,
            is_steep,
            dx,
            dy,
            d: 2 * dy - dx,
            ystep,
            xstep,
        }
    }

    // calculate next non steep point in the line
    fn next_non_steep_point(&mut self) -> Option<PointU32> {
        if (self.start.x > i64::from(self.end.x) && self.xstep > 0)
            || (self.start.x < i64::from(self.end.x) && self.xstep < 0)
        {
            return None;
        }

        if self.start.x < 0 || self.start.y < 0 {
            return None;
        }

        let old = PointU32 {
            x: self.start.x as u32,
            y: self.start.y as u32,
        };

        if self.d > 0 {
            self.start.y += self.ystep;
            self.d -= 2 * self.dx;
        }

        self.d += 2 * self.dy;

        self.start.x += self.xstep;

        Some(old)
    }
}

impl Iterator for BresenhamLineIter {
    type Item = PointU32;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_non_steep_point().map(|mut res| {
            // in case the line is steep then we need to swap back the
            // coordinates before returning to reverse the swap done in `new`.
            if self.is_steep {
                mem::swap(&mut res.x, &mut res.y);
            }
            res
        })
    }
}

/// Draw a line on the given image using [Bresenham's line
/// algorithm](https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm).
pub fn line<I>(img: &mut I, start: PointU32, end: PointU32, pix: &I::Pixel)
where
    I: image::GenericImage,
{
    let it = BresenhamLineIter::new(start, end);
    for pt in it {
        if pt.x >= img.width() || pt.y >= img.height() {
            break;
        }

        img.put_pixel(pt.x, pt.y, *pix);
    }
}

/// Draw a hollow triangle on the given image.
pub fn hollow_triangle<I>(img: &mut I, p1: &PointU32, p2: &PointU32, p3: &PointU32, pix: &I::Pixel)
where
    I: image::GenericImage,
{
    line(img, p1.clone(), p2.clone(), pix);
    line(img, p1.clone(), p3.clone(), pix);
    line(img, p2.clone(), p3.clone(), pix);
}

/// Draw a triangle on the given image filled with the given pix.
pub fn triangle<I>(img: &mut I, p1: &PointU32, p2: &PointU32, p3: &PointU32, pix: &I::Pixel)
where
    I: image::GenericImage,
{
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

    triangle_impl(img, tl, mid, &break_point, pix);
    triangle_impl(img, br, &break_point, mid, pix);
}

fn triangle_impl<I>(img: &mut I, p1: &PointU32, p2: &PointU32, p3: &PointU32, pix: &I::Pixel)
where
    I: image::GenericImage,
{
    let mut p1p2 = BresenhamLineIter::new(p1.clone(), p2.clone());
    let mut p1p3 = BresenhamLineIter::new(p1.clone(), p3.clone());

    let mut last_start = p1.clone();
    let mut last_end = p1.clone();

    let mut exit = false;

    while !exit {
        line(img, last_start.clone(), last_end.clone(), pix);

        loop {
            match p1p2.next() {
                Some(new_start) => {
                    if new_start.y != last_start.y {
                        last_start = new_start;
                        break;
                    }
                }
                None => {
                    exit = true;
                    break;
                }
            }
        }

        while let Some(new_end) = p1p3.next() {
            if new_end.y != last_end.y {
                last_end = new_end;
                break;
            }
        }
    }

    line(img, last_start.clone(), last_end.clone(), pix);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _test_line_bresenham(start: PointU32, end: PointU32, exp_points: Vec<PointU32>) {
        assert_eq!(
            BresenhamLineIter::new(start.clone(), end.clone()).collect::<Vec<_>>(),
            exp_points,
            "line from start {:?} to end {:?}",
            start,
            end,
        );

        assert_eq!(
            BresenhamLineIter::new(end.clone(), start.clone()).collect::<Vec<_>>(),
            exp_points.iter().cloned().rev().collect::<Vec<_>>(),
            "line from end {:?} to start {:?}",
            end,
            start,
        );
    }

    #[test]
    fn test_bresenham_line_basic() {
        let origin = Point { x: 0, y: 0 };

        _test_line_bresenham(origin.clone(), origin.clone(), vec![origin.clone()]);

        let bis = Point { x: 3, y: 3 };
        let bis_exp_points = vec![
            origin.clone(),
            Point { x: 1, y: 1 },
            Point { x: 2, y: 2 },
            bis.clone(),
        ];

        _test_line_bresenham(origin.clone(), bis.clone(), bis_exp_points);
    }

    #[test]
    fn test_bresenham_line_non_steep() {
        let origin = Point { x: 0, y: 0 };
        let non_steep_pt = Point { x: 3, y: 1 };
        let exp_points = vec![
            origin.clone(),
            Point { x: 1, y: 0 },
            Point { x: 2, y: 1 },
            non_steep_pt.clone(),
        ];

        _test_line_bresenham(origin.clone(), non_steep_pt.clone(), exp_points);
    }

    #[test]
    fn test_bresenham_line_steep() {
        let origin = Point { x: 0, y: 0 };
        let steep_pt = Point { x: 1, y: 3 };
        let exp_points = vec![
            origin.clone(),
            Point { x: 0, y: 1 },
            Point { x: 1, y: 2 },
            steep_pt.clone(),
        ];

        _test_line_bresenham(origin.clone(), steep_pt.clone(), exp_points);
    }

    #[test]
    fn test_bresenham_line_dec() {
        let start = Point { x: 4, y: 0 };
        let end = Point { x: 1, y: 3 };
        let exp_points = vec![
            start.clone(),
            Point { x: 3, y: 1 },
            Point { x: 2, y: 2 },
            end.clone(),
        ];

        _test_line_bresenham(start.clone(), end.clone(), exp_points);
    }
}
