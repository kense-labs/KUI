//! A read-aloud controller. Synthesis and playback belong to the host.
use crate::primitives::{
    brand_mark::brand_mark,
    family::{Paint, Slide, Tone, face},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use unicode_segmentation::UnicodeSegmentation;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeechPhase {
    Idle,
    Preparing,
    Playing,
    Paused,
    Finished,
    Error,
}
#[derive(Clone, Debug, PartialEq)]
pub enum SpeechEvent {
    Start,
    Pause,
    Resume,
    Retry,
    Stop,
    Seek(f32),
    Speed(f32),
    Voice(SharedString),
}
#[derive(Clone)]
pub struct SpeechVoice {
    pub id: SharedString,
    pub label: SharedString,
}
impl SpeechVoice {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}
fn blocked_by_pending(pending: bool, event: &SpeechEvent) -> bool {
    pending && !matches!(event, SpeechEvent::Stop)
}
fn is_live_session(active: bool, current: u64, incoming: u64) -> bool {
    active && current == incoming
}
fn is_current_operation(
    active: bool,
    current_session: u64,
    session: u64,
    current_operation: u64,
    operation: u64,
) -> bool {
    is_live_session(active, current_session, session) && current_operation == operation
}
fn seek_at(bounds: Bounds<Pixels>, x: Pixels, duration: f32) -> f32 {
    let fraction =
        (f32::from(x - bounds.left()) / f32::from(bounds.size.width).max(1.)).clamp(0., 1.);
    fraction * duration.max(0.)
}
pub struct SpeechPlayer {
    label: SharedString,
    text: SharedString,
    model: Option<SharedString>,
    show_transport: bool,
    phase: SpeechPhase,
    word: Option<usize>,
    hovered_word: Option<usize>,
    seek_hover: bool,
    mark_x: Slide,
    mark_width: Slide,
    mark_y: f32,
    mark_height: f32,
    mark_target: Option<(f32, f32, f32)>,
    position: f32,
    duration: f32,
    speed: f32,
    voices: Vec<SpeechVoice>,
    voice: usize,
    voice_menu: bool,
    menu_index: usize,
    menu_focus: Option<FocusHandle>,
    trigger_focus: Option<FocusHandle>,
    error: SharedString,
    expanded: Slide,
    pending: bool,
    session_id: u64,
    session_active: bool,
    operation_id: u64,
    seek_bounds: TriggerBounds,
    seek_dragging: bool,
    seek_preview: Option<f32>,
    touch_axis: Rc<Cell<u8>>,
    touch_start_x: Rc<Cell<Pixels>>,
    touch_dx: Rc<Cell<Pixels>>,
    touch_dy: Rc<Cell<Pixels>>,
}
impl EventEmitter<SpeechEvent> for SpeechPlayer {}
impl SpeechPlayer {
    pub fn new(label: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            text: text.into(),
            model: None,
            show_transport: false,
            phase: SpeechPhase::Idle,
            word: None,
            hovered_word: None,
            seek_hover: false,
            mark_x: Slide::new(0.),
            mark_width: Slide::new(0.),
            mark_y: 0.,
            mark_height: 0.,
            mark_target: None,
            position: 0.,
            duration: 0.,
            speed: 1.,
            voices: Vec::new(),
            voice: 0,
            voice_menu: false,
            menu_index: 0,
            menu_focus: None,
            trigger_focus: None,
            error: "".into(),
            expanded: Slide::new(0.),
            pending: false,
            session_id: 0,
            session_active: false,
            operation_id: 0,
            seek_bounds: TriggerBounds::default(),
            seek_dragging: false,
            seek_preview: None,
            touch_axis: Default::default(),
            touch_start_x: Default::default(),
            touch_dx: Default::default(),
            touch_dy: Default::default(),
        }
    }
    pub fn with_voices(mut self, voices: Vec<SpeechVoice>) -> Self {
        self.voices = voices;
        self
    }
    /// Attribute the text to its model and show that model's mark when available.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }
    /// Show playback controls before the host starts the first session.
    pub fn with_transport(mut self, duration: f32) -> Self {
        self.show_transport = true;
        self.duration = if duration.is_finite() {
            duration.max(0.)
        } else {
            0.
        };
        self
    }
    /// Legacy unscoped update; asynchronous hosts should use `set_phase_for`.
    pub fn set_phase(&mut self, phase: SpeechPhase, cx: &mut Context<Self>) {
        self.phase = phase;
        if matches!(
            phase,
            SpeechPhase::Idle | SpeechPhase::Error | SpeechPhase::Finished
        ) {
            self.session_active = false;
        }
        if phase == SpeechPhase::Idle {
            self.seek_dragging = false;
            self.seek_preview = None;
            self.touch_axis.set(0);
        }
        self.pending = false;
        self.expanded
            .duration(if phase == SpeechPhase::Idle { 0. } else { 1. }, 0.22, cx);
        cx.notify();
    }
    /// Capture this ID when handling Start or Retry; attach it to asynchronous callbacks.
    pub fn session_id(&self) -> Option<u64> {
        self.session_active.then_some(self.session_id)
    }
    pub fn set_phase_for(
        &mut self,
        session: u64,
        phase: SpeechPhase,
        cx: &mut Context<Self>,
    ) -> bool {
        if !is_live_session(self.session_active, self.session_id, session) {
            return false;
        }
        self.set_phase(phase, cx);
        true
    }
    /// Capture this alongside session_id when handling Start, Pause, Resume, Retry or Stop.
    pub fn operation_id(&self) -> u64 {
        self.operation_id
    }
    /// Reject a result from an older playback operation in the same session.
    pub fn set_phase_for_operation(
        &mut self,
        session: u64,
        operation: u64,
        phase: SpeechPhase,
        cx: &mut Context<Self>,
    ) -> bool {
        if !is_current_operation(
            self.session_active,
            self.session_id,
            session,
            self.operation_id,
            operation,
        ) {
            return false;
        }
        self.set_phase_for(session, phase, cx)
    }
    pub fn set_progress_for(
        &mut self,
        session: u64,
        word: Option<usize>,
        position: f32,
        duration: f32,
        cx: &mut Context<Self>,
    ) -> bool {
        if !is_live_session(self.session_active, self.session_id, session) {
            return false;
        }
        self.set_progress(word, position, duration, cx);
        true
    }
    pub fn set_error_for(
        &mut self,
        session: u64,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !is_live_session(self.session_active, self.session_id, session) {
            return false;
        }
        self.set_error(error, cx);
        true
    }
    pub fn set_speed_for(&mut self, session: u64, speed: f32, cx: &mut Context<Self>) -> bool {
        if !is_live_session(self.session_active, self.session_id, session) {
            return false;
        }
        self.set_speed(speed, cx);
        true
    }
    pub fn set_voice_for(&mut self, session: u64, id: &str, cx: &mut Context<Self>) -> bool {
        if !is_live_session(self.session_active, self.session_id, session) {
            return false;
        }
        self.set_voice(id, cx);
        true
    }
    /// Legacy unscoped update; use `set_progress_for` for asynchronous playback.
    pub fn set_progress(
        &mut self,
        word: Option<usize>,
        position: f32,
        duration: f32,
        cx: &mut Context<Self>,
    ) {
        self.word = word;
        if !self.seek_dragging && self.touch_axis.get() != 2 {
            self.seek_preview = None;
        }
        self.duration = if duration.is_finite() {
            duration.max(0.)
        } else {
            0.
        };
        self.position = if position.is_finite() {
            position.clamp(0., self.duration)
        } else {
            0.
        };
        cx.notify();
    }
    pub fn set_error(&mut self, error: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.error = error.into();
        self.set_phase(SpeechPhase::Error, cx);
    }
    pub fn set_speed(&mut self, speed: f32, cx: &mut Context<Self>) {
        self.speed = if speed.is_finite() {
            speed.clamp(0.5, 3.)
        } else {
            1.
        };
        cx.notify();
    }
    pub fn set_voice(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(index) = self.voices.iter().position(|v| v.id.as_ref() == id) {
            self.voice = index;
            cx.notify();
        }
    }
    fn preview_seek(&mut self, x: Pixels, cx: &mut Context<Self>) {
        self.seek_preview = Some(seek_at(self.seek_bounds.get(), x, self.duration));
        cx.notify();
    }
    fn finish_seek(&mut self, cancelled: bool, cx: &mut Context<Self>) {
        self.seek_dragging = false;
        if cancelled {
            self.seek_preview = None;
        } else if let Some(position) = self.seek_preview {
            cx.emit(SpeechEvent::Seek(position));
        }
        cx.notify();
    }
    fn request(&mut self, event: SpeechEvent, cx: &mut Context<Self>) {
        if blocked_by_pending(self.pending, &event) {
            return;
        }
        if matches!(
            event,
            SpeechEvent::Start
                | SpeechEvent::Retry
                | SpeechEvent::Pause
                | SpeechEvent::Resume
                | SpeechEvent::Stop
        ) {
            self.operation_id = self.operation_id.wrapping_add(1);
        }
        match event {
            SpeechEvent::Stop => {
                if self.phase == SpeechPhase::Idle {
                    return;
                }
                self.set_phase(SpeechPhase::Idle, cx);
            }
            SpeechEvent::Start | SpeechEvent::Retry => {
                self.session_id = self.session_id.wrapping_add(1);
                self.session_active = true;
                self.pending = true;
                self.phase = SpeechPhase::Preparing;
                self.expanded.duration(1., 0.22, cx);
            }
            SpeechEvent::Resume if self.phase == SpeechPhase::Finished => {
                self.session_id = self.session_id.wrapping_add(1);
                self.session_active = true;
                self.pending = true;
                self.phase = SpeechPhase::Preparing;
            }
            _ => {}
        }
        cx.emit(event);
        cx.notify();
    }
}
impl Render for SpeechPlayer {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.menu_focus.is_none() {
            self.menu_focus = Some(cx.focus_handle().tab_stop(true));
            self.trigger_focus = Some(cx.focus_handle().tab_stop(true));
        }
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let a = self.expanded.get(w, cx).clamp(0., 1.);
        let mark_x = self.mark_x.get(w, cx);
        let mark_width = self.mark_width.get(w, cx);
        let mark_y = self.mark_y;
        let mark_height = self.mark_height;
        let selected = self.word;
        let weak = cx.weak_entity();
        let mut words = div()
            .relative()
            .flex()
            .flex_wrap()
            .gap(px(2.))
            .line_height(px(26.))
            .on_children_prepainted(move |bounds, _, cx| {
                let (Some(first), Some(index)) = (bounds.get(1), selected) else {
                    return;
                };
                let Some(selected) = bounds.get(index + 1) else {
                    return;
                };
                let x = f32::from(selected.origin.x - first.origin.x);
                let y = f32::from(selected.origin.y - first.origin.y);
                let width = f32::from(selected.size.width);
                let height = f32::from(selected.size.height);
                let _ = weak.update(cx, |s, cx| {
                    let changed = s.mark_target.is_none_or(|(old_x, old_y, old_w)| {
                        (old_x - x).abs() > 0.1
                            || (old_y - y).abs() > 0.1
                            || (old_w - width).abs() > 0.1
                    });
                    if changed {
                        if s.mark_target
                            .is_none_or(|(_, old_y, _)| (old_y - y).abs() > 0.5)
                        {
                            s.mark_x.reset(x);
                            s.mark_width.reset(width);
                        } else {
                            let duration = (0.15 / s.speed).clamp(0.05, 0.3);
                            s.mark_x.duration(x, duration, cx);
                            s.mark_width.duration(width, duration, cx);
                        }
                        s.mark_y = y;
                        s.mark_height = height;
                        s.mark_target = Some((x, y, width));
                        cx.notify();
                    }
                });
            })
            .child(
                div()
                    .absolute()
                    .left(px(mark_x))
                    .top(px(mark_y))
                    .w(px(mark_width))
                    .h(px(mark_height))
                    .rounded(px(7.))
                    .bg(if selected.is_some() {
                        p.bright.opacity(0.18)
                    } else {
                        transparent_black()
                    }),
            );
        for (i, word) in self.text.split_word_bounds().enumerate() {
            words = words.child(
                div()
                    .id(("speech-word", i))
                    .text_color(if self.word == Some(i) || self.hovered_word == Some(i) {
                        p.ink
                    } else {
                        p.secondary
                    })
                    .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                        s.hovered_word = if *over { Some(i) } else { None };
                        cx.notify();
                    }))
                    .child(word.to_owned()),
            );
        }
        let mut root = div()
            .id("speech-player")
            .role(Role::Group)
            .aria_label(self.label.clone())
            .w(px(522.))
            .max_w_full()
            .pt(px(16.))
            .px(px(16.))
            .pb(px(12.))
            .bg(p.panel)
            .rounded(r)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap_3()
            .when_some(self.model.clone(), |root, model| {
                root.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .when_some(brand_mark(&model, 16., cx), |row, mark| row.child(mark))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(model),
                        ),
                )
            })
            .child(words);
        if self.phase == SpeechPhase::Error {
            root = root.child(
                div()
                    .id("speech-error")
                    .role(Role::Alert)
                    .text_color(p.danger)
                    .child(self.error.clone()),
            );
        }
        let mut actions = div()
            .h(px(32.))
            .flex()
            .items_center()
            .gap(px(2.))
            .overflow_hidden();
        if self.phase == SpeechPhase::Idle && !self.show_transport {
            actions = actions.child(
                face(
                    "speech-start",
                    tr(cx, "Read aloud", "朗读"),
                    32.,
                    10.,
                    Tone::Surface,
                    cx,
                )
                .h(px(32.))
                .px(px(10.))
                .pl(px(8.))
                .gap(px(6.))
                .rounded(px(8.))
                .text_color(p.secondary)
                .hover(|s| s.bg(p.tray).text_color(p.ink))
                .child(tr(cx, "Read aloud", "朗读"))
                .on_click(cx.listener(|s, _, _, cx| s.request(SpeechEvent::Start, cx))),
            );
        } else {
            let event = match self.phase {
                SpeechPhase::Playing => SpeechEvent::Pause,
                SpeechPhase::Error => SpeechEvent::Retry,
                SpeechPhase::Paused | SpeechPhase::Finished => SpeechEvent::Resume,
                SpeechPhase::Idle => SpeechEvent::Start,
                SpeechPhase::Preparing => SpeechEvent::Pause,
            };
            let label = match self.phase {
                SpeechPhase::Playing => tr(cx, "Pause", "暂停"),
                SpeechPhase::Error => tr(cx, "Retry", "重试"),
                SpeechPhase::Preparing => tr(cx, "Preparing…", "正在准备…"),
                _ => tr(cx, "Play", "播放"),
            };
            actions = actions.pl(px(8.)).child(
                face("speech-play", label, 32., 16., Tone::Primary, cx)
                    .size(px(32.))
                    .rounded_full()
                    .p_0()
                    .bg(p.bright)
                    .hover(|s| s.bg(p.bright.blend(p.ink.opacity(0.12))))
                    .opacity(if self.phase == SpeechPhase::Preparing {
                        0.5
                    } else {
                        1.
                    })
                    .child(icon(
                        if self.phase == SpeechPhase::Playing {
                            Glyph::Pause
                        } else {
                            Glyph::Play
                        },
                        16.,
                        p.inverse,
                    ))
                    .on_click(cx.listener(move |s, _, _, cx| s.request(event.clone(), cx))),
            );
            let position = self.seek_preview.unwrap_or(self.position);
            let ratio = if self.duration > 0. {
                position / self.duration
            } else {
                0.
            };
            let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if s.seek_dragging {
                    s.preview_seek(e.position.x, cx);
                }
            });
            let released = cx.listener(|s, e: &MouseUpEvent, _, cx| {
                if s.seek_dragging && e.button == MouseButton::Left {
                    s.preview_seek(e.position.x, cx);
                    s.finish_seek(false, cx);
                }
            });
            let touch_axis = self.touch_axis.clone();
            let touch_start = self.touch_start_x.clone();
            let touch_dx = self.touch_dx.clone();
            let touch_dy = self.touch_dy.clone();
            let touch_position = touch_start.clone();
            let touch_delta = touch_dx.clone();
            let touch_seek = cx.listener(move |s, e: &ScrollEvent, _, cx| {
                s.preview_seek(touch_position.get() + touch_delta.get(), cx);
                if matches!(e.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                    s.finish_seek(e.touch_phase == TouchPhase::Cancelled, cx);
                }
            });
            let shown = self.seek_preview.unwrap_or(self.position);
            actions = actions.child(
                div()
                    .min_w(px(29.))
                    .ml(px(10.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.faint)
                    .child(format!(
                        "{}:{:02}",
                        (shown as u64) / 60,
                        (shown as u64) % 60
                    )),
            );
            actions = actions.child(
                div()
                    .id("speech-seek")
                    .relative()
                    .role(Role::Slider)
                    .aria_label(tr(cx, "Speech progress", "朗读进度"))
                    .aria_value(format!("{}%", (ratio * 100.).round()))
                    .tab_index(0)
                    .flex_1()
                    .h(px(24.))
                    .ml(px(6.))
                    .mr(px(6.))
                    .rounded(px(6.))
                    .flex()
                    .on_hover(cx.listener(|s, over: &bool, _, cx| {
                        s.seek_hover = *over;
                        cx.notify();
                    }))
                    .items_center()
                    .cursor_pointer()
                    .child(self.seek_bounds.measure())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, _, cx| {
                            s.seek_dragging = true;
                            s.preview_seek(e.position.x, cx);
                        }),
                    )
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |_, _, w, _| {
                                w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                                    if phase.capture() {
                                        moved(e, w, cx);
                                    }
                                });
                                w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                                    if phase.capture() {
                                        released(e, w, cx);
                                    }
                                });
                            },
                        )
                        .absolute()
                        .size(px(0.)),
                    )
                    .child(
                        canvas(
                            |bounds, w, _| w.insert_hitbox(bounds, HitboxBehavior::Normal),
                            move |_, hit, w, _| {
                                w.on_mouse_event(move |e: &ScrollEvent, phase, w, cx| {
                                    if !phase.capture() || e.source != ScrollSource::Touch {
                                        return;
                                    }
                                    if e.touch_phase == TouchPhase::Started {
                                        touch_axis.set(if hit.is_hovered_at(e.position, w) {
                                            1
                                        } else {
                                            0
                                        });
                                        touch_start.set(e.position.x);
                                        touch_dx.set(px(0.));
                                        touch_dy.set(px(0.));
                                    }
                                    if touch_axis.get() == 0 {
                                        return;
                                    }
                                    let delta = e.delta.pixel_delta(px(16.));
                                    touch_dx.set(touch_dx.get() + delta.x);
                                    touch_dy.set(touch_dy.get() + delta.y);
                                    if touch_axis.get() == 1
                                        && (touch_dx.get().abs() > px(4.)
                                            || touch_dy.get().abs() > px(4.))
                                    {
                                        touch_axis.set(
                                            if touch_dx.get().abs() > touch_dy.get().abs() {
                                                2
                                            } else {
                                                3
                                            },
                                        );
                                    }
                                    if touch_axis.get() == 2 {
                                        touch_seek(e, w, cx);
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    }
                                    if matches!(
                                        e.touch_phase,
                                        TouchPhase::Ended | TouchPhase::Cancelled
                                    ) {
                                        touch_axis.set(0);
                                    }
                                });
                            },
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(if self.seek_hover || self.seek_dragging {
                                6.
                            } else {
                                4.
                            }))
                            .rounded(px(2.))
                            .bg(p.ink.opacity(0.12))
                            .child(
                                div()
                                    .w(relative(ratio))
                                    .h_full()
                                    .rounded(px(2.))
                                    .bg(p.bright),
                            )
                            .when(self.seek_hover || self.seek_dragging, |track| {
                                track.child(
                                    div()
                                        .absolute()
                                        .top(px(-3.))
                                        .left(relative(ratio))
                                        .ml(px(-6.))
                                        .size(px(12.))
                                        .rounded_full()
                                        .bg(p.bright)
                                        .border_2()
                                        .border_color(p.panel),
                                )
                            }),
                    )
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                        let v = match e.keystroke.key.as_str() {
                            "left" => s.position - 5.,
                            "right" => s.position + 5.,
                            "home" => 0.,
                            "end" => s.duration,
                            _ => return,
                        };
                        cx.emit(SpeechEvent::Seek(v.clamp(0., s.duration)));
                        cx.stop_propagation();
                    })),
            );
            let speed = if self.speed < 1.25 {
                1.5
            } else if self.speed < 1.75 {
                2.
            } else {
                1.
            };
            actions = actions.child(
                face(
                    "speech-speed",
                    tr(cx, "Speed", "语速"),
                    32.,
                    10.,
                    Tone::Surface,
                    cx,
                )
                .min_w(px(40.))
                .h(px(32.))
                .px(px(8.))
                .rounded(px(8.))
                .text_color(p.secondary)
                .hover(|s| s.bg(p.tray).text_color(p.ink))
                .child(format!("{}×", self.speed))
                .on_click(cx.listener(move |s, _, _, cx| s.request(SpeechEvent::Speed(speed), cx))),
            );
            if let Some(voice) = self.voices.get(self.voice) {
                actions = actions.child(
                    face(
                        "speech-voice",
                        tr(cx, "Voice", "声音"),
                        32.,
                        10.,
                        Tone::Surface,
                        cx,
                    )
                    .track_focus(self.trigger_focus.as_ref().unwrap())
                    .aria_expanded(self.voice_menu)
                    .h(px(32.))
                    .pl(px(8.))
                    .pr(px(6.))
                    .gap(px(4.))
                    .rounded(px(8.))
                    .text_color(p.secondary)
                    .hover(|s| s.bg(p.tray).text_color(p.ink))
                    .child(voice.label.clone())
                    .on_click(cx.listener(|s, _, w, cx| {
                        s.voice_menu = !s.voice_menu;
                        s.menu_index = s.voice;
                        if s.voice_menu {
                            if let Some(focus) = &s.menu_focus {
                                focus.focus(w, cx);
                            }
                        } else if let Some(focus) = &s.trigger_focus {
                            focus.focus(w, cx);
                        }
                        cx.notify();
                    })),
                );
            }
            actions = actions.child(
                face(
                    "speech-stop",
                    tr(cx, "Stop", "停止"),
                    32.,
                    16.,
                    Tone::Surface,
                    cx,
                )
                .size(px(32.))
                .rounded(px(8.))
                .p_0()
                .text_color(p.secondary)
                .hover(|s| s.bg(p.tray).text_color(p.ink))
                .child(icon(Glyph::Close, 14., p.secondary))
                .on_click(cx.listener(|s, _, _, cx| s.request(SpeechEvent::Stop, cx))),
            );
        }
        root = root.child(actions.opacity(0.8 + 0.2 * a));
        if self.voice_menu {
            let mut menu = div()
                .id("speech-voice-menu")
                .role(Role::Menu)
                .aria_label(tr(cx, "Choose voice", "选择声音"))
                .track_focus(self.menu_focus.as_ref().unwrap())
                .w(px(240.))
                .p(px(6.))
                .rounded(r)
                .border_1()
                .border_color(p.line)
                .bg(p.panel)
                .flex()
                .flex_col()
                .gap_1()
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    match e.keystroke.key.as_str() {
                        "escape" => {
                            s.voice_menu = false;
                            if let Some(focus) = &s.trigger_focus {
                                focus.focus(w, cx);
                            }
                        }
                        "up" => s.menu_index = s.menu_index.saturating_sub(1),
                        "down" => {
                            s.menu_index = s
                                .menu_index
                                .saturating_add(1)
                                .min(s.voices.len().saturating_sub(1))
                        }
                        "enter" | "space" => {
                            if let Some(voice) = s.voices.get(s.menu_index) {
                                let id = voice.id.clone();
                                s.voice_menu = false;
                                s.request(SpeechEvent::Voice(id), cx);
                                if let Some(focus) = &s.trigger_focus {
                                    focus.focus(w, cx);
                                }
                            }
                        }
                        _ => return,
                    }
                    cx.notify();
                    cx.stop_propagation();
                }));
            for (index, voice) in self.voices.iter().enumerate() {
                let id = voice.id.clone();
                menu = menu.child(
                    div()
                        .id(("speech-voice-option", index))
                        .role(Role::MenuItemRadio)
                        .aria_label(voice.label.clone())
                        .aria_toggled((index == self.voice).into())
                        .px(px(10.))
                        .py(px(7.))
                        .flex()
                        .items_center()
                        .rounded(r)
                        .bg(if index == self.menu_index {
                            p.tray
                        } else {
                            transparent_black()
                        })
                        .hover(|s| s.bg(p.tray))
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(voice.label.clone())
                        .on_click(cx.listener(move |s, _, w, cx| {
                            s.voice_menu = false;
                            s.request(SpeechEvent::Voice(id.clone()), cx);
                            if let Some(focus) = &s.trigger_focus {
                                focus.focus(w, cx);
                            }
                            cx.notify();
                        })),
                );
            }
            root = root.child(menu);
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{SpeechEvent, blocked_by_pending, is_live_session, seek_at};
    use gpui::{Bounds, point, px, size};
    #[test]
    fn stop_is_allowed_while_preparing_but_other_requests_are_blocked() {
        assert!(!blocked_by_pending(true, &SpeechEvent::Stop));
        assert!(blocked_by_pending(true, &SpeechEvent::Start));
        assert!(blocked_by_pending(true, &SpeechEvent::Pause));
        assert!(!blocked_by_pending(false, &SpeechEvent::Pause));
    }
    #[test]
    fn stop_and_restart_reject_late_synthesis_callbacks() {
        assert!(is_live_session(true, 3, 3));
        assert!(!is_live_session(false, 3, 3));
        assert!(!is_live_session(true, 4, 3));
        assert!(is_live_session(true, 4, 4));
    }
    #[test]
    fn seek_clamps_pointer_to_track() {
        let bounds = Bounds::new(point(px(100.), px(0.)), size(px(200.), px(24.)));
        assert_eq!(seek_at(bounds, px(50.), 60.), 0.);
        assert_eq!(seek_at(bounds, px(200.), 60.), 30.);
        assert_eq!(seek_at(bounds, px(350.), 60.), 60.);
    }
}

#[cfg(test)]
mod operation_tests {
    use super::is_current_operation;
    #[test]
    fn late_pause_or_resume_result_does_not_override_new_operation() {
        assert!(is_current_operation(true, 3, 3, 5, 5));
        assert!(!is_current_operation(true, 3, 3, 5, 4));
        assert!(!is_current_operation(true, 4, 3, 5, 5));
        assert!(!is_current_operation(false, 3, 3, 5, 5));
    }
}
