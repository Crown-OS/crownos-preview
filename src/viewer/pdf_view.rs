use std::path::PathBuf;

use crownui::ext::{DrawList, Key, KeyboardEvent, NamedKey};
use crownui::kit::{FieldShape, TextInputOptions, icons, text_input, use_theme};
use crownui::prelude::*;

use super::chrome::{
    FRAME_PADDING, content_card, file_name, notice, title_bar, tool, window_frame,
};
use super::control_pill::{floating_controls, pill_button, pill_label};
use super::pdf_sidebar::pdf_sidebar;
use super::pdf_state::{PAGE_GAP, PdfState, page_rects};
use super::shortcuts::on_shortcut;
use super::zoom::Zoom;
use crate::pdf::{self, PdfRequest};
use crate::util::mailbox::mailbox;
use crate::util::measured_canvas::measured_canvas;
use crate::util::pinch::PinchTracker;
use crate::util::rect::spanned;

const LINE_HEIGHT: f32 = 40.0;
const SEARCH_FIELD_WIDTH: f32 = 280.0;
const PAGE_LABEL_WIDTH: f32 = 88.0;
const PAGE_SHADOW: Color = rgba(0x00000040);
const MATCH_FILL: Color = rgba(0xffcc0066);
const CURRENT_MATCH_FILL: Color = rgba(0xff9500aa);
const SELECTION_FILL: Color = rgba(0x007aff33);

pub fn pdf_view(path: PathBuf) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        let state = PdfState::new(cx);
        let sidebar_open = cx.signal(true);
        let query = cx.signal(String::new());
        let searched = cx.store(String::new());
        let device_scale = cx.scale_factor();

        let events = mailbox(cx, move |cx, event| state.receive(cx, event));
        let requests = pdf::open(path.clone(), events);
        state.connect(cx, requests);

        let search = move |cx: &mut Cx| {
            let text = query.get(cx);
            if searched.with(cx, |previous| *previous == text) {
                state.step_match(cx, true);
                return;
            }
            searched.update(cx, |previous| previous.clone_from(&text));
            state.status.set(cx, "Searching…".into());
            state.request(cx, PdfRequest::Search(text));
        };

        on_shortcut(cx, move |cx, event| page_shortcut(cx, state, event));

        let status = text(bind(move |rt| state.status.get(rt)))
            .text_size(12.0)
            .text_color(bind(move |rt| theme.with(rt, |theme| theme.text.muted)));
        let search_field = text_input(query)
            .placeholder("Search")
            .leading_icon(icons::SEARCH)
            .shape(FieldShape::Pill)
            .width(SEARCH_FIELD_WIDTH)
            .on_submit(move |cx, _| search(cx));
        let sidebar_toggle = tool(icons::PANEL_LEFT, "Sidebar", move |cx| {
            sidebar_open.update(cx, |open| *open = !*open)
        });

        window_frame(
            title_bar(sidebar_toggle, file_name(&path), search_field, status),
            show(
                bind(move |rt| state.failure.with(rt, Option::is_some)),
                move || {
                    content_card(notice(bind(move |rt| {
                        state.failure.get(rt).unwrap_or_default()
                    })))
                },
                move || {
                    hstack((
                        show(
                            bind(move |rt| sidebar_open.get(rt)),
                            move || pdf_sidebar(state),
                            || hstack(()),
                        ),
                        content_card((
                            pages_canvas(state, device_scale),
                            floating_controls(page_controls(state)),
                        )),
                    ))
                    .gap(FRAME_PADDING)
                    .grow(1.0)
                    .min_h(0.0)
                },
            )
            .grow(1.0)
            .min_h(0.0),
        )
    })
}

fn page_controls(state: PdfState) -> Vec<AnyView> {
    vec![
        pill_button(icons::CHEVRON_UP, "Previous match", move |cx| {
            state.step_match(cx, false)
        }),
        pill_button(icons::CHEVRON_DOWN, "Next match", move |cx| {
            state.step_match(cx, true)
        }),
        pill_label(
            bind(move |rt| {
                let count = state.pages.with(rt, Vec::len);
                if count == 0 {
                    String::new()
                } else {
                    format!("{} / {count}", state.current_page(rt) + 1)
                }
            }),
            PAGE_LABEL_WIDTH,
        ),
        pill_button(icons::ZOOM_OUT, "Zoom out", move |cx| {
            state.change_zoom(cx, Zoom::zoomed_out)
        }),
        pill_button(icons::ZOOM_IN, "Zoom in", move |cx| {
            state.change_zoom(cx, Zoom::zoomed_in)
        }),
        pill_button(icons::MOVE_HORIZONTAL, "Fit width", move |cx| {
            state.set_zoom(cx, Zoom::Fit)
        }),
        pill_button(icons::SCAN, "Actual size", move |cx| {
            state.set_zoom(cx, Zoom::Scale(1.0))
        }),
    ]
}

fn pages_canvas(state: PdfState, device_scale: Signal<f32>) -> impl View {
    component(move |cx| pages_canvas_in(cx, state, device_scale))
}

fn pages_canvas_in(cx: &mut Cx, state: PdfState, device_scale: Signal<f32>) -> impl View + use<> {
    let anchor = cx.store(Point::ZERO);
    let pinch = cx.store(PinchTracker::default());
    let to_document = move |rt: &mut Runtime, point: Point| {
        let scroll = state.scroll.get(rt);
        Point::new(point.x + scroll.x, point.y + scroll.y)
    };
    measured_canvas(state.viewport, move |rt, size, list| {
        paint_pages(rt, state, device_scale, size, list);
    })
    .grow(1.0)
    .overflow_hidden()
    .bg(rgb(0x3a3a3c))
    .dark(|style| style.bg(rgb(0x1c1c1e)))
    .on_wheel(move |cx, event| {
        let detail = event.detail();
        let delta = detail.delta.to_pixels(LINE_HEIGHT);
        if detail.modifiers.ctrl_key {
            let factor = 1.1_f32.powf(-delta.y / LINE_HEIGHT);
            state.change_zoom(cx, |zoom, fit| zoom.multiplied(factor, fit));
        } else {
            state.scroll_by(cx, delta.x, delta.y);
        }
        event.prevent_default();
    })
    .on_pinch(move |cx, event| {
        let factor = pinch.update(cx, |pinch| pinch.step(event.detail().scale));
        state.change_zoom(cx, |zoom, fit| zoom.multiplied(factor, fit));
        event.prevent_default();
    })
    .on_drag_start(move |cx, event| {
        let at = to_document(cx, event.detail().position);
        anchor.update(cx, |anchor| *anchor = at);
        state.selection.set(cx, Some(spanned(at, at)));
    })
    .on_drag(move |cx, event| {
        let at = to_document(cx, event.detail().position);
        let start = anchor.with(cx, |anchor| *anchor);
        state.selection.set(cx, Some(spanned(start, at)));
    })
    .on_drag_end(move |cx, _| {
        let Some(selection) = state.selection.get(cx) else {
            return;
        };
        let Some((page, start)) = state.page_point(cx, selection.origin) else {
            return;
        };
        let scale = state.scale(cx);
        let area = Rect::new(
            start,
            Size::new(selection.size.width / scale, selection.size.height / scale),
        );
        state.request(cx, PdfRequest::Select { page, area });
    })
    .on_click(move |cx, _| state.selection.set(cx, None))
}

fn paint_pages(
    rt: &mut Runtime,
    state: PdfState,
    device_scale: Signal<f32>,
    size: Size,
    list: &mut DrawList,
) {
    let scale = state.scale(rt);
    let scroll = state.scroll.get(rt);
    let rects = state
        .pages
        .with(rt, |pages| page_rects(pages, scale, size.width));
    let in_view =
        |rect: &Rect| rect.max_y() + PAGE_GAP >= scroll.y && rect.min_y() <= scroll.y + size.height;
    let first = rects.iter().position(in_view).unwrap_or(0);
    let last = rects
        .iter()
        .rposition(in_view)
        .map_or(first, |last| last + 1);
    let device_pixels = scale * device_scale.get(rt);
    state.request_missing(rt, first..last, device_pixels);

    let on_screen = |rect: Rect| {
        Rect::new(
            Point::new(rect.min_x() - scroll.x, rect.min_y() - scroll.y),
            rect.size,
        )
    };
    let textures = state.textures.get(rt);
    for (page, rect) in rects.iter().enumerate().take(last).skip(first) {
        let rect = on_screen(*rect);
        list.shadow(
            rect,
            CornerRadii::default(),
            PAGE_SHADOW,
            8.0,
            Vector::new(0.0, 2.0),
        );
        list.fill(rect, CornerRadii::default(), WHITE);
        if let Some(texture) = textures.get(&page) {
            list.image(texture.image, rect);
        }
    }

    let current = state.current_match.get(rt);
    state.matches.with(rt, |matches| {
        for (index, found) in matches
            .iter()
            .enumerate()
            .filter(|(_, found)| (first..last).contains(&found.page))
        {
            let page = on_screen(rects[found.page]);
            let area = Rect::new(
                Point::new(
                    page.min_x() + found.area.min_x() * scale,
                    page.min_y() + found.area.min_y() * scale,
                ),
                Size::new(
                    found.area.size.width * scale,
                    found.area.size.height * scale,
                ),
            );
            let fill = if index == current {
                CURRENT_MATCH_FILL
            } else {
                MATCH_FILL
            };
            list.fill(area, CornerRadii::default(), fill);
        }
    });
    if let Some(selection) = state.selection.get(rt) {
        list.fill(on_screen(selection), CornerRadii::default(), SELECTION_FILL);
    }
}

fn page_shortcut(cx: &mut Cx, state: PdfState, event: &KeyboardEvent) -> bool {
    let page_height = state.viewport.get(cx).height - LINE_HEIGHT;
    match &event.key {
        Key::Named(NamedKey::ArrowDown) => state.scroll_by(cx, 0.0, LINE_HEIGHT),
        Key::Named(NamedKey::ArrowUp) => state.scroll_by(cx, 0.0, -LINE_HEIGHT),
        Key::Named(NamedKey::PageDown | NamedKey::Space) => state.scroll_by(cx, 0.0, page_height),
        Key::Named(NamedKey::PageUp) => state.scroll_by(cx, 0.0, -page_height),
        Key::Named(NamedKey::Home) => state.go_to_page(cx, 0),
        Key::Named(NamedKey::End) => {
            let height = state.document_height(cx);
            state.scroll_by(cx, 0.0, height);
        }
        Key::Character('+' | '=') => state.change_zoom(cx, Zoom::zoomed_in),
        Key::Character('-') => state.change_zoom(cx, Zoom::zoomed_out),
        Key::Character('0') => state.set_zoom(cx, Zoom::Fit),
        _ => return false,
    }
    true
}
