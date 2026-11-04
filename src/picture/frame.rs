use std::time::Duration;

/// One decoded image in straight-alpha RGBA8.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub delay: Duration,
}

/// A still image is one frame; an animation is several.
pub struct Picture {
    pub frames: Vec<Frame>,
}

impl Picture {
    pub fn still(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            frames: vec![Frame {
                width,
                height,
                rgba,
                delay: Duration::ZERO,
            }],
        }
    }
}
