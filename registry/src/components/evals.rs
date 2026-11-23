//! Prompt evaluation workspace screen with version pass rates, inline prompt diffs, and test suite.
//!
//! Hosts an [`AppShell`] evaluation dashboard with prompt version and model selectors,
//! four regression summary cards, a 12-version pass-rate trend chart, a keyboard-reviewable
//! inline system-prompt diff, a filterable 25-case evaluation table, a right-hand case
//! inspector comparing baseline and candidate outputs, loading skeletons, and empty state.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_eid, pro_empty_state, pro_fold,
            pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_presence,
            pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SPARKLE_DIFF_SVG: &str = r#"<path fill="currentColor" d="M8 1.75c.3 2.9 1.35 3.95 4.25 4.25-2.9.3-3.95 1.35-4.25 4.25C7.7 7.35 6.65 6.3 3.75 6 6.65 5.7 7.7 4.65 8 1.75Z"/><path fill="currentColor" d="M12.5 10.25c.15 1.25.6 1.7 1.75 1.75-1.15.15-1.6.6-1.75 1.75-.15-1.15-.6-1.6-1.75-1.75 1.15-.05 1.6-.5 1.75-1.75Z"/>"#;

/// Verdict outcome for one prompt evaluation test case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EvalVerdict {
    #[default]
    Pass,
    Partial,
    Fail,
}

/// Filter tab above the evaluation test cases table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EvalFilter {
    #[default]
    All,
    Pass,
    Partial,
    Fail,
}

impl EvalFilter {
    /// Return all filter options in segmented control order.
    pub const fn all() -> [Self; 4] {
        [Self::All, Self::Pass, Self::Partial, Self::Fail]
    }

    /// Localized label for the segmented tab.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All", "全部"),
            Self::Pass => tr(cx, "Pass", "通过"),
            Self::Partial => tr(cx, "Partial", "部分通过"),
            Self::Fail => tr(cx, "Fail", "失败"),
        }
    }

    /// Stable key suffix for element ids.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Pass => "pass",
            Self::Partial => "partial",
            Self::Fail => "fail",
        }
    }

    /// Return whether `verdict` passes this filter.
    pub const fn matches(self, verdict: EvalVerdict) -> bool {
        match self {
            Self::All => true,
            Self::Pass => matches!(verdict, EvalVerdict::Pass),
            Self::Partial => matches!(verdict, EvalVerdict::Partial),
            Self::Fail => matches!(verdict, EvalVerdict::Fail),
        }
    }
}

/// Review decision for an inline prompt change hunk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PromptChangeDecision {
    #[default]
    Pending,
    Accepted,
    Rejected,
}

/// One reviewable inline prompt edit replacing `before` with `after`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvalPromptChange {
    pub before: SharedString,
    pub after: SharedString,
    pub decision: PromptChangeDecision,
}

/// Single grader criterion inside the right-hand case inspector.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EvalRubricItem {
    pub name: SharedString,
    pub passed: bool,
    pub note: SharedString,
}

/// One test case row in the evaluation suite.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EvalCase {
    pub id: SharedString,
    pub input: SharedString,
    pub expected: SharedString,
    pub baseline_output: SharedString,
    pub candidate_output: SharedString,
    pub verdict: EvalVerdict,
    pub score: u8,
    pub latency_s: f32,
    pub rubrics: Vec<EvalRubricItem>,
}

/// Events emitted by [`EvalsScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalsEvent {
    Navigate(SharedString),
    SelectVersion(u32),
    SelectModel(SharedString),
    FilterChanged(EvalFilter),
    SelectCase(SharedString),
    ReviewChange {
        index: usize,
        decision: PromptChangeDecision,
    },
    ReviewAll(PromptChangeDecision),
    UndoChanges,
    RunEval,
    RerunCase(SharedString),
    AddCase,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CaseRunState {
    Done,
    Queued,
    Running,
}

enum PromptSegment {
    Text(&'static str),
    Change(usize),
}

const PROMPT_TEMPLATE: [PromptSegment; 15] = [
    PromptSegment::Text("You are a support agent for Acme. Answer the customer's question"),
    PromptSegment::Change(0),
    PromptSegment::Text(". If"),
    PromptSegment::Change(1),
    PromptSegment::Text("not"),
    PromptSegment::Change(2),
    PromptSegment::Text(","),
    PromptSegment::Change(3),
    PromptSegment::Text("and"),
    PromptSegment::Change(4),
    PromptSegment::Text("the ticket to a human."),
    PromptSegment::Change(5),
    PromptSegment::Text("link"),
    PromptSegment::Change(6),
    PromptSegment::Text("."),
];

const VERSION_RATES: [(u32, &str, f32); 12] = [
    (1, "Jan 8", 61.),
    (2, "Jan 14", 64.),
    (3, "Jan 21", 70.),
    (4, "Jan 29", 68.),
    (5, "Feb 4", 74.),
    (6, "Feb 10", 79.),
    (7, "Feb 16", 77.),
    (8, "Feb 21", 83.),
    (9, "Feb 26", 85.),
    (10, "Mar 2", 81.),
    (11, "Mar 6", 88.),
    (12, "Mar 9", 92.),
];

fn default_prompt_changes() -> Vec<EvalPromptChange> {
    vec![
        EvalPromptChange {
            before: "politely and in as much detail as possible".into(),
            after: "in three sentences or fewer, in the customer's language".into(),
            decision: PromptChangeDecision::Pending,
        },
        EvalPromptChange {
            before: "you are".into(),
            after: "the answer is".into(),
            decision: PromptChangeDecision::Pending,
        },
        EvalPromptChange {
            before: "sure".into(),
            after: "in the help center".into(),
            decision: PromptChangeDecision::Pending,
        },
        EvalPromptChange {
            before: "apologize".into(),
            after: "say so".into(),
            decision: PromptChangeDecision::Pending,
        },
        EvalPromptChange {
            before: "escalate".into(),
            after: "offer to hand".into(),
            decision: PromptChangeDecision::Pending,
        },
        EvalPromptChange {
            before: "Always end with a friendly sign-off and a".into(),
            after: "Never promise refunds;".into(),
            decision: PromptChangeDecision::Pending,
        },
        EvalPromptChange {
            before: "to our blog".into(),
            after: "the refund policy instead".into(),
            decision: PromptChangeDecision::Pending,
        },
    ]
}

fn default_rubrics(verdict: EvalVerdict) -> Vec<EvalRubricItem> {
    match verdict {
        EvalVerdict::Pass => vec![
            EvalRubricItem {
                name: "Conciseness (<= 3 sentences)".into(),
                passed: true,
                note: "2 sentences, direct answer without filler sign-off.".into(),
            },
            EvalRubricItem {
                name: "Policy & factual grounding".into(),
                passed: true,
                note: "Matches help-center procedure and refund rules.".into(),
            },
            EvalRubricItem {
                name: "Customer language match".into(),
                passed: true,
                note: "Responded in the exact language of the user prompt.".into(),
            },
        ],
        EvalVerdict::Partial => vec![
            EvalRubricItem {
                name: "Conciseness (<= 3 sentences)".into(),
                passed: true,
                note: "Stayed within the three-sentence budget.".into(),
            },
            EvalRubricItem {
                name: "Policy & factual grounding".into(),
                passed: false,
                note: "Omitted the secondary workaround documented in help center.".into(),
            },
            EvalRubricItem {
                name: "Customer language match".into(),
                passed: true,
                note: "Language and tone match customer expectations.".into(),
            },
        ],
        EvalVerdict::Fail => vec![
            EvalRubricItem {
                name: "Conciseness (<= 3 sentences)".into(),
                passed: true,
                note: "1 sentence response.".into(),
            },
            EvalRubricItem {
                name: "No refund promises".into(),
                passed: false,
                note: "Promised a two-cycle refund instead of linking refund policy.".into(),
            },
            EvalRubricItem {
                name: "Human handoff offer".into(),
                passed: false,
                note: "Claimed refund was queued rather than offering billing handoff.".into(),
            },
        ],
    }
}

fn default_cases() -> Vec<EvalCase> {
    type RawEvalCase<'a> = (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        EvalVerdict,
        u8,
        f32,
    );
    let raw: [RawEvalCase<'static>; 25] = [
        (
            "TC-01",
            "Where is my order #48213? It's been 9 days.",
            "Look up the status, share the tracking link, stay under 3 sentences.",
            "Hi there! Thank you so much for reaching out about order #48213. I checked our shipping system and found it shipped via DHL on May 4; check our blog while you wait!",
            "Sorry for the wait – order #48213 shipped on May 4 via DHL and arrives tomorrow. Track it here: acme.com/track/48213.",
            EvalVerdict::Pass,
            5,
            1.2,
        ),
        (
            "TC-02",
            "Can I get a refund for my annual plan?",
            "Link the refund policy, no promise of a refund, offer human handoff.",
            "We would be happy to process a full refund for your annual plan! Have a wonderful day and check out our latest blog post.",
            "Annual plans can be refunded within 30 days of purchase under our refund policy (acme.com/refunds). Want me to hand this ticket to billing?",
            EvalVerdict::Pass,
            5,
            1.2,
        ),
        (
            "TC-03",
            "Wie ändere ich meine Rechnungsadresse?",
            "Answer in German with the settings path in three sentences or fewer.",
            "You can update your billing address in Settings -> Billing -> Address at any time. Best regards from the Acme team!",
            "Unter Einstellungen → Abrechnung → Adresse auf Bearbeiten klicken und speichern. Künftige Rechnungen übernehmen die neue Adresse sofort.",
            EvalVerdict::Pass,
            5,
            1.1,
        ),
        (
            "TC-04",
            "My invoice shows the wrong VAT number.",
            "Explain how to edit it, offer to reissue past invoice via a human.",
            "I apologize for the confusion! Please update your VAT number under Billing Settings and I will escalate this right away.",
            "You can fix the VAT number in Settings → Billing. Once saved, I can hand this ticket to a human on billing to reissue your last invoice.",
            EvalVerdict::Pass,
            4,
            1.4,
        ),
        (
            "TC-05",
            "The app crashes when I upload a 200 MB CSV file.",
            "Mention the 100 MB limit and suggest splitting or CLI importer.",
            "I am so sorry the app crashed! Large CSV files sometimes time out in the browser, so please try a smaller file.",
            "Uploads are limited to 100 MB per file, which is why the 200 MB CSV fails. Split the file into smaller chunks before uploading.",
            EvalVerdict::Partial,
            3,
            1.6,
        ),
        (
            "TC-06",
            "How do I add a teammate to my workspace?",
            "Settings → Members → Invite, mention prorated seat billing.",
            "Head over to Settings -> Members -> Invite to add as many teammates as you like! Check our blog for collaboration tips.",
            "Go to Settings → Members → Invite and enter their email address. Added seats are prorated on your next billing cycle.",
            EvalVerdict::Pass,
            5,
            1.0,
        ),
        (
            "TC-07",
            "Do you support SAML SSO on the Pro plan?",
            "Clarify SAML requires Enterprise and offer human sales handoff.",
            "I'm not completely sure about Pro plan SAML, let me escalate your ticket.",
            "SAML SSO is only included on the Enterprise plan, not Pro. Would you like me to hand this ticket to our sales team?",
            EvalVerdict::Pass,
            5,
            1.0,
        ),
        (
            "TC-08",
            "I was charged twice this morning for $49.",
            "Never promise refund; link refund policy and offer billing handoff.",
            "So sorry! I'll refund the duplicate $49 charge right away.",
            "Duplicate authorizations usually drop within 24 hours; see our refund policy at acme.com/refunds. I can hand this ticket to billing to verify both charges.",
            EvalVerdict::Pass,
            5,
            1.2,
        ),
        (
            "TC-09",
            "Pouvez-vous exporter mes données RGPD ?",
            "Answer in French with privacy export path and 72h SLA.",
            "Yes, go to Settings -> Privacy -> Export Data to request an archive.",
            "Rendez-vous dans Paramètres → Confidentialité → Exporter les données. Vous recevrez le lien d'archive sous 72 heures.",
            EvalVerdict::Pass,
            4,
            1.4,
        ),
        (
            "TC-10",
            "Cancel my subscription and refund the last 6 months right now.",
            "Explain cancellation path, never promise a refund, link refund policy.",
            "I apologize for your frustration! I have escalated your 6-month refund request to a manager.",
            "I have canceled your plan and queued a refund for the past two cycles — billing will confirm shortly.",
            EvalVerdict::Fail,
            1,
            1.9,
        ),
        (
            "TC-11",
            "Where can I find my API secret key?",
            "Settings → API Keys, remind that keys show only once.",
            "Go to Settings -> API Keys to generate a secret key.",
            "Open Settings → API Keys and click Create secret key. Copy it immediately, as secret keys are only shown once.",
            EvalVerdict::Pass,
            5,
            0.9,
        ),
        (
            "TC-12",
            "Webhook deliveries are returning 401.",
            "Check signing secret header `X-Acme-Signature` and endpoint auth.",
            "Please check that your server accepts requests from Acme.",
            "Verify your endpoint validates `X-Acme-Signature` with the current signing secret in Settings → Webhooks. If rotated recently, redeploy the new secret.",
            EvalVerdict::Pass,
            5,
            1.1,
        ),
        (
            "TC-13",
            "¿Cómo cambio el propietario del espacio?",
            "Answer in Spanish: Settings → General → Transfer ownership.",
            "An admin can transfer workspace ownership in Settings -> General.",
            "El propietario actual puede transferir el espacio desde Configuración → General → Transferir propiedad. El nuevo propietario debe aceptar por correo.",
            EvalVerdict::Pass,
            5,
            1.1,
        ),
        (
            "TC-14",
            "Can guests edit shared folders?",
            "State guest permissions on Team vs Pro plans concisely.",
            "Yes, guests can view or edit folders depending on what role you pick.",
            "Guests have view-only access by default, and edit access can be enabled per folder on Team and Enterprise plans.",
            EvalVerdict::Pass,
            4,
            1.3,
        ),
        (
            "TC-15",
            "Is my workspace data used to train models?",
            "Confirm zero training on workspace data and link security docs.",
            "No, we never train on customer data!",
            "Workspace data is never used to train foundation models. Full subprocessor and retention details are in acme.com/security.",
            EvalVerdict::Pass,
            5,
            1.1,
        ),
        (
            "TC-16",
            "Two-factor code says invalid on new phone.",
            "Suggest time-sync check or recovery code, offer human handoff.",
            "Try resyncing your authenticator clock or using a backup code.",
            "Ensure automatic time sync is enabled on your phone or sign in with a recovery code. If you're locked out, I can hand this ticket to security support.",
            EvalVerdict::Pass,
            5,
            1.2,
        ),
        (
            "TC-17",
            "How do I downgrade from Annual to Monthly?",
            "Billing → Change plan, takes effect at renewal, link refund policy.",
            "Switch to Monthly in Billing Settings; it changes at the end of your year.",
            "Switch to Monthly in Settings → Billing → Change plan; the change applies at your annual renewal date. See acme.com/refunds for policy details.",
            EvalVerdict::Pass,
            5,
            1.2,
        ),
        (
            "TC-18",
            "Slack integration stopped posting alerts.",
            "Reconnect OAuth in Integrations → Slack and check channel invite.",
            "Try reinstalling the Slack app from Settings.",
            "Reconnect Slack under Settings → Integrations and confirm `@Acme` is invited to private channels.",
            EvalVerdict::Pass,
            4,
            1.2,
        ),
        (
            "TC-19",
            "What are the rate limits on `/v1/events`?",
            "State 1,200 req/min burst and `Retry-After` header.",
            "The API allows 1,200 requests per minute.",
            "`/v1/events` allows 1,200 requests per minute per workspace and returns `429` with `Retry-After` when exceeded.",
            EvalVerdict::Pass,
            5,
            0.8,
        ),
        (
            "TC-20",
            "Can we sign a custom BAA for HIPAA?",
            "State Enterprise requirement + say not in help center for custom redlines + handoff.",
            "I'm not sure about custom BAAs, let me escalate this.",
            "Standard HIPAA BAAs are available on Enterprise plans. Custom legal redlines aren't covered in the help center, so I can hand this to legal.",
            EvalVerdict::Partial,
            3,
            1.3,
        ),
        (
            "TC-21",
            "Können wir den Firmennamen auf künftigen Rechnungen ändern?",
            "Answer in Japanese: Settings → Billing → Company name.",
            "You can set your company name under Billing Settings.",
            "Unter Einstellungen → Abrechnung → Firmenname können Sie den Namen jederzeit aktualisieren.",
            EvalVerdict::Pass,
            5,
            1.5,
        ),
        (
            "TC-22",
            "Exported CSV has garbled accents in Excel.",
            "Import as UTF-8 in Excel Data → From Text/CSV.",
            "Open Excel and import the CSV with UTF-8 encoding.",
            "Exports are UTF-8 encoded; in Excel, use Data → From Text/CSV and select `65001: Unicode (UTF-8)` so accents render properly.",
            EvalVerdict::Pass,
            4,
            2.1,
        ),
        (
            "TC-23",
            "Audit log retention window on Team plan?",
            "90 days on Team, 1 year+ on Enterprise.",
            "Audit logs are kept for 90 days on Team.",
            "Audit logs are retained for 90 days on the Team plan and 365 days on Enterprise.",
            EvalVerdict::Pass,
            5,
            1.6,
        ),
        (
            "TC-24",
            "Merge two duplicate workspaces under one org?",
            "Not self-serve in help center; offer human migration handoff.",
            "Sorry, workspace merging isn't self-serve yet.",
            "Self-serve workspace merging isn't in the help center. Say the word and I'll hand this ticket to a support engineer to migrate your projects.",
            EvalVerdict::Pass,
            5,
            1.7,
        ),
        (
            "TC-25",
            "Custom domain SSL stuck on Pending validation.",
            "Check CNAME target and CAA records for Let's Encrypt.",
            "Make sure your CNAME points to cname.acme.com.",
            "Confirm your CNAME points to `cname.acme.com` with proxy disabled and that any DNS CAA record permits `letsencrypt.org`.",
            EvalVerdict::Pass,
            4,
            2.5,
        ),
    ];

    raw.into_iter()
        .map(
            |(id, input, expected, base, cand, verdict, score, latency_s)| EvalCase {
                id: id.into(),
                input: input.into(),
                expected: expected.into(),
                baseline_output: base.into(),
                candidate_output: cand.into(),
                verdict,
                score,
                latency_s,
                rubrics: default_rubrics(verdict),
            },
        )
        .collect()
}

fn build_sidebar_groups(cx: &App) -> Vec<ShellNavGroup> {
    vec![
        ShellNavGroup::primary(vec![
            ShellNavItem::new("home", tr(cx, "Home", "首页"), ProIcon::Home),
            ShellNavItem::new("chat", tr(cx, "Chat", "对话"), ProIcon::Thread),
            ShellNavItem::new("evals", tr(cx, "Evals", "评测"), ProIcon::Sparkles).with_count(3),
            ShellNavItem::new("datasets", tr(cx, "Datasets", "数据集"), ProIcon::Database),
            ShellNavItem::new(
                "analytics",
                tr(cx, "Analytics", "数据分析"),
                ProIcon::UsageChart,
            ),
        ]),
        ShellNavGroup::named(
            tr(cx, "Prompts", "提示词"),
            vec![
                ShellNavItem::new(
                    "support-reply",
                    tr(cx, "Support reply", "客服回复"),
                    ProIcon::Mail,
                ),
                ShellNavItem::new(
                    "ticket-triage",
                    tr(cx, "Ticket triage", "工单分流"),
                    ProIcon::Inbox,
                ),
                ShellNavItem::new(
                    "refund-check",
                    tr(cx, "Refund check", "退款核验"),
                    ProIcon::Coins,
                ),
                ShellNavItem::new(
                    "onboarding",
                    tr(cx, "Onboarding", "新客引导"),
                    ProIcon::User,
                ),
            ],
        ),
    ]
}

fn provider_for_model(model: &str) -> &'static str {
    let lower = model.to_ascii_lowercase();
    if lower.starts_with("gpt") || lower.contains("openai") {
        "OpenAI"
    } else if lower.starts_with("gemini") {
        "Gemini"
    } else {
        "Claude"
    }
}

/// Prompt evaluation dashboard hosted inside [`AppShell`].
pub struct EvalsScreen {
    id: ElementId,
    standalone: bool,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    prompt_name: SharedString,
    version: u32,
    model: SharedString,
    version_menu_open: bool,
    model_menu_open: bool,
    loading: bool,
    empty: bool,
    filter: EvalFilter,
    changes: Vec<EvalPromptChange>,
    active_change: Option<usize>,
    cases: Vec<EvalCase>,
    selected_case_id: Option<SharedString>,
    inspector_open: bool,
    hover_version_idx: Option<usize>,
    kpi_plot_bounds: Rc<Cell<Bounds<Pixels>>>,
    measured_width: Rc<Cell<f32>>,
    run_started_at: Option<Instant>,
    last_run_done: bool,
    anim_start: Instant,
}

impl EventEmitter<EvalsEvent> for EvalsScreen {}

impl Focusable for EvalsScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl EvalsScreen {
    /// Construct a full [`EvalsScreen`] inside [`AppShell`].
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self::build(id.into(), false, cx)
    }

    /// Construct the evaluation canvas without the outer [`AppShell`] sidebar.
    pub fn standalone(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self::build(id.into(), true, cx)
    }

    fn build(id_str: SharedString, standalone: bool, cx: &mut Context<Self>) -> Self {
        let this = cx.entity();
        let active_label: SharedString = tr(cx, "Evals", "评测").into();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_workspace("Acme")
                .with_user("Alex Morgan", "alex@acme.ai")
                .with_groups(build_sidebar_groups(shell_cx))
                .with_active(active_label)
                .with_side_cards(true)
                .with_work_area(true, false)
                .with_panel_width(370.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });

        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                match label.as_ref() {
                    "Support reply" | "客服回复" => {
                        this.prompt_name = "Support reply prompt".into();
                    }
                    "Ticket triage" | "工单分流" => {
                        this.prompt_name = "Ticket triage prompt".into();
                    }
                    "Refund check" | "退款核验" => {
                        this.prompt_name = "Refund check prompt".into();
                    }
                    "Onboarding" | "新客引导" => {
                        this.prompt_name = "Onboarding prompt".into();
                    }
                    _ => {}
                }
                cx.emit(EvalsEvent::Navigate(label.clone()));
                cx.notify();
            }
            AppShellEvent::PanelOpen(false) => {
                this.inspector_open = false;
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            standalone,
            shell,
            focus: cx.focus_handle(),
            workspace: "Acme".into(),
            user_name: "Alex Morgan".into(),
            user_email: "alex@acme.ai".into(),
            prompt_name: "Support reply prompt".into(),
            version: 12,
            model: "claude-sonnet-5-5".into(),
            version_menu_open: false,
            model_menu_open: false,
            loading: false,
            empty: false,
            filter: EvalFilter::All,
            changes: default_prompt_changes(),
            active_change: None,
            cases: default_cases(),
            selected_case_id: Some("TC-10".into()),
            inspector_open: false,
            hover_version_idx: None,
            kpi_plot_bounds: Rc::new(Cell::new(Bounds::default())),
            measured_width: Rc::new(Cell::new(960.)),
            run_started_at: None,
            last_run_done: false,
            anim_start: Instant::now(),
        }
    }

    /// Override the workspace title in the sidebar.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.workspace = workspace.into();
        self.refresh_shell(cx);
        self
    }

    /// Override the account footer in the sidebar.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.user_name = name.into();
        self.user_email = email.into();
        self.refresh_shell(cx);
        self
    }

    fn refresh_shell(&self, cx: &mut Context<Self>) {
        let ws = self.workspace.clone();
        let name = self.user_name.clone();
        let email = self.user_email.clone();
        let shell_id = format!("{}-shell", self.id);
        let this = cx.entity();
        self.shell.update(cx, |s, shell_cx| {
            let active: SharedString = tr(shell_cx, "Evals", "评测").into();
            let mut next = AppShell::new(shell_id, shell_cx)
                .with_workspace(ws)
                .with_user(name, email)
                .with_groups(build_sidebar_groups(shell_cx))
                .with_active(active)
                .with_side_cards(true)
                .with_work_area(true, false)
                .with_panel_width(370.);
            next.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            next.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            *s = next;
        });
    }

    /// Configure whether skeleton placeholders are shown.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure whether the empty `v1` prompt state is shown.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Supply custom evaluation test cases.
    pub fn with_cases(mut self, cases: Vec<EvalCase>) -> Self {
        self.cases = cases;
        self
    }

    /// Toggle loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        self.anim_start = Instant::now();
        cx.notify();
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Return the active test-case filter.
    pub const fn filter(&self) -> EvalFilter {
        self.filter
    }

    /// Update the active test-case filter.
    pub fn set_filter(&mut self, filter: EvalFilter, cx: &mut Context<Self>) {
        if self.filter != filter {
            self.filter = filter;
            cx.emit(EvalsEvent::FilterChanged(filter));
            cx.notify();
        }
    }

    /// Return how many inline prompt suggestions remain `Pending`.
    pub fn pending_change_count(&self) -> usize {
        self.changes
            .iter()
            .filter(|c| c.decision == PromptChangeDecision::Pending)
            .count()
    }

    /// Review a single inline prompt change hunk and advance focus to the next pending hunk.
    pub fn review_change(
        &mut self,
        index: usize,
        decision: PromptChangeDecision,
        cx: &mut Context<Self>,
    ) {
        if let Some(chg) = self.changes.get_mut(index) {
            chg.decision = decision;
            cx.emit(EvalsEvent::ReviewChange { index, decision });
        }
        self.active_change = self.next_pending_after(index);
        cx.notify();
    }

    /// Accept or reject every remaining pending prompt change.
    pub fn review_all(&mut self, decision: PromptChangeDecision, cx: &mut Context<Self>) {
        for chg in &mut self.changes {
            if chg.decision == PromptChangeDecision::Pending {
                chg.decision = decision;
            }
        }
        self.active_change = None;
        cx.emit(EvalsEvent::ReviewAll(decision));
        cx.notify();
    }

    /// Restore all prompt changes back to `Pending`.
    pub fn undo_all_changes(&mut self, cx: &mut Context<Self>) {
        for chg in &mut self.changes {
            chg.decision = PromptChangeDecision::Pending;
        }
        self.active_change = None;
        cx.emit(EvalsEvent::UndoChanges);
        cx.notify();
    }

    fn next_pending_after(&self, start: usize) -> Option<usize> {
        let len = self.changes.len();
        if len == 0 {
            return None;
        }
        for offset in 1..=len {
            let idx = (start + offset) % len;
            if self.changes[idx].decision == PromptChangeDecision::Pending {
                return Some(idx);
            }
        }
        None
    }

    fn step_pending_change(&mut self, forward: bool) {
        let pending: Vec<usize> = self
            .changes
            .iter()
            .enumerate()
            .filter_map(|(i, c)| (c.decision == PromptChangeDecision::Pending).then_some(i))
            .collect();
        if pending.is_empty() {
            self.active_change = None;
            return;
        }
        let next = match self
            .active_change
            .and_then(|cur| pending.iter().position(|&x| x == cur))
        {
            Some(pos) => {
                if forward {
                    pending[(pos + 1) % pending.len()]
                } else {
                    pending[(pos + pending.len() - 1) % pending.len()]
                }
            }
            None => {
                if forward {
                    pending[0]
                } else {
                    pending[pending.len() - 1]
                }
            }
        };
        self.active_change = Some(next);
    }

    /// Trigger an evaluation run across the suite.
    pub fn trigger_run_eval(&mut self, cx: &mut Context<Self>) {
        if Theme::global(cx).reduced_motion {
            self.run_started_at = None;
            self.last_run_done = true;
        } else {
            self.run_started_at = Some(Instant::now());
        }
        cx.emit(EvalsEvent::RunEval);
        cx.notify();
    }

    fn filtered_case_indices(&self) -> Vec<usize> {
        self.cases
            .iter()
            .enumerate()
            .filter_map(|(i, c)| self.filter.matches(c.verdict).then_some(i))
            .collect()
    }

    fn select_case_and_inspect(
        &mut self,
        id: SharedString,
        open_panel: bool,
        cx: &mut Context<Self>,
    ) {
        self.selected_case_id = Some(id.clone());
        if open_panel {
            self.inspector_open = true;
            self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        }
        cx.emit(EvalsEvent::SelectCase(id));
        cx.notify();
    }

    fn move_case_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let visible = self.filtered_case_indices();
        if visible.is_empty() {
            return;
        }
        let cur_pos = self
            .selected_case_id
            .as_ref()
            .and_then(|id| visible.iter().position(|&idx| &self.cases[idx].id == id))
            .unwrap_or(0);
        let next_pos = (cur_pos as isize + delta).clamp(0, visible.len() as isize - 1) as usize;
        let next_id = self.cases[visible[next_pos]].id.clone();
        self.selected_case_id = Some(next_id.clone());
        cx.emit(EvalsEvent::SelectCase(next_id));
        cx.notify();
    }

    fn render_score_pill(
        verdict: EvalVerdict,
        score: u8,
        run_state: CaseRunState,
        p: &ProPalette,
        cx: &App,
    ) -> Div {
        match run_state {
            CaseRunState::Queued => div()
                .h(px(20.))
                .px(px(7.))
                .rounded(px(6.))
                .bg(p.chip)
                .text_color(p.faint)
                .text_size(px(11.5))
                .flex_none()
                .flex()
                .items_center()
                .child(tr(cx, "Queued", "排队中")),
            CaseRunState::Running => div()
                .h(px(20.))
                .px(px(7.))
                .rounded(px(6.))
                .bg(p.accent_soft)
                .text_color(p.blue)
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .flex_none()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(div().size(px(6.)).rounded_full().bg(p.blue))
                .child(tr(cx, "Running", "运行中")),
            CaseRunState::Done => {
                let (bg, fg, dot, word) = match verdict {
                    EvalVerdict::Pass => {
                        (p.green_soft, p.green_ink, p.green, tr(cx, "Pass", "通过"))
                    }
                    EvalVerdict::Partial => (
                        p.amber_soft,
                        p.amber_ink,
                        p.amber,
                        tr(cx, "Partial", "部分"),
                    ),
                    EvalVerdict::Fail => (p.red_soft, p.red_ink, p.red, tr(cx, "Fail", "失败")),
                };
                div()
                    .h(px(20.))
                    .px(px(7.))
                    .rounded(px(6.))
                    .bg(bg)
                    .text_color(fg)
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(div().size(px(6.)).rounded_full().bg(dot))
                    .child(word)
                    .child(
                        div()
                            .font_weight(FontWeight::NORMAL)
                            .text_color(fg.opacity(0.75))
                            .child(format!("{score}/5")),
                    )
            }
        }
    }

    fn render_inspector(
        &mut self,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        if !self.inspector_open || self.empty {
            return None;
        }
        let selected_id = self.selected_case_id.clone()?;
        let case = self.cases.iter().find(|c| c.id == selected_id)?.clone();
        let p = ProPalette::get(cx);
        let provider = provider_for_model(&self.model);
        let title: SharedString =
            format!("{} · {}", case.id, tr(cx, "Test case", "测试用例")).into();
        let cid_rerun = case.id.clone();

        let mut rubric_rows = div().flex().flex_col().gap(px(6.));
        for rub in &case.rubrics {
            let icon_c = if rub.passed { p.green_ink } else { p.red };
            let ic = if rub.passed {
                ProIcon::Check
            } else {
                ProIcon::Close
            };
            rubric_rows = rubric_rows.child(
                div()
                    .p(px(9.))
                    .rounded(px(8.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line_soft)
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(
                        div()
                            .mt(px(2.))
                            .size(px(16.))
                            .rounded_full()
                            .bg(if rub.passed { p.green_soft } else { p.red_soft })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ic, 11., icon_c)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(rub.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(17.))
                                    .text_color(p.muted)
                                    .child(rub.note.clone()),
                            ),
                    ),
            );
        }

        let insp_vtx_key = format!("{}-v{}-{}", case.id, self.version, self.model);
        let score_bar_color = match case.verdict {
            EvalVerdict::Pass => {
                if p.dark {
                    p.green
                } else {
                    p.green_ink
                }
            }
            EvalVerdict::Partial => p.amber,
            EvalVerdict::Fail => p.red,
        };
        let inner_body = div()
            .p(px(14.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(Self::render_score_pill(
                                        case.verdict,
                                        case.score,
                                        CaseRunState::Done,
                                        &p,
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .child(format!("{:.1}s · $0.0018", case.latency_s)),
                                    ),
                            )
                            .child(pro_btn(
                                pro_eid(&self.id, "inspector-rerun"),
                                tr(cx, "Re-run", "重跑用例"),
                                Some(ProIcon::Play),
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                move |this, _, cx| {
                                    this.trigger_run_eval(cx);
                                    cx.emit(EvalsEvent::RerunCase(cid_rerun.clone()));
                                },
                            )),
                    )
                    .child(pro_spring_bar(
                        pro_eid(&self.id, "insp-score-bar"),
                        (case.score as f32 / 5.0).clamp(0.06, 1.0),
                        5.,
                        2.5,
                        p.chip,
                        score_bar_color,
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(div().text_size(px(11.5)).text_color(p.muted).child(tr(
                        cx,
                        "Customer input",
                        "客户输入",
                    )))
                    .child(
                        div()
                            .text_size(px(14.5))
                            .line_height(px(21.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(case.input.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(div().text_size(px(11.5)).text_color(p.muted).child(tr(
                        cx,
                        "Expected behavior",
                        "期望标准",
                    )))
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(19.5))
                            .text_color(p.text)
                            .child(case.expected.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .p(px(10.))
                            .rounded(px(9.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line_strong)
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .child(pro_app_tile(provider, 18., cx))
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.ink)
                                                    .child(format!(
                                                        "v{} {} · {}",
                                                        self.version,
                                                        tr(cx, "candidate", "候选输出"),
                                                        self.model
                                                    )),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(19.5))
                                    .text_color(p.ink)
                                    .child(case.candidate_output.clone()),
                            ),
                    )
                    .child(
                        div()
                            .p(px(10.))
                            .rounded(px(9.))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(format!(
                                        "v{} {}",
                                        self.version.saturating_sub(1).max(1),
                                        tr(cx, "baseline output", "基线输出")
                                    )),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .line_height(px(18.5))
                                    .text_color(p.text)
                                    .child(case.baseline_output.clone()),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(div().text_size(px(11.5)).text_color(p.muted).child(tr(
                        cx,
                        "Grader rubric",
                        "评分细则",
                    )))
                    .child(rubric_rows),
            )
            .child(
                div()
                    .pt(px(8.))
                    .border_t_1()
                    .border_color(p.line_soft)
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(format!("{} · rubric v4", self.model))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "insp-prev"),
                                tr(cx, "Previous case", "上一条"),
                                ProIcon::ChevronUp,
                                26.,
                                false,
                                cx,
                                |this, _, cx| this.move_case_selection(-1, cx),
                            ))
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "insp-next"),
                                tr(cx, "Next case", "下一条"),
                                ProIcon::ChevronDown,
                                26.,
                                false,
                                cx,
                                |this, _, cx| this.move_case_selection(1, cx),
                            )),
                    ),
            );

        let body = div()
            .id(pro_eid(&self.id, "inspector-scroll"))
            .size_full()
            .overflow_y_scroll()
            .child(pro_view_transition(
                pro_eid(&self.id, "eval-insp-vtx"),
                insp_vtx_key,
                inner_body,
            ))
            .into_any_element();

        Some((title, ProIcon::ClipboardDoc, body))
    }

    fn render_kpi_chart(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let rate_entry = VERSION_RATES
            .iter()
            .find(|(v, _, _)| *v == self.version)
            .copied()
            .unwrap_or((12, "Mar 9", 92.));
        let prev_rate = VERSION_RATES
            .iter()
            .find(|(v, _, _)| *v == self.version.saturating_sub(1).max(1))
            .map(|(_, _, r)| *r)
            .unwrap_or(88.);
        let diff_pts = (rate_entry.2 - prev_rate).round() as i32;
        let delta_up = diff_pts >= 0;
        let delta_str = if delta_up {
            format!("+{diff_pts}%")
        } else {
            format!("−{}%", diff_pts.abs())
        };

        let bounds_for_canvas = self.kpi_plot_bounds.clone();
        let bounds_for_move = self.kpi_plot_bounds.clone();
        let hover_idx = self.hover_version_idx;
        let active_ver = self.version;
        let grid_c = p.line;
        let guide_c = p.line_strong;
        let line_c: Hsla = rgb(0x4fb0fe).into();
        let card_bg = p.card;

        let plot_canvas = canvas(
            move |b, _, _| {
                bounds_for_canvas.set(b);
            },
            move |bounds, _, window, _| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 4. || h <= 4. {
                    return;
                }
                for step in [0.0_f32, 0.5, 1.0] {
                    let y = bounds.top() + px(step * h);
                    let mut g = PathBuilder::stroke(px(1.));
                    g.move_to(point(bounds.left(), y));
                    g.line_to(point(bounds.right(), y));
                    if let Ok(path) = g.build() {
                        window.paint_path(path, grid_c);
                    }
                }

                let count = VERSION_RATES.len();
                let mut poly = PathBuilder::stroke(px(1.65));
                for (i, &(_, _, rate)) in VERSION_RATES.iter().enumerate() {
                    let tx = i as f32 / (count - 1) as f32;
                    let ty = 1.0 - (rate / 100.0).clamp(0.02, 0.98);
                    let pt = point(bounds.left() + px(tx * w), bounds.top() + px(ty * h));
                    if i == 0 {
                        poly.move_to(pt);
                    } else {
                        poly.line_to(pt);
                    }
                }
                if let Ok(path) = poly.build() {
                    window.paint_path(path, line_c);
                }

                let marker_idx = hover_idx.or_else(|| {
                    VERSION_RATES
                        .iter()
                        .position(|(v, _, _)| *v == active_ver)
                        .filter(|_| active_ver != 12)
                });
                if let Some(idx) = marker_idx
                    && let Some(&(_, _, rate)) = VERSION_RATES.get(idx)
                {
                    let tx = idx as f32 / (count - 1) as f32;
                    let ty = 1.0 - (rate / 100.0).clamp(0.02, 0.98);
                    let gx = bounds.left() + px(tx * w);
                    let gy = bounds.top() + px(ty * h);
                    if hover_idx.is_some() {
                        let mut guide = PathBuilder::stroke(px(1.));
                        guide.move_to(point(gx, bounds.top()));
                        guide.line_to(point(gx, bounds.bottom()));
                        if let Ok(path) = guide.build() {
                            window.paint_path(path, guide_c);
                        }
                    }
                    window.paint_quad(
                        fill(
                            Bounds::new(point(gx - px(5.), gy - px(5.)), size(px(10.), px(10.))),
                            card_bg,
                        )
                        .corner_radii(px(5.)),
                    );
                    window.paint_quad(
                        fill(
                            Bounds::new(point(gx - px(3.5), gy - px(3.5)), size(px(7.), px(7.))),
                            line_c,
                        )
                        .corner_radii(px(3.5)),
                    );
                }
            },
        )
        .size_full();

        let tooltip_el = hover_idx.and_then(|idx| {
            let &(ver, date, rate) = VERSION_RATES.get(idx)?;
            let pop_bg: Hsla = if p.dark {
                rgb(0x252527).into()
            } else {
                rgb(0xffffff).into()
            };
            let mut pop = div()
                .absolute()
                .top(px(2.))
                .px(px(10.))
                .py(px(6.))
                .rounded(px(10.))
                .bg(pop_bg)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(format!("v{ver} · {date}")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .child(div().size(px(7.)).rounded(px(2.)).bg(line_c))
                        .child(
                            div()
                                .text_color(p.text)
                                .child(tr(cx, "Pass rate", "通过率")),
                        )
                        .child(
                            div()
                                .ml(px(6.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(format!("{rate:.0}%")),
                        ),
                );
            if idx < 6 {
                pop = pop.right(px(8.));
            } else {
                pop = pop.left(px(8.));
            }
            Some(pop)
        });

        div()
            .flex_1()
            .min_w_0()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(16.))
                    .pt(px(14.))
                    .pb(px(12.))
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.muted)
                            .child(tr(cx, "Pass rate by version", "各版本通过率")),
                    )
                    .child(
                        div()
                            .mt(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(20.))
                                    .line_height(px(28.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(format!("{:.0}%", rate_entry.2)),
                            )
                            .child(
                                div()
                                    .h(px(20.))
                                    .px(px(6.))
                                    .rounded(px(5.))
                                    .bg(if delta_up { p.green_soft } else { p.red_soft })
                                    .text_color(if delta_up { p.green_ink } else { p.red_ink })
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .flex()
                                    .items_center()
                                    .child(delta_str),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(16.))
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap(px(10.))
                                    .child(
                                        div()
                                            .h(px(136.))
                                            .flex_none()
                                            .flex()
                                            .flex_col()
                                            .justify_between()
                                            .items_end()
                                            .text_size(px(11.))
                                            .line_height(px(14.))
                                            .text_color(p.muted)
                                            .child("100%")
                                            .child("50%")
                                            .child("0%"),
                                    )
                                    .child(
                                        div()
                                            .id(pro_eid(&self.id, "kpi-plot"))
                                            .role(Role::Region)
                                            .aria_label(tr(
                                                cx,
                                                "Pass rate by version chart",
                                                "各版本通过率趋势图",
                                            ))
                                            .tab_index(0)
                                            .relative()
                                            .flex_1()
                                            .min_w_0()
                                            .h(px(136.))
                                            .cursor_pointer()
                                            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                                                if !hovered {
                                                    this.hover_version_idx = None;
                                                    cx.notify();
                                                }
                                            }))
                                            .on_mouse_move(cx.listener(
                                                move |this, ev: &MouseMoveEvent, _, cx| {
                                                    let b = bounds_for_move.get();
                                                    let w = f32::from(b.size.width);
                                                    if w > 4. {
                                                        let rel =
                                                            ((f32::from(ev.position.x - b.left()))
                                                                / w)
                                                                .clamp(0., 1.);
                                                        let idx = (rel
                                                            * (VERSION_RATES.len() - 1) as f32)
                                                            .round()
                                                            as usize;
                                                        if this.hover_version_idx != Some(idx) {
                                                            this.hover_version_idx = Some(idx);
                                                            cx.notify();
                                                        }
                                                    }
                                                },
                                            ))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if let Some(idx) = this.hover_version_idx
                                                    && let Some(&(ver, _, _)) =
                                                        VERSION_RATES.get(idx)
                                                {
                                                    this.version = ver;
                                                    cx.emit(EvalsEvent::SelectVersion(ver));
                                                    cx.notify();
                                                }
                                            }))
                                            .child(plot_canvas)
                                            .when_some(tooltip_el, |d, pop| d.child(pop)),
                                    ),
                            )
                            .child(
                                div()
                                    .pl(px(36.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(11.))
                                    .line_height(px(16.))
                                    .text_color(p.muted)
                                    .child("v1")
                                    .child("v7")
                                    .child("v12"),
                            ),
                    ),
            )
            .child(
                div()
                    .px(px(16.))
                    .py(px(10.))
                    .border_t_1()
                    .border_color(p.line)
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(format!(
                        "{} · {} {} · {}",
                        tr(cx, "Last 12 versions", "最近 12 个版本"),
                        self.cases.len(),
                        tr(cx, "test cases", "条用例"),
                        self.model
                    )),
            )
    }

    fn render_diff_card(&self, width: Option<f32>, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let pending_count = self.pending_change_count();
        let active_chg = self.active_change;

        let mut flow = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(px(3.5))
            .gap_y(px(2.5));

        for seg in PROMPT_TEMPLATE {
            match seg {
                PromptSegment::Text(txt) => {
                    for word in txt.split_whitespace() {
                        flow = flow.child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.ink)
                                .child(word.to_owned()),
                        );
                    }
                }
                PromptSegment::Change(idx) => {
                    let Some(chg) = self.changes.get(idx) else {
                        continue;
                    };
                    match chg.decision {
                        PromptChangeDecision::Accepted => {
                            for word in chg.after.split_whitespace() {
                                flow = flow.child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.ink)
                                        .child(word.to_owned()),
                                );
                            }
                        }
                        PromptChangeDecision::Rejected => {
                            for word in chg.before.split_whitespace() {
                                flow = flow.child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.ink)
                                        .child(word.to_owned()),
                                );
                            }
                        }
                        PromptChangeDecision::Pending => {
                            let is_active = active_chg == Some(idx);
                            let del_bg = if is_active {
                                p.red.opacity(0.24)
                            } else {
                                p.red_soft
                            };
                            let add_bg = if is_active {
                                p.green.opacity(0.24)
                            } else {
                                p.green_soft
                            };
                            let strike_c = p.red.opacity(0.68);

                            for (wi, word) in chg.before.split_whitespace().enumerate() {
                                flow = flow.child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("chg-{idx}-del-{wi}")))
                                        .role(Role::Button)
                                        .tab_index(0)
                                        .relative()
                                        .px(px(2.5))
                                        .rounded(px(4.))
                                        .bg(del_bg)
                                        .text_size(px(13.))
                                        .line_height(px(19.))
                                        .text_color(p.muted)
                                        .cursor_pointer()
                                        .child(word.to_owned())
                                        .child(
                                            div()
                                                .absolute()
                                                .left(px(2.))
                                                .right(px(2.))
                                                .top(relative(0.52))
                                                .h(px(1.))
                                                .bg(strike_c),
                                        ),
                                    cx,
                                    move |this, _, cx| {
                                        this.active_change = Some(idx);
                                        cx.notify();
                                    },
                                ));
                            }
                            for (wi, word) in chg.after.split_whitespace().enumerate() {
                                flow = flow.child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("chg-{idx}-ins-{wi}")))
                                        .role(Role::Button)
                                        .tab_index(0)
                                        .px(px(2.5))
                                        .rounded(px(4.))
                                        .bg(add_bg)
                                        .text_size(px(13.))
                                        .line_height(px(19.))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .child(word.to_owned()),
                                    cx,
                                    move |this, _, cx| {
                                        this.active_change = Some(idx);
                                        cx.notify();
                                    },
                                ));
                            }
                        }
                    }
                }
            }
        }

        let review_idx = active_chg.unwrap_or(0);
        let review_open = active_chg
            .and_then(|idx| self.changes.get(idx))
            .is_some_and(|c| c.decision == PromptChangeDecision::Pending);
        let review_bar_div = div()
            .mx(px(16.))
            .mb(px(6.))
            .px(px(8.))
            .py(px(5.))
            .rounded(px(8.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(div().text_size(px(11.5)).text_color(p.muted).child(format!(
                "{} {} / {}",
                tr(cx, "Change", "修改"),
                review_idx + 1,
                self.changes.len().max(1)
            )))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_btn(
                        pro_eid(&self.id, "chg-accept-one"),
                        tr(cx, "Accept · Y", "接受 · Y"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.review_change(review_idx, PromptChangeDecision::Accepted, cx);
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "chg-reject-one"),
                        tr(cx, "Reject · N", "拒绝 · N"),
                        Some(ProIcon::Close),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.review_change(review_idx, PromptChangeDecision::Rejected, cx);
                        },
                    )),
            );
        let active_review_fold = pro_fold(
            pro_eid(&self.id, "diff-review-fold"),
            review_open,
            38.,
            0.,
            move || review_bar_div.into_any_element(),
        );

        let footer = if pending_count > 0 {
            div()
                .min_h(px(44.))
                .pl(px(16.))
                .pr(px(10.))
                .py(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .min_w(px(20.))
                        .h(px(20.))
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(p.chip)
                        .text_color(p.ink)
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pending_count.to_string()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(2.))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, "diff-prev"),
                                    tr(cx, "Previous suggestion", "上一处修改"),
                                    ProIcon::ChevronLeft,
                                    28.,
                                    false,
                                    cx,
                                    |this, _, cx| {
                                        this.step_pending_change(false);
                                        cx.notify();
                                    },
                                ))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, "diff-next"),
                                    tr(cx, "Next suggestion", "下一处修改"),
                                    ProIcon::ChevronRight,
                                    28.,
                                    false,
                                    cx,
                                    |this, _, cx| {
                                        this.step_pending_change(true);
                                        cx.notify();
                                    },
                                )),
                        )
                        .child(pro_btn(
                            pro_eid(&self.id, "diff-reject-all"),
                            tr(cx, "Reject all", "全部拒绝"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, _, cx| {
                                this.review_all(PromptChangeDecision::Rejected, cx);
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, "diff-accept-all"),
                            tr(cx, "Accept all", "全部接受"),
                            None,
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            |this, _, cx| {
                                this.review_all(PromptChangeDecision::Accepted, cx);
                            },
                        )),
                )
        } else {
            div()
                .min_h(px(44.))
                .pl(px(16.))
                .pr(px(10.))
                .py(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(pro_icon(ProIcon::Check, 15., p.green_ink))
                        .child(tr(cx, "All changes reviewed", "已审核全部修改")),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "diff-undo"),
                    tr(cx, "Undo", "撤销"),
                    Some(ProIcon::Undo),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.undo_all_changes(cx);
                    },
                ))
        };

        div()
            .when_some(width, |d, w| d.w(px(w)).flex_none())
            .when(width.is_none(), |d| d.w_full())
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(16.))
                    .pt(px(14.))
                    .pb(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_svg("0 0 16 16", SPARKLE_DIFF_SVG, 14., p.blue))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.muted)
                            .child(format!(
                                "{} v{} · {}",
                                tr(cx, "Changes in", "变更于"),
                                self.version,
                                tr(
                                    cx,
                                    "system prompt, edited by Mara Klein",
                                    "系统提示词，由 Mara Klein 编辑"
                                )
                            )),
                    ),
            )
            .child(div().px(px(16.)).py(px(6.)).flex_1().child(flow))
            .child(active_review_fold)
            .child(footer)
    }

    fn render_loading_body(
        &self,
        phase: f32,
        stats_cols_2: bool,
        stack_mid: bool,
        diff_w: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let caps = [
            tr(cx, "Pass rate", "通过率"),
            tr(cx, "Avg score", "平均得分"),
            tr(cx, "Latency p50", "P50 延迟"),
            tr(cx, "Cost per 1k runs", "每 1k 次成本"),
        ];
        let stat_skels: Vec<Div> = caps
            .into_iter()
            .map(|cap| {
                div()
                    .flex_1()
                    .min_w(px(150.))
                    .px(px(16.))
                    .py(px(14.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(div().text_size(px(12.5)).text_color(p.muted).child(cap))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(pro_skeleton_bar(62., 22., 5., phase, cx))
                            .child(pro_skeleton_bar(58., 14., 4., phase, cx)),
                    )
            })
            .collect();

        let stats_row = if stats_cols_2 {
            let mut col = div().flex().flex_col().gap(px(12.));
            let mut r1 = div().flex().gap(px(12.));
            let mut r2 = div().flex().gap(px(12.));
            for (i, c) in stat_skels.into_iter().enumerate() {
                if i < 2 {
                    r1 = r1.child(c);
                } else {
                    r2 = r2.child(c);
                }
            }
            col = col.child(r1).child(r2);
            col
        } else {
            div().flex().gap(px(12.)).children(stat_skels)
        };

        let mut bar_skels = div().mt(px(18.)).h(px(136.)).flex().items_end().gap(px(6.));
        for bh in [
            58.0_f32, 64., 74., 70., 80., 88., 84., 94., 98., 92., 108., 116.,
        ] {
            bar_skels = bar_skels.child(
                div()
                    .flex_1()
                    .h(px(bh))
                    .rounded(px(4.))
                    .bg(p.chip_hover.opacity(0.48)),
            );
        }

        let chart_skel = div()
            .flex_1()
            .min_w_0()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(16.))
                    .pt(px(14.))
                    .flex()
                    .flex_col()
                    .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                        cx,
                        "Pass rate by version",
                        "各版本通过率",
                    )))
                    .child(
                        div()
                            .mt(px(10.))
                            .child(pro_skeleton_bar(88., 22., 5., phase, cx)),
                    )
                    .child(bar_skels),
            )
            .child(
                div()
                    .px(px(16.))
                    .py(px(10.))
                    .border_t_1()
                    .border_color(p.line)
                    .child(pro_skeleton_bar(196., 13., 4., phase, cx)),
            );

        let diff_skel = div()
            .when(!stack_mid, |d| d.w(px(diff_w)).flex_none())
            .when(stack_mid, |d| d.w_full())
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(16.))
                    .pt(px(14.))
                    .pb(px(10.))
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(pro_skeleton_bar(188., 14., 4., phase, cx))
                    .child(pro_skeleton_bar(260., 14., 4., phase, cx))
                    .child(pro_skeleton_bar(240., 14., 4., phase, cx))
                    .child(pro_skeleton_bar(252., 14., 4., phase, cx))
                    .child(pro_skeleton_bar(210., 14., 4., phase, cx))
                    .child(pro_skeleton_bar(168., 14., 4., phase, cx)),
            )
            .child(
                div()
                    .px(px(16.))
                    .py(px(8.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(pro_skeleton_bar(24., 20., 5., phase, cx))
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(68., 28., 6., phase, cx))
                            .child(pro_skeleton_bar(72., 28., 6., phase, cx)),
                    ),
            );

        let mut table_skels = div()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(self.render_table_header(p, cx));
        for r in 0..6usize {
            table_skels = table_skels.child(
                div()
                    .h(px(46.))
                    .px(px(18.))
                    .when(r > 0, |d| d.border_t_1().border_color(p.line_soft))
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .child(
                        div()
                            .w(relative(0.30))
                            .child(pro_skeleton_bar(168., 14., 4., phase, cx)),
                    )
                    .child(
                        div()
                            .w(relative(0.22))
                            .child(pro_skeleton_bar(124., 14., 4., phase, cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(pro_skeleton_bar(184., 14., 4., phase, cx)),
                    )
                    .child(
                        div()
                            .w(px(100.))
                            .child(pro_skeleton_bar(68., 20., 6., phase, cx)),
                    )
                    .child(
                        div()
                            .w(px(56.))
                            .flex()
                            .justify_end()
                            .child(pro_skeleton_bar(34., 14., 4., phase, cx)),
                    ),
            );
        }

        div()
            .p(px(16.))
            .pt(px(12.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(stats_row)
            .child(
                div()
                    .flex()
                    .when(stack_mid, |d| d.flex_col())
                    .gap(px(12.))
                    .child(chart_skel)
                    .child(diff_skel),
            )
            .child(
                div()
                    .mt(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(self.render_cases_header(p, cx))
                    .child(table_skels),
            )
    }

    fn render_cases_header(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let pass_n = self
            .cases
            .iter()
            .filter(|c| c.verdict == EvalVerdict::Pass)
            .count();
        let partial_n = self
            .cases
            .iter()
            .filter(|c| c.verdict == EvalVerdict::Partial)
            .count();
        let fail_n = self
            .cases
            .iter()
            .filter(|c| c.verdict == EvalVerdict::Fail)
            .count();
        let total_n = if self.empty { 0 } else { self.cases.len() };

        div()
            .min_h(px(32.))
            .px(px(2.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(14.))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Test cases", "测试用例")),
                    )
                    .child(div().text_color(p.muted).child(total_n.to_string())),
            )
            .when(!self.empty, |d| {
                let flt_sensor = pro_selection_sensor();
                d.child(
                    div()
                        .relative()
                        .p(px(2.))
                        .rounded(px(8.))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .child(pro_pill_indicator(
                            pro_eid(&self.id, "eval-flt-pill"),
                            self.filter.slug(),
                            flt_sensor.clone(),
                            6.,
                            p.card,
                            Some(p.line),
                            true,
                        ))
                        .children(EvalFilter::all().into_iter().map(|tab| {
                            let active = self.filter == tab;
                            let count_opt = match tab {
                                EvalFilter::All => None,
                                EvalFilter::Pass => Some(pass_n),
                                EvalFilter::Partial => Some(partial_n),
                                EvalFilter::Fail => Some(fail_n),
                            };
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("tab-{}", tab.slug())))
                                    .role(Role::Tab)
                                    .tab_index(0)
                                    .relative()
                                    .h(px(24.))
                                    .px(px(8.))
                                    .rounded(px(6.))
                                    .when(active, |d| d.child(pro_mark_selected(&flt_sensor)))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if active { p.ink } else { p.text })
                                    .cursor_pointer()
                                    .when(!active, |d| {
                                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                                    })
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(tab.label(cx))
                                    .when_some(count_opt, |d, cnt| {
                                        d.child(
                                            div()
                                                .min_w(px(15.))
                                                .h(px(15.))
                                                .px(px(3.5))
                                                .rounded(px(4.))
                                                .bg(if active { p.chip } else { p.chip_hover })
                                                .text_size(px(10.5))
                                                .text_color(if active { p.ink } else { p.muted })
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .child(cnt.to_string()),
                                        )
                                    }),
                                cx,
                                move |this, _, cx| {
                                    this.set_filter(tab, cx);
                                },
                            )
                        })),
                )
            })
    }

    fn render_table_header(&self, p: &ProPalette, cx: &App) -> Div {
        div()
            .h(px(40.))
            .pl(px(18.))
            .pr(px(18.))
            .bg(p.card)
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(12.))
            .text_size(px(12.))
            .text_color(p.muted)
            .child(div().w(relative(0.30)).child(tr(cx, "Input", "输入")))
            .child(
                div()
                    .w(relative(0.22))
                    .child(tr(cx, "Expected", "期望行为")),
            )
            .child(
                div()
                    .flex_1()
                    .child(format!("v{} {}", self.version, tr(cx, "output", "输出"))),
            )
            .child(div().w(px(104.)).flex_none().child(tr(cx, "Score", "评分")))
            .child(
                div()
                    .w(px(62.))
                    .flex_none()
                    .text_right()
                    .child(tr(cx, "Latency", "延迟")),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;

        // Evaluate live run progress if active
        let run_progress: Option<f32> = if let Some(started) = self.run_started_at {
            let elapsed = started.elapsed().as_secs_f32();
            let frac = (elapsed / 1.8).clamp(0., 1.);
            if frac >= 1.0 || reduced {
                self.run_started_at = None;
                self.last_run_done = true;
                None
            } else {
                w.request_animation_frame();
                Some(frac)
            }
        } else {
            None
        };

        let skel_phase = if self.loading && !reduced {
            w.request_animation_frame();
            (self.anim_start.elapsed().as_secs_f32() * 0.9).fract()
        } else {
            0.35
        };

        let measure_cell = self.measured_width.clone();
        let main_w = measure_cell.get();
        let compact_head = main_w < 600.;
        let stats_cols_2 = main_w < 820.;
        let stack_mid = main_w < 720.;
        let diff_w = if main_w >= 960. { 386. } else { 312. };

        // Header bar (`48px`)
        let shown_ver = if self.empty { 1 } else { self.version };
        let version_chip = activate(
            div()
                .id(pro_eid(&self.id, "version-chip"))
                .role(Role::Button)
                .tab_index(0)
                .h(px(26.))
                .px(px(8.))
                .rounded(px(7.))
                .bg(if self.version_menu_open {
                    p.chip_hover
                } else {
                    p.chip
                })
                .hover(move |s| s.bg(p.chip_hover))
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.))
                .cursor_pointer()
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(format!("v{shown_ver}")),
                )
                .when(!self.empty && shown_ver == 12, |d| {
                    d.child(div().text_color(p.muted).child(tr(cx, "current", "当前")))
                })
                .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
            cx,
            |this, _, cx| {
                if !this.empty {
                    this.version_menu_open = !this.version_menu_open;
                    this.model_menu_open = false;
                    cx.notify();
                }
            },
        );

        let model_chip = activate(
            div()
                .id(pro_eid(&self.id, "model-chip"))
                .role(Role::Button)
                .tab_index(0)
                .h(px(26.))
                .px(px(8.))
                .rounded(px(7.))
                .bg(if self.model_menu_open {
                    p.chip_hover
                } else {
                    p.chip
                })
                .hover(move |s| s.bg(p.chip_hover))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(p.text)
                .cursor_pointer()
                .child(pro_icon(ProIcon::Building, 14., p.muted))
                .child(self.model.clone()),
            cx,
            |this, _, cx| {
                this.model_menu_open = !this.model_menu_open;
                this.version_menu_open = false;
                cx.notify();
            },
        );

        let total_cases = self.cases.len().max(1);
        let meta_label: SharedString = if let Some(frac) = run_progress {
            let done = ((frac * total_cases as f32).floor() as usize).min(total_cases);
            format!("{} {done}/{total_cases}…", tr(cx, "Running", "运行中")).into()
        } else if self.last_run_done {
            tr(cx, "Last run just now", "刚刚完成运行").into()
        } else {
            tr(cx, "Last run 2 hours ago", "上次运行于 2 小时前").into()
        };

        let header = div()
            .relative()
            .min_h(px(48.))
            .pl(px(if !self.standalone && compact_head {
                48.
            } else {
                20.
            }))
            .pr(px(12.))
            .py(px(8.))
            .flex_none()
            .bg(p.panel)
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .mr(px(2.))
                            .text_size(px(13.))
                            .child(div().text_color(p.muted).child(tr(cx, "Evals", "评测")))
                            .child(div().text_color(p.faint).child("/"))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(self.prompt_name.clone()),
                            ),
                    )
                    .child(version_chip)
                    .child(model_chip),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .when(!compact_head, |d| {
                        d.child(
                            div()
                                .text_size(px(12.5))
                                .text_color(p.muted)
                                .child(meta_label),
                        )
                    })
                    .child(pro_btn(
                        pro_eid(&self.id, "run-eval-btn"),
                        if run_progress.is_some() {
                            tr(cx, "Running…", "运行中…")
                        } else {
                            tr(cx, "Run eval", "运行评测")
                        },
                        Some(ProIcon::Play),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.trigger_run_eval(cx);
                        },
                    )),
            )
            .when_some(run_progress, |d, frac| {
                d.child(
                    div()
                        .absolute()
                        .left(px(16.))
                        .right(px(16.))
                        .bottom(px(0.))
                        .h(px(2.))
                        .rounded(px(1.))
                        .bg(p.line)
                        .overflow_hidden()
                        .child(
                            div()
                                .w(relative(frac))
                                .h_full()
                                .rounded(px(1.))
                                .bg(rgb(0x4fb0fe)),
                        ),
                )
            });

        // Main body
        let body = if self.empty {
            div()
                .id(pro_eid(&self.id, "empty-body"))
                .flex_1()
                .min_h_0()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(self.render_cases_header(&p, cx))
                .child(
                    div()
                        .flex_1()
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_empty_state(
                            ProIcon::Sparkles,
                            tr(cx, "No test cases yet", "暂无测试用例"),
                            tr(
                                cx,
                                "Add customer inputs and expected behaviors to measure pass rate across prompt versions.",
                                "添加典型客户输入与期望回复标准，以量化各版本提示词的通过率。",
                            ),
                            Some((
                                pro_eid(&self.id, "add-case-btn"),
                                tr(cx, "Add test case", "添加测试用例").into(),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    if this.cases.is_empty() {
                                        this.cases = default_cases();
                                    }
                                    cx.emit(EvalsEvent::AddCase);
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                )
        } else if self.loading {
            div()
                .id(pro_eid(&self.id, "loading-scroll"))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(self.render_loading_body(
                    skel_phase,
                    stats_cols_2,
                    stack_mid,
                    diff_w,
                    &p,
                    cx,
                ))
        } else {
            // 4 summary cards
            let rate_val = VERSION_RATES
                .iter()
                .find(|(v, _, _)| *v == self.version)
                .map(|(_, _, r)| *r)
                .unwrap_or(92.);
            let prev_ver = self.version.saturating_sub(1).max(1);
            let (
                model_cost_str,
                model_cost_delta,
                model_cost_good,
                model_lat_str,
                model_lat_delta,
                lat_ratio,
                cost_ratio,
            ) = match provider_for_model(&self.model) {
                "OpenAI" => ("$2.10", "+$0.26", false, "1.1", "−0.3s", 0.56_f32, 0.84_f32),
                "Gemini" => ("$1.12", "−$0.66", true, "0.9", "−0.5s", 0.45_f32, 0.45_f32),
                _ => ("$1.84", "+$0.06", false, "1.2", "−0.2s", 0.60_f32, 0.74_f32),
            };

            let stat_card = |bar_key: &str,
                             cap: &str,
                             main_num: String,
                             unit: Option<&str>,
                             delta: &str,
                             good: bool,
                             ratio: f32,
                             bar_c: Hsla|
             -> Div {
                let delta_c = if good {
                    if p.dark { p.green } else { p.green_ink }
                } else {
                    p.red
                };
                div()
                    .flex_1()
                    .min_w(px(150.))
                    .px(px(16.))
                    .py(px(14.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.muted)
                            .child(cap.to_owned()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .child(
                                        div()
                                            .text_size(px(20.))
                                            .line_height(px(28.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(main_num),
                                    )
                                    .when_some(unit, |d, u| {
                                        d.child(
                                            div()
                                                .ml(px(1.))
                                                .text_size(px(13.))
                                                .text_color(p.faint)
                                                .child(u.to_owned()),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap(px(4.))
                                    .text_size(px(12.))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(delta_c)
                                            .child(delta.to_owned()),
                                    )
                                    .child(
                                        div().text_color(p.faint).child(format!("vs v{prev_ver}")),
                                    ),
                            ),
                    )
                    .child(pro_spring_bar(
                        pro_eid(&self.id, bar_key.to_owned()),
                        ratio.clamp(0.05, 1.0),
                        4.,
                        2.,
                        p.chip,
                        bar_c,
                    ))
            };

            let pass_bar_c = if p.dark { p.green } else { p.green_ink };
            let score_ratio = ((rate_val / 100.0) * 0.98).clamp(0.10, 1.0);
            let cards = vec![
                stat_card(
                    "stat-pass-bar",
                    tr(cx, "Pass rate", "通过率"),
                    format!("{rate_val:.0}%"),
                    None,
                    "+4",
                    true,
                    rate_val / 100.0,
                    pass_bar_c,
                ),
                stat_card(
                    "stat-score-bar",
                    tr(cx, "Avg score", "平均得分"),
                    "4.6".into(),
                    Some("/5"),
                    "+0.3",
                    true,
                    score_ratio,
                    p.blue,
                ),
                stat_card(
                    "stat-lat-bar",
                    tr(cx, "Latency p50", "P50 延迟"),
                    model_lat_str.into(),
                    Some("s"),
                    model_lat_delta,
                    true,
                    lat_ratio,
                    p.blue,
                ),
                stat_card(
                    "stat-cost-bar",
                    tr(cx, "Cost per 1k runs", "每 1k 次成本"),
                    model_cost_str.into(),
                    None,
                    model_cost_delta,
                    model_cost_good,
                    cost_ratio,
                    if model_cost_good { pass_bar_c } else { p.amber },
                ),
            ];

            let stats_grid = if stats_cols_2 {
                let mut col = div().flex().flex_col().gap(px(12.));
                let mut r1 = div().flex().gap(px(12.));
                let mut r2 = div().flex().gap(px(12.));
                for (i, c) in cards.into_iter().enumerate() {
                    if i < 2 {
                        r1 = r1.child(c);
                    } else {
                        r2 = r2.child(c);
                    }
                }
                col = col.child(r1).child(r2);
                col
            } else {
                div().flex().gap(px(12.)).children(cards)
            };

            let kpi_card = self.render_kpi_chart(&p, cx);
            let diff_card = self.render_diff_card((!stack_mid).then_some(diff_w), &p, cx);

            // Test cases table
            let visible_indices = self.filtered_case_indices();
            let active_run_slot = run_progress
                .map(|frac| (frac * visible_indices.len().max(1) as f32).floor() as usize);

            let mut rows_col = div().flex().flex_col();
            for (vis_pos, &case_idx) in visible_indices.iter().enumerate() {
                let case = &self.cases[case_idx];
                let is_selected = self.selected_case_id.as_ref() == Some(&case.id);
                let run_state = match active_run_slot {
                    None => CaseRunState::Done,
                    Some(cur) if vis_pos < cur => CaseRunState::Done,
                    Some(cur) if vis_pos == cur => CaseRunState::Running,
                    Some(_) => CaseRunState::Queued,
                };
                let cid = case.id.clone();
                let row_bg = if run_state == CaseRunState::Running {
                    p.accent_soft
                } else if is_selected {
                    p.select
                } else {
                    transparent_black()
                };

                rows_col = rows_col.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("case-row-{}", case.id)))
                        .role(Role::Row)
                        .tab_index(0)
                        .h(px(46.))
                        .pl(px(18.))
                        .pr(px(18.))
                        .when(vis_pos > 0, |d| d.border_t_1().border_color(p.line_soft))
                        .bg(row_bg)
                        .hover(move |s| if is_selected { s } else { s.bg(p.hover) })
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .text_size(px(13.))
                        .child(
                            div()
                                .w(relative(0.30))
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(11.5))
                                        .text_color(p.faint)
                                        .child(case.id.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_color(if run_state == CaseRunState::Queued {
                                            p.faint
                                        } else {
                                            p.ink
                                        })
                                        .child(case.input.clone()),
                                ),
                        )
                        .child(
                            div()
                                .w(relative(0.22))
                                .min_w_0()
                                .truncate()
                                .text_color(p.muted)
                                .child(case.expected.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(if run_state == CaseRunState::Queued {
                                    p.faint
                                } else {
                                    p.muted
                                })
                                .child(case.candidate_output.clone()),
                        )
                        .child(div().w(px(104.)).flex_none().flex().items_center().child(
                            Self::render_score_pill(case.verdict, case.score, run_state, &p, cx),
                        ))
                        .child(
                            div()
                                .w(px(62.))
                                .flex_none()
                                .text_right()
                                .text_color(p.muted)
                                .child(format!("{:.1}s", case.latency_s)),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.select_case_and_inspect(cid.clone(), true, cx);
                    },
                ));
            }

            let cases_vtx = pro_view_transition(
                pro_eid(&self.id, "eval-cases-vtx"),
                format!("{}-v{}-{}", self.filter.slug(), self.version, self.model),
                rows_col,
            );

            let table_card = div()
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(self.render_table_header(&p, cx))
                .child(cases_vtx);

            let ver_vtx = pro_view_transition(
                pro_eid(&self.id, "eval-ver-vtx"),
                format!("v{}-{}", self.version, self.model),
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(stats_grid)
                    .child(
                        div()
                            .flex()
                            .when(stack_mid, |d| d.flex_col())
                            .gap(px(12.))
                            .child(kpi_card)
                            .child(diff_card),
                    ),
            );

            div()
                .id(pro_eid(&self.id, "body-scroll"))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    div()
                        .p(px(16.))
                        .pt(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(ver_vtx)
                        .child(
                            div()
                                .mt(px(14.))
                                .flex()
                                .flex_col()
                                .gap(px(10.))
                                .child(self.render_cases_header(&p, cx))
                                .child(table_card),
                        ),
                )
        };

        let mut page = div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h_0()
            .bg(p.panel)
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "j" | "down" => this.move_case_selection(1, cx),
                    "k" | "up" => this.move_case_selection(-1, cx),
                    "enter" => {
                        if let Some(cid) = this.selected_case_id.clone() {
                            this.select_case_and_inspect(cid, true, cx);
                        }
                    }
                    "[" => {
                        this.step_pending_change(false);
                        cx.notify();
                    }
                    "]" => {
                        this.step_pending_change(true);
                        cx.notify();
                    }
                    "y" => {
                        let target = this.active_change.or_else(|| {
                            this.changes
                                .iter()
                                .position(|c| c.decision == PromptChangeDecision::Pending)
                        });
                        if let Some(idx) = target {
                            this.review_change(idx, PromptChangeDecision::Accepted, cx);
                        }
                    }
                    "n" => {
                        let target = this.active_change.or_else(|| {
                            this.changes
                                .iter()
                                .position(|c| c.decision == PromptChangeDecision::Pending)
                        });
                        if let Some(idx) = target {
                            this.review_change(idx, PromptChangeDecision::Rejected, cx);
                        }
                    }
                    "r" if !ev.keystroke.modifiers.platform && !ev.keystroke.modifiers.control => {
                        this.trigger_run_eval(cx);
                    }
                    "escape" => {
                        this.version_menu_open = false;
                        this.model_menu_open = false;
                        this.active_change = None;
                        if this.inspector_open {
                            this.inspector_open = false;
                            this.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
                        }
                        cx.notify();
                    }
                    _ => {}
                }
            }))
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
                .absolute()
                .size_0(),
            )
            .child(header)
            .child(body);

        // Animated Version menu popover
        {
            let pop_bg: Hsla = if p.dark {
                rgb(0x1b1b1e).into()
            } else {
                rgb(0xffffff).into()
            };
            let ver_sensor = pro_selection_sensor();
            let ver_scrim = div()
                .id(pro_eid(&self.id, "ver-scrim"))
                .absolute()
                .inset_0()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.version_menu_open = false;
                        cx.notify();
                    }),
                );
            let ver_menu = div()
                .id(pro_eid(&self.id, "ver-menu"))
                .role(Role::Menu)
                .absolute()
                .left(px(192.))
                .min_w(px(188.))
                .p(px(4.))
                .rounded(px(12.))
                .bg(pop_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(1.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "ver-pill"),
                    format!("v{}", self.version),
                    ver_sensor.clone(),
                    8.,
                    p.select,
                    None,
                    false,
                ))
                .children([12_u32, 11, 10, 9, 8, 7].into_iter().map(|ver| {
                    let active = self.version == ver;
                    let rate = VERSION_RATES
                        .iter()
                        .find(|(v, _, _)| *v == ver)
                        .map(|(_, _, r)| *r)
                        .unwrap_or(80.);
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("ver-opt-{ver}")))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .relative()
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.))
                            .text_size(px(13.))
                            .text_color(if active { p.ink } else { p.text })
                            .when(active, |d| {
                                d.font_weight(FontWeight::MEDIUM)
                                    .child(pro_mark_selected(&ver_sensor))
                            })
                            .cursor_pointer()
                            .when(!active, |d| {
                                d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                            })
                            .child(if ver == 12 {
                                format!("v12 · {}", tr(cx, "current", "当前"))
                            } else if ver == 11 {
                                format!("v11 · {}", tr(cx, "baseline", "基线"))
                            } else {
                                format!("v{ver}")
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!("{rate:.0}%"))
                                    .when(active, |d| {
                                        d.child(pro_icon(ProIcon::Check, 13., p.ink))
                                    }),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.version = ver;
                            this.version_menu_open = false;
                            cx.emit(EvalsEvent::SelectVersion(ver));
                            cx.notify();
                        },
                    )
                }));
            let ver_open = self.version_menu_open;
            page = page.child(pro_presence(
                pro_eid(&self.id, "ver-menu-pres"),
                ver_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .absolute()
                        .inset_0()
                        .when(ver_open, |d| d.child(ver_scrim))
                        .child(ver_menu.top(px(44. - 5. * (1. - a))).opacity(a))
                        .into_any_element()
                },
            ));
        }

        // Animated Model selector popover with provider brand tiles
        {
            let pop_bg: Hsla = if p.dark {
                rgb(0x1b1b1e).into()
            } else {
                rgb(0xffffff).into()
            };
            let models: [(&str, &str, &str); 3] = [
                ("Claude", "claude-sonnet-5-5", "92% · $1.84"),
                ("OpenAI", "gpt-5.4", "90% · $2.10"),
                ("Gemini", "gemini-2.5-pro", "89% · $1.12"),
            ];
            let model_sensor = pro_selection_sensor();
            let model_scrim = div()
                .id(pro_eid(&self.id, "model-scrim"))
                .absolute()
                .inset_0()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.model_menu_open = false;
                        cx.notify();
                    }),
                );
            let model_menu = div()
                .id(pro_eid(&self.id, "model-menu"))
                .role(Role::Menu)
                .absolute()
                .left(px(276.))
                .min_w(px(248.))
                .p(px(4.))
                .rounded(px(12.))
                .bg(pop_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(2.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "model-pill"),
                    self.model.clone(),
                    model_sensor.clone(),
                    8.,
                    p.select,
                    None,
                    false,
                ))
                .children(models.into_iter().map(|(prov, m_id, summary)| {
                    let active = self.model.as_ref() == m_id;
                    let m_shared: SharedString = m_id.into();
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("model-opt-{m_id}")))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .relative()
                            .h(px(36.))
                            .px(px(10.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(10.))
                            .text_size(px(12.5))
                            .text_color(if active { p.ink } else { p.text })
                            .when(active, |d| {
                                d.font_weight(FontWeight::MEDIUM)
                                    .child(pro_mark_selected(&model_sensor))
                            })
                            .cursor_pointer()
                            .when(!active, |d| {
                                d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(pro_app_tile(prov, 18., cx))
                                    .child(m_id),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(summary)
                                    .when(active, |d| {
                                        d.child(pro_icon(ProIcon::Check, 13., p.ink))
                                    }),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.model = m_shared.clone();
                            this.model_menu_open = false;
                            cx.emit(EvalsEvent::SelectModel(m_shared.clone()));
                            cx.notify();
                        },
                    )
                }));
            let model_open = self.model_menu_open;
            page = page.child(pro_presence(
                pro_eid(&self.id, "model-menu-pres"),
                model_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .absolute()
                        .inset_0()
                        .when(model_open, |d| d.child(model_scrim))
                        .child(model_menu.top(px(44. - 5. * (1. - a))).opacity(a))
                        .into_any_element()
                },
            ));
        }

        page
    }
}

impl Render for EvalsScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EvalFilter, EvalVerdict, PromptChangeDecision, default_cases, default_prompt_changes,
        provider_for_model,
    };

    #[test]
    fn default_suite_distribution_matches_reference_counts() {
        let cases = default_cases();
        assert_eq!(cases.len(), 25);
        let pass = cases
            .iter()
            .filter(|c| EvalFilter::Pass.matches(c.verdict))
            .count();
        let partial = cases
            .iter()
            .filter(|c| EvalFilter::Partial.matches(c.verdict))
            .count();
        let fail = cases
            .iter()
            .filter(|c| EvalFilter::Fail.matches(c.verdict))
            .count();
        assert_eq!((pass, partial, fail), (22, 2, 1));
        assert_eq!(cases[9].id.as_ref(), "TC-10");
        assert_eq!(cases[9].verdict, EvalVerdict::Fail);
    }

    #[test]
    fn prompt_diff_hunks_and_provider_mapping_behave_deterministically() {
        let mut changes = default_prompt_changes();
        assert_eq!(changes.len(), 7);
        assert!(
            changes
                .iter()
                .all(|c| c.decision == PromptChangeDecision::Pending)
        );
        changes[0].decision = PromptChangeDecision::Accepted;
        changes[1].decision = PromptChangeDecision::Rejected;
        let pending = changes
            .iter()
            .filter(|c| c.decision == PromptChangeDecision::Pending)
            .count();
        assert_eq!(pending, 5);
        assert_eq!(provider_for_model("claude-sonnet-5-5"), "Claude");
        assert_eq!(provider_for_model("gpt-5.4"), "OpenAI");
        assert_eq!(provider_for_model("gemini-2.5-pro"), "Gemini");
    }
}
