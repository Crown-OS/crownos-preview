use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};

use crownui::prelude::{Point, Rect, Size};
use poppler::{Document, Page, SelectionStyle};
use tokio::sync::mpsc::UnboundedSender;

use super::outline::{OutlineEntry, outline};
use super::render::{Bitmap, render};

const THUMBNAIL_WIDTH: f64 = 112.0;

pub enum PdfRequest {
    Render { page: usize, scale: f32 },
    Thumbnail { page: usize },
    Search(String),
    Select { page: usize, area: Rect },
}

/// A search hit, in points from the page's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextMatch {
    pub page: usize,
    pub area: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    pub page: usize,
    pub area: Rect,
    pub text: String,
}

pub enum PdfEvent {
    Opened {
        pages: Vec<Size>,
        outline: Vec<OutlineEntry>,
    },
    Rendered {
        page: usize,
        scale: f32,
        bitmap: Bitmap,
    },
    Thumbnail {
        page: usize,
        bitmap: Bitmap,
    },
    Found(Vec<TextMatch>),
    Selected(Selection),
    Failed(String),
}

#[derive(PartialEq, Eq, Hash)]
enum Slot {
    Render(usize),
    Thumbnail(usize),
    Search,
    Select,
}

impl PdfRequest {
    fn slot(&self) -> Slot {
        match self {
            Self::Render { page, .. } => Slot::Render(*page),
            Self::Thumbnail { page } => Slot::Thumbnail(*page),
            Self::Search(_) => Slot::Search,
            Self::Select { .. } => Slot::Select,
        }
    }
}

/// Opens `path` on a poppler thread of its own; poppler documents never leave it.
pub fn open(path: PathBuf, events: UnboundedSender<PdfEvent>) -> Sender<PdfRequest> {
    let (requests, inbox) = channel();
    std::thread::spawn(move || serve(&path, &inbox, &events));
    requests
}

fn serve(path: &Path, inbox: &Receiver<PdfRequest>, events: &UnboundedSender<PdfEvent>) {
    let document = match load(path) {
        Ok(document) => document,
        Err(message) => {
            let _ = events.send(PdfEvent::Failed(message));
            return;
        }
    };
    let pages = (0..document.n_pages())
        .filter_map(|index| document.page(index))
        .map(|page| {
            let (width, height) = page.size();
            Size::new(width as f32, height as f32)
        })
        .collect();
    let opened = PdfEvent::Opened {
        pages,
        outline: outline(&document),
    };
    if events.send(opened).is_err() {
        return;
    }
    while let Ok(first) = inbox.recv() {
        let pending: Vec<_> = std::iter::once(first).chain(inbox.try_iter()).collect();
        for request in latest_per_slot(pending) {
            let Some(event) = answer(&document, request) else {
                continue;
            };
            if events.send(event).is_err() {
                return;
            }
        }
    }
}

fn load(path: &Path) -> Result<Document, String> {
    let uri = glib::filename_to_uri(path, None).map_err(|error| error.to_string())?;
    Document::from_file(&uri, None).map_err(|error| error.to_string())
}

/// Newest first, one request per slot: a burst of scroll-driven renders
/// collapses to what is on screen now.
fn latest_per_slot(pending: Vec<PdfRequest>) -> Vec<PdfRequest> {
    let mut seen = HashSet::new();
    pending
        .into_iter()
        .rev()
        .filter(|request| seen.insert(request.slot()))
        .collect()
}

fn answer(document: &Document, request: PdfRequest) -> Option<PdfEvent> {
    let page_at = |index: usize| document.page(i32::try_from(index).ok()?);
    match request {
        PdfRequest::Render { page, scale } => {
            let bitmap = render(&page_at(page)?, f64::from(scale))?;
            Some(PdfEvent::Rendered {
                page,
                scale,
                bitmap,
            })
        }
        PdfRequest::Thumbnail { page } => {
            let source = page_at(page)?;
            let bitmap = render(&source, THUMBNAIL_WIDTH / source.size().0)?;
            Some(PdfEvent::Thumbnail { page, bitmap })
        }
        PdfRequest::Search(query) => Some(PdfEvent::Found(search(document, &query))),
        PdfRequest::Select { page, area } => {
            let text = selected_text(&page_at(page)?, area)?;
            Some(PdfEvent::Selected(Selection { page, area, text }))
        }
    }
}

fn search(document: &Document, query: &str) -> Vec<TextMatch> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    (0..document.n_pages())
        .filter_map(|index| Some((usize::try_from(index).ok()?, document.page(index)?)))
        .flat_map(|(index, page)| {
            let height = page.size().1;
            page.find_text(query).into_iter().map(move |hit| TextMatch {
                page: index,
                area: Rect::new(
                    Point::new(hit.x1() as f32, (height - hit.y2()) as f32),
                    Size::new((hit.x2() - hit.x1()) as f32, (hit.y2() - hit.y1()) as f32),
                ),
            })
        })
        .collect()
}

fn selected_text(page: &Page, area: Rect) -> Option<String> {
    let mut selection = poppler::Rectangle::new();
    selection.set_x1(f64::from(area.min_x()));
    selection.set_y1(f64::from(area.min_y()));
    selection.set_x2(f64::from(area.max_x()));
    selection.set_y2(f64::from(area.max_y()));
    let text = page.selected_text(SelectionStyle::Glyph, &mut selection)?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_page_pdf(path: &Path) {
        let surface = cairo::PdfSurface::new(200.0, 100.0, path).expect("pdf surface");
        let context = cairo::Context::new(&surface).expect("context");
        let chapter = surface
            .add_outline(
                cairo::PDF_OUTLINE_ROOT,
                "Chapter two",
                "page=2",
                cairo::PdfOutline::empty(),
            )
            .expect("outline");
        surface
            .add_outline(chapter, "Section", "page=2", cairo::PdfOutline::empty())
            .expect("outline");
        for word in ["Hello preview", "Second page"] {
            context.set_font_size(14.0);
            context.move_to(20.0, 50.0);
            context.show_text(word).expect("text");
            context.show_page().expect("page");
        }
        surface.finish();
    }

    #[test]
    fn pages_open_render_search_and_select() {
        let path = std::env::temp_dir().join(format!("crownos-preview-{}.pdf", std::process::id()));
        two_page_pdf(&path);
        let (events, mut inbox) = tokio::sync::mpsc::unbounded_channel();
        let requests = open(path.clone(), events);
        let Some(PdfEvent::Opened { pages, outline }) = inbox.blocking_recv() else {
            panic!("the document did not open");
        };
        assert_eq!(pages, vec![Size::new(200.0, 100.0); 2]);
        let headings: Vec<_> = outline
            .iter()
            .map(|entry| (entry.title.as_str(), entry.page, entry.depth))
            .collect();
        assert_eq!(headings, [("Chapter two", 1, 0), ("Section", 1, 1)]);

        requests
            .send(PdfRequest::Search("preview".into()))
            .expect("send");
        let Some(PdfEvent::Found(matches)) = inbox.blocking_recv() else {
            panic!("no search answer");
        };
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].page, 0);
        assert!(
            matches[0].area.min_y() < 50.0 && matches[0].area.max_y() > 35.0,
            "{matches:?}"
        );

        let everything = Rect::new(Point::ZERO, Size::new(200.0, 100.0));
        requests
            .send(PdfRequest::Select {
                page: 1,
                area: everything,
            })
            .expect("send");
        let Some(PdfEvent::Selected(selection)) = inbox.blocking_recv() else {
            panic!("no selection answer");
        };
        assert_eq!(selection.text, "Second page");

        requests
            .send(PdfRequest::Render {
                page: 0,
                scale: 2.0,
            })
            .expect("send");
        let Some(PdfEvent::Rendered { bitmap, .. }) = inbox.blocking_recv() else {
            panic!("no render answer");
        };
        std::fs::remove_file(&path).ok();
        assert_eq!((bitmap.width, bitmap.height), (400, 200));
        assert_eq!(bitmap.rgba.len(), 400 * 200 * 4);
    }
}
