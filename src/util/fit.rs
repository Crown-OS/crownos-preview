use crownui::prelude::{Point, Rect, Size};

/// The largest scale at which `content` fits inside `bounds`.
pub fn fit_scale(content: Size, bounds: Size) -> f32 {
    if content.width <= 0.0 || content.height <= 0.0 {
        return 1.0;
    }
    (bounds.width / content.width).min(bounds.height / content.height)
}

/// `content` scaled by `scale` and centered in `bounds`, then shifted by `pan`.
pub fn centered(content: Size, scale: f32, bounds: Size, pan: Point) -> Rect {
    let size = Size::new(content.width * scale, content.height * scale);
    let origin = Point::new(
        (bounds.width - size.width) / 2.0 + pan.x,
        (bounds.height - size.height) / 2.0 + pan.y,
    );
    Rect::new(origin, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_content_fits_by_width() {
        let scale = fit_scale(Size::new(400.0, 100.0), Size::new(200.0, 200.0));
        assert_eq!(scale, 0.5);
        let rect = centered(
            Size::new(400.0, 100.0),
            scale,
            Size::new(200.0, 200.0),
            Point::new(0.0, 0.0),
        );
        assert_eq!(
            rect,
            Rect::new(Point::new(0.0, 75.0), Size::new(200.0, 50.0))
        );
    }
}
