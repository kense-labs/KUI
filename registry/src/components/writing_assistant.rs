//! Floating AI writing companion with atmospheric glow, streaming replies, and dictation.
//!
//! Renders a compact writing assistant card with a three-lobe drifting color glow,
//! quick rewrite suggestion chips, word-by-word streaming drafts with accept, retry,
//! and copy actions, a multiline [`Editor`] composer, and live voice waveform dictation.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_eid, pro_empty_state, pro_presence, pro_shimmer_text, pro_skeleton_bar,
        pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_raw_svg, pro_svg},
    },
};
use gpui::{prelude::*, *};
use image::{Frame, RgbaImage};
use std::{cell::RefCell, f32::consts::TAU, rc::Rc, sync::Arc};
use web_time::Instant;

const SPARK_LIGHT_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 16 16" fill="none"><defs><linearGradient id="kui_wa_gl" x1="2" y1="2" x2="14" y2="14" gradientUnits="userSpaceOnUse"><stop stop-color="#EE3E9E"/><stop offset="0.5" stop-color="#A651F0"/><stop offset="1" stop-color="#4FB0FE"/></linearGradient></defs><path fill="url(#kui_wa_gl)" d="M8 1.5c.3 2.9 1.6 4.2 4.5 4.5-2.9.3-4.2 1.6-4.5 4.5-.3-2.9-1.6-4.2-4.5-4.5 2.9-.3 4.2-1.6 4.5-4.5Z"/><path fill="url(#kui_wa_gl)" d="M12.25 10c.13 1.3.7 1.87 2 2-1.3.13-1.87.7-2 2-.13-1.3-.7-1.87-2-2 1.3-.13 1.87-.7 2-2Z"/></svg>"##;

const SPARK_DARK_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 16 16" fill="none"><defs><linearGradient id="kui_wa_gd" x1="2" y1="2" x2="14" y2="14" gradientUnits="userSpaceOnUse"><stop stop-color="#F06DB5"/><stop offset="0.5" stop-color="#B98AF0"/><stop offset="1" stop-color="#4FB0FE"/></linearGradient></defs><path fill="url(#kui_wa_gd)" d="M8 1.5c.3 2.9 1.6 4.2 4.5 4.5-2.9.3-4.2 1.6-4.5 4.5-.3-2.9-1.6-4.2-4.5-4.5 2.9-.3 4.2-1.6 4.5-4.5Z"/><path fill="url(#kui_wa_gd)" d="M12.25 10c.13 1.3.7 1.87 2 2-1.3.13-1.87.7-2 2-.13-1.3-.7-1.87-2-2 1.3-.13 1.87-.7 2-2Z"/></svg>"##;

const ICON_MIC: &str = r#"<path d="M12 3.5a3 3 0 0 1 3 3v5a3 3 0 0 1-6 0v-5a3 3 0 0 1 3-3ZM6 11.5a6 6 0 0 0 12 0M12 17.5v3" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round"/>"#;

const GLOW_W: usize = 96;
const GLOW_H: usize = 84;
const THINKING_SECS: f32 = 0.24;
const WORD_STEP_SECS: f32 = 0.034;
const DICTATION_WORD_SECS: f32 = 0.14;

/// Quick suggestion chip shown on the [`WritingAssistant`] welcome view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WritingSuggestion {
    pub id: SharedString,
    pub label: SharedString,
    pub prompt: SharedString,
}

impl WritingSuggestion {
    /// Create a suggestion chip whose label also acts as the prompt unless overridden.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            id: id.into(),
            prompt: label.clone(),
            label,
        }
    }

    /// Override the full prompt submitted when the chip is activated.
    pub fn with_prompt(mut self, prompt: impl Into<SharedString>) -> Self {
        self.prompt = prompt.into();
        self
    }
}

/// Author role for a [`WritingMessage`] inside [`WritingAssistant`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WritingRole {
    User,
    Assistant,
}

/// Conversation turn rendered inside [`WritingAssistant`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WritingMessage {
    pub role: WritingRole,
    pub text: SharedString,
    pub accepted: bool,
}

impl WritingMessage {
    /// Construct a user message turn.
    pub fn user(text: impl Into<SharedString>) -> Self {
        Self {
            role: WritingRole::User,
            text: text.into(),
            accepted: false,
        }
    }

    /// Construct an assistant message turn.
    pub fn assistant(text: impl Into<SharedString>) -> Self {
        Self {
            role: WritingRole::Assistant,
            text: text.into(),
            accepted: false,
        }
    }
}

/// Speech input behavior for the microphone control on [`WritingAssistant`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpeechMode {
    /// Stream simulated dictation into the prompt field when toggled.
    #[default]
    Simulate,
    /// Hide the voice dictation button when voice input is not desired.
    Hidden,
}

/// Typed events emitted by [`WritingAssistant`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WritingAssistantEvent {
    /// User submitted a prompt from a chip, keyboard, Send button, or completed dictation.
    Submitted(SharedString),
    /// User accepted/inserted the assistant's latest draft into the host document.
    Accepted(SharedString),
    /// User requested a fresh rewrite variation for the latest prompt.
    Retried {
        prompt: SharedString,
        attempt: usize,
    },
    /// User copied an assistant reply to the clipboard.
    Copied(SharedString),
    /// Voice dictation toggled on (`true`) or off (`false`).
    DictationToggled(bool),
    /// Conversation reset to the welcome view.
    NewChat,
}

/// Custom synchronous responder `(prompt, attempt, language_is_zh) -> reply`.
pub type WritingResponder = Rc<dyn Fn(&str, usize, bool) -> SharedString>;

#[derive(Clone, Copy, PartialEq, Eq)]
struct GlowCacheKey {
    dark: bool,
    dim_q: u8,
    phase_q: u16,
}

struct GlowSurface {
    key: Option<GlowCacheKey>,
    image: Option<Arc<RenderImage>>,
    raw: Vec<u8>,
}

impl GlowSurface {
    fn new() -> Self {
        Self {
            key: None,
            image: None,
            raw: vec![0u8; GLOW_W * GLOW_H * 4],
        }
    }

    fn frame(
        &mut self,
        dark: bool,
        intensity: f32,
        elapsed_secs: f32,
        static_pose: bool,
        window: &mut Window,
    ) -> Arc<RenderImage> {
        let dim_q = (intensity.clamp(0., 1.) * 100.).round() as u8;
        let phase_q = if static_pose {
            u16::MAX
        } else {
            (((elapsed_secs % 42.0) / 42.0) * 840.0).round() as u16
        };
        let key = GlowCacheKey {
            dark,
            dim_q,
            phase_q,
        };
        if self.key == Some(key)
            && let Some(existing) = &self.image
        {
            return existing.clone();
        }

        rasterize_glow(
            dark,
            intensity,
            if static_pose { 0.0 } else { elapsed_secs },
            &mut self.raw,
        );
        let buf = RgbaImage::from_raw(GLOW_W as u32, GLOW_H as u32, self.raw.clone())
            .expect("GLOW_W * GLOW_H * 4 matches raw buffer length");
        let next = Arc::new(RenderImage::new(vec![Frame::new(buf)]));
        if let Some(prev) = self.image.replace(next.clone()) {
            let _ = window.drop_image(prev);
        }
        self.key = Some(key);
        next
    }
}

fn ping_pong(t: f32, period: f32) -> f32 {
    let u = (t / period).rem_euclid(2.0);
    let tri = if u <= 1.0 { u } else { 2.0 - u };
    tri * tri * (3.0 - 2.0 * tri)
}

fn eval_three_stop(
    u: f32,
    p0: (f32, f32, f32),
    p50: (f32, f32, f32),
    p100: (f32, f32, f32),
) -> (f32, f32, f32) {
    if u <= 0.5 {
        let k = u * 2.0;
        let s = k * k * (3.0 - 2.0 * k);
        (
            p0.0 + (p50.0 - p0.0) * s,
            p0.1 + (p50.1 - p0.1) * s,
            p0.2 + (p50.2 - p0.2) * s,
        )
    } else {
        let k = (u - 0.5) * 2.0;
        let s = k * k * (3.0 - 2.0 * k);
        (
            p50.0 + (p100.0 - p50.0) * s,
            p50.1 + (p100.1 - p50.1) * s,
            p50.2 + (p100.2 - p50.2) * s,
        )
    }
}

fn rasterize_glow(dark: bool, intensity: f32, t: f32, raw: &mut [u8]) {
    let bg: [f32; 3] = if dark {
        [0x1c as f32 / 255., 0x1c as f32 / 255., 0x1e as f32 / 255.]
    } else {
        [1.0, 1.0, 1.0]
    };
    let pink: [f32; 3] = if dark {
        [0xf0 as f32 / 255., 0x6d as f32 / 255., 0xb5 as f32 / 255.]
    } else {
        [0xee as f32 / 255., 0x3e as f32 / 255., 0x9e as f32 / 255.]
    };
    let violet: [f32; 3] = if dark {
        [0xb9 as f32 / 255., 0x8a as f32 / 255., 0xf0 as f32 / 255.]
    } else {
        [0xa6 as f32 / 255., 0x51 as f32 / 255., 0xf0 as f32 / 255.]
    };
    let blue: [f32; 3] = [0x4f as f32 / 255., 0xb0 as f32 / 255., 0xfe as f32 / 255.];
    let base_alpha = (if dark { 0.175 } else { 0.145 }) * intensity.clamp(0., 1.);

    let ua = ping_pong(t, 6.0);
    let ub = ping_pong(t, 7.0);
    let uc = ping_pong(t, 5.5);

    let da = eval_three_stop(ua, (0.0, 0.0, 1.0), (0.55, -0.22, 1.25), (0.20, 0.14, 0.85));
    let db = eval_three_stop(
        ub,
        (0.0, 0.0, 1.0),
        (-0.55, 0.24, 0.85),
        (-0.22, -0.16, 1.25),
    );
    let dc = eval_three_stop(
        uc,
        (0.0, 0.0, 1.0),
        (-0.45, -0.26, 1.20),
        (0.30, -0.10, 0.88),
    );

    let lobes: [([f32; 3], f32, f32, f32, f32); 3] = [
        (
            pink,
            0.11 + 0.58 * da.0,
            0.74 + 0.40 * da.1,
            0.35 * da.2,
            0.28 * da.2,
        ),
        (
            violet,
            0.89 + 0.54 * db.0,
            0.67 + 0.42 * db.1,
            0.33 * db.2,
            0.29 * db.2,
        ),
        (
            blue,
            0.53 + 0.62 * dc.0,
            0.96 + 0.40 * dc.1,
            0.37 * dc.2,
            0.27 * dc.2,
        ),
    ];

    for y in 0..GLOW_H {
        let ny = (y as f32 + 0.5) / GLOW_H as f32;
        for x in 0..GLOW_W {
            let nx = (x as f32 + 0.5) / GLOW_W as f32;
            let mut r = bg[0];
            let mut g = bg[1];
            let mut b = bg[2];
            for &(col, cx_n, cy_n, rx, ry) in &lobes {
                let dx = (nx - cx_n) / rx.max(0.08);
                let dy = (ny - cy_n) / ry.max(0.08);
                let d2 = dx * dx + dy * dy;
                if d2 < 4.5 {
                    let w = (-1.15 * d2).exp() * base_alpha;
                    r = r * (1.0 - w) + col[0] * w;
                    g = g * (1.0 - w) + col[1] * w;
                    b = b * (1.0 - w) + col[2] * w;
                }
            }
            let dst = (y * GLOW_W + x) * 4;
            raw[dst] = (b.clamp(0., 1.) * 255.).round() as u8;
            raw[dst + 1] = (g.clamp(0., 1.) * 255.).round() as u8;
            raw[dst + 2] = (r.clamp(0., 1.) * 255.).round() as u8;
            raw[dst + 3] = 255;
        }
    }
}

fn default_suggestions(zh: bool) -> Vec<WritingSuggestion> {
    if zh {
        vec![
            WritingSuggestion::new("polish", "润色邮件")
                .with_prompt("帮我润色这段客户跟进邮件，让表达更清晰得体。"),
            WritingSuggestion::new("shorter", "精简篇幅")
                .with_prompt("把这段更新压缩到两句话以内，只保留核心结论。"),
            WritingSuggestion::new("warmer", "语气更温和")
                .with_prompt("把这段项目延期说明改得更真诚温和，同时给出明确时间点。"),
            WritingSuggestion::new("translate", "多语翻译")
                .with_prompt("将发布说明翻译成自然地道的英文产品公告。"),
            WritingSuggestion::new("reply", "起草回复")
                .with_prompt("起草一段简短回复，确认周四下午三点评审设计稿。"),
        ]
    } else {
        vec![
            WritingSuggestion::new("polish", "Polish my email")
                .with_prompt("Polish my follow-up email so it reads clearly and confidently."),
            WritingSuggestion::new("shorter", "Make it shorter")
                .with_prompt("Shorten this status update to two crisp sentences."),
            WritingSuggestion::new("warmer", "Make warmer")
                .with_prompt("Make the tone warmer while keeping the timeline clear."),
            WritingSuggestion::new("translate", "Translate")
                .with_prompt("Translate our release note into natural Japanese and Spanish."),
            WritingSuggestion::new("reply", "Draft a reply")
                .with_prompt("Draft a concise reply confirming Thursday at 3 PM for the review."),
        ]
    }
}

/// Built-in sample writer returning context-aware drafts and alternate retry variations.
pub fn sample_writing_response(prompt: &str, attempt: usize, zh: bool) -> SharedString {
    let lower = prompt.to_ascii_lowercase();
    let var = attempt % 3;
    if zh {
        if prompt.contains("润色") || lower.contains("polish") {
            return match var {
                0 => "您好，想跟进一下我们上周同步的方案进展。如果您这边已经看完核心条款，我们可以约在周四下午用十五分钟对齐下一步安排。".into(),
                1 => "感谢您抽空审阅提案。若目前方向符合预期，我很乐意在本周四前配合完成上线排期与细节确认。".into(),
                _ => "跟进一下上周发您的合作草案——随时告诉我您的修改意见，我们当天即可更新定稿。".into(),
            };
        }
        if prompt.contains("精简") || lower.contains("short") {
            return match var {
                0 => {
                    "本周已完成核心接口联调与压测，预计周五按时灰度发布。唯一待定项是法务终审签字。"
                        .into()
                }
                1 => "核心联调已全部通过，周五准时开启灰度；法务确认后即可全量上线。".into(),
                _ => "接口与压测均已就绪，待法务签字后于周五正式发布。".into(),
            };
        }
        if prompt.contains("温和") || lower.contains("warm") || lower.contains("tone") {
            return match var {
                0 => "非常感谢大家的耐心配合。为了让交付体验更稳妥，我们将发布时间顺延至周三上午十点，届时会第一时间同步最新进展。".into(),
                1 => "感谢团队一路支持！我们正在打磨最后几处交互细节，定于周三上午十点正式上线，确保呈现最完整的效果。".into(),
                _ => "想提前和您同步一下：我们多预留了一天做全链路验证，周三上午十点准时交付，感谢理解与支持。".into(),
            };
        }
        if prompt.contains("翻译") || lower.contains("translate") {
            return match var {
                0 => "English: \"Introducing live workspace sync — edits, comments, and approvals now update instantaneously across desktop and web.\"".into(),
                1 => "English: \"Real-time workspace sync is here. Every edit and review stays effortlessly aligned across all your devices.\"".into(),
                _ => "English: \"Ship faster together: instant cross-platform sync is now live for every workspace.\"".into(),
            };
        }
        return match var {
            0 => "收到，周四下午 3:00 完全没问题。我已经把最新版设计稿链接附在日历邀请里，届时我们重点过一下交互细节。".into(),
            1 => "周四下午三点见！我提前整理好了三处关键改动说明，方便我们高效对齐。".into(),
            _ => "已确认周四 15:00 的评审会议，期待听取您的反馈意见。".into(),
        };
    }

    if lower.contains("polish") || lower.contains("email") {
        match var {
            0 => "Hi Maya — following up on the proposal we shared on Tuesday. If the scope looks right on your end, let's lock in 15 minutes on Thursday to align on rollout.".into(),
            1 => "Hi Maya, thanks again for reviewing the draft proposal. Whenever you're ready, I'd love to confirm the timeline so our team can reserve next week's build slot.".into(),
            _ => "Quick check-in on Tuesday's proposal — happy to adjust any milestones before we kick off on Thursday.".into(),
        }
    } else if lower.contains("short") {
        match var {
            0 => "Core API integration and load testing are complete, keeping Friday's rollout on track. Final legal sign-off is the only open item.".into(),
            1 => "Integration passed load testing; we ship Friday pending final legal sign-off.".into(),
            _ => "Ready for Friday release — engineering is green, awaiting legal approval.".into(),
        }
    } else if lower.contains("warm") || lower.contains("tone") {
        match var {
            0 => "Thanks so much for bearing with us while we get the details right. We're moving launch to Wednesday at 10 AM so everything feels rock-solid from minute one.".into(),
            1 => "Really appreciate everyone's patience — taking one extra day to polish edge cases means we'll ship a much smoother release Wednesday morning at 10 AM.".into(),
            _ => "Grateful for your flexibility! We'll wrap final QA tomorrow and go live Wednesday at 10 AM sharp.".into(),
        }
    } else if lower.contains("translate") {
        match var {
            0 => "Japanese: 「リアルタイム同期を公開しました。デスクトップとWebの間で変更が即座に反映されます。」\nSpanish: «Sincronización en vivo disponible: cada cambio se actualiza al instante en escritorio y web.»".into(),
            1 => "Japanese: 「すべての端末で瞬時に同期する新しいワークスペース体験をお届けします。」\nSpanish: «Presentamos la sincronización instantánea en todos tus dispositivos.»".into(),
            _ => "Japanese: 「編集もレビューもリアルタイムに共有できるようになりました。」\nSpanish: «Ediciones y comentarios ahora se sincronizan en tiempo real.»".into(),
        }
    } else {
        match var {
            0 => "Thursday at 3 PM works great — I've added the updated Figma frames to the calendar invite so we can walk through the key flows together.".into(),
            1 => "Count me in for Thursday at 3 PM. I'll send a two-minute summary beforehand so we can jump straight into open decisions.".into(),
            _ => "Confirmed for Thursday at 3 PM — looking forward to reviewing the final polish together.".into(),
        }
    }
}

/// Split word tokens preserving trailing spaces for natural streaming cadence.
pub fn split_stream_words(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut seen_non_ws = false;
    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            seen_non_ws = false;
        } else {
            if !seen_non_ws && idx > start {
                let slice = &text[start..idx];
                if !slice.is_empty() {
                    out.push(slice);
                }
                start = idx;
            }
            seen_non_ws = true;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

struct StreamState {
    msg_idx: usize,
    started_at: Instant,
}

struct DictationState {
    started_at: Instant,
    applied_words: usize,
}

/// Floating AI writing assistant card with atmospheric glow, streaming replies, and dictation.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::writing_assistant::{WritingAssistant, WritingAssistantEvent};
///
/// # fn doc(cx: &mut gpui::App) {
/// let assistant = cx.new(|cx| {
///     WritingAssistant::new("quill-helper", cx)
///         .with_name("Quill")
///         .with_tagline("Your writing companion.")
/// });
/// cx.subscribe(&assistant, |_, ev: &WritingAssistantEvent, _| {
///     let _ = ev;
/// })
/// .detach();
/// # }
/// ```
pub struct WritingAssistant {
    id: ElementId,
    name: Option<SharedString>,
    tagline: Option<SharedString>,
    greeting: Option<SharedString>,
    placeholder: Option<SharedString>,
    custom_suggestions: Option<Vec<WritingSuggestion>>,
    dictation_sample: Option<SharedString>,
    speech_mode: SpeechMode,
    responder: Option<WritingResponder>,
    messages: Vec<WritingMessage>,
    stream: Option<StreamState>,
    dictation: Option<DictationState>,
    last_prompt: Option<(SharedString, usize)>,
    copied_at: Option<(usize, Instant)>,
    prompt_editor: Option<Entity<Editor>>,
    editor_lang_zh: Option<bool>,
    pending_clear_in_render: bool,
    glow_intensity: Slide,
    glow_surface: Rc<RefCell<GlowSurface>>,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl EventEmitter<WritingAssistantEvent> for WritingAssistant {}

impl WritingAssistant {
    /// Create a [`WritingAssistant`] card with default `"Quill"` persona and rewrite chips.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            name: None,
            tagline: None,
            greeting: None,
            placeholder: None,
            custom_suggestions: None,
            dictation_sample: None,
            speech_mode: SpeechMode::Simulate,
            responder: None,
            messages: Vec::new(),
            stream: None,
            dictation: None,
            last_prompt: None,
            copied_at: None,
            prompt_editor: None,
            editor_lang_zh: None,
            pending_clear_in_render: false,
            glow_intensity: Slide::new(1.0),
            glow_surface: Rc::new(RefCell::new(GlowSurface::new())),
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Override the assistant persona name shown in the card header.
    pub fn with_name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Override the subtitle line beneath the assistant name.
    pub fn with_tagline(mut self, tagline: impl Into<SharedString>) -> Self {
        self.tagline = Some(tagline.into());
        self
    }

    /// Override the welcome headline displayed before the first message.
    pub fn with_greeting(mut self, greeting: impl Into<SharedString>) -> Self {
        self.greeting = Some(greeting.into());
        self
    }

    /// Override the prompt composer placeholder text.
    pub fn with_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Supply custom quick-action suggestion chips for the welcome screen.
    pub fn with_suggestions(mut self, suggestions: Vec<WritingSuggestion>) -> Self {
        self.custom_suggestions = Some(suggestions);
        self
    }

    /// Override the simulated dictation sentence spoken into the field when Mic is active.
    pub fn with_dictation(mut self, sentence: impl Into<SharedString>) -> Self {
        self.dictation_sample = Some(sentence.into());
        self
    }

    /// Configure whether the voice dictation toggle is simulated or hidden.
    pub fn with_speech_mode(mut self, mode: SpeechMode) -> Self {
        self.speech_mode = mode;
        self
    }

    /// Provide a custom synchronous responder `(prompt, attempt, is_zh) -> draft`.
    pub fn with_responder(
        mut self,
        responder: impl Fn(&str, usize, bool) -> SharedString + 'static,
    ) -> Self {
        self.responder = Some(Rc::new(responder));
        self
    }

    /// Seed an existing conversation into the card.
    pub fn with_messages(mut self, messages: Vec<WritingMessage>) -> Self {
        let has_chat = !messages.is_empty();
        self.messages = messages;
        self.glow_intensity = Slide::new(if has_chat { 0.42 } else { 1.0 });
        self
    }

    /// Enable or disable the loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the empty state placeholder.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle the loading skeleton state dynamically.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle the empty state dynamically.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Return whether the loading skeleton state is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Return whether the empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Return the current conversation messages.
    pub fn messages(&self) -> &[WritingMessage] {
        &self.messages
    }

    /// Return whether live voice dictation is currently listening.
    pub fn is_listening(&self) -> bool {
        self.dictation.is_some()
    }

    /// Return whether an assistant reply is actively streaming.
    pub fn is_streaming(&self) -> bool {
        self.stream.is_some()
    }

    /// Reset the card back to the welcome screen.
    pub fn reset_chat(&mut self, w: Option<&mut Window>, cx: &mut Context<Self>) {
        self.messages.clear();
        self.stream = None;
        self.dictation = None;
        self.last_prompt = None;
        self.copied_at = None;
        self.glow_intensity.duration(1.0, 0.55, cx);
        if let (Some(win), Some(ed)) = (w, self.prompt_editor.clone()) {
            ed.update(cx, |e, cx| e.set_value("", win, cx));
        }
        cx.emit(WritingAssistantEvent::NewChat);
        cx.notify();
    }

    /// Submit a prompt string and begin streaming the assistant's reply.
    pub fn submit_prompt(
        &mut self,
        prompt: impl Into<SharedString>,
        w: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        let text: SharedString = prompt.into();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let clean: SharedString = trimmed.to_string().into();
        self.dictation = None;
        if let (Some(win), Some(ed)) = (w, self.prompt_editor.clone()) {
            ed.update(cx, |e, cx| e.set_value("", win, cx));
        }
        self.messages.push(WritingMessage::user(clean.clone()));
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        let reply = if let Some(resp) = &self.responder {
            resp(clean.as_ref(), 0, zh)
        } else {
            sample_writing_response(clean.as_ref(), 0, zh)
        };
        let msg_idx = self.messages.len();
        self.messages.push(WritingMessage::assistant(reply));
        self.last_prompt = Some((clean.clone(), 0));
        self.glow_intensity.duration(0.42, 0.55, cx);
        if disabled_motion(cx) {
            self.stream = None;
        } else {
            self.stream = Some(StreamState {
                msg_idx,
                started_at: Instant::now(),
            });
        }
        cx.emit(WritingAssistantEvent::Submitted(clean));
        cx.notify();
    }

    fn submit_from_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.prompt_editor.clone() else {
            return;
        };
        let val = ed.read(cx).value().trim().to_string();
        if val.is_empty() {
            return;
        }
        self.submit_prompt(val, Some(w), cx);
    }

    fn retry_last(&mut self, cx: &mut Context<Self>) {
        let Some((prompt, prev_attempt)) = self.last_prompt.clone() else {
            return;
        };
        let attempt = prev_attempt + 1;
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        let reply = if let Some(resp) = &self.responder {
            resp(prompt.as_ref(), attempt, zh)
        } else {
            sample_writing_response(prompt.as_ref(), attempt, zh)
        };
        let target_idx = if let Some(last) = self.messages.last_mut()
            && last.role == WritingRole::Assistant
        {
            last.text = reply;
            last.accepted = false;
            self.messages.len() - 1
        } else {
            let idx = self.messages.len();
            self.messages.push(WritingMessage::assistant(reply));
            idx
        };
        self.last_prompt = Some((prompt.clone(), attempt));
        if disabled_motion(cx) {
            self.stream = None;
        } else {
            self.stream = Some(StreamState {
                msg_idx: target_idx,
                started_at: Instant::now(),
            });
        }
        cx.emit(WritingAssistantEvent::Retried { prompt, attempt });
        cx.notify();
    }

    fn accept_message(&mut self, idx: usize, cx: &mut Context<Self>) {
        if let Some(msg) = self.messages.get_mut(idx)
            && msg.role == WritingRole::Assistant
        {
            msg.accepted = true;
            let text = msg.text.clone();
            if self.stream.as_ref().is_some_and(|s| s.msg_idx == idx) {
                self.stream = None;
            }
            cx.emit(WritingAssistantEvent::Accepted(text));
            cx.notify();
        }
    }

    fn copy_message(&mut self, idx: usize, cx: &mut Context<Self>) {
        if let Some(msg) = self.messages.get(idx) {
            let text = msg.text.clone();
            cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
            self.copied_at = Some((idx, Instant::now()));
            cx.emit(WritingAssistantEvent::Copied(text));
            cx.notify();
        }
    }

    fn toggle_dictation(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.speech_mode == SpeechMode::Hidden {
            return;
        }
        if self.dictation.take().is_some() {
            cx.emit(WritingAssistantEvent::DictationToggled(false));
            let pending = self
                .prompt_editor
                .as_ref()
                .map(|e| e.read(cx).value().trim().to_string())
                .unwrap_or_default();
            if !pending.is_empty() {
                self.submit_prompt(pending, Some(w), cx);
            } else {
                cx.notify();
            }
            return;
        }
        if let Some(ed) = self.prompt_editor.clone() {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        if disabled_motion(cx) {
            let sample = self.resolved_dictation(cx);
            self.submit_prompt(sample, Some(w), cx);
            return;
        }
        self.dictation = Some(DictationState {
            started_at: Instant::now(),
            applied_words: 0,
        });
        cx.emit(WritingAssistantEvent::DictationToggled(true));
        cx.notify();
    }

    fn resolved_dictation(&self, cx: &App) -> SharedString {
        if let Some(s) = &self.dictation_sample {
            return s.clone();
        }
        tr(
            cx,
            "Make this customer note warmer and confirm we ship on Wednesday morning.",
            "把这段客户说明改得更温和，并确认我们在周三上午正式发布。",
        )
        .into()
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        let ph: SharedString = self
            .placeholder
            .clone()
            .unwrap_or_else(|| tr(cx, "Ask anything…", "输入写作需求…").into());
        if let Some(ed) = &self.prompt_editor {
            if self.editor_lang_zh != Some(zh) {
                self.editor_lang_zh = Some(zh);
                ed.update(cx, |e, cx| {
                    e.set_placeholder(ph);
                    cx.notify();
                });
            }
            if self.pending_clear_in_render {
                self.pending_clear_in_render = false;
                let ed_clone = ed.clone();
                ed_clone.update(cx, |e, cx| e.set_value("", w, cx));
            }
            return;
        }
        self.editor_lang_zh = Some(zh);
        let ed = cx.new(|cx| {
            let mut e = Editor::new(tr(cx, "Message", "写作消息"), InputMode::InlineMulti, w, cx);
            e.set_placeholder(ph);
            e.font_size = 13.;
            e
        });
        cx.subscribe(&ed, |this, ed_ent, ev: &EditEvent, cx| match ev {
            EditEvent::Changed => {
                cx.notify();
            }
            EditEvent::Submitted => {
                let raw = ed_ent.read(cx).value().trim().to_string();
                if !raw.is_empty() {
                    this.pending_clear_in_render = true;
                    this.submit_prompt(raw, None, cx);
                }
            }
            EditEvent::Escape if this.dictation.take().is_some() => {
                cx.emit(WritingAssistantEvent::DictationToggled(false));
                cx.notify();
            }
            _ => {}
        })
        .detach();
        self.prompt_editor = Some(ed);
    }

    fn advance_live_timers(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(usize, usize, bool)> {
        let reduced = disabled_motion(cx);

        if let Some(dic_started) = self.dictation.as_ref().map(|d| d.started_at) {
            let sample = self.resolved_dictation(cx);
            let tokens = split_stream_words(sample.as_ref());
            let total = tokens.len().max(1);
            let elapsed = dic_started.elapsed().as_secs_f32();
            let count = ((elapsed / DICTATION_WORD_SECS).floor() as usize + 1).min(total);
            let already = self.dictation.as_ref().map_or(0, |d| d.applied_words);
            if count > already {
                let partial: String = tokens[..count].concat();
                if let Some(ed) = self.prompt_editor.clone() {
                    ed.update(cx, |e, cx| e.set_value(partial, w, cx));
                }
                if let Some(d) = &mut self.dictation {
                    d.applied_words = count;
                }
            }
            if elapsed >= (total as f32) * DICTATION_WORD_SECS + 0.42 {
                self.dictation = None;
                cx.emit(WritingAssistantEvent::DictationToggled(false));
                self.submit_prompt(sample, Some(w), cx);
            } else if !reduced {
                w.request_animation_frame();
            }
        }

        if let Some(st) = &self.stream {
            let idx = st.msg_idx;
            if let Some(msg) = self.messages.get(idx) {
                let elapsed = st.started_at.elapsed().as_secs_f32();
                if reduced {
                    self.stream = None;
                    return None;
                }
                w.request_animation_frame();
                if elapsed < THINKING_SECS {
                    return Some((idx, 0, true));
                }
                let words = split_stream_words(msg.text.as_ref());
                let total = words.len();
                let shown =
                    (((elapsed - THINKING_SECS) / WORD_STEP_SECS).floor() as usize + 1).min(total);
                if shown >= total {
                    self.stream = None;
                    return None;
                }
                return Some((idx, shown, false));
            } else {
                self.stream = None;
            }
        }
        None
    }

    fn render_loading_card(&self, cx: &App) -> AnyElement {
        let p = ProPalette::get(cx);
        let phase = (self.started_at.elapsed().as_secs_f32() / 1.4).fract();
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        div()
            .w_full()
            .max_w(px(380.))
            .rounded(px(10.))
            .bg(card_bg)
            .border_1()
            .border_color(p.line)
            .shadow_sm()
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .pl(px(16.))
                    .pr(px(12.))
                    .py(px(12.))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(pro_skeleton_bar(92., 14., 5., phase, cx))
                    .child(pro_skeleton_bar(148., 11., 4., phase + 0.12, cx)),
            )
            .child(
                div()
                    .h(px(340.))
                    .p(px(16.))
                    .flex()
                    .flex_col()
                    .justify_end()
                    .gap(px(12.))
                    .child(pro_skeleton_bar(188., 16., 5., phase + 0.2, cx))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(106., 28., 6., phase + 0.25, cx))
                            .child(pro_skeleton_bar(98., 28., 6., phase + 0.32, cx))
                            .child(pro_skeleton_bar(88., 28., 6., phase + 0.38, cx)),
                    )
                    .child(pro_skeleton_bar(340., 28., 6., phase + 0.45, cx)),
            )
            .into_any_element()
    }
}

impl Render for WritingAssistant {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editor(w, cx);
        let stream_info = self.advance_live_timers(w, cx);

        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);

        if self.loading {
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(self.render_loading_card(cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(380.))
                .rounded(px(10.))
                .bg(if p.dark { rgb(0x1c1c1e) } else { rgb(0xffffff) })
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "Assistant unavailable", "写作助手暂不可用"),
                    tr(
                        cx,
                        "Connect a writing model to draft, polish, and translate inline.",
                        "连接写作模型后即可进行行内润色、精简与多语翻译。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let user_bubble_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };

        let name_str: SharedString = self
            .name
            .clone()
            .unwrap_or_else(|| tr(cx, "Quill", "Quill").into());
        let tagline_str: SharedString = self
            .tagline
            .clone()
            .unwrap_or_else(|| tr(cx, "Your writing companion.", "你的随身写作搭档。").into());
        let greeting_str: SharedString = self
            .greeting
            .clone()
            .unwrap_or_else(|| tr(cx, "What should we write?", "今天想写点什么？").into());

        let has_chat = !self.messages.is_empty();
        let glow_level = self.glow_intensity.get(w, cx).clamp(0., 1.);
        let started_at = self.started_at;
        let glow_surface = self.glow_surface.clone();
        let dark = p.dark;

        // Header with multicolor sparkle + New chat button
        let spark_mark = if dark {
            pro_raw_svg(SPARK_DARK_SVG, 16., 16.)
        } else {
            pro_raw_svg(SPARK_LIGHT_SVG, 16., 16.)
        };

        let new_chat_lbl: SharedString = tr(cx, "New chat", "新建对话").into();
        let this_new = cx.entity();
        let new_btn = pro_presence(pro_eid(&self.id, "new-chat-pres"), has_chat, 0.24, 0.18, {
            let id = pro_eid(&self.id, "new-chat-btn");
            move |amt| {
                div()
                    .id(id)
                    .role(Role::Button)
                    .aria_label(new_chat_lbl.clone())
                    .tab_index(0)
                    .size(px(28.))
                    .rounded(px(6.))
                    .opacity(amt)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(move |_, w, cx| {
                        this_new.update(cx, |v, cx| v.reset_chat(Some(w), cx));
                    })
                    .child(pro_icon(ProIcon::Plus, 15., p.muted))
                    .into_any_element()
            }
        });

        let header = div()
            .pl(px(16.))
            .pr(px(12.))
            .py(px(12.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(spark_mark)
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(name_str),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.text)
                            .truncate()
                            .child(tagline_str),
                    ),
            )
            .child(div().size(px(28.)).flex_none().child(new_btn));

        // Drifting 3-lobe Gaussian glow canvas
        let glow_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                if bounds.size.width <= px(2.) || bounds.size.height <= px(2.) {
                    return;
                }
                let elapsed = started_at.elapsed().as_secs_f32();
                let image = glow_surface
                    .borrow_mut()
                    .frame(dark, glow_level, elapsed, reduced, window);
                let _ = window.paint_image(bounds, bounds, Corners::default(), image, 0, false);
                if !reduced {
                    window.request_animation_frame();
                }
            },
        )
        .absolute()
        .inset_0();

        // Content: Welcome or Conversation
        let last_assistant_idx = self
            .messages
            .iter()
            .rposition(|m| m.role == WritingRole::Assistant);

        let copied_recent = self
            .copied_at
            .filter(|(_, at)| at.elapsed().as_secs_f32() < 1.8)
            .map(|(idx, _)| idx);
        if self
            .copied_at
            .is_some_and(|(_, at)| at.elapsed().as_secs_f32() >= 1.8)
            && !reduced
        {
            w.request_animation_frame();
        }

        let main_stage = if !has_chat {
            let chips = self
                .custom_suggestions
                .clone()
                .unwrap_or_else(|| default_suggestions(zh));
            let chip_bg = if p.dark {
                rgb(0x1c1c1e).opacity(0.74)
            } else {
                rgb(0xffffff).opacity(0.76)
            };
            let chip_hover_bg = if p.dark {
                rgb(0x28282c).opacity(0.88)
            } else {
                rgb(0xf6f6f7).opacity(0.92)
            };

            let mut chip_row = div().flex().flex_wrap().gap(px(6.));
            for (idx, chip) in chips.into_iter().enumerate() {
                let chip_id = pro_eid(&self.id, format!("chip-{idx}-{}", chip.id));
                let prompt = chip.prompt;
                let label = chip.label;
                chip_row = chip_row.child(
                    div()
                        .id(chip_id)
                        .role(Role::Button)
                        .aria_label(label.clone())
                        .tab_index(0)
                        .px(px(10.))
                        .py(px(4.))
                        .rounded(px(6.))
                        .bg(chip_bg)
                        .border_1()
                        .border_color(p.line)
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .cursor_pointer()
                        .hover(move |s| s.bg(chip_hover_bg).text_color(p.ink))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .on_key_down({
                            let prompt = prompt.clone();
                            cx.listener(move |v, ev: &KeyDownEvent, w, cx| {
                                if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                                    w.prevent_default();
                                    cx.stop_propagation();
                                    v.submit_prompt(prompt.clone(), Some(w), cx);
                                }
                            })
                        })
                        .on_click(cx.listener(move |v, _, w, cx| {
                            v.submit_prompt(prompt.clone(), Some(w), cx);
                        }))
                        .child(label),
                );
            }

            pro_view_transition(
                pro_eid(&self.id, "stage-vtx"),
                "welcome",
                div()
                    .p(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(24.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(greeting_str),
                    )
                    .child(chip_row),
            )
        } else {
            let mut list = div()
                .id(pro_eid(&self.id, "messages-scroll"))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(16.))
                .pt(px(16.))
                .pb(px(12.))
                .flex()
                .flex_col()
                .gap(px(12.));

            for (idx, msg) in self.messages.iter().enumerate() {
                match msg.role {
                    WritingRole::User => {
                        list = list.child(
                            div().w_full().flex().justify_end().child(
                                div()
                                    .max_w(relative(0.86))
                                    .px(px(10.))
                                    .py(px(6.))
                                    .rounded(px(10.))
                                    .bg(user_bubble_bg)
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(p.ink)
                                    .child(msg.text.clone()),
                            ),
                        );
                    }
                    WritingRole::Assistant => {
                        let stream_state = stream_info.filter(|(s_idx, _, _)| *s_idx == idx);
                        let is_thinking = stream_state.is_some_and(|(_, _, th)| th);
                        let is_streaming_words = stream_state.is_some_and(|(_, _, th)| !th);
                        let visible_text: SharedString =
                            if let Some((_, count, false)) = stream_state {
                                let tokens = split_stream_words(msg.text.as_ref());
                                tokens[..count.min(tokens.len())].concat().into()
                            } else {
                                msg.text.clone()
                            };

                        let body_el = if is_thinking {
                            pro_shimmer_text(
                                pro_eid(&self.id, format!("think-{idx}")),
                                tr(cx, "Drafting…", "正在起草…"),
                                p.muted,
                                if p.dark {
                                    rgb(0xffffff).into()
                                } else {
                                    p.strong
                                },
                                13.,
                                FontWeight::NORMAL,
                                true,
                            )
                        } else {
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.ink)
                                .child(visible_text)
                                .into_any_element()
                        };

                        let show_actions =
                            last_assistant_idx == Some(idx) && !is_thinking && !is_streaming_words;
                        let accepted = msg.accepted;
                        let is_copied = copied_recent == Some(idx);

                        let mut msg_col = div()
                            .max_w(relative(0.94))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(body_el);

                        if show_actions {
                            let accept_lbl: SharedString = if accepted {
                                tr(cx, "Inserted", "已插入").into()
                            } else {
                                tr(cx, "Accept", "采纳插入").into()
                            };
                            let retry_lbl: SharedString = tr(cx, "Retry", "重写").into();
                            let copy_lbl: SharedString = if is_copied {
                                tr(cx, "Copied", "已复制").into()
                            } else {
                                tr(cx, "Copy", "复制").into()
                            };

                            msg_col = msg_col.child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .id(pro_eid(&self.id, format!("accept-{idx}")))
                                            .role(Role::Button)
                                            .aria_label(accept_lbl.clone())
                                            .tab_index(0)
                                            .h(px(24.))
                                            .px(px(8.))
                                            .rounded(px(6.))
                                            .bg(if accepted { p.green_soft } else { p.primary })
                                            .text_color(if accepted {
                                                p.green_ink
                                            } else {
                                                p.on_primary
                                            })
                                            .text_size(px(11.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .cursor_pointer()
                                            .focus_visible(move |s| {
                                                s.border_1().border_color(p.focus)
                                            })
                                            .on_click(cx.listener(move |v, _, _, cx| {
                                                v.accept_message(idx, cx);
                                            }))
                                            .child(pro_icon(
                                                ProIcon::Check,
                                                12.,
                                                if accepted { p.green_ink } else { p.on_primary },
                                            ))
                                            .child(accept_lbl),
                                    )
                                    .child(
                                        div()
                                            .id(pro_eid(&self.id, format!("retry-{idx}")))
                                            .role(Role::Button)
                                            .aria_label(retry_lbl.clone())
                                            .tab_index(0)
                                            .h(px(24.))
                                            .px(px(8.))
                                            .rounded(px(6.))
                                            .bg(p.chip)
                                            .border_1()
                                            .border_color(p.line)
                                            .text_color(p.text)
                                            .text_size(px(11.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .cursor_pointer()
                                            .hover(move |s| s.bg(p.chip_hover).text_color(p.ink))
                                            .focus_visible(move |s| {
                                                s.border_1().border_color(p.focus)
                                            })
                                            .on_click(cx.listener(move |v, _, _, cx| {
                                                v.retry_last(cx);
                                            }))
                                            .child(pro_icon(ProIcon::Refresh, 12., p.muted))
                                            .child(retry_lbl),
                                    )
                                    .child(
                                        div()
                                            .id(pro_eid(&self.id, format!("copy-{idx}")))
                                            .role(Role::Button)
                                            .aria_label(copy_lbl.clone())
                                            .tab_index(0)
                                            .h(px(24.))
                                            .px(px(8.))
                                            .rounded(px(6.))
                                            .bg(p.chip)
                                            .border_1()
                                            .border_color(p.line)
                                            .text_color(if is_copied {
                                                p.green_ink
                                            } else {
                                                p.text
                                            })
                                            .text_size(px(11.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .cursor_pointer()
                                            .hover(move |s| s.bg(p.chip_hover).text_color(p.ink))
                                            .focus_visible(move |s| {
                                                s.border_1().border_color(p.focus)
                                            })
                                            .on_click(cx.listener(move |v, _, _, cx| {
                                                v.copy_message(idx, cx);
                                            }))
                                            .child(pro_icon(
                                                if is_copied {
                                                    ProIcon::Check
                                                } else {
                                                    ProIcon::Copy
                                                },
                                                12.,
                                                if is_copied { p.green_ink } else { p.muted },
                                            ))
                                            .child(copy_lbl),
                                    ),
                            );
                        }

                        list = list.child(msg_col);
                    }
                }
            }

            pro_view_transition(pro_eid(&self.id, "stage-vtx"), "chat", list)
        };

        // Composer row: Editor + Mic waveform button + Send pop button
        let has_input_text = self
            .prompt_editor
            .as_ref()
            .is_some_and(|e| !e.read(cx).value().trim().is_empty());
        let listening = self.dictation.is_some();

        let editor_view = self
            .prompt_editor
            .as_ref()
            .map(|ed| ed.clone().into_any_element())
            .unwrap_or_else(|| div().into_any_element());

        let mic_el = if self.speech_mode == SpeechMode::Simulate {
            let mic_aria: SharedString = if listening {
                tr(cx, "Stop dictation", "结束语音输入").into()
            } else {
                tr(cx, "Dictate", "语音输入").into()
            };
            let accent_fg: Hsla = if p.dark {
                rgb(0x4fb0fe).into()
            } else {
                rgba(0x0060e2c4).into()
            };
            let accent_wash: Hsla = if p.dark {
                hsla(218. / 360., 1.0, 0.65, 0.16)
            } else {
                hsla(214. / 360., 1.0, 0.44, 0.09)
            };
            let ring_color: Hsla = if p.dark {
                hsla(218. / 360., 1.0, 0.65, 0.34)
            } else {
                hsla(214. / 360., 1.0, 0.44, 0.24)
            };

            let elapsed_dic = self
                .dictation
                .as_ref()
                .map_or(0.0, |d| d.started_at.elapsed().as_secs_f32());

            let inner_icon = if listening {
                let mut wave = div()
                    .relative()
                    .h(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(2.));
                for bar_i in 0..3usize {
                    let scale = if reduced {
                        0.70
                    } else {
                        let phase = ((elapsed_dic - bar_i as f32 * 0.35) / 1.1) * TAU;
                        0.35 + 0.65 * (0.5 + 0.5 * phase.sin())
                    };
                    wave = wave.child(
                        div()
                            .w(px(2.))
                            .h(px((12. * scale).max(3.5)))
                            .rounded(px(1.))
                            .bg(accent_fg),
                    );
                }
                wave.into_any_element()
            } else {
                pro_svg("0 0 24 24", ICON_MIC, 16., p.muted)
            };

            let pulse_ring = if listening && !reduced {
                let u = (elapsed_dic / 1.6).fract();
                let spread = 6.5 * (1. - (1. - u) * (1. - u));
                let alpha = (1. - u).clamp(0., 1.);
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let expanded = Bounds::new(
                            point(bounds.origin.x - px(spread), bounds.origin.y - px(spread)),
                            size(
                                bounds.size.width + px(2. * spread),
                                bounds.size.height + px(2. * spread),
                            ),
                        );
                        window.paint_quad(quad(
                            expanded,
                            Corners::all(px(6. + spread * 0.6)),
                            transparent_black(),
                            Edges::all(px(1.5)),
                            ring_color.opacity(alpha),
                            BorderStyle::Solid,
                        ));
                    },
                )
                .absolute()
                .inset_0()
                .into_any_element()
            } else {
                div().into_any_element()
            };

            div()
                .id(pro_eid(&self.id, "mic-btn"))
                .role(Role::Button)
                .aria_label(mic_aria)
                .tab_index(0)
                .size(px(28.))
                .flex_none()
                .relative()
                .rounded(px(6.))
                .bg(if listening {
                    accent_wash
                } else {
                    transparent_black()
                })
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .when(!listening, move |d| d.hover(move |s| s.bg(p.hover)))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_key_down(cx.listener(|v, ev: &KeyDownEvent, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                        w.prevent_default();
                        cx.stop_propagation();
                        v.toggle_dictation(w, cx);
                    }
                }))
                .on_click(cx.listener(|v, _, w, cx| {
                    v.toggle_dictation(w, cx);
                }))
                .child(pulse_ring)
                .child(inner_icon)
                .into_any_element()
        } else {
            div().into_any_element()
        };

        let send_lbl: SharedString = tr(cx, "Send message", "发送").into();
        let this_send = cx.entity();
        let send_el = pro_presence(
            pro_eid(&self.id, "send-pres"),
            has_input_text && !listening,
            0.20,
            0.15,
            {
                let id = pro_eid(&self.id, "send-btn");
                move |amt| {
                    div()
                        .id(id)
                        .role(Role::Button)
                        .aria_label(send_lbl.clone())
                        .tab_index(0)
                        .size(px(28.))
                        .flex_none()
                        .rounded(px(6.))
                        .opacity(amt)
                        .bg(p.primary)
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .on_click(move |_, w, cx| {
                            this_send.update(cx, |v, cx| v.submit_from_editor(w, cx));
                        })
                        .child(pro_icon(ProIcon::ArrowUp, 15., p.on_primary))
                        .into_any_element()
                }
            },
        );

        let composer_row = div()
            .relative()
            .flex_none()
            .pl(px(16.))
            .pr(px(10.))
            .pt(px(6.))
            .pb(px(10.))
            .when(has_chat, |d| d.border_t_1().border_color(p.line_soft))
            .flex()
            .items_end()
            .gap(px(4.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(28.))
                    .py(px(2.))
                    .mr(px(4.))
                    .flex()
                    .items_center()
                    .child(editor_view),
            )
            .child(mic_el)
            .child(send_el);

        let body = div()
            .relative()
            .h(px(340.))
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(glow_canvas)
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .justify_end()
                    .overflow_hidden()
                    .child(main_stage),
            )
            .child(composer_row);

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(380.))
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow_sm()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(body),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{
        GLOW_H, GLOW_W, SpeechMode, WritingAssistant, WritingRole, rasterize_glow,
        sample_writing_response, split_stream_words,
    };
    use gpui::TestAppContext;

    #[test]
    fn stream_word_splitter_preserves_text_exactness() {
        let sentence = "Hi Maya — following up on Tuesday's proposal.\nLet's meet at 3 PM.";
        let chunks = split_stream_words(sentence);
        assert!(chunks.len() >= 10);
        assert_eq!(chunks.concat(), sentence);
    }

    #[test]
    fn sample_responder_provides_distinct_retry_variations() {
        let a = sample_writing_response("Polish my email", 0, false);
        let b = sample_writing_response("Polish my email", 1, false);
        let c = sample_writing_response("Polish my email", 2, false);
        assert_ne!(a, b);
        assert_ne!(b, c);
    }

    #[test]
    fn glow_rasterizer_tints_lower_half_while_preserving_clean_upper_surface() {
        let mut raw = vec![0u8; GLOW_W * GLOW_H * 4];
        rasterize_glow(false, 1.0, 0.5, &mut raw);
        let top_px = (2 * GLOW_W + GLOW_W / 2) * 4;
        assert!(raw[top_px] >= 250 && raw[top_px + 1] >= 250 && raw[top_px + 2] >= 250);
        let bot_px = ((GLOW_H - 6) * GLOW_W + GLOW_W / 2) * 4;
        assert!(
            raw[bot_px] < 252 || raw[bot_px + 1] < 252 || raw[bot_px + 2] < 252,
            "expected warm color glow in lower region"
        );
    }

    #[gpui::test]
    fn submit_retry_accept_and_reset_flow(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx
            .new(|cx| WritingAssistant::new("wa-test", cx).with_speech_mode(SpeechMode::Simulate));
        view.update(cx, |wa, cx| {
            assert!(wa.messages().is_empty());
            wa.submit_prompt("Make it shorter", None, cx);
            assert_eq!(wa.messages().len(), 2);
            assert_eq!(wa.messages()[0].role, WritingRole::User);
            assert_eq!(wa.messages()[1].role, WritingRole::Assistant);
            let first_draft = wa.messages()[1].text.clone();
            wa.retry_last(cx);
            assert_ne!(wa.messages()[1].text, first_draft);
            wa.accept_message(1, cx);
            assert!(wa.messages()[1].accepted);
            wa.reset_chat(None, cx);
            assert!(wa.messages().is_empty());
        });
    }
}
