//! Rich AI prompt composer with model picker, voice waveform, app toggles, and tool approval.
//!
//! Combines a multiline [`Editor`], file attachment chips, an inline tool-permission
//! banner, a provider-branded model popover (`Claude`, `GPT`, `Gemini`, `DeepSeek`),
//! live voice dictation waveform mode, folder scoping, and connected app switches.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_check_draw, pro_ease, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_mix_color,
        pro_pill_indicator, pro_popover_presence, pro_selection_sensor, pro_shimmer_text,
        pro_skeleton_bar, pro_swap_label, pro_switch, pro_tactile, pro_toast_banner,
        pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SVG_HAND_PALM: &str = r#"<path d="M52.5305 32.6557V24.8463C52.5305 22.1016 54.7555 19.8766 57.5001 19.8766C60.2448 19.8766 62.4698 22.1016 62.4698 24.8463V42.027C62.4698 53.3194 53.3154 62.4735 42.0233 62.4735H34.2388M22.7127 39.7552L15.94 31.8537C14.8314 30.5602 13.2129 29.8159 11.5094 29.8159H11.1235C8.11379 29.8159 5.67395 32.2558 5.67395 35.2655C5.67395 36.3412 5.99241 37.393 6.58919 38.2881L16.4118 53.0221C20.3482 58.9269 26.9753 62.4735 34.0719 62.4735" stroke="currentColor" stroke-width="4.8" stroke-linecap="round" stroke-linejoin="round"></path><path d="M52.534 24.1378V16.3284C52.534 13.5837 50.3091 11.3588 47.5644 11.3588C44.8197 11.3588 42.5948 13.5837 42.5948 16.3284V29.8174" stroke="currentColor" stroke-width="4.8" stroke-linecap="round" stroke-linejoin="round"></path><path d="M42.5958 29.8198V10.6513C42.5958 7.90661 40.3708 5.68163 37.6262 5.68163C34.8815 5.68163 32.6565 7.90661 32.6565 10.6513V28.4" stroke="currentColor" stroke-width="4.8" stroke-linecap="round" stroke-linejoin="round"></path><path d="M32.6576 28.3975V16.3284C32.6576 13.5837 30.4326 11.3588 27.6879 11.3588C24.9433 11.3588 22.7183 13.5837 22.7183 16.3284V39.7567" stroke="currentColor" stroke-width="4.8" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SVG_BLUE_FOLDER: &str = r#"<path d="M1.4 3.1c0-.72.58-1.3 1.3-1.3h1.72c.38 0 .73.19.94.5l.6.9h3.34c.72 0 1.3.58 1.3 1.3V6H1.4Z" fill="currentColor"></path><path d="M1 6.95C.86 6.3 1.36 5.7 2.02 5.7h7.96c.66 0 1.16.6 1.02 1.25l-.5 2.4a1.3 1.3 0 0 1-1.27 1.05H2.77A1.3 1.3 0 0 1 1.5 9.35Z" fill="currentColor" opacity="0.48"></path>"#;

struct SendBtnAnim {
    hover: Slide,
    press: Slide,
    active_val: bool,
    active_slide: Slide,
}

type ComposerClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
struct ComposerSendVisual {
    state_id: ElementId,
    aria: SharedString,
    enabled: bool,
    on_click: ComposerClickHandler,
}

impl RenderOnce for ComposerSendVisual {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let init_en = self.enabled;
        let st = w.use_keyed_state(self.state_id, cx, |_, _| SendBtnAnim {
            hover: Slide::snappy(0.),
            press: Slide::snappy(0.),
            active_val: init_en,
            active_slide: Slide::snappy(f32::from(init_en)),
        });
        let (hov, down, act) = st.update(cx, |s, cx| {
            if s.active_val != self.enabled {
                s.active_val = self.enabled;
                s.active_slide.duration(f32::from(self.enabled), 0.18, cx);
            }
            (
                s.hover.get(w, cx).clamp(0., 1.),
                s.press.get(w, cx).clamp(0., 1.),
                s.active_slide.get(w, cx).clamp(0., 1.),
            )
        });
        let base_bg = pro_mix_color(p.chip, p.primary, act);
        let hover_target = pro_mix_color(p.primary, p.on_primary, 0.14);
        let face_bg = pro_mix_color(base_bg, hover_target, hov * act);
        let fg = pro_mix_color(p.faint, p.on_primary, act);
        let icon_sz = 15. * (1. - 0.10 * down);
        let enabled = self.enabled;
        let on_click = self.on_click;

        let paint_face = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 1. || bh <= 1. {
                    return;
                }
                let inset = 1.6 * down;
                let rect = Bounds::new(
                    point(bounds.origin.x + px(inset), bounds.origin.y + px(inset)),
                    size(px((bw - 2. * inset).max(1.)), px((bh - 2. * inset).max(1.))),
                );
                let r = px(8.).min(rect.size.width / 2.).min(rect.size.height / 2.);
                window.paint_quad(fill(rect, face_bg).corner_radii(r));
            },
        )
        .absolute()
        .inset_0();

        div()
            .id("composer-send-hit")
            .role(Role::Button)
            .aria_label(self.aria)
            .tab_index(0)
            .size(px(28.))
            .relative()
            .rounded(px(8.))
            .flex()
            .items_center()
            .justify_center()
            .when(enabled, |d| d.cursor_pointer())
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.hover.set(if *inside && enabled { 1. } else { 0. }, cx);
                        if !*inside {
                            s.press.set(0., cx);
                        }
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_down(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    if enabled {
                        st.update(cx, |s, cx| {
                            s.press.set(1., cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_click(move |ev, w, cx| (on_click)(ev, w, cx))
            .child(paint_face)
            .child(
                div()
                    .relative()
                    .top(px(-0.8 * hov * act + 0.6 * down))
                    .child(pro_icon(ProIcon::ArrowUp, icon_sz, fg)),
            )
    }
}

#[derive(IntoElement)]
struct ChipEntrance {
    id: ElementId,
    animate_in: bool,
    exiting_progress: Option<f32>,
    content: AnyElement,
}

impl RenderOnce for ChipEntrance {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init_anim = self.animate_in;
        let st = w.use_keyed_state(pro_eid(&self.id, "chip-in"), cx, |_, inner_cx| {
            let mut clk = PhaseClock::default();
            if init_anim {
                clk.restart(inner_cx);
            }
            clk
        });
        let enter_t = st.update(cx, |clk, cx| {
            let age = clk.time(0.18, w, cx);
            pro_ease((age / 0.18).clamp(0., 1.))
        });
        let t = match self.exiting_progress {
            Some(ex_u) => (1. - pro_ease(ex_u.clamp(0., 1.))) * enter_t,
            None => enter_t,
        };
        div()
            .relative()
            .top(px(3.0 * (1. - t)))
            .opacity(t)
            .child(self.content)
    }
}

/// Execution permission policy selected in the [`ChatComposer`] toolbar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ComposerApprovalMode {
    /// Prompt before running side-effecting tool calls.
    #[default]
    RequestApproval,
    /// Auto-approve workspace file edits, ask on external APIs.
    ApproveEdits,
    /// Unrestricted autonomous tool execution.
    Unrestricted,
}

impl ComposerApprovalMode {
    /// Human-readable English/Chinese label for the active policy.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::RequestApproval => tr(cx, "Request approval", "每次请求审批"),
            Self::ApproveEdits => tr(cx, "Approve edits", "自动批准编辑"),
            Self::Unrestricted => tr(cx, "Unrestricted access", "无限制自主执行"),
        }
    }
}

///Selectable LLM option inside [`ChatComposer`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatModelOption {
    /// Stable model identifier (for example `"claude-sonnet"`).
    pub id: SharedString,
    /// Display label shown in the trigger and popover.
    pub name: SharedString,
    /// Brand lookup key passed to `pro_app_artwork` (`"claude"`, `"openai"`, `"gemini"`, `"deepseek"`).
    pub provider: SharedString,
    /// Short one-line capability hint.
    pub description: SharedString,
}

impl ChatModelOption {
    /// Construct a [`ChatModelOption`].
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        provider: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            provider: provider.into(),
            description: description.into(),
        }
    }
}

/// Connected workspace integration entry in [`ChatComposer`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComposerApp {
    /// Stable integration id (for example `"github"`).
    pub id: SharedString,
    /// Display name (`"GitHub"`, `"Linear"`, `"Slack"`, `"Notion"`, `"Google Drive"`).
    pub name: SharedString,
    /// Whether this app is currently enabled for the prompt.
    pub connected: bool,
}

impl ComposerApp {
    /// Construct a [`ComposerApp`] integration item.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        connected: bool,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            connected,
        }
    }
}

/// File chip attached to the active draft in [`ChatComposer`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComposerAttachment {
    /// Stable attachment id.
    pub id: SharedString,
    /// File name (such as `"architecture-rfc.md"`).
    pub name: SharedString,
    /// Compact size or line-count badge (such as `"18 KB"`).
    pub size_label: SharedString,
}

impl ComposerAttachment {
    /// Construct a [`ComposerAttachment`] chip.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        size_label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            size_label: size_label.into(),
        }
    }
}

/// Inline tool-permission request banner shown above the [`ChatComposer`] write card.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PendingToolApproval {
    /// Unique tool call identifier.
    pub id: SharedString,
    /// Connected app or tool provider name (`"Linear"`, `"GitHub"`, etc.).
    pub app: SharedString,
    /// One-line action summary.
    pub summary: SharedString,
    /// Expandable payload preview (`JSON` or CLI command).
    pub payload: SharedString,
}

impl PendingToolApproval {
    /// Construct a [`PendingToolApproval`] request banner.
    pub fn new(
        id: impl Into<SharedString>,
        app: impl Into<SharedString>,
        summary: impl Into<SharedString>,
        payload: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            app: app.into(),
            summary: summary.into(),
            payload: payload.into(),
        }
    }
}

/// Typed events emitted by [`ChatComposer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatComposerEvent {
    /// User submitted a prompt via Enter, Cmd+Enter, or the Send button.
    Send {
        message: SharedString,
        model_id: SharedString,
        attachments: Vec<ComposerAttachment>,
        connected_apps: Vec<SharedString>,
        folder: Option<SharedString>,
    },
    /// User selected a different LLM from the model popover.
    ModelChanged(ChatModelOption),
    /// User attached a file chip.
    Attach(ComposerAttachment),
    /// User removed an attached file chip by id.
    RemoveAttachment(SharedString),
    /// Connected app ids changed.
    AppsChanged(Vec<SharedString>),
    /// Scoped workspace folder changed (`None` when cleared).
    FolderChanged(Option<SharedString>),
    /// Execution approval policy changed.
    ApprovalModeChanged(ComposerApprovalMode),
    /// User approved or denied the inline tool request banner.
    ToolApprovalDecision { id: SharedString, approved: bool },
    /// Voice dictation mode started (`true`) or ended (`false`).
    VoiceRecordingChanged(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenMenu {
    Model,
    Approval,
    Folder,
    Apps,
}

fn default_models() -> Vec<ChatModelOption> {
    vec![
        ChatModelOption::new(
            "claude-sonnet",
            "Claude Sonnet 4.5",
            "claude",
            "Fast coding & tool use",
        ),
        ChatModelOption::new("gpt-5", "GPT-5", "openai", "Deep synthesis & planning"),
        ChatModelOption::new(
            "gemini-2.5-pro",
            "Gemini 2.5 Pro",
            "gemini",
            "2M token repository context",
        ),
        ChatModelOption::new(
            "deepseek-r1",
            "DeepSeek R1",
            "deepseek",
            "Open-weight reasoning",
        ),
    ]
}

fn default_apps() -> Vec<ComposerApp> {
    vec![
        ComposerApp::new("github", "GitHub", true),
        ComposerApp::new("linear", "Linear", true),
        ComposerApp::new("slack", "Slack", true),
        ComposerApp::new("notion", "Notion", false),
        ComposerApp::new("google drive", "Google Drive", false),
    ]
}

fn default_attachments() -> Vec<ComposerAttachment> {
    vec![
        ComposerAttachment::new("att-1", "release-checklist.md", "4.2 KB"),
        ComposerAttachment::new("att-2", "latency-budget.csv", "18 KB"),
    ]
}

fn default_folders() -> Vec<SharedString> {
    vec![
        "kense-ui/registry".into(),
        "apps/web".into(),
        "docs/architecture".into(),
    ]
}

fn default_tool_approval() -> PendingToolApproval {
    PendingToolApproval::new(
        "call-linear-42",
        "linear",
        "Linear MCP wants to create 3 blockers in Q4 Launch",
        r#"linear.create_issues({ team: "KUI", labels: ["launch", "p1"], count: 3 })"#,
    )
}

/// Compute progressive dictation word split `(confirmed, in_flight)` at `elapsed_secs`.
pub(crate) fn split_dictation_words(transcript: &str, elapsed_secs: f32) -> (String, String) {
    let words: Vec<&str> = transcript.split_whitespace().collect();
    if words.is_empty() {
        return (String::new(), String::new());
    }
    let revealed = ((elapsed_secs * 5.2).floor() as usize + 2).min(words.len());
    let confirmed_n = if revealed >= words.len() && elapsed_secs > (words.len() as f32 / 5.2 + 0.35)
    {
        words.len()
    } else {
        revealed.saturating_sub(2)
    };
    let confirmed = words[..confirmed_n].join(" ");
    let trailing = words[confirmed_n..revealed].join(" ");
    (confirmed, trailing)
}

/// Rich AI prompt composer with multiline `Editor`, model popover, voice dictation, and app switches.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::chat_composer::{ChatComposer, ChatComposerEvent};
///
/// fn mount(cx: &mut gpui::App) {
///     let composer = cx.new(|cx| {
///         ChatComposer::new("assistant-composer", cx)
///             .with_placeholder("Ask Kense to inspect PRs or update Linear…")
///     });
///     cx.subscribe(&composer, |_, ev: &ChatComposerEvent, _| {
///         let _ = ev;
///     })
///     .detach();
/// }
/// ```
pub struct ChatComposer {
    id: ElementId,
    focus: FocusHandle,
    placeholder: SharedString,
    initial_draft: SharedString,
    editor: Option<Entity<Editor>>,
    editor_focused: bool,
    models: Vec<ChatModelOption>,
    active_model_idx: usize,
    approval_mode: ComposerApprovalMode,
    approval_requesting_since: Option<Instant>,
    load_ms: u64,
    pending_approval: Option<PendingToolApproval>,
    last_approval: Option<PendingToolApproval>,
    inspect_payload_open: bool,
    attachments: Vec<ComposerAttachment>,
    last_attachments: Vec<ComposerAttachment>,
    exiting_attachments: Vec<(ComposerAttachment, Instant)>,
    animate_added_chips: bool,
    sample_attach_seq: usize,
    folders: Vec<SharedString>,
    active_folder: Option<SharedString>,
    apps: Vec<ComposerApp>,
    dictation_script: SharedString,
    recording_since: Option<Instant>,
    open_menu: Option<OpenMenu>,
    last_open_menu: Option<OpenMenu>,
    menu_cursor: usize,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl EventEmitter<ChatComposerEvent> for ChatComposer {}

impl ChatComposer {
    /// Create a [`ChatComposer`] preloaded with models, connected apps, file chips, and an inline tool banner.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let init_approval = Some(default_tool_approval());
        let init_attachments = default_attachments();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            placeholder: "Ask anything, attach files, or run connected tools…".into(),
            initial_draft: SharedString::default(),
            editor: None,
            editor_focused: false,
            models: default_models(),
            active_model_idx: 0,
            approval_mode: ComposerApprovalMode::RequestApproval,
            approval_requesting_since: None,
            load_ms: 1100,
            pending_approval: init_approval.clone(),
            last_approval: init_approval,
            inspect_payload_open: false,
            attachments: init_attachments.clone(),
            last_attachments: init_attachments,
            exiting_attachments: Vec::new(),
            animate_added_chips: false,
            sample_attach_seq: 0,
            folders: default_folders(),
            active_folder: Some("kense-ui/registry".into()),
            apps: default_apps(),
            dictation_script: "Summarize the open PRs in GitHub, cross-check Linear blockers, and post the release checklist to Slack.".into(),
            recording_since: None,
            open_menu: None,
            last_open_menu: None,
            menu_cursor: 0,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(640.)),
            toast: None,
        }
    }

    /// Set the placeholder shown when the message editor is empty.
    pub fn with_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Pre-populate an initial prompt draft inside the multiline editor.
    pub fn with_draft(mut self, draft: impl Into<SharedString>) -> Self {
        self.initial_draft = draft.into();
        self
    }

    /// Override the selectable [`ChatModelOption`] entries (`0`th item selected initially).
    pub fn with_models(mut self, models: Vec<ChatModelOption>) -> Self {
        if !models.is_empty() {
            self.models = models;
            self.active_model_idx = 0;
        }
        self
    }

    /// Override the connected [`ComposerApp`] integrations.
    pub fn with_apps(mut self, apps: Vec<ComposerApp>) -> Self {
        self.apps = apps;
        self
    }

    /// Override the initial file [`ComposerAttachment`] chips.
    pub fn with_attachments(mut self, attachments: Vec<ComposerAttachment>) -> Self {
        self.last_attachments = attachments.clone();
        self.attachments = attachments;
        self
    }

    /// Override the available workspace folders (`empty` hides the folder picker).
    pub fn with_folders(mut self, folders: Vec<SharedString>) -> Self {
        self.folders = folders;
        self
    }

    /// Set or clear (`None`) the initially scoped workspace folder.
    pub fn with_folder(mut self, folder: Option<SharedString>) -> Self {
        self.active_folder = folder;
        self
    }

    /// Set or hide (`None`) the inline tool approval request banner above the composer.
    pub fn with_tool_approval(mut self, approval: Option<PendingToolApproval>) -> Self {
        if approval.is_some() {
            self.last_approval = approval.clone();
        }
        self.pending_approval = approval;
        self
    }

    /// Set the simulated transcript revealed during live voice dictation.
    pub fn with_dictation(mut self, transcript: impl Into<SharedString>) -> Self {
        self.dictation_script = transcript.into();
        self
    }

    /// Set the duration in milliseconds of the `"Request approval"` elevation transition.
    pub fn with_load_ms(mut self, load_ms: u64) -> Self {
        self.load_ms = load_ms.clamp(200, 5000);
        self
    }

    /// Toggle loading skeleton placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state presentation.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently selected [`ChatModelOption`].
    pub fn active_model(&self) -> Option<&ChatModelOption> {
        self.models.get(self.active_model_idx)
    }

    /// Return the ids of all currently connected apps.
    pub fn connected_app_ids(&self) -> Vec<SharedString> {
        self.apps
            .iter()
            .filter(|a| a.connected)
            .map(|a| a.id.clone())
            .collect()
    }

    /// Return whether live voice dictation is currently recording.
    pub fn is_recording(&self) -> bool {
        self.recording_since.is_some()
    }

    /// Toggle an app connection by id and emit [`ChatComposerEvent::AppsChanged`].
    pub fn toggle_app(&mut self, app_id: &str, cx: &mut Context<Self>) {
        if let Some(app) = self.apps.iter_mut().find(|a| a.id.as_ref() == app_id) {
            app.connected = !app.connected;
            let ids = self.connected_app_ids();
            cx.emit(ChatComposerEvent::AppsChanged(ids));
            cx.notify();
        }
    }

    /// Resolve the active inline tool approval banner (`approved = true` or `false`).
    pub fn decide_tool_approval(&mut self, approved: bool, cx: &mut Context<Self>) {
        if let Some(req) = self.pending_approval.take() {
            self.inspect_payload_open = false;
            let msg = if approved {
                tr(cx, "Approved tool execution", "已批准执行工具调用")
            } else {
                tr(cx, "Denied tool execution", "已拒绝工具调用")
            };
            self.toast = Some((msg.into(), Instant::now()));
            cx.emit(ChatComposerEvent::ToolApprovalDecision {
                id: req.id,
                approved,
            });
            cx.notify();
        }
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.editor.is_none() {
            let ph = self.placeholder.clone();
            let init = self.initial_draft.clone();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::InlineMulti, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 13.;
                if !init.is_empty() {
                    e.set_value(init.to_string(), w, cx);
                }
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    this.open_menu = None;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => this.submit_prompt(w, cx),
                EditEvent::Escape => {
                    if this.open_menu.take().is_some() || this.recording_since.take().is_some() {
                        cx.notify();
                    } else {
                        this.focus.focus(w, cx);
                    }
                }
                _ => {}
            })
            .detach();
            cx.subscribe_in(
                &ed,
                w,
                |this, _, ev: &EditorShortcutEvent, w, cx| match ev {
                    EditorShortcutEvent::CommandSubmit => this.submit_prompt(w, cx),
                },
            )
            .detach();
            self.editor = Some(ed);
        }
    }

    fn submit_prompt(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let draft = self
            .editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        if draft.is_empty() && self.attachments.is_empty() {
            return;
        }
        let message: SharedString = if draft.is_empty() {
            format!("Analyze {} attached files", self.attachments.len()).into()
        } else {
            draft.into()
        };
        if let Some(ed) = &self.editor {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        let model_id = self
            .models
            .get(self.active_model_idx)
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "claude-sonnet".into());
        if !self.attachments.is_empty() {
            self.last_attachments = self.attachments.clone();
        }
        let attachments = std::mem::take(&mut self.attachments);
        self.exiting_attachments.clear();
        let connected_apps = self.connected_app_ids();
        let folder = self.active_folder.clone();
        self.open_menu = None;
        self.toast = Some((
            SharedString::from(format!(
                "{} · {}",
                tr(cx, "Sent prompt", "已发送指令"),
                model_id
            )),
            Instant::now(),
        ));
        cx.emit(ChatComposerEvent::Send {
            message,
            model_id,
            attachments,
            connected_apps,
            folder,
        });
        cx.notify();
    }

    fn add_sample_attachment(&mut self, cx: &mut Context<Self>) {
        const SAMPLES: [(&str, &str); 4] = [
            ("q4-metrics.json", "12 KB"),
            ("schema-migration.sql", "6.4 KB"),
            ("design-tokens.json", "9.1 KB"),
            ("onboarding-flow.md", "3.8 KB"),
        ];
        let (name, sz) = SAMPLES[self.sample_attach_seq % SAMPLES.len()];
        self.sample_attach_seq += 1;
        let att =
            ComposerAttachment::new(format!("att-extra-{}", self.sample_attach_seq), name, sz);
        self.animate_added_chips = true;
        self.attachments.push(att.clone());
        self.last_attachments = self.attachments.clone();
        cx.emit(ChatComposerEvent::Attach(att));
        cx.notify();
    }

    fn start_recording(&mut self, cx: &mut Context<Self>) {
        self.open_menu = None;
        self.recording_since = Some(Instant::now());
        cx.emit(ChatComposerEvent::VoiceRecordingChanged(true));
        cx.notify();
    }

    fn finish_recording(&mut self, commit: bool, w: &mut Window, cx: &mut Context<Self>) {
        let Some(started) = self.recording_since.take() else {
            return;
        };
        if commit {
            let elapsed = if disabled_motion(cx) {
                99.0
            } else {
                started.elapsed().as_secs_f32()
            };
            let (confirmed, trailing) = split_dictation_words(&self.dictation_script, elapsed);
            let combined = match (confirmed.is_empty(), trailing.is_empty()) {
                (true, true) => self.dictation_script.to_string(),
                (false, true) => confirmed,
                (true, false) => trailing,
                (false, false) => format!("{confirmed} {trailing}"),
            };
            if let Some(ed) = &self.editor {
                let existing = ed.read(cx).value().trim().to_owned();
                let next = if existing.is_empty() {
                    combined
                } else {
                    format!("{existing} {combined}")
                };
                ed.update(cx, |e, cx| {
                    e.set_value(next, w, cx);
                    e.focus(w, cx);
                });
            }
        }
        cx.emit(ChatComposerEvent::VoiceRecordingChanged(false));
        cx.notify();
    }

    fn toggle_menu(&mut self, target: OpenMenu, cx: &mut Context<Self>) {
        if self.open_menu == Some(target) {
            self.open_menu = None;
        } else {
            self.open_menu = Some(target);
            self.last_open_menu = Some(target);
            self.menu_cursor = match target {
                OpenMenu::Model => self.active_model_idx,
                OpenMenu::Approval => match self.approval_mode {
                    ComposerApprovalMode::RequestApproval => 0,
                    ComposerApprovalMode::ApproveEdits => 1,
                    ComposerApprovalMode::Unrestricted => 2,
                },
                OpenMenu::Folder => self
                    .active_folder
                    .as_ref()
                    .and_then(|f| self.folders.iter().position(|x| x == f))
                    .unwrap_or(0),
                OpenMenu::Apps => 0,
            };
        }
        cx.notify();
    }

    fn menu_len(&self, menu: OpenMenu) -> usize {
        match menu {
            OpenMenu::Model => self.models.len(),
            OpenMenu::Approval => 3,
            OpenMenu::Folder => self.folders.len(),
            OpenMenu::Apps => self.apps.len(),
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if key == "escape" {
            if self.open_menu.take().is_some() {
                cx.notify();
                return;
            }
            if self.recording_since.is_some() {
                self.finish_recording(false, w, cx);
                return;
            }
        }
        let Some(menu) = self.open_menu else {
            return;
        };
        let len = self.menu_len(menu);
        if len == 0 {
            return;
        }
        match key {
            "down" => {
                self.menu_cursor = (self.menu_cursor + 1) % len;
                cx.notify();
            }
            "up" => {
                self.menu_cursor = if self.menu_cursor == 0 {
                    len - 1
                } else {
                    self.menu_cursor - 1
                };
                cx.notify();
            }
            "home" => {
                self.menu_cursor = 0;
                cx.notify();
            }
            "end" => {
                self.menu_cursor = len - 1;
                cx.notify();
            }
            "enter" | "space" => {
                let idx = self.menu_cursor.min(len - 1);
                match menu {
                    OpenMenu::Model => {
                        self.active_model_idx = idx;
                        if let Some(m) = self.models.get(idx).cloned() {
                            cx.emit(ChatComposerEvent::ModelChanged(m));
                        }
                        self.open_menu = None;
                    }
                    OpenMenu::Approval => {
                        let next_mode = match idx {
                            0 => ComposerApprovalMode::RequestApproval,
                            1 => ComposerApprovalMode::ApproveEdits,
                            _ => ComposerApprovalMode::Unrestricted,
                        };
                        self.approval_mode = next_mode;
                        cx.emit(ChatComposerEvent::ApprovalModeChanged(next_mode));
                        self.open_menu = None;
                    }
                    OpenMenu::Folder => {
                        if let Some(f) = self.folders.get(idx).cloned() {
                            let next = if self.active_folder.as_ref() == Some(&f) {
                                None
                            } else {
                                Some(f)
                            };
                            self.active_folder = next.clone();
                            cx.emit(ChatComposerEvent::FolderChanged(next));
                        }
                        self.open_menu = None;
                    }
                    OpenMenu::Apps => {
                        if let Some(app_id) = self.apps.get(idx).map(|a| a.id.to_string()) {
                            self.toggle_app(&app_id, cx);
                        }
                    }
                }
                cx.notify();
            }
            _ => {}
        }
    }

    fn render_model_artwork(provider: &str, side: f32, p: &ProPalette, cx: &App) -> AnyElement {
        if let Some(art) = pro_app_artwork(provider, side, cx) {
            art
        } else {
            pro_icon(ProIcon::Sparkles, side, p.strong)
        }
    }
}

impl Render for ChatComposer {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editor(w, cx);
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let measure_cell = self.measured_width.clone();
        let compact = width < 440.;

        // Settle simulated `"Request approval"` elevation handshake after `load_ms`.
        let mut requesting_access = false;
        if let Some(started) = self.approval_requesting_since {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            if reduced || elapsed_ms >= self.load_ms {
                self.approval_requesting_since = None;
                self.approval_mode = ComposerApprovalMode::Unrestricted;
                cx.emit(ChatComposerEvent::ApprovalModeChanged(
                    ComposerApprovalMode::Unrestricted,
                ));
            } else {
                requesting_access = true;
                w.request_animation_frame();
            }
        }

        let rec_elapsed = self.recording_since.map(|st| {
            if !reduced {
                w.request_animation_frame();
            }
            st.elapsed().as_secs_f32()
        });
        let recording = rec_elapsed.is_some();

        if self.loading {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .p(px(16.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(620.))
                        .p(px(14.))
                        .rounded(px(16.))
                        .bg(p.tray)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(480., 46., 10., 0.3, cx))
                        .child(pro_skeleton_bar(320., 24., 6., 0.6, cx)),
                );
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .p(px(16.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(620.))
                        .rounded(px(14.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::NewChat,
                            tr(cx, "Composer disabled", "输入器当前未启用"),
                            tr(
                                cx,
                                "Connect a model provider to start prompting and running tools.",
                                "请先启用模型提供方以开始对话并调用已连接的应用工具。",
                            ),
                            Some((
                                pro_eid(&self.id, "enable-composer"),
                                tr(cx, "Enable composer", "启用输入器").into(),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                );
        }

        let draft_non_empty = self
            .editor
            .as_ref()
            .is_some_and(|e| !e.read(cx).value().trim().is_empty())
            || !self.attachments.is_empty();

        if let Some(ref cur_req) = self.pending_approval {
            self.last_approval = Some(cur_req.clone());
        }
        if !self.attachments.is_empty() {
            self.last_attachments = self.attachments.clone();
        }
        if let Some(cur_menu) = self.open_menu {
            self.last_open_menu = Some(cur_menu);
        }
        self.exiting_attachments
            .retain(|(_, started)| !reduced && started.elapsed().as_secs_f32() < 0.18);
        if !self.exiting_attachments.is_empty() {
            w.request_animation_frame();
        }

        // 1. Inline Tool Approval Banner tucked cleanly at the top of the outer tray (`pro_fold` + tactile `pro_btn`).
        let banner_open = self.pending_approval.is_some();
        let shown_approval = self
            .pending_approval
            .clone()
            .or_else(|| self.last_approval.clone());
        let approval_body: AnyElement = if let Some(req) = shown_approval {
            let app_tile = pro_app_tile(&req.app, 18., cx);
            let payload_txt = req.payload.clone();
            let payload_fold = pro_fold(
                pro_eid(&self.id, "tool-payload-fold"),
                self.inspect_payload_open,
                34.,
                0.,
                move || {
                    div()
                        .pt(px(2.))
                        .child(
                            div()
                                .px(px(8.))
                                .py(px(5.))
                                .rounded(px(6.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line)
                                .text_size(px(11.))
                                .text_color(p.text)
                                .child(payload_txt),
                        )
                        .into_any_element()
                },
            );

            div()
                .w_full()
                .px(px(10.))
                .pt(px(8.))
                .pb(px(6.))
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .min_w_0()
                                .flex_1()
                                .child(app_tile)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(req.summary),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .child(pro_btn(
                                    pro_eid(&self.id, "tool-inspect"),
                                    tr(cx, "Inspect", "查看参数"),
                                    None,
                                    ProBtnStyle::Ghost,
                                    true,
                                    cx,
                                    |this, _, cx| {
                                        this.inspect_payload_open = !this.inspect_payload_open;
                                        cx.notify();
                                    },
                                ))
                                .child(pro_btn(
                                    pro_eid(&self.id, "tool-deny"),
                                    tr(cx, "Deny", "拒绝"),
                                    None,
                                    ProBtnStyle::Secondary,
                                    true,
                                    cx,
                                    |this, _, cx| {
                                        this.decide_tool_approval(false, cx);
                                    },
                                ))
                                .child(pro_btn(
                                    pro_eid(&self.id, "tool-allow"),
                                    tr(cx, "Allow", "允许执行"),
                                    Some(ProIcon::Check),
                                    ProBtnStyle::Primary,
                                    true,
                                    cx,
                                    |this, _, cx| {
                                        this.decide_tool_approval(true, cx);
                                    },
                                )),
                        ),
                )
                .child(payload_fold)
                .into_any_element()
        } else {
            div().into_any_element()
        };
        let approval_banner = pro_fold(
            pro_eid(&self.id, "tool-approval-fold"),
            banner_open,
            54.,
            0.,
            move || approval_body,
        );

        // 2. Attachment Chips inside the write card (`pro_fold` + per-chip entrance/exit presence).
        let chips_open = !self.attachments.is_empty() || !self.exiting_attachments.is_empty();
        let chips_body: AnyElement = {
            let live_items: Vec<(ComposerAttachment, Option<f32>)> = if chips_open {
                let mut v: Vec<(ComposerAttachment, Option<f32>)> = self
                    .attachments
                    .iter()
                    .cloned()
                    .map(|a| (a, None))
                    .collect();
                for (ex_att, started) in &self.exiting_attachments {
                    let u = (started.elapsed().as_secs_f32() / 0.18).clamp(0., 1.);
                    v.push((ex_att.clone(), Some(u)));
                }
                v
            } else {
                self.last_attachments
                    .iter()
                    .cloned()
                    .map(|a| (a, None))
                    .collect()
            };
            let animate_in = self.animate_added_chips;
            let mut row = div()
                .pb(px(8.))
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(6.));
            for (ai, (att, exiting_u)) in live_items.into_iter().enumerate() {
                let aid = att.id.clone();
                let att_snapshot = att.clone();
                let chip_eid = pro_eid(&self.id, format!("chip-item-{aid}"));
                let rm_btn = pro_icon_btn(
                    pro_eid(&self.id, format!("rm-att-{ai}")),
                    "Remove attachment",
                    ProIcon::Close,
                    18.,
                    false,
                    cx,
                    move |this, _, cx| {
                        let rm_id = aid.clone();
                        if this.attachments.iter().any(|x| x.id == rm_id) {
                            if this.attachments.len() > 1 && !disabled_motion(cx) {
                                this.exiting_attachments
                                    .push((att_snapshot.clone(), Instant::now()));
                            }
                            this.attachments.retain(|x| x.id != rm_id);
                            cx.emit(ChatComposerEvent::RemoveAttachment(rm_id));
                            cx.notify();
                        }
                    },
                );
                let chip_card = div()
                    .h(px(26.))
                    .pl(px(7.))
                    .pr(px(5.))
                    .rounded(px(7.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(pro_icon(ProIcon::PageDoc, 13., p.blue))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(att.name),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.faint)
                            .child(att.size_label),
                    )
                    .child(rm_btn)
                    .into_any_element();
                row = row.child(ChipEntrance {
                    id: chip_eid,
                    animate_in,
                    exiting_progress: exiting_u,
                    content: chip_card,
                });
            }
            row.into_any_element()
        };
        let attachments_fold = pro_fold(
            pro_eid(&self.id, "att-chips-fold"),
            chips_open,
            34.,
            0.,
            move || chips_body,
        );

        // 3. Message Editor or Live Voice Dictation Draft Surface.
        let ed_click = self.editor.clone();
        let field_area = if let Some(elapsed) = rec_elapsed {
            let (confirmed, trailing) =
                split_dictation_words(&self.dictation_script, if reduced { 99.0 } else { elapsed });
            div()
                .min_h(px(44.))
                .px(px(4.))
                .flex()
                .flex_wrap()
                .items_start()
                .gap_x(px(4.))
                .text_size(px(13.))
                .line_height(px(20.))
                .when(!confirmed.is_empty(), |d| {
                    d.child(div().text_color(p.strong).child(confirmed))
                })
                .when(!trailing.is_empty(), |d| {
                    d.child(div().text_color(p.muted).child(trailing))
                })
                .into_any_element()
        } else {
            div()
                .id(pro_eid(&self.id, "editor-hit"))
                .h(px(44.))
                .px(px(4.))
                .w_full()
                .cursor_text()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |_, _, w, cx| {
                        if let Some(ed) = &ed_click {
                            ed.read(cx).focus.clone().focus(w, cx);
                        }
                    }),
                )
                .when_some(self.editor.clone(), |d, ed| d.child(ed))
                .into_any_element()
        };

        // 4. Toolbar inside the write card.
        let active_model = self
            .models
            .get(self.active_model_idx)
            .cloned()
            .unwrap_or_else(|| {
                ChatModelOption::new("claude-sonnet", "Claude Sonnet 4.5", "claude", "")
            });
        let is_unrestricted = self.approval_mode == ComposerApprovalMode::Unrestricted;
        let approval_fg = if is_unrestricted { p.red_ink } else { p.muted };

        let appr_open = self.open_menu == Some(OpenMenu::Approval);
        let appr_inner = div()
            .h(px(28.))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(6.))
            .child(pro_svg(
                "0 0 68.155 68.155",
                SVG_HAND_PALM,
                15.,
                approval_fg,
            ))
            .when(!compact, |d| {
                d.child(if requesting_access {
                    pro_shimmer_text(
                        pro_eid(&self.id, "req-shim"),
                        tr(cx, "Requesting access…", "正在请求权限…"),
                        p.muted,
                        p.strong,
                        12.5,
                        FontWeight::NORMAL,
                        true,
                    )
                } else {
                    pro_swap_label(
                        pro_eid(&self.id, "appr-mode-lbl"),
                        self.approval_mode.label(cx),
                        12.5,
                        FontWeight::NORMAL,
                        if is_unrestricted { p.red_ink } else { p.text },
                    )
                })
            });

        let left_tools = div()
            .flex()
            .items_center()
            .gap(px(4.))
            .min_w_0()
            // `+` Add attachment button (`pro_icon_btn`)
            .child(pro_icon_btn(
                pro_eid(&self.id, "btn-attach"),
                tr(cx, "Add attachment", "添加附件"),
                ProIcon::Plus,
                28.,
                false,
                cx,
                |this, _, cx| this.add_sample_attachment(cx),
            ))
            // Approval mode trigger (`pro_tactile`)
            .child(
                pro_tactile(
                    pro_eid(&self.id, "btn-approval-mode"),
                    self.approval_mode.label(cx),
                    8.,
                    if appr_open {
                        if is_unrestricted { p.red_soft } else { p.hover }
                    } else {
                        transparent_black()
                    },
                    if is_unrestricted { p.red_soft } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    appr_open,
                    appr_inner,
                    cx,
                    |this, _, cx| {
                        this.toggle_menu(OpenMenu::Approval, cx);
                    },
                )
                .flex_none()
                .aria_expanded(appr_open),
            );

        let right_tools = if recording {
            let t = rec_elapsed.unwrap_or(0.0);
            let mut waveform = div()
                .mr(px(4.))
                .h(px(22.))
                .flex()
                .items_center()
                .gap(px(3.));
            for i in 0..14usize {
                let bar_h = if reduced {
                    6. + ((i * 5) % 10) as f32
                } else {
                    let wave = (t * 6.8 + i as f32 * 0.62).sin() * 0.5 + 0.5;
                    let env = ((i as f32 / 13.0) * std::f32::consts::PI).sin();
                    (4.0 + 13.0 * wave * (0.35 + 0.65 * env)).round()
                };
                waveform =
                    waveform.child(div().w(px(2.5)).h(px(bar_h)).rounded(px(1.25)).bg(p.blue));
            }

            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(waveform)
                .child(pro_icon_btn(
                    pro_eid(&self.id, "voice-discard"),
                    tr(cx, "Discard recording", "放弃录音"),
                    ProIcon::Close,
                    28.,
                    false,
                    cx,
                    |this, w, cx| {
                        this.finish_recording(false, w, cx);
                    },
                ))
                .child(
                    pro_tactile(
                        pro_eid(&self.id, "voice-commit"),
                        tr(cx, "Use recording", "使用语音转写"),
                        8.,
                        p.primary,
                        pro_mix_color(p.primary, p.on_primary, 0.14),
                        transparent_black(),
                        transparent_black(),
                        false,
                        div()
                            .size(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Check, 15., p.on_primary)),
                        cx,
                        |this, w, cx| {
                            this.finish_recording(true, w, cx);
                        },
                    )
                    .flex_none(),
                )
        } else {
            let model_open = self.open_menu == Some(OpenMenu::Model);
            let model_btn_inner = div()
                .h(px(28.))
                .pl(px(8.))
                .pr(px(6.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(Self::render_model_artwork(
                    &active_model.provider,
                    14.,
                    &p,
                    cx,
                ))
                .child(
                    div()
                        .max_w(px(if compact { 100. } else { 150. }))
                        .overflow_hidden()
                        .child(pro_swap_label(
                            pro_eid(&self.id, "active-model-lbl"),
                            active_model.name.clone(),
                            12.5,
                            FontWeight::MEDIUM,
                            p.ink,
                        )),
                )
                .child(pro_icon(
                    if model_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    13.,
                    p.muted,
                ));

            let send_id = pro_eid(&self.id, "btn-send");
            let send_state_id = pro_eid(&send_id, "send-st");
            let on_send: ComposerClickHandler =
                Rc::new(cx.listener(|this, _: &ClickEvent, w, cx| {
                    this.submit_prompt(w, cx);
                }));

            div()
                .flex()
                .items_center()
                .gap(px(4.))
                // Model picker button (`pro_tactile`)
                .child(
                    pro_tactile(
                        pro_eid(&self.id, "btn-model-picker"),
                        format!("Model: {}", active_model.name),
                        8.,
                        if model_open {
                            p.hover
                        } else {
                            transparent_black()
                        },
                        p.hover,
                        transparent_black(),
                        transparent_black(),
                        model_open,
                        model_btn_inner,
                        cx,
                        |this, _, cx| {
                            this.toggle_menu(OpenMenu::Model, cx);
                        },
                    )
                    .flex_none()
                    .aria_expanded(model_open),
                )
                // Voice input button (`pro_icon_btn`)
                .child(pro_icon_btn(
                    pro_eid(&self.id, "btn-voice"),
                    tr(cx, "Voice input", "语音输入"),
                    ProIcon::Mic,
                    28.,
                    false,
                    cx,
                    |this, _, cx| this.start_recording(cx),
                ))
                // Snappy Send button (`ComposerSendVisual` with `Slide::snappy` press & active spring)
                .child(
                    div()
                        .id(send_id)
                        .ml(px(2.))
                        .size(px(28.))
                        .flex_none()
                        .rounded(px(8.))
                        .child(ComposerSendVisual {
                            state_id: send_state_id,
                            aria: tr(cx, "Send message", "发送消息").into(),
                            enabled: draft_non_empty,
                            on_click: on_send,
                        }),
                )
        };

        let write_card = div()
            .w_full()
            .p(px(12.))
            .pb(px(10.))
            .rounded(px(15.))
            .bg(p.card)
            .border_1()
            .border_color(if self.editor_focused {
                p.line_strong
            } else {
                p.line
            })
            .shadow_xs()
            .flex()
            .flex_col()
            .child(attachments_fold)
            .child(field_area)
            .child(
                div()
                    .mt(px(10.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(left_tools)
                    .child(right_tools),
            );

        // 5. Bottom Tray Footer: Folder picker (left) + Connected apps with stacked brand marks (right).
        let connected_list: Vec<ComposerApp> =
            self.apps.iter().filter(|a| a.connected).cloned().collect();
        let folder_label = self
            .active_folder
            .clone()
            .unwrap_or_else(|| tr(cx, "Work in a folder", "选择工作目录").into());

        let mut app_marks = div().flex().items_center().gap(px(5.));
        for app in connected_list.iter().take(4) {
            if let Some(mark) = pro_app_artwork(&app.id, 13., cx) {
                app_marks = app_marks.child(mark);
            } else {
                app_marks = app_marks.child(pro_app_tile(&app.id, 14., cx));
            }
        }

        let tray_footer = div()
            .h(px(36.))
            .px(px(6.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(if !self.folders.is_empty() {
                div()
                    .id(pro_eid(&self.id, "btn-folder"))
                    .role(Role::Button)
                    .aria_label(folder_label.clone())
                    .aria_expanded(self.open_menu == Some(OpenMenu::Folder))
                    .tab_index(0)
                    .h(px(26.))
                    .px(px(8.))
                    .rounded(px(8.))
                    .bg(if self.open_menu == Some(OpenMenu::Folder) {
                        p.hover
                    } else {
                        transparent_black()
                    })
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_menu(OpenMenu::Folder, cx);
                    }))
                    .child(pro_svg(
                        "0 0 12 12",
                        SVG_BLUE_FOLDER,
                        14.,
                        rgb(0x0099ff).into(),
                    ))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(if self.active_folder.is_some() {
                                p.ink
                            } else {
                                p.muted
                            })
                            .child(folder_label),
                    )
                    .into_any_element()
            } else {
                div().into_any_element()
            })
            .child(
                div()
                    .id(pro_eid(&self.id, "btn-apps"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Connect apps", "连接应用"))
                    .aria_expanded(self.open_menu == Some(OpenMenu::Apps))
                    .tab_index(0)
                    .h(px(26.))
                    .px(px(8.))
                    .rounded(px(8.))
                    .bg(if self.open_menu == Some(OpenMenu::Apps) {
                        p.hover
                    } else {
                        transparent_black()
                    })
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_menu(OpenMenu::Apps, cx);
                    }))
                    .child(div().text_size(px(12.)).text_color(p.muted).child(
                        if connected_list.is_empty() {
                            tr(cx, "Connect apps", "连接应用").to_owned()
                        } else {
                            format!(
                                "{} ({})",
                                tr(cx, "Connected apps", "已连应用"),
                                connected_list.len()
                            )
                        },
                    ))
                    .when(!connected_list.is_empty(), |d| d.child(app_marks)),
            );

        // 6. Floating Popover Menu (`pro_popover_presence` + `pro_pill_indicator` + `pro_check_draw`).
        let popover_open = self.open_menu.is_some();
        let shown_menu = self.open_menu.or(self.last_open_menu);
        let popover_body: Option<AnyElement> = {
            let cursor = self.menu_cursor;
            let models = self.models.clone();
            let active_model_idx = self.active_model_idx;
            let approval_mode = self.approval_mode;
            let folders = self.folders.clone();
            let active_folder = self.active_folder.clone();
            let apps = self.apps.clone();
            let has_pending_banner = self.pending_approval.is_some();
            let base_id = self.id.clone();
            let entity = cx.entity().clone();

            shown_menu.map(|open| {
                let align_right = matches!(open, OpenMenu::Model | OpenMenu::Apps);
                let menu_key = match open {
                    OpenMenu::Model => "menu-model",
                    OpenMenu::Approval => "menu-approval",
                    OpenMenu::Folder => "menu-folder",
                    OpenMenu::Apps => "menu-apps",
                };
                let row_sensor = pro_selection_sensor();
                let mut rows_wrap =
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(pro_pill_indicator(
                            pro_eid(&base_id, format!("pop-pill-{menu_key}")),
                            format!("cur-{cursor}"),
                            row_sensor.clone(),
                            7.,
                            p.hover,
                            None,
                            false,
                        ));

                let mut panel = div()
                    .w(px(276.))
                    .p(px(5.))
                    .rounded(px(11.))
                    .bg(if p.dark { rgb(0x202024) } else { rgb(0xffffff) })
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap(px(2.));

                match open {
                    OpenMenu::Model => {
                        panel = panel.child(
                            div()
                                .px(px(8.))
                                .py(px(4.))
                                .text_size(px(11.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.muted)
                                .child(tr(cx, "SELECT MODEL", "选择模型")),
                        );
                        for (mi, m) in models.into_iter().enumerate() {
                            let selected = mi == active_model_idx;
                            let highlighted = mi == cursor;
                            let ent = entity.clone();
                            let ent_hov = entity.clone();
                            let m_emit = m.clone();
                            rows_wrap = rows_wrap.child(
                                div()
                                    .id(pro_eid(&base_id, format!("opt-model-{mi}")))
                                    .role(Role::ListBoxOption)
                                    .aria_label(m.name.clone())
                                    .aria_selected(selected)
                                    .tab_index(0)
                                    .relative()
                                    .px(px(8.))
                                    .py(px(6.))
                                    .rounded(px(7.))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .gap(px(9.))
                                    .when(highlighted, |d| d.child(pro_mark_selected(&row_sensor)))
                                    .on_hover(move |inside: &bool, _, app| {
                                        if *inside {
                                            ent_hov.update(app, |this, cx| {
                                                if this.menu_cursor != mi {
                                                    this.menu_cursor = mi;
                                                    cx.notify();
                                                }
                                            });
                                        }
                                    })
                                    .on_click(move |_, _, app| {
                                        let chosen = m_emit.clone();
                                        ent.update(app, |this, cx| {
                                            this.active_model_idx = mi;
                                            this.open_menu = None;
                                            cx.emit(ChatComposerEvent::ModelChanged(chosen));
                                            cx.notify();
                                        });
                                    })
                                    .child(Self::render_model_artwork(&m.provider, 16., &p, cx))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(px(12.5))
                                                    .font_weight(if selected {
                                                        FontWeight::SEMIBOLD
                                                    } else {
                                                        FontWeight::MEDIUM
                                                    })
                                                    .text_color(p.strong)
                                                    .child(m.name),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.5))
                                                    .text_color(p.muted)
                                                    .truncate()
                                                    .child(m.description),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w(px(if selected { 15. } else { 0. }))
                                            .h(px(15.))
                                            .overflow_hidden()
                                            .child(pro_check_draw(
                                                pro_eid(&base_id, format!("opt-model-chk-{mi}")),
                                                selected,
                                                15.,
                                                false,
                                                p.blue,
                                            )),
                                    ),
                            );
                        }
                        panel = panel.child(rows_wrap);
                    }
                    OpenMenu::Approval => {
                        let modes = [
                            (
                                ComposerApprovalMode::RequestApproval,
                                tr(
                                    cx,
                                    "Ask before running external tools",
                                    "执行外部工具前确认",
                                ),
                            ),
                            (
                                ComposerApprovalMode::ApproveEdits,
                                tr(
                                    cx,
                                    "Auto-apply file edits, ask on network",
                                    "自动写入代码，联网前询问",
                                ),
                            ),
                            (
                                ComposerApprovalMode::Unrestricted,
                                tr(
                                    cx,
                                    "Run commands & tools without prompts",
                                    "无需确认直接自主执行",
                                ),
                            ),
                        ];
                        for (idx, (mode, desc)) in modes.into_iter().enumerate() {
                            let selected = mode == approval_mode;
                            let highlighted = idx == cursor;
                            let alert_tone = mode == ComposerApprovalMode::Unrestricted;
                            let ent = entity.clone();
                            let ent_hov = entity.clone();
                            rows_wrap = rows_wrap.child(
                                div()
                                    .id(pro_eid(&base_id, format!("opt-appr-{idx}")))
                                    .role(Role::ListBoxOption)
                                    .aria_label(mode.label(cx))
                                    .aria_selected(selected)
                                    .tab_index(0)
                                    .relative()
                                    .px(px(8.))
                                    .py(px(6.))
                                    .rounded(px(7.))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .when(highlighted, |d| d.child(pro_mark_selected(&row_sensor)))
                                    .on_hover(move |inside: &bool, _, app| {
                                        if *inside {
                                            ent_hov.update(app, |this, cx| {
                                                if this.menu_cursor != idx {
                                                    this.menu_cursor = idx;
                                                    cx.notify();
                                                }
                                            });
                                        }
                                    })
                                    .on_click(move |_, _, app| {
                                        ent.update(app, |this, cx| {
                                            this.open_menu = None;
                                            if mode == ComposerApprovalMode::Unrestricted
                                                && this.approval_mode
                                                    != ComposerApprovalMode::Unrestricted
                                            {
                                                this.approval_requesting_since =
                                                    Some(Instant::now());
                                            } else {
                                                this.approval_mode = mode;
                                                cx.emit(ChatComposerEvent::ApprovalModeChanged(
                                                    mode,
                                                ));
                                            }
                                            cx.notify();
                                        });
                                    })
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(px(12.5))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(if alert_tone {
                                                        p.red_ink
                                                    } else {
                                                        p.strong
                                                    })
                                                    .child(mode.label(cx)),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.5))
                                                    .text_color(p.muted)
                                                    .child(desc),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w(px(if selected { 15. } else { 0. }))
                                            .h(px(15.))
                                            .overflow_hidden()
                                            .child(pro_check_draw(
                                                pro_eid(&base_id, format!("opt-appr-chk-{idx}")),
                                                selected,
                                                15.,
                                                false,
                                                p.blue,
                                            )),
                                    ),
                            );
                        }
                        panel = panel.child(rows_wrap);
                        if !has_pending_banner {
                            panel = panel.child(
                                div()
                                    .mt(px(3.))
                                    .pt(px(3.))
                                    .border_t_1()
                                    .border_color(p.line)
                                    .child(pro_btn(
                                        pro_eid(&base_id, "requeue-tool-banner"),
                                        tr(
                                            cx,
                                            "Simulate tool approval request",
                                            "模拟工具权限请求",
                                        ),
                                        Some(ProIcon::Shield),
                                        ProBtnStyle::Ghost,
                                        true,
                                        cx,
                                        |this, _, cx| {
                                            let req = default_tool_approval();
                                            this.last_approval = Some(req.clone());
                                            this.pending_approval = Some(req);
                                            this.open_menu = None;
                                            cx.notify();
                                        },
                                    )),
                            );
                        }
                    }
                    OpenMenu::Folder => {
                        panel = panel.child(
                            div()
                                .px(px(8.))
                                .py(px(4.))
                                .text_size(px(11.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.muted)
                                .child(tr(cx, "WORKSPACE FOLDER", "工作区目录")),
                        );
                        for (fi, folder) in folders.into_iter().enumerate() {
                            let selected = active_folder.as_ref() == Some(&folder);
                            let highlighted = fi == cursor;
                            let ent = entity.clone();
                            let ent_hov = entity.clone();
                            let f_click = folder.clone();
                            rows_wrap = rows_wrap.child(
                                div()
                                    .id(pro_eid(&base_id, format!("opt-folder-{fi}")))
                                    .role(Role::ListBoxOption)
                                    .aria_label(folder.clone())
                                    .aria_selected(selected)
                                    .tab_index(0)
                                    .relative()
                                    .h(px(30.))
                                    .px(px(8.))
                                    .rounded(px(7.))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .when(highlighted, |d| d.child(pro_mark_selected(&row_sensor)))
                                    .on_hover(move |inside: &bool, _, app| {
                                        if *inside {
                                            ent_hov.update(app, |this, cx| {
                                                if this.menu_cursor != fi {
                                                    this.menu_cursor = fi;
                                                    cx.notify();
                                                }
                                            });
                                        }
                                    })
                                    .on_click(move |_, _, app| {
                                        let picked = f_click.clone();
                                        ent.update(app, |this, cx| {
                                            let next =
                                                if this.active_folder.as_ref() == Some(&picked) {
                                                    None
                                                } else {
                                                    Some(picked)
                                                };
                                            this.active_folder = next.clone();
                                            this.open_menu = None;
                                            cx.emit(ChatComposerEvent::FolderChanged(next));
                                            cx.notify();
                                        });
                                    })
                                    .child(pro_icon(ProIcon::Folder, 14., p.blue))
                                    .child(
                                        div()
                                            .flex_1()
                                            .truncate()
                                            .text_size(px(12.5))
                                            .text_color(p.strong)
                                            .child(folder),
                                    )
                                    .child(
                                        div()
                                            .w(px(if selected { 15. } else { 0. }))
                                            .h(px(15.))
                                            .overflow_hidden()
                                            .child(pro_check_draw(
                                                pro_eid(&base_id, format!("opt-folder-chk-{fi}")),
                                                selected,
                                                15.,
                                                false,
                                                p.blue,
                                            )),
                                    ),
                            );
                        }
                        panel = panel.child(rows_wrap);
                    }
                    OpenMenu::Apps => {
                        panel = panel.child(
                            div()
                                .px(px(8.))
                                .py(px(4.))
                                .text_size(px(11.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.muted)
                                .child(tr(cx, "CONNECTED APPS", "已连接的工具应用")),
                        );
                        for (ai, app_item) in apps.into_iter().enumerate() {
                            let highlighted = ai == cursor;
                            let app_id_str = app_item.id.to_string();
                            let app_id_row = app_id_str.clone();
                            let ent_row = entity.clone();
                            let ent_hov = entity.clone();
                            rows_wrap = rows_wrap.child(
                                div()
                                    .id(pro_eid(&base_id, format!("opt-app-row-{ai}")))
                                    .relative()
                                    .h(px(34.))
                                    .px(px(8.))
                                    .rounded(px(7.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .cursor_pointer()
                                    .when(highlighted, |d| d.child(pro_mark_selected(&row_sensor)))
                                    .on_hover(move |inside: &bool, _, app| {
                                        if *inside {
                                            ent_hov.update(app, |this, cx| {
                                                if this.menu_cursor != ai {
                                                    this.menu_cursor = ai;
                                                    cx.notify();
                                                }
                                            });
                                        }
                                    })
                                    .on_click(move |_, _, app| {
                                        let id_c = app_id_row.clone();
                                        ent_row.update(app, |this, cx| {
                                            this.toggle_app(&id_c, cx);
                                        });
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .child(pro_app_tile(&app_item.id, 18., cx))
                                            .child(
                                                div()
                                                    .text_size(px(12.5))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.strong)
                                                    .child(app_item.name.clone()),
                                            ),
                                    )
                                    .child(pro_switch(
                                        pro_eid(&base_id, format!("sw-app-{ai}")),
                                        app_item.name,
                                        app_item.connected,
                                        cx,
                                        move |this, _, cx| {
                                            this.toggle_app(&app_id_str, cx);
                                        },
                                    )),
                            );
                        }
                        panel = panel.child(rows_wrap);
                    }
                }

                div()
                    .w_full()
                    .mb(px(8.))
                    .flex()
                    .when(align_right, |d| d.justify_end())
                    .child(div().w(px(276.)).child(pro_view_transition(
                        pro_eid(&base_id, "composer-pop-vtx"),
                        menu_key,
                        panel,
                    )))
                    .into_any_element()
            })
        };
        let popover = pro_popover_presence(
            pro_eid(&self.id, "composer-popover"),
            popover_open,
            move || {
                div()
                    .w_full()
                    .when_some(popover_body, |d, b| d.child(b))
                    .into_any_element()
            },
        );

        let outer_tray_bg: Hsla = if p.dark {
            rgb(0x161618).into()
        } else {
            rgb(0xf4f4f5).into()
        };

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(tr(cx, "Message composer", "消息输入框"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .p(px(if compact { 12. } else { 20. }))
            .flex()
            .flex_col()
            .items_center()
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10. && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .w_full()
                .h(px(0.)),
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(620.))
                    .flex()
                    .flex_col()
                    .child(popover)
                    .child(
                        div()
                            .w_full()
                            .rounded(px(16.))
                            .bg(outer_tray_bg)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .child(approval_banner)
                            .child(write_card)
                            .child(tray_footer),
                    ),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{default_apps, default_models, split_dictation_words};

    #[test]
    fn dictation_word_splitter_progresses_confirmed_and_trailing_words() {
        let script = "Summarize the open PRs in GitHub and Linear today";
        let (early_c, early_t) = split_dictation_words(script, 0.2);
        assert_eq!(early_c, "Summarize");
        assert!(!early_t.is_empty());

        let (done_c, done_t) = split_dictation_words(script, 10.0);
        assert_eq!(done_c, script);
        assert!(done_t.is_empty());
    }

    #[test]
    fn defaults_include_four_providers_and_five_workspace_apps() {
        let models = default_models();
        assert_eq!(models.len(), 4);
        assert_eq!(models[0].provider.as_ref(), "claude");
        assert_eq!(models[1].provider.as_ref(), "openai");
        assert_eq!(models[2].provider.as_ref(), "gemini");
        assert_eq!(models[3].provider.as_ref(), "deepseek");

        let apps = default_apps();
        assert_eq!(apps.len(), 5);
        assert_eq!(apps.iter().filter(|a| a.connected).count(), 3);
    }
}
