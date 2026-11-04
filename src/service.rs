use std::time::Duration;

use anyhow::{Context, Result};
use crownos_ipc::adapter::tokio::serve_with_outbox;
use crownos_ipc::{RemoteError, Server, ServiceBuilder};
use crownos_preview::protocol::{self, Handler};
use crownos_preview::{Client, Closed, Navigated};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::launch::{Mode, Request};
use crate::playlist::Playlist;

const REPLY_TIMEOUT: Option<Duration> = Some(Duration::from_secs(2));

pub enum Command {
    Show(Request),
    ShowList(Playlist),
    Select(usize),
    Close,
}

/// What the viewer tells its clients without being asked.
pub enum Outgoing {
    Navigated {
        index: usize,
        path: String,
    },
    /// Announced, then the process exits.
    Closed {
        path: String,
    },
}

struct Forwarder(UnboundedSender<Command>);

impl Forwarder {
    fn show(&self, path: String, mode: Mode) -> bool {
        Request::new(path, mode).is_some_and(|request| self.0.send(Command::Show(request)).is_ok())
    }
}

impl Handler for Forwarder {
    fn open(&mut self, path: String) -> Result<bool, RemoteError> {
        Ok(self.show(path, Mode::View))
    }

    fn edit(&mut self, path: String) -> Result<bool, RemoteError> {
        Ok(self.show(path, Mode::Edit))
    }

    fn close(&mut self) {
        let _ = self.0.send(Command::Close);
    }

    fn open_list(&mut self, paths: Vec<String>, index: u32) -> Result<bool, RemoteError> {
        let shown = Playlist::new(paths, index)
            .is_some_and(|playlist| self.0.send(Command::ShowList(playlist)).is_ok());
        Ok(shown)
    }

    fn select(&mut self, index: u32) {
        if let Ok(index) = usize::try_from(index) {
            let _ = self.0.send(Command::Select(index));
        }
    }
}

pub fn bind() -> Result<Server> {
    ServiceBuilder::new(protocol::SERVICE)
        .build()
        .context("failed to bind the crownos-preview socket")
}

/// Hands `request` to an already running previewer; `false` when none runs.
/// With no request there is nothing to hand over, so a running previewer is
/// left as it is.
pub fn forward_to_running(request: Option<&Request>) -> Result<bool> {
    let Ok(mut client) = Client::connect() else {
        return Ok(false);
    };
    let Some(request) = request else {
        return Ok(true);
    };
    let path = request
        .path
        .to_str()
        .context("the path is not valid UTF-8")?
        .to_owned();
    let pending = match request.mode {
        Mode::View => client.open(path)?,
        Mode::Edit => client.edit(path)?,
    };
    client.wait(pending, REPLY_TIMEOUT)?;
    Ok(true)
}

pub async fn serve_commands(
    mut server: Server,
    commands: UnboundedSender<Command>,
    mut outgoing: UnboundedReceiver<Outgoing>,
) {
    let mut forwarder = Forwarder(commands);
    let served = serve_with_outbox(
        &mut server,
        |server, peer, message| {
            protocol::dispatch(&mut forwarder, server, peer, message).map(|()| true)
        },
        |cx| outgoing.poll_recv(cx),
        announce,
    )
    .await;
    if let Err(error) = served {
        eprintln!("crownos-preview: service stopped: {error}");
    }
}

fn announce(server: &mut Server, outgoing: Outgoing) {
    let emitted = match outgoing {
        Outgoing::Navigated { index, path } => {
            u32::try_from(index).map_or(Ok(()), |index| server.emit(&Navigated { index, path }))
        }
        Outgoing::Closed { path } => {
            if let Err(error) = server.emit(&Closed { path }) {
                eprintln!("crownos-preview: failed to announce closing: {error}");
            }
            std::process::exit(0)
        }
    };
    if let Err(error) = emitted {
        eprintln!("crownos-preview: failed to announce navigation: {error}");
    }
}
