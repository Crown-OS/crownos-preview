use std::path::PathBuf;

use anyhow::{Context, Result};
use crownos_ipc::{RemoteError, ServiceBuilder, blocking};
use crownos_preview::protocol::{self, Handler};

#[derive(Default)]
struct Previewer {
    current: Option<PathBuf>,
}

impl Previewer {
    fn show(&mut self, path: PathBuf) -> bool {
        let previewable = path.is_file();
        if previewable {
            eprintln!("crownos-preview: open {}", path.display());
            self.current = Some(path);
        }
        previewable
    }
}

impl Handler for Previewer {
    fn open(&mut self, path: String) -> Result<bool, RemoteError> {
        Ok(self.show(PathBuf::from(path)))
    }

    fn close(&mut self) {
        self.current = None;
    }
}

fn main() -> Result<()> {
    let mut previewer = Previewer::default();
    if let Some(path) = std::env::args_os().nth(1) {
        previewer.show(PathBuf::from(path));
    }
    let mut server = ServiceBuilder::new(protocol::SERVICE)
        .build()
        .context("failed to bind the crownos-preview socket")?;
    blocking::serve(&mut server, |server, peer, message| {
        protocol::dispatch(&mut previewer, server, peer, message).map(|()| true)
    })
    .context("crownos-preview service stopped")
}
