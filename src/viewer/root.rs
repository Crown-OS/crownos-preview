use crownos_ipc::Server;
use crownui::ext::{Key, NamedKey};
use crownui::kit::{provide_configured_theme, use_theme};
use crownui::prelude::*;

use super::shortcuts::{Shortcuts, dispatch_shortcut, provide_shortcuts};
use super::{image_view, media_view, pdf_view, text_view};
use crate::document::DocumentKind;
use crate::launch::{Mode, Request};
use crate::playlist::Playlist;
use crate::service::{self, Command, Outgoing};
use crate::util::mailbox::mailbox;
use crate::util::siblings::{Direction, sibling};

/// Bumped to rebuild the shown document, e.g. after an edit was saved.
type Revision = u64;

pub fn root(initial: Request, server: Server) -> impl View {
    component(move |cx| {
        provide_configured_theme(cx);
        let theme = use_theme(cx);
        let shortcuts = provide_shortcuts(cx);
        let current = cx.signal(initial);
        let playlist = cx.store(None::<Playlist>);
        let revision = cx.signal(Revision::default());
        let (outgoing, outbox) = tokio::sync::mpsc::unbounded_channel();
        let dismiss = {
            let outgoing = outgoing.clone();
            move |cx: &mut Cx| {
                let path = current.with(cx, |request| request.path.display().to_string());
                if outgoing.send(Outgoing::Closed { path }).is_err() {
                    std::process::exit(0);
                }
            }
        };
        let dismiss_on_escape = dismiss.clone();
        let commands = mailbox(cx, move |cx, command| match command {
            Command::Show(request) => {
                playlist.update(cx, |list| *list = None);
                current.set(cx, request);
            }
            Command::ShowList(list) => {
                if let Some(request) = list.request() {
                    playlist.update(cx, |listed| *listed = Some(list));
                    current.set(cx, request);
                }
            }
            Command::Select(index) => {
                let selected = playlist.update(cx, |list| list.as_mut()?.select(index));
                if let Some(request) = selected {
                    current.set(cx, request);
                }
            }
            Command::Close => dismiss(cx),
        });
        cx.spawn(
            service::serve_commands(server, commands, outbox),
            |_, ()| {},
        );

        let reload = move |cx: &mut Cx, request: Request| {
            current.set(cx, request);
            revision.update(cx, |revision| *revision += 1);
        };

        keyed(
            move |rt| (current.get(rt), revision.get(rt)),
            move |cx, (request, _)| {
                shortcuts.set(cx, Shortcuts::default());
                document_view(request, reload)
            },
        )
        .grow(1.0)
        .bg(bind(move |rt| theme.with(rt, |theme| theme.surface.bg)))
        .on_key_down(move |cx, event| {
            if event.is_default_prevented() || dispatch_shortcut(cx, shortcuts, event.detail()) {
                return;
            }
            let direction = match &event.detail().key {
                Key::Named(NamedKey::ArrowLeft) => Direction::Previous,
                Key::Named(NamedKey::ArrowRight) => Direction::Next,
                Key::Named(NamedKey::Escape) => return dismiss_on_escape(cx),
                _ => return,
            };
            let stepped = playlist.update(cx, |list| {
                let list = list.as_mut()?;
                let request = list.step(direction)?;
                Some((list.index(), request))
            });
            if let Some((index, request)) = stepped {
                let path = request.path.display().to_string();
                let _ = outgoing.send(Outgoing::Navigated { index, path });
                current.set(cx, request);
                return;
            }
            let listed = playlist.with(cx, Option::is_some);
            let neighbour = (!listed)
                .then(|| current.with(cx, |request| sibling(&request.path, direction)))
                .flatten();
            if let Some(request) = neighbour.and_then(|path| Request::new(path, Mode::View)) {
                current.set(cx, request);
            }
        })
    })
}

fn document_view(request: Request, reload: impl Fn(&mut Cx, Request) + Copy + 'static) -> AnyView {
    let Some(kind) = DocumentKind::of(&request.path) else {
        return super::chrome::notice("This file cannot be previewed.").boxed();
    };
    match kind {
        DocumentKind::Raster | DocumentKind::Vector | DocumentKind::Heif => {
            image_view::image_view(request, kind, reload).boxed()
        }
        DocumentKind::Pdf => pdf_view::pdf_view(request.path).boxed(),
        DocumentKind::Video | DocumentKind::Audio => {
            media_view::media_view(request.path, kind).boxed()
        }
        DocumentKind::Markdown | DocumentKind::Code | DocumentKind::PlainText => {
            text_view::text_view(request.path, kind).boxed()
        }
    }
}
