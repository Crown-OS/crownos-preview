mod app_config;
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

use launch::Request;

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
    crownui::run(viewer::root(request, server), app_config::app_config())?;
    Ok(())
}
