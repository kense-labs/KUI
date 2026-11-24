//! Live bidirectional voice call card with dual waveforms and latency budget.
//!
//! Renders scrolling `You` and `Agent` audio waveform lanes via [`PathBuilder`] with the turn-taking
//! silence pause measured in milliseconds between them, live spoken words firming up from faint to
//! ink, and a stacked `Endpoint` + `STT` + `LLM` + `TTS` + `Network` latency bar plotted against a target budget line.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_eid, pro_empty_state, pro_fold, pro_presence, pro_rolling_number,
        pro_skeleton_bar, pro_spring_f32, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::disabled_motion,
        pro_icons::{
            AGENT_WAVE_MS, ProIcon, agent_pixel_cells, person_initials, pro_icon, pro_svg,
        },
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const GAUGE_STAT_SVG: &str = r#"<path d="M3.5 15a8.5 8.5 0 1 1 17 0M12 15l3.5-4.5M12 16a1 1 0 1 0 0-2 1 1 0 0 0 0 2Z" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Live call phase in [`VoiceSession`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VoiceCallStatus {
    /// Establishing audio transport.
    Connecting,
    /// Person is speaking / microphone open.
    Listening,
    /// Person finished speaking; measuring silence gap while STT + LLM + TTS prepare audio.
    Thinking,
    /// Agent is actively speaking back word by word.
    #[default]
    Speaking,
    /// Call ended.
    Ended,
}

/// Speaker role for a [`VoiceTurn`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceSpeaker {
    /// Human caller (`"Maya"` / `"You"`).
    Person,
    /// AI voice agent (`"Atlas"`).
    Agent,
}

/// Five-stage voice-to-voice latency breakdown in milliseconds (`Endpoint + STT + LLM + TTS + Network`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceLatencyBreakdown {
    /// Voice-activity endpointing wait in ms.
    pub endpoint_ms: u32,
    /// Speech-to-text (`STT`) transcription time in ms.
    pub stt_ms: u32,
    /// Language model (`LLM`) time-to-first-sentence in ms.
    pub llm_ms: u32,
    /// Text-to-speech (`TTS`) synthesis time in ms.
    pub tts_ms: u32,
    /// Audio transport / network jitter in ms.
    pub network_ms: u32,
}

impl Default for VoiceLatencyBreakdown {
    fn default() -> Self {
        Self {
            endpoint_ms: 175,
            stt_ms: 70,
            llm_ms: 220,
            tts_ms: 115,
            network_ms: 45,
        }
    }
}

impl VoiceLatencyBreakdown {
    /// Construct a custom latency breakdown in milliseconds.
    pub fn new(endpoint_ms: u32, stt_ms: u32, llm_ms: u32, tts_ms: u32, network_ms: u32) -> Self {
        Self {
            endpoint_ms,
            stt_ms,
            llm_ms,
            tts_ms,
            network_ms,
        }
    }

    /// Total voice-to-voice turn-around latency in milliseconds.
    pub fn total_ms(&self) -> u32 {
        self.endpoint_ms + self.stt_ms + self.llm_ms + self.tts_ms + self.network_ms
    }

    fn as_array(&self) -> [u32; 5] {
        [
            self.endpoint_ms,
            self.stt_ms,
            self.llm_ms,
            self.tts_ms,
            self.network_ms,
        ]
    }
}

/// Spoken turn in a [`VoiceSession`] exchange.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceTurn {
    /// Stable turn identifier.
    pub id: SharedString,
    /// Who spoke this turn (`Person` or `Agent`).
    pub speaker: VoiceSpeaker,
    /// Full transcript sentence.
    pub text: SharedString,
    /// Number of words already spoken aloud (`None` means all words spoken).
    pub spoken_words: Option<usize>,
    /// Whether the caller barged in and cut off this agent turn early.
    pub interrupted: bool,
    /// Measured voice-to-voice latency breakdown on agent turns.
    pub latency: Option<VoiceLatencyBreakdown>,
}

impl VoiceTurn {
    /// Construct a [`VoiceTurn`].
    pub fn new(
        id: impl Into<SharedString>,
        speaker: VoiceSpeaker,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            speaker,
            text: text.into(),
            spoken_words: None,
            interrupted: false,
            latency: None,
        }
    }

    /// Attach a [`VoiceLatencyBreakdown`] to an agent turn.
    pub fn with_latency(mut self, latency: VoiceLatencyBreakdown) -> Self {
        self.latency = Some(latency);
        self
    }

    /// Set the number of words spoken aloud so far.
    pub fn with_spoken_words(mut self, count: usize) -> Self {
        self.spoken_words = Some(count);
        self
    }
}

/// Agent identity on [`VoiceSession`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceAgentSpec {
    /// Seed id for the 3x3 mirrored pixel mark.
    pub id: SharedString,
    /// Agent display name.
    pub name: SharedString,
}

impl VoiceAgentSpec {
    /// Construct a [`VoiceAgentSpec`].
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// Typed events emitted by [`VoiceSession`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiceSessionEvent {
    /// Emitted when `Pause` / `Resume` toggles live call sampling.
    PauseChanged(bool),
    /// Emitted when `Barge in` interrupts the agent and hands the floor back to the caller.
    BargedIn {
        /// Turn number interrupted.
        turn_number: usize,
    },
    /// Emitted when call settings (`show_transcript`, `noise_reduction`, `record_call`, or `budget_ms`) change.
    SettingChanged {
        /// Setting key.
        key: SharedString,
    },
}

impl EventEmitter<VoiceSessionEvent> for VoiceSession {}

const WAVE_BINS: usize = 52;

/// Live bidirectional voice call card with dual waveforms, silence pause meter, transcript, and latency budget.
pub struct VoiceSession {
    id: ElementId,
    focus: FocusHandle,
    agent: VoiceAgentSpec,
    subtitle: Option<SharedString>,
    person_name: SharedString,
    status: VoiceCallStatus,
    turns: Vec<VoiceTurn>,
    person_wave: Vec<f32>,
    agent_wave: Vec<f32>,
    pause_span_bins: (usize, usize),
    budget_ms: u32,
    paused: bool,
    show_transcript: bool,
    noise_reduction: bool,
    record_call: bool,
    settings_open: bool,
    hovered_slice: Option<usize>,
    pause_tip_open: bool,
    base_call_secs: f32,
    call_accum_secs: f32,
    sample_accum: f32,
    cycle_clock: f32,
    started_at: Instant,
    last_frame: Option<Instant>,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl VoiceSession {
    /// Create a [`VoiceSession`] preloaded with `"Atlas · Billing support"` and `"Maya"` on Turn 5.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let (pw, aw, span) = initial_waveform_buffers();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            agent: VoiceAgentSpec::new("atlas", "Atlas"),
            subtitle: Some("Billing support".into()),
            person_name: "Maya".into(),
            status: VoiceCallStatus::Speaking,
            turns: default_voice_turns(),
            person_wave: pw,
            agent_wave: aw,
            pause_span_bins: span,
            budget_ms: 800,
            paused: false,
            show_transcript: true,
            noise_reduction: true,
            record_call: false,
            settings_open: false,
            hovered_slice: None,
            pause_tip_open: false,
            base_call_secs: 99.0,
            call_accum_secs: 0.0,
            sample_accum: 0.0,
            cycle_clock: 2.2,
            started_at: Instant::now(),
            last_frame: None,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(500.0)),
            toast: None,
        }
    }

    /// Configure the voice agent (`id` seeds the 3x3 pixel mark).
    pub fn with_agent(
        mut self,
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
    ) -> Self {
        self.agent = VoiceAgentSpec::new(id, name);
        self
    }

    /// Set or clear (`None`) the call subtitle shown beside the agent name.
    pub fn with_subtitle(mut self, subtitle: Option<SharedString>) -> Self {
        self.subtitle = subtitle;
        self
    }

    /// Set the caller's display name (`"Maya"` default).
    pub fn with_person_name(mut self, name: impl Into<SharedString>) -> Self {
        self.person_name = name.into();
        self
    }

    /// Set the initial [`VoiceCallStatus`].
    pub fn with_status(mut self, status: VoiceCallStatus) -> Self {
        self.status = status;
        self
    }

    /// Supply conversation turns (oldest first; agent turns feed the P50 latency stat).
    pub fn with_turns(mut self, turns: Vec<VoiceTurn>) -> Self {
        self.turns = turns;
        self
    }

    /// Set the target voice-to-voice latency budget in ms (`800` default).
    pub fn with_budget_ms(mut self, budget_ms: u32) -> Self {
        self.budget_ms = budget_ms.clamp(200, 3_000);
        self
    }

    /// Set whether live waveform scrolling starts paused (`false` default).
    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Show or hide the live word-by-word transcript rows (`true` default).
    pub fn with_show_transcript(mut self, show: bool) -> Self {
        self.show_transcript = show;
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the current [`VoiceCallStatus`].
    pub fn status(&self) -> VoiceCallStatus {
        self.status
    }

    /// Return whether the call stream is paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Compute the P50 median voice-to-voice latency across all agent turns with [`VoiceLatencyBreakdown`].
    pub fn p50_latency_ms(&self) -> u32 {
        compute_p50_ms(&self.turns)
    }

    /// Active latency breakdown from the most recent agent turn.
    pub fn active_latency(&self) -> VoiceLatencyBreakdown {
        self.turns
            .iter()
            .rev()
            .find_map(|t| t.latency)
            .unwrap_or_default()
    }

    /// Toggle `Pause` / `Resume`.
    pub fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        self.paused = !self.paused;
        self.last_frame = None;
        cx.emit(VoiceSessionEvent::PauseChanged(self.paused));
        cx.notify();
    }

    /// Interrupt the agent immediately (`Barge in`), marking the current agent turn `interrupted` and switching to `Listening`.
    pub fn barge_in(&mut self, cx: &mut Context<Self>) {
        if let Some(last_agent) = self
            .turns
            .iter_mut()
            .rev()
            .find(|t| t.speaker == VoiceSpeaker::Agent)
        {
            last_agent.interrupted = true;
        }
        self.status = VoiceCallStatus::Listening;
        self.cycle_clock = 0.0;
        self.paused = false;
        let turn_number = self.turns.len();
        self.toast = Some((
            tr(
                cx,
                "Barged in — microphone active",
                "已打断智能体播报，麦克风已接管",
            )
            .into(),
            Instant::now(),
        ));
        cx.emit(VoiceSessionEvent::BargedIn { turn_number });
        cx.notify();
    }

    fn step_simulation_tick(&mut self) {
        // 9.0s natural conversational loop:
        // 0.0..2.2s -> Listening (person wave active)
        // 2.2..3.1s -> Thinking (silence gap measured, both waves quiet)
        // 3.1..9.0s -> Speaking (agent wave active, words firm up one by one)
        let phase = self.cycle_clock % 9.0;
        let next_status = if phase < 2.2 {
            VoiceCallStatus::Listening
        } else if phase < 3.1 {
            VoiceCallStatus::Thinking
        } else {
            VoiceCallStatus::Speaking
        };
        self.status = next_status;

        let t = self.call_accum_secs;
        let (p_lvl, a_lvl) = match self.status {
            VoiceCallStatus::Listening => {
                let env = (0.35 + 0.55 * (t * 6.3).sin().abs() * (t * 2.7 + 0.8).cos().abs())
                    .clamp(0.12, 0.94);
                (env, 0.02)
            }
            VoiceCallStatus::Thinking => (0.02, 0.02),
            VoiceCallStatus::Speaking => {
                let env = (0.30 + 0.62 * (t * 7.1 + 0.4).sin().abs() * (t * 2.9).cos().abs())
                    .clamp(0.10, 0.96);
                (0.02, env)
            }
            _ => (0.02, 0.02),
        };

        if !self.person_wave.is_empty() {
            self.person_wave.remove(0);
            self.person_wave.push(p_lvl);
        }
        if !self.agent_wave.is_empty() {
            self.agent_wave.remove(0);
            self.agent_wave.push(a_lvl);
        }

        // Update spoken word frontier on the last agent turn
        if let Some(agent_turn) = self
            .turns
            .iter_mut()
            .rev()
            .find(|t| t.speaker == VoiceSpeaker::Agent)
        {
            let total_w = agent_turn.text.split_whitespace().count().max(1);
            if self.status == VoiceCallStatus::Speaking {
                agent_turn.interrupted = false;
                let frac = ((phase - 3.1) / 5.2).clamp(0.08, 1.0);
                let revealed = ((total_w as f32 * frac).ceil() as usize).clamp(1, total_w);
                agent_turn.spoken_words = Some(revealed);
            } else if self.status == VoiceCallStatus::Thinking {
                agent_turn.spoken_words = Some(0);
            }
        }

        // Locate most recent person->agent silence gap in the wave buffers
        if let Some((g0, g1)) = find_silence_gap(&self.person_wave, &self.agent_wave) {
            self.pause_span_bins = (g0, g1);
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.key == "escape" {
            if self.settings_open {
                self.settings_open = false;
                cx.notify();
            } else if self.pause_tip_open {
                self.pause_tip_open = false;
                cx.notify();
            }
        }
    }

    fn render_pixel_mark(&self, busy: bool, elapsed_ms: f32, p: &ProPalette, cx: &App) -> Div {
        let reduced = Theme::global(cx).reduced_motion;
        let cells = agent_pixel_cells(&self.agent.id);
        let mut grid = div()
            .size(px(16.))
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(1.5));
        for r in 0..3usize {
            let mut row = div().flex().items_center().gap(px(1.5));
            for c in 0..3usize {
                let i = r * 3 + c;
                let on = cells[i];
                let alpha = if busy && !reduced {
                    let ph = ((elapsed_ms + AGENT_WAVE_MS[i]) / 1200.0) * std::f32::consts::TAU;
                    let wv = 0.56 + 0.44 * ph.cos();
                    if on { wv } else { 0.16 * wv }
                } else if on {
                    0.90
                } else {
                    0.0
                };
                row = row.child(
                    div()
                        .size(px(3.))
                        .rounded(px(0.8))
                        .bg(p.blue.opacity(alpha)),
                );
            }
            grid = grid.child(row);
        }
        grid
    }

    fn render_person_dot(&self, p: &ProPalette) -> Div {
        let init = person_initials(&self.person_name);
        let ch: String = init.chars().take(1).collect();
        div()
            .size(px(16.))
            .flex_none()
            .rounded_full()
            .bg(p.chip)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(9.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.text)
            .child(if ch.is_empty() { "Y".to_owned() } else { ch })
    }
}

impl Render for VoiceSession {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let compact = width < 400.0;
        let measure_cell = self.measured_width.clone();

        if self.loading {
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(480.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(20.))
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .child(pro_skeleton_bar(180., 16., 5., 0.1, cx))
                        .child(pro_skeleton_bar(420., 92., 8., 0.35, cx))
                        .child(pro_skeleton_bar(420., 80., 8., 0.6, cx)),
                );
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Mic,
                    tr(cx, "No active voice call", "当前无活跃语音通话"),
                    tr(
                        cx,
                        "Connect a live voice session to inspect bidirectional waveforms and turn latency.",
                        "建立实时语音会话后即可查看双向声波轨道与端到端延迟拆解。",
                    ),
                    None,
                    cx,
                ));
        }

        // Advance live 90ms audio waveform bins & call timer unless paused or ended
        let now = Instant::now();
        if !self.paused && self.status != VoiceCallStatus::Ended {
            if let Some(prev) = self.last_frame {
                let dt = now.duration_since(prev).as_secs_f32().min(0.10);
                self.call_accum_secs += dt;
                self.cycle_clock += dt;
                self.sample_accum += dt;
                while self.sample_accum >= 0.09 {
                    self.sample_accum -= 0.09;
                    self.step_simulation_tick();
                }
            }
            self.last_frame = Some(now);
            w.request_animation_frame();
        } else {
            self.last_frame = None;
        }

        let elapsed_ms = self.started_at.elapsed().as_secs_f32() * 1000.0;
        let agent_busy = matches!(
            self.status,
            VoiceCallStatus::Thinking | VoiceCallStatus::Speaking
        ) && !self.paused;

        // Header + Call settings menu
        let settings_menu = pro_presence(
            pro_eid(&self.id, "vsn-settings-pres"),
            self.settings_open,
            0.18,
            0.14,
            {
                let base_id = self.id.clone();
                let show_tr = self.show_transcript;
                let nr = self.noise_reduction;
                let rec = self.record_call;
                let bud = self.budget_ms;

                let mk_toggle = |key: &'static str,
                                 label: &str,
                                 checked: bool,
                                 handler: fn(&mut Self, &mut Context<Self>)|
                 -> Stateful<Div> {
                    div()
                        .id(pro_eid(&base_id, format!("vsn-opt-{key}")))
                        .role(Role::Button)
                        .tab_index(0)
                        .w_full()
                        .h(px(30.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .hover(|s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .cursor_pointer()
                        .text_size(px(12.5))
                        .text_color(p.ink)
                        .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
                        .child(label.to_owned())
                        .when(checked, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink)))
                };

                let tr_row = mk_toggle(
                    "transcript",
                    tr(cx, "Show transcript", "显示实时字幕"),
                    show_tr,
                    |this, cx| {
                        this.show_transcript = !this.show_transcript;
                        cx.emit(VoiceSessionEvent::SettingChanged {
                            key: "show_transcript".into(),
                        });
                        cx.notify();
                    },
                );
                let nr_row = mk_toggle(
                    "noise",
                    tr(cx, "Noise reduction", "AI 降噪"),
                    nr,
                    |this, cx| {
                        this.noise_reduction = !this.noise_reduction;
                        cx.emit(VoiceSessionEvent::SettingChanged {
                            key: "noise_reduction".into(),
                        });
                        cx.notify();
                    },
                );
                let rec_row = mk_toggle(
                    "record",
                    tr(cx, "Record call", "通话录音"),
                    rec,
                    |this, cx| {
                        this.record_call = !this.record_call;
                        cx.emit(VoiceSessionEvent::SettingChanged {
                            key: "record_call".into(),
                        });
                        cx.notify();
                    },
                );
                let bud_600 = mk_toggle("b600", "600 ms budget", bud == 600, |this, cx| {
                    this.budget_ms = 600;
                    cx.emit(VoiceSessionEvent::SettingChanged {
                        key: "budget_600".into(),
                    });
                    cx.notify();
                });
                let bud_800 = mk_toggle("b800", "800 ms budget", bud == 800, |this, cx| {
                    this.budget_ms = 800;
                    cx.emit(VoiceSessionEvent::SettingChanged {
                        key: "budget_800".into(),
                    });
                    cx.notify();
                });
                let sec_lbl = tr(cx, "Latency target", "延迟预算线").to_owned();

                move |amt| {
                    div()
                        .absolute()
                        .top(px(32.0 - 4.0 * (1.0 - amt)))
                        .right(px(0.))
                        .opacity(amt)
                        .w(px(210.))
                        .p(px(4.))
                        .rounded(px(10.))
                        .bg(if p.dark { rgb(0x202024).into() } else { p.card })
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_lg()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(tr_row)
                        .child(nr_row)
                        .child(rec_row)
                        .child(
                            div()
                                .mt(px(3.))
                                .pt(px(4.))
                                .border_t_1()
                                .border_color(p.line)
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .px(px(8.))
                                        .py(px(3.))
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .child(sec_lbl),
                                )
                                .child(bud_600)
                                .child(bud_800),
                        )
                        .into_any_element()
                }
            },
        );

        let head = div()
            .px(px(20.))
            .pt(px(16.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(self.render_pixel_mark(agent_busy, elapsed_ms, &p, cx))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(self.agent.name.clone()),
            )
            .when_some(self.subtitle.clone().filter(|_| !compact), |d, sub| {
                d.child(
                    div()
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .truncate()
                        .child(sub),
                )
            })
            .child(
                div()
                    .relative()
                    .ml_auto()
                    .child(
                        div()
                            .id(pro_eid(&self.id, "vsn-settings-btn"))
                            .role(Role::Button)
                            .aria_label(SharedString::from(tr(cx, "Call settings", "通话设置")))
                            .tab_index(0)
                            .size(px(28.))
                            .rounded(px(6.))
                            .bg(if self.settings_open {
                                p.chip
                            } else {
                                transparent_black()
                            })
                            .hover(|s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.settings_open = !this.settings_open;
                                cx.notify();
                            }))
                            .child(pro_icon(ProIcon::Sliders, 15., p.muted)),
                    )
                    .child(settings_menu),
            );

        // Turn badge + Status badge
        let turn_num = self.turns.len().max(1);
        let (status_lbl, status_bg, status_fg) = match self.status {
            VoiceCallStatus::Listening => {
                (tr(cx, "Listening", "正在倾听"), p.green_soft, p.green_ink)
            }
            VoiceCallStatus::Thinking => (tr(cx, "Thinking", "思考响应中"), p.accent_soft, p.blue),
            VoiceCallStatus::Speaking => (tr(cx, "Speaking", "正在播报"), p.accent_soft, p.blue),
            VoiceCallStatus::Connecting => (tr(cx, "Connecting", "连接中"), p.chip, p.muted),
            VoiceCallStatus::Ended => (tr(cx, "Ended", "已结束"), p.chip, p.muted),
        };

        // Live word-by-word transcript exchange
        let last_person = self
            .turns
            .iter()
            .rev()
            .find(|t| t.speaker == VoiceSpeaker::Person)
            .cloned();
        let last_agent = self
            .turns
            .iter()
            .rev()
            .find(|t| t.speaker == VoiceSpeaker::Agent)
            .cloned();

        let transcript_fold = pro_fold(
            pro_eid(&self.id, "vsn-tr-fold"),
            self.show_transcript,
            68.,
            0.,
            {
                let person_nm = self.person_name.clone();
                let agent_nm = self.agent.name.clone();
                let p_dot = self.render_person_dot(&p);
                let a_mark = self.render_pixel_mark(agent_busy, elapsed_ms, &p, cx);
                let cut_lbl = tr(cx, "Interrupted", "已打断").to_owned();
                let wait_lbl = tr(cx, "Preparing reply…", "正在组织语音回复…").to_owned();
                let cur_status = self.status;

                move || {
                    let render_words_row = |turn_opt: Option<VoiceTurn>, is_agent: bool| -> Div {
                        let mut words_wrap = div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_x(px(3.5))
                            .gap_y(px(2.))
                            .text_size(px(13.))
                            .line_height(px(19.));
                        if let Some(turn) = turn_opt {
                            let ws: Vec<&str> = turn.text.split_whitespace().collect();
                            let spoken = turn.spoken_words.unwrap_or(ws.len());
                            if is_agent && cur_status == VoiceCallStatus::Thinking && spoken == 0 {
                                words_wrap = words_wrap
                                    .child(div().text_color(p.muted).child(wait_lbl.clone()));
                            } else {
                                for (wi, w_str) in ws.into_iter().enumerate() {
                                    let said = wi < spoken;
                                    words_wrap = words_wrap.child(
                                        div()
                                            .text_color(if said { p.ink } else { p.faint })
                                            .child(w_str.to_owned()),
                                    );
                                }
                                if turn.interrupted {
                                    words_wrap = words_wrap.child(
                                        div()
                                            .h(px(18.))
                                            .px(px(6.))
                                            .rounded(px(5.))
                                            .bg(p.chip)
                                            .flex()
                                            .items_center()
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.text)
                                            .child(cut_lbl.clone()),
                                    );
                                }
                            }
                        }
                        words_wrap
                    };

                    div()
                        .mt(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .w(px(68.))
                                        .flex_none()
                                        .h(px(20.))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(12.))
                                        .text_color(p.text)
                                        .child(p_dot)
                                        .child(div().truncate().child(person_nm)),
                                )
                                .child(render_words_row(last_person, false)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .w(px(68.))
                                        .flex_none()
                                        .h(px(20.))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(12.))
                                        .text_color(p.text)
                                        .child(a_mark)
                                        .child(div().truncate().child(agent_nm)),
                                )
                                .child(render_words_row(last_agent, true)),
                        )
                        .into_any_element()
                }
            },
        );

        // Dual Audio Waveform Lanes + Measured Silence Pause (PathBuilder)
        let lat = self.active_latency();
        let total_ms = lat.total_ms();
        let over_budget = total_ms > self.budget_ms;
        let p_wave = self.person_wave.clone();
        let a_wave = self.agent_wave.clone();
        let (gap_start, gap_end) = self.pause_span_bins;
        let is_thinking = self.status == VoiceCallStatus::Thinking && !self.paused;
        let shimmer_u = (self.call_accum_secs * 1.4).fract();
        let glide_frac = if reduced || self.paused {
            0.0
        } else {
            (self.sample_accum / 0.09).clamp(0.0, 1.0)
        };

        let person_col = p.ink.opacity(0.84);
        let agent_col = p.blue;
        let dot_col = p.line_strong;
        let bracket_col = if over_budget {
            p.red_ink.opacity(0.65)
        } else {
            p.muted.opacity(0.55)
        };

        let wave_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw < 20.0 || bh < 30.0 {
                    return;
                }
                let ox = f32::from(bounds.origin.x);
                let oy = f32::from(bounds.origin.y);
                let n = p_wave.len().min(a_wave.len()).max(1);
                let step_x = bw / n as f32;
                let y_person = oy + 18.0;
                let y_mid = oy + bh * 0.5;
                let y_agent = oy + bh - 18.0;
                let max_half = 14.0;

                for i in 0..n {
                    let x = ox + (i as f32 + 0.5 - glide_frac) * step_x;
                    if x < ox + 1.0 || x > ox + bw - 1.0 {
                        continue;
                    }
                    // Left-edge soft fade (first 16% of track)
                    let edge_fade = ((x - ox) / (bw * 0.16)).clamp(0.15, 1.0);

                    // Top lane: Person
                    let pv = p_wave[i].clamp(0.0, 1.0);
                    let ph = (pv * max_half).max(1.2);
                    let mut pb_p = PathBuilder::stroke(px(2.3));
                    pb_p.move_to(point(px(x), px(y_person - ph)));
                    pb_p.line_to(point(px(x), px(y_person + ph)));
                    if let Ok(path) = pb_p.build() {
                        let c = if pv < 0.06 {
                            dot_col.opacity(edge_fade)
                        } else {
                            person_col.opacity(edge_fade)
                        };
                        window.paint_path(path, c);
                    }

                    // Bottom lane: Agent (with blue shimmer wave while Thinking)
                    let av = a_wave[i].clamp(0.0, 1.0);
                    let ah = (av * max_half).max(1.2);
                    let mut pb_a = PathBuilder::stroke(px(2.3));
                    pb_a.move_to(point(px(x), px(y_agent - ah)));
                    pb_a.line_to(point(px(x), px(y_agent + ah)));
                    if let Ok(path) = pb_a.build() {
                        let u = i as f32 / n as f32;
                        let c = if av >= 0.06 {
                            agent_col.opacity(edge_fade)
                        } else if is_thinking && (u - shimmer_u).abs() < 0.16 {
                            let glint = 1.0 - ((u - shimmer_u).abs() / 0.16);
                            agent_col.opacity(0.25 + 0.65 * glint)
                        } else {
                            dot_col.opacity(edge_fade)
                        };
                        window.paint_path(path, c);
                    }
                }

                // Silence pause bracket between person speech end and agent speech start
                let max_r0 = (bw - 30.0).max(6.0);
                let max_r1 = (bw - 4.0).max(6.0);
                let rel0 = (gap_start as f32 * step_x).clamp(6.0, max_r0);
                let min_r1 = (rel0 + 24.0).min(max_r1);
                let rel1 = (gap_end as f32 * step_x).clamp(min_r1, max_r1);
                let gx0 = ox + rel0;
                let gx1 = ox + rel1;
                let mut brk = PathBuilder::stroke(px(1.1));
                brk.move_to(point(px(gx0), px(y_mid - 4.5)));
                brk.line_to(point(px(gx0), px(y_mid + 4.5)));
                brk.move_to(point(px(gx0), px(y_mid)));
                brk.line_to(point(px(gx1), px(y_mid)));
                brk.move_to(point(px(gx1), px(y_mid - 4.5)));
                brk.line_to(point(px(gx1), px(y_mid + 4.5)));
                if let Ok(path) = brk.build() {
                    window.paint_path(path, bracket_col);
                }
            },
        )
        .size_full();

        let mid_ratio = ((gap_start + gap_end) as f32 * 0.5 / WAVE_BINS as f32).clamp(0.18, 0.82);

        let pause_badge = div()
            .id(pro_eid(&self.id, "vsn-pause-pill"))
            .role(Role::Button)
            .aria_label(SharedString::from(format!(
                "Silence pause {total_ms} ms: STT {} ms, LLM {} ms, TTS {} ms",
                lat.stt_ms, lat.llm_ms, lat.tts_ms
            )))
            .tab_index(0)
            .h(px(19.))
            .px(px(6.))
            .rounded(px(5.))
            .bg(if over_budget { p.red_soft } else { p.card })
            .border_1()
            .border_color(if over_budget {
                p.red.opacity(0.45)
            } else {
                p.line_strong
            })
            .shadow_xs()
            .flex()
            .items_center()
            .cursor_pointer()
            .text_size(px(11.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if over_budget {
                p.red_ink
            } else if is_thinking {
                p.blue
            } else {
                p.text
            })
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if this.pause_tip_open != *inside {
                    this.pause_tip_open = *inside;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(|this, _, _, cx| {
                this.pause_tip_open = !this.pause_tip_open;
                cx.notify();
            }))
            .child(format!("{total_ms} ms"));

        let pause_tooltip = self.pause_tip_open.then(|| {
            div()
                .absolute()
                .bottom(px(54.))
                .left(relative(mid_ratio))
                .ml(px(-92.))
                .w(px(186.))
                .p(px(8.))
                .rounded(px(8.))
                .bg(if p.dark { rgb(0xf4f4f5) } else { rgb(0x18181b) })
                .text_color(if p.dark { rgb(0x111113) } else { rgb(0xfafafa) })
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(4.))
                .text_size(px(11.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!("{total_ms} ms")),
                        )
                        .child(
                            div()
                                .opacity(0.7)
                                .child(tr(cx, "silence gap", "轮次静音间隔")),
                        ),
                )
                .child(
                    div()
                        .pt(px(4.))
                        .border_t_1()
                        .border_color(hsla(0., 0., 0.5, 0.25))
                        .flex()
                        .justify_between()
                        .child("STT + LLM + TTS")
                        .child(format!("{}+ {}+ {} ms", lat.stt_ms, lat.llm_ms, lat.tts_ms)),
                )
        });

        let lanes_section = div()
            .mt(px(16.))
            .w_full()
            .h(px(86.))
            .flex()
            .items_stretch()
            .gap(px(8.))
            .child(
                div()
                    .w(px(if compact { 22. } else { 68. }))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .py(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(p.text)
                            .child(self.render_person_dot(&p))
                            .when(!compact, |d| {
                                d.child(div().truncate().child(self.person_name.clone()))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(p.text)
                            .child(self.render_pixel_mark(agent_busy, elapsed_ms, &p, cx))
                            .when(!compact, |d| {
                                d.child(div().truncate().child(self.agent.name.clone()))
                            }),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(wave_canvas)
                    .child(
                        div()
                            .absolute()
                            .top(px(33.5))
                            .left(relative(mid_ratio))
                            .ml(px(-28.))
                            .child(pause_badge),
                    )
                    .when_some(pause_tooltip, |d, tip| d.child(tip)),
            );

        // Panel 2: Stacked Voice-to-Voice Latency Breakdown vs Target Budget Line
        let scale_max = self.budget_ms.max(total_ms).max(1) as f32 * 1.12;
        let budget_ratio = pro_spring_f32(
            pro_eid(&self.id, "vsn-bud-x"),
            (self.budget_ms as f32 / scale_max).clamp(0.1, 0.98),
            0.30,
            w,
            cx,
        );

        let seg_colors: [Hsla; 5] = if p.dark {
            [
                hsla(0., 0., 1., 0.32),
                rgb(0xa78bfa).into(),
                rgb(0x3aa8ff).into(),
                rgb(0xf472b6).into(),
                rgb(0xf9a8d4).into(),
            ]
        } else {
            [
                hsla(0., 0., 0., 0.24),
                rgb(0x5e3ee0).into(),
                rgb(0x0090fe).into(),
                rgb(0xdd0674).into(),
                rgb(0xf28fc0).into(),
            ]
        };

        let seg_meta = [
            (tr(cx, "Endpoint", "端点检测"), lat.endpoint_ms),
            (tr(cx, "STT", "语音转写 STT"), lat.stt_ms),
            (tr(cx, "LLM", "模型推理 LLM"), lat.llm_ms),
            (tr(cx, "TTS", "语音合成 TTS"), lat.tts_ms),
            (tr(cx, "Network", "网络传输"), lat.network_ms),
        ];

        let mut stacked_bar = div()
            .relative()
            .mt(px(14.))
            .w_full()
            .h(px(7.))
            .rounded(px(3.5))
            .bg(p.subtle)
            .flex()
            .items_center()
            .gap(px(2.));

        let raw_vals = lat.as_array();
        for (si, &ms_val) in raw_vals.iter().enumerate() {
            let frac = pro_spring_f32(
                pro_eid(&self.id, format!("vsn-seg-{si}")),
                (ms_val as f32 / scale_max).clamp(0.01, 0.95),
                0.34,
                w,
                cx,
            );
            let dimmed = self.hovered_slice.is_some() && self.hovered_slice != Some(si);
            stacked_bar = stacked_bar.child(
                div()
                    .h_full()
                    .w(relative(frac))
                    .rounded(px(2.))
                    .bg(seg_colors[si])
                    .opacity(if dimmed { 0.26 } else { 1.0 }),
            );
        }

        // Target latency budget vertical line
        stacked_bar = stacked_bar.child(
            div()
                .absolute()
                .left(relative(budget_ratio))
                .top(px(-4.))
                .bottom(px(-4.))
                .w(px(1.5))
                .bg(if over_budget { p.red_ink } else { p.text }),
        );

        let diff_ms = total_ms.abs_diff(self.budget_ms);
        let budget_badge_txt = if over_budget {
            format!("{diff_ms} ms {}", tr(cx, "over", "超预算"))
        } else {
            format!("{diff_ms} ms {}", tr(cx, "under", "低于预算"))
        };

        let mut legend_grid = div()
            .mt(px(14.))
            .w_full()
            .flex()
            .flex_wrap()
            .gap_x(px(14.))
            .gap_y(px(6.));

        for (li, (lbl, ms_v)) in seg_meta.into_iter().enumerate() {
            let dimmed = self.hovered_slice.is_some() && self.hovered_slice != Some(li);
            let hot = self.hovered_slice == Some(li);
            legend_grid = legend_grid.child(
                div()
                    .id(pro_eid(&self.id, format!("vsn-leg-{li}")))
                    .role(Role::Button)
                    .tab_index(0)
                    .w(px(if compact { 126. } else { 132. }))
                    .px(px(5.))
                    .py(px(3.))
                    .rounded(px(6.))
                    .bg(if hot { p.hover } else { transparent_black() })
                    .opacity(if dimmed { 0.32 } else { 1.0 })
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        let next = (*inside).then_some(li);
                        if this.hovered_slice != next {
                            this.hovered_slice = next;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.hovered_slice = if this.hovered_slice == Some(li) {
                            None
                        } else {
                            Some(li)
                        };
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .min_w_0()
                            .child(
                                div()
                                    .size(px(7.))
                                    .flex_none()
                                    .rounded(px(2.))
                                    .bg(seg_colors[li]),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.text)
                                    .truncate()
                                    .child(lbl),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(format!("{ms_v} ms")),
                    ),
            );
        }

        // Footer stats & Pause / Barge in controls
        let total_call_s = (self.base_call_secs + self.call_accum_secs).floor() as u32;
        let call_clock = format!("{:02}:{:02}", total_call_s / 60, total_call_s % 60);
        let p50_ms = self.p50_latency_ms();
        let sec_btn_bg = if p.dark { rgb(0x2c2c2f).into() } else { p.card };

        let mk_stat = |icon_el: AnyElement, label: String| -> Div {
            div()
                .h(px(22.))
                .px(px(7.))
                .rounded(px(6.))
                .bg(p.chip)
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.))
                .text_color(p.text)
                .when(!compact, |d| d.child(icon_el))
                .child(label)
        };

        let footer = div()
            .w_full()
            .min_h(px(60.))
            .px(px(20.))
            .py(px(12.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(mk_stat(pro_icon(ProIcon::Clock, 13., p.muted), call_clock))
                    .child(mk_stat(
                        pro_icon(ProIcon::Thread, 13., p.muted),
                        format!("{turn_num} {}", tr(cx, "turns", "轮")),
                    ))
                    .child(mk_stat(
                        pro_svg("0 0 24 24", GAUGE_STAT_SVG, 13., p.muted),
                        format!("P50 {p50_ms} ms"),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .id(pro_eid(&self.id, "vsn-pause-btn"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(28.))
                            .px(px(10.))
                            .rounded(px(6.))
                            .bg(sec_btn_bg)
                            .border_1()
                            .border_color(p.line_strong)
                            .shadow_xs()
                            .hover(|s| s.bg(p.chip_hover))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .cursor_pointer()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_pause(cx)))
                            .child(pro_icon(
                                if self.paused {
                                    ProIcon::Play
                                } else {
                                    ProIcon::Pause
                                },
                                13.,
                                p.muted,
                            ))
                            .child(if self.paused {
                                tr(cx, "Resume", "继续")
                            } else {
                                tr(cx, "Pause", "暂停")
                            }),
                    )
                    .child(
                        div()
                            .id(pro_eid(&self.id, "vsn-barge-btn"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(28.))
                            .px(px(10.))
                            .rounded(px(6.))
                            .bg(p.primary)
                            .text_color(p.on_primary)
                            .hover(|s| s.opacity(0.9))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .cursor_pointer()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .on_click(cx.listener(|this, _, _, cx| this.barge_in(cx)))
                            .child(pro_icon(ProIcon::Mic, 13., p.on_primary))
                            .child(tr(cx, "Barge in", "插话打断")),
                    ),
            );

        let card = div()
            .w_full()
            .max_w(px(480.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow_sm()
            .flex()
            .flex_col()
            .child(head)
            .child(
                div()
                    .px(px(20.))
                    .pt(px(16.))
                    .pb(px(22.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(format!("{} {turn_num}", tr(cx, "Turn", "第"))),
                            )
                            .child(
                                div()
                                    .h(px(20.))
                                    .px(px(7.))
                                    .rounded(px(5.))
                                    .bg(status_bg)
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(status_fg)
                                    .child(status_lbl),
                            ),
                    )
                    .child(transcript_fold)
                    .child(lanes_section),
            )
            .child(
                div()
                    .px(px(20.))
                    .py(px(20.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap(px(4.))
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "vsn-big-ms"),
                                        total_ms as f64,
                                        0,
                                        "",
                                        "",
                                        20.,
                                        FontWeight::MEDIUM,
                                        p.strong,
                                    ))
                                    .child(div().text_size(px(12.)).text_color(p.muted).child("ms"))
                                    .when(!compact, |d| {
                                        d.child(
                                            div()
                                                .ml(px(5.))
                                                .text_size(px(12.))
                                                .text_color(p.muted)
                                                .child(format!(
                                                    "{} ({} ms {})",
                                                    tr(cx, "voice to voice", "端到端延迟"),
                                                    self.budget_ms,
                                                    tr(cx, "budget", "预算")
                                                )),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .h(px(20.))
                                    .px(px(7.))
                                    .rounded(px(5.))
                                    .bg(if over_budget {
                                        p.red_soft
                                    } else {
                                        p.green_soft
                                    })
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if over_budget { p.red_ink } else { p.green_ink })
                                    .child(budget_badge_txt),
                            ),
                    )
                    .child(stacked_bar)
                    .child(legend_grid),
            )
            .child(footer);

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(SharedString::from(format!(
                "Voice call with {}",
                self.agent.name
            )))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .flex()
            .justify_center()
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10.0 && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .w(px(0.))
                .h(px(0.)),
            )
            .child(card)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

fn find_silence_gap(person: &[f32], agent: &[f32]) -> Option<(usize, usize)> {
    let n = person.len().min(agent.len());
    if n < 12 {
        return None;
    }
    // Look for a transition from active person speech (>0.12) to quiet gap to active agent speech (>0.12)
    let mut last_p_end = None;
    for i in 2..(n - 4) {
        if person[i - 1] > 0.12 && person[i] <= 0.10 {
            last_p_end = Some(i);
        }
        if let Some(p_end) = last_p_end
            && i > p_end + 2
            && agent[i] > 0.12
        {
            return Some((p_end, i));
        }
    }
    None
}

fn initial_waveform_buffers() -> (Vec<f32>, Vec<f32>, (usize, usize)) {
    let mut pw = vec![0.02f32; WAVE_BINS];
    let mut aw = vec![0.02f32; WAVE_BINS];
    for (idx, slot) in pw.iter_mut().enumerate().take(21).skip(3) {
        let u = idx as f32 * 0.65;
        *slot = (0.28 + 0.56 * u.sin().abs()).clamp(0.14, 0.90);
    }
    for (idx, slot) in aw.iter_mut().enumerate().take(WAVE_BINS).skip(29) {
        let u = idx as f32 * 0.72 + 0.4;
        *slot = (0.32 + 0.58 * u.cos().abs()).clamp(0.15, 0.92);
    }
    (pw, aw, (21, 29))
}

fn compute_p50_ms(turns: &[VoiceTurn]) -> u32 {
    let mut vals: Vec<u32> = turns
        .iter()
        .filter_map(|t| t.latency.map(|l| l.total_ms()))
        .collect();
    if vals.is_empty() {
        return 0;
    }
    vals.sort_unstable();
    let mid = vals.len() / 2;
    if vals.len() % 2 == 1 {
        vals[mid]
    } else {
        (vals[mid - 1] + vals[mid]) / 2
    }
}

fn default_voice_turns() -> Vec<VoiceTurn> {
    vec![
        VoiceTurn::new(
            "t1",
            VoiceSpeaker::Person,
            "Hi, I saw an unexpected charge on our workspace card this morning.",
        ),
        VoiceTurn::new(
            "t2",
            VoiceSpeaker::Agent,
            "I see invoice #4821 on your account—let me check the billing cycle.",
        )
        .with_latency(VoiceLatencyBreakdown::new(165, 68, 235, 120, 42)),
        VoiceTurn::new(
            "t3",
            VoiceSpeaker::Person,
            "Right, we thought we were still on monthly seats.",
        ),
        VoiceTurn::new(
            "t4",
            VoiceSpeaker::Agent,
            "Your plan switched to Annual Pro when two seats were added on October 12.",
        )
        .with_latency(VoiceLatencyBreakdown::new(180, 72, 230, 112, 46)),
        VoiceTurn::new(
            "t5",
            VoiceSpeaker::Person,
            "So why was I charged for the year instead of monthly?",
        ),
        VoiceTurn::new(
            "t6",
            VoiceSpeaker::Agent,
            "Annual Pro bills upfront at a 20% discount—I can switch you back to monthly and refund the difference now.",
        )
        .with_spoken_words(11)
        .with_latency(VoiceLatencyBreakdown::new(175, 70, 220, 115, 45)),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        VoiceCallStatus, VoiceLatencyBreakdown, compute_p50_ms, default_voice_turns,
        find_silence_gap, initial_waveform_buffers,
    };

    #[test]
    fn latency_breakdown_totals_and_p50_match_turn_data() {
        let turns = default_voice_turns();
        let active = turns.last().and_then(|t| t.latency).unwrap_or_default();
        assert_eq!(active.total_ms(), 625);
        assert_eq!(compute_p50_ms(&turns), 630);
        assert_eq!(VoiceCallStatus::default(), VoiceCallStatus::Speaking);
    }

    #[test]
    fn initial_waveforms_detect_turn_taking_silence_pause() {
        let (pw, aw, span) = initial_waveform_buffers();
        assert_eq!(span, (21, 29));
        assert_eq!(find_silence_gap(&pw, &aw), Some((21, 29)));
        let custom = VoiceLatencyBreakdown::new(200, 90, 310, 160, 60);
        assert!(custom.total_ms() > 800);
    }
}
