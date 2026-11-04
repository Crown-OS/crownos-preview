use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app::{AppSink, AppSinkCallbacks};
use gstreamer_video::{VideoCapsBuilder, VideoFormat, VideoFrameExt, VideoFrameRef, VideoInfo};
use tokio::sync::mpsc::UnboundedSender;

use crate::util::pixels::packed_rows;

/// One decoded video frame in RGBA8.
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub enum PlayerEvent {
    Ended,
    Failed(String),
}

/// A `playbin` pipeline; GStreamer picks hardware decoders when they are installed.
pub struct Player {
    playbin: gst::Element,
}

impl Player {
    /// Starts playing `path`. Frames are handed to `frames` one at a time:
    /// while one is in flight to the UI, newer ones are dropped.
    pub fn open(
        path: &Path,
        frames: UnboundedSender<VideoFrame>,
        in_flight: Arc<AtomicBool>,
    ) -> Result<Self> {
        gst::init()?;
        let uri = gst::glib::filename_to_uri(path, None)?;
        let sink = AppSink::builder()
            .caps(&VideoCapsBuilder::new().format(VideoFormat::Rgba).build())
            .max_buffers(1)
            .drop(true)
            .build();
        sink.set_callbacks(
            AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    if in_flight.swap(true, Ordering::AcqRel) {
                        return Ok(gst::FlowSuccess::Ok);
                    }
                    let frame = copy_frame(&sample).ok_or(gst::FlowError::Error)?;
                    frames.send(frame).map_err(|_| gst::FlowError::Flushing)?;
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );
        let playbin = gst::ElementFactory::make("playbin")
            .property("uri", uri.as_str())
            .property("video-sink", &sink)
            .build()
            .context("GStreamer's playbin is not installed")?;
        playbin.set_state(gst::State::Playing)?;
        Ok(Self { playbin })
    }

    pub fn set_playing(&self, playing: bool) {
        let state = if playing {
            gst::State::Playing
        } else {
            gst::State::Paused
        };
        let _ = self.playbin.set_state(state);
    }

    pub fn position(&self) -> Option<Duration> {
        self.playbin
            .query_position::<gst::ClockTime>()
            .map(Duration::from)
    }

    pub fn duration(&self) -> Option<Duration> {
        self.playbin
            .query_duration::<gst::ClockTime>()
            .map(Duration::from)
    }

    pub fn seek(&self, position: Duration) {
        let target = gst::ClockTime::from_nseconds(position.as_nanos() as u64);
        let _ = self
            .playbin
            .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, target);
    }

    pub fn set_volume(&self, volume: f64) {
        self.playbin.set_property("volume", volume.clamp(0.0, 1.0));
    }

    pub fn set_muted(&self, muted: bool) {
        self.playbin.set_property("mute", muted);
    }

    /// The next end-of-stream or error waiting on the bus.
    pub fn poll(&self) -> Option<PlayerEvent> {
        let bus = self.playbin.bus()?;
        while let Some(message) = bus.pop() {
            match message.view() {
                gst::MessageView::Eos(_) => return Some(PlayerEvent::Ended),
                gst::MessageView::Error(error) => {
                    return Some(PlayerEvent::Failed(error.error().to_string()));
                }
                _ => {}
            }
        }
        None
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.playbin.set_state(gst::State::Null);
    }
}

fn copy_frame(sample: &gst::Sample) -> Option<VideoFrame> {
    let info = VideoInfo::from_caps(sample.caps()?).ok()?;
    let frame = VideoFrameRef::from_buffer_ref_readable(sample.buffer()?, &info).ok()?;
    let (width, height) = (frame.width(), frame.height());
    let stride = usize::try_from(frame.plane_stride()[0]).ok()?;
    let row_bytes = width as usize * 4;
    let rgba = packed_rows(
        frame.plane_data(0).ok()?,
        stride,
        row_bytes,
        height as usize,
    );
    Some(VideoFrame {
        width,
        height,
        rgba,
    })
}
