use crownui::ext::{Key, NamedKey};
use crownui::prelude::*;

use super::shortcuts::on_shortcut;

pub const ROW_HEIGHT: f32 = 20.0;
const MOUNTED_ROWS: usize = 120;
const PAGE_ROWS: f32 = 30.0;

/// A scrolling list of fixed-height rows that mounts only the rows from the
/// first visible one, so files with many thousands of lines stay cheap.
pub fn line_list<T: 'static>(
    lines: Vec<T>,
    row: impl Fn(usize, &T) -> AnyView + Copy + 'static,
) -> impl View {
    component(move |cx| {
        let count = lines.len();
        let lines = cx.store(lines);
        let scroll = cx.signal(0.0_f32);
        let first = cx.memo(move |rt| (scroll.get(rt) / ROW_HEIGHT) as usize);
        let max_scroll = count.saturating_sub(1) as f32 * ROW_HEIGHT;
        let scroll_by = move |cx: &mut Cx, delta: f32| {
            scroll.update(cx, |scroll| {
                *scroll = (*scroll + delta).clamp(0.0, max_scroll)
            });
        };
        on_shortcut(cx, move |cx, event| {
            let delta = match event.key {
                Key::Named(NamedKey::ArrowDown) => ROW_HEIGHT,
                Key::Named(NamedKey::ArrowUp) => -ROW_HEIGHT,
                Key::Named(NamedKey::PageDown | NamedKey::Space) => PAGE_ROWS * ROW_HEIGHT,
                Key::Named(NamedKey::PageUp) => -PAGE_ROWS * ROW_HEIGHT,
                Key::Named(NamedKey::Home) => -max_scroll,
                Key::Named(NamedKey::End) => max_scroll,
                _ => return false,
            };
            scroll_by(cx, delta);
            true
        });
        vstack((keyed(
            move |rt| first.get(rt),
            move |cx, first| {
                let end = (first + MOUNTED_ROWS).min(count);
                let rows: Vec<AnyView> = lines.with(cx, |lines| {
                    (first..end)
                        .map(|index| row(index, &lines[index]))
                        .collect()
                });
                vstack(rows)
            },
        ),))
        .grow(1.0)
        .overflow_hidden()
        .on_wheel(move |cx, event| {
            scroll_by(cx, event.detail().delta.to_pixels(ROW_HEIGHT).y);
            event.prevent_default();
        })
    })
}
