use std::path::Path;

use crownui::kit::{icon_button, use_theme};
use crownui::prelude::*;

pub const TOOLBAR_HEIGHT: f32 = 44.0;

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The bar above every document: its name on the left, its controls on the right.
pub fn toolbar(title: String, controls: impl ViewSeq) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        hstack((
            text(title)
                .font_semibold()
                .truncate()
                .shrink(1.0)
                .text_color(bind(move |rt| theme.with(rt, |theme| theme.text.primary))),
            spacer(),
            hstack(controls).gap(4.0).items_center(),
        ))
        .h(TOOLBAR_HEIGHT)
        .px(12.0)
        .gap(12.0)
        .items_center()
        .bg(bind(move |rt| theme.with(rt, |theme| theme.surface.raised)))
    })
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
