use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use capture::CaptureApp;
use crownos_ipc::ServiceBuilder;
use crowntest::{Role, role};
use crownui::AppConfig;

use crate::app_config::app_config;
use crate::fixtures::{PAGE, SCREEN};
use crate::launch::{Mode, Request};
use crate::tasks::{Finished, TaskGuard, Tasks};
use crate::viewer;

const SOCKET_DIR_PREFIX: &str = "/tmp/claude-1000/crownos-preview-capture";
static MOUNT_COUNT: AtomicUsize = AtomicUsize::new(0);
const FRAME_MS: u64 = 16;
const DRAG_STEPS: usize = 10;
const SWATCH_EDGE: f32 = 20.0;
const SWATCH_PITCH: f32 = 26.0;
const SWATCH_COUNT: usize = 6;
const PAGE_MARGIN: f32 = 24.0;

/// What every mount shares: the task runtime and the app's real window config.
pub struct Studio {
    runtime: tokio::runtime::Runtime,
    config: AppConfig,
}

/// One mounted Preview window, its spawned tasks run by the studio's runtime.
pub struct Session {
    pub app: CaptureApp,
    finished: Receiver<Finished>,
    _tasks: TaskGuard,
}

impl Studio {
    pub fn new() -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        Ok(Self {
            runtime,
            config: app_config(),
        })
    }

    pub fn open(&self, samples: &Path, file: &str, mode: Mode) -> Result<Session> {
        let path: PathBuf = samples.join(file);
        let request = Request::new(&path, mode)
            .with_context(|| format!("{} cannot be previewed", path.display()))?;
        self.mount(Some(request))
    }

    pub fn open_empty(&self) -> Result<Session> {
        self.mount(None)
    }

    fn mount(&self, request: Option<Request>) -> Result<Session> {
        let mount_index = MOUNT_COUNT.fetch_add(1, Ordering::Relaxed);
        let server = ServiceBuilder::new(crownos_preview::SERVICE)
            .directory(format!(
                "{SOCKET_DIR_PREFIX}-{}-{mount_index}",
                std::process::id()
            ))
            .build()?;
        let mut app = capture::mount_app(viewer::root(request, server), &self.config, None)?;
        let (tasks, finished, guard) = Tasks::new(self.runtime.handle().clone());
        app.ui().set_task_executor(Box::new(tasks));
        Ok(Session {
            app,
            finished,
            _tasks: guard,
        })
    }
}

impl Session {
    /// Lets background work (decoding, rendering, playback) land for `millis` of real time.
    pub fn pump_ms(&mut self, millis: u64) {
        let deadline = Instant::now() + Duration::from_millis(millis);
        while Instant::now() < deadline {
            while let Ok((id, output)) = self.finished.try_recv() {
                self.app.ui().complete_task(id, output);
            }
            self.app.advance_ms(FRAME_MS);
            std::thread::sleep(Duration::from_millis(FRAME_MS / 2));
        }
    }

    pub fn choose_tool(&mut self, label: &str) {
        self.app.click(role(Role::RadioButton).name(label));
    }

    /// Picks the markup palette's `index`th color.
    pub fn choose_color(&mut self, index: usize) -> Result<()> {
        let swatch = self
            .swatches()?
            .get(index)
            .copied()
            .context("no such swatch")?;
        self.tap(swatch);
        Ok(())
    }

    pub fn drag_on_image(&mut self, from: (f32, f32), to: (f32, f32)) -> Result<()> {
        let (from, to) = (self.image_point(from)?, self.image_point(to)?);
        self.drag_through(&interpolate(from, to));
        Ok(())
    }

    pub fn stroke_on_image(&mut self, fractions: &[(f32, f32)]) -> Result<()> {
        let points = fractions
            .iter()
            .map(|fraction| self.image_point(*fraction))
            .collect::<Result<Vec<_>>>()?;
        let path: Vec<_> = points
            .windows(2)
            .flat_map(|pair| interpolate(pair[0], pair[1]))
            .collect();
        self.drag_through(&path);
        Ok(())
    }

    pub fn click_on_image(&mut self, fraction: (f32, f32)) -> Result<()> {
        let point = self.image_point(fraction)?;
        self.tap(point);
        Ok(())
    }

    /// Drags across the first PDF page between two points given in PDF points.
    pub fn drag_on_page(&mut self, from: (f32, f32), to: (f32, f32)) -> Result<()> {
        let [x, y, width, _] = self.canvas()?;
        let scale = (width - 2.0 * PAGE_MARGIN) / PAGE.0 as f32;
        let at =
            |(px, py): (f32, f32)| (x + PAGE_MARGIN + px * scale, y + PAGE_MARGIN + py * scale);
        self.drag_through(&interpolate(at(from), at(to)));
        Ok(())
    }

    fn tap(&mut self, (x, y): (f32, f32)) {
        self.app
            .pointer_move(x, y)
            .pointer_down(x, y)
            .pointer_up(x, y);
        self.pump_ms(100);
    }

    fn drag_through(&mut self, points: &[(f32, f32)]) {
        let Some((&(x, y), rest)) = points.split_first() else {
            return;
        };
        self.app.pointer_move(x, y).pointer_down(x, y);
        for &(x, y) in rest {
            self.app.pointer_move(x, y);
        }
        if let Some(&(x, y)) = points.last() {
            self.app.pointer_up(x, y);
        }
        self.pump_ms(100);
    }

    /// The window point over `fraction` of the screenshot, as the stage fits it.
    fn image_point(&mut self, (fx, fy): (f32, f32)) -> Result<(f32, f32)> {
        let [x, y, width, height] = self.canvas()?;
        let (image_width, image_height) = (SCREEN.0 as f32, SCREEN.1 as f32);
        let scale = (width / image_width).min(height / image_height).min(1.0);
        let left = x + (width - image_width * scale) / 2.0;
        let top = y + (height - image_height * scale) / 2.0;
        Ok((
            left + fx * image_width * scale,
            top + fy * image_height * scale,
        ))
    }

    fn canvas(&mut self) -> Result<[f32; 4]> {
        let layout = self.app.ui().inspect_layout();
        layout
            .nodes
            .iter()
            .rev()
            .find(|node| node.path.contains("measured_canvas"))
            .map(|node| node.rect)
            .with_context(|| {
                let paths: Vec<_> = layout.nodes.iter().map(|node| node.path.as_str()).collect();
                format!("no measured canvas in {paths:#?}")
            })
    }

    /// Centers of the palette swatches: a run of equal circles at a fixed pitch.
    fn swatches(&mut self) -> Result<Vec<(f32, f32)>> {
        let layout = self.app.ui().inspect_layout();
        let mut circles: Vec<[f32; 4]> = layout
            .nodes
            .iter()
            .map(|node| node.rect)
            .filter(|rect| {
                (rect[2] - SWATCH_EDGE).abs() < 0.5 && (rect[3] - SWATCH_EDGE).abs() < 0.5
            })
            .collect();
        circles.sort_by(|a, b| a[1].total_cmp(&b[1]).then(a[0].total_cmp(&b[0])));
        circles.dedup_by(|a, b| (a[0] - b[0]).abs() < 0.5 && (a[1] - b[1]).abs() < 0.5);
        circles
            .windows(SWATCH_COUNT)
            .find(|run| {
                run.windows(2).all(|pair| {
                    (pair[1][0] - pair[0][0] - SWATCH_PITCH).abs() < 0.5
                        && (pair[1][1] - pair[0][1]).abs() < 0.5
                })
            })
            .map(|run| {
                run.iter()
                    .map(|rect| (rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0))
                    .collect()
            })
            .with_context(|| format!("no swatch row among {circles:?}"))
    }
}

fn interpolate(from: (f32, f32), to: (f32, f32)) -> Vec<(f32, f32)> {
    (0..=DRAG_STEPS)
        .map(|step| {
            let t = step as f32 / DRAG_STEPS as f32;
            (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
        })
        .collect()
}
