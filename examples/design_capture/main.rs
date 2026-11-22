//! Captures every Preview state for the CrownOS-Design Figma file. Run both appearances with
//! `../crownui/capture/scripts/capture-app.sh . design_capture <out>`; output lands in
//! `$CROWN_DESIGN_DIR`, the sample documents in `$CROWN_DESIGN_DIR/.fixtures`.
#![allow(dead_code)]

#[path = "../../src/app_config.rs"]
mod app_config;
#[path = "../../src/document.rs"]
mod document;
#[path = "../../src/launch.rs"]
mod launch;
#[path = "../../src/markup/mod.rs"]
mod markup;
#[path = "../../src/media/mod.rs"]
mod media;
#[path = "../../src/pdf/mod.rs"]
mod pdf;
#[path = "../../src/picture/mod.rs"]
mod picture;
#[path = "../../src/playlist.rs"]
mod playlist;
#[path = "../../src/service.rs"]
mod service;
#[path = "../../src/text/mod.rs"]
mod text;
#[path = "../../src/util/mod.rs"]
mod util;
#[path = "../../src/viewer/mod.rs"]
mod viewer;

mod fixtures;
mod session;
mod tasks;

use std::path::{Path, PathBuf};

use anyhow::Result;
use capture::Carrier;
use crowntest::{Role, role};

use fixtures::*;
use launch::Mode;
use session::Studio;

fn main() -> Result<()> {
    let output = std::env::var_os("CROWN_DESIGN_DIR")
        .map_or_else(|| PathBuf::from("target/design"), PathBuf::from);
    let samples = fixtures::prepare(&output.join(".fixtures"))?;
    let studio = Studio::new()?;
    let mut carrier = Carrier::new(
        "Preview",
        "crownos-preview: the quick-look previewer for pictures, screenshots, PDFs, recordings, songs and text, with markup and crop tools, captured headlessly from the current source.",
    );
    images(&studio, &samples, &mut carrier)?;
    markup(&studio, &samples, &mut carrier)?;
    pdfs(&studio, &samples, &mut carrier)?;
    media(&studio, &samples, &mut carrier)?;
    texts(&studio, &samples, &mut carrier)?;
    empty_and_errors(&studio, &samples, &mut carrier)?;
    let path = carrier.write(None)?;
    println!("{}", path.display());
    capture::finish()
}

fn images(studio: &Studio, samples: &Path, carrier: &mut Carrier) -> Result<()> {
    const GROUP: &str = "Images";
    let mut session = studio.open(samples, SCREENSHOT, Mode::View)?;
    session.pump_ms(800);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Screenshot",
        "A screenshot fitted to the window, with the zoom pill",
    )?;
    for _ in 0..3 {
        session.app.click(role(Role::Button).name("Zoom in"));
    }
    session.pump_ms(200);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Zoomed in",
        "Zoomed past fit; drag pans, double-click toggles actual size",
    )?;
    drop(session);

    let mut session = studio.open(samples, LOGO, Mode::View)?;
    session.pump_ms(800);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Vector image",
        "An SVG: no Markup button, since only pictures can be marked up",
    )?;
    Ok(())
}

fn markup(studio: &Studio, samples: &Path, carrier: &mut Carrier) -> Result<()> {
    const GROUP: &str = "Markup";
    let mut session = studio.open(samples, SCREENSHOT, Mode::View)?;
    session.pump_ms(800);
    session.app.click(role(Role::Button).name("Markup"));
    session.pump_ms(200);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Markup tools",
        "Markup toolbar: tools, colors, undo, Cancel and Done",
    )?;

    session.choose_tool("Rectangle");
    session.drag_on_image((0.212, 0.268), (0.347, 0.338))?;
    session.choose_color(1)?;
    session.choose_tool("Arrow");
    session.drag_on_image((0.47, 0.10), (0.36, 0.27))?;
    session.choose_color(2)?;
    session.choose_tool("Highlight");
    session.drag_on_image((0.36, 0.268), (0.555, 0.268))?;
    session.choose_color(4)?;
    session.choose_tool("Oval");
    session.drag_on_image((0.345, 0.315), (0.5, 0.535))?;
    session.choose_color(3)?;
    session.choose_tool("Sketch");
    session.stroke_on_image(&[
        (0.62, 0.42),
        (0.635, 0.45),
        (0.65, 0.47),
        (0.67, 0.43),
        (0.70, 0.38),
        (0.73, 0.34),
    ])?;
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Annotated",
        "A rectangle, arrow, highlight, oval and sketch in five colors",
    )?;

    session.choose_color(0)?;
    session.choose_tool("Text");
    session
        .app
        .click(role(Role::TextField).name("Type, then click the image"));
    session.app.type_text("New sidebar");
    session.click_on_image((0.215, 0.215))?;
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Text",
        "The Text tool's field, and a label placed on the picture",
    )?;
    drop(session);

    let mut session = studio.open(samples, SCREENSHOT, Mode::View)?;
    session.pump_ms(800);
    session.app.click(role(Role::Button).name("Markup"));
    session.choose_tool("Crop");
    session.drag_on_image((0.2, 0.16), (0.8, 0.84))?;
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Crop",
        "The Crop tool dims everything outside the kept area",
    )?;
    Ok(())
}

fn pdfs(studio: &Studio, samples: &Path, carrier: &mut Carrier) -> Result<()> {
    const GROUP: &str = "PDF";
    let mut session = studio.open(samples, RELEASE_NOTES, Mode::View)?;
    session.pump_ms(2000);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Document",
        "Pages with the thumbnail sidebar, search field and page pill",
    )?;

    session.app.click(role(Role::RadioButton).name("Contents"));
    session.pump_ms(300);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Contents",
        "The sidebar's table of contents from the PDF outline",
    )?;

    session.app.click(role(Role::TextField).name("Search"));
    session.app.type_text("Preview");
    session.app.press_key("Enter");
    session.pump_ms(1500);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Search results",
        "Matches highlighted, the current one in orange, count in the title bar",
    )?;
    drop(session);

    let mut session = studio.open(samples, RELEASE_NOTES, Mode::View)?;
    session.pump_ms(1500);
    session.drag_on_page((60.0, 176.0), (535.0, 262.0))?;
    session.pump_ms(800);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Text selected",
        "A dragged selection is copied; the title bar says Copied",
    )?;
    drop(session);

    let mut session = studio.open(samples, RELEASE_NOTES, Mode::View)?;
    session.pump_ms(1000);
    session.app.click(role(Role::Button).name("Sidebar"));
    for _ in 0..2 {
        session.app.click(role(Role::Button).name("Zoom in"));
    }
    session.pump_ms(1500);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Zoomed, sidebar hidden",
        "Sidebar toggled off and the pages zoomed in",
    )?;
    Ok(())
}

fn media(studio: &Studio, samples: &Path, carrier: &mut Carrier) -> Result<()> {
    const GROUP: &str = "Video & audio";
    let mut session = studio.open(samples, RECORDING, Mode::View)?;
    session.pump_ms(2500);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Video playing",
        "A screen recording playing, with the transport bar",
    )?;
    session.app.click(role(Role::Button).name("Pause"));
    session.pump_ms(600);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Video paused",
        "Paused: the transport shows Play",
    )?;
    drop(session);

    let mut session = studio.open(samples, VOICE_MEMO, Mode::View)?;
    session.pump_ms(1500);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Audio",
        "A song or memo: artwork placeholder and transport",
    )?;
    session.app.click(role(Role::Button).name("Mute"));
    session.pump_ms(400);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "Audio muted",
        "Muted: the speaker shows a cross",
    )?;
    Ok(())
}

fn texts(studio: &Studio, samples: &Path, carrier: &mut Carrier) -> Result<()> {
    const GROUP: &str = "Text";
    let documents = [
        (
            README,
            "Markdown",
            "Markdown rendered as prose: headings, lists, quote, code, rule",
        ),
        (
            SOURCE,
            "Code",
            "Source code with line numbers and syntax colors",
        ),
        (
            NOTES,
            "Plain text",
            "A plain text file in the monospace face",
        ),
        (
            LOG,
            "Large log",
            "A log over 2 MB: numbered lines, only the first 2 MB shown",
        ),
    ];
    for (file, name, description) in documents {
        let mut session = studio.open(samples, file, Mode::View)?;
        session.pump_ms(2000);
        carrier.capture_state(&mut session.app, GROUP, name, description)?;
    }
    Ok(())
}

fn empty_and_errors(studio: &Studio, samples: &Path, carrier: &mut Carrier) -> Result<()> {
    const GROUP: &str = "Empty & error states";
    let mut session = studio.open_empty()?;
    session.pump_ms(300);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "No file",
        "Opened without a file: drop one to preview it",
    )?;
    drop(session);

    let mut session = studio.open(samples, INVOICE, Mode::View)?;
    session.pump_ms(1500);
    session.app.click(role(Role::RadioButton).name("Contents"));
    session.pump_ms(300);
    carrier.capture_state(
        &mut session.app,
        GROUP,
        "No table of contents",
        "A PDF without an outline, Contents selected",
    )?;
    drop(session);

    let failures = [
        (DAMAGED_IMAGE, "Image cannot open", "A damaged picture", 800),
        (DAMAGED_PDF, "PDF cannot open", "A damaged PDF", 1000),
        (
            DAMAGED_VIDEO,
            "Cannot play",
            "A file GStreamer cannot play",
            2000,
        ),
        (
            LOCKED_TEXT,
            "Cannot read",
            "A text file without read permission",
            800,
        ),
    ];
    for (file, name, description, wait) in failures {
        let mut session = studio.open(samples, file, Mode::View)?;
        session.pump_ms(wait);
        carrier.capture_state(&mut session.app, GROUP, name, description)?;
    }
    Ok(())
}
