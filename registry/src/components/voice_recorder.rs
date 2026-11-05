//! Host-driven voice-note recording, review, and retry controls.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecorderPhase {
    Idle,
    Requesting,
    Recording,
    Review,
    Sending,
    Error,
}
#[derive(Clone)]
pub struct VoiceMessage {
    pub id: SharedString,
    pub author: SharedString,
    pub meta: SharedString,
    pub duration: f32,
    pub peaks: Vec<f32>,
    pub outgoing: bool,
}
impl VoiceMessage {
    pub fn new(
        id: impl Into<SharedString>,
        author: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        duration: f32,
        peaks: Vec<f32>,
    ) -> Self {
        Self {
            id: id.into(),
            author: author.into(),
            meta: meta.into(),
            duration: if duration.is_finite() {
                duration.max(0.)
            } else {
                0.
            },
            peaks: peaks
                .into_iter()
                .map(|p| if p.is_finite() { p.clamp(0., 1.) } else { 0. })
                .collect(),
            outgoing: false,
        }
    }
    pub fn outgoing(mut self, outgoing: bool) -> Self {
        self.outgoing = outgoing;
        self
    }
}
fn newest_peaks(peaks: Vec<f32>) -> Vec<f32> {
    let first = peaks.len().saturating_sub(1000);
    peaks
        .into_iter()
        .skip(first)
        .map(|p| if p.is_finite() { p.clamp(0., 1.) } else { 0. })
        .collect()
}
fn remap_playing(
    old: &[VoiceMessage],
    index: Option<usize>,
    new: &[VoiceMessage],
) -> Option<usize> {
    let id = &old.get(index?)?.id;
    new.iter().position(|message| &message.id == id)
}
fn insert_unique(messages: &mut Vec<VoiceMessage>, message: VoiceMessage) -> bool {
    if messages.iter().any(|existing| existing.id == message.id) {
        return false;
    }
    messages.push(message);
    true
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RecorderEvent {
    Start,
    Stop,
    Play(bool),
    Seek(f32),
    Send,
    Discard,
    Retry,
    PlayMessage(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecorderOperation {
    Start,
    Stop,
    Send,
}
fn retry_phase(operation: RecorderOperation) -> RecorderPhase {
    match operation {
        RecorderOperation::Start => RecorderPhase::Requesting,
        RecorderOperation::Stop => RecorderPhase::Recording,
        RecorderOperation::Send => RecorderPhase::Sending,
    }
}
fn is_live_session(active: bool, current: u64, incoming: u64) -> bool {
    active && current == incoming
}
fn is_current_request(active: bool, current: u64, incoming: u64) -> bool {
    active && current == incoming
}
fn can_append_confirmation(
    active: bool,
    pending: bool,
    current: u64,
    incoming: u64,
    phase: RecorderPhase,
) -> bool {
    is_current_request(active && pending, current, incoming) && phase == RecorderPhase::Sending
}
pub struct VoiceRecorder {
    phase: RecorderPhase,
    seconds: f32,
    position: f32,
    peaks: Vec<f32>,
    playing: bool,
    stop_pending: bool,
    session_id: u64,
    session_active: bool,
    request_id: u64,
    request_pending: bool,
    operation: Option<RecorderOperation>,
    failed_operation: Option<RecorderOperation>,
    focus_armed: bool,
    focus_pending: bool,
    stage_height: Slide,
    start_focus: Option<FocusHandle>,
    stop_focus: Option<FocusHandle>,
    send_focus: Option<FocusHandle>,
    error: SharedString,
    label: SharedString,
    seek_bounds: TriggerBounds,
    seek_touch: Rc<Cell<bool>>,
    seek_x: Rc<Cell<Pixels>>,
    messages: Vec<VoiceMessage>,
    playing_message: Option<usize>,
}
impl EventEmitter<RecorderEvent> for VoiceRecorder {}
impl VoiceRecorder {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            phase: RecorderPhase::Idle,
            seconds: 0.,
            position: 0.,
            peaks: Vec::new(),
            playing: false,
            stop_pending: false,
            session_id: 0,
            session_active: false,
            request_id: 0,
            request_pending: false,
            operation: None,
            failed_operation: None,
            focus_armed: false,
            focus_pending: false,
            stage_height: Slide::new(60.),
            start_focus: None,
            stop_focus: None,
            send_focus: None,
            error: "".into(),
            label: label.into(),
            seek_bounds: TriggerBounds::default(),
            seek_touch: Default::default(),
            seek_x: Rc::new(Cell::new(px(0.))),
            messages: Vec::new(),
            playing_message: None,
        }
    }
    pub fn with_messages(mut self, messages: Vec<VoiceMessage>) -> Self {
        self.messages = messages;
        self
    }
    pub fn set_messages(&mut self, messages: Vec<VoiceMessage>, cx: &mut Context<Self>) {
        self.playing_message = remap_playing(&self.messages, self.playing_message, &messages);
        self.messages = messages;
        cx.notify();
    }
    /// Append a host-confirmed message once. Repeated delivery of the same ID is ignored.
    pub fn append_message(&mut self, message: VoiceMessage, cx: &mut Context<Self>) -> bool {
        let inserted = insert_unique(&mut self.messages, message);
        if inserted {
            cx.notify();
        }
        inserted
    }
    pub fn set_message_playing(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        self.playing_message = index.filter(|index| *index < self.messages.len());
        cx.notify();
    }
    pub fn playing_message(&self) -> Option<usize> {
        self.playing_message
    }
    fn phase_height(phase: RecorderPhase) -> f32 {
        match phase {
            RecorderPhase::Idle => 60.,
            RecorderPhase::Requesting => 60.,
            RecorderPhase::Recording => 60.,
            RecorderPhase::Review => 136.,
            RecorderPhase::Sending => 118.,
            RecorderPhase::Error => 164.,
        }
    }
    fn animate_phase(&mut self, cx: &mut Context<Self>) {
        self.stage_height
            .duration(Self::phase_height(self.phase), 0.42, cx);
    }
    /// Legacy unscoped update. Async hosts should use `set_phase_for` with a captured request ID.
    pub fn set_phase(&mut self, phase: RecorderPhase, cx: &mut Context<Self>) {
        if phase == RecorderPhase::Error {
            self.failed_operation = self.operation.or(Some(match self.phase {
                RecorderPhase::Requesting => RecorderOperation::Start,
                RecorderPhase::Recording => RecorderOperation::Stop,
                _ => RecorderOperation::Send,
            }));
        }
        self.phase = phase;
        self.request_pending = false;
        self.operation = None;
        if phase == RecorderPhase::Idle {
            self.session_active = false;
        }
        if self.focus_armed {
            self.focus_pending = true;
            self.focus_armed = false;
        }
        self.stop_pending = false;
        self.animate_phase(cx);
        cx.notify();
    }
    /// Capture this in the Start handler; it identifies the current recording draft.
    pub fn session_id(&self) -> Option<u64> {
        self.session_active.then_some(self.session_id)
    }
    /// Capture this in each Start, Stop, Send or Retry handler to tag that operation's result.
    pub fn request_id(&self) -> Option<u64> {
        (self.session_active && self.request_pending).then_some(self.request_id)
    }
    /// Apply an operation result only if it still belongs to the latest request.
    pub fn set_phase_for(
        &mut self,
        request: u64,
        phase: RecorderPhase,
        cx: &mut Context<Self>,
    ) -> bool {
        if !is_current_request(
            self.session_active && self.request_pending,
            self.request_id,
            request,
        ) {
            return false;
        }
        if phase == self.phase {
            return true;
        }
        let allowed = matches!(phase, RecorderPhase::Error | RecorderPhase::Idle)
            || matches!(
                (self.phase, phase),
                (RecorderPhase::Requesting, RecorderPhase::Recording)
                    | (RecorderPhase::Recording, RecorderPhase::Review)
                    | (RecorderPhase::Sending, RecorderPhase::Review)
            );
        if !allowed {
            return false;
        }
        self.set_phase(phase, cx);
        true
    }
    /// A late failure from a superseded request cannot replace the current state.
    pub fn set_error_for(
        &mut self,
        request: u64,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !is_current_request(
            self.session_active && self.request_pending,
            self.request_id,
            request,
        ) {
            return false;
        }
        self.set_error(message, cx);
        true
    }
    /// Capture-device failures may arrive while the recording is active without a pending action.
    pub fn set_recording_error_for(
        &mut self,
        session: u64,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !is_live_session(self.session_active, self.session_id, session)
            || self.phase != RecorderPhase::Recording
            || self.stop_pending
        {
            return false;
        }
        self.operation = Some(RecorderOperation::Stop);
        self.set_error(message, cx);
        true
    }
    /// Progress and playback updates are scoped to the recording session, not a single send.
    pub fn set_audio_for(
        &mut self,
        session: u64,
        duration: f32,
        position: f32,
        peaks: Vec<f32>,
        playing: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        if !is_live_session(self.session_active, self.session_id, session) {
            return false;
        }
        self.set_audio(duration, position, peaks, playing, cx);
        true
    }
    /// Append a confirmation only for the active send request, with the usual message-ID deduplication.
    pub fn append_message_for(
        &mut self,
        request: u64,
        message: VoiceMessage,
        cx: &mut Context<Self>,
    ) -> bool {
        if !can_append_confirmation(
            self.session_active,
            self.request_pending,
            self.request_id,
            request,
            self.phase,
        ) {
            return false;
        }
        self.append_message(message, cx)
    }
    /// Legacy unscoped update; asynchronous device updates should use `set_audio_for`.
    pub fn set_audio(
        &mut self,
        duration: f32,
        position: f32,
        peaks: Vec<f32>,
        playing: bool,
        cx: &mut Context<Self>,
    ) {
        self.seconds = if duration.is_finite() {
            duration.max(0.)
        } else {
            0.
        };
        self.position = if position.is_finite() {
            position.clamp(0., self.seconds)
        } else {
            0.
        };
        self.peaks = newest_peaks(peaks);
        self.playing = playing;
        cx.notify();
    }
    /// Legacy unscoped error; asynchronous errors should use the appropriate guarded method.
    pub fn set_error(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.error = message.into();
        self.failed_operation = Some(self.operation.unwrap_or(match self.phase {
            RecorderPhase::Requesting => RecorderOperation::Start,
            RecorderPhase::Recording => RecorderOperation::Stop,
            _ => RecorderOperation::Send,
        }));
        self.operation = None;
        self.request_pending = false;
        self.stop_pending = false;
        self.phase = RecorderPhase::Error;
        self.animate_phase(cx);
        if self.focus_armed {
            self.focus_pending = true;
            self.focus_armed = false;
        }
        cx.notify();
    }
    /// The operation to retry after an error; remains available during Retry handling.
    pub fn failed_operation(&self) -> Option<RecorderOperation> {
        self.failed_operation
    }
    fn request(&mut self, event: RecorderEvent, cx: &mut Context<Self>) {
        match event {
            RecorderEvent::Start if self.phase == RecorderPhase::Idle => {
                self.phase = RecorderPhase::Requesting
            }
            RecorderEvent::Stop if self.phase == RecorderPhase::Recording && !self.stop_pending => {
                self.stop_pending = true
            }
            RecorderEvent::Send if self.phase == RecorderPhase::Review => {
                self.phase = RecorderPhase::Sending
            }
            RecorderEvent::Retry if self.phase == RecorderPhase::Error => {
                let operation = self.failed_operation.unwrap_or(RecorderOperation::Send);
                self.phase = retry_phase(operation);
                self.stop_pending = operation == RecorderOperation::Stop;
            }
            RecorderEvent::Discard
                if matches!(
                    self.phase,
                    RecorderPhase::Recording | RecorderPhase::Review | RecorderPhase::Error
                ) =>
            {
                self.phase = RecorderPhase::Idle
            }
            _ => return,
        }
        self.operation = match event {
            RecorderEvent::Start => Some(RecorderOperation::Start),
            RecorderEvent::Stop => Some(RecorderOperation::Stop),
            RecorderEvent::Send => Some(RecorderOperation::Send),
            RecorderEvent::Retry => self.failed_operation,
            _ => None,
        };
        self.request_id = self.request_id.wrapping_add(1);
        self.request_pending = event != RecorderEvent::Discard;
        if event == RecorderEvent::Start
            || (event == RecorderEvent::Retry
                && self.failed_operation == Some(RecorderOperation::Start))
        {
            self.session_id = self.session_id.wrapping_add(1);
            self.session_active = true;
        }
        if event == RecorderEvent::Discard {
            self.session_active = false;
        }
        self.animate_phase(cx);
        if event == RecorderEvent::Discard {
            self.focus_pending = true;
        } else {
            self.focus_armed = true;
        }
        cx.emit(event);
        cx.notify();
    }
    pub fn phase(&self) -> RecorderPhase {
        self.phase
    }
}
impl Render for VoiceRecorder {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.start_focus.is_none() {
            self.start_focus = Some(cx.focus_handle().tab_stop(true));
            self.stop_focus = Some(cx.focus_handle().tab_stop(true));
            self.send_focus = Some(cx.focus_handle().tab_stop(true));
        }
        if self.focus_pending {
            let target = match self.phase {
                RecorderPhase::Idle => self.start_focus.clone(),
                RecorderPhase::Recording => self.stop_focus.clone(),
                RecorderPhase::Review | RecorderPhase::Error => self.send_focus.clone(),
                _ => None,
            };
            if let Some(target) = target {
                self.focus_pending = false;
                cx.on_next_frame(w, move |_, w, cx| target.focus(w, cx));
            }
        }
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let label = self.label.clone();
        let clock = format!(
            "{}:{:02}",
            (self.seconds as u64) / 60,
            (self.seconds as u64) % 60
        );
        let mut waveform = div()
            .flex()
            .gap(px(2.))
            .items_center()
            .h(px(28.))
            .overflow_hidden();
        for peak in self.peaks.iter().rev().take(52).rev() {
            waveform = waveform.child(
                div()
                    .w(px(3.))
                    .h(px((peak * 28.).max(3.)))
                    .rounded(px(2.))
                    .bg(p.ink.opacity(0.28)),
            );
        }
        let mut root = div()
            .id("voice-recorder")
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape"
                    && matches!(
                        s.phase,
                        RecorderPhase::Recording | RecorderPhase::Review | RecorderPhase::Error
                    )
                {
                    s.request(RecorderEvent::Discard, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
            .role(Role::Group)
            .aria_label(label)
            .w_full()
            .max_w(px(416.))
            .pt(px(14.))
            .px(px(14.))
            .pb(px(12.))
            .rounded(r)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .flex()
            .flex_col()
            .gap_3();
        if !self.messages.is_empty() {
            let mut history = div().flex().flex_col().gap_3();
            for (i, message) in self.messages.iter().enumerate() {
                let mut bars = div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .h(px(22.))
                    .overflow_hidden();
                for peak in message.peaks.iter().take(36) {
                    bars = bars.child(
                        div()
                            .w(px(3.))
                            .h(px((peak * 22.).max(3.)))
                            .rounded(px(2.))
                            .bg(p.ink.opacity(0.28)),
                    );
                }
                let duration = format!(
                    "{}:{:02}",
                    message.duration as u64 / 60,
                    message.duration as u64 % 60
                );
                history = history.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .when(message.outgoing, |d| d.items_end())
                        .child(
                            div()
                                .px(px(6.))
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.faint)
                                .child(message.meta.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .h(px(44.))
                                .gap(px(10.))
                                .py(px(6.))
                                .pl(px(6.))
                                .pr(px(14.))
                                .rounded(r)
                                .bg(if message.outgoing { p.ink } else { p.tray })
                                .text_color(if message.outgoing { p.inverse } else { p.ink })
                                .child(
                                    face(
                                        ("message-play", i),
                                        tr(cx, "Play message", "播放语音"),
                                        32.,
                                        16.,
                                        Tone::Surface,
                                        cx,
                                    )
                                    .size(px(32.))
                                    .rounded_full()
                                    .p_0()
                                    .bg(if message.outgoing { p.inverse } else { p.ink })
                                    .child(icon(
                                        if self.playing_message == Some(i) {
                                            Glyph::Pause
                                        } else {
                                            Glyph::Play
                                        },
                                        14.,
                                        if message.outgoing { p.ink } else { p.inverse },
                                    ))
                                    .on_click(cx.listener(
                                        move |_, _, _, cx| cx.emit(RecorderEvent::PlayMessage(i)),
                                    )),
                                )
                                .child(bars)
                                .child(
                                    div()
                                        .min_w(px(34.))
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .opacity(0.8)
                                        .child(duration),
                                ),
                        ),
                );
            }
            root = root.child(history);
        }
        let stage_height = self.stage_height.get(w, cx);
        let mut composer = div()
            .w_full()
            .h(px(stage_height.max(0.)))
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap_3();
        match self.phase {
            RecorderPhase::Idle => {
                composer = composer.child(
                    div()
                        .h(px(60.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .border_t_1()
                        .border_color(p.line)
                        .pt(px(12.))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .text_color(p.faint)
                                .child(tr(cx, "Tap to record", "点击录音")),
                        )
                        .child(
                            face(
                                "record",
                                tr(cx, "Record", "录音"),
                                48.,
                                24.,
                                Tone::Surface,
                                cx,
                            )
                            .track_focus(self.start_focus.as_ref().unwrap())
                            .size(px(48.))
                            .rounded_full()
                            .p_0()
                            .bg(p.ink)
                            .child(icon(Glyph::Comment, 18., p.inverse))
                            .on_click(
                                cx.listener(|s, _, _, cx| s.request(RecorderEvent::Start, cx)),
                            ),
                        ),
                );
            }
            RecorderPhase::Requesting => {
                composer = composer.child(div().text_color(p.secondary).child(tr(
                    cx,
                    "Requesting microphone…",
                    "正在申请麦克风…",
                )));
            }
            RecorderPhase::Recording => {
                composer = composer.child(
                    div()
                        .h(px(60.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .border_t_1()
                        .border_color(p.line)
                        .pt(px(12.))
                        .child(
                            face(
                                "discard-recording",
                                tr(cx, "Discard", "放弃"),
                                40.,
                                20.,
                                Tone::Clear,
                                cx,
                            )
                            .size(px(40.))
                            .rounded_full()
                            .p_0()
                            .text_color(p.secondary)
                            .hover(|s| s.bg(p.tray).text_color(p.danger))
                            .child(icon(Glyph::Trash, 16., p.secondary))
                            .on_click(cx.listener(|s, _, _, cx| {
                                s.request(RecorderEvent::Discard, cx)
                            })),
                        )
                        .child(
                            div()
                                .h(px(48.))
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .pl(px(16.))
                                .pr(px(14.))
                                .rounded(r)
                                .bg(p.tray.blend(p.bright.opacity(0.07)))
                                .child(div().size(px(8.)).rounded_full().bg(p.danger))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .line_height(px(20.))
                                        .child(clock.clone()),
                                )
                                .child(waveform),
                        )
                        .child(
                            face(
                                "record-stop",
                                tr(cx, "Stop", "停止"),
                                48.,
                                24.,
                                Tone::Primary,
                                cx,
                            )
                            .track_focus(self.stop_focus.as_ref().unwrap())
                            .size(px(48.))
                            .rounded_full()
                            .p_0()
                            .bg(p.bright)
                            .hover(|s| s.bg(p.bright.blend(black().opacity(0.12))))
                            .child(div().size(px(14.)).rounded(px(2.)).bg(p.inverse))
                            .on_click(
                                cx.listener(|s, _, _, cx| s.request(RecorderEvent::Stop, cx)),
                            ),
                        ),
                );
            }
            RecorderPhase::Review | RecorderPhase::Sending | RecorderPhase::Error => {
                let playing = self.playing;
                let sending = self.phase == RecorderPhase::Sending;
                let play = face(
                    "record-play",
                    if playing {
                        tr(cx, "Pause", "暂停")
                    } else {
                        tr(cx, "Play", "播放")
                    },
                    32.,
                    16.,
                    Tone::Surface,
                    cx,
                )
                .p_0()
                .w(px(32.))
                .h(px(32.))
                .rounded_full()
                .bg(p.ink)
                .child(icon(
                    if playing { Glyph::Pause } else { Glyph::Play },
                    16.,
                    p.inverse,
                ))
                .on_click(cx.listener(move |_, _, _, cx| cx.emit(RecorderEvent::Play(!playing))));
                composer = composer.child(
                    div()
                        .flex()
                        .gap_3()
                        .items_center()
                        .child(play)
                        .child(waveform)
                        .child(clock),
                );
                if self.phase == RecorderPhase::Error {
                    composer = composer.child(
                        div()
                            .id("recorder-error")
                            .role(Role::Alert)
                            .child(self.error.clone()),
                    );
                }
                let is_error = self.phase == RecorderPhase::Error;
                let fraction = if self.seconds > 0. {
                    self.position / self.seconds
                } else {
                    0.
                };
                let bounds = self.seek_bounds.clone();
                let touch_bounds = self.seek_bounds.clone();
                let touch_active = self.seek_touch.clone();
                let touch_x = self.seek_x.clone();
                let listener_x = touch_x.clone();
                let touch_seek = cx.listener(move |s, e: &ScrollEvent, _, cx| {
                    if matches!(e.touch_phase, TouchPhase::Started | TouchPhase::Moved) {
                        listener_x.set(listener_x.get() + e.delta.pixel_delta(px(16.)).x);
                        let b = touch_bounds.get();
                        let fraction = (f32::from(listener_x.get() - b.origin.x)
                            / f32::from(b.size.width).max(1.))
                        .clamp(0., 1.);
                        cx.emit(RecorderEvent::Seek(fraction * s.seconds));
                    }
                });
                composer = composer.child(
                    div()
                        .id("recorder-seek")
                        .relative()
                        .role(Role::Slider)
                        .aria_label(tr(cx, "Seek", "播放位置"))
                        .aria_value(format!("{}%", (fraction * 100.).round()))
                        .tab_index(0)
                        .w_full()
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .child(self.seek_bounds.measure())
                        .child(
                            div()
                                .w_full()
                                .h(px(4.))
                                .bg(p.line)
                                .child(div().w(relative(fraction)).h_full().bg(p.ink)),
                        )
                        .child(
                            canvas(
                                |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
                                move |_, hit, w, _| {
                                    w.on_mouse_event(move |e: &ScrollEvent, phase, w, cx| {
                                        if !phase.capture() || e.source != ScrollSource::Touch {
                                            return;
                                        }
                                        if e.touch_phase == TouchPhase::Started {
                                            let d = e.delta.pixel_delta(px(16.));
                                            touch_active.set(
                                                hit.is_hovered_at(e.position, w)
                                                    && d.x.abs() > d.y.abs(),
                                            );
                                            if touch_active.get() {
                                                touch_x.set(e.position.x);
                                            }
                                        }
                                        if !touch_active.get() {
                                            return;
                                        }
                                        touch_seek(e, w, cx);
                                        if matches!(
                                            e.touch_phase,
                                            TouchPhase::Ended | TouchPhase::Cancelled
                                        ) {
                                            touch_active.set(false);
                                        }
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    });
                                },
                            )
                            .absolute()
                            .inset_0(),
                        )
                        .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                            let next = match e.keystroke.key.as_str() {
                                "left" => s.position - 1.,
                                "right" => s.position + 1.,
                                "home" => 0.,
                                "end" => s.seconds,
                                _ => return,
                            };
                            cx.emit(RecorderEvent::Seek(next.clamp(0., s.seconds)));
                            w.prevent_default();
                            cx.stop_propagation();
                        }))
                        .on_click(cx.listener(move |s, e: &ClickEvent, _, cx| {
                            let b = bounds.get();
                            let fraction = (f32::from(e.position().x - b.origin.x)
                                / f32::from(b.size.width).max(1.))
                            .clamp(0., 1.);
                            cx.emit(RecorderEvent::Seek(fraction * s.seconds));
                        })),
                );
                let mut actions = div().flex().gap_2();
                if !sending {
                    actions = actions.child(
                        face(
                            "record-discard",
                            tr(cx, "Discard", "放弃"),
                            40.,
                            20.,
                            Tone::Clear,
                            cx,
                        )
                        .size(px(40.))
                        .rounded_full()
                        .p_0()
                        .text_color(p.secondary)
                        .hover(|s| s.bg(p.tray).text_color(p.danger))
                        .child(icon(Glyph::Trash, 16., p.secondary))
                        .on_click(cx.listener(|s, _, _, cx| s.request(RecorderEvent::Discard, cx))),
                    );
                    actions = actions.child(
                        face(
                            "record-send",
                            if self.phase == RecorderPhase::Error {
                                tr(cx, "Retry", "重试")
                            } else {
                                tr(cx, "Send", "发送")
                            },
                            48.,
                            24.,
                            Tone::Primary,
                            cx,
                        )
                        .track_focus(self.send_focus.as_ref().unwrap())
                        .size(px(48.))
                        .rounded_full()
                        .p_0()
                        .bg(p.bright)
                        .hover(|s| s.bg(p.bright.blend(black().opacity(0.12))))
                        .child(icon(Glyph::Send, 18., p.inverse))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.request(
                                if is_error {
                                    RecorderEvent::Retry
                                } else {
                                    RecorderEvent::Send
                                },
                                cx,
                            )
                        })),
                    );
                } else {
                    actions = actions.child(div().text_color(p.secondary).child(tr(
                        cx,
                        "Sending…",
                        "正在发送…",
                    )));
                }
                composer = composer.child(actions);
            }
        }
        root.child(composer)
    }
}

#[cfg(test)]
mod tests {
    use super::{VoiceMessage, insert_unique, newest_peaks, remap_playing};
    #[test]
    fn waveform_keeps_recent_samples_and_message_identity_survives_reorder() {
        let mut peaks = vec![0.; 1000];
        peaks.extend([0.4, 0.8]);
        let recent = newest_peaks(peaks);
        assert_eq!(recent.len(), 1000);
        assert_eq!(&recent[998..], &[0.4, 0.8]);
        let a = VoiceMessage::new("a", "A", "", 1., vec![]);
        let b = VoiceMessage::new("b", "B", "", 1., vec![]);
        assert_eq!(
            remap_playing(&[a.clone(), b.clone()], Some(0), &[b, a]),
            Some(1)
        );
    }
    #[test]
    fn repeated_host_confirmation_adds_only_one_message() {
        let mut messages = Vec::new();
        let message = VoiceMessage::new("same-id", "You", "now", 6., vec![0.1]);
        assert!(insert_unique(&mut messages, message.clone()));
        assert!(!insert_unique(&mut messages, message));
        assert!(insert_unique(
            &mut messages,
            VoiceMessage::new("next-id", "You", "now", 4., vec![])
        ));
        assert_eq!(messages.len(), 2);
    }
}

#[cfg(test)]
mod generation_tests {
    use super::{
        RecorderOperation, RecorderPhase, can_append_confirmation, is_current_request,
        is_live_session, retry_phase,
    };
    #[test]
    fn retries_return_to_the_failed_operation() {
        assert_eq!(
            retry_phase(RecorderOperation::Start),
            RecorderPhase::Requesting
        );
        assert_eq!(
            retry_phase(RecorderOperation::Stop),
            RecorderPhase::Recording
        );
        assert_eq!(retry_phase(RecorderOperation::Send), RecorderPhase::Sending);
    }
    #[test]
    fn stale_sessions_and_superseded_send_requests_are_rejected() {
        assert!(is_live_session(true, 4, 4));
        assert!(!is_live_session(false, 4, 4));
        assert!(!is_live_session(true, 5, 4));
        assert!(is_current_request(true, 9, 9));
        assert!(!is_current_request(true, 10, 9));
        assert!(!is_current_request(false, 10, 10));
    }
    #[test]
    fn failed_send_then_retry_only_accepts_retry_confirmation() {
        assert!(!can_append_confirmation(
            true,
            false,
            9,
            9,
            RecorderPhase::Error
        ));
        assert!(!can_append_confirmation(
            true,
            true,
            10,
            9,
            RecorderPhase::Sending
        ));
        assert!(can_append_confirmation(
            true,
            true,
            10,
            10,
            RecorderPhase::Sending
        ));
        assert!(!can_append_confirmation(
            false,
            false,
            10,
            10,
            RecorderPhase::Idle
        ));
    }
}
