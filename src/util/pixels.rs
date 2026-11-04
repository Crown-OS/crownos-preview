/// Converts premultiplied native-endian ARGB32 (cairo's layout) into
/// straight-alpha RGBA8 in place.
pub fn premultiplied_bgra_to_rgba(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let [blue, green, red, alpha] = [pixel[0], pixel[1], pixel[2], pixel[3]];
        let unpremultiply = |channel: u8| match alpha {
            0 => 0,
            u8::MAX => channel,
            _ => ((u16::from(channel) * 255 + u16::from(alpha) / 2) / u16::from(alpha)) as u8,
        };
        pixel.copy_from_slice(&[
            unpremultiply(red),
            unpremultiply(green),
            unpremultiply(blue),
            alpha,
        ]);
    }
}

/// Converts premultiplied RGBA8 (tiny-skia's layout) into straight alpha in place.
pub fn unpremultiply_rgba(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = pixel[3];
        if alpha != 0 && alpha != u8::MAX {
            for channel in &mut pixel[..3] {
                *channel =
                    ((u16::from(*channel) * 255 + u16::from(alpha) / 2) / u16::from(alpha)) as u8;
            }
        }
    }
}

/// Converts straight-alpha RGBA8 into premultiplied alpha in place.
pub fn premultiply_rgba(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);
        if alpha != 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u16::from(*channel) * alpha + 127) / 255) as u8;
            }
        }
    }
}

/// The first `row_bytes` of each of `rows` rows spaced `stride` apart, packed tightly.
pub fn packed_rows(data: &[u8], stride: usize, row_bytes: usize, rows: usize) -> Vec<u8> {
    if stride == row_bytes {
        return data[..row_bytes * rows].to_vec();
    }
    let mut packed = Vec::with_capacity(row_bytes * rows);
    data.chunks(stride)
        .take(rows)
        .for_each(|row| packed.extend_from_slice(&row[..row_bytes]));
    packed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_pixels_only_swap_channels() {
        let mut pixels = [10, 20, 30, 255, 0, 0, 0, 0];
        premultiplied_bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, [30, 20, 10, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn translucent_pixels_are_unpremultiplied() {
        let mut pixels = [0, 0, 64, 128];
        premultiplied_bgra_to_rgba(&mut pixels);
        assert_eq!(pixels, [128, 0, 0, 128]);
    }
}
