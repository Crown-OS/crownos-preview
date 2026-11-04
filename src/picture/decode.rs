use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};
use resvg::{tiny_skia, usvg};

use super::{Frame, Picture};
use crate::document::DocumentKind;
use crate::util::fonts::system_fonts;
use crate::util::pixels::{packed_rows, unpremultiply_rgba};

const MAX_ANIMATION_FRAMES: usize = 600;
const MIN_FRAME_DELAY: Duration = Duration::from_millis(20);
const VECTOR_RENDER_EDGE: f32 = 2048.0;

pub fn decode(path: &Path, kind: DocumentKind) -> Result<Picture> {
    match kind {
        DocumentKind::Vector => vector(path),
        DocumentKind::Heif => heif(path),
        _ => raster(path),
    }
}

fn raster(path: &Path) -> Result<Picture> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    if reader.format() == Some(ImageFormat::Gif) {
        return animation(path);
    }
    let mut decoder = reader.into_decoder().context("unsupported image")?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    let rgba = image.into_rgba8();
    Ok(Picture::still(rgba.width(), rgba.height(), rgba.into_raw()))
}

fn animation(path: &Path) -> Result<Picture> {
    let decoder = GifDecoder::new(BufReader::new(File::open(path)?))?;
    let frames = decoder
        .into_frames()
        .take(MAX_ANIMATION_FRAMES)
        .map(|frame| {
            let frame = frame?;
            let delay = Duration::from(frame.delay()).max(MIN_FRAME_DELAY);
            let buffer = frame.into_buffer();
            Ok(Frame {
                width: buffer.width(),
                height: buffer.height(),
                rgba: buffer.into_raw(),
                delay,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    anyhow::ensure!(!frames.is_empty(), "the GIF has no frames");
    Ok(Picture { frames })
}

fn vector(path: &Path) -> Result<Picture> {
    let options = usvg::Options {
        resources_dir: path.parent().map(Path::to_path_buf),
        fontdb: system_fonts(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(&std::fs::read(path)?, &options)?;
    let natural = tree.size();
    let scale = VECTOR_RENDER_EDGE / natural.width().max(natural.height());
    let width = (natural.width() * scale).ceil() as u32;
    let height = (natural.height() * scale).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).context("the SVG has no area")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let mut rgba = pixmap.take();
    unpremultiply_rgba(&mut rgba);
    Ok(Picture::still(width, height, rgba))
}

fn heif(path: &Path) -> Result<Picture> {
    let context = HeifContext::read_from_file(path.to_str().context("non UTF-8 path")?)?;
    let handle = context.primary_image_handle()?;
    let image = LibHeif::new().decode(&handle, ColorSpace::Rgb(RgbChroma::Rgba), None)?;
    let plane = image
        .planes()
        .interleaved
        .context("libheif returned no interleaved plane")?;
    let row_bytes = plane.width as usize * 4;
    let rgba = packed_rows(plane.data, plane.stride, row_bytes, plane.height as usize);
    Ok(Picture::still(plane.width, plane.height, rgba))
}
