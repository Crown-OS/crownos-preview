use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crownui::prelude::{Rect, Size};
use image::{DynamicImage, ImageFormat, RgbaImage, imageops};
use resvg::tiny_skia::{IntSize, Pixmap};

use super::annotation::Annotation;
use super::svg::{overlay, render_onto};
use crate::document::DocumentKind;
use crate::picture;
use crate::util::pixels::{premultiply_rgba, unpremultiply_rgba};

/// Burns `annotations` into the image at `path`, applies `crop`, and writes
/// the result back: in place when the format can be written, otherwise as a
/// PNG beside it. Returns where it was written.
pub fn save(
    path: &Path,
    kind: DocumentKind,
    annotations: &[Annotation],
    crop: Option<Rect>,
) -> Result<PathBuf> {
    let frame = picture::decode(path, kind)?
        .frames
        .into_iter()
        .next()
        .context("the image has no frames")?;
    let (width, height) = (frame.width, frame.height);
    let mut rgba = frame.rgba;
    premultiply_rgba(&mut rgba);
    let mut pixmap = IntSize::from_wh(width, height)
        .and_then(|size| Pixmap::from_vec(rgba, size))
        .context("the image has no area")?;
    let document = overlay(annotations, Size::new(width as f32, height as f32));
    render_onto(&document, &mut pixmap.as_mut())?;
    let mut rgba = pixmap.take();
    unpremultiply_rgba(&mut rgba);
    let image = RgbaImage::from_raw(width, height, rgba).context("pixel buffer size mismatch")?;
    let image = match crop.filter(|crop| crop.size.width >= 1.0 && crop.size.height >= 1.0) {
        Some(crop) => imageops::crop_imm(
            &image,
            crop.origin.x as u32,
            crop.origin.y as u32,
            crop.size.width as u32,
            crop.size.height as u32,
        )
        .to_image(),
        None => image,
    };
    write(DynamicImage::ImageRgba8(image), &target_for(path, kind))
}

fn target_for(path: &Path, kind: DocumentKind) -> PathBuf {
    match kind {
        DocumentKind::Raster => path.to_path_buf(),
        _ => path.with_extension("png"),
    }
}

fn write(image: DynamicImage, target: &Path) -> Result<PathBuf> {
    let format = ImageFormat::from_path(target)?;
    let image = match format {
        ImageFormat::Jpeg => DynamicImage::ImageRgb8(image.to_rgb8()),
        _ => image,
    };
    let partial = target.with_extension("partial");
    image
        .save_with_format(&partial, format)
        .with_context(|| format!("cannot write {}", target.display()))?;
    std::fs::rename(&partial, target)?;
    Ok(target.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::annotation::Shape;
    use crownui::prelude::{Point, rgb};

    #[test]
    fn marks_are_burnt_in_and_the_crop_applied_in_place() {
        let path = std::env::temp_dir().join(format!("crownos-preview-{}.png", std::process::id()));
        RgbaImage::from_pixel(40, 30, image::Rgba([255, 255, 255, 255]))
            .save(&path)
            .expect("write the fixture");
        let line = Annotation {
            shape: Shape::Line(Point::new(0.0, 10.0), Point::new(40.0, 10.0)),
            color: rgb(0xff0000),
            width: 4.0,
        };
        let crop = Rect::new(Point::new(5.0, 5.0), Size::new(20.0, 10.0));
        let written = save(&path, DocumentKind::Raster, &[line], Some(crop)).expect("save");
        let saved = image::open(&written).expect("reopen").into_rgba8();
        std::fs::remove_file(&written).ok();
        assert_eq!(written, path);
        assert_eq!(saved.dimensions(), (20, 10));
        assert_eq!(
            saved.get_pixel(10, 5).0,
            [255, 0, 0, 255],
            "the line crosses the crop"
        );
        assert_eq!(saved.get_pixel(10, 0).0, [255, 255, 255, 255]);
    }
}
