use std::time::Duration;

use crownui::ext::{ImageId, Key, NamedKey};
use crownui::kit::{ButtonOptions, ButtonVariant, button, icons, use_theme};
use crownui::prelude::*;

use super::chrome::{file_name, notice, tool, toolbar};
use super::shortcuts::on_shortcut;
use super::stage::Stage;
use crate::document::DocumentKind;
use crate::launch::{Mode, Request};
use crate::markup::{self, Markup, markup_toolbar};
use crate::picture::{self, Picture};
use crate::util::background::in_background;
use crate::util::fonts::system_fonts;
use crate::util::measured_canvas::measured_canvas;
use crate::util::timer::after;

#[derive(Clone, Copy, PartialEq)]
struct Shown {
    image: ImageId,
    delay: Duration,
}

pub fn image_view(
    request: Request,
    kind: DocumentKind,
    reload: impl Fn(&mut Cx, Request) + Copy + 'static,
) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        let stage = Stage::new(cx);
        let frames = cx.signal(Vec::<Shown>::new());
        let frame_index = cx.signal(0_usize);
        let failure = cx.signal(None::<String>);
        let status = cx.signal(String::new());
        let editable = kind.is_editable();
        let editing = cx.signal(editable && request.mode == Mode::Edit);
        let markup = Markup::new(cx);
        let path = request.path;

        let source = path.clone();
        in_background(
            cx,
            move || picture::decode(&source, kind),
            move |cx, decoded| match decoded {
                Ok(picture) => show_picture(cx, picture, stage, frames, frame_index),
                Err(error) => failure.set(cx, Some(format!("Cannot open this image: {error}"))),
            },
        );
        if editable {
            std::thread::spawn(system_fonts);
        }

        let cancel = move |cx: &mut Cx| {
            markup.discard(cx);
            editing.set(cx, false);
        };
        let saved_path = cx.store(path.clone());
        let done = move |cx: &mut Cx| {
            if markup.is_pristine(cx) {
                editing.set(cx, false);
                return;
            }
            status.set(cx, "Saving…".into());
            let (annotations, crop) = (markup.annotations.get(cx), markup.crop.get(cx));
            let target = saved_path.with(cx, Clone::clone);
            in_background(
                cx,
                move || markup::save(&target, kind, &annotations, crop),
                move |cx, saved| match saved {
                    Ok(written) => {
                        markup.discard(cx);
                        reload(
                            cx,
                            Request {
                                path: written,
                                mode: Mode::View,
                            },
                        );
                    }
                    Err(error) => status.set(cx, format!("Not saved: {error}")),
                },
            );
        };

        on_shortcut(cx, move |cx, event| {
            let ctrl = event.modifiers.ctrl_key;
            match &event.key {
                Key::Character('z') if ctrl && editing.get(cx) => markup.undo(cx),
                Key::Character('+' | '=') => stage.zoom_in(cx),
                Key::Character('-') => stage.zoom_out(cx),
                Key::Character('0') => stage.reset(cx),
                Key::Named(NamedKey::Escape) if editing.get(cx) => cancel(cx),
                Key::Named(NamedKey::Enter) if editing.get(cx) => done(cx),
                Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowRight) if editing.get(cx) => {}
                _ => return false,
            }
            true
        });

        let controls = (
            text(bind(move |rt| status.get(rt)))
                .text_size(12.0)
                .text_color(bind(move |rt| theme.with(rt, |theme| theme.text.muted))),
            tool(icons::ZOOM_OUT, "Zoom out", move |cx| stage.zoom_out(cx)),
            text(bind(move |rt| stage.percent(rt)))
                .text_size(12.0)
                .min_w(44.0)
                .text_center()
                .text_color(bind(move |rt| theme.with(rt, |theme| theme.text.body))),
            tool(icons::ZOOM_IN, "Zoom in", move |cx| stage.zoom_in(cx)),
            tool(icons::SCAN, "Zoom to fit", move |cx| stage.reset(cx)),
            show(
                bind(move |rt| editable && !editing.get(rt)),
                move || {
                    button("Markup")
                        .variant(ButtonVariant::Secondary)
                        .leading_icon(icons::PEN_LINE)
                        .on_click(move |cx, _| editing.set(cx, true))
                },
                || hstack(()),
            ),
        );

        vstack((
            toolbar(file_name(&path), controls),
            show(
                bind(move |rt| editing.get(rt)),
                move || markup_toolbar(markup, cancel, done),
                || hstack(()),
            ),
            show(
                bind(move |rt| failure.with(rt, Option::is_some)),
                move || notice(bind(move |rt| failure.get(rt).unwrap_or_default())),
                move || picture_canvas(stage, frames, frame_index, editing, markup),
            )
            .grow(1.0),
        ))
        .grow(1.0)
    })
}

fn show_picture(
    cx: &mut Cx,
    picture: Picture,
    stage: Stage,
    frames: Signal<Vec<Shown>>,
    frame_index: Signal<usize>,
) {
    let mut natural = Size::new(0.0, 0.0);
    let shown: Vec<Shown> = picture
        .frames
        .into_iter()
        .map(|frame| {
            natural = Size::new(frame.width as f32, frame.height as f32);
            let image = cx.register_image(frame.width, frame.height, frame.rgba);
            cx.release_image_on_cleanup(image);
            Shown {
                image,
                delay: frame.delay,
            }
        })
        .collect();
    let animated = shown.len() > 1;
    stage.content.set(cx, natural);
    frames.set(cx, shown);
    if animated {
        advance_after(cx, frames, frame_index);
    }
}

fn advance_after(cx: &mut Cx, frames: Signal<Vec<Shown>>, frame_index: Signal<usize>) {
    let index = frame_index.get(cx);
    let Some(delay) = frames.with(cx, |frames| frames.get(index).map(|frame| frame.delay)) else {
        return;
    };
    after(cx, delay, move |cx| {
        let count = frames.with(cx, Vec::len);
        frame_index.update(cx, |index| *index = (*index + 1) % count.max(1));
        advance_after(cx, frames, frame_index);
    });
}

fn picture_canvas(
    stage: Stage,
    frames: Signal<Vec<Shown>>,
    frame_index: Signal<usize>,
    editing: Signal<bool>,
    markup: Markup,
) -> impl View {
    let image_point = move |cx: &mut Cx, position: Point| stage.to_content(cx, position);
    measured_canvas(stage.viewport, move |rt, _, list| {
        let index = frame_index.get(rt);
        let Some(shown) = frames.with(rt, |frames| frames.get(index).copied()) else {
            return;
        };
        let (rect, scale) = stage.placement(rt);
        list.image(shown.image, rect);
        let annotations = markup.annotations.get(rt);
        let draft = markup.draft.get(rt);
        let crop = markup.crop.get(rt).filter(|_| editing.get(rt));
        markup::paint(list, &annotations, draft.as_ref(), crop, rect, scale);
    })
    .grow(1.0)
    .overflow_hidden()
    .on_drag_start(move |cx, event| {
        if editing.get(cx) {
            let at = image_point(cx, event.detail().position);
            let content = stage.content.get(cx);
            markup.begin(cx, at, content);
        }
    })
    .on_drag(move |cx, event| {
        if editing.get(cx) {
            let at = image_point(cx, event.detail().position);
            let content = stage.content.get(cx);
            markup.extend(cx, at, content);
        } else {
            stage.pan_by(cx, event.detail().delta);
        }
    })
    .on_drag_end(move |cx, _| {
        if editing.get(cx) {
            markup.finish(cx);
        }
    })
    .on_click(move |cx, event| {
        if editing.get(cx) {
            let at = image_point(cx, event.detail().position);
            let content = stage.content.get(cx);
            markup.place_text(cx, at, content);
        }
    })
    .on_double_click(move |cx, _| {
        if !editing.get(cx) {
            match stage.zoom.get(cx) {
                super::zoom::Zoom::Fit => stage.actual_size(cx),
                super::zoom::Zoom::Scale(_) => stage.reset(cx),
            }
        }
    })
    .on_wheel(move |cx, event| {
        stage.wheel(cx, event.detail());
        event.prevent_default();
    })
}
