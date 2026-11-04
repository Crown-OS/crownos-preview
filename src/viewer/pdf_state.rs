use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::mpsc::Sender;

use crownui::ext::ImageId;
use crownui::prelude::{Cx, Point, Rect, Runtime, Signal, Size, Store};

use super::zoom::Zoom;
use crate::pdf::{OutlineEntry, PdfEvent, PdfRequest, Selection, TextMatch};

pub const PAGE_GAP: f32 = 16.0;
pub const MARGIN: f32 = 24.0;
const RENDER_SCALE_STEPS: f32 = 4.0;
const KEPT_AROUND_VIEW: usize = 2;
const READING_LINE: f32 = 0.33;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageTexture {
    pub image: ImageId,
    pub scale: f32,
}

/// Everything the PDF view shows, in document space: logical pixels from the
/// top of the first page's margin, at the current zoom.
#[derive(Clone, Copy)]
pub struct PdfState {
    pub pages: Signal<Vec<Size>>,
    pub outline: Signal<Vec<OutlineEntry>>,
    pub viewport: Signal<Size>,
    pub zoom: Signal<Zoom>,
    pub scroll: Signal<Point>,
    pub textures: Signal<HashMap<usize, PageTexture>>,
    pub thumbnails: Signal<HashMap<usize, ImageId>>,
    pub matches: Signal<Vec<TextMatch>>,
    pub current_match: Signal<usize>,
    pub selection: Signal<Option<Rect>>,
    pub status: Signal<String>,
    pub failure: Signal<Option<String>>,
    requests: Store<Option<Sender<PdfRequest>>>,
    requested: Store<HashSet<(usize, u32)>>,
    in_view: Store<Range<usize>>,
}

impl PdfState {
    pub fn new(cx: &mut Cx) -> Self {
        Self {
            pages: cx.signal(Vec::new()),
            outline: cx.signal(Vec::new()),
            viewport: cx.signal(Size::new(0.0, 0.0)),
            zoom: cx.signal(Zoom::Fit),
            scroll: cx.signal(Point::ZERO),
            textures: cx.signal(HashMap::new()),
            thumbnails: cx.signal(HashMap::new()),
            matches: cx.signal(Vec::new()),
            current_match: cx.signal(0),
            selection: cx.signal(None),
            status: cx.signal(String::new()),
            failure: cx.signal(None),
            requests: cx.store(None),
            requested: cx.store(HashSet::new()),
            in_view: cx.store(0..0),
        }
    }

    pub fn connect(self, cx: &mut Cx, requests: Sender<PdfRequest>) {
        self.requests.update(cx, |slot| *slot = Some(requests));
    }

    pub fn request(self, rt: &Runtime, request: PdfRequest) {
        self.requests.with(rt, |requests| {
            if let Some(requests) = requests {
                let _ = requests.send(request);
            }
        });
    }

    /// The scale at which the widest page fills the viewport's width.
    pub fn fit(self, rt: &mut Runtime) -> f32 {
        let widest = self.pages.with(rt, |pages| {
            pages.iter().map(|page| page.width).fold(0.0, f32::max)
        });
        let available = self.viewport.get(rt).width - 2.0 * MARGIN;
        if widest > 0.0 {
            (available / widest).max(0.05)
        } else {
            1.0
        }
    }

    pub fn scale(self, rt: &mut Runtime) -> f32 {
        let fit = self.fit(rt);
        self.zoom.get(rt).scale(fit)
    }

    /// Every page's rectangle in document space.
    pub fn layout(self, rt: &mut Runtime) -> Vec<Rect> {
        let scale = self.scale(rt);
        let width = self.viewport.get(rt).width;
        self.pages.with(rt, |pages| page_rects(pages, scale, width))
    }

    pub fn document_height(self, rt: &mut Runtime) -> f32 {
        self.layout(rt)
            .last()
            .map_or(0.0, |last| last.max_y() + MARGIN)
    }

    pub fn scroll_by(self, cx: &mut Cx, dx: f32, dy: f32) {
        let current = self.scroll.get(cx);
        self.scroll_to(cx, Point::new(current.x + dx, current.y + dy));
    }

    pub fn scroll_to(self, cx: &mut Cx, target: Point) {
        let viewport = self.viewport.get(cx);
        let rects = self.layout(cx);
        let height = rects.last().map_or(0.0, |last| last.max_y() + MARGIN);
        let width = rects
            .iter()
            .map(|rect| rect.max_x() + MARGIN)
            .fold(viewport.width, f32::max);
        let clamped = Point::new(
            target.x.clamp(0.0, (width - viewport.width).max(0.0)),
            target.y.clamp(0.0, (height - viewport.height).max(0.0)),
        );
        self.scroll.set(cx, clamped);
    }

    pub fn go_to_page(self, cx: &mut Cx, page: usize) {
        if let Some(rect) = self.layout(cx).get(page) {
            let x = self.scroll.get(cx).x;
            self.scroll_to(cx, Point::new(x, rect.min_y() - MARGIN));
        }
    }

    /// Zooms while keeping the same part of the document in view.
    pub fn set_zoom(self, cx: &mut Cx, zoom: Zoom) {
        let before = self.scale(cx);
        self.zoom.set(cx, zoom);
        let after = self.scale(cx);
        let scroll = self.scroll.get(cx);
        let ratio = after / before;
        self.scroll_to(cx, Point::new(scroll.x * ratio, scroll.y * ratio));
    }

    pub fn change_zoom(self, cx: &mut Cx, change: impl FnOnce(Zoom, f32) -> Zoom) {
        let fit = self.fit(cx);
        let zoom = change(self.zoom.get(cx), fit);
        self.set_zoom(cx, zoom);
    }

    /// The page under the upper third of the viewport.
    pub fn current_page(self, rt: &mut Runtime) -> usize {
        let line = self.scroll.get(rt).y + self.viewport.get(rt).height * READING_LINE;
        let rects = self.layout(rt);
        rects
            .iter()
            .position(|rect| rect.max_y() + PAGE_GAP >= line)
            .unwrap_or(rects.len().saturating_sub(1))
    }

    /// The page under `point` (document space) and the point in its PDF points.
    pub fn page_point(self, rt: &mut Runtime, point: Point) -> Option<(usize, Point)> {
        let scale = self.scale(rt);
        let rects = self.layout(rt);
        let page = rects.iter().position(|rect| rect.contains(point))?;
        let origin = rects[page].origin;
        Some((
            page,
            Point::new((point.x - origin.x) / scale, (point.y - origin.y) / scale),
        ))
    }

    /// Renders that the visible pages still need at `scale` device pixels per point.
    pub fn request_missing(self, rt: &mut Runtime, visible: Range<usize>, scale: f32) {
        let wanted = (scale * RENDER_SCALE_STEPS).ceil() / RENDER_SCALE_STEPS;
        self.in_view
            .update(rt, |in_view| *in_view = visible.clone());
        let missing: Vec<usize> = self.textures.with(rt, |textures| {
            visible
                .filter(|page| {
                    textures
                        .get(page)
                        .is_none_or(|texture| texture.scale != wanted)
                })
                .collect()
        });
        let fresh: Vec<usize> = self.requested.update(rt, |requested| {
            missing
                .into_iter()
                .filter(|page| requested.insert((*page, wanted.to_bits())))
                .collect()
        });
        for page in fresh {
            self.request(
                rt,
                PdfRequest::Render {
                    page,
                    scale: wanted,
                },
            );
        }
    }

    pub fn receive(self, cx: &mut Cx, event: PdfEvent) {
        match event {
            PdfEvent::Opened { pages, outline } => {
                self.pages.set(cx, pages);
                self.outline.set(cx, outline);
            }
            PdfEvent::Rendered {
                page,
                scale,
                bitmap,
            } => {
                self.requested.update(cx, |requested| {
                    requested.retain(|(pending, _)| *pending != page)
                });
                let image = cx.register_image(bitmap.width, bitmap.height, bitmap.rgba);
                cx.release_image_on_cleanup(image);
                let keep = self.in_view.with(cx, |range| {
                    range.start.saturating_sub(KEPT_AROUND_VIEW)..range.end + KEPT_AROUND_VIEW
                });
                let mut released = Vec::new();
                self.textures.update(cx, |textures| {
                    released.extend(
                        textures
                            .insert(page, PageTexture { image, scale })
                            .map(|old| old.image),
                    );
                    textures.retain(|page, texture| {
                        let kept = keep.contains(page);
                        if !kept {
                            released.push(texture.image);
                        }
                        kept
                    });
                });
                released
                    .into_iter()
                    .for_each(|image| cx.release_image(image));
            }
            PdfEvent::Thumbnail { page, bitmap } => {
                let image = cx.register_image(bitmap.width, bitmap.height, bitmap.rgba);
                cx.release_image_on_cleanup(image);
                self.thumbnails.update(cx, |thumbnails| {
                    thumbnails.insert(page, image);
                });
            }
            PdfEvent::Found(matches) => {
                let status = match matches.len() {
                    0 => "No matches".to_owned(),
                    count => format!("1 of {count}"),
                };
                self.status.set(cx, status);
                self.matches.set(cx, matches);
                self.current_match.set(cx, 0);
                self.show_match(cx, 0);
            }
            PdfEvent::Selected(Selection { text, .. }) => {
                cx.copy_to_clipboard(text);
                self.status.set(cx, "Copied".into());
            }
            PdfEvent::Failed(message) => self
                .failure
                .set(cx, Some(format!("Cannot open this PDF: {message}"))),
        }
    }

    pub fn step_match(self, cx: &mut Cx, forward: bool) {
        let count = self.matches.with(cx, Vec::len);
        if count == 0 {
            return;
        }
        let current = self.current_match.get(cx);
        let next = if forward {
            (current + 1) % count
        } else {
            (current + count - 1) % count
        };
        self.current_match.set(cx, next);
        self.status.set(cx, format!("{} of {count}", next + 1));
        self.show_match(cx, next);
    }

    fn show_match(self, cx: &mut Cx, index: usize) {
        let Some(found) = self.matches.with(cx, |matches| matches.get(index).copied()) else {
            return;
        };
        let scale = self.scale(cx);
        let Some(page) = self.layout(cx).get(found.page).copied() else {
            return;
        };
        let viewport = self.viewport.get(cx);
        let target = page.min_y() + found.area.min_y() * scale - viewport.height * READING_LINE;
        let x = self.scroll.get(cx).x;
        self.scroll_to(cx, Point::new(x, target));
    }
}

/// Pages stacked top to bottom, centered horizontally in `viewport_width`.
pub fn page_rects(pages: &[Size], scale: f32, viewport_width: f32) -> Vec<Rect> {
    let mut top = MARGIN;
    pages
        .iter()
        .map(|page| {
            let size = Size::new(page.width * scale, page.height * scale);
            let left = ((viewport_width - size.width) / 2.0).max(MARGIN);
            let rect = Rect::new(Point::new(left, top), size);
            top += size.height + PAGE_GAP;
            rect
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_stack_with_gaps_and_center() {
        let rects = page_rects(
            &[Size::new(100.0, 200.0), Size::new(50.0, 100.0)],
            2.0,
            400.0,
        );
        assert_eq!(
            rects[0],
            Rect::new(Point::new(100.0, MARGIN), Size::new(200.0, 400.0))
        );
        assert_eq!(rects[1].min_y(), MARGIN + 400.0 + PAGE_GAP);
        assert_eq!(rects[1].min_x(), 150.0);
    }

    #[test]
    fn pages_wider_than_the_viewport_keep_their_margin() {
        let rects = page_rects(&[Size::new(1000.0, 10.0)], 1.0, 400.0);
        assert_eq!(rects[0].min_x(), MARGIN);
    }
}
