use crownui::prelude::{Point, Rect, Size};

/// The rectangle spanned by two corners given in any order.
pub fn spanned(anchor: Point, corner: Point) -> Rect {
    let origin = Point::new(anchor.x.min(corner.x), anchor.y.min(corner.y));
    Rect::new(
        origin,
        Size::new((anchor.x - corner.x).abs(), (anchor.y - corner.y).abs()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_are_normalised() {
        let rect = spanned(Point::new(10.0, 40.0), Point::new(4.0, 20.0));
        assert_eq!(rect, Rect::new(Point::new(4.0, 20.0), Size::new(6.0, 20.0)));
    }
}
