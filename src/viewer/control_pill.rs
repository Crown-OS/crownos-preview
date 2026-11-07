use crownui::kit::{self, use_theme};
use crownui::prelude::*;

const BUTTON_SIZE: f32 = 40.0;
const ICON_SIZE: f32 = 20.0;
const PILL_PADDING: f32 = 4.0;
const SEPARATOR_HEIGHT: f32 = 20.0;
const LABEL_SIZE: f32 = 14.0;
const LABEL_PADDING: f32 = 12.0;
const BOTTOM_OFFSET: f32 = 16.0;
const PILL_SHADOW: Color = rgba(0x00000040);
const PILL_SHADOW_BLUR: f32 = 16.0;

/// `items` in a fully rounded bar floating over the bottom center of its
/// parent, a line between each.
pub fn floating_controls(items: Vec<AnyView>) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        let mut separated = Vec::with_capacity(items.len() * 2);
        for item in items {
            if !separated.is_empty() {
                separated.push(separator().boxed());
            }
            separated.push(item);
        }
        let pill = hstack(separated)
            .items_center()
            .p(PILL_PADDING)
            .rounded(BUTTON_SIZE / 2.0 + PILL_PADDING)
            .border(1.0)
            .border_color(bind(move |rt| theme.with(rt, |theme| theme.surface.border)))
            .bg(bind(move |rt| theme.with(rt, |theme| theme.surface.raised)))
            .shadow(Shadow::new(
                PILL_SHADOW,
                PILL_SHADOW_BLUR,
                Vector::new(0.0, 4.0),
            ))
            .pointer_events_auto();
        hstack((pill,))
            .absolute()
            .left(0.0)
            .right(0.0)
            .bottom(BOTTOM_OFFSET)
            .justify_center()
            .pointer_events_none()
    })
}

pub fn pill_button(
    body: &'static str,
    label: &'static str,
    mut action: impl FnMut(&mut Cx) + 'static,
) -> AnyView {
    component(move |cx| {
        let theme = use_theme(cx);
        vstack((kit::icon(body).size(ICON_SIZE),))
            .size(BUTTON_SIZE)
            .rounded(BUTTON_SIZE / 2.0)
            .items_center()
            .justify_center()
            .cursor(CursorIcon::Pointer)
            .hover(move |style| {
                style.bg(bind(move |rt| theme.with(rt, |theme| theme.surface.hover)))
            })
            .role(Role::Button)
            .label(label)
            .on_click(move |cx, _| action(cx))
    })
    .boxed()
}

pub fn pill_label(content: impl IntoProp<String> + 'static, min_width: f32) -> AnyView {
    component(move |cx| {
        let theme = use_theme(cx);
        text(content)
            .text_size(LABEL_SIZE)
            .min_w(min_width)
            .px(LABEL_PADDING)
            .text_center()
            .text_color(bind(move |rt| theme.with(rt, |theme| theme.text.body)))
    })
    .boxed()
}

fn separator() -> impl View {
    component(|cx| {
        let theme = use_theme(cx);
        vstack(())
            .w(1.0)
            .h(SEPARATOR_HEIGHT)
            .shrink(0.0)
            .bg(bind(move |rt| theme.with(rt, |theme| theme.surface.border)))
    })
}
