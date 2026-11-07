use std::path::Path;

use crownui::kit::{WindowRadius, icon_button, title_bar_spacer, use_theme, window_controls};
use crownui::prelude::*;

/// Between the window's edge, the title bar, the side column and the card.
pub const FRAME_PADDING: f32 = 6.0;
const TITLE_BAR_GAP: f32 = 8.0;
const TITLE_INSET: f32 = 8.0;
const MIN_DRAG_WIDTH: f32 = 48.0;
const FALLBACK_CARD_RADIUS: f32 = 16.0;
const CARD_BORDER_WIDTH: f32 = 1.0;

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The client-drawn window: `title_bar` above `body`, all on the title bar's color.
pub fn window_frame(title_bar: impl View, body: impl ViewSeq + 'static) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        vstack((title_bar, body))
            .grow(1.0)
            .min_h(0.0)
            .gap(FRAME_PADDING)
            .p(FRAME_PADDING)
            .bg(bind(move |rt| theme.with(rt, |theme| theme.title_bar.bg)))
    })
}

/// `leading` and `title` on the left, `center` in the middle of the window
/// and `trailing` before the window controls, with drag areas between.
pub fn title_bar(
    leading: impl ViewSeq + 'static,
    title: impl IntoProp<String> + 'static,
    center: impl ViewSeq + 'static,
    trailing: impl ViewSeq + 'static,
) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        let start = hstack((
            leading,
            text(title)
                .font_semibold()
                .truncate()
                .shrink(1.0)
                .pl(TITLE_INSET)
                .text_color(bind(move |rt| theme.with(rt, |theme| theme.text.primary))),
            title_bar_spacer().min_w(MIN_DRAG_WIDTH),
        ));
        let end = hstack((
            title_bar_spacer().min_w(MIN_DRAG_WIDTH),
            hstack(trailing).gap(4.0).items_center(),
            window_controls(),
        ));
        hstack((
            start.flex_1().min_w(0.0).items_center(),
            hstack(center).items_center(),
            end.flex_1().min_w(0.0).gap(TITLE_BAR_GAP).items_center(),
        ))
        .gap(TITLE_BAR_GAP)
        .items_center()
        .shrink(0.0)
    })
}

/// The rounded surface a document is shown on, clipped to its corners and
/// concentric with the window.
pub fn content_card(content: impl ViewSeq + 'static) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        let window = cx.environment::<WindowRadius>();
        vstack(content)
            .grow(1.0)
            .min_w(0.0)
            .min_h(0.0)
            .rounded(bind(move |rt| {
                window.map_or(FALLBACK_CARD_RADIUS, |window| {
                    window.with(rt, |window| window.concentric(FRAME_PADDING))
                })
            }))
            .overflow_hidden()
            .border(CARD_BORDER_WIDTH)
            .border_color(bind(move |rt| theme.with(rt, |theme| theme.surface.border)))
            .bg(bind(move |rt| theme.with(rt, |theme| theme.surface.bg)))
    })
}

/// A frame whose card shows only `message`.
pub fn notice_window(title: impl IntoProp<String> + 'static, message: &'static str) -> impl View {
    window_frame(title_bar((), title, (), ()), content_card(notice(message)))
}

pub fn tool(
    body: &'static str,
    label: &'static str,
    action: impl FnMut(&mut Cx) + 'static,
) -> impl View {
    let mut action = action;
    icon_button(body, label).on_click(move |cx, _| action(cx))
}

/// A centered note in place of a document that cannot be shown.
pub fn notice(message: impl IntoProp<String> + 'static) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        vstack((text(message)
            .text_center()
            .text_color(bind(move |rt| theme.with(rt, |theme| theme.text.muted))),))
        .grow(1.0)
        .items_center()
        .justify_center()
        .p(24.0)
    })
}
