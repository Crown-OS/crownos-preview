use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use cairo::{Context as Cairo, FontSlant, FontWeight, PdfOutline, PdfSurface};
use image::{Rgba, RgbaImage};

pub const SCREENSHOT: &str = "Screenshot 2026-11-11 at 09.41.27.png";
pub const LOGO: &str = "crown-mark.svg";
pub const RELEASE_NOTES: &str = "CrownOS 1.0 Release Notes.pdf";
pub const INVOICE: &str = "CrownOS 1.0 Release Notes (print).pdf";
pub const RECORDING: &str = "Screen Recording 2026-11-11 at 10.02.15.mp4";
pub const VOICE_MEMO: &str = "Voice Memo 12.ogg";
pub const README: &str = "README.md";
pub const SOURCE: &str = "wallpaper.rs";
pub const NOTES: &str = "Meeting notes.txt";
pub const LOG: &str = "crownpositor.log";
pub const DAMAGED_IMAGE: &str = "IMG_2041.png";
pub const DAMAGED_PDF: &str = "Boarding pass.pdf";
pub const DAMAGED_VIDEO: &str = "Clip 07.mp4";
pub const LOCKED_TEXT: &str = "Draft.txt";

pub const SCREEN: (u32, u32) = (1920, 1080);
pub const PAGE: (f64, f64) = (595.0, 842.0);
const PAGE_MARGIN: f64 = 64.0;
const LOG_BYTES: usize = 2_300_000;

/// Writes every sample document once into `dir`; returns `dir`.
pub fn prepare(dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let missing = |name: &str| !dir.join(name).exists();
    if missing(SCREENSHOT) {
        screenshot().save(dir.join(SCREENSHOT))?;
    }
    write_missing(dir, LOGO, LOGO_SVG.as_bytes())?;
    if missing(RELEASE_NOTES) {
        release_notes(&dir.join(RELEASE_NOTES), true)?;
    }
    if missing(INVOICE) {
        release_notes(&dir.join(INVOICE), false)?;
    }
    if missing(RECORDING) {
        ffmpeg(&[
            "-loop",
            "1",
            "-framerate",
            "30",
            "-i",
            &path_arg(dir, SCREENSHOT),
            "-t",
            "42",
            "-vf",
            "scale=1280:720,format=yuv420p",
            "-c:v",
            "libx264",
            "-tune",
            "stillimage",
            "-an",
            &path_arg(dir, RECORDING),
        ])?;
    }
    if missing(VOICE_MEMO) {
        ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=48000:cl=mono",
            "-t",
            "95",
            "-c:a",
            "libopus",
            &path_arg(dir, VOICE_MEMO),
        ])?;
    }
    write_missing(dir, README, README_MD.as_bytes())?;
    write_missing(dir, SOURCE, SOURCE_RS.as_bytes())?;
    write_missing(dir, NOTES, NOTES_TXT.as_bytes())?;
    if missing(LOG) {
        std::fs::write(dir.join(LOG), log_lines())?;
    }
    write_missing(dir, DAMAGED_IMAGE, b"\x89PNG\r\n\x1a\n-- truncated --")?;
    write_missing(dir, DAMAGED_PDF, b"%PDF-1.7\n%truncated download\n")?;
    write_missing(dir, DAMAGED_VIDEO, &[0x5a; 4096])?;
    if missing(LOCKED_TEXT) {
        let locked = dir.join(LOCKED_TEXT);
        std::fs::write(&locked, "")?;
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))?;
    }
    Ok(dir.to_path_buf())
}

fn write_missing(dir: &Path, name: &str, contents: &[u8]) -> Result<()> {
    let path = dir.join(name);
    if !path.exists() {
        std::fs::write(path, contents)?;
    }
    Ok(())
}

fn path_arg(dir: &Path, name: &str) -> String {
    dir.join(name).to_string_lossy().into_owned()
}

fn ffmpeg(args: &[&str]) -> Result<()> {
    let status = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error"])
        .args(args)
        .status()
        .context("ffmpeg is needed for the media samples")?;
    if !status.success() {
        bail!("ffmpeg failed: {args:?}");
    }
    Ok(())
}

/// A desktop screenshot: a gradient wallpaper, one app window and a dock.
fn screenshot() -> RgbaImage {
    let (width, height) = SCREEN;
    let mut canvas = RgbaImage::from_fn(width, height, |x, y| {
        let along = (x as f32 / width as f32) * 0.6 + (y as f32 / height as f32) * 0.4;
        let glow = 1.0 - ((x as f32 - 1500.0).hypot(y as f32 - 260.0) / 1400.0).min(1.0);
        let mix = |from: f32, to: f32| from + (to - from) * along + 70.0 * glow * glow;
        Rgba([
            mix(46.0, 236.0).min(255.0) as u8,
            mix(38.0, 132.0).min(255.0) as u8,
            mix(112.0, 112.0).min(255.0) as u8,
            255,
        ])
    });
    let window = [410.0, 190.0, 1100.0, 680.0];
    fill(&mut canvas, window, 18.0, [250, 250, 252, 250]);
    fill(
        &mut canvas,
        [410.0, 190.0, 1100.0, 52.0],
        18.0,
        [236, 236, 241, 255],
    );
    fill(
        &mut canvas,
        [410.0, 222.0, 1100.0, 20.0],
        0.0,
        [236, 236, 241, 255],
    );
    for (index, color) in [[255, 95, 87, 255], [254, 188, 46, 255], [40, 200, 64, 255]]
        .into_iter()
        .enumerate()
    {
        fill(
            &mut canvas,
            [432.0 + index as f32 * 22.0, 209.0, 13.0, 13.0],
            6.5,
            color,
        );
    }
    fill(
        &mut canvas,
        [418.0, 250.0, 236.0, 612.0],
        12.0,
        [241, 239, 248, 255],
    );
    for row in 0..7 {
        let top = 272.0 + row as f32 * 44.0;
        let tint = if row == 1 {
            [214, 205, 247, 255]
        } else {
            [241, 239, 248, 255]
        };
        fill(&mut canvas, [428.0, top - 8.0, 216.0, 36.0], 9.0, tint);
        fill(
            &mut canvas,
            [442.0, top + 2.0, 16.0, 16.0],
            5.0,
            [138, 120, 220, 255],
        );
        fill(
            &mut canvas,
            [468.0, top + 5.0, 90.0 + (row % 3) as f32 * 30.0, 10.0],
            5.0,
            [170, 168, 184, 255],
        );
    }
    fill(
        &mut canvas,
        [690.0, 276.0, 380.0, 26.0],
        8.0,
        [40, 38, 52, 255],
    );
    fill(
        &mut canvas,
        [690.0, 316.0, 560.0, 12.0],
        6.0,
        [190, 188, 202, 255],
    );
    for card in 0..3 {
        let left = 690.0 + card as f32 * 262.0;
        let hue = [
            [120, 104, 238, 255],
            [236, 132, 112, 255],
            [74, 178, 160, 255],
        ][card];
        fill(&mut canvas, [left, 360.0, 244.0, 200.0], 14.0, hue);
        fill(
            &mut canvas,
            [left, 576.0, 180.0, 12.0],
            6.0,
            [150, 148, 164, 255],
        );
        fill(
            &mut canvas,
            [left, 598.0, 120.0, 10.0],
            5.0,
            [200, 198, 210, 255],
        );
    }
    for line in 0..5 {
        let width = 740.0 - (line % 2) as f32 * 160.0;
        fill(
            &mut canvas,
            [690.0, 650.0 + line as f32 * 34.0, width, 12.0],
            6.0,
            [212, 210, 222, 255],
        );
    }
    fill(
        &mut canvas,
        [664.0, 990.0, 592.0, 72.0],
        24.0,
        [255, 255, 255, 120],
    );
    let icons = [
        [96, 165, 250, 255],
        [129, 140, 248, 255],
        [244, 114, 182, 255],
        [251, 191, 36, 255],
        [52, 211, 153, 255],
        [248, 113, 113, 255],
        [167, 139, 250, 255],
        [45, 212, 191, 255],
    ];
    for (index, color) in icons.into_iter().enumerate() {
        fill(
            &mut canvas,
            [680.0 + index as f32 * 71.0, 1002.0, 48.0, 48.0],
            13.0,
            color,
        );
    }
    canvas
}

/// Blends `color` into the rounded rectangle `[x, y, width, height]`.
fn fill(canvas: &mut RgbaImage, [x, y, width, height]: [f32; 4], radius: f32, color: [u8; 4]) {
    let alpha = f32::from(color[3]) / 255.0;
    for py in y as u32..(y + height) as u32 {
        for px in x as u32..(x + width) as u32 {
            let (cx, cy) = (px as f32 + 0.5, py as f32 + 0.5);
            let dx = (x + radius - cx).max(cx - (x + width - radius)).max(0.0);
            let dy = (y + radius - cy).max(cy - (y + height - radius)).max(0.0);
            let coverage = (radius - dx.hypot(dy) + 0.5).clamp(0.0, 1.0) * alpha;
            if coverage <= 0.0 {
                continue;
            }
            let pixel = canvas.get_pixel_mut(px, py);
            for channel in 0..3 {
                let blended = f32::from(pixel[channel]) * (1.0 - coverage)
                    + f32::from(color[channel]) * coverage;
                pixel[channel] = blended as u8;
            }
        }
    }
}

const SECTIONS: [(&str, &[&str]); 6] = [
    (
        "Overview",
        &[
            "CrownOS 1.0 brings the desktop, the window manager and every system app together on one toolkit. Apps share the same theme, the same Inter type scale and the same glass materials, so the system feels like one product from the lock screen to the last dialog.",
            "This release focuses on speed: windows open in under a frame, animations run on the compositor's clock, and every app renders on the GPU through Impeller.",
        ],
    ),
    (
        "Desktop and windows",
        &[
            "Windows snap to halves and quarters, remember their size per app, and round their corners concentrically with the content inside them. The dock shows running apps and lifts an icon while an app launches.",
            "The new overview groups windows by app. Drag a window onto another workspace, or press Super and an arrow key to move it without the mouse.",
        ],
    ),
    (
        "Preview",
        &[
            "Preview opens pictures, PDFs, videos, songs and text files instantly. Press Space on a file in Files to preview it, and the arrow keys to step through the folder.",
            "Markup tools let you sketch, highlight, draw shapes, add text and crop a screenshot before you share it. Preview saves the result back to the original file.",
            "In a PDF, Preview shows page thumbnails and the table of contents in the sidebar, and finds text across every page as you search.",
        ],
    ),
    (
        "Files",
        &[
            "Files remembers the view of every folder, shows previews for pictures and documents, and copies large folders in the background with a progress pill in the toolbar.",
        ],
    ),
    (
        "Settings",
        &[
            "Settings gathers appearance, displays, sound, network and accounts in one window. Changes apply live: switch to dark mode and every open app follows within a frame.",
            "Accent colors, the window corner radius and the wallpaper are shared with Preview, Files and every other app through the CrownOS configuration service.",
        ],
    ),
    (
        "Known issues",
        &[
            "Some HEIF pictures with depth maps open without their depth data. Very large PDFs may show blank pages for a moment while they render.",
            "Screen recordings longer than two hours cannot yet be trimmed in Preview.",
        ],
    ),
];

/// A multi-page document, with an outline when `outlined`.
fn release_notes(path: &Path, outlined: bool) -> Result<()> {
    let surface = PdfSurface::new(PAGE.0, PAGE.1, path)?;
    let cr = Cairo::new(&surface)?;
    let text_width = PAGE.0 - 2.0 * PAGE_MARGIN;
    let mut page = 1;
    let mut y = PAGE_MARGIN + 40.0;
    cr.select_font_face("Inter", FontSlant::Normal, FontWeight::Bold);
    cr.set_font_size(26.0);
    cr.set_source_rgb(0.08, 0.08, 0.1);
    cr.move_to(PAGE_MARGIN, y);
    cr.show_text("CrownOS 1.0 Release Notes")?;
    y += 26.0;
    cr.select_font_face("Inter", FontSlant::Normal, FontWeight::Normal);
    cr.set_font_size(12.0);
    cr.set_source_rgb(0.45, 0.45, 0.5);
    cr.move_to(PAGE_MARGIN, y);
    cr.show_text("November 2026")?;
    y += 36.0;
    for (title, paragraphs) in SECTIONS {
        if y > PAGE.1 - PAGE_MARGIN - 120.0 {
            cr.show_page()?;
            page += 1;
            y = PAGE_MARGIN + 20.0;
        }
        if outlined {
            let link = format!("page={page} pos=[{} {}]", PAGE_MARGIN, y - 20.0);
            surface.add_outline(cairo::PDF_OUTLINE_ROOT, title, &link, PdfOutline::empty())?;
        }
        cr.select_font_face("Inter", FontSlant::Normal, FontWeight::Bold);
        cr.set_font_size(17.0);
        cr.set_source_rgb(0.08, 0.08, 0.1);
        cr.move_to(PAGE_MARGIN, y);
        cr.show_text(title)?;
        y += 26.0;
        cr.select_font_face("Inter", FontSlant::Normal, FontWeight::Normal);
        cr.set_font_size(11.5);
        cr.set_source_rgb(0.2, 0.2, 0.24);
        for paragraph in paragraphs.iter() {
            for line in wrap(&cr, paragraph, text_width)? {
                if y > PAGE.1 - PAGE_MARGIN {
                    cr.show_page()?;
                    page += 1;
                    y = PAGE_MARGIN + 20.0;
                }
                cr.move_to(PAGE_MARGIN, y);
                cr.show_text(&line)?;
                y += 17.0;
            }
            y += 10.0;
        }
        y += 30.0;
    }
    cr.show_page()?;
    surface.finish();
    Ok(())
}

fn wrap(cr: &Cairo, paragraph: &str, width: f64) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in paragraph.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && cr.text_extents(&candidate)?.x_advance() > width {
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        } else {
            line = candidate;
        }
    }
    lines.push(line);
    Ok(lines)
}

fn log_lines() -> String {
    const MODULES: [&str; 5] = [
        "crownpositor::output",
        "crownpositor::input",
        "crownpositor::render",
        "crownpositor::xdg",
        "crownpositor::layer",
    ];
    let mut log = String::with_capacity(LOG_BYTES + 256);
    let mut sequence = 128_734_u64;
    while log.len() < LOG_BYTES {
        let seconds = sequence / 60;
        let module = MODULES[(sequence % 5) as usize];
        let millis = 3.1 + (sequence % 17) as f32 * 0.13;
        let _ = writeln!(
            log,
            "2026-11-11T{:02}:{:02}:{:02}.{:03}Z  INFO {module}: frame presented in {millis:.1} ms (output=DP-1, seq={sequence})",
            9 + seconds / 3600 % 10,
            seconds / 60 % 60,
            seconds % 60,
            sequence % 1000,
        );
        sequence += 1;
    }
    log
}

const LOGO_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#8b7bf4"/><stop offset="1" stop-color="#f08a5d"/></linearGradient></defs>
<rect x="32" y="32" width="448" height="448" rx="112" fill="url(#g)"/>
<path d="M136 336 L120 184 L196 248 L256 152 L316 248 L392 184 L376 336 Z" fill="#fff"/>
<rect x="136" y="352" width="240" height="32" rx="16" fill="#fff"/>
</svg>
"##;

const README_MD: &str = "# CrownOS Preview

Preview is the quick-look viewer of CrownOS. Press **Space** on a file in Files to see it without opening an app.

## What it opens

- Pictures: PNG, JPEG, WebP, GIF, HEIC and SVG
- Documents: PDF with thumbnails, contents and search
- Media: videos and songs, with a scrubber and volume
- Text: Markdown, source code and logs

## Markup

Click **Markup** to sketch, highlight, draw arrows and shapes, add text or crop. Press Enter to save and Escape to cancel.

> Edits are written back to the original file, so keep a copy if you need the untouched picture.

```
crownos-preview --edit ~/Pictures/Screenshot.png
```

---

Preview follows the system appearance and accent color.
";

const SOURCE_RS: &str = r#"use crownui::prelude::{Rect, Size};

/// How a wallpaper fills an output of a different shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fit {
    #[default]
    Fill,
    Fit,
    Center,
}

impl Fit {
    /// Where an image of `image` size lands on an output of `output` size.
    pub fn place(self, image: Size, output: Size) -> Rect {
        let scale = match self {
            Self::Fill => (output.width / image.width).max(output.height / image.height),
            Self::Fit => (output.width / image.width).min(output.height / image.height),
            Self::Center => 1.0,
        };
        let size = Size::new(image.width * scale, image.height * scale);
        let x = (output.width - size.width) / 2.0;
        let y = (output.height - size.height) / 2.0;
        Rect::from_origin_size((x, y), size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_covers_the_output() {
        let placed = Fit::Fill.place(Size::new(400.0, 300.0), Size::new(1920.0, 1080.0));
        assert!(placed.width() >= 1920.0 && placed.height() >= 1080.0);
    }
}
"#;

const NOTES_TXT: &str = "Design review - 11 November 2026

Attendees: design, shell, apps

1. Preview
   - Markup toolbar wraps on narrow windows; keep Done always visible.
   - Zoom pill should hide after two seconds without pointer movement.
   - PDF search: show the match count next to the field.

2. Files
   - Space opens Preview; arrow keys step through the selection.
   - Copy progress moves into the toolbar pill.

3. Settings
   - Accent color picker gets two more swatches.

Next review: 18 November, same time.
";
