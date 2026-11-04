use crownui::kit::{
    ButtonOptions, ButtonVariant, TextInputOptions, ToggleItem, button, icon_button, icons,
    text_input, toggle_group, use_theme,
};
use crownui::prelude::*;

use super::state::{Markup, PALETTE};
use super::tool::Tool;

const SWATCH_SIZE: f32 = 20.0;
const LABEL_FIELD_WIDTH: f32 = 180.0;

/// The markup tools, colors, undo and the cancel / done pair.
pub fn markup_toolbar(
    markup: Markup,
    mut on_cancel: impl FnMut(&mut Cx) + 'static,
    mut on_done: impl FnMut(&mut Cx) + 'static,
) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        let selected = cx.signal(0_usize);
        cx.effect(move |rt| {
            let tool = Tool::ALL[selected.get(rt)];
            markup.tool.set(rt, tool);
        });
        let swatches: Vec<_> = PALETTE
            .iter()
            .map(|color| swatch(markup.color, *color))
            .collect();
        hstack((
            toggle_group(
                Tool::ALL.map(|tool| ToggleItem::icon(tool.icon(), tool.label())),
                selected,
            ),
            hstack(swatches).gap(6.0).items_center(),
            show(
                bind(move |rt| markup.tool.get(rt) == Tool::Text),
                move || {
                    text_input(markup.label)
                        .placeholder("Type, then click the image")
                        .width(LABEL_FIELD_WIDTH)
                },
                || hstack(()),
            ),
            spacer(),
            icon_button(icons::UNDO_2, "Undo").on_click(move |cx, _| markup.undo(cx)),
            button("Cancel")
                .variant(ButtonVariant::Secondary)
                .on_click(move |cx, _| on_cancel(cx)),
            button("Done")
                .variant(ButtonVariant::Primary)
                .on_click(move |cx, _| on_done(cx)),
        ))
        .px(12.0)
        .py(6.0)
        .gap(12.0)
        .items_center()
        .flex_wrap()
        .bg(bind(move |rt| theme.with(rt, |theme| theme.surface.sunken)))
    })
}

fn swatch(chosen: Signal<Color>, color: Color) -> impl View {
    vstack(())
        .size(SWATCH_SIZE)
        .rounded(SWATCH_SIZE / 2.0)
        .bg(color)
        .border(bind(
            move |rt| if chosen.get(rt) == color { 3.0 } else { 1.0 },
        ))
        .border_color(WHITE)
        .cursor(CursorIcon::Pointer)
        .on_click(move |cx, _| chosen.set(cx, color))
}
