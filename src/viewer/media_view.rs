use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crownui::ext::{ImageId, Key, NamedKey};
use crownui::kit::{SliderOptions, icon, icon_button, icons, slider, use_theme};
use crownui::prelude::*;

use super::chrome::{content_card, file_name, notice, title_bar, window_frame};
use super::shortcuts::on_shortcut;
use crate::document::DocumentKind;
use crate::media::{Player, PlayerEvent, VideoFrame};
use crate::util::fit::{centered, fit_scale};
use crate::util::mailbox::mailbox;
use crate::util::timecode::timecode;
use crate::util::timer::after;

const TICK: Duration = Duration::from_millis(250);
const SEEK_THRESHOLD: f32 = 0.002;
const AUDIO_ICON_SIZE: f32 = 96.0;
const VOLUME_SLIDER_WIDTH: f32 = 96.0;

#[derive(Clone, Copy)]
struct Playback {
    player: Store<Option<Player>>,
    frame: Signal<Option<(ImageId, Size)>>,
    playing: Signal<bool>,
    ended: Signal<bool>,
    position: Signal<Duration>,
    duration: Signal<Duration>,
    progress: Signal<f32>,
    reported_progress: Store<f32>,
    volume: Signal<f32>,
    muted: Signal<bool>,
    failure: Signal<Option<String>>,
}

impl Playback {
    fn new(cx: &mut Cx) -> Self {
        Self {
            player: cx.store(None),
            frame: cx.signal(None),
            playing: cx.signal(true),
            ended: cx.signal(false),
            position: cx.signal(Duration::ZERO),
            duration: cx.signal(Duration::ZERO),
            progress: cx.signal(0.0),
            reported_progress: cx.store(0.0),
            volume: cx.signal(1.0),
            muted: cx.signal(false),
            failure: cx.signal(None),
        }
    }

    fn with_player(self, rt: &Runtime, action: impl FnOnce(&Player)) {
        self.player.with(rt, |player| player.as_ref().map(action));
    }

    fn toggle(self, cx: &mut Cx) {
        let playing = !self.playing.get(cx);
        if playing && self.ended.get(cx) {
            self.with_player(cx, |player| player.seek(Duration::ZERO));
            self.ended.set(cx, false);
        }
        self.with_player(cx, |player| player.set_playing(playing));
        self.playing.set(cx, playing);
    }

    fn show_frame(self, cx: &mut Cx, frame: VideoFrame) {
        let size = Size::new(frame.width as f32, frame.height as f32);
        let image = cx.register_image(frame.width, frame.height, frame.rgba);
        cx.release_image_on_cleanup(image);
        if let Some((previous, _)) = self.frame.get(cx) {
            cx.release_image(previous);
        }
        self.frame.set(cx, Some((image, size)));
    }

    fn tick(self, cx: &mut Cx) {
        let mut sampled = (None, None, None);
        self.with_player(cx, |player| {
            sampled = (player.position(), player.duration(), player.poll())
        });
        let (position, duration, event) = sampled;
        if let Some(duration) = duration {
            self.duration.set(cx, duration);
        }
        if let Some(position) = position {
            self.position.set(cx, position);
            let total = self.duration.get(cx).as_secs_f32();
            if total > 0.0 {
                let progress = position.as_secs_f32() / total;
                self.reported_progress
                    .update(cx, |reported| *reported = progress);
                self.progress.set(cx, progress);
            }
        }
        match event {
            Some(PlayerEvent::Ended) => {
                self.playing.set(cx, false);
                self.ended.set(cx, true);
            }
            Some(PlayerEvent::Failed(message)) => self
                .failure
                .set(cx, Some(format!("Cannot play this file: {message}"))),
            None => {}
        }
    }
}

pub fn media_view(path: PathBuf, kind: DocumentKind) -> impl View {
    component(move |cx| {
        let playback = Playback::new(cx);
        let in_flight = Arc::new(AtomicBool::new(false));
        let released = in_flight.clone();
        let frames = mailbox(cx, move |cx, frame| {
            playback.show_frame(cx, frame);
            let released = released.clone();
            cx.request_animation_frame(move |_, _| released.store(false, Ordering::Release));
        });
        match Player::open(&path, frames, in_flight) {
            Ok(player) => playback.player.update(cx, |slot| *slot = Some(player)),
            Err(error) => playback
                .failure
                .set(cx, Some(format!("Cannot play this file: {error}"))),
        }
        cx.on_cleanup(move |rt| playback.player.update(rt, |slot| *slot = None));
        keep_ticking(cx, playback);

        cx.effect(move |rt| {
            let progress = playback.progress.get(rt);
            let reported = playback.reported_progress.with(rt, |reported| *reported);
            if (progress - reported).abs() > SEEK_THRESHOLD {
                let target = playback.duration.get(rt).mul_f32(progress.clamp(0.0, 1.0));
                playback
                    .reported_progress
                    .update(rt, |reported| *reported = progress);
                playback.with_player(rt, |player| player.seek(target));
            }
        });
        cx.effect(move |rt| {
            let (volume, muted) = (playback.volume.get(rt), playback.muted.get(rt));
            playback.with_player(rt, |player| {
                player.set_volume(f64::from(volume));
                player.set_muted(muted);
            });
        });

        on_shortcut(cx, move |cx, event| match event.key {
            Key::Named(NamedKey::Space) | Key::Character('k') => {
                playback.toggle(cx);
                true
            }
            Key::Character('m') => {
                playback.muted.update(cx, |muted| *muted = !*muted);
                true
            }
            _ => false,
        });

        let stage = if kind == DocumentKind::Audio {
            audio_artwork(file_name(&path)).boxed()
        } else {
            video_canvas(playback).boxed()
        };
        window_frame(
            title_bar((), file_name(&path), (), ()),
            content_card((
                show(
                    bind(move |rt| playback.failure.with(rt, Option::is_some)),
                    move || notice(bind(move |rt| playback.failure.get(rt).unwrap_or_default())),
                    move || hstack(()),
                ),
                vstack((stage,)).grow(1.0),
                transport(playback),
            )),
        )
    })
}

fn keep_ticking(cx: &mut Cx, playback: Playback) {
    after(cx, TICK, move |cx| {
        playback.tick(cx);
        keep_ticking(cx, playback);
    });
}

fn video_canvas(playback: Playback) -> impl View {
    canvas(move |rt, size, list| {
        if let Some((image, natural)) = playback.frame.get(rt) {
            list.image(
                image,
                centered(natural, fit_scale(natural, size), size, Point::ZERO),
            );
        }
    })
    .grow(1.0)
    .bg(BLACK)
    .on_click(move |cx, _| playback.toggle(cx))
}

fn audio_artwork(title: String) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        vstack((
            icon(icons::FILE_MUSIC).size(AUDIO_ICON_SIZE),
            text(title)
                .font_semibold()
                .text_size(16.0)
                .text_color(bind(move |rt| theme.with(rt, |theme| theme.text.primary))),
        ))
        .grow(1.0)
        .gap(16.0)
        .items_center()
        .justify_center()
    })
}

fn transport(playback: Playback) -> impl View {
    component(move |cx| {
        let theme = use_theme(cx);
        let muted_text = move |rt: &mut Runtime| theme.with(rt, |theme| theme.text.muted);
        hstack((
            show(
                bind(move |rt| playback.playing.get(rt)),
                move || {
                    icon_button(icons::PAUSE, "Pause").on_click(move |cx, _| playback.toggle(cx))
                },
                move || icon_button(icons::PLAY, "Play").on_click(move |cx, _| playback.toggle(cx)),
            ),
            text(bind(move |rt| timecode(playback.position.get(rt))))
                .text_size(12.0)
                .min_w(48.0)
                .text_color(bind(muted_text)),
            slider(playback.progress).range(0.0, 1.0).grow(1.0),
            text(bind(move |rt| timecode(playback.duration.get(rt))))
                .text_size(12.0)
                .min_w(48.0)
                .text_color(bind(muted_text)),
            show(
                bind(move |rt| playback.muted.get(rt)),
                move || {
                    icon_button(icons::VOLUME_X, "Unmute")
                        .on_click(move |cx, _| playback.muted.set(cx, false))
                },
                move || {
                    icon_button(icons::VOLUME_2, "Mute")
                        .on_click(move |cx, _| playback.muted.set(cx, true))
                },
            ),
            slider(playback.volume)
                .range(0.0, 1.0)
                .w(VOLUME_SLIDER_WIDTH),
        ))
        .px(12.0)
        .py(8.0)
        .gap(10.0)
        .items_center()
        .bg(bind(move |rt| theme.with(rt, |theme| theme.surface.raised)))
    })
}
