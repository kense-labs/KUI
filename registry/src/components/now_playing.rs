//! A host-controlled compact player that expands into a full playback surface.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::{button, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
#[derive(Clone)]
pub struct Track {
    pub id: SharedString,
    pub title: SharedString,
    pub artist: SharedString,
    pub duration: f32,
    pub artwork: ImageSource,
}
impl Track {
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        artist: impl Into<SharedString>,
        duration: f32,
        artwork: impl Into<ImageSource>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            artist: artist.into(),
            duration: if duration.is_finite() {
                duration.max(0.)
            } else {
                0.
            },
            artwork: artwork.into(),
        }
    }
}
fn safe_duration(duration: f32) -> f32 {
    if duration.is_finite() {
        duration.max(0.)
    } else {
        0.
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerEvent {
    Play(bool),
    Track(usize),
    Seek(f32),
    Expanded(bool),
}
pub struct NowPlaying {
    label: SharedString,
    tracks: Vec<Track>,
    index: usize,
    position: f32,
    playing: bool,
    expanded: bool,
    animation: Slide,
    seek_bounds: TriggerBounds,
    container_width: Rc<Cell<Pixels>>,
}
impl EventEmitter<PlayerEvent> for NowPlaying {}
impl NowPlaying {
    pub fn new(label: impl Into<SharedString>, tracks: Vec<Track>) -> Self {
        Self {
            label: label.into(),
            tracks,
            index: 0,
            position: 0.,
            playing: false,
            expanded: false,
            animation: Slide::new(0.),
            seek_bounds: TriggerBounds::default(),
            container_width: Default::default(),
        }
    }
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        self.playing = playing;
        cx.notify();
    }
    pub fn set_position(&mut self, position: f32, cx: &mut Context<Self>) {
        let duration = self
            .tracks
            .get(self.index)
            .map_or(0., |t| safe_duration(t.duration));
        self.position = if position.is_finite() {
            position.clamp(0., duration)
        } else {
            0.
        };
        cx.notify();
    }
    pub fn set_index(&mut self, index: usize, cx: &mut Context<Self>) {
        if !self.tracks.is_empty() {
            self.index = index.min(self.tracks.len() - 1);
            self.position = 0.;
            cx.notify();
        }
    }
    fn request_track(&self, delta: isize, cx: &mut Context<Self>) {
        if delta < 0 && self.position > 3. {
            cx.emit(PlayerEvent::Seek(0.));
            return;
        }
        if !self.tracks.is_empty() {
            let next =
                (self.index as isize + delta).rem_euclid(self.tracks.len() as isize) as usize;
            cx.emit(PlayerEvent::Track(next));
        }
    }
    pub fn set_expanded(&mut self, expanded: bool, cx: &mut Context<Self>) {
        if self.expanded == expanded {
            return;
        }
        self.expanded = expanded;
        self.animation.duration(f32::from(expanded), 0.4, cx);
        cx.emit(PlayerEvent::Expanded(expanded));
        cx.notify();
    }
}
fn timestamp(seconds: f32) -> String {
    let value = seconds.max(0.) as u64;
    format!("{}:{:02}", value / 60, value % 60)
}
// Measure the available parent width independently of the component's inner layout.
fn constrained(
    content: impl IntoElement,
    width: Rc<Cell<Pixels>>,
    maximum: Pixels,
) -> impl IntoElement {
    div()
        .relative()
        .w_full()
        .max_w(maximum)
        .child(
            canvas(
                move |bounds, w, _| {
                    if (width.get() - bounds.size.width).abs() > px(0.5) {
                        width.set(bounds.size.width);
                        w.request_animation_frame();
                    }
                },
                |_, _, _, _| (),
            )
            .absolute()
            .inset_0(),
        )
        .child(content)
}
impl Render for NowPlaying {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let t = self.animation.get(w, cx).clamp(0., 1.);
        let measured = self.container_width.get();
        let width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width - px(32.)
        }
        .max(px(1.))
        .min(px(336.));
        let mut root = div()
            .id("now-playing")
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && s.expanded {
                    s.set_expanded(false, cx);
                    cx.stop_propagation();
                }
            }))
            .role(Role::Group)
            .aria_label(self.label.clone())
            .w(width)
            .relative()
            .h(px(64. + 398. * t))
            .pt(px(10. + 2. * t))
            .px(px(10. + 2. * t))
            .pb(px(10. + 4. * t))
            .overflow_hidden()
            .rounded(crate::foundation::surface_radius(cx))
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap_3();
        let Some(track) = self.tracks.get(self.index) else {
            return constrained(root, self.container_width.clone(), px(336.));
        };
        let title = track.title.clone();
        let artist = track.artist.clone();
        let duration = safe_duration(track.duration);
        let image = track.artwork.clone();
        let expanded = self.expanded;
        let playing = self.playing;
        let mini = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                img(image.clone())
                    .size(px(44.))
                    .object_fit(ObjectFit::Cover),
            )
            .child(
                div()
                    .id("player-expand")
                    .min_w_0()
                    .h(px(44.))
                    .px(px(2.))
                    .flex_1()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(cx.listener(|s, _, _, cx| {
                        if !s.expanded {
                            s.set_expanded(true, cx);
                        }
                    }))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(title.clone()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .line_height(px(17.))
                            .text_color(p.secondary)
                            .child(artist.clone()),
                    ),
            )
            .child(
                face(
                    "player-play",
                    if playing {
                        tr(cx, "Pause", "暂停")
                    } else {
                        tr(cx, "Play", "播放")
                    },
                    40. + 12. * t,
                    20.,
                    Tone::Surface,
                    cx,
                )
                .size(px(40. + 12. * t))
                .p_0()
                .bg(if expanded { p.ink } else { p.tray })
                .child(icon(
                    if playing { Glyph::Pause } else { Glyph::Play },
                    16.,
                    if expanded { p.inverse } else { p.ink },
                ))
                .on_click(cx.listener(move |_, _, _, cx| cx.emit(PlayerEvent::Play(!playing)))),
            )
            .child({
                let scales = if playing {
                    [0.7, 1.0, 0.55]
                } else {
                    [0.3, 0.6, 0.42]
                };
                let mut levels = div()
                    .h(px(14.))
                    .mr(px(4.))
                    .flex()
                    .items_end()
                    .gap(px(2.))
                    .opacity(if playing { 1. } else { 0.55 });
                for scale in scales {
                    levels =
                        levels.child(div().w(px(3.)).h(px(14. * scale)).rounded(px(1.)).bg(p.ink));
                }
                levels
            });
        if !expanded {
            root = root.child(mini);
        }
        if !expanded {
            root = root.child(
                div()
                    .absolute()
                    .bottom_0()
                    .left(px(18.))
                    .right(px(18.))
                    .h(px(2.))
                    .child(
                        div()
                            .h_full()
                            .w(relative(if duration > 0. {
                                (self.position / duration).clamp(0., 1.)
                            } else {
                                0.
                            }))
                            .rounded(px(1.))
                            .bg(p.ink),
                    ),
            );
        }
        if t > 0.001 {
            let bounds = self.seek_bounds.clone();
            let position = self.position;
            let ratio = if duration > 0. {
                position / duration
            } else {
                0.
            };
            let mut waveform = div()
                .w_full()
                .h(px(36.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(2.));
            for index in 0..54 {
                let shape = ((index as f32 * 1.7).sin().abs() * 0.65 + 0.25) * 36.;
                waveform = waveform.child(div().flex_1().h(px(shape.max(3.))).rounded(px(2.)).bg(
                    if (index as f32 / 54.) <= ratio {
                        p.ink
                    } else {
                        p.ink.opacity(0.16)
                    },
                ));
            }
            root = root.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .relative()
                            .child(img(image).w_full().h(px(192.)).object_fit(ObjectFit::Cover))
                            .child(
                                button("player-collapse", tr(cx, "Collapse", "收起"), cx)
                                    .absolute()
                                    .top(px(10.))
                                    .right(px(10.))
                                    .size(px(34.))
                                    .rounded_full()
                                    .p_0()
                                    .bg(black().opacity(0.42))
                                    .hover(|s| s.bg(black().opacity(0.56)))
                                    .child(icon(Glyph::Down, 16., white()))
                                    .on_click(cx.listener(|s, _, _, cx| s.set_expanded(false, cx))),
                            ),
                    )
                    .child(
                        div()
                            .min_h(px(46.))
                            .px(px(4.))
                            .min_w_0()
                            .truncate()
                            .text_size(px(18.))
                            .line_height(px(25.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(title),
                    )
                    .child(
                        div()
                            .px(px(4.))
                            .min_w_0()
                            .truncate()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(artist),
                    )
                    .child(
                        div()
                            .id("player-seek")
                            .role(Role::Slider)
                            .aria_label(tr(cx, "Playback position", "播放进度"))
                            .aria_value(format!(
                                "{} / {}",
                                timestamp(position),
                                timestamp(duration)
                            ))
                            .tab_index(0)
                            .w_full()
                            .h(px(36.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .child(self.seek_bounds.measure())
                            .child(waveform)
                            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                                let d = s
                                    .tracks
                                    .get(s.index)
                                    .map_or(0., |t| safe_duration(t.duration));
                                let n = match e.keystroke.key.as_str() {
                                    "left" | "down" => s.position - 5.,
                                    "right" | "up" => s.position + 5.,
                                    "pageup" => s.position + 30.,
                                    "pagedown" => s.position - 30.,
                                    "home" => 0.,
                                    "end" => d,
                                    _ => return,
                                };
                                cx.emit(PlayerEvent::Seek(n.clamp(0., d)));
                                cx.stop_propagation();
                            }))
                            .on_click(cx.listener(move |_, e: &ClickEvent, _, cx| {
                                let b = bounds.get();
                                let v = (f32::from(e.position().x - b.origin.x)
                                    / f32::from(b.size.width).max(1.))
                                .clamp(0., 1.)
                                    * duration;
                                cx.emit(PlayerEvent::Seek(v));
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .text_size(px(12.))
                            .line_height(px(17.))
                            .text_color(p.faint)
                            .child(timestamp(position))
                            .child(timestamp(duration)),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .items_center()
                            .gap(px(28.))
                            .child(
                                face(
                                    "player-prev",
                                    tr(cx, "Previous", "上一首"),
                                    44.,
                                    22.,
                                    Tone::Clear,
                                    cx,
                                )
                                .size(px(44.))
                                .rounded_full()
                                .p_0()
                                .hover(|s| s.bg(p.tray))
                                .child("‹")
                                .on_click(cx.listener(|s, _, _, cx| s.request_track(-1, cx))),
                            )
                            .child(
                                face(
                                    "player-play-large",
                                    if playing {
                                        tr(cx, "Pause", "暂停")
                                    } else {
                                        tr(cx, "Play", "播放")
                                    },
                                    52.,
                                    26.,
                                    Tone::Surface,
                                    cx,
                                )
                                .size(px(52.))
                                .rounded_full()
                                .p_0()
                                .bg(p.ink)
                                .child(icon(
                                    if playing { Glyph::Pause } else { Glyph::Play },
                                    18.,
                                    p.inverse,
                                ))
                                .on_click(cx.listener(
                                    move |_, _, _, cx| cx.emit(PlayerEvent::Play(!playing)),
                                )),
                            )
                            .child(
                                face(
                                    "player-next",
                                    tr(cx, "Next", "下一首"),
                                    44.,
                                    22.,
                                    Tone::Clear,
                                    cx,
                                )
                                .size(px(44.))
                                .rounded_full()
                                .p_0()
                                .hover(|s| s.bg(p.tray))
                                .child("›")
                                .on_click(cx.listener(|s, _, _, cx| s.request_track(1, cx))),
                            ),
                    ),
            );
        }
        constrained(root, self.container_width.clone(), px(336.))
    }
}
