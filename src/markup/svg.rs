use std::fmt::Write;

use anyhow::Result;
use crownui::prelude::{Color, Point, Size};
use resvg::{tiny_skia, usvg};

use super::annotation::{Annotation, Shape};
use crate::util::fonts::system_fonts;
use crate::util::pixels::unpremultiply_rgba;

const FONT_FAMILY: &str = "Inter, sans-serif";
const ADVANCE_PER_CHARACTER: f32 = 0.62;
const LINE_HEIGHT: f32 = 1.35;

/// The whole markup layer as one SVG document the size of the image.
pub fn overlay(annotations: &[Annotation], size: Size) -> String {
    let mut document = open_document(size);
    for annotation in annotations {
        match &annotation.shape {
            Shape::Text(note) => push_text(
                &mut document,
                &note.content,
                note.font_size,
                annotation.color,
                note.origin,
            ),
            _ => annotation
                .polylines()
                .iter()
                .for_each(|line| push_polyline(&mut document, line, annotation)),
        }
    }
    document.push_str("</svg>");
    document
}

/// `content` set in the markup font, as straight-alpha RGBA sized to fit it.
pub fn rasterized_text(content: &str, font_size: f32, color: Color) -> Option<(Size, Vec<u8>)> {
    let size = Size::new(
        (content.chars().count() as f32 * ADVANCE_PER_CHARACTER + 0.5) * font_size,
        font_size * LINE_HEIGHT,
    );
    let mut document = open_document(size);
    push_text(&mut document, content, font_size, color, Point::ZERO);
    document.push_str("</svg>");
    let mut pixmap = tiny_skia::Pixmap::new(size.width.ceil() as u32, size.height.ceil() as u32)?;
    render_onto(&document, &mut pixmap.as_mut()).ok()?;
    let mut rgba = pixmap.take();
    unpremultiply_rgba(&mut rgba);
    Some((size, rgba))
}

pub fn render_onto(document: &str, target: &mut tiny_skia::PixmapMut<'_>) -> Result<()> {
    let options = usvg::Options {
        fontdb: system_fonts(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_str(document, &options)?;
    resvg::render(&tree, tiny_skia::Transform::identity(), target);
    Ok(())
}

fn open_document(size: Size) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
        w = size.width,
        h = size.height
    )
}

fn paint(color: Color) -> (String, f32) {
    let [red, green, blue, _] = color.to_rgba8();
    (format!("rgb({red},{green},{blue})"), color.a)
}

fn push_polyline(document: &mut String, line: &[Point], annotation: &Annotation) {
    let (stroke, opacity) = paint(annotation.color);
    document.push_str(
        r#"<polyline fill="none" stroke-linecap="round" stroke-linejoin="round" points=""#,
    );
    for point in line {
        let _ = write!(document, "{},{} ", point.x, point.y);
    }
    let _ = write!(
        document,
        r#"" stroke="{stroke}" stroke-opacity="{opacity}" stroke-width="{}"/>"#,
        annotation.width
    );
}

fn push_text(document: &mut String, content: &str, font_size: f32, color: Color, origin: Point) {
    let (fill, opacity) = paint(color);
    let _ = write!(
        document,
        r#"<text x="{}" y="{}" font-family="{FONT_FAMILY}" font-size="{font_size}" font-weight="600" fill="{fill}" fill-opacity="{opacity}">{}</text>"#,
        origin.x,
        origin.y + font_size,
        escaped(content)
    );
}

fn escaped(content: &str) -> String {
    content
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_is_escaped_into_valid_svg() {
        let note = Annotation {
            shape: Shape::Line(Point::ZERO, Point::new(4.0, 4.0)),
            color: Color::new(1.0, 0.0, 0.0, 1.0),
            width: 2.0,
        };
        let document = overlay(&[note], Size::new(8.0, 8.0));
        assert!(document.contains(r#"points="0,0 4,4 ""#));
        assert!(usvg::Tree::from_str(&document, &usvg::Options::default()).is_ok());
        assert_eq!(escaped("a<b & c>"), "a&lt;b &amp; c&gt;");
    }
}
