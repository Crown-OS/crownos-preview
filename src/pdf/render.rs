use cairo::{Context, Format, ImageSurface};
use poppler::Page;

use crate::util::pixels::{packed_rows, premultiplied_bgra_to_rgba};

pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// `page` rasterised at `scale` pixels per point onto white paper.
pub fn render(page: &Page, scale: f64) -> Option<Bitmap> {
    let (points_wide, points_high) = page.size();
    let width = (points_wide * scale).ceil() as i32;
    let height = (points_high * scale).ceil() as i32;
    let mut surface = ImageSurface::create(Format::ARgb32, width, height).ok()?;
    {
        let context = Context::new(&surface).ok()?;
        context.set_source_rgb(1.0, 1.0, 1.0);
        context.paint().ok()?;
        context.scale(scale, scale);
        page.render(&context);
    }
    surface.flush();
    let stride = surface.stride() as usize;
    let row_bytes = width as usize * 4;
    let data = surface.data().ok()?;
    let mut rgba = packed_rows(&data, stride, row_bytes, height as usize);
    premultiplied_bgra_to_rgba(&mut rgba);
    Some(Bitmap {
        width: width as u32,
        height: height as u32,
        rgba,
    })
}
