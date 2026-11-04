use crownui::prelude::{Color, Cx, Point, Rect, Runtime, Signal, Size, Store, rgb};

use super::annotation::{Annotation, Shape, TextNote};
use super::svg::rasterized_text;
use super::tool::Tool;
use crate::util::rect::spanned;

pub const PALETTE: [Color; 6] = [
    rgb(0xff3b30),
    rgb(0xff9500),
    rgb(0xffcc00),
    rgb(0x34c759),
    rgb(0x007aff),
    rgb(0x1c1c1e),
];
const HIGHLIGHT_OPACITY: f32 = 0.35;
const HIGHLIGHT_WIDTH_FACTOR: f32 = 5.0;
const STROKE_PER_EDGE: f32 = 0.004;
const MIN_STROKE: f32 = 2.0;
const TEXT_PER_EDGE: f32 = 0.03;
const MIN_TEXT: f32 = 14.0;

/// The markup session over one image: the chosen tool, the marks made so
/// far, the one being drawn and the crop, all in image pixels.
#[derive(Clone, Copy)]
pub struct Markup {
    pub tool: Signal<Tool>,
    pub color: Signal<Color>,
    pub label: Signal<String>,
    pub annotations: Signal<Vec<Annotation>>,
    pub draft: Signal<Option<Annotation>>,
    pub crop: Signal<Option<Rect>>,
    anchor: Store<Point>,
}

impl Markup {
    pub fn new(cx: &mut Cx) -> Self {
        Self {
            tool: cx.signal(Tool::Pen),
            color: cx.signal(PALETTE[0]),
            label: cx.signal(String::new()),
            annotations: cx.signal(Vec::new()),
            draft: cx.signal(None),
            crop: cx.signal(None),
            anchor: cx.store(Point::ZERO),
        }
    }

    pub fn is_pristine(self, rt: &mut Runtime) -> bool {
        self.annotations.with(rt, Vec::is_empty) && self.crop.with(rt, Option::is_none)
    }

    pub fn begin(self, cx: &mut Cx, at: Point, image: Size) {
        self.anchor.update(cx, |anchor| *anchor = at);
        let tool = self.tool.get(cx);
        if tool == Tool::Crop {
            self.crop.set(cx, Some(spanned(at, at)));
            return;
        }
        let shape = match tool {
            Tool::Pen | Tool::Highlighter => Shape::Stroke(vec![at]),
            Tool::Line => Shape::Line(at, at),
            Tool::Arrow => Shape::Arrow(at, at),
            Tool::Rectangle => Shape::Rectangle(spanned(at, at)),
            Tool::Ellipse => Shape::Ellipse(spanned(at, at)),
            Tool::Text | Tool::Crop => return,
        };
        let stroke = (image.width.max(image.height) * STROKE_PER_EDGE).max(MIN_STROKE);
        let color = self.color.get(cx);
        let (color, width) = match tool {
            Tool::Highlighter => (
                color.with_alpha_factor(HIGHLIGHT_OPACITY),
                stroke * HIGHLIGHT_WIDTH_FACTOR,
            ),
            _ => (color, stroke),
        };
        self.draft.set(
            cx,
            Some(Annotation {
                shape,
                color,
                width,
            }),
        );
    }

    pub fn extend(self, cx: &mut Cx, at: Point, image: Size) {
        let anchor = self.anchor.with(cx, |anchor| *anchor);
        if self.tool.get(cx) == Tool::Crop {
            let bounds = Rect::new(Point::ZERO, image);
            self.crop.set(cx, spanned(anchor, at).intersection(bounds));
            return;
        }
        self.draft.update(cx, |draft| {
            let Some(draft) = draft else { return };
            match &mut draft.shape {
                Shape::Stroke(points) => points.push(at),
                Shape::Line(_, end) | Shape::Arrow(_, end) => *end = at,
                Shape::Rectangle(rect) | Shape::Ellipse(rect) => *rect = spanned(anchor, at),
                Shape::Text(_) => {}
            }
        });
    }

    pub fn finish(self, cx: &mut Cx) {
        let mut finished = None;
        self.draft.update(cx, |draft| finished = draft.take());
        if let Some(annotation) = finished {
            self.annotations
                .update(cx, |annotations| annotations.push(annotation));
        }
    }

    pub fn place_text(self, cx: &mut Cx, at: Point, image: Size) {
        let content = self.label.get(cx);
        if self.tool.get(cx) != Tool::Text || content.trim().is_empty() {
            return;
        }
        let color = self.color.get(cx);
        let font_size = (image.width.max(image.height) * TEXT_PER_EDGE).max(MIN_TEXT);
        let Some((size, rgba)) = rasterized_text(&content, font_size, color) else {
            return;
        };
        let image = cx.register_image(size.width.ceil() as u32, size.height.ceil() as u32, rgba);
        cx.release_image_on_cleanup(image);
        let note = TextNote {
            origin: at,
            content,
            font_size,
            image,
            size,
        };
        self.annotations.update(cx, |annotations| {
            annotations.push(Annotation {
                shape: Shape::Text(note),
                color,
                width: 0.0,
            })
        });
    }

    pub fn undo(self, cx: &mut Cx) {
        let mut removed = None;
        self.annotations
            .update(cx, |annotations| removed = annotations.pop());
        match removed {
            Some(annotation) => release(cx, &annotation),
            None => self.crop.set(cx, None),
        }
    }

    pub fn discard(self, cx: &mut Cx) {
        let mut removed = Vec::new();
        self.annotations
            .update(cx, |annotations| removed = std::mem::take(annotations));
        removed
            .iter()
            .for_each(|annotation| release(cx, annotation));
        self.draft.set(cx, None);
        self.crop.set(cx, None);
    }
}

fn release(cx: &mut Cx, annotation: &Annotation) {
    if let Shape::Text(note) = &annotation.shape {
        cx.release_image(note.image);
    }
}
