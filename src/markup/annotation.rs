use std::f32::consts::{FRAC_PI_6, TAU};

use crownui::ext::ImageId;
use crownui::prelude::{Color, Point, Rect, Size};

const ELLIPSE_SEGMENTS: usize = 64;
const ARROW_HEAD_PER_WIDTH: f32 = 5.0;

/// Text placed on the image, pre-rendered once for display.
#[derive(Debug, Clone, PartialEq)]
pub struct TextNote {
    pub origin: Point,
    pub content: String,
    pub font_size: f32,
    pub image: ImageId,
    pub size: Size,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Stroke(Vec<Point>),
    Line(Point, Point),
    Arrow(Point, Point),
    Rectangle(Rect),
    Ellipse(Rect),
    Text(TextNote),
}

/// One mark in image pixel coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub shape: Shape,
    pub color: Color,
    pub width: f32,
}

impl Annotation {
    /// The open polylines that draw this mark; text has none.
    pub fn polylines(&self) -> Vec<Vec<Point>> {
        match &self.shape {
            Shape::Stroke(points) => vec![points.clone()],
            Shape::Line(start, end) => vec![vec![*start, *end]],
            Shape::Arrow(start, end) => {
                vec![vec![*start, *end], arrow_head(*start, *end, self.width)]
            }
            Shape::Rectangle(rect) => vec![rectangle(*rect)],
            Shape::Ellipse(rect) => vec![ellipse(*rect)],
            Shape::Text(_) => Vec::new(),
        }
    }
}

fn rectangle(rect: Rect) -> Vec<Point> {
    let (left, top, right, bottom) = (rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y());
    vec![
        Point::new(left, top),
        Point::new(right, top),
        Point::new(right, bottom),
        Point::new(left, bottom),
        Point::new(left, top),
    ]
}

fn ellipse(rect: Rect) -> Vec<Point> {
    let center = rect.center();
    let (radius_x, radius_y) = (rect.size.width / 2.0, rect.size.height / 2.0);
    (0..=ELLIPSE_SEGMENTS)
        .map(|step| {
            let angle = step as f32 / ELLIPSE_SEGMENTS as f32 * TAU;
            Point::new(
                center.x + radius_x * angle.cos(),
                center.y + radius_y * angle.sin(),
            )
        })
        .collect()
}

fn arrow_head(start: Point, tip: Point, width: f32) -> Vec<Point> {
    let angle = (tip.y - start.y).atan2(tip.x - start.x);
    let length = width * ARROW_HEAD_PER_WIDTH;
    let barb = |offset: f32| {
        let direction = angle + std::f32::consts::PI + offset;
        Point::new(
            tip.x + length * direction.cos(),
            tip.y + length * direction.sin(),
        )
    };
    vec![barb(FRAC_PI_6), tip, barb(-FRAC_PI_6)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_shapes_end_where_they_start() {
        let mark = |shape| Annotation {
            shape,
            color: Color::new(1.0, 0.0, 0.0, 1.0),
            width: 2.0,
        };
        let rect = Rect::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0));
        for shape in [Shape::Rectangle(rect), Shape::Ellipse(rect)] {
            let line = &mark(shape).polylines()[0];
            let (first, last) = (line[0], line[line.len() - 1]);
            assert!((first.x - last.x).abs() < 1e-3 && (first.y - last.y).abs() < 1e-3);
        }
        assert_eq!(
            mark(Shape::Arrow(Point::ZERO, Point::new(10.0, 0.0)))
                .polylines()
                .len(),
            2
        );
    }
}
