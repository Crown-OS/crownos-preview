use crownos_ipc::Server;
use crownui::ext::{Key, NamedKey};
use crownui::kit::provide_configured_theme;
use crownui::prelude::*;

use super::chrome::{file_name, notice_window};
use super::shortcuts::{Shortcuts, dispatch_shortcut, provide_shortcuts};
use super::{image_view, media_view, pdf_view, text_view};
use crate::document::DocumentKind;
use crate::launch::{Mode, Request};
use crate::playlist::Playlist;
use crate::service::{self, Command, Outgoing};
use crate::util::file_uri::{URI_LIST_MIME, file_paths};
use crate::util::mailbox::mailbox;
use crate::util::siblings::{Direction, sibling};
use crate::util::window_radius::provide_configured_window_radius;

const APP_NAME: &str = "Preview";
const UNPREVIEWABLE: &str = "This file cannot be previewed.";
const EMPTY_STATE: &str = "Drop a picture, PDF, video, song or text file here to preview it.";

/// Bumped to rebuild the shown document, e.g. after an edit was saved.
type Revision = u64;

pub fn root(initial: Option<Request>, server: Server) -> impl View {
    component(move |cx| {
        provide_configured_theme(cx);
        provide_configured_window_radius(cx);
        let shortcuts = provide_shortcuts(cx);
        let current = cx.signal(initial);
        let playlist = cx.store(None::<Playlist>);
        let revision = cx.signal(Revision::default());
        let (outgoing, outbox) = tokio::sync::mpsc::unbounded_channel();
        let dismiss = {
            let outgoing = outgoing.clone();
            move |cx: &mut Cx| {
                let path = current.with(cx, |request| {
                    request
                        .as_ref()
                        .map(|request| request.path.display().to_string())
                        .unwrap_or_default()
                });
                if outgoing.send(Outgoing::Closed { path }).is_err() {
                    std::process::exit(0);
                }
            }
        };
        let dismiss_on_escape = dismiss.clone();
        let commands = mailbox(cx, move |cx, command| match command {
            Command::Show(request) => {
                playlist.update(cx, |list| *list = None);
                current.set(cx, Some(request));
            }
            Command::ShowList(list) => {
                if let Some(request) = list.request() {
                    playlist.update(cx, |listed| *listed = Some(list));
                    current.set(cx, Some(request));
                }
            }
            Command::Select(index) => {
                let selected = playlist.update(cx, |list| list.as_mut()?.select(index));
                if let Some(request) = selected {
                    current.set(cx, Some(request));
                }
            }
            Command::Close => dismiss(cx),
        });
        cx.spawn(
            service::serve_commands(server, commands, outbox),
            |_, ()| {},
        );

        let reload = move |cx: &mut Cx, request: Request| {
            current.set(cx, Some(request));
            revision.update(cx, |revision| *revision += 1);
        };

        keyed(
            move |rt| (current.get(rt), revision.get(rt)),
            move |cx, (request, _)| {
                shortcuts.set(cx, Shortcuts::default());
                match request {
                    Some(request) => document_view(request, reload),
                    None => notice_window(APP_NAME, EMPTY_STATE).boxed(),
                }
            },
        )
        .grow(1.0)
        .min_h(0.0)
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
                current.set(cx, Some(request));
                return;
            }
            let listed = playlist.with(cx, Option::is_some);
            let neighbour = (!listed)
                .then(|| current.with(cx, |request| sibling(&request.as_ref()?.path, direction)))
                .flatten();
            if let Some(request) = neighbour.and_then(|path| Request::new(path, Mode::View)) {
                current.set(cx, Some(request));
            }
        })
        .on_drag_over(|_, event| {
            if event.offers(URI_LIST_MIME) {
                event.accept(URI_LIST_MIME);
            }
        })
        .on_drop(move |cx, event| {
            let dropped = std::str::from_utf8(&event.data).ok().and_then(|uri_list| {
                file_paths(uri_list).find_map(|path| Request::new(path, Mode::View))
            });
            if let Some(request) = dropped {
                playlist.update(cx, |list| *list = None);
                current.set(cx, Some(request));
            }
        })
    })
}

fn document_view(request: Request, reload: impl Fn(&mut Cx, Request) + Copy + 'static) -> AnyView {
    let Some(kind) = DocumentKind::of(&request.path) else {
        return notice_window(file_name(&request.path), UNPREVIEWABLE).boxed();
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
