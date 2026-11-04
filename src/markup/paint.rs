use crownui::ext::{DrawCommand, DrawList, Paint, Path, Stroke, StrokeCap, StrokeJoin};
use crownui::prelude::{Color, CornerRadii, Point, Rect, Size, rgba};

use super::annotation::{Annotation, Shape};

const CROP_SHADE: Color = rgba(0x0000_0088);
const CROP_EDGE: Color = rgba(0xffff_ffff);
const CROP_EDGE_WIDTH: f32 = 1.5;

/// Draws marks given in image pixels onto `image_rect`, where the image is
/// shown at `scale` view pixels per image pixel.
pub fn paint(
    list: &mut DrawList,
    annotations: &[Annotation],
    draft: Option<&Annotation>,
    crop: Option<Rect>,
    image_rect: Rect,
    scale: f32,
) {
    let to_view = |point: Point| {
        Point::new(
            image_rect.origin.x + point.x * scale,
            image_rect.origin.y + point.y * scale,
        )
    };
    for annotation in annotations.iter().chain(draft) {
        if let Shape::Text(note) = &annotation.shape {
            let size = Size::new(note.size.width * scale, note.size.height * scale);
            list.image(note.image, Rect::new(to_view(note.origin), size));
            continue;
        }
        let stroke = Stroke {
            width: annotation.width * scale,
            cap: StrokeCap::Round,
            join: StrokeJoin::Round,
        };
        for line in annotation.polylines() {
            let Some((first, rest)) = line.split_first() else {
                continue;
            };
            let path = rest
                .iter()
                .fold(Path::new().move_to(to_view(*first)), |path, point| {
                    path.line_to(to_view(*point))
                });
            list.push(DrawCommand::StrokePath(
                path,
                stroke,
                Paint::solid(annotation.color),
            ));
        }
    }
    if let Some(crop) = crop {
        paint_crop(
            list,
            image_rect,
            Rect::new(
                to_view(crop.origin),
                Size::new(crop.size.width * scale, crop.size.height * scale),
            ),
        );
    }
}

fn paint_crop(list: &mut DrawList, image: Rect, kept: Rect) {
    let shade = |list: &mut DrawList, left: f32, top: f32, right: f32, bottom: f32| {
        if right > left && bottom > top {
            let rect = Rect::new(Point::new(left, top), Size::new(right - left, bottom - top));
            list.fill(rect, CornerRadii::default(), CROP_SHADE);
        }
    };
    shade(
        list,
        image.min_x(),
        image.min_y(),
        image.max_x(),
        kept.min_y(),
    );
    shade(
        list,
        image.min_x(),
        kept.max_y(),
        image.max_x(),
        image.max_y(),
    );
    shade(
        list,
        image.min_x(),
        kept.min_y(),
        kept.min_x(),
        kept.max_y(),
    );
    shade(
        list,
        kept.max_x(),
        kept.min_y(),
        image.max_x(),
        kept.max_y(),
    );
    list.stroke(kept, CornerRadii::default(), CROP_EDGE_WIDTH, CROP_EDGE);
}
