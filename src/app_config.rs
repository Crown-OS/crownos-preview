use crownui::AppConfig;
use crownui::kit::{INTER_FAMILY, inter_font};
use crownui::prelude::{BLACK, CornerShape};

use crate::util;

const INITIAL_SIZE: (u32, u32) = (960, 680);
const MIN_SIZE: (u32, u32) = (420, 320);

pub fn app_config() -> AppConfig {
    let config = AppConfig::new("Preview")
        .app_id("crownpreview")
        .size(INITIAL_SIZE.0, INITIAL_SIZE.1)
        .min_size(MIN_SIZE.0, MIN_SIZE.1)
        .clear_color(BLACK)
        .corner_shape(CornerShape::Squircle)
        .font(inter_font().to_vec(), Some(INTER_FAMILY));
    match util::fonts::monospace_font() {
        Some(monospace) => config.font(monospace, Some(util::fonts::MONOSPACE_FAMILY)),
        None => config,
    }
}
