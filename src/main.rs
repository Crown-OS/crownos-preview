mod document;
mod launch;
mod markup;
mod media;
mod pdf;
mod picture;
mod playlist;
mod service;
mod text;
mod util;
mod viewer;

use anyhow::{Context, Result};
use crownui::AppConfig;
use crownui::kit::{INTER_FAMILY, inter_font};

use launch::Request;

const INITIAL_SIZE: (u32, u32) = (960, 680);
const MIN_SIZE: (u32, u32) = (420, 320);
const USAGE: &str = "usage: crownos-preview [--edit] [previewable file]";

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1).peekable();
    let request = match args.peek() {
        None => None,
        Some(_) => Some(Request::from_args(args).context(USAGE)?),
    };
    if service::forward_to_running(request.as_ref())? {
        return Ok(());
    }
    let server = service::bind()?;
    let mut config = AppConfig::new("Preview")
        .app_id("crownpreview")
        .size(INITIAL_SIZE.0, INITIAL_SIZE.1)
        .min_size(MIN_SIZE.0, MIN_SIZE.1)
        .font(inter_font().to_vec(), Some(INTER_FAMILY));
    if let Some(monospace) = util::fonts::monospace_font() {
        config = config.font(monospace, Some(util::fonts::MONOSPACE_FAMILY));
    }
    crownui::run(viewer::root(request, server), config)?;
    Ok(())
}
