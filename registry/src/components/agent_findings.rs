//! Parallel audit and verification swarm card with per-agent progress bars and a live sliding findings feed.
//!
//! Pairs a calm upper list of concurrently working agents ([`pro_agent_avatar`] + [`pro_spring_bar`])
//! with a bottom live feed where newly uncovered `Critical`, `Warning`, and `Note` findings slide
//! smoothly into view as checks progress.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_agent_avatar, pro_app_tile, pro_card_shadow, pro_check_draw, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_presence, pro_rolling_number,
        pro_skeleton_bar, pro_spring_bar, pro_spring_f32, pro_swap_label, pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use web_time::Instant;

const SPARK_SVG: &str = r#"<path d="M7 1.2c.4 2.9 1.9 4.4 4.8 4.8-2.9.4-4.4 1.9-4.8 4.8C6.6 7.9 5.1 6.4 2.2 6 5.1 5.6 6.6 4.1 7 1.2Z" fill="currentColor"></path><circle cx="11.6" cy="11.4" r="1.1" fill="currentColor" opacity="0.55"></circle>"#;

const CRIT_GRID_SVG: &str = r#"<rect x="0" y="0" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="1"></rect><rect x="5.5" y="0" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="1"></rect><rect x="0" y="5.5" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="1"></rect><rect x="5.5" y="5.5" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="0.6"></rect>"#;

const WARN_GRID_SVG: &str = r#"<rect x="0" y="0" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="1"></rect><rect x="5.5" y="0" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="0.6"></rect><rect x="0" y="5.5" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="0.6"></rect><rect x="5.5" y="5.5" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="0.3"></rect>"#;

const NOTE_GRID_SVG: &str = r#"<rect x="0" y="0" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="0.6"></rect><rect x="5.5" y="0" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="0.3"></rect><rect x="0" y="5.5" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="0.3"></rect><rect x="5.5" y="5.5" width="4.5" height="4.5" rx="1" fill="currentColor" opacity="0.3"></rect>"#;

/// Events emitted by [`AgentFindings`].
#[derive(Clone, Debug, PartialEq)]
pub enum AgentFindingsEvent {
    /// User clicked or activated a finding row in the live feed.
    OpenFinding(Finding),
    /// User filtered the feed to one agent (`None` shows all agents).
    FilterAgent(Option<SharedString>),
    /// A new finding entered the live feed during swarm execution.
    FindingDiscovered(Finding),
    /// User replayed the verification run from its initial snapshot.
    ResetRun,
}

/// Severity classification of an uncovered [`Finding`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FindingSeverity {
    /// High-risk defect or security vulnerability requiring immediate attention.
    Critical,
    /// Potential bug, edge case, or invariant hazard.
    Warning,
    /// Optimization or informational note.
    #[default]
    Note,
}

impl FindingSeverity {
    fn svg(self) -> &'static str {
        match self {
            Self::Critical => CRIT_GRID_SVG,
            Self::Warning => WARN_GRID_SVG,
            Self::Note => NOTE_GRID_SVG,
        }
    }

    fn color(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Critical => p.red_ink,
            Self::Warning => p.amber_ink,
            Self::Note => p.accent_info,
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Critical => tr(cx, "Critical", "严重"),
            Self::Warning => tr(cx, "Warning", "警告"),
            Self::Note => tr(cx, "Note", "提示"),
        }
    }
}

/// Execution state of a [`FindingAgent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingAgentStatus {
    /// Actively executing checks.
    Running,
    /// Completed all checks cleanly.
    Done,
    /// Halted due to tool or execution error.
    Failed,
}

/// One auditing or verification agent row in [`AgentFindings`].
#[derive(Clone, Debug, PartialEq)]
pub struct FindingAgent {
    /// Unique agent id referenced by [`Finding::agent_id`].
    pub id: SharedString,
    /// Display name (`"Auditor"`, `"Fuzzer"`, etc.).
    pub name: SharedString,
    /// Optional Chinese display name.
    pub name_zh: Option<SharedString>,
    /// Optional model/provider brand logo key (`"claude"`, `"openai"`, `"gemini"`, etc.).
    pub brand: Option<SharedString>,
    /// Optional custom pixel avatar accent color.
    pub color: Option<Hsla>,
    /// Current live step subtitle.
    pub step: SharedString,
    /// Optional Chinese step subtitle.
    pub step_zh: Option<SharedString>,
    /// Completed checks count.
    pub done: u32,
    /// Total planned checks count.
    pub total: u32,
    /// Explicit status override (`None` infers `Running` while `done < total`, else `Done`).
    pub status: Option<FindingAgentStatus>,
}

impl FindingAgent {
    /// Create a [`FindingAgent`] with `id`, `name`, `done`, and `total`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        done: u32,
        total: u32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            brand: None,
            color: None,
            step: SharedString::default(),
            step_zh: None,
            done,
            total: total.max(1),
            status: None,
        }
    }

    /// Set localized Chinese agent name.
    pub fn with_name_zh(mut self, name_zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(name_zh.into());
        self
    }

    /// Set the provider brand logo (`"claude"`, `"openai"`, `"gemini"`, etc.) rendered via [`pro_app_tile`].
    pub fn with_brand(mut self, brand: impl Into<SharedString>) -> Self {
        self.brand = Some(brand.into());
        self
    }

    /// Set the pixel avatar color for [`pro_agent_avatar`].
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the current activity subtitle (`"Tracing calls in Vault.withdraw"`).
    pub fn with_step(
        mut self,
        step: impl Into<SharedString>,
        step_zh: impl Into<SharedString>,
    ) -> Self {
        self.step = step.into();
        self.step_zh = Some(step_zh.into());
        self
    }

    /// Override inferred execution status.
    pub fn with_status(mut self, status: FindingAgentStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Resolve effective execution status (`Running` until `done >= total` unless overridden).
    pub fn resolved_status(&self) -> FindingAgentStatus {
        self.status.unwrap_or(if self.done >= self.total {
            FindingAgentStatus::Done
        } else {
            FindingAgentStatus::Running
        })
    }

    fn display_name<'a>(&'a self, cx: &App) -> &'a str {
        match &self.name_zh {
            Some(zh) => tr(cx, self.name.as_ref(), zh.as_ref()),
            None => self.name.as_ref(),
        }
    }

    fn display_step<'a>(&'a self, cx: &App) -> &'a str {
        match &self.step_zh {
            Some(zh) => tr(cx, self.step.as_ref(), zh.as_ref()),
            None => self.step.as_ref(),
        }
    }
}

/// One discovered issue or observation surfaced in the bottom feed of [`AgentFindings`].
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    /// Stable finding identifier.
    pub id: SharedString,
    /// Reporting agent id.
    pub agent_id: SharedString,
    /// Short one-line summary (`"Reentrancy in withdraw()"`).
    pub title: SharedString,
    /// Optional Chinese summary.
    pub title_zh: Option<SharedString>,
    /// Severity level (`Critical`, `Warning`, or `Note`).
    pub severity: FindingSeverity,
    /// Optional source file and line (`"Vault.sol:142"`).
    pub location: Option<SharedString>,
    /// Elapsed age in seconds when captured (`0` renders `"now"`).
    pub age_secs: u64,
    /// Optional diagnostic remediation note shown when inspected.
    pub detail: Option<SharedString>,
    /// Optional Chinese remediation note.
    pub detail_zh: Option<SharedString>,
}

impl Finding {
    /// Create a [`Finding`] reported by `agent_id`.
    pub fn new(
        id: impl Into<SharedString>,
        agent_id: impl Into<SharedString>,
        severity: FindingSeverity,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            agent_id: agent_id.into(),
            title: title.into(),
            title_zh: None,
            severity,
            location: None,
            age_secs: 0,
            detail: None,
            detail_zh: None,
        }
    }

    /// Set localized Chinese finding title.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Set the code file and line reference (`"Vault.sol:142"`).
    pub fn with_location(mut self, location: impl Into<SharedString>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Set relative age in seconds (`0` -> `"now"`, `43` -> `"43s"`).
    pub fn with_age_secs(mut self, age_secs: u64) -> Self {
        self.age_secs = age_secs;
        self
    }

    /// Attach an expandable explanation or fix note.
    pub fn with_detail(
        mut self,
        detail: impl Into<SharedString>,
        detail_zh: impl Into<SharedString>,
    ) -> Self {
        self.detail = Some(detail.into());
        self.detail_zh = Some(detail_zh.into());
        self
    }

    fn display_title<'a>(&'a self, cx: &App) -> &'a str {
        match &self.title_zh {
            Some(zh) => tr(cx, self.title.as_ref(), zh.as_ref()),
            None => self.title.as_ref(),
        }
    }

    fn display_detail<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.detail, &self.detail_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// Format relative elapsed seconds into compact feed timestamps (`"now"`, `"43s"`, `"2m"`).
pub fn format_relative_age(secs: u64, zh: bool) -> String {
    if secs < 4 {
        if zh {
            "刚刚".to_string()
        } else {
            "now".to_string()
        }
    } else if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

#[derive(IntoElement)]
struct SlidingFindingRow {
    id: ElementId,
    content: AnyElement,
}

impl RenderOnce for SlidingFindingRow {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let st = w.use_keyed_state(pro_eid(&self.id, "fslide"), cx, |_, inner_cx| {
            let mut s = Slide::new(if reduced { 1.0 } else { 0.0 });
            s.duration(1.0, 0.34, inner_cx);
            s
        });
        let t = if reduced {
            1.0
        } else {
            st.update(cx, |s, cx| s.get(w, cx).clamp(0.0, 1.0))
        };
        let dy = -10.0 * (1.0 - t);
        div()
            .w_full()
            .relative()
            .top(px(dy))
            .opacity(0.22 + 0.78 * t)
            .child(self.content)
    }
}

/// Parallel audit/verification swarm card with live per-agent progress bars and a sliding findings feed.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::agent_findings::{
///     AgentFindings, AgentFindingsEvent, Finding, FindingAgent, FindingSeverity,
/// };
///
/// let audit = cx.new(|cx| {
///     AgentFindings::new("security-verification", cx)
///         .with_title("Verification")
///         .with_agents(vec![
///             FindingAgent::new("auditor", "Auditor", 8, 24)
///                 .with_step("Tracing calls in Vault.withdraw", "追踪 Vault.withdraw 调用链"),
///             FindingAgent::new("fuzzer", "Fuzzer", 7, 18)
///                 .with_step("Fuzzing transfer with 10k inputs", "模糊测试 transfer"),
///         ])
///         .with_findings(vec![
///             Finding::new("f1", "auditor", FindingSeverity::Critical, "Reentrancy in withdraw()")
///                 .with_location("Vault.sol:142"),
///         ])
/// });
/// cx.subscribe(&audit, |_, ev: &AgentFindingsEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct AgentFindings {
    id: ElementId,
    title: SharedString,
    title_zh: Option<SharedString>,
    agents: Vec<FindingAgent>,
    initial_agents: Vec<FindingAgent>,
    findings: Vec<Finding>,
    initial_findings: Vec<Finding>,
    pending_findings: Vec<(usize, Finding)>,
    max_rows: usize,
    filter_agent: Option<SharedString>,
    selected_finding: Option<SharedString>,
    live_swarm: bool,
    applied_ticks: usize,
    loading: bool,
    empty: bool,
    started_at: Instant,
}

impl EventEmitter<AgentFindingsEvent> for AgentFindings {}

impl AgentFindings {
    /// Create an [`AgentFindings`] swarm card preloaded with a 4-agent smart-contract security audit.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let agents = default_agents();
        let findings = default_initial_findings();
        Self {
            id: id.into(),
            title: SharedString::from("Verification"),
            title_zh: Some(SharedString::from("并行安全核验")),
            initial_agents: agents.clone(),
            agents,
            initial_findings: findings.clone(),
            findings,
            pending_findings: default_staged_findings(),
            max_rows: 4,
            filter_agent: None,
            selected_finding: None,
            live_swarm: true,
            applied_ticks: 0,
            loading: false,
            empty: false,
            started_at: Instant::now(),
        }
    }

    /// Set the card header heading (`"Verification"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self.title_zh = None;
        self
    }

    /// Set localized Chinese header heading.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Supply custom auditing agents, disabling automatic default swarm staging.
    pub fn with_agents(mut self, agents: Vec<FindingAgent>) -> Self {
        self.initial_agents = agents.clone();
        self.agents = agents;
        self.live_swarm = false;
        self
    }

    /// Supply findings (ordered newest-first), disabling automatic staged discoveries.
    pub fn with_findings(mut self, findings: Vec<Finding>) -> Self {
        self.initial_findings = findings.clone();
        self.findings = findings;
        self.pending_findings.clear();
        self
    }

    /// Set maximum visible finding rows in the bottom feed (`4` by default).
    pub fn with_rows(mut self, rows: usize) -> Self {
        self.max_rows = rows.clamp(1, 10);
        self
    }

    /// Pre-filter the findings feed to a single agent id.
    pub fn with_filter_agent(mut self, agent_id: Option<SharedString>) -> Self {
        self.filter_agent = agent_id;
        self
    }

    /// Enable or disable live swarm progression and automatic finding discovery.
    pub fn with_live_swarm(mut self, live: bool) -> Self {
        self.live_swarm = live;
        self
    }

    /// Show skeleton placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show empty state when no verification agents are configured.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return current agent statuses and check counts.
    pub fn agents(&self) -> &[FindingAgent] {
        &self.agents
    }

    /// Return discovered findings ordered newest-first.
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// Return the active agent filter id, if any.
    pub fn filter_agent(&self) -> Option<&str> {
        self.filter_agent.as_deref()
    }

    /// Toggle filtering the bottom feed by `agent_id`.
    pub fn toggle_filter(&mut self, agent_id: &str, cx: &mut Context<Self>) {
        if self.filter_agent.as_deref() == Some(agent_id) {
            self.filter_agent = None;
        } else {
            self.filter_agent = Some(SharedString::from(agent_id.to_string()));
        }
        cx.emit(AgentFindingsEvent::FilterAgent(self.filter_agent.clone()));
        cx.notify();
    }

    /// Push a newly uncovered [`Finding`] to the top of the feed.
    pub fn push_finding(&mut self, finding: Finding, cx: &mut Context<Self>) {
        for existing in &mut self.findings {
            existing.age_secs = existing.age_secs.saturating_add(8);
        }
        self.findings.insert(0, finding.clone());
        cx.emit(AgentFindingsEvent::FindingDiscovered(finding));
        cx.notify();
    }

    /// Advance one swarm verification tick, incrementing running checks and surfacing staged findings.
    pub fn step_swarm(&mut self, cx: &mut Context<Self>) -> bool {
        let mut changed = false;
        self.applied_ticks += 1;
        let increments = [2u32, 2, 1, 1];
        for (i, ag) in self.agents.iter_mut().enumerate() {
            if ag.resolved_status() == FindingAgentStatus::Running && ag.done < ag.total {
                let inc = increments[i % increments.len()];
                ag.done = (ag.done + inc).min(ag.total);
                changed = true;
            }
        }
        let tick = self.applied_ticks;
        let mut ready = Vec::new();
        self.pending_findings.retain(|(at_tick, f)| {
            if *at_tick <= tick {
                ready.push(f.clone());
                false
            } else {
                true
            }
        });
        for f in ready {
            for existing in &mut self.findings {
                existing.age_secs = existing.age_secs.saturating_add(9);
            }
            self.findings.insert(0, f.clone());
            cx.emit(AgentFindingsEvent::FindingDiscovered(f));
            changed = true;
        }
        if changed {
            cx.notify();
        }
        changed
    }

    /// Reset swarm checks and findings back to the initial run snapshot.
    pub fn reset_run(&mut self, cx: &mut Context<Self>) {
        self.agents = self.initial_agents.clone();
        self.findings = self.initial_findings.clone();
        self.pending_findings = default_staged_findings();
        self.applied_ticks = 0;
        self.selected_finding = None;
        self.started_at = Instant::now();
        cx.emit(AgentFindingsEvent::ResetRun);
        cx.notify();
    }
}

impl Render for AgentFindings {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let elapsed_secs = if reduced {
            0.0
        } else {
            self.started_at.elapsed().as_secs_f32()
        };
        let elapsed_ms = elapsed_secs * 1000.0;

        let any_running = self
            .agents
            .iter()
            .any(|a| a.resolved_status() == FindingAgentStatus::Running);

        if self.live_swarm && !reduced && any_running {
            let target_ticks = ((elapsed_secs / 2.2) as usize).min(14);
            while self.applied_ticks < target_ticks {
                if !self.step_swarm(cx) {
                    break;
                }
            }
            w.request_animation_frame();
        } else if !reduced && self.loading {
            w.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };

        let done_checks: u32 = self.agents.iter().map(|a| a.done.min(a.total)).sum();
        let total_checks: u32 = self.agents.iter().map(|a| a.total).sum();
        let crit_total = self
            .findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Critical)
            .count();
        let warn_total = self
            .findings
            .iter()
            .filter(|f| f.severity == FindingSeverity::Warning)
            .count();

        let title_str = match &self.title_zh {
            Some(zh) => tr(cx, self.title.as_ref(), zh.as_ref()).to_string(),
            None => self.title.to_string(),
        };
        let progress_str = if tr(cx, "en", "zh") == "zh" {
            format!("已完成 {done_checks} / {total_checks} 项检查")
        } else {
            format!("{done_checks} of {total_checks} checks")
        };

        let crit_col = if crit_total > 0 { p.red_ink } else { p.faint };
        let warn_col = if warn_total > 0 { p.amber_ink } else { p.faint };

        let header = div()
            .px(px(16.0))
            .py(px(13.0))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(pro_svg("0 0 14 14", SPARK_SVG, 14.0, p.accent_info))
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title_str),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.0))
                    .text_color(p.muted)
                    .child(progress_str),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .child(div().size(px(6.0)).rounded_full().bg(crit_col))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "tot-crit"),
                                crit_total as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                crit_col,
                            ))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(crit_col)
                                    .child(tr(cx, "critical", "严重")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.0))
                            .child(div().size(px(6.0)).rounded_full().bg(warn_col))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "tot-warn"),
                                warn_total as f64,
                                0,
                                "",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                warn_col,
                            ))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(warn_col)
                                    .child(tr(cx, "warnings", "警告")),
                            ),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "replay-btn"),
                        tr(cx, "Replay verification swarm", "重放并行核验"),
                        ProIcon::Refresh,
                        24.0,
                        false,
                        cx,
                        |this, _, cx| {
                            this.reset_run(cx);
                        },
                    )),
            );

        if self.loading {
            let phase = (elapsed_secs * 0.9).fract();
            let skel = div()
                .p(px(16.0))
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(pro_skeleton_bar(560.0, 38.0, 8.0, phase, cx))
                .child(pro_skeleton_bar(560.0, 38.0, 8.0, phase + 0.15, cx))
                .child(pro_skeleton_bar(560.0, 38.0, 8.0, phase + 0.30, cx))
                .child(pro_skeleton_bar(480.0, 32.0, 8.0, phase + 0.45, cx));
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(640.0))
                        .rounded(px(10.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .shadow(pro_card_shadow(p.dark))
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(skel),
                )
                .into_any_element();
        }

        if self.empty || self.agents.is_empty() {
            let empty_el = pro_empty_state(
                ProIcon::Shield,
                tr(
                    cx,
                    "No active verification agents",
                    "暂无运行中的核验智能体",
                ),
                tr(
                    cx,
                    "Start an audit swarm to stream checks and severity-ranked findings.",
                    "启动并行核验集群即可实时追踪检查进度与分级问题流。",
                ),
                Some((
                    pro_eid(&self.id, "start-swarm"),
                    SharedString::from(tr(cx, "Start verification", "启动并行核验")),
                    Box::new(|this: &mut Self, _, cx| {
                        this.empty = false;
                        if this.agents.is_empty() {
                            this.agents = default_agents();
                            this.findings = default_initial_findings();
                            this.pending_findings = default_staged_findings();
                        }
                        cx.notify();
                    }),
                )),
                cx,
            );
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(640.0))
                        .rounded(px(10.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .shadow(pro_card_shadow(p.dark))
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(empty_el),
                )
                .into_any_element();
        }

        // Upper agents list
        let mut agents_list = div()
            .id(pro_eid(&self.id, "agents-list"))
            .role(Role::Group)
            .aria_label(tr(cx, "Agents", "核验智能体列表"))
            .px(px(6.0))
            .pt(px(6.0))
            .pb(px(2.0))
            .flex()
            .flex_col()
            .gap(px(2.0));

        for ag in self.agents.clone() {
            let aid = ag.id.clone();
            let status = ag.resolved_status();
            let is_running = status == FindingAgentStatus::Running;
            let selected = self.filter_agent.as_ref() == Some(&aid);
            let dimmed = self.filter_agent.as_ref().is_some_and(|f| f != &aid);
            let avatar_col = ag.color.unwrap_or(p.accent_info);
            let avatar = if let Some(ref brand) = ag.brand {
                if finding_brand_is_model(brand.as_ref()) {
                    pro_agent_avatar(brand.as_ref(), avatar_col, 24.0, is_running, elapsed_ms, cx)
                } else {
                    pro_app_tile(brand.as_ref(), 24.0, cx)
                }
            } else {
                pro_agent_avatar(
                    finding_avatar_seed(&ag),
                    avatar_col,
                    24.0,
                    is_running,
                    elapsed_ms,
                    cx,
                )
            };

            let ag_crit = self
                .findings
                .iter()
                .filter(|f| f.agent_id == aid && f.severity == FindingSeverity::Critical)
                .count();
            let ag_warn = self
                .findings
                .iter()
                .filter(|f| f.agent_id == aid && f.severity == FindingSeverity::Warning)
                .count();

            let ratio = (ag.done as f32 / ag.total.max(1) as f32).clamp(0.0, 1.0);
            let fill_col = match status {
                FindingAgentStatus::Running => p.blue,
                FindingAgentStatus::Done => p.green,
                FindingAgentStatus::Failed => p.red,
            };

            let name_txt = ag.display_name(cx).to_string();
            let is_done = status == FindingAgentStatus::Done;
            let step_txt = if is_done {
                tr(cx, "All checks passed", "全部检查已完成").to_string()
            } else {
                ag.display_step(cx).to_string()
            };
            let dim_alpha = pro_spring_f32(
                pro_eid(&self.id, format!("ag-dim-{aid}")),
                if dimmed { 0.50 } else { 1.0 },
                0.22,
                w,
                cx,
            );

            agents_list = agents_list.child(
                div().w_full().opacity(dim_alpha).child(pro_tactile(
                    pro_eid(&self.id, format!("agent-{aid}")),
                    format!("{name_txt} – {step_txt}"),
                    8.0,
                    if selected {
                        p.select
                    } else {
                        transparent_black()
                    },
                    p.hover,
                    if selected {
                        p.line_strong
                    } else {
                        transparent_black()
                    },
                    p.line_strong,
                    selected,
                    div()
                        .w_full()
                        .h(px(48.0))
                        .px(px(10.0))
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(avatar)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(1.5))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(13.0))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(name_txt),
                                        )
                                        .when(is_done, |d| {
                                            d.child(pro_check_draw(
                                                pro_eid(&self.id, format!("ag-chk-{aid}")),
                                                true,
                                                13.5,
                                                false,
                                                p.green,
                                            ))
                                        }),
                                )
                                .child(div().truncate().child(pro_swap_label(
                                    pro_eid(&self.id, format!("ag-step-{aid}")),
                                    step_txt,
                                    12.0,
                                    FontWeight::NORMAL,
                                    if is_done { p.green_ink } else { p.muted },
                                ))),
                        )
                        .child(div().w(px(124.0)).flex_none().child(pro_spring_bar(
                            pro_eid(&self.id, format!("bar-{aid}")),
                            ratio,
                            6.0,
                            3.0,
                            p.chip_hover,
                            fill_col,
                        )))
                        .child(
                            div()
                                .w(px(46.0))
                                .flex_none()
                                .text_size(px(12.0))
                                .text_color(p.muted)
                                .text_right()
                                .child(format!("{} / {}", ag.done, ag.total)),
                        )
                        .child(
                            div()
                                .w(px(62.0))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap(px(9.0))
                                .child(agent_found_pill(
                                    CRIT_GRID_SVG,
                                    ag_crit,
                                    p.red_ink,
                                    p.faint,
                                    p.muted,
                                ))
                                .child(agent_found_pill(
                                    WARN_GRID_SVG,
                                    ag_warn,
                                    p.amber_ink,
                                    p.faint,
                                    p.muted,
                                )),
                        ),
                    cx,
                    {
                        let aid = aid.clone();
                        move |this, _, cx| {
                            this.toggle_filter(aid.as_ref(), cx);
                        }
                    },
                )),
            );
        }

        // Feed header bar (`LATEST FINDINGS` + optional active agent filter chip)
        let active_filter_name = self.filter_agent.as_ref().and_then(|fid| {
            self.agents
                .iter()
                .find(|a| &a.id == fid)
                .map(|a| a.display_name(cx).to_string())
        });
        let has_filter_chip = active_filter_name.is_some();
        let clear_lbl = format!(
            "{} {}",
            tr(cx, "Clear filter", "清除筛选"),
            active_filter_name.as_deref().unwrap_or("")
        );
        let clear_chip_btn = pro_tactile(
            pro_eid(&self.id, "clear-filter-chip"),
            clear_lbl,
            6.0,
            p.chip,
            p.chip_hover,
            transparent_black(),
            p.line,
            false,
            div()
                .h(px(20.0))
                .pl(px(8.0))
                .pr(px(6.0))
                .flex()
                .items_center()
                .gap(px(4.0))
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child(active_filter_name.unwrap_or_default())
                .child(pro_icon(ProIcon::Close, 11.0, p.muted)),
            cx,
            |this, _, cx| {
                this.filter_agent = None;
                cx.emit(AgentFindingsEvent::FilterAgent(None));
                cx.notify();
            },
        )
        .w_auto();
        let filter_chip_pres = pro_presence(
            pro_eid(&self.id, "flt-chip-pres"),
            has_filter_chip,
            0.18,
            0.14,
            {
                let mut chip_opt = Some(clear_chip_btn);
                move |amt| {
                    div()
                        .opacity(amt)
                        .children(chip_opt.take())
                        .into_any_element()
                }
            },
        );

        let show_next_btn = any_running && !self.pending_findings.is_empty();
        let next_btn = pro_tactile(
            pro_eid(&self.id, "step-next-btn"),
            tr(cx, "Advance check", "推进检查"),
            5.0,
            p.chip,
            p.chip_hover,
            transparent_black(),
            p.line,
            false,
            div()
                .h(px(22.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .gap(px(4.0))
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child(pro_icon(ProIcon::Bolt, 11.5, p.accent_info))
                .child(tr(cx, "Next check", "下一步")),
            cx,
            |this, _, cx| {
                this.step_swarm(cx);
            },
        )
        .w_auto();
        let next_btn_pres = pro_presence(
            pro_eid(&self.id, "next-btn-pres"),
            show_next_btn,
            0.18,
            0.14,
            {
                let mut btn_opt = Some(next_btn);
                move |amt| {
                    div()
                        .opacity(amt)
                        .children(btn_opt.take())
                        .into_any_element()
                }
            },
        );

        let feed_head = div()
            .h(px(40.0))
            .mt(px(4.0))
            .px(px(16.0))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.muted)
                            .child(tr(cx, "LATEST FINDINGS", "最新发现")),
                    )
                    .child(filter_chip_pres),
            )
            .child(next_btn_pres);

        // Filtered visible findings
        let zh_mode = tr(cx, "en", "zh") == "zh";
        let visible_findings: Vec<Finding> = self
            .findings
            .iter()
            .filter(|f| {
                self.filter_agent
                    .as_ref()
                    .is_none_or(|fid| &f.agent_id == fid)
            })
            .take(self.max_rows)
            .cloned()
            .collect();

        let feed_body = if visible_findings.is_empty() {
            div()
                .h(px(44.0))
                .px(px(16.0))
                .pb(px(8.0))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.0))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "No findings reported for this filter yet.",
                    "当前筛选条件下暂未发现问题。",
                ))
                .into_any_element()
        } else {
            let target_h = visible_findings.len() as f32 * 40.0 + 8.0;
            let animated_h = pro_spring_f32(pro_eid(&self.id, "feed-h"), target_h, 0.34, w, cx);
            let has_open_detail = visible_findings
                .iter()
                .any(|f| self.selected_finding.as_ref() == Some(&f.id));

            let mut list = div()
                .id(pro_eid(&self.id, "findings-feed"))
                .role(Role::Status)
                .px(px(6.0))
                .pb(px(8.0))
                .when(!has_open_detail, |d| d.min_h(px(animated_h)))
                .flex()
                .flex_col();

            for f in visible_findings {
                let fid = f.id.clone();
                let is_open = self.selected_finding.as_ref() == Some(&fid);
                let sev_col = f.severity.color(&p);
                let sev_lbl = f.severity.label(cx);
                let f_title = f.display_title(cx).to_string();
                let ag_name = self
                    .agents
                    .iter()
                    .find(|a| a.id == f.agent_id)
                    .map(|a| a.display_name(cx).to_string())
                    .unwrap_or_else(|| f.agent_id.to_string());
                let time_lbl = format_relative_age(f.age_secs, zh_mode);
                let detail_opt = f.display_detail(cx).map(SharedString::from);
                let f_for_click = f.clone();

                let row_btn = pro_tactile(
                    pro_eid(&self.id, format!("finding-{fid}")),
                    format!("{sev_lbl}: {f_title} ({ag_name})"),
                    8.0,
                    if is_open {
                        p.select
                    } else {
                        transparent_black()
                    },
                    p.hover,
                    if is_open {
                        p.line_strong
                    } else {
                        transparent_black()
                    },
                    p.line,
                    is_open,
                    div()
                        .w_full()
                        .h(px(40.0))
                        .px(px(10.0))
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(pro_svg("0 0 10 10", f.severity.svg(), 10.0, sev_col))
                        .child(
                            div()
                                .flex_none()
                                .max_w(px(270.0))
                                .truncate()
                                .text_size(px(13.0))
                                .text_color(p.ink)
                                .child(f_title),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .text_size(px(12.0))
                                .text_color(p.muted)
                                .child(div().truncate().child(ag_name))
                                .when_some(f.location.clone(), |meta, loc| {
                                    meta.child(div().text_color(p.faint).child("·")).child(
                                        div()
                                            .truncate()
                                            .text_size(px(11.5))
                                            .text_color(p.muted)
                                            .child(loc),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .w(px(34.0))
                                .flex_none()
                                .text_size(px(12.0))
                                .text_color(p.faint)
                                .text_right()
                                .child(time_lbl),
                        ),
                    cx,
                    {
                        let fid = fid.clone();
                        move |this, _, cx| {
                            if this.selected_finding.as_ref() == Some(&fid) {
                                this.selected_finding = None;
                            } else {
                                this.selected_finding = Some(fid.clone());
                            }
                            cx.emit(AgentFindingsEvent::OpenFinding(f_for_click.clone()));
                            cx.notify();
                        }
                    },
                );

                let detail_fold = pro_fold(
                    pro_eid(&self.id, format!("fdet-{fid}")),
                    is_open && detail_opt.is_some(),
                    48.0,
                    0.0,
                    {
                        let sev_lbl = sev_lbl.to_string();
                        let loc = f.location.clone();
                        move || {
                            let note = detail_opt.unwrap_or_default();
                            div()
                                .mx(px(10.0))
                                .mb(px(6.0))
                                .px(px(11.0))
                                .py(px(8.0))
                                .rounded(px(7.0))
                                .bg(p.subtle)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .flex_col()
                                .gap(px(3.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(sev_col)
                                        .child(sev_lbl)
                                        .when_some(loc, |d, l| {
                                            d.child(
                                                div()
                                                    .font_weight(FontWeight::NORMAL)
                                                    .text_color(p.muted)
                                                    .child(l),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .line_height(px(17.5))
                                        .text_color(p.text)
                                        .child(note),
                                )
                                .into_any_element()
                        }
                    },
                );

                list = list.child(SlidingFindingRow {
                    id: pro_eid(&self.id, format!("slot-{fid}")),
                    content: div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .child(row_btn)
                        .child(detail_fold)
                        .into_any_element(),
                });
            }
            list.into_any_element()
        };

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(640.0))
                    .rounded(px(10.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .shadow(pro_card_shadow(p.dark))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(agents_list)
                    .child(feed_head)
                    .child(feed_body),
            )
            .into_any_element()
    }
}

fn finding_brand_is_model(brand: &str) -> bool {
    let lower = brand.trim().to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "claude"
            | "anthropic"
            | "openai"
            | "gpt"
            | "codex"
            | "gemini"
            | "deepseek"
            | "mistral"
            | "qwen"
            | "meta"
            | "llama"
            | "xai"
            | "grok"
    )
}

fn finding_avatar_seed(agent: &FindingAgent) -> &'static str {
    let combined = format!("{} {}", agent.id, agent.name).to_ascii_lowercase();
    if combined.contains("claude")
        || combined.contains("opus")
        || combined.contains("sonnet")
        || combined.contains("gas")
        || combined.contains("review")
    {
        "claude"
    } else if combined.contains("gpt")
        || combined.contains("openai")
        || combined.contains("fuzz")
        || combined.contains("test")
        || combined.contains("lint")
    {
        "gpt"
    } else if combined.contains("gemini")
        || combined.contains("prov")
        || combined.contains("formal")
        || combined.contains("invar")
    {
        "gemini"
    } else if combined.contains("deepseek") {
        "deepseek"
    } else if combined.contains("mistral") {
        "mistral"
    } else if combined.contains("qwen") {
        "qwen"
    } else {
        "claude"
    }
}

fn agent_found_pill(
    svg_inner: &'static str,
    count: usize,
    active_col: Hsla,
    zero_icon: Hsla,
    zero_num: Hsla,
) -> Div {
    let is_zero = count == 0;
    let icon_c = if is_zero { zero_icon } else { active_col };
    let num_c = if is_zero { zero_num } else { active_col };
    div()
        .min_w(px(22.0))
        .opacity(if is_zero { 0.34 } else { 1.0 })
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(pro_svg("0 0 10 10", svg_inner, 10.0, icon_c))
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(num_c)
                .child(count.to_string()),
        )
}

fn default_agents() -> Vec<FindingAgent> {
    vec![
        FindingAgent::new("auditor", "Auditor", 6, 24)
            .with_name_zh("合约审计器")
            .with_brand("claude")
            .with_color(rgb(0xd97757).into())
            .with_step(
                "Tracing calls in Vault.withdraw",
                "追踪 Vault.withdraw 外部调用链",
            ),
        FindingAgent::new("fuzzer", "Fuzzer", 5, 18)
            .with_name_zh("模糊测试器")
            .with_brand("openai")
            .with_color(rgb(0x0190fe).into())
            .with_step(
                "Fuzzing transfer with 10k inputs",
                "注入 10,000 组边界输入测试 transfer",
            ),
        FindingAgent::new("prover", "Prover", 4, 12)
            .with_name_zh("形式化证明器")
            .with_brand("gemini")
            .with_color(rgb(0xa651f0).into())
            .with_step(
                "Proving totalSupply invariant",
                "形式化验证 totalSupply 守恒不变量",
            ),
        FindingAgent::new("gas", "Gas review", 3, 10)
            .with_name_zh("Gas 审查器")
            .with_brand("claude")
            .with_color(rgb(0x13b3a0).into())
            .with_step("Profiling storage reads", "分析循环中的存储槽读取开销"),
    ]
}

fn default_initial_findings() -> Vec<Finding> {
    vec![
        Finding::new(
            "f-reentrancy",
            "auditor",
            FindingSeverity::Critical,
            "Reentrancy in withdraw()",
        )
        .with_title_zh("withdraw() 存在外部回调重入风险")
        .with_location("Vault.sol:142")
        .with_age_secs(0)
        .with_detail(
            "External ETH transfer executes before balances[msg.sender] is zeroed; apply checks-effects-interactions or nonReentrant.",
            "外部转账发生在扣减 balances[msg.sender] 之前，请调整状态写入顺序或增加重入锁。",
        ),
        Finding::new(
            "f-immutable",
            "gas",
            FindingSeverity::Note,
            "Constant could be immutable",
        )
        .with_title_zh("构造期常量可声明为 immutable")
        .with_location("Pool.sol:12")
        .with_age_secs(43)
        .with_detail(
            "feeRecipient is assigned once in the constructor and read on every swap, saving ~2,100 gas per call as immutable.",
            "feeRecipient 仅在构造函数中赋值，改为 immutable 后每次交易可节省约 2,100 Gas。",
        ),
    ]
}

fn default_staged_findings() -> Vec<(usize, Finding)> {
    vec![
        (
            2,
            Finding::new(
                "f-unchecked",
                "fuzzer",
                FindingSeverity::Warning,
                "Unchecked return on external transfer",
            )
            .with_title_zh("未检查非标代币 transfer 返回值")
            .with_location("Router.sol:88")
            .with_age_secs(0)
            .with_detail(
                "Non-standard ERC-20 tokens return false instead of reverting; wrap transfers with SafeERC20.",
                "部分非标准代币在失败时返回 false 而非回滚，建议通过安全封装检查返回值。",
            ),
        ),
        (
            4,
            Finding::new(
                "f-rounding",
                "prover",
                FindingSeverity::Warning,
                "Share rounding favors first depositor",
            )
            .with_title_zh("份额向下取整存在首笔存款通胀边界")
            .with_location("Vault.sol:97")
            .with_age_secs(0)
            .with_detail(
                "Counterexample found when totalAssets == 1 and donation inflates share price; mint virtual dead shares.",
                "当 totalAssets 极小时外部捐赠会放大单份净值，建议引入虚拟锚定份额抵御通胀攻击。",
            ),
        ),
        (
            6,
            Finding::new(
                "f-sload",
                "gas",
                FindingSeverity::Note,
                "Redundant SLOAD inside reward loop",
            )
            .with_title_zh("奖励结算循环内重复读取存储变量")
            .with_location("Staking.sol:204")
            .with_age_secs(0)
            .with_detail(
                "Cache rewardRate in a stack local before iterating over pool checkpoints.",
                "在遍历检查点前将 rewardRate 缓存到局部变量即可消除重复 SLOAD。",
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        AgentFindings, Finding, FindingAgent, FindingAgentStatus, FindingSeverity,
        format_relative_age,
    };
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn agent_status_infers_running_until_done_and_formats_ages() {
        let running = FindingAgent::new("a1", "Auditor", 6, 24);
        assert_eq!(running.resolved_status(), FindingAgentStatus::Running);

        let done = FindingAgent::new("a2", "Prover", 12, 12);
        assert_eq!(done.resolved_status(), FindingAgentStatus::Done);

        assert_eq!(format_relative_age(0, false), "now");
        assert_eq!(format_relative_age(43, false), "43s");
        assert_eq!(format_relative_age(125, false), "2m");
    }

    #[gpui::test]
    fn swarm_stepping_filtering_and_reset_work(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| AgentFindings::new("test-findings", cx).with_live_swarm(false));

        view.update(cx, |v, cx| {
            assert_eq!(v.findings().len(), 2);
            assert_eq!(v.filter_agent(), None);

            v.toggle_filter("auditor", cx);
            assert_eq!(v.filter_agent(), Some("auditor"));

            v.step_swarm(cx);
            v.step_swarm(cx);
            assert!(
                v.findings().len() >= 3,
                "staged finding should slide into the feed by tick 2"
            );

            v.push_finding(
                Finding::new(
                    "custom",
                    "auditor",
                    FindingSeverity::Critical,
                    "Custom issue",
                ),
                cx,
            );
            assert_eq!(v.findings()[0].id.as_ref(), "custom");

            v.reset_run(cx);
            assert_eq!(v.findings().len(), 2);
        });
    }
}
