use crownui::ext::WheelEvent;
use crownui::prelude::{Cx, Point, Rect, Runtime, Signal, Size, Store, Vector};

use super::zoom::Zoom;
use crate::util::fit::{centered, fit_scale};
use crate::util::pinch::PinchTracker;

const LINE_HEIGHT: f32 = 40.0;
const WHEEL_ZOOM_STEP: f32 = 1.1;

/// A zoomable, pannable view of one piece of content.
#[derive(Clone, Copy)]
pub struct Stage {
    pub viewport: Signal<Size>,
    pub content: Signal<Size>,
    pub zoom: Signal<Zoom>,
    pub pan: Signal<Point>,
    pinch: Store<PinchTracker>,
}

impl Stage {
    pub fn new(cx: &mut Cx) -> Self {
        Self {
            viewport: cx.signal(Size::new(0.0, 0.0)),
            content: cx.signal(Size::new(0.0, 0.0)),
            zoom: cx.signal(Zoom::Fit),
            pan: cx.signal(Point::ZERO),
            pinch: cx.store(PinchTracker::default()),
        }
    }

    /// Small content is shown at its own size rather than blown up.
    pub fn fit(self, rt: &mut Runtime) -> f32 {
        let (content, viewport) = (self.content.get(rt), self.viewport.get(rt));
        fit_scale(content, viewport).min(1.0)
    }

    pub fn scale(self, rt: &mut Runtime) -> f32 {
        let fit = self.fit(rt);
        self.zoom.get(rt).scale(fit)
    }

    /// Where the content lands in the viewport, and at what scale.
    pub fn placement(self, rt: &mut Runtime) -> (Rect, f32) {
        let scale = self.scale(rt);
        let rect = centered(
            self.content.get(rt),
            scale,
            self.viewport.get(rt),
            self.pan.get(rt),
        );
        (rect, scale)
    }

    /// `point` in the viewport mapped to content coordinates.
    pub fn to_content(self, rt: &mut Runtime, point: Point) -> Point {
        let (rect, scale) = self.placement(rt);
        Point::new(
            (point.x - rect.origin.x) / scale,
            (point.y - rect.origin.y) / scale,
        )
    }

    pub fn zoom_by(self, cx: &mut Cx, factor: f32) {
        let fit = self.fit(cx);
        self.zoom
            .update(cx, |zoom| *zoom = zoom.multiplied(factor, fit));
    }

    pub fn zoom_in(self, cx: &mut Cx) {
        let fit = self.fit(cx);
        self.zoom.update(cx, |zoom| *zoom = zoom.zoomed_in(fit));
    }

    pub fn zoom_out(self, cx: &mut Cx) {
        let fit = self.fit(cx);
        self.zoom.update(cx, |zoom| *zoom = zoom.zoomed_out(fit));
    }

    pub fn reset(self, cx: &mut Cx) {
        self.zoom.set(cx, Zoom::Fit);
        self.pan.set(cx, Point::ZERO);
    }

    pub fn actual_size(self, cx: &mut Cx) {
        self.zoom.set(cx, Zoom::Scale(1.0));
        self.pan.set(cx, Point::ZERO);
    }

    pub fn pan_by(self, cx: &mut Cx, delta: Vector) {
        self.pan.update(cx, |pan| {
            *pan = Point::new(pan.x + delta.x, pan.y + delta.y)
        });
    }

    /// Ctrl + wheel zooms; a plain wheel pans.
    pub fn wheel(self, cx: &mut Cx, event: &WheelEvent) {
        let delta = event.delta.to_pixels(LINE_HEIGHT);
        if event.modifiers.ctrl_key {
            self.zoom_by(cx, WHEEL_ZOOM_STEP.powf(-delta.y / LINE_HEIGHT));
        } else {
            self.pan_by(cx, Vector::new(-delta.x, -delta.y));
        }
    }

    /// `scale` is the pinch's scale since it began.
    pub fn pinch(self, cx: &mut Cx, scale: f32) {
        let factor = self.pinch.update(cx, |pinch| pinch.step(scale));
        self.zoom_by(cx, factor);
    }

    pub fn percent(self, rt: &mut Runtime) -> String {
        let fit = self.fit(rt);
        self.zoom.get(rt).percent(fit)
    }
}
