use std::path::PathBuf;

use crownui::kit::{Theme, ThemeMode, use_theme};
use crownui::prelude::*;

use super::chrome::{content_card, file_name, notice, title_bar, window_frame};
use super::line_list::{ROW_HEIGHT, line_list};
use crate::document::DocumentKind;
use crate::text::{Block, Line, blocks, highlight, read_text};
use crate::util::background::in_background;
use crate::util::fonts::MONOSPACE_FAMILY as MONOSPACE;

const CODE_TEXT_SIZE: f32 = 13.0;
const GUTTER_WIDTH: f32 = 56.0;
const PROSE_WIDTH: f32 = 820.0;
const WRAPPED_TEXT_LIMIT: usize = 256 * 1024;

enum Content {
    Markdown(Vec<Block>),
    Code(Vec<Line>),
    Plain(String),
    Log(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Loading,
    Ready { truncated: bool },
    Failed(String),
}

pub fn text_view(path: PathBuf, kind: DocumentKind) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        let dark = theme.with(cx, |theme| theme.mode == ThemeMode::Dark);
        let phase = cx.signal(Phase::Loading);
        let content = cx.store(None::<Content>);
        let source = path.clone();
        in_background(
            cx,
            move || {
                let loaded = read_text(&source)?;
                let extension = source
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or_default();
                let content = match kind {
                    DocumentKind::Markdown => Content::Markdown(blocks(&loaded.text)),
                    DocumentKind::Code => Content::Code(highlight(&loaded.text, extension, dark)),
                    _ if loaded.text.len() <= WRAPPED_TEXT_LIMIT => Content::Plain(loaded.text),
                    _ => Content::Log(loaded.text.lines().map(str::to_owned).collect()),
                };
                anyhow::Ok((content, loaded.truncated))
            },
            move |cx, loaded| match loaded {
                Ok((loaded, truncated)) => {
                    content.update(cx, |slot| *slot = Some(loaded));
                    phase.set(cx, Phase::Ready { truncated });
                }
                Err(error) => {
                    phase.set(cx, Phase::Failed(format!("Cannot read this file: {error}")))
                }
            },
        );

        window_frame(
            title_bar(
                (),
                file_name(&path),
                (),
                (text(bind(move |rt| match phase.get(rt) {
                    Phase::Ready { truncated: true } => "Showing the first 2 MB".to_owned(),
                    _ => String::new(),
                }))
                .text_size(12.0)
                .text_color(bind(move |rt| theme.with(rt, |theme| theme.text.muted))),),
            ),
            content_card((keyed(
                move |rt| phase.get(rt),
                move |cx, phase| match phase {
                    Phase::Loading => vstack(()).boxed(),
                    Phase::Failed(message) => notice(message).boxed(),
                    Phase::Ready { .. } => match content.update(cx, Option::take) {
                        Some(Content::Markdown(blocks)) => markdown_view(blocks).boxed(),
                        Some(Content::Code(lines)) => code_view(lines).boxed(),
                        Some(Content::Plain(text)) => plain_view(text).boxed(),
                        Some(Content::Log(lines)) => log_view(lines).boxed(),
                        None => vstack(()).boxed(),
                    },
                },
            )
            .grow(1.0),)),
        )
    })
}

fn themed(theme: Signal<Theme>, pick: fn(&Theme) -> Color) -> impl IntoProp<Color> {
    bind(move |rt| theme.with(rt, pick))
}

fn plain_view(content: String) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        scroll_view(
            text(content)
                .font_family(MONOSPACE)
                .text_size(CODE_TEXT_SIZE)
                .text_color(themed(theme, |theme| theme.text.body))
                .p(20.0),
        )
        .grow(1.0)
    })
}

fn log_view(lines: Vec<String>) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        line_list(lines, move |number, line| {
            numbered_row(
                theme,
                number,
                (monospace(keep_spaces(line)).text_color(themed(theme, |theme| theme.text.body)),),
            )
        })
    })
}

fn code_view(lines: Vec<Line>) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        line_list(lines, move |number, spans| {
            let spans: Vec<_> = spans
                .iter()
                .map(|span| {
                    let [red, green, blue] = span.rgb;
                    monospace(keep_spaces(&span.text))
                        .text_color(rgb(u32::from_be_bytes([0, red, green, blue])))
                })
                .collect();
            numbered_row(theme, number, spans)
        })
    })
}

/// Shaping drops runs of plain spaces at span edges; non-breaking ones keep their width.
fn keep_spaces(line: &str) -> String {
    line.replace(' ', "\u{a0}")
}

fn monospace(content: String) -> impl View + Styled {
    text(content)
        .font_family(MONOSPACE)
        .text_size(CODE_TEXT_SIZE)
        .shrink(0.0)
}

fn numbered_row(theme: Signal<Theme>, number: usize, content: impl ViewSeq) -> AnyView {
    hstack((gutter(theme, number), hstack(content).shrink(0.0)))
        .h(ROW_HEIGHT)
        .items_center()
        .boxed()
}

fn gutter(theme: Signal<Theme>, number: usize) -> impl View {
    text((number + 1).to_string())
        .font_family(MONOSPACE)
        .text_size(CODE_TEXT_SIZE)
        .text_right()
        .w(GUTTER_WIDTH)
        .pr(16.0)
        .shrink(0.0)
        .text_color(themed(theme, |theme| theme.text.muted))
}

fn markdown_view(blocks: Vec<Block>) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        let rendered: Vec<AnyView> = blocks
            .into_iter()
            .map(|block| markdown_block(theme, block))
            .collect();
        scroll_view(
            vstack((vstack(rendered).gap(12.0).max_w(PROSE_WIDTH).w_full(),))
                .items_center()
                .px(32.0)
                .py(28.0),
        )
        .grow(1.0)
    })
}

fn markdown_block(theme: Signal<Theme>, block: Block) -> AnyView {
    let body = themed(theme, |theme| theme.text.body);
    match block {
        Block::Heading {
            level,
            text: heading,
        } => text(heading)
            .text_size(match level {
                1 => 28.0,
                2 => 22.0,
                3 => 18.0,
                _ => 16.0,
            })
            .font_bold()
            .pt(8.0)
            .text_color(themed(theme, |theme| theme.text.primary))
            .boxed(),
        Block::Paragraph(paragraph) => text(paragraph).text_size(15.0).text_color(body).boxed(),
        Block::Quote(quote) => hstack((
            vstack(())
                .w(3.0)
                .self_stretch()
                .rounded(2.0)
                .bg(themed(theme, |theme| theme.accent.start)),
            text(quote)
                .italic()
                .text_size(15.0)
                .shrink(1.0)
                .text_color(themed(theme, |theme| theme.text.muted)),
        ))
        .gap(12.0)
        .boxed(),
        Block::Item {
            depth,
            marker,
            text: item,
        } => hstack((
            text(marker)
                .text_size(15.0)
                .min_w(22.0)
                .text_color(themed(theme, |theme| theme.text.muted)),
            text(item).text_size(15.0).shrink(1.0).text_color(body),
        ))
        .pl(depth as f32 * 22.0)
        .gap(6.0)
        .boxed(),
        Block::Code(code) => text(code)
            .font_family(MONOSPACE)
            .text_size(CODE_TEXT_SIZE)
            .p(14.0)
            .rounded(8.0)
            .bg(themed(theme, |theme| theme.surface.sunken))
            .text_color(body)
            .boxed(),
        Block::Rule => divider().boxed(),
    }
}
