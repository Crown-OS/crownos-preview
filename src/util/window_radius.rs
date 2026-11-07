use crownconfig::{Appearance, SectionWatch, load_appearance, watch_section};
use crownui::kit::{WindowRadius, provide_window_radius};
use crownui::prelude::{Cx, Signal};

/// Provides the corner radius crownpositor rounds this window with, read from
/// `appearance.ron` and kept following edits to it.
pub fn provide_configured_window_radius(cx: &mut Cx) {
    let appearance = load_appearance();
    let radius = provide_window_radius(cx, window_radius(&appearance));
    follow(cx, radius, watch_section(Appearance::SECTION, appearance));
}

fn window_radius(appearance: &Appearance) -> WindowRadius {
    WindowRadius(f32::from(appearance.border_radius))
}

fn follow(cx: &mut Cx, radius: Signal<WindowRadius>, mut watch: SectionWatch<Appearance>) {
    let next_edit = async move {
        let appearance = watch.changed().await;
        (watch, appearance)
    };
    cx.spawn(next_edit, move |cx, (watch, appearance)| {
        let next = window_radius(&appearance);
        if radius.with_untracked(cx, |current| *current != next) {
            radius.set(cx, next);
        }
        follow(cx, radius, watch);
    });
}
