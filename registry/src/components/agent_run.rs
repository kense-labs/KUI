//! Autonomous agent run detail view with live plan, tool activity, approvals, and draft diff.
//!
//! Displays one multi-step background agent execution ticking through its plan,
//! grouped tool call traces, human-in-the-loop approval gates, inline memo diff
//! hunks, and tabbed deliverable viewers inside an [`AppShell`] workspace.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProEmptyCta, ProPalette, pro_app_tile, pro_btn, pro_eid, pro_empty_state,
            pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_presence,
            pro_progress_ring, pro_selection_sensor, pro_skeleton_bar, pro_spring_bar,
            pro_trend_canvas, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        clipboard,
        control::activate,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const RUNS_PLAY_SVG: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.9"/><path d="M15.945 12.395c-.176.627-1.012 1.07-2.682 1.955-1.615.856-2.422 1.285-3.073 1.113-.269-.072-.514-.207-.712-.393C9 14.62 9 13.747 9 12s0-2.62.478-3.07c.198-.186.443-.321.712-.392.651-.173 1.458.256 3.073 1.112 1.67.886 2.506 1.329 2.682 1.955.073.259.073.531 0 .79Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"/>"#;

const STOP_CIRCLE_SVG: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.9"/><path d="M9.389 15.163c.504.337 1.207.337 2.611.337s2.107 0 2.611-.337a2 2 0 0 0 .552-.552c.337-.504.337-1.207.337-2.611s0-2.107-.337-2.611a2 2 0 0 0-.552-.552C14.107 8.5 13.404 8.5 12 8.5s-2.107 0-2.611.337a2 2 0 0 0-.552.552C8.5 9.893 8.5 10.596 8.5 12s0 2.107.337 2.611c.146.218.334.406.552.552Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const BRAIN_AGENT_SVG: &str = r#"<path d="M7 4.5C5.343 4.5 4 5.843 4 7.5c0 .569.158 1.1.433 1.554C3.047 9.319 2 10.537 2 12s1.047 2.681 2.433 2.946M7 4.5C7 3.119 8.119 2 9.5 2S12 3.119 12 4.5v15c0 1.381-1.119 2.5-2.5 2.5S7 20.881 7 19.5c-1.657 0-3-1.343-3-3 0-.569.158-1.1.433-1.554M7 4.5c0 .818.393 1.544 1 2M4.433 14.946A3.01 3.01 0 0 1 6 13.671" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M17 19.5c1.657 0 3-1.343 3-3 0-.569-.158-1.1-.433-1.554C20.953 14.681 22 13.463 22 12s-1.047-2.682-2.433-2.947M17 19.5c0 1.381-1.119 2.5-2.5 2.5S12 20.881 12 19.5v-15C12 3.119 13.119 2 14.5 2S17 3.119 17 4.5c1.657 0 3 1.343 3 3 0 .569-.158 1.1-.433 1.553M17 19.5c0-.818-.393-1.544-1-2m3.567-8.447a3.01 3.01 0 0 1-1.567 1.276" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const ACTIVITY_BOX_SVG: &str = r#"<path d="M4.318 19.682C3 18.364 3 16.243 3 12s0-6.364 1.318-7.682C5.636 3 7.757 3 12 3s6.364 0 7.682 1.318C21 5.636 21 7.757 21 12s0 6.364-1.318 7.682C18.364 21 16.243 21 12 21s-6.364 0-7.682-1.318Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="m7 14 2.793-2.793a1 1 0 0 1 1.414 0l1.586 1.586a1 1 0 0 0 1.414 0L17 10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const BELL_HOOD_SVG: &str = r#"<path d="M20 18.501 18.349 7.934C17.86 4.806 15.166 2.5 12 2.5S6.14 4.806 5.651 7.934L4 18.501" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M20 18.5c0-1.657-3.582-3-8-3s-8 1.343-8 3 3.582 3 8 3 8-1.343 8-3Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M13 18.5h-2" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const DIFF_SPARK_SVG: &str = r#"<path d="M8 1.75c.3 2.9 1.35 3.95 4.25 4.25-2.9.3-3.95 1.35-4.25 4.25C7.7 7.35 6.65 6.3 3.75 6 6.65 5.7 7.7 4.65 8 1.75Z" fill="currentColor"/><path d="M12.5 10.25c.15 1.25.6 1.7 1.75 1.75-1.15.15-1.6.6-1.75 1.75-.15-1.15-.6-1.6-1.75-1.75 1.15-.05 1.6-.5 1.75-1.75Z" fill="currentColor"/>"#;

const GDOC_TILE_SVG: &str = r##"<path d="M6 2h8.5L20 7.5V20a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2Z" fill="currentColor"/><path d="M14.5 2 20 7.5h-4a1.5 1.5 0 0 1-1.5-1.5Z" fill="#A1C2FA"/><path d="M8 11h8v1.4H8zm0 3h8v1.4H8zm0 3h5.5v1.4H8z" fill="#ffffff"/>"##;

const FILE_DOC_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" fill-opacity="0.45"/><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"/><path d="M4 6.6h4M4 8.6h2.6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/>"#;

const FILE_SHEET_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" fill-opacity="0.45"/><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"/><rect x="3.4" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"/><rect x="6.3" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"/><rect x="3.4" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"/><rect x="6.3" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"/>"#;

/// Overall lifecycle phase of an autonomous agent run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunPhase {
    #[default]
    Running,
    Waiting,
    Done,
    Stopped,
}

/// Execution status of a single step inside [`AgentRunScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunStepStatus {
    #[default]
    Pending,
    Active,
    Waiting,
    Done,
    Failed,
    Skipped,
}

/// Risk level displayed on an approval gate card.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApprovalRisk {
    Low,
    #[default]
    Medium,
    High,
}

/// Resolution state of a human-in-the-loop approval request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApprovalDecision {
    #[default]
    Pending,
    Approved,
    Denied,
}

/// Individual tool invocation belonging to a [`RunStep`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunToolCall {
    pub id: SharedString,
    pub app: SharedString,
    pub name: SharedString,
    pub summary: SharedString,
    pub duration: SharedString,
    pub done: bool,
    pub args: SharedString,
    pub result: SharedString,
}

/// Human-in-the-loop approval gate attached to a [`RunStep`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunApproval {
    pub title: SharedString,
    pub description: SharedString,
    pub tool: SharedString,
    pub app: SharedString,
    pub risk: ApprovalRisk,
    pub details: Vec<(SharedString, SharedString)>,
    pub decision: ApprovalDecision,
}

/// One step in the autonomous run plan and activity feed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunStep {
    pub id: SharedString,
    pub title: SharedString,
    pub detail: SharedString,
    pub tool: SharedString,
    pub duration: SharedString,
    pub status: RunStepStatus,
    pub calls: Vec<RunToolCall>,
    pub approval: Option<RunApproval>,
}

/// Review decision for a single inline diff hunk in [`RunDraft`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffHunkState {
    #[default]
    Pending,
    Accepted,
    Rejected,
}

/// Segment of an inline diff draft: unchanged prose or a reviewable hunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DraftToken {
    Plain(SharedString),
    Hunk {
        index: usize,
        deleted: SharedString,
        inserted: SharedString,
        state: DiffHunkState,
    },
}

/// Inline diff draft shown in the right-hand work area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunDraft {
    pub label: SharedString,
    pub tokens: Vec<DraftToken>,
}

impl Default for RunDraft {
    fn default() -> Self {
        default_run_draft()
    }
}

impl RunDraft {
    /// Count of diff hunks still awaiting review.
    pub fn pending_count(&self) -> usize {
        self.tokens
            .iter()
            .filter(|t| {
                matches!(
                    t,
                    DraftToken::Hunk {
                        state: DiffHunkState::Pending,
                        ..
                    }
                )
            })
            .count()
    }

    /// Total number of diff hunks in the draft.
    pub fn total_hunks(&self) -> usize {
        self.tokens
            .iter()
            .filter(|t| matches!(t, DraftToken::Hunk { .. }))
            .count()
    }

    /// Accept a single diff hunk by zero-based hunk `index`.
    pub fn accept_hunk(&mut self, index: usize) {
        for tok in &mut self.tokens {
            if let DraftToken::Hunk {
                index: idx, state, ..
            } = tok
                && *idx == index
            {
                *state = DiffHunkState::Accepted;
            }
        }
    }

    /// Reject a single diff hunk by zero-based hunk `index`.
    pub fn reject_hunk(&mut self, index: usize) {
        for tok in &mut self.tokens {
            if let DraftToken::Hunk {
                index: idx, state, ..
            } = tok
                && *idx == index
            {
                *state = DiffHunkState::Rejected;
            }
        }
    }

    /// Accept all remaining diff hunks.
    pub fn accept_all(&mut self) {
        for tok in &mut self.tokens {
            if let DraftToken::Hunk { state, .. } = tok
                && *state == DiffHunkState::Pending
            {
                *state = DiffHunkState::Accepted;
            }
        }
    }

    /// Reject all remaining diff hunks.
    pub fn reject_all(&mut self) {
        for tok in &mut self.tokens {
            if let DraftToken::Hunk { state, .. } = tok
                && *state == DiffHunkState::Pending
            {
                *state = DiffHunkState::Rejected;
            }
        }
    }

    /// Reset all diff hunks back to [`DiffHunkState::Pending`].
    pub fn reset_all(&mut self) {
        for tok in &mut self.tokens {
            if let DraftToken::Hunk { state, .. } = tok {
                *state = DiffHunkState::Pending;
            }
        }
    }

    /// Reconstruct the original text before any suggested edits.
    pub fn original_text(&self) -> String {
        let mut out = String::new();
        for tok in &self.tokens {
            match tok {
                DraftToken::Plain(s) => out.push_str(s),
                DraftToken::Hunk { deleted, .. } => out.push_str(deleted),
            }
        }
        out
    }

    /// Reconstruct the fully suggested text if all edits are accepted.
    pub fn suggested_text(&self) -> String {
        let mut out = String::new();
        for tok in &self.tokens {
            match tok {
                DraftToken::Plain(s) => out.push_str(s),
                DraftToken::Hunk { inserted, .. } => out.push_str(inserted),
            }
        }
        out
    }

    /// Reconstruct the text reflecting current `Accepted` vs `Rejected`/`Pending` hunk choices.
    pub fn committed_text(&self) -> String {
        let mut out = String::new();
        for tok in &self.tokens {
            match tok {
                DraftToken::Plain(s) => out.push_str(s),
                DraftToken::Hunk {
                    deleted,
                    inserted,
                    state,
                    ..
                } => match state {
                    DiffHunkState::Accepted => out.push_str(inserted),
                    DiffHunkState::Pending | DiffHunkState::Rejected => out.push_str(deleted),
                },
            }
        }
        out
    }
}

/// Viewer format for a deliverable file produced by the run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunFileKind {
    #[default]
    Document,
    Sheet,
    Image,
}

/// Deliverable file listed under `Files` that opens in its own work-area tab.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunFileArtifact {
    pub name: SharedString,
    pub kind: RunFileKind,
    pub subtitle: SharedString,
    pub heading: SharedString,
    pub paragraphs: Vec<SharedString>,
    pub bullets: Vec<SharedString>,
    pub sheet_headers: Vec<SharedString>,
    pub sheet_rows: Vec<Vec<SharedString>>,
}

/// Events emitted by [`AgentRunScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentRunEvent {
    /// Sidebar or breadcrumb navigation requested.
    Navigate(SharedString),
    /// Approval granted for the step with the given `id`.
    Approved(SharedString),
    /// Approval denied for the step with the given `id`.
    Denied(SharedString),
    /// Run manually stopped by the user.
    Stopped,
    /// Every step in the run completed.
    Completed,
    /// Deliverable file opened in a work-area tab.
    FileOpened(SharedString),
}

impl EventEmitter<AgentRunEvent> for AgentRunScreen {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum InspectorTab {
    NeedsInput,
    File(usize),
}

/// Advance one step in `steps`, pausing on [`RunPhase::Waiting`] if the next step requires approval.
pub fn advance_run_step(steps: &mut [RunStep]) -> RunPhase {
    if let Some(active_idx) = steps.iter().position(|s| s.status == RunStepStatus::Active) {
        for call in &mut steps[active_idx].calls {
            call.done = true;
        }
        steps[active_idx].status = RunStepStatus::Done;
    }

    let Some(next_idx) = steps
        .iter()
        .position(|s| s.status == RunStepStatus::Pending)
    else {
        return RunPhase::Done;
    };

    if steps[next_idx]
        .approval
        .as_ref()
        .is_some_and(|a| a.decision == ApprovalDecision::Pending)
    {
        steps[next_idx].status = RunStepStatus::Waiting;
        return RunPhase::Waiting;
    }

    steps[next_idx].status = RunStepStatus::Active;
    if let Some(first_call) = steps[next_idx].calls.first_mut() {
        first_call.done = true;
    }
    RunPhase::Running
}

/// Approve the waiting step matching `step_id` (or the current waiting step) and continue execution.
pub fn approve_waiting_step(
    steps: &mut [RunStep],
    step_id: &str,
) -> Option<(SharedString, RunPhase)> {
    let idx = steps.iter().position(|s| {
        s.status == RunStepStatus::Waiting && (step_id.is_empty() || s.id.as_ref() == step_id)
    })?;
    let resolved_id = steps[idx].id.clone();
    if let Some(ask) = &mut steps[idx].approval {
        ask.decision = ApprovalDecision::Approved;
    }
    for call in &mut steps[idx].calls {
        call.done = true;
    }
    steps[idx].status = RunStepStatus::Done;
    let phase = if let Some(next_idx) = steps
        .iter()
        .position(|s| s.status == RunStepStatus::Pending)
    {
        if steps[next_idx]
            .approval
            .as_ref()
            .is_some_and(|a| a.decision == ApprovalDecision::Pending)
        {
            steps[next_idx].status = RunStepStatus::Waiting;
            RunPhase::Waiting
        } else {
            steps[next_idx].status = RunStepStatus::Active;
            RunPhase::Running
        }
    } else {
        RunPhase::Done
    };
    Some((resolved_id, phase))
}

/// Deny the waiting step matching `step_id` (or the current waiting step) and stop the run.
pub fn deny_waiting_step(steps: &mut [RunStep], step_id: &str) -> Option<SharedString> {
    let idx = steps.iter().position(|s| {
        s.status == RunStepStatus::Waiting && (step_id.is_empty() || s.id.as_ref() == step_id)
    })?;
    let resolved_id = steps[idx].id.clone();
    if let Some(ask) = &mut steps[idx].approval {
        ask.decision = ApprovalDecision::Denied;
    }
    steps[idx].status = RunStepStatus::Failed;
    Some(resolved_id)
}

/// Canonical 5-step Q3 board preparation run matching the reference specification.
pub fn default_run_steps() -> Vec<RunStep> {
    vec![
        RunStep {
            id: "step-1".into(),
            title: "Pull Q3 revenue and churn".into(),
            detail: "From Stripe, July to September".into(),
            tool: "stripe.query".into(),
            duration: "1.3s".into(),
            status: RunStepStatus::Active,
            calls: vec![
                RunToolCall {
                    id: "c1-rev".into(),
                    app: "stripe".into(),
                    name: "stripe.query".into(),
                    summary: "Revenue by month, Q3".into(),
                    duration: "700ms".into(),
                    done: true,
                    args: "{\n  \"metric\": \"net_revenue\",\n  \"period\": \"2026-Q3\",\n  \"group_by\": \"month\"\n}".into(),
                    result: "{\n  \"jul\": \"$1.31M\",\n  \"aug\": \"$1.42M\",\n  \"sep\": \"$1.57M\",\n  \"q3_total\": \"$4.30M\",\n  \"yoy_growth\": \"21%\"\n}".into(),
                },
                RunToolCall {
                    id: "c1-churn".into(),
                    app: "stripe".into(),
                    name: "stripe.query".into(),
                    summary: "Customer churn, Q3".into(),
                    duration: "0.1s".into(),
                    done: false,
                    args: "{\n  \"metric\": \"logo_churn\",\n  \"segment\": \"enterprise\",\n  \"period\": \"2026-Q3\"\n}".into(),
                    result: "{\n  \"churn_rate\": \"1.8%\",\n  \"new_enterprise_customers\": 6\n}".into(),
                },
            ],
            approval: None,
        },
        RunStep {
            id: "step-2".into(),
            title: "Read last quarter's board memo".into(),
            detail: "Keeps the structure the board knows".into(),
            tool: "search".into(),
            duration: "0.9s".into(),
            status: RunStepStatus::Pending,
            calls: vec![
                RunToolCall {
                    id: "c2-drive".into(),
                    app: "google drive".into(),
                    name: "drive.search".into(),
                    summary: "Board memo – Q2".into(),
                    duration: "480ms".into(),
                    done: false,
                    args: "{\n  \"query\": \"Board memo Q2\",\n  \"folder\": \"Board Updates\"\n}".into(),
                    result: "{\n  \"file_id\": \"doc_q2_board\",\n  \"name\": \"Board memo – Q2\"\n}".into(),
                },
                RunToolCall {
                    id: "c2-doc".into(),
                    app: "gdoc".into(),
                    name: "docs.read".into(),
                    summary: "Outline and KPI baseline".into(),
                    duration: "420ms".into(),
                    done: false,
                    args: "{\n  \"doc_id\": \"doc_q2_board\"\n}".into(),
                    result: "{\n  \"sections\": [\"Revenue\", \"Retention\", \"Q4 Outlook\"]\n}".into(),
                },
            ],
            approval: None,
        },
        RunStep {
            id: "step-3".into(),
            title: "Update the memo draft".into(),
            detail: "Revenue, churn and the Q4 outlook".into(),
            tool: "docs.write".into(),
            duration: "1.4s".into(),
            status: RunStepStatus::Pending,
            calls: vec![
                RunToolCall {
                    id: "c3-write".into(),
                    app: "gdoc".into(),
                    name: "docs.write".into(),
                    summary: "Rewrote Revenue & Q4 outlook section".into(),
                    duration: "860ms".into(),
                    done: false,
                    args: "{\n  \"file\": \"board-memo-q3.md\",\n  \"section\": \"Revenue\"\n}".into(),
                    result: "{\n  \"hunks\": 9,\n  \"status\": \"draft_ready\"\n}".into(),
                },
                RunToolCall {
                    id: "c3-chart".into(),
                    app: "google drive".into(),
                    name: "sheets.export_chart".into(),
                    summary: "Exported churn-chart.png from q3-revenue.csv".into(),
                    duration: "540ms".into(),
                    done: false,
                    args: "{\n  \"source\": \"q3-revenue.csv\",\n  \"target\": \"churn-chart.png\"\n}".into(),
                    result: "{\n  \"saved\": \"churn-chart.png\",\n  \"resolution\": \"1200x680\"\n}".into(),
                },
            ],
            approval: None,
        },
        RunStep {
            id: "step-4".into(),
            title: "Send the draft to Maya".into(),
            detail: "Needs your approval first".into(),
            tool: "gmail.send".into(),
            duration: "0.5s".into(),
            status: RunStepStatus::Pending,
            calls: vec![RunToolCall {
                id: "c4-gmail".into(),
                app: "gmail".into(),
                name: "gmail.send".into(),
                summary: "Email Q3 board memo draft to Maya Lin".into(),
                duration: "520ms".into(),
                done: false,
                args: "{\n  \"to\": \"maya@acme.com\",\n  \"subject\": \"Q3 board memo – draft for review\",\n  \"attachments\": [\"board-memo-q3.md\", \"churn-chart.png\"]\n}".into(),
                result: "{\n  \"message_id\": \"msg_q3_9941\",\n  \"status\": \"sent\"\n}".into(),
            }],
            approval: Some(RunApproval {
                title: "Send Q3 board memo draft to Maya".into(),
                description: "Emails maya@acme.com with board-memo-q3.md attached".into(),
                tool: "gmail.send".into(),
                app: "gmail".into(),
                risk: ApprovalRisk::Medium,
                details: vec![
                    ("To".into(), "Maya Lin <maya@acme.com>".into()),
                    ("Subject".into(), "Q3 board memo – draft for review".into()),
                    ("Files".into(), "board-memo-q3.md, churn-chart.png".into()),
                ],
                decision: ApprovalDecision::Pending,
            }),
        },
        RunStep {
            id: "step-5".into(),
            title: "Book a review with Maya".into(),
            detail: "30 minutes before the board meeting".into(),
            tool: "calendar.create".into(),
            duration: "0.8s".into(),
            status: RunStepStatus::Pending,
            calls: vec![RunToolCall {
                id: "c5-cal".into(),
                app: "google calendar".into(),
                name: "calendar.create".into(),
                summary: "Hold 30m board memo review with Maya".into(),
                duration: "780ms".into(),
                done: false,
                args: "{\n  \"attendees\": [\"maya@acme.com\", \"alex@acme.com\"],\n  \"duration_min\": 30,\n  \"slot\": \"Thu 10:00 AM\"\n}".into(),
                result: "{\n  \"event_id\": \"cal_8820\",\n  \"status\": \"confirmed\"\n}".into(),
            }],
            approval: None,
        },
    ]
}

/// Canonical 9-hunk inline diff draft matching the Q3 board memo reference.
pub fn default_run_draft() -> RunDraft {
    RunDraft {
        label: "Board memo – Q3 · Revenue".into(),
        tokens: vec![
            DraftToken::Plain("Revenue grew ".into()),
            DraftToken::Hunk {
                index: 0,
                deleted: "18".into(),
                inserted: "21".into(),
                state: DiffHunkState::Pending,
            },
            DraftToken::Plain("% in Q3 to $4.".into()),
            DraftToken::Hunk {
                index: 1,
                deleted: "2M".into(),
                inserted: "3M".into(),
                state: DiffHunkState::Pending,
            },
            DraftToken::Plain(", ".into()),
            DraftToken::Hunk {
                index: 2,
                deleted: "mostly ".into(),
                inserted: "".into(),
                state: DiffHunkState::Pending,
            },
            DraftToken::Plain("driven by ".into()),
            DraftToken::Hunk {
                index: 3,
                deleted: "".into(),
                inserted: "six ".into(),
                state: DiffHunkState::Pending,
            },
            DraftToken::Plain("new enterprise customers. Churn ".into()),
            DraftToken::Hunk {
                index: 4,
                deleted: "stayed flat at 2.".into(),
                inserted: "fell to 1.8".into(),
                state: DiffHunkState::Pending,
            },
            DraftToken::Plain("%, and ".into()),
            DraftToken::Hunk {
                index: 5,
                deleted: "we expect ".into(),
                inserted: "".into(),
                state: DiffHunkState::Pending,
            },
            DraftToken::Plain("the pipeline ".into()),
            DraftToken::Hunk {
                index: 6,
                deleted: "to carry into".into(),
                inserted: "supports raising".into(),
                state: DiffHunkState::Pending,
            },
            DraftToken::Plain(" the Q4 ".into()),
            DraftToken::Hunk {
                index: 7,
                deleted: "without changes to the plan".into(),
                inserted: "target".into(),
                state: DiffHunkState::Pending,
            },
            DraftToken::Plain(" ".into()),
            DraftToken::Hunk {
                index: 8,
                deleted: "".into(),
                inserted: "by 5%".into(),
                state: DiffHunkState::Pending,
            },
            DraftToken::Plain(".".into()),
        ],
    }
}

/// Canonical deliverable files written and pulled by the Q3 board prep run.
pub fn default_run_files() -> Vec<RunFileArtifact> {
    vec![
        RunFileArtifact {
            name: "board-memo-q3.md".into(),
            kind: RunFileKind::Document,
            subtitle: "Markdown · Updated just now".into(),
            heading: "Board memo – Q3".into(),
            paragraphs: vec![
                "Revenue grew 21% in Q3 to $4.3M, driven by six new enterprise customers. Churn fell to 1.8%, and the pipeline supports raising the Q4 target by 5%.".into(),
                "Net dollar retention reached 118% across annual cohorts following the shared-folders rollout.".into(),
            ],
            bullets: vec![
                "Q3 net revenue: $4.30M (+21% YoY)".into(),
                "Enterprise logo churn: 1.8% (-20 bps QoQ)".into(),
                "Proposed Q4 ARR target increase: +5%".into(),
            ],
            sheet_headers: Vec::new(),
            sheet_rows: Vec::new(),
        },
        RunFileArtifact {
            name: "q3-revenue.csv".into(),
            kind: RunFileKind::Sheet,
            subtitle: "Spreadsheet · 4 rows × 4 columns".into(),
            heading: "Q3 Revenue Breakdown".into(),
            paragraphs: Vec::new(),
            bullets: Vec::new(),
            sheet_headers: vec![
                "Month".into(),
                "Net Revenue".into(),
                "New Logos".into(),
                "Logo Churn".into(),
            ],
            sheet_rows: vec![
                vec!["July".into(), "$1,310,000".into(), "1".into(), "2.0%".into()],
                vec!["August".into(), "$1,420,000".into(), "2".into(), "1.9%".into()],
                vec!["September".into(), "$1,570,000".into(), "3".into(), "1.8%".into()],
                vec!["Q3 Total".into(), "$4,300,000".into(), "6".into(), "1.8%".into()],
            ],
        },
        RunFileArtifact {
            name: "churn-chart.png".into(),
            kind: RunFileKind::Image,
            subtitle: "PNG Chart · 1200 × 680".into(),
            heading: "Enterprise Logo Churn (Q1–Q3)".into(),
            paragraphs: vec![
                "Quarterly enterprise churn trend exported from Stripe billing cohorts.".into(),
            ],
            bullets: Vec::new(),
            sheet_headers: Vec::new(),
            sheet_rows: Vec::new(),
        },
    ]
}

fn render_run_app_tile(app: &str, side: f32, cx: &App) -> AnyElement {
    if app == "gdoc" {
        let p = ProPalette::get(cx);
        let radius = (side * 0.30).clamp(5., 8.);
        return div()
            .size(px(side))
            .flex_none()
            .rounded(px(radius))
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .items_center()
            .justify_center()
            .child(pro_svg(
                "0 0 24 24",
                GDOC_TILE_SVG,
                (side * 0.62).round(),
                rgb(0x4285f4).into(),
            ))
            .into_any_element();
    }
    pro_app_tile(app, side, cx)
}

fn render_file_chip_glyph(kind: RunFileKind, side: f32, p: &ProPalette) -> AnyElement {
    match kind {
        RunFileKind::Document => pro_svg("0 0 12 12", FILE_DOC_SVG, side, p.blue),
        RunFileKind::Sheet => pro_svg("0 0 12 12", FILE_SHEET_SVG, side, p.green),
        RunFileKind::Image => pro_svg("0 0 12 12", FILE_DOC_SVG, side, p.pink),
    }
}

/// Autonomous agent run detail screen with plan progress, activity trace, approvals, and diff work area.
pub struct AgentRunScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    _shell_sub: Option<Subscription>,
    shell_dirty: bool,
    standalone: bool,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    title: SharedString,
    agent: SharedString,
    auto_run: bool,
    loading: bool,
    empty: bool,
    focus: FocusHandle,
    phase: RunPhase,
    steps: Vec<RunStep>,
    draft: RunDraft,
    files: Vec<RunFileArtifact>,
    selected_hunk: Option<usize>,
    last_selected_hunk: Cell<usize>,
    collapsed_groups: Vec<SharedString>,
    expanded_calls: Vec<SharedString>,
    copied_key: Option<(SharedString, Instant)>,
    tabs: Vec<InspectorTab>,
    active_tab: usize,
    selected_cell: (usize, usize),
    options_open: bool,
    started_at: Instant,
    elapsed_secs_frozen: Option<u32>,
    last_advanced_stage: usize,
    measured_width: Rc<Cell<f32>>,
}

impl Focusable for AgentRunScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AgentRunScreen {
    /// Construct a new [`AgentRunScreen`] with the default Q3 board update run.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let eid = ElementId::Name(id_str.clone());
        let workspace: SharedString = "Acme".into();
        let user_name: SharedString = "Alex Morgan".into();
        let user_email: SharedString = "alex@acme.ai".into();
        let this = cx.entity();
        let shell = Self::build_shell(&id_str, &this, &workspace, &user_name, &user_email, cx);
        let sub = cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => cx.emit(AgentRunEvent::Navigate(label.clone())),
            AppShellEvent::PanelOpen(_) => cx.notify(),
            _ => {}
        });

        Self {
            id: eid,
            shell,
            _shell_sub: Some(sub),
            shell_dirty: false,
            standalone: false,
            workspace,
            user_name,
            user_email,
            title: "Prepare the Q3 board update".into(),
            agent: "Board prep".into(),
            auto_run: true,
            loading: false,
            empty: false,
            focus: cx.focus_handle(),
            phase: RunPhase::Running,
            steps: default_run_steps(),
            draft: default_run_draft(),
            files: default_run_files(),
            selected_hunk: None,
            last_selected_hunk: Cell::new(0),
            collapsed_groups: Vec::new(),
            expanded_calls: Vec::new(),
            copied_key: None,
            tabs: vec![InspectorTab::NeedsInput],
            active_tab: 0,
            selected_cell: (0, 1),
            options_open: false,
            started_at: Instant::now(),
            elapsed_secs_frozen: None,
            last_advanced_stage: 0,
            measured_width: Rc::new(Cell::new(760.)),
        }
    }

    fn sidebar_groups(cx: &App) -> Vec<ShellNavGroup> {
        vec![
            ShellNavGroup::primary(vec![
                ShellNavItem::new("home", tr(cx, "Home", "首页"), ProIcon::Home),
                ShellNavItem::new("chat", tr(cx, "Chat", "对话"), ProIcon::Thread),
                ShellNavItem::new("runs", tr(cx, "Runs", "运行"), ProIcon::Play).with_count(3),
                ShellNavItem::new(
                    "analytics",
                    tr(cx, "Analytics", "分析"),
                    ProIcon::UsageChart,
                ),
                ShellNavItem::new(
                    "deliverables",
                    tr(cx, "Deliverables", "交付物"),
                    ProIcon::Folder,
                ),
            ]),
            ShellNavGroup::named(
                tr(cx, "Agents", "智能体"),
                vec![
                    ShellNavItem::new("board-prep", "Board prep", ProIcon::Activity),
                    ShellNavItem::new("revenue-ops", "Revenue ops", ProIcon::Coins),
                    ShellNavItem::new("inbox-triage", "Inbox triage", ProIcon::Mail),
                    ShellNavItem::new("workflows", "Workflows", ProIcon::GitBranch),
                ],
            ),
        ]
    }

    fn build_shell(
        id_str: &str,
        host: &Entity<Self>,
        workspace: &SharedString,
        user_name: &SharedString,
        user_email: &SharedString,
        cx: &mut Context<Self>,
    ) -> Entity<AppShell> {
        let ws = workspace.clone();
        let un = user_name.clone();
        let ue = user_email.clone();
        cx.new(|shell_cx| {
            let groups = Self::sidebar_groups(shell_cx);
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_workspace(ws)
                .with_user(un, ue)
                .with_groups(groups)
                .with_active("Runs")
                .with_side_cards(true)
                .with_work_area(true, true)
                .with_panel_width(356.);
            s.bind_page(host, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(host, |screen, w, cx| screen.render_inspector(w, cx));
            s
        })
    }

    fn sync_shell_if_dirty(&mut self, cx: &mut Context<Self>) {
        if !self.shell_dirty {
            return;
        }
        self.shell_dirty = false;
        let was_open = self.shell.read(cx).is_panel_open();
        let this = cx.entity();
        let id_str = self.id.to_string();
        let shell = Self::build_shell(
            &id_str,
            &this,
            &self.workspace,
            &self.user_name,
            &self.user_email,
            cx,
        );
        shell.update(cx, |s, cx| s.set_panel_open(was_open, cx));
        self._shell_sub = Some(
            cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
                AppShellEvent::Navigate(label) => cx.emit(AgentRunEvent::Navigate(label.clone())),
                AppShellEvent::PanelOpen(_) => cx.notify(),
                _ => {}
            }),
        );
        self.shell = shell;
    }

    /// Render without the surrounding [`AppShell`] frame.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the goal title of the run.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Override the agent name shown in the run attribution line.
    pub fn with_agent(mut self, agent: impl Into<SharedString>) -> Self {
        self.agent = agent.into();
        self
    }

    /// Override the workspace title in the [`AppShell`] sidebar.
    pub fn with_workspace(mut self, workspace: impl Into<SharedString>) -> Self {
        self.workspace = workspace.into();
        self.shell_dirty = true;
        self
    }

    /// Override the user display name and email in the [`AppShell`] sidebar.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
    ) -> Self {
        self.user_name = name.into();
        self.user_email = email.into();
        self.shell_dirty = true;
        self
    }

    /// Replace the steps of the run.
    pub fn with_steps(mut self, steps: Vec<RunStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Replace the inline diff draft shown in the right panel.
    pub fn with_draft(mut self, draft: RunDraft) -> Self {
        self.draft = draft;
        self
    }

    /// Replace the deliverable files listed in the right panel.
    pub fn with_files(mut self, files: Vec<RunFileArtifact>) -> Self {
        self.files = files;
        self
    }

    /// Enable or disable automatic step simulation on mount.
    pub fn with_auto_run(mut self, auto_run: bool) -> Self {
        self.auto_run = auto_run;
        self
    }

    /// Show skeleton placeholders while the run loads from history.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show the empty no-runs state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Update loading skeleton state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Update empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return the current [`RunPhase`] of the run.
    pub fn phase(&self) -> RunPhase {
        self.phase
    }

    /// Approve a waiting step (`step_id`, or empty string for the current waiting step).
    pub fn approve(&mut self, step_id: &str, cx: &mut Context<Self>) {
        if let Some((resolved_id, next_phase)) = approve_waiting_step(&mut self.steps, step_id) {
            self.phase = next_phase;
            self.started_at = Instant::now();
            self.last_advanced_stage = 0;
            if next_phase == RunPhase::Done {
                self.elapsed_secs_frozen = Some(self.current_elapsed_secs());
                cx.emit(AgentRunEvent::Approved(resolved_id));
                cx.emit(AgentRunEvent::Completed);
            } else {
                self.elapsed_secs_frozen = None;
                cx.emit(AgentRunEvent::Approved(resolved_id));
            }
            cx.notify();
        }
    }

    /// Deny a waiting step (`step_id`, or empty string for the current waiting step) and stop the run.
    pub fn deny(&mut self, step_id: &str, cx: &mut Context<Self>) {
        if let Some(resolved_id) = deny_waiting_step(&mut self.steps, step_id) {
            self.phase = RunPhase::Stopped;
            self.elapsed_secs_frozen = Some(self.current_elapsed_secs());
            cx.emit(AgentRunEvent::Denied(resolved_id));
            cx.notify();
        }
    }

    /// Stop the active run immediately.
    pub fn stop_run(&mut self, cx: &mut Context<Self>) {
        if matches!(self.phase, RunPhase::Done | RunPhase::Stopped) {
            return;
        }
        self.phase = RunPhase::Stopped;
        self.elapsed_secs_frozen = Some(self.current_elapsed_secs());
        cx.emit(AgentRunEvent::Stopped);
        cx.notify();
    }

    /// Restart the simulation from Step 1.
    pub fn restart_run(&mut self, cx: &mut Context<Self>) {
        self.steps = default_run_steps();
        self.phase = RunPhase::Running;
        self.started_at = Instant::now();
        self.elapsed_secs_frozen = None;
        self.last_advanced_stage = 0;
        self.options_open = false;
        cx.notify();
    }

    /// Open a deliverable file by index in its own work-area tab and reveal the panel.
    pub fn open_file_tab(&mut self, file_index: usize, cx: &mut Context<Self>) {
        if file_index >= self.files.len() {
            return;
        }
        let target = InspectorTab::File(file_index);
        if let Some(pos) = self.tabs.iter().position(|t| *t == target) {
            self.active_tab = pos;
        } else {
            self.tabs.push(target);
            self.active_tab = self.tabs.len() - 1;
        }
        let name = self.files[file_index].name.clone();
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentRunEvent::FileOpened(name));
        cx.notify();
    }

    /// Close a work-area tab by index; closing the last tab folds the work area.
    pub fn close_inspector_tab(&mut self, tab_idx: usize, cx: &mut Context<Self>) {
        if tab_idx >= self.tabs.len() {
            return;
        }
        self.tabs.remove(tab_idx);
        if self.tabs.is_empty() {
            self.tabs.push(InspectorTab::NeedsInput);
            self.active_tab = 0;
            self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        cx.notify();
    }

    fn current_elapsed_secs(&self) -> u32 {
        if let Some(frozen) = self.elapsed_secs_frozen {
            return frozen;
        }
        1 + self.started_at.elapsed().as_secs() as u32
    }

    fn tick_auto_run(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.auto_run || self.loading || self.empty || self.phase != RunPhase::Running {
            return;
        }
        let elapsed_ms = self.started_at.elapsed().as_secs_f32() * 1000.;
        let stage = (elapsed_ms / 1800.).floor() as usize;
        while self.last_advanced_stage < stage && self.phase == RunPhase::Running {
            self.last_advanced_stage += 1;
            let next = advance_run_step(&mut self.steps);
            self.phase = next;
            if next == RunPhase::Waiting {
                self.elapsed_secs_frozen = Some(self.current_elapsed_secs());
                self.active_tab = 0;
                if !self.tabs.contains(&InspectorTab::NeedsInput) {
                    self.tabs.insert(0, InspectorTab::NeedsInput);
                }
            } else if next == RunPhase::Done {
                self.elapsed_secs_frozen = Some(self.current_elapsed_secs());
                cx.emit(AgentRunEvent::Completed);
            }
        }
        if self.phase == RunPhase::Running {
            w.request_animation_frame();
        }
    }

    fn toggle_group(&mut self, step_id: &str) {
        if let Some(pos) = self
            .collapsed_groups
            .iter()
            .position(|x| x.as_ref() == step_id)
        {
            self.collapsed_groups.remove(pos);
        } else {
            self.collapsed_groups.push(step_id.to_owned().into());
        }
    }

    fn toggle_call(&mut self, call_id: &str) {
        if let Some(pos) = self
            .expanded_calls
            .iter()
            .position(|x| x.as_ref() == call_id)
        {
            self.expanded_calls.remove(pos);
        } else {
            self.expanded_calls.push(call_id.to_owned().into());
        }
    }

    fn render_call_card(
        &self,
        call: &RunToolCall,
        spin_phase: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let is_open = self.expanded_calls.iter().any(|x| x == &call.id);
        let cid = call.id.clone();

        let status_mark = if call.done {
            div()
                .size(px(14.))
                .rounded_full()
                .bg(p.blue)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(ProIcon::Check, 9., rgb(0xffffff).into()))
                .into_any_element()
        } else {
            pro_progress_ring(0.30 + 0.55 * spin_phase, 14., 1.6, p.blue, p.line_strong)
        };

        let head = activate(
            div()
                .id(pro_eid(&self.id, format!("rcall-head-{}", call.id)))
                .role(Role::Button)
                .aria_label(format!("{} — {}", call.name, call.summary))
                .tab_index(0)
                .min_h(px(42.))
                .px(px(12.))
                .flex()
                .items_center()
                .gap(px(9.))
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(render_run_app_tile(&call.app, 20., cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_baseline()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_none()
                                .font_family("monospace")
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(call.name.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(12.5))
                                .text_color(p.text)
                                .child(call.summary.clone()),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(status_mark)
                        .child(call.duration.clone()),
                )
                .child(pro_icon(
                    if is_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    12.,
                    p.muted,
                )),
            cx,
            move |this, _, cx| {
                this.toggle_call(&cid);
                cx.notify();
            },
        );

        let code_bg: Hsla = if p.dark {
            rgb(0x212123).into()
        } else {
            rgb(0xfafafa).into()
        };
        let ak: SharedString = format!("{}-a", call.id).into();
        let rk: SharedString = format!("{}-r", call.id).into();
        let args_copied = self
            .copied_key
            .as_ref()
            .is_some_and(|(k, t)| k == &ak && t.elapsed().as_millis() < 1600);
        let res_copied = self
            .copied_key
            .as_ref()
            .is_some_and(|(k, t)| k == &rk && t.elapsed().as_millis() < 1600);
        let args_txt = call.args.clone();
        let res_txt = call.result.clone();

        let detail_body = div()
            .px(px(12.))
            .pt(px(10.))
            .pb(px(12.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "ARGUMENTS", "调用参数")),
                            )
                            .child(pro_icon_btn(
                                pro_eid(&self.id, format!("cp-{ak}")),
                                tr(cx, "Copy arguments", "复制参数"),
                                if args_copied {
                                    ProIcon::Check
                                } else {
                                    ProIcon::Copy
                                },
                                24.,
                                args_copied,
                                cx,
                                move |this, _, cx| {
                                    clipboard::write(&args_txt, cx).detach();
                                    this.copied_key = Some((ak.clone(), Instant::now()));
                                    cx.notify();
                                },
                            )),
                    )
                    .child(
                        div()
                            .p(px(9.))
                            .rounded(px(8.))
                            .bg(code_bg)
                            .border_1()
                            .border_color(p.line)
                            .font_family("monospace")
                            .text_size(px(11.5))
                            .line_height(px(18.))
                            .text_color(p.text)
                            .child(call.args.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "RESULT", "返回结果")),
                            )
                            .child(pro_icon_btn(
                                pro_eid(&self.id, format!("cp-{rk}")),
                                tr(cx, "Copy result", "复制结果"),
                                if res_copied {
                                    ProIcon::Check
                                } else {
                                    ProIcon::Copy
                                },
                                24.,
                                res_copied,
                                cx,
                                move |this, _, cx| {
                                    clipboard::write(&res_txt, cx).detach();
                                    this.copied_key = Some((rk.clone(), Instant::now()));
                                    cx.notify();
                                },
                            )),
                    )
                    .child(
                        div()
                            .p(px(9.))
                            .rounded(px(8.))
                            .bg(code_bg)
                            .border_1()
                            .border_color(p.line)
                            .font_family("monospace")
                            .text_size(px(11.5))
                            .line_height(px(18.))
                            .text_color(p.text)
                            .child(call.result.clone()),
                    ),
            );

        div()
            .id(pro_eid(&self.id, format!("rcall-{}", call.id)))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(head)
            .child(pro_fold(
                pro_eid(&self.id, format!("rcall-fold-{}", call.id)),
                is_open,
                196.,
                0.,
                move || detail_body.into_any_element(),
            ))
    }

    fn render_file_viewer(&self, file_idx: usize, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let Some(file) = self.files.get(file_idx) else {
            return div();
        };

        let head = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .size(px(28.))
                    .flex_none()
                    .rounded(px(7.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(render_file_chip_glyph(file.kind, 14., &p)),
            )
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .truncate()
                            .child(file.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .truncate()
                            .child(file.subtitle.clone()),
                    ),
            );

        let content = match file.kind {
            RunFileKind::Document => div()
                .p(px(14.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .text_size(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(file.heading.clone()),
                )
                .children(file.paragraphs.iter().map(|para| {
                    div()
                        .text_size(px(12.5))
                        .line_height(px(19.))
                        .text_color(p.text)
                        .child(para.clone())
                }))
                .when(!file.bullets.is_empty(), |d| {
                    d.child(div().pt(px(2.)).flex().flex_col().gap(px(4.)).children(
                        file.bullets.iter().map(|b| {
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .text_size(px(12.5))
                                .line_height(px(18.))
                                .text_color(p.ink)
                                .child(div().text_color(p.muted).child("•"))
                                .child(div().flex_1().min_w_0().child(b.clone()))
                        }),
                    ))
                })
                .into_any_element(),
            RunFileKind::Sheet => {
                let (sel_r, sel_c) = self.selected_cell;
                let col_letter = (b'A' + (sel_c as u8).min(25)) as char;
                let cell_val = file
                    .sheet_rows
                    .get(sel_r)
                    .and_then(|r| r.get(sel_c))
                    .cloned()
                    .unwrap_or_default();

                div()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(28.))
                            .px(px(8.))
                            .border_b_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(11.5))
                            .child(
                                div()
                                    .w(px(36.))
                                    .text_color(p.muted)
                                    .font_family("monospace")
                                    .child(format!("{col_letter}{}", sel_r + 1)),
                            )
                            .child(div().text_color(p.faint).child("fx"))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_color(p.ink)
                                    .child(cell_val),
                            ),
                    )
                    .child(
                        div()
                            .h(px(24.))
                            .bg(p.subtle)
                            .border_b_1()
                            .border_color(p.line)
                            .flex()
                            .children(file.sheet_headers.iter().map(|h| {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .px(px(6.))
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(h.clone())
                            })),
                    )
                    .child({
                        let cell_sensor = pro_selection_sensor();
                        let hover_c = p.hover;
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .child(pro_pill_indicator(
                                pro_eid(&self.id, "sheet-cell-pill"),
                                format!("{sel_r}-{sel_c}"),
                                cell_sensor.clone(),
                                0.,
                                p.accent_soft,
                                Some(p.blue),
                                false,
                            ))
                            .children(file.sheet_rows.iter().enumerate().map(|(ri, row)| {
                                let is_total = ri + 1 == file.sheet_rows.len();
                                div()
                                    .h(px(26.))
                                    .border_b_1()
                                    .border_color(p.line_soft)
                                    .flex()
                                    .children(row.iter().enumerate().map(|(ci, val)| {
                                        let chosen = ri == sel_r && ci == sel_c;
                                        activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("cell-{ri}-{ci}")))
                                                .role(Role::Cell)
                                                .tab_index(0)
                                                .relative()
                                                .flex_1()
                                                .min_w_0()
                                                .h_full()
                                                .px(px(6.))
                                                .when(chosen, |d| {
                                                    d.child(pro_mark_selected(&cell_sensor))
                                                })
                                                .when(!chosen, |d| d.hover(move |s| s.bg(hover_c)))
                                                .flex()
                                                .items_center()
                                                .text_size(px(11.5))
                                                .font_weight(if is_total {
                                                    FontWeight::MEDIUM
                                                } else {
                                                    FontWeight::NORMAL
                                                })
                                                .text_color(p.ink)
                                                .cursor_pointer()
                                                .truncate()
                                                .child(val.clone()),
                                            cx,
                                            move |this, _, cx| {
                                                this.selected_cell = (ri, ci);
                                                cx.notify();
                                            },
                                        )
                                    }))
                            }))
                    })
                    .child(
                        div()
                            .h(px(26.))
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(12.))
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child("Sum $4.30M")
                            .child("Avg $1.43M")
                            .child("Count 4"),
                    )
                    .into_any_element()
            }
            RunFileKind::Image => div()
                .p(px(14.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(file.heading.clone()),
                )
                .child(pro_trend_canvas(
                    vec![2.4, 2.3, 2.2, 2.1, 2.0, 2.0, 1.9, 1.8],
                    Some(vec![2.2, 2.2, 2.1, 2.1, 2.0, 2.0, 2.0, 2.0]),
                    p.pink,
                    0.18,
                    1.0,
                    118.,
                ))
                .children(file.paragraphs.iter().map(|para| {
                    div()
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.muted)
                        .child(para.clone())
                }))
                .into_any_element(),
        };

        div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(head)
            .child(content)
    }

    fn render_draft_card(&self, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let pending_n = self.draft.pending_count();
        let frame_del: Hsla = if p.dark {
            hsla(4. / 360., 1.0, 0.69, 0.16)
        } else {
            hsla(4. / 360., 0.86, 0.58, 0.10)
        };
        let frame_add: Hsla = if p.dark {
            hsla(145. / 360., 0.46, 0.65, 0.16)
        } else {
            hsla(145. / 360., 0.48, 0.58, 0.13)
        };

        // Build inline word-wrapped diff tokens so deletions and insertions flow naturally
        let mut flow = div().flex().flex_wrap().items_baseline().gap(px(2.));
        for (ti, tok) in self.draft.tokens.iter().enumerate() {
            match tok {
                DraftToken::Plain(txt) => {
                    for (wi, word) in txt.split_inclusive(' ').enumerate() {
                        flow = flow.child(
                            div()
                                .id(pro_eid(&self.id, format!("dt-{ti}-{wi}")))
                                .text_size(px(12.5))
                                .line_height(px(21.))
                                .text_color(p.ink)
                                .child(word.to_owned()),
                        );
                    }
                }
                DraftToken::Hunk {
                    index,
                    deleted,
                    inserted,
                    state,
                } => {
                    let h_idx = *index;
                    let is_sel = self.selected_hunk == Some(h_idx);
                    match state {
                        DiffHunkState::Accepted => {
                            if !inserted.is_empty() {
                                flow = flow.child(
                                    div()
                                        .text_size(px(12.5))
                                        .line_height(px(21.))
                                        .text_color(p.ink)
                                        .child(inserted.clone()),
                                );
                            }
                        }
                        DiffHunkState::Rejected => {
                            if !deleted.is_empty() {
                                flow = flow.child(
                                    div()
                                        .text_size(px(12.5))
                                        .line_height(px(21.))
                                        .text_color(p.ink)
                                        .child(deleted.clone()),
                                );
                            }
                        }
                        DiffHunkState::Pending => {
                            flow = flow.child(activate(
                                div()
                                    .id(pro_eid(&self.id, format!("hunk-{h_idx}")))
                                    .role(Role::Button)
                                    .aria_label(format!("Review suggestion {}", h_idx + 1))
                                    .tab_index(0)
                                    .rounded(px(4.))
                                    .when(is_sel, |d| d.border_1().border_color(p.focus))
                                    .flex()
                                    .items_baseline()
                                    .gap(px(3.))
                                    .cursor_pointer()
                                    .when(!deleted.is_empty(), |d| {
                                        d.child(
                                            div()
                                                .px(px(3.))
                                                .rounded(px(4.))
                                                .bg(frame_del)
                                                .text_size(px(12.5))
                                                .line_height(px(20.))
                                                .text_color(p.red_ink)
                                                .line_through()
                                                .child(deleted.clone()),
                                        )
                                    })
                                    .when(!inserted.is_empty(), |d| {
                                        d.child(
                                            div()
                                                .px(px(3.))
                                                .rounded(px(4.))
                                                .bg(frame_add)
                                                .text_size(px(12.5))
                                                .line_height(px(20.))
                                                .text_color(p.ink)
                                                .child(inserted.clone()),
                                        )
                                    }),
                                cx,
                                move |this, _, cx| {
                                    this.selected_hunk = if this.selected_hunk == Some(h_idx) {
                                        None
                                    } else {
                                        Some(h_idx)
                                    };
                                    cx.notify();
                                },
                            ));
                        }
                    }
                }
            }
        }

        let sel_open = self.selected_hunk.is_some_and(|h_idx| {
            let still_pending = self.draft.tokens.iter().any(|t| {
                matches!(
                    t,
                    DraftToken::Hunk {
                        index,
                        state: DiffHunkState::Pending,
                        ..
                    } if *index == h_idx
                )
            });
            if still_pending {
                self.last_selected_hunk.set(h_idx);
            }
            still_pending
        });
        let bar_h_idx = self.last_selected_hunk.get();
        let sel_bar_inner = div().pt(px(10.)).child(pro_view_transition(
            pro_eid(&self.id, "hunk-bar-vtx"),
            format!("hunk-{bar_h_idx}"),
            div()
                .p(px(5.))
                .rounded(px(8.))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .child(
                    div()
                        .pl(px(4.))
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(format!("{} #{}", tr(cx, "Change", "修改"), bar_h_idx + 1)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(pro_btn(
                            pro_eid(&self.id, format!("hunk-rej-{bar_h_idx}")),
                            tr(cx, "Reject (N)", "拒绝 (N)"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.draft.reject_hunk(bar_h_idx);
                                this.selected_hunk = None;
                                cx.notify();
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, format!("hunk-acc-{bar_h_idx}")),
                            tr(cx, "Accept (Y)", "接受 (Y)"),
                            Some(ProIcon::Check),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.draft.accept_hunk(bar_h_idx);
                                this.selected_hunk = None;
                                cx.notify();
                            },
                        )),
                ),
        ));
        let sel_bar = pro_fold(
            pro_eid(&self.id, "hunk-bar-fold"),
            sel_open,
            48.,
            0.,
            move || sel_bar_inner.into_any_element(),
        );

        let foot = if pending_n > 0 {
            div()
                .min_h(px(44.))
                .pl(px(14.))
                .pr(px(8.))
                .py(px(8.))
                .border_t_1()
                .border_color(p.line)
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
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(pending_n.to_string()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(&self.id, "dfc-reject-all"),
                            tr(cx, "Reject all", "全部拒绝"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, _, cx| {
                                this.draft.reject_all();
                                this.selected_hunk = None;
                                cx.notify();
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, "dfc-accept-all"),
                            tr(cx, "Accept all", "全部接受"),
                            None,
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            |this, _, cx| {
                                this.draft.accept_all();
                                this.selected_hunk = None;
                                cx.notify();
                            },
                        )),
                )
        } else {
            div()
                .min_h(px(44.))
                .pl(px(14.))
                .pr(px(8.))
                .py(px(8.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.green_ink)
                        .child(pro_icon(ProIcon::Check, 14., p.green_ink))
                        .child(tr(cx, "All changes reviewed", "已审阅全部修改")),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "dfc-undo"),
                    tr(cx, "Undo", "撤销"),
                    Some(ProIcon::Undo),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.draft.reset_all();
                        cx.notify();
                    },
                ))
        };

        div()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(14.))
                    .pb(px(16.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .mb(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(p.text)
                            .child(pro_svg("0 0 16 16", DIFF_SPARK_SVG, 14., p.blue))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .child(self.draft.label.clone()),
                            ),
                    )
                    .child(flow)
                    .child(sel_bar),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "dfc-foot-vtx"),
                if pending_n > 0 { "pending" } else { "done" },
                foot,
            ))
    }

    fn render_inspector(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        if self.loading && !reduced {
            w.request_animation_frame();
        }
        let skel_phase = if reduced {
            0.3
        } else {
            (self.started_at.elapsed().as_secs_f32() * 0.85).fract()
        };

        let waiting_step = self.steps.iter().enumerate().find(|(_, s)| {
            s.status == RunStepStatus::Waiting
                || s.approval
                    .as_ref()
                    .is_some_and(|a| a.decision != ApprovalDecision::Pending)
        });
        let pending_approvals = self
            .steps
            .iter()
            .filter(|s| {
                s.status == RunStepStatus::Waiting
                    && s.approval
                        .as_ref()
                        .is_some_and(|a| a.decision == ApprovalDecision::Pending)
            })
            .count();

        let active_idx = self.active_tab.min(self.tabs.len().saturating_sub(1));
        let current_tab = self
            .tabs
            .get(active_idx)
            .cloned()
            .unwrap_or(InspectorTab::NeedsInput);

        let (panel_title, panel_icon): (SharedString, ProIcon) = match &current_tab {
            InspectorTab::NeedsInput => {
                let base = tr(cx, "Needs your input", "需要你确认");
                let lbl = if pending_approvals > 0 {
                    format!("{base} · {pending_approvals}").into()
                } else {
                    base.into()
                };
                (lbl, ProIcon::Lock)
            }
            InspectorTab::File(fi) => {
                let name = self
                    .files
                    .get(*fi)
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| "Artifact".into());
                let ic = match self.files.get(*fi).map(|f| f.kind) {
                    Some(RunFileKind::Sheet) => ProIcon::ClipboardDoc,
                    Some(RunFileKind::Image) => ProIcon::Eye,
                    _ => ProIcon::PageDoc,
                };
                (name, ic)
            }
        };

        // Multi-tab strip when files have been opened
        let tab_bar = (self.tabs.len() > 1).then(|| {
            let tab_sensor = pro_selection_sensor();
            let hover_c = p.hover;
            let mut strip = div()
                .id(pro_eid(&self.id, "itab-strip"))
                .role(Role::TabList)
                .relative()
                .pb(px(10.))
                .mb(px(12.))
                .border_b_1()
                .border_color(p.line_soft)
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(4.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "itab-pill"),
                    format!("itab-{active_idx}"),
                    tab_sensor.clone(),
                    6.,
                    p.chip,
                    None,
                    false,
                ));
            for (ti, tab) in self.tabs.iter().enumerate() {
                let selected = ti == active_idx;
                let t_label: SharedString = match tab {
                    InspectorTab::NeedsInput => tr(cx, "Needs your input", "需要你确认").into(),
                    InspectorTab::File(fi) => self
                        .files
                        .get(*fi)
                        .map(|f| f.name.clone())
                        .unwrap_or_else(|| "File".into()),
                };
                strip = strip.child(
                    div()
                        .relative()
                        .h(px(26.))
                        .pl(px(8.))
                        .pr(px(4.))
                        .rounded(px(6.))
                        .when(selected, |d| d.child(pro_mark_selected(&tab_sensor)))
                        .when(!selected, |d| d.hover(move |s| s.bg(hover_c)))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("itab-{ti}")))
                                .role(Role::Tab)
                                .tab_index(0)
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .text_size(px(12.))
                                .font_weight(if selected {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if selected { p.ink } else { p.muted })
                                .cursor_pointer()
                                .child(t_label)
                                .when(
                                    matches!(tab, InspectorTab::NeedsInput)
                                        && pending_approvals > 0,
                                    |d| {
                                        d.child(
                                            div()
                                                .min_w(px(16.))
                                                .h(px(16.))
                                                .px(px(4.))
                                                .rounded_full()
                                                .bg(rgb(0xd97506))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_size(px(10.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(rgb(0xffffff))
                                                .child(pending_approvals.to_string()),
                                        )
                                    },
                                ),
                            cx,
                            move |this, _, cx| {
                                this.active_tab = ti;
                                cx.notify();
                            },
                        ))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("itab-x-{ti}")))
                                .role(Role::Button)
                                .aria_label("Close tab")
                                .tab_index(0)
                                .size(px(16.))
                                .rounded(px(4.))
                                .hover(move |s| s.bg(p.hover))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .child(pro_icon(ProIcon::Close, 9., p.faint)),
                            cx,
                            move |this, cx_w, cx| {
                                let _ = cx_w;
                                this.close_inspector_tab(ti, cx);
                            },
                        )),
                );
            }
            strip
        });

        let frame_bg: Hsla = hsla(
            0.,
            0.,
            if p.dark { 1. } else { 0. },
            if p.dark { 0.03 } else { 0.02 },
        );

        let body_el = if self.loading {
            div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(
                    div()
                        .p(px(14.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(pro_skeleton_bar(96., 12., 4., skel_phase, cx))
                                .child(pro_skeleton_bar(62., 18., 5., skel_phase, cx)),
                        )
                        .child(pro_skeleton_bar(210., 14., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(260., 56., 8., skel_phase, cx)),
                )
                .child(
                    div()
                        .p(px(14.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(140., 12., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(260., 12., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(246., 12., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(218., 12., 4., skel_phase, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(6.))
                        .child(pro_skeleton_bar(112., 26., 6., skel_phase, cx))
                        .child(pro_skeleton_bar(98., 26., 6., skel_phase, cx))
                        .child(pro_skeleton_bar(104., 26., 6., skel_phase, cx)),
                )
                .into_any_element()
        } else if self.empty {
            div()
                .p(px(14.))
                .child(
                    div()
                        .py(px(36.))
                        .px(px(18.))
                        .rounded(px(12.))
                        .bg(frame_bg)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .size(px(42.))
                                .rounded(px(11.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_svg("0 0 24 24", BELL_HOOD_SVG, 18., p.muted)),
                        )
                        .child(
                            div()
                                .mt(px(4.))
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Nothing needs you yet", "暂时无需操作")),
                        )
                        .child(
                            div()
                                .text_size(px(12.5))
                                .line_height(px(18.))
                                .text_color(p.muted)
                                .text_center()
                                .child(tr(
                                    cx,
                                    "The agent asks here before it sends or books anything.",
                                    "智能体在发送邮件或预约日程前会在此请求确认。",
                                )),
                        ),
                )
                .into_any_element()
        } else {
            let (tab_body_key, tab_content): (SharedString, AnyElement) =
                if let InspectorTab::File(fi) = current_tab {
                    (
                        format!("file-{fi}").into(),
                        self.render_file_viewer(fi, cx).into_any_element(),
                    )
                } else {
                    // Needs your input tab: Section 1 (Approval or Nothing needs you yet), Section 2 (Draft), Section 3 (Files)
                    let approval_key: SharedString = if let Some((_, step)) = waiting_step
                        && let Some(ask) = &step.approval
                    {
                        let dec = match ask.decision {
                            ApprovalDecision::Pending => "pending",
                            ApprovalDecision::Approved => "approved",
                            ApprovalDecision::Denied => "denied",
                        };
                        format!("ask-{}-{dec}", step.id).into()
                    } else {
                        "ask-none".into()
                    };

                    let approval_section = if let Some((s_idx, step)) = waiting_step
                        && let Some(ask) = &step.approval
                    {
                        let step_id = step.id.clone();
                        let step_id_deny = step.id.clone();
                        let is_pending = ask.decision == ApprovalDecision::Pending;
                        let (risk_bg, risk_fg, risk_lbl) = match ask.risk {
                            ApprovalRisk::Low => (p.chip, p.text, tr(cx, "Low risk", "低风险")),
                            ApprovalRisk::Medium => {
                                (p.amber_soft, p.amber_ink, tr(cx, "Medium risk", "中风险"))
                            }
                            ApprovalRisk::High => {
                                (p.red_soft, p.red_ink, tr(cx, "High risk", "高风险"))
                            }
                        };

                        div()
                            .p(px(14.))
                            .rounded(px(12.))
                            .bg(p.card)
                            .border_1()
                            .border_color(if is_pending {
                                p.amber.opacity(0.45)
                            } else {
                                p.line
                            })
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!("{} {}", tr(cx, "Step", "步骤"), s_idx + 1))
                                    .child(
                                        div()
                                            .px(px(5.))
                                            .rounded(px(5.))
                                            .bg(p.chip)
                                            .font_family("monospace")
                                            .text_size(px(11.))
                                            .line_height(px(18.))
                                            .text_color(p.text)
                                            .child(ask.tool.clone()),
                                    )
                                    .child(
                                        div()
                                            .ml_auto()
                                            .px(px(7.))
                                            .rounded(px(6.))
                                            .bg(risk_bg)
                                            .text_size(px(11.5))
                                            .line_height(px(20.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(risk_fg)
                                            .child(risk_lbl),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap(px(10.))
                                    .child(render_run_app_tile(&ask.app, 28., cx))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .line_height(px(18.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.ink)
                                                    .child(ask.title.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .line_height(px(17.))
                                                    .text_color(p.muted)
                                                    .child(ask.description.clone()),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .p(px(10.))
                                    .rounded(px(10.))
                                    .bg(frame_bg)
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .children(ask.details.iter().map(|(k, v)| {
                                        div()
                                            .flex()
                                            .items_start()
                                            .gap(px(10.))
                                            .text_size(px(12.))
                                            .line_height(px(17.))
                                            .child(
                                                div()
                                                    .w(px(64.))
                                                    .flex_none()
                                                    .text_color(p.muted)
                                                    .child(k.clone()),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .text_color(p.ink)
                                                    .child(v.clone()),
                                            )
                                    })),
                            )
                            .child(if is_pending {
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(6.))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "ask-deny"),
                                        tr(cx, "Deny", "拒绝"),
                                        None,
                                        ProBtnStyle::Secondary,
                                        true,
                                        cx,
                                        move |this, _, cx| this.deny(&step_id_deny, cx),
                                    ))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "ask-approve"),
                                        tr(cx, "Approve", "批准"),
                                        Some(ProIcon::Check),
                                        ProBtnStyle::Primary,
                                        true,
                                        cx,
                                        move |this, _, cx| this.approve(&step_id, cx),
                                    ))
                                    .into_any_element()
                            } else {
                                let approved = ask.decision == ApprovalDecision::Approved;
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(12.5))
                                    .text_color(if approved { p.green_ink } else { p.red_ink })
                                    .child(pro_icon(
                                        if approved {
                                            ProIcon::Check
                                        } else {
                                            ProIcon::Close
                                        },
                                        14.,
                                        if approved { p.green_ink } else { p.red_ink },
                                    ))
                                    .child(if approved {
                                        tr(
                                            cx,
                                            "Approved · sent to Maya Lin",
                                            "已批准 · 已发送给 Maya Lin",
                                        )
                                    } else {
                                        tr(
                                            cx,
                                            "Denied · run stopped at Step 4",
                                            "已拒绝 · 运行停在步骤 4",
                                        )
                                    })
                                    .into_any_element()
                            })
                            .into_any_element()
                    } else {
                        div()
                            .py(px(34.))
                            .px(px(18.))
                            .rounded(px(12.))
                            .bg(frame_bg)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .size(px(42.))
                                    .rounded(px(11.))
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_svg("0 0 24 24", BELL_HOOD_SVG, 18., p.muted)),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Nothing needs you yet", "暂时无需操作")),
                            )
                            .child(
                                div()
                                    .max_w(px(230.))
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .text_color(p.muted)
                                    .text_center()
                                    .child(tr(
                                        cx,
                                        "The agent asks here before it sends or books anything.",
                                        "智能体在发送邮件或预约日程前会在此请求确认。",
                                    )),
                            )
                            .into_any_element()
                    };

                    let draft_card = self.render_draft_card(cx);

                    let mut files_wrap = div().flex().flex_wrap().gap(px(6.));
                    for (fi, file) in self.files.iter().enumerate() {
                        files_wrap = files_wrap.child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("fchip-{fi}")))
                                .role(Role::Button)
                                .aria_label(file.name.clone())
                                .tab_index(0)
                                .h(px(26.))
                                .pl(px(7.))
                                .pr(px(9.))
                                .rounded(px(6.))
                                .bg(p.chip)
                                .hover(move |s| s.bg(p.chip_hover))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(p.ink)
                                .cursor_pointer()
                                .child(render_file_chip_glyph(file.kind, 12., &p))
                                .child(file.name.clone()),
                            cx,
                            move |this, _, cx| this.open_file_tab(fi, cx),
                        ));
                    }

                    (
                        "needs-input".into(),
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(20.))
                            .child(pro_view_transition(
                                pro_eid(&self.id, "approval-sec-vtx"),
                                approval_key,
                                approval_section,
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .pb(px(8.))
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "Draft", "草稿审阅")),
                                    )
                                    .child(draft_card),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .pb(px(8.))
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "Files", "相关文件")),
                                    )
                                    .child(files_wrap),
                            )
                            .into_any_element(),
                    )
                };

            div()
                .p(px(14.))
                .pb(px(18.))
                .flex()
                .flex_col()
                .when_some(tab_bar, |d, tb| d.child(tb))
                .child(pro_view_transition(
                    pro_eid(&self.id, "itab-body-vtx"),
                    tab_body_key,
                    tab_content,
                ))
                .into_any_element()
        };

        Some((panel_title, panel_icon, body_el))
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.tick_auto_run(w, cx);

        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        if self.loading && !reduced {
            w.request_animation_frame();
        }
        let now_s = self.started_at.elapsed().as_secs_f32();
        let skel_phase = if reduced { 0.3 } else { (now_s * 0.85).fract() };
        let spin_phase = if reduced { 0.5 } else { (now_s * 1.25).fract() };
        let breathe = if reduced {
            1.0
        } else {
            0.45 + 0.55 * (now_s * 2.2).sin().abs()
        };

        let measure_cell = self.measured_width.clone();
        let main_w = measure_cell.get();
        let compact = main_w < 600.;
        let wide_time = main_w >= 860.;
        let panel_open = self.standalone || self.shell.read(cx).is_panel_open();

        let is_loading = self.loading;
        let is_empty = !is_loading && self.empty;

        let total_steps = self.steps.len();
        let done_steps = self
            .steps
            .iter()
            .filter(|s| s.status == RunStepStatus::Done)
            .count();
        let total_calls: usize = self.steps.iter().map(|s| s.calls.len()).sum();
        let elapsed_s = self.current_elapsed_secs();
        let time_str = format!("{}:{:02}", elapsed_s / 60, elapsed_s % 60);

        // Top header bar
        let (pill_bg, pill_fg, pill_txt) = match self.phase {
            RunPhase::Running => (p.accent_soft, p.blue, tr(cx, "Running", "运行中")),
            RunPhase::Waiting => (
                p.amber_soft,
                if p.dark { p.amber } else { p.amber_ink },
                tr(cx, "Needs approval", "等待审批"),
            ),
            RunPhase::Done => (
                p.green_soft,
                if p.dark { p.green } else { p.green_ink },
                tr(cx, "Completed", "已完成"),
            ),
            RunPhase::Stopped => (p.chip, p.muted, tr(cx, "Stopped", "已停止")),
        };

        let top_bar = div()
            .h(px(48.))
            .pl(px(if compact { 48. } else { 20. }))
            .pr(px(12.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .when(!compact, |d| {
                        d.child(activate(
                            div()
                                .id(pro_eid(&self.id, "crumb-runs"))
                                .role(Role::Button)
                                .tab_index(0)
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_color(p.muted)
                                .hover(move |s| s.text_color(p.ink))
                                .cursor_pointer()
                                .child(pro_svg("0 0 24 24", RUNS_PLAY_SVG, 15., p.muted))
                                .child(tr(cx, "Runs", "运行")),
                            cx,
                            |_, _, cx| cx.emit(AgentRunEvent::Navigate("Runs".into())),
                        ))
                        .when(!is_empty, |b| b.child(div().text_color(p.faint).child("/")))
                    })
                    .when(is_loading, |d| {
                        d.child(pro_skeleton_bar(168., 13., 4., skel_phase, cx))
                            .child(pro_skeleton_bar(66., 20., 99., skel_phase, cx))
                    })
                    .when(!is_loading && !is_empty, |d| {
                        d.child(
                            div()
                                .min_w_0()
                                .truncate()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(self.title.clone()),
                        )
                        .child(
                            div()
                                .h(px(22.))
                                .pl(px(8.))
                                .pr(px(9.))
                                .flex_none()
                                .rounded_full()
                                .bg(pill_bg)
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(pill_fg)
                                .child(div().size(px(6.)).rounded_full().bg(pill_fg.opacity(
                                    if matches!(self.phase, RunPhase::Running | RunPhase::Waiting) {
                                        breathe
                                    } else {
                                        1.0
                                    },
                                )))
                                .when(!compact, |b| b.child(pill_txt)),
                        )
                    }),
            )
            .when(!is_empty, |d| {
                d.child(
                    div()
                        .relative()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .when(!panel_open, |t| t.mr(px(34.)))
                        .when(wide_time && !is_loading, |t| {
                            t.child(
                                div()
                                    .mr(px(6.))
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(12.5))
                                    .text_color(p.muted)
                                    .child(pro_icon(ProIcon::Clock, 13., p.muted))
                                    .child(format!(
                                        "{} · {time_str}",
                                        tr(cx, "Started 9:10 AM", "9:10 开始")
                                    )),
                            )
                        })
                        .child(if is_loading {
                            pro_skeleton_bar(78., 28., 6., skel_phase, cx).into_any_element()
                        } else if matches!(self.phase, RunPhase::Running | RunPhase::Waiting) {
                            activate(
                                div()
                                    .id(pro_eid(&self.id, "stop-run-btn"))
                                    .role(Role::Button)
                                    .aria_label(tr(cx, "Stop run", "停止运行"))
                                    .tab_index(0)
                                    .h(px(28.))
                                    .pl(px(8.))
                                    .pr(px(10.))
                                    .rounded(px(6.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .hover(move |s| s.bg(p.hover))
                                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .child(pro_svg("0 0 24 24", STOP_CIRCLE_SVG, 14., p.muted))
                                    .child(tr(cx, "Stop run", "停止运行")),
                                cx,
                                |this, _, cx| this.stop_run(cx),
                            )
                            .into_any_element()
                        } else {
                            pro_btn(
                                pro_eid(&self.id, "restart-run-btn"),
                                tr(cx, "Run again", "重新运行"),
                                Some(ProIcon::Refresh),
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, _, cx| this.restart_run(cx),
                            )
                            .into_any_element()
                        })
                        .child(pro_icon_btn(
                            pro_eid(&self.id, "run-more-btn"),
                            tr(cx, "Run options", "运行选项"),
                            ProIcon::MoreHorizontal,
                            28.,
                            self.options_open,
                            cx,
                            |this, _, cx| {
                                this.options_open = !this.options_open;
                                cx.notify();
                            },
                        ))
                        .child({
                            let opt_pop = div()
                                .id(pro_eid(&self.id, "run-options-pop"))
                                .absolute()
                                .right(px(0.))
                                .w(px(196.))
                                .p(px(5.))
                                .rounded(px(11.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .shadow_md()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, "opt-replay"))
                                        .role(Role::MenuItem)
                                        .tab_index(0)
                                        .h(px(30.))
                                        .px(px(8.))
                                        .rounded(px(7.))
                                        .hover(move |s| s.bg(p.hover))
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .child(pro_icon(ProIcon::Refresh, 14., p.muted))
                                        .child(tr(cx, "Replay run", "重放运行")),
                                    cx,
                                    |this, _, cx| this.restart_run(cx),
                                ))
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, "opt-skip-approval"))
                                        .role(Role::MenuItem)
                                        .tab_index(0)
                                        .h(px(30.))
                                        .px(px(8.))
                                        .rounded(px(7.))
                                        .hover(move |s| s.bg(p.hover))
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .child(pro_icon(ProIcon::Lock, 14., p.muted))
                                        .child(tr(cx, "Jump to approval", "跳至审批步骤")),
                                    cx,
                                    |this, _, cx| {
                                        while this.phase == RunPhase::Running {
                                            this.phase = advance_run_step(&mut this.steps);
                                        }
                                        this.options_open = false;
                                        this.active_tab = 0;
                                        this.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                                        cx.notify();
                                    },
                                ))
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, "opt-open-panel"))
                                        .role(Role::MenuItem)
                                        .tab_index(0)
                                        .h(px(30.))
                                        .px(px(8.))
                                        .rounded(px(7.))
                                        .hover(move |s| s.bg(p.hover))
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .child(pro_icon(ProIcon::SidebarRight, 14., p.muted))
                                        .child(tr(cx, "Toggle run details", "切换详情面板")),
                                    cx,
                                    |this, _, cx| {
                                        let open = this.shell.read(cx).is_panel_open();
                                        this.shell.update(cx, |s, cx| s.set_panel_open(!open, cx));
                                        this.options_open = false;
                                        cx.notify();
                                    },
                                ));
                            pro_presence(
                                pro_eid(&self.id, "run-opt-pres"),
                                self.options_open,
                                0.18,
                                0.12,
                                move |a| {
                                    opt_pop
                                        .top(px(34. - 4. * (1. - a)))
                                        .opacity(a)
                                        .into_any_element()
                                },
                            )
                        }),
                )
            });

        let frame_bg: Hsla = hsla(
            0.,
            0.,
            if p.dark { 1. } else { 0. },
            if p.dark { 0.03 } else { 0.02 },
        );

        // Scrollable center body
        let body_el = if is_loading {
            div()
                .max_w(px(720.))
                .w_full()
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(
                    div()
                        .pt(px(4.))
                        .pb(px(18.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(pro_skeleton_bar(22., 22., 7., skel_phase, cx))
                                .child(pro_skeleton_bar(220., 13., 4., skel_phase, cx)),
                        )
                        .child(pro_skeleton_bar(310., 24., 6., skel_phase, cx))
                        .child(pro_skeleton_bar(480., 4., 2., skel_phase, cx))
                        .child(div().flex().items_center().gap(px(6.)).children(
                            (0..5).map(|_| pro_skeleton_bar(20., 20., 6., skel_phase, cx)),
                        ))
                        .child(div().flex().gap(px(12.)).children((0..4).map(|_| {
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .child(pro_skeleton_bar(48., 11., 3., skel_phase, cx))
                                .child(pro_skeleton_bar(62., 16., 4., skel_phase, cx))
                        }))),
                )
                .child(
                    div()
                        .p(px(16.))
                        .rounded(px(14.))
                        .bg(frame_bg)
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(72., 13., 4., skel_phase, cx))
                        .children((0..5).map(|_| {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(pro_skeleton_bar(16., 16., 99., skel_phase, cx))
                                .child(pro_skeleton_bar(220., 13., 4., skel_phase, cx))
                        })),
                )
                .into_any_element()
        } else if is_empty {
            let cta: ProEmptyCta<Self> = (
                pro_eid(&self.id, "empty-new-run"),
                tr(cx, "New run", "新建运行").into(),
                Box::new(|_, _, cx| {
                    cx.emit(AgentRunEvent::Navigate("Chat".into()));
                }),
            );
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Play,
                    tr(cx, "No runs yet", "暂无运行记录"),
                    tr(
                        cx,
                        "Autonomous runs show their plan, tool calls and approvals here as they work.",
                        "自主任务运行时会在此实时展示执行计划、工具调用与待审批操作。",
                    ),
                    Some(cta),
                    cx,
                ))
                .into_any_element()
        } else {
            let progress_frac = if total_steps == 0 {
                0.
            } else {
                done_steps as f32 / total_steps as f32
            };
            let bar_color = match self.phase {
                RunPhase::Running => p.blue,
                RunPhase::Waiting => p.amber,
                RunPhase::Done => p.green,
                RunPhase::Stopped => p.faint,
            };

            let uses_row = div()
                .mt(px(12.))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(div().mr(px(4.)).child(tr(cx, "Uses", "使用工具")))
                .children(
                    ["stripe", "google drive", "gdoc", "gmail", "google calendar"]
                        .into_iter()
                        .map(|app| render_run_app_tile(app, 20., cx)),
                );

            let figures_data = [
                (
                    tr(cx, "Steps", "步骤"),
                    format!("{done_steps} / {total_steps}"),
                ),
                (tr(cx, "Tool calls", "工具调用"), total_calls.to_string()),
                (tr(cx, "Time", "耗时"), time_str),
                (tr(cx, "Cost", "费用"), "$0.14".to_string()),
            ];
            let figures_el = if compact {
                div()
                    .mt(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .gap(px(12.))
                            .children(figures_data[..2].iter().map(|(k, v)| {
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(div().text_size(px(12.)).text_color(p.muted).child(*k))
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(v.clone()),
                                    )
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(12.))
                            .children(figures_data[2..].iter().map(|(k, v)| {
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(div().text_size(px(12.)).text_color(p.muted).child(*k))
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(v.clone()),
                                    )
                            })),
                    )
            } else {
                div()
                    .mt(px(14.))
                    .flex()
                    .gap(px(12.))
                    .children(figures_data.iter().map(|(k, v)| {
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(div().text_size(px(12.)).text_color(p.muted).child(*k))
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(v.clone()),
                            )
                    }))
            };

            let hero = div()
                .pt(px(4.))
                .px(px(4.))
                .pb(px(18.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.5))
                        .text_color(p.muted)
                        .child(
                            div()
                                .size(px(22.))
                                .rounded(px(7.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_svg("0 0 24 24", BRAIN_AGENT_SVG, 14., p.ink)),
                        )
                        .child(self.agent.clone())
                        .child(div().text_color(p.faint).child("·"))
                        .child(format!(
                            "{} {} {}",
                            tr(cx, "started by", "由"),
                            self.user_name,
                            tr(cx, "at 9:10 AM", "于 9:10 发起")
                        )),
                )
                .child(
                    div()
                        .mt(px(10.))
                        .text_size(px(22.))
                        .line_height(px(30.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(self.title.clone()),
                )
                .child(div().mt(px(16.)).w_full().child(pro_spring_bar(
                    pro_eid(&self.id, "run-hero-bar"),
                    progress_frac,
                    4.,
                    2.,
                    p.chip,
                    bar_color,
                )))
                .child(uses_row)
                .child(figures_el);

            // Plan block
            let current_idx = self
                .steps
                .iter()
                .position(|s| matches!(s.status, RunStepStatus::Active | RunStepStatus::Waiting))
                .map(|i| i + 1)
                .unwrap_or(done_steps.max(1));

            let mut plan_rows = div().flex().flex_col();
            for (si, step) in self.steps.iter().enumerate() {
                let is_last = si + 1 == total_steps;
                let marker = match step.status {
                    RunStepStatus::Done => div()
                        .size(px(16.))
                        .rounded_full()
                        .bg(p.blue)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Check, 9., rgb(0xffffff).into()))
                        .into_any_element(),
                    RunStepStatus::Active => {
                        pro_progress_ring(0.28 + 0.56 * spin_phase, 16., 1.7, p.blue, p.line_strong)
                    }
                    RunStepStatus::Waiting => {
                        pro_progress_ring(0.65, 16., 1.7, p.amber, p.line_strong)
                    }
                    RunStepStatus::Failed => div()
                        .size(px(16.))
                        .rounded_full()
                        .bg(p.red_soft)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Close, 9., p.red))
                        .into_any_element(),
                    RunStepStatus::Pending | RunStepStatus::Skipped => div()
                        .size(px(16.))
                        .rounded_full()
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(9.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child((si + 1).to_string())
                        .into_any_element(),
                };

                let show_dur = matches!(
                    step.status,
                    RunStepStatus::Done | RunStepStatus::Active | RunStepStatus::Waiting
                );

                plan_rows = plan_rows.child(
                    div()
                        .relative()
                        .py(px(7.))
                        .flex()
                        .items_start()
                        .gap(px(10.))
                        .when(!is_last, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .left(px(7.5))
                                    .top(px(26.))
                                    .bottom(px(-6.))
                                    .w(px(1.))
                                    .bg(if step.status == RunStepStatus::Done {
                                        p.blue
                                    } else {
                                        p.line_strong
                                    }),
                            )
                        })
                        .child(div().mt(px(2.)).flex_none().child(marker))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(19.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(if step.status == RunStepStatus::Pending {
                                            p.text
                                        } else {
                                            p.ink
                                        })
                                        .child(step.title.clone()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(step.detail.clone())
                                        .child(
                                            div()
                                                .px(px(5.))
                                                .rounded(px(5.))
                                                .bg(p.chip)
                                                .font_family("monospace")
                                                .text_size(px(11.))
                                                .line_height(px(16.))
                                                .text_color(p.text)
                                                .child(step.tool.clone()),
                                        ),
                                ),
                        )
                        .when(show_dur, |d| {
                            d.child(
                                div()
                                    .flex_none()
                                    .text_size(px(12.))
                                    .text_color(if step.status == RunStepStatus::Active {
                                        p.ink
                                    } else {
                                        p.muted
                                    })
                                    .child(step.duration.clone()),
                            )
                        }),
                );
            }

            let plan_foot_label = match self.phase {
                RunPhase::Running => {
                    format!(
                        "{} · {current_idx} of {total_steps}",
                        tr(cx, "Running", "运行中")
                    )
                }
                RunPhase::Waiting => format!(
                    "{} · {current_idx} of {total_steps}",
                    tr(cx, "Waiting for approval", "等待审批")
                ),
                RunPhase::Done => format!(
                    "{} · {total_steps} of {total_steps}",
                    tr(cx, "Completed", "已完成")
                ),
                RunPhase::Stopped => format!(
                    "{} · {done_steps} of {total_steps}",
                    tr(cx, "Stopped", "已停止")
                ),
            };

            let plan_block = div()
                .p(px(if compact { 12. } else { 16. }))
                .pt(px(14.))
                .rounded(px(14.))
                .bg(frame_bg)
                .flex()
                .flex_col()
                .child(
                    div()
                        .mb(px(10.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_icon(ProIcon::Tasks, 15., p.muted))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(tr(cx, "Plan", "执行计划")),
                        )
                        .child(
                            div()
                                .ml_auto()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(format!("{total_steps} {}", tr(cx, "steps", "步"))),
                        ),
                )
                .child(plan_rows)
                .child(
                    div()
                        .mt(px(8.))
                        .pt(px(12.))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .text_size(px(12.5))
                                .text_color(p.muted)
                                .child(match self.phase {
                                    RunPhase::Running => pro_progress_ring(
                                        0.25 + 0.55 * spin_phase,
                                        14.,
                                        1.6,
                                        p.blue,
                                        p.line_strong,
                                    ),
                                    RunPhase::Waiting => {
                                        pro_progress_ring(0.65, 14., 1.6, p.amber, p.line_strong)
                                    }
                                    RunPhase::Done => div()
                                        .size(px(14.))
                                        .rounded_full()
                                        .bg(p.green)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_icon(ProIcon::Check, 9., rgb(0xffffff).into()))
                                        .into_any_element(),
                                    RunPhase::Stopped => div()
                                        .size(px(14.))
                                        .rounded_full()
                                        .bg(p.chip)
                                        .into_any_element(),
                                })
                                .child(plan_foot_label),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(format!("{elapsed_s}s")),
                        ),
                );

            // Activity feed block (steps that are non-pending)
            let active_entries: Vec<(usize, RunStep)> = self
                .steps
                .iter()
                .cloned()
                .enumerate()
                .filter(|(_, s)| s.status != RunStepStatus::Pending)
                .collect();
            let entry_len = active_entries.len();

            let mut activity_col = div().flex().flex_col();
            for (pos, (si, step)) in active_entries.into_iter().enumerate() {
                let is_last_entry = pos + 1 == entry_len;
                let step_done = step.status == RunStepStatus::Done;
                let group_open = !self.collapsed_groups.iter().any(|x| x == &step.id);
                let sid = step.id.clone();
                let n_calls = step.calls.len();
                let group_lbl = if step.status == RunStepStatus::Active {
                    tr(cx, "Using tools…", "正在调用工具…").to_string()
                } else if n_calls == 1 {
                    tr(cx, "Used 1 tool", "已调用 1 个工具").to_string()
                } else {
                    format!(
                        "{} {n_calls} {}",
                        tr(cx, "Used", "已调用"),
                        tr(cx, "tools", "个工具")
                    )
                };

                let mut stack = div().flex().items_center();
                for (ci, c) in step.calls.iter().enumerate() {
                    stack = stack.child(
                        div()
                            .when(ci > 0, |d| d.ml(px(-4.)))
                            .child(render_run_app_tile(&c.app, 20., cx)),
                    );
                }

                let mut entry = div()
                    .relative()
                    .pl(px(28.))
                    .pb(px(if is_last_entry { 12. } else { 18. }))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .when(!is_last_entry, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left(px(8.))
                                .top(px(22.))
                                .bottom(px(4.))
                                .w(px(1.))
                                .bg(p.line_strong),
                        )
                    })
                    .child(
                        div()
                            .absolute()
                            .top(px(0.))
                            .left(px(0.))
                            .size(px(17.))
                            .rounded_full()
                            .bg(if step_done { p.chip_hover } else { p.card })
                            .when(!step_done, |d| d.border_1().border_color(p.line_strong))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(10.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if step_done { p.ink } else { p.muted })
                            .child((si + 1).to_string()),
                    )
                    .child(
                        div()
                            .min_h(px(18.))
                            .flex()
                            .items_center()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(step.title.clone()),
                    );

                if !step.calls.is_empty() && step.status != RunStepStatus::Waiting {
                    let group_head = activate(
                        div()
                            .id(pro_eid(&self.id, format!("grp-head-{sid}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .min_h(px(30.))
                            .ml(px(-6.))
                            .pl(px(6.))
                            .pr(px(10.))
                            .rounded(px(8.))
                            .hover(move |s| s.bg(p.hover))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .cursor_pointer()
                            .child(stack)
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if step.status == RunStepStatus::Active {
                                        p.muted
                                    } else {
                                        p.ink
                                    })
                                    .child(group_lbl),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(step.duration.clone()),
                            )
                            .child(pro_icon(
                                if group_open {
                                    ProIcon::ChevronUp
                                } else {
                                    ProIcon::ChevronDown
                                },
                                12.,
                                p.muted,
                            )),
                        cx,
                        move |this, _, cx| {
                            this.toggle_group(&sid);
                            cx.notify();
                        },
                    );

                    let sid_fold = step.id.clone();
                    let mut list = div().flex().flex_col().gap(px(8.));
                    for c in &step.calls {
                        list = list.child(self.render_call_card(c, spin_phase, cx));
                    }
                    entry = entry.child(div().flex().child(group_head)).child(pro_fold(
                        pro_eid(&self.id, format!("grp-fold-{sid_fold}")),
                        group_open,
                        step.calls.len() as f32 * 52.,
                        8.,
                        move || list.into_any_element(),
                    ));
                }

                if let Some(ask) = &step.approval {
                    let note = match ask.decision {
                        ApprovalDecision::Pending => div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.5))
                            .text_color(if p.dark { p.amber } else { p.amber_ink })
                            .child(pro_icon(
                                ProIcon::Lock,
                                14.,
                                if p.dark { p.amber } else { p.amber_ink },
                            ))
                            .child(tr(cx, "Waiting on your approval", "等待你批准该操作"))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "focus-approval-link"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .text_color(p.ink)
                                    .font_weight(FontWeight::MEDIUM)
                                    .underline()
                                    .cursor_pointer()
                                    .child(tr(cx, "Review", "前往审阅")),
                                cx,
                                |this, _, cx| {
                                    this.active_tab = 0;
                                    this.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                                    cx.notify();
                                },
                            )),
                        ApprovalDecision::Approved => div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.5))
                            .text_color(if p.dark { p.green } else { p.green_ink })
                            .child(pro_icon(
                                ProIcon::Check,
                                14.,
                                if p.dark { p.green } else { p.green_ink },
                            ))
                            .child(tr(cx, "Approved by Alex Morgan", "已由 Alex Morgan 批准")),
                        ApprovalDecision::Denied => div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.5))
                            .text_color(p.red_ink)
                            .child(pro_icon(ProIcon::Close, 14., p.red_ink))
                            .child(tr(cx, "Denied — run stopped", "已拒绝 — 运行已终止")),
                    };
                    entry = entry.child(note);
                }

                activity_col = activity_col.child(entry);
            }

            let active_loader_title = self
                .steps
                .iter()
                .find(|s| s.status == RunStepStatus::Active)
                .map(|s| s.title.clone());

            let activity_block = div()
                .p(px(if compact { 12. } else { 16. }))
                .pt(px(14.))
                .rounded(px(14.))
                .bg(frame_bg)
                .flex()
                .flex_col()
                .child(
                    div()
                        .mb(px(12.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_svg("0 0 24 24", ACTIVITY_BOX_SVG, 15., p.muted))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(tr(cx, "Activity", "活动明细")),
                        )
                        .child(
                            div()
                                .ml_auto()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(format!("{current_idx} of {total_steps}")),
                        ),
                )
                .child(activity_col)
                .child({
                    let has_loader = active_loader_title.is_some();
                    let loader_txt = active_loader_title.unwrap_or_default();
                    let loader_key = loader_txt.clone();
                    let loader_chip = div().ml(px(28.)).flex().child(pro_view_transition(
                        pro_eid(&self.id, "act-loader-vtx"),
                        loader_key,
                        div().flex().child(
                            div()
                                .h(px(26.))
                                .pl(px(7.))
                                .pr(px(10.))
                                .rounded(px(6.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text)
                                .child(pro_progress_ring(
                                    0.30 + 0.55 * spin_phase,
                                    13.,
                                    1.5,
                                    p.blue,
                                    p.line_strong,
                                ))
                                .child(loader_txt),
                        ),
                    ));
                    pro_presence(
                        pro_eid(&self.id, "act-loader-pres"),
                        has_loader,
                        0.18,
                        0.14,
                        move |a| loader_chip.opacity(a).into_any_element(),
                    )
                });

            div()
                .max_w(px(720.))
                .w_full()
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(hero)
                .child(plan_block)
                .child(activity_block)
                .into_any_element()
        };

        let body_state_key = if is_loading {
            "loading"
        } else if is_empty {
            "empty"
        } else {
            "ready"
        };
        let body_el =
            pro_view_transition(pro_eid(&self.id, "run-body-vtx"), body_state_key, body_el);

        div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                let key = ev.keystroke.key.as_str();
                if ev.keystroke.modifiers.platform && key == "enter" {
                    if this.phase == RunPhase::Waiting {
                        this.approve("", cx);
                    }
                    return;
                }
                match key {
                    "escape" => {
                        if this.options_open {
                            this.options_open = false;
                            cx.notify();
                        } else if this.selected_hunk.take().is_some() {
                            cx.notify();
                        } else if this.phase == RunPhase::Waiting {
                            this.deny("", cx);
                        }
                    }
                    "y" => {
                        if let Some(h) = this.selected_hunk.take() {
                            this.draft.accept_hunk(h);
                            cx.notify();
                        }
                    }
                    "n" => {
                        if let Some(h) = this.selected_hunk.take() {
                            this.draft.reject_hunk(h);
                            cx.notify();
                        }
                    }
                    "left" if this.tabs.len() > 1 => {
                        this.active_tab = this
                            .active_tab
                            .checked_sub(1)
                            .unwrap_or(this.tabs.len() - 1);
                        cx.notify();
                    }
                    "right" if this.tabs.len() > 1 => {
                        this.active_tab = (this.active_tab + 1) % this.tabs.len();
                        cx.notify();
                    }
                    "backspace" | "delete" if this.active_tab > 0 => {
                        let idx = this.active_tab;
                        this.close_inspector_tab(idx, cx);
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
            .child(top_bar)
            .child(
                div()
                    .id(pro_eid(&self.id, "run-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pt(px(if compact { 12. } else { 20. }))
                    .px(px(if compact { 12. } else { 32. }))
                    .pb(px(if compact { 24. } else { 40. }))
                    .child(body_el),
            )
    }
}

impl Render for AgentRunScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_shell_if_dirty(cx);
        if self.standalone {
            let p = ProPalette::get(cx);
            let page = self.render_page(w, cx);
            let show_side = self.measured_width.get() >= 760.;
            let side = if show_side {
                self.render_inspector(w, cx)
            } else {
                None
            };
            return div()
                .size_full()
                .min_h_0()
                .bg(p.panel)
                .flex()
                .child(div().flex_1().min_w_0().h_full().child(page))
                .when_some(side, |d, (title, ic, body)| {
                    d.child(
                        div()
                            .w(px(356.))
                            .h_full()
                            .flex_none()
                            .border_l_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .h(px(44.))
                                    .px(px(12.))
                                    .border_b_1()
                                    .border_color(p.line_soft)
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(pro_icon(ic, 13., p.muted))
                                    .child(title),
                            )
                            .child(
                                div()
                                    .id(pro_eid(&self.id, "standalone-panel-scroll"))
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .child(body),
                            ),
                    )
                })
                .into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApprovalDecision, RunPhase, RunStepStatus, advance_run_step, approve_waiting_step,
        default_run_draft, default_run_steps, deny_waiting_step,
    };

    #[test]
    fn diff_review_accept_reject_and_undo_reconstructs_text() {
        let mut draft = default_run_draft();
        assert_eq!(draft.total_hunks(), 9);
        assert_eq!(draft.pending_count(), 9);

        draft.accept_all();
        assert_eq!(draft.pending_count(), 0);
        assert_eq!(
            draft.committed_text(),
            "Revenue grew 21% in Q3 to $4.3M, driven by six new enterprise customers. Churn fell to 1.8%, and the pipeline supports raising the Q4 target by 5%."
        );

        draft.reset_all();
        assert_eq!(draft.pending_count(), 9);
        draft.reject_all();
        assert_eq!(draft.committed_text(), draft.original_text());
    }

    #[test]
    fn run_state_machine_pauses_at_approval_and_completes_or_stops() {
        let mut steps = default_run_steps();
        assert_eq!(steps[0].status, RunStepStatus::Active);

        // Advance Steps 1 -> 2 -> 3 -> 4 (pauses at Step 4 approval)
        assert_eq!(advance_run_step(&mut steps), RunPhase::Running);
        assert_eq!(steps[1].status, RunStepStatus::Active);
        assert_eq!(advance_run_step(&mut steps), RunPhase::Running);
        assert_eq!(steps[2].status, RunStepStatus::Active);
        assert_eq!(advance_run_step(&mut steps), RunPhase::Waiting);
        assert_eq!(steps[3].status, RunStepStatus::Waiting);

        // Branch A: Deny stops at Step 4
        let mut denied_steps = steps.clone();
        let denied_id = deny_waiting_step(&mut denied_steps, "step-4");
        assert_eq!(denied_id.as_deref(), Some("step-4"));
        assert_eq!(denied_steps[3].status, RunStepStatus::Failed);
        assert_eq!(
            denied_steps[3].approval.as_ref().map(|a| a.decision),
            Some(ApprovalDecision::Denied)
        );

        // Branch B: Approve continues to Step 5 and finishes Done
        let approved = approve_waiting_step(&mut steps, "step-4");
        assert!(matches!(approved, Some((_, RunPhase::Running))));
        assert_eq!(steps[4].status, RunStepStatus::Active);
        assert_eq!(advance_run_step(&mut steps), RunPhase::Done);
        assert!(steps.iter().all(|s| s.status == RunStepStatus::Done));
    }
}
