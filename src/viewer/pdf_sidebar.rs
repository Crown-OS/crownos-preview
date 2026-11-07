use crownui::kit::{Theme, ToggleItem, icons, toggle_group, use_theme};
use crownui::prelude::*;

use super::chrome::notice;
use super::pdf_state::PdfState;
use crate::pdf::{OutlineEntry, PdfRequest};
use crate::util::fit::{centered, fit_scale};

const SIDEBAR_WIDTH: f32 = 168.0;
const THUMBNAIL_BOX: Size = Size::new(112.0, 144.0);
const THUMBNAIL_ROW_HEIGHT: f32 = 176.0;
const CONTENTS_ROW_HEIGHT: f32 = 30.0;
const INDENT_PER_LEVEL: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Thumbnails,
    Contents,
}

pub fn pdf_sidebar(state: PdfState) -> impl View {
    component(move |cx| {
        let selected = cx.signal(0_usize);
        let current_page = cx.memo(move |rt| state.current_page(rt));
        let page_numbers = cx.signal(Vec::<usize>::new());
        let headings = cx.signal(Vec::<(usize, OutlineEntry)>::new());
        cx.effect(move |rt| {
            let count = state.pages.with(rt, Vec::len);
            page_numbers.set(rt, (0..count).collect());
        });
        cx.effect(move |rt| {
            let outline = state.outline.get(rt);
            headings.set(rt, outline.into_iter().enumerate().collect());
        });
        vstack((
            toggle_group(
                [
                    ToggleItem::icon(icons::LAYOUT_GRID, "Thumbnails"),
                    ToggleItem::icon(icons::LIST_TREE, "Contents"),
                ],
                selected,
            )
            .self_center(),
            keyed(
                move |rt| {
                    if selected.get(rt) == 0 {
                        Panel::Thumbnails
                    } else {
                        Panel::Contents
                    }
                },
                move |_, panel| match panel {
                    Panel::Thumbnails => thumbnails(state, page_numbers, current_page).boxed(),
                    Panel::Contents => contents(state, headings).boxed(),
                },
            )
            .grow(1.0),
        ))
        .w(SIDEBAR_WIDTH)
        .shrink(0.0)
        .gap(8.0)
    })
}

fn thumbnails(
    state: PdfState,
    page_numbers: Signal<Vec<usize>>,
    current_page: Memo<usize>,
) -> impl View {
    lazy_vstack(
        page_numbers,
        |page| *page,
        THUMBNAIL_ROW_HEIGHT,
        move |cx, page| thumbnail(cx, state, page, current_page),
    )
    .grow(1.0)
}

fn thumbnail(
    cx: &mut Cx,
    state: PdfState,
    page: Signal<usize>,
    current_page: Memo<usize>,
) -> impl View + use<> {
    let theme = use_theme(cx);
    let index = page.get(cx);
    if !state
        .thumbnails
        .with(cx, |thumbnails| thumbnails.contains_key(&index))
    {
        state.request(cx, PdfRequest::Thumbnail { page: index });
    }
    vstack((
        canvas(move |rt, size, list| {
            let index = page.get(rt);
            let Some(image) = state
                .thumbnails
                .with(rt, |thumbnails| thumbnails.get(&index).copied())
            else {
                return;
            };
            let natural = state
                .pages
                .with(rt, |pages| pages.get(index).copied())
                .unwrap_or(THUMBNAIL_BOX);
            let rect = centered(natural, fit_scale(natural, size), size, Point::ZERO);
            list.fill(rect, CornerRadii::default(), WHITE);
            list.image(image, rect);
        })
        .w(THUMBNAIL_BOX.width)
        .h(THUMBNAIL_BOX.height),
        text(bind(move |rt| (page.get(rt) + 1).to_string()))
            .text_size(11.0)
            .text_color(bind(move |rt| theme.with(rt, |theme| theme.sidebar.text))),
    ))
    .items_center()
    .gap(4.0)
    .py(6.0)
    .mx(8.0)
    .rounded(8.0)
    .bg(bind(move |rt| {
        let highlighted = current_page.get(rt) == page.get(rt);
        theme.with(rt, |theme| highlight(theme, highlighted))
    }))
    .on_click(move |cx, _| {
        let index = page.get(cx);
        state.go_to_page(cx, index);
    })
}

fn contents(state: PdfState, headings: Signal<Vec<(usize, OutlineEntry)>>) -> AnyView {
    component(move |cx| {
        let empty = headings.with(cx, Vec::is_empty);
        if empty {
            return notice("No table of contents").boxed();
        }
        lazy_vstack(
            headings,
            |(index, _)| *index,
            CONTENTS_ROW_HEIGHT,
            move |cx, heading| heading_row(cx, state, heading),
        )
        .grow(1.0)
        .boxed()
    })
    .boxed()
}

fn heading_row(
    cx: &mut Cx,
    state: PdfState,
    heading: Signal<(usize, OutlineEntry)>,
) -> impl View + use<> {
    let theme = use_theme(cx);
    let (_, entry) = heading.get(cx);
    text(entry.title)
        .text_size(13.0)
        .truncate()
        .h(CONTENTS_ROW_HEIGHT)
        .pl(10.0 + entry.depth as f32 * INDENT_PER_LEVEL)
        .pr(10.0)
        .mx(6.0)
        .rounded(6.0)
        .text_color(bind(move |rt| theme.with(rt, |theme| theme.sidebar.text)))
        .hover(move |style| {
            style.bg(bind(move |rt| {
                theme.with(rt, |theme| theme.sidebar.hover_bg)
            }))
        })
        .on_click(move |cx, _| state.go_to_page(cx, entry.page))
}

fn highlight(theme: &Theme, highlighted: bool) -> Color {
    if highlighted {
        theme.sidebar.selected_bg
    } else {
        TRANSPARENT
    }
}
