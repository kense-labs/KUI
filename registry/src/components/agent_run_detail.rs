//! Single-agent execution trace screen with live step waterfall, metrics, and payload inspector.
//!
//! Renders a step-by-step agent run inside [`AppShell`] with live elapsed time, cost, token
//! totals, a round-step time axis, sub-agent indentation, running bar sheen, and a synchronized
//! right-hand step inspector with copyable input/output payloads, collapsible reasoning, and
//! produced artifacts.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProPalette, pro_agent_avatar, pro_app_tile, pro_eid, pro_empty_state, pro_fold,
            pro_mark_selected, pro_pill_indicator, pro_selection_sensor, pro_skeleton_bar,
            pro_spring_bar, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        clipboard,
        control::activate,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashSet, rc::Rc};
use web_time::Instant;

const BRAIN_SVG: &str = r#"<path d="M12 4.49988C12 3.11917 10.8807 1.99988 9.5 1.99988C8.11929 1.99988 7 3.11917 7 4.49988C5.34315 4.49988 4 5.84303 4 7.49988C4 8.06854 4.15822 8.60025 4.43304 9.05338C3.04727 9.31843 2 10.5369 2 11.9999C2 13.4629 3.04727 14.6813 4.43304 14.9464C4.15822 15.3995 4 15.9312 4 16.4999C4 18.1568 5.34315 19.4999 7 19.4999C7 20.8806 8.11929 21.9999 9.5 21.9999C10.8807 21.9999 12 20.8806 12 19.4999" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12 19.4999C12 20.8806 13.1193 21.9999 14.5 21.9999C15.8807 21.9999 17 20.8806 17 19.4999C18.6569 19.4999 20 18.1568 20 16.4999C20 15.9312 19.8418 15.3995 19.567 14.9464C20.9527 14.6813 22 13.4629 22 11.9999C22 10.5369 20.9527 9.31843 19.567 9.05338C19.8418 8.60025 20 8.06854 20 7.49988C20 5.84303 18.6569 4.49988 17 4.49988C17 3.11917 15.8807 1.99988 14.5 1.99988C13.1193 1.99988 12 3.11917 12 4.49988" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M10.487 7.00085V8.97987M7 10.5012H9.05198M15.0231 10.5012H17.075M15.0231 13.4745H17.075M7 13.4745H9.05198M10.487 15.0201V16.9991M13.5125 15.0201V16.9991M13.5017 7.00085V8.97987M10.052 14.9684H14.0231C14.5753 14.9684 15.0231 14.5207 15.0231 13.9684V9.97987C15.0231 9.42759 14.5753 8.97987 14.0231 8.97987H10.052C9.49969 8.97987 9.05198 9.42759 9.05198 9.97987V13.9684C9.05198 14.5207 9.49969 14.9684 10.052 14.9684Z" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path>"#;

const STOP_SQUARE_SVG: &str = r#"<path d="M4 12C4 8.72077 4 7.08116 4.81382 5.91891C5.1149 5.48891 5.48891 5.1149 5.91891 4.81382C7.08116 4 8.72077 4 12 4C15.2792 4 16.9188 4 18.0811 4.81382C18.5111 5.1149 18.8851 5.48891 19.1862 5.91891C20 7.08116 20 8.72077 20 12C20 15.2792 20 16.9188 19.1862 18.0811C18.8851 18.5111 18.5111 18.8851 18.0811 19.1862C16.9188 20 15.2792 20 12 20C8.72077 20 7.08116 20 5.91891 19.1862C5.48891 18.8851 5.1149 18.5111 4.81382 18.0811C4 16.9188 4 15.2792 4 12Z" stroke="currentColor" stroke-width="1.9"></path>"#;

const POSTGRES_P_SVG: &str = r#"<path fill="currentColor" d="M7.2 6.5h5.1c2.3 0 3.9 1.4 3.9 3.5s-1.6 3.6-3.9 3.6H9.6v3.9H7.2zm2.4 2.1v2.9h2.5c1 0 1.7-.6 1.7-1.5s-.7-1.4-1.7-1.4z"></path>"#;

/// Execution step category within [`AgentRunDetailScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunStepKind {
    /// Model reasoning / generation call (`Thinking`).
    #[default]
    Llm,
    /// External tool or integration invocation (`Tool call`).
    Tool,
    /// Delegated sub-agent span (`Sub-agent`).
    Agent,
}

/// Input and output token counts consumed by a single [`AgentRunStep`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepTokens {
    /// Prompt / context input tokens.
    pub input: u64,
    /// Generated completion tokens.
    pub output: u64,
}

impl StepTokens {
    /// Construct a token pair with `input` and `output` counts.
    pub const fn new(input: u64, output: u64) -> Self {
        Self { input, output }
    }

    /// Combined input + output token count.
    pub const fn total(self) -> u64 {
        self.input.saturating_add(self.output)
    }
}

/// Artifact produced by a step (for example a SQL migration file or pull request).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunStepArtifact {
    /// File name or artifact title.
    pub name: SharedString,
    /// Kind identifier (`"code"`, `"doc"`, `"link"`).
    pub kind: SharedString,
    /// Short metadata caption.
    pub summary: SharedString,
}

impl RunStepArtifact {
    /// Create a step artifact descriptor.
    pub fn new(
        name: impl Into<SharedString>,
        kind: impl Into<SharedString>,
        summary: impl Into<SharedString>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
            summary: summary.into(),
        }
    }
}

/// One execution step on the [`AgentRunDetailScreen`] time axis.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentRunStep {
    /// Stable step identifier.
    pub id: SharedString,
    /// Display title shown in the waterfall row and right inspector.
    pub label: SharedString,
    /// Step category (`Llm`, `Tool`, or `Agent`).
    pub kind: RunStepKind,
    /// Start offset in seconds from the start of the run.
    pub start: f32,
    /// Duration in seconds; `None` indicates the step is still running.
    pub duration: Option<f32>,
    /// Nesting level (`0` top-level, `1` child under a sub-agent).
    pub depth: u8,
    /// Optional integration/provider name for tool calls (`"GitHub"`, `"Postgres"`).
    pub app: Option<SharedString>,
    /// Optional sub-agent seed name for `RunStepKind::Agent` rows.
    pub sub_agent: Option<SharedString>,
    /// Optional accent color override for the sub-agent pixel mark.
    pub sub_agent_color: Option<Hsla>,
    /// Whether this step failed.
    pub failed: bool,
    /// Error message displayed in the inspector when `failed` is `true`.
    pub error: Option<SharedString>,
    /// Token breakdown for this step.
    pub tokens: Option<StepTokens>,
    /// Monetary cost for this step in the run's currency.
    pub cost: Option<f64>,
    /// Optional chain-of-thought summary inspectable in a collapsible section.
    pub reasoning: Option<SharedString>,
    /// Input prompt or JSON/SQL payload passed to the step.
    pub input: Option<SharedString>,
    /// Output text, SQL, or JSON returned by the step.
    pub output: Option<SharedString>,
    /// Files or links created during this step.
    pub artifacts: Vec<RunStepArtifact>,
}

impl AgentRunStep {
    /// Construct a finished or running step starting at `start` seconds.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        kind: RunStepKind,
        start: f32,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind,
            start,
            duration: None,
            depth: 0,
            app: None,
            sub_agent: None,
            sub_agent_color: None,
            failed: false,
            error: None,
            tokens: None,
            cost: None,
            reasoning: None,
            input: None,
            output: None,
            artifacts: Vec::new(),
        }
    }

    /// Set the completed duration in seconds.
    pub fn with_duration(mut self, seconds: f32) -> Self {
        self.duration = Some(seconds);
        self
    }

    /// Set the nesting depth (`1` nests under the preceding sub-agent).
    pub fn with_depth(mut self, depth: u8) -> Self {
        self.depth = depth;
        self
    }

    /// Attach an app/provider mark (`"GitHub"`, `"Postgres"`, `"Slack"`, etc.).
    pub fn with_app(mut self, app: impl Into<SharedString>) -> Self {
        self.app = Some(app.into());
        self
    }

    /// Set the sub-agent pixel-avatar seed and tint.
    pub fn with_sub_agent(mut self, name: impl Into<SharedString>, color: Hsla) -> Self {
        self.sub_agent = Some(name.into());
        self.sub_agent_color = Some(color);
        self
    }

    /// Mark the step as failed with an error description.
    pub fn with_failed(mut self, error: impl Into<SharedString>) -> Self {
        self.failed = true;
        self.error = Some(error.into());
        self
    }

    /// Set input and output token counts.
    pub fn with_tokens(mut self, input: u64, output: u64) -> Self {
        self.tokens = Some(StepTokens::new(input, output));
        self
    }

    /// Set monetary cost.
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Attach a collapsible reasoning summary.
    pub fn with_reasoning(mut self, reasoning: impl Into<SharedString>) -> Self {
        self.reasoning = Some(reasoning.into());
        self
    }

    /// Set input and output payloads.
    pub fn with_io(mut self, input: impl Into<SharedString>, output: Option<SharedString>) -> Self {
        self.input = Some(input.into());
        self.output = output;
        self
    }

    /// Attach artifacts produced by this step.
    pub fn with_artifacts(mut self, artifacts: Vec<RunStepArtifact>) -> Self {
        self.artifacts = artifacts;
        self
    }
}

/// Complete single-agent execution record displayed by [`AgentRunDetailScreen`].
#[derive(Clone, Debug, PartialEq)]
pub struct AgentRunRecord {
    /// Run identifier (`"run_0042"`).
    pub id: SharedString,
    /// Human-readable task title (`"Migrate users table"`).
    pub title: SharedString,
    /// Agent pixel-avatar seed id.
    pub agent_id: SharedString,
    /// Agent display name (`"Builder"`).
    pub agent_name: SharedString,
    /// Agent pixel mark color.
    pub agent_color: Hsla,
    /// Model name (`"Opus 5.5"`).
    pub model: SharedString,
    /// Clock start label (`"09:09"`).
    pub started_at: SharedString,
    /// Optional trigger label (`"PR #491"`).
    pub trigger: Option<SharedString>,
    /// Currency symbol prefix (`"€"` or `"$"`).
    pub currency: SharedString,
    /// Total finished run duration in seconds when complete.
    pub total_duration: Option<f32>,
    /// Ordered execution steps.
    pub steps: Vec<AgentRunStep>,
}

impl Default for AgentRunRecord {
    fn default() -> Self {
        default_migration_run()
    }
}

/// Events emitted by [`AgentRunDetailScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentRunDetailEvent {
    /// User clicked a sidebar destination.
    Navigate(SharedString),
    /// User opened or cleared a step in the right-hand inspector.
    SelectStep(Option<SharedString>),
    /// User clicked `Stop` while the run was live.
    Stop(SharedString),
    /// User clicked `Rerun` after a run finished or was stopped.
    Rerun(SharedString),
    /// User copied an input or output block (`field` is `"input"` or `"output"`).
    CopyPayload {
        /// Step identifier.
        step_id: SharedString,
        /// Copied section key.
        field: SharedString,
    },
}

impl EventEmitter<AgentRunDetailEvent> for AgentRunDetailScreen {}

#[derive(Clone, Copy, Debug)]
struct KindVisuals {
    bg: Hsla,
    edge: Hsla,
    ink: Hsla,
    mark: Hsla,
}

fn kind_visuals(kind: RunStepKind, failed: bool, p: &ProPalette) -> KindVisuals {
    if failed {
        return if p.dark {
            KindVisuals {
                bg: hsla(5. / 360., 1.0, 0.57, 0.17),
                edge: p.red.opacity(0.40),
                ink: p.red_ink,
                mark: p.red.opacity(0.75),
            }
        } else {
            KindVisuals {
                bg: hsla(10. / 360., 1.0, 0.48, 0.095),
                edge: p.red.opacity(0.34),
                ink: p.red_ink,
                mark: p.red.opacity(0.62),
            }
        };
    }
    match kind {
        RunStepKind::Llm => {
            if p.dark {
                KindVisuals {
                    bg: p.purple.opacity(0.16),
                    edge: p.purple.opacity(0.38),
                    ink: rgb(0xd1b3f5).into(),
                    mark: p.purple.opacity(0.80),
                }
            } else {
                KindVisuals {
                    bg: p.purple.opacity(0.10),
                    edge: p.purple.opacity(0.30),
                    ink: rgb(0x8a35d6).into(),
                    mark: p.purple.opacity(0.72),
                }
            }
        }
        RunStepKind::Tool => {
            if p.dark {
                KindVisuals {
                    bg: p.cyan.opacity(0.13),
                    edge: p.cyan.opacity(0.34),
                    ink: p.cyan,
                    mark: p.cyan.opacity(0.75),
                }
            } else {
                KindVisuals {
                    bg: p.cyan.opacity(0.085),
                    edge: p.cyan.opacity(0.28),
                    ink: rgb(0x08658c).into(),
                    mark: p.cyan.opacity(0.62),
                }
            }
        }
        RunStepKind::Agent => {
            if p.dark {
                KindVisuals {
                    bg: hsla(0., 0., 1., 0.07),
                    edge: hsla(0., 0., 1., 0.14),
                    ink: hsla(0., 0., 1., 0.70),
                    mark: hsla(0., 0., 1., 0.40),
                }
            } else {
                KindVisuals {
                    bg: hsla(0., 0., 0., 0.045),
                    edge: hsla(0., 0., 0., 0.10),
                    ink: hsla(0., 0., 0., 0.61),
                    mark: hsla(0., 0., 0., 0.30),
                }
            }
        }
    }
}

/// Format seconds cleanly (`"0s"`, `"3.2s"`, `"43s"`, `"1m"`, `"1m 10s"`).
pub(crate) fn format_run_seconds(sec: f32, fractional_under_ten: bool) -> String {
    let s = sec.max(0.);
    if s < 0.05 {
        return "0s".to_string();
    }
    if s < 60.0 {
        if fractional_under_ten || (s - s.round()).abs() >= 0.05 {
            let tenths = (s * 10.0).round() / 10.0;
            if (tenths - tenths.round()).abs() < 0.04 && s >= 10.0 && !fractional_under_ten {
                return format!("{}s", tenths.round() as u32);
            }
            return format!("{tenths:.1}s");
        }
        return format!("{}s", s.round() as u32);
    }
    let total_secs = s.round() as u32;
    let mins = total_secs / 60;
    let rem = total_secs % 60;
    if rem == 0 {
        format!("{mins}m")
    } else {
        format!("{mins}m {rem}s")
    }
}

/// Format large token totals (`"46.6K"`, `"1.2M"`, `"840"`).
pub(crate) fn format_compact_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Format an integer with comma separators (`18640` -> `"18,640"`).
pub(crate) fn format_with_commas(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (digits.len() - idx).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Compute round-step axis ticks and ceiling in seconds for a trace of `span_secs`.
pub(crate) fn compute_axis_ticks(span_secs: f32, live: bool) -> (f32, Vec<f32>) {
    let target = if live {
        (span_secs * 1.60).max(10.0)
    } else {
        (span_secs * 1.08).max(10.0)
    };
    let nice_steps = [
        1.0_f32, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0,
    ];
    let step = nice_steps
        .into_iter()
        .find(|&st| (target / st) <= 7.5)
        .unwrap_or(600.0);
    let count = ((target / step).ceil() as usize).max(2);
    let max_val = step * count as f32;
    let ticks = (0..=count).map(|i| i as f32 * step).collect();
    (max_val, ticks)
}

fn default_migration_run() -> AgentRunRecord {
    AgentRunRecord {
        id: "run_0042".into(),
        title: "Migrate users table".into(),
        agent_id: "builder-opus".into(),
        agent_name: "Builder".into(),
        agent_color: rgb(0x0190fe).into(),
        model: "Opus 5.5".into(),
        started_at: "09:09".into(),
        trigger: Some("PR #491".into()),
        currency: "€".into(),
        total_duration: None,
        steps: vec![
            AgentRunStep::new("plan", "Plan the migration", RunStepKind::Llm, 0.0)
                .with_duration(3.2)
                .with_tokens(2_100, 640)
                .with_cost(0.06)
                .with_reasoning(
                    "Inspect existing foreign keys on `users`, split identity credentials into `accounts` and mutable fields into `profiles`, and keep a compatibility view.",
                )
                .with_io(
                    "Split `users` into `accounts` and `profiles` without downtime for read replicas.",
                    Some("1. Read current schema & 14 prior migrations\n2. Author 0042_split_users.sql with 5,000-row batches\n3. Open PR #491 & run staging dry-run".into()),
                ),
            AgentRunStep::new("schema", "Read the schema", RunStepKind::Tool, 3.2)
                .with_duration(1.4)
                .with_app("GitHub")
                .with_tokens(1_480, 120)
                .with_cost(0.01)
                .with_io(
                    "{\n  \"repo\": \"fieldwork/core\",\n  \"path\": \"db/schema.sql\",\n  \"ref\": \"main\"\n}",
                    Some("CREATE TABLE users (\n  id uuid PRIMARY KEY,\n  email citext UNIQUE NOT NULL,\n  password_hash text NOT NULL,\n  display_name text,\n  locale text DEFAULT 'en'\n);".into()),
                ),
            AgentRunStep::new("migrations", "Read 14 migrations", RunStepKind::Tool, 4.6)
                .with_duration(3.2)
                .with_app("GitHub")
                .with_tokens(4_210, 190)
                .with_cost(0.02)
                .with_io(
                    "{\n  \"repo\": \"fieldwork/core\",\n  \"glob\": \"db/migrations/0028_*.sql..0041_*.sql\"\n}",
                    Some("Loaded 14 files (412 lines). Latest: 0041_add_sessions_user_idx.sql".into()),
                ),
            AgentRunStep::new("write", "Write migration 0042", RunStepKind::Llm, 7.8)
                .with_duration(6.3)
                .with_tokens(18_640, 2_104)
                .with_cost(0.41)
                .with_reasoning(
                    "Copy rows in deterministic `id` keyset batches of 5,000 to bound WAL lag under 40ms, then expose `CREATE VIEW users` so legacy SELECT callers stay uninterrupted.",
                )
                .with_io(
                    "Write 0042_split_users with an up and a down step. Copy in batches of 5,000 and keep a view named users for old reads.",
                    Some("CREATE TABLE accounts (…);\nCREATE TABLE profiles (…);\nINSERT INTO accounts SELECT … LIMIT 5000;\nCREATE VIEW users AS SELECT … ;".into()),
                )
                .with_artifacts(vec![RunStepArtifact::new(
                    "0042_split_users.sql",
                    "code",
                    "84 lines · SQL migration",
                )]),
            AgentRunStep::new("open-pr", "Open pull request #491", RunStepKind::Tool, 14.1)
                .with_duration(0.9)
                .with_app("GitHub")
                .with_tokens(820, 180)
                .with_cost(0.01)
                .with_io(
                    "{\n  \"title\": \"db: split users into accounts + profiles (0042)\",\n  \"head\": \"agent/migrate-users-0042\",\n  \"base\": \"main\"\n}",
                    Some("Opened pull request #491 · https://github.com/fieldwork/core/pull/491".into()),
                )
                .with_artifacts(vec![RunStepArtifact::new(
                    "Pull request #491",
                    "link",
                    "fieldwork/core#491",
                )]),
            AgentRunStep::new("reviewer", "Reviewer checks the diff", RunStepKind::Agent, 15.0)
                .with_duration(7.4)
                .with_sub_agent("reviewer-diff", rgb(0xa651f0).into())
                .with_tokens(2_400, 510)
                .with_cost(0.05)
                .with_io(
                    "Review PR #491 migration safety, index coverage, and rollback symmetry.",
                    Some("Flagged missing index on profiles(account_id) for join lookups; unit suite green.".into()),
                ),
            AgentRunStep::new("test-1", "Run the test suite", RunStepKind::Tool, 15.4)
                .with_duration(5.2)
                .with_depth(1)
                .with_app("GitHub")
                .with_tokens(1_150, 240)
                .with_cost(0.01)
                .with_io(
                    "{\n  \"workflow\": \"ci-db.yml\",\n  \"target\": \"test_migrations\"\n}",
                    Some("148 passed, 0 failed in 4.9s".into()),
                ),
            AgentRunStep::new("write-review", "Write the review", RunStepKind::Llm, 20.6)
                .with_duration(1.6)
                .with_depth(1)
                .with_tokens(1_920, 410)
                .with_cost(0.04)
                .with_io(
                    "Summarize diff findings on PR #491 for Builder.",
                    Some("Add `CREATE INDEX CONCURRENTLY profiles_account_id_idx ON profiles(account_id)` before swapping the view.".into()),
                ),
            AgentRunStep::new("dry-run", "Dry run on staging", RunStepKind::Tool, 22.4)
                .with_duration(2.2)
                .with_app("Postgres")
                .with_failed(
                    "ERROR: canceling statement due to lock timeout on relation \"users\" (SQLSTATE 55P03) after 2000ms",
                )
                .with_tokens(640, 90)
                .with_cost(0.01)
                .with_io(
                    "{\n  \"cluster\": \"staging-eu-1\",\n  \"lock_timeout\": \"2s\",\n  \"file\": \"0042_split_users.sql\"\n}",
                    Some("ROLLBACK — blocked by analytics_hourly_rollup PID 41892".into()),
                ),
            AgentRunStep::new(
                "dry-run-retry",
                "Dry run on staging · retry",
                RunStepKind::Tool,
                24.6,
            )
            .with_duration(9.6)
            .with_app("Postgres")
            .with_tokens(1_310, 260)
            .with_cost(0.02)
            .with_io(
                "{\n  \"cluster\": \"staging-eu-1\",\n  \"lock_timeout\": \"5s\",\n  \"retry_backoff_ms\": 1500\n}",
                Some("Backfilled 184,220 rows across 37 batches (p99 batch 19ms). Sequential scan detected on profiles(account_id).".into()),
            ),
            AgentRunStep::new("add-index", "Add the missing index", RunStepKind::Llm, 34.2)
                .with_duration(2.5)
                .with_tokens(2_680, 520)
                .with_cost(0.06)
                .with_reasoning(
                    "Create `profiles_account_id_idx` concurrently prior to view replacement so staging and production joins stay index-only.",
                )
                .with_io(
                    "Append concurrent index on `profiles(account_id)` to 0042_split_users.sql.",
                    Some("CREATE INDEX CONCURRENTLY IF NOT EXISTS profiles_account_id_idx\n  ON profiles (account_id);".into()),
                ),
            AgentRunStep::new("update-pr", "Update pull request #491", RunStepKind::Tool, 36.7)
                .with_duration(0.9)
                .with_app("GitHub")
                .with_tokens(540, 110)
                .with_cost(0.01)
                .with_io(
                    "{\n  \"pr\": 491,\n  \"commit\": \"fix(db): add concurrent profiles(account_id) index\"\n}",
                    Some("Pushed commit 9c41e0b to agent/migrate-users-0042".into()),
                ),
            AgentRunStep::new("test-2", "Run the test suite again", RunStepKind::Tool, 37.6)
                .with_duration(5.4)
                .with_app("GitHub")
                .with_tokens(1_180, 210)
                .with_cost(0.01)
                .with_io(
                    "{\n  \"workflow\": \"ci-db.yml\",\n  \"commit\": \"9c41e0b\"\n}",
                    Some("149 passed, 0 failed · index scan verified on `users` view".into()),
                ),
            AgentRunStep::new("guard", "Guard checks the data rules", RunStepKind::Agent, 37.9)
                .with_sub_agent("guard-pii", rgb(0x13b3a0).into())
                .with_tokens(720, 140)
                .with_cost(0.02)
                .with_io(
                    "Verify PII column grants, audit logging, and row-level access policies on `accounts` and `profiles`.",
                    None,
                ),
            AgentRunStep::new(
                "scan-logs",
                "Scan the logs for personal data",
                RunStepKind::Tool,
                38.3,
            )
            .with_duration(3.4)
            .with_depth(1)
            .with_app("Postgres")
            .with_tokens(490, 96)
            .with_cost(0.01)
            .with_io(
                "{\n  \"query\": \"SELECT count(*) FROM pg_stat_statements WHERE query ILIKE '%email%'\"\n}",
                Some("0 unmasked PII literals emitted in staging statement logs.".into()),
            ),
            AgentRunStep::new(
                "access-rules",
                "Check the access rules",
                RunStepKind::Llm,
                41.7,
            )
            .with_depth(1)
            .with_tokens(410, 90)
            .with_cost(0.01)
            .with_reasoning(
                "Comparing role grants on `accounts` and `profiles` against `readonly_analyst` policy matrix…",
            )
            .with_io(
                "Confirm `readonly_analyst` cannot select `accounts.password_hash` through `users` view.",
                None,
            ),
        ],
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum StepFilter {
    #[default]
    All,
    Llm,
    Tool,
    Agent,
    Failed,
}

impl StepFilter {
    fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Llm => "llm",
            Self::Tool => "tool",
            Self::Agent => "agent",
            Self::Failed => "failed",
        }
    }

    fn matches(self, step: &AgentRunStep) -> bool {
        match self {
            Self::All => true,
            Self::Llm => step.kind == RunStepKind::Llm && !step.failed,
            Self::Tool => step.kind == RunStepKind::Tool && !step.failed,
            Self::Agent => step.kind == RunStepKind::Agent && !step.failed,
            Self::Failed => step.failed,
        }
    }
}

/// Interactive single-agent run waterfall and step detail screen.
pub struct AgentRunDetailScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    standalone: bool,
    run: AgentRunRecord,
    base_elapsed: Option<f32>,
    live: bool,
    stopped: bool,
    selected_step: Option<SharedString>,
    focused_idx: usize,
    step_filter: StepFilter,
    loading: bool,
    empty: bool,
    reasoning_open: HashSet<SharedString>,
    collapsed_io: HashSet<SharedString>,
    copied_key: Option<(SharedString, Instant)>,
    boot_instant: Instant,
    play_instant: Option<Instant>,
    measured_width: Rc<Cell<f32>>,
}

impl AgentRunDetailScreen {
    /// Construct an [`AgentRunDetailScreen`] pre-populated with the live migration trace.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let groups = vec![
                ShellNavGroup::primary(vec![
                    ShellNavItem::new("thread", tr(shell_cx, "Thread", "会话"), ProIcon::Thread),
                    ShellNavItem::new("inbox", tr(shell_cx, "Inbox", "收件箱"), ProIcon::Inbox)
                        .with_count(2),
                    ShellNavItem::new("agents", tr(shell_cx, "Agents", "智能体"), ProIcon::Users),
                    ShellNavItem::new("tasks", tr(shell_cx, "Tasks", "任务"), ProIcon::Tasks),
                    ShellNavItem::new("memory", tr(shell_cx, "Memory", "记忆"), ProIcon::Memory),
                    ShellNavItem::new("skills", tr(shell_cx, "Skills", "技能"), ProIcon::Sparkles),
                ]),
                ShellNavGroup::named(
                    tr(shell_cx, "Workspace", "工作区"),
                    vec![
                        ShellNavItem::new(
                            "schedules",
                            tr(shell_cx, "Schedules", "定时计划"),
                            ProIcon::Calendar,
                        ),
                        ShellNavItem::new(
                            "activity",
                            tr(shell_cx, "Activity", "活动记录"),
                            ProIcon::Activity,
                        ),
                        ShellNavItem::new(
                            "usage",
                            tr(shell_cx, "Usage", "用量统计"),
                            ProIcon::UsageChart,
                        ),
                        ShellNavItem::new(
                            "permissions",
                            tr(shell_cx, "Permissions", "权限策略"),
                            ProIcon::Shield,
                        ),
                        ShellNavItem::new(
                            "settings",
                            tr(shell_cx, "Settings", "设置"),
                            ProIcon::Settings,
                        ),
                    ],
                ),
            ];
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_workspace("Fieldwork")
                .with_user("Alex Morgan", "alex@fieldwork.co")
                .with_side_cards(false)
                .with_groups(groups)
                .with_active("agents")
                .with_work_area(true, true)
                .with_panel_width(340.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });

        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(AgentRunDetailEvent::Navigate(label.clone()));
            }
            AppShellEvent::PanelOpen(false) => {
                cx.emit(AgentRunDetailEvent::SelectStep(None));
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            focus,
            standalone: false,
            run: AgentRunRecord::default(),
            base_elapsed: Some(43.0),
            live: true,
            stopped: false,
            selected_step: Some("write".into()),
            focused_idx: 3,
            step_filter: StepFilter::All,
            loading: false,
            empty: false,
            reasoning_open: HashSet::new(),
            collapsed_io: HashSet::new(),
            copied_key: None,
            boot_instant: Instant::now(),
            play_instant: None,
            measured_width: Rc::new(Cell::new(720.)),
        }
    }

    /// Render without the surrounding [`AppShell`] frame.
    pub fn standalone(mut self) -> Self {
        self.standalone = true;
        self
    }

    /// Supply a custom [`AgentRunRecord`].
    pub fn with_run(mut self, run: AgentRunRecord) -> Self {
        self.focused_idx = 0;
        self.selected_step = run.steps.first().map(|s| s.id.clone());
        self.run = run;
        self
    }

    /// Set the elapsed seconds from the start of the run (`None` renders the run as finished).
    pub fn with_elapsed(mut self, elapsed: Option<f32>) -> Self {
        self.base_elapsed = elapsed;
        self.live = elapsed.is_some();
        self
    }

    /// Set which step opens on the right when the screen initializes.
    pub fn with_default_open(mut self, step_id: Option<SharedString>) -> Self {
        if let Some(ref target) = step_id
            && let Some(pos) = self.run.steps.iter().position(|s| &s.id == target)
        {
            self.focused_idx = pos;
        }
        self.selected_step = step_id;
        self
    }

    /// Configure skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure empty state (a run with no steps yet).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Return the currently selected step id, if any.
    pub fn selected_step_id(&self) -> Option<&SharedString> {
        self.selected_step.as_ref()
    }

    /// Return whether the run is currently live.
    pub fn is_live(&self) -> bool {
        self.live && !self.stopped && !self.loading && !self.empty
    }

    fn effective_elapsed(&self) -> Option<f32> {
        let base = self.base_elapsed?;
        if let Some(play_start) = self.play_instant {
            let replay_sec = base + play_start.elapsed().as_secs_f32() * 3.5;
            if replay_sec >= 48.0 {
                return Some(48.0);
            }
            return Some(replay_sec);
        }
        Some(base)
    }

    fn elapsed_steps(&self, now_opt: Option<f32>) -> Vec<&AgentRunStep> {
        if self.empty {
            return Vec::new();
        }
        match now_opt {
            Some(now) => self
                .run
                .steps
                .iter()
                .filter(|s| s.start <= now + 0.01)
                .collect(),
            None => self.run.steps.iter().collect(),
        }
    }

    fn visible_steps(&self, now_opt: Option<f32>) -> Vec<&AgentRunStep> {
        let flt = self.step_filter;
        self.elapsed_steps(now_opt)
            .into_iter()
            .filter(|s| flt.matches(s))
            .collect()
    }

    fn select_step_by_index(&mut self, idx: usize, open_panel: bool, cx: &mut Context<Self>) {
        let now_opt = self.effective_elapsed();
        let vis = self.visible_steps(now_opt);
        if vis.is_empty() {
            return;
        }
        let clamped = idx.min(vis.len() - 1);
        let id = vis[clamped].id.clone();
        self.focused_idx = clamped;
        if open_panel {
            self.selected_step = Some(id.clone());
            self.shell
                .update(cx, |s, s_cx| s.set_panel_open(true, s_cx));
            cx.emit(AgentRunDetailEvent::SelectStep(Some(id)));
        }
        cx.notify();
    }

    fn stop_run(&mut self, cx: &mut Context<Self>) {
        if let Some(cur) = self.effective_elapsed() {
            self.base_elapsed = Some(cur);
        }
        self.play_instant = None;
        self.live = false;
        self.stopped = true;
        cx.emit(AgentRunDetailEvent::Stop(self.run.id.clone()));
        cx.notify();
    }

    fn rerun(&mut self, cx: &mut Context<Self>) {
        self.stopped = false;
        self.live = true;
        self.base_elapsed = Some(3.5);
        self.play_instant = Some(Instant::now());
        cx.emit(AgentRunDetailEvent::Rerun(self.run.id.clone()));
        cx.notify();
    }

    fn render_step_tile(
        &self,
        step: &AgentRunStep,
        side: f32,
        step_running: bool,
        anim_ms: f32,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        if step.kind == RunStepKind::Agent {
            let seed = step.sub_agent.as_deref().unwrap_or(step.id.as_ref());
            let tint = step.sub_agent_color.unwrap_or(p.purple);
            return pro_agent_avatar(seed, tint, side, step_running, anim_ms, cx);
        }
        if step.kind == RunStepKind::Tool
            && let Some(ref app_name) = step.app
        {
            let lower = app_name.to_ascii_lowercase();
            if lower.contains("postgres") {
                let inner_side = (side * 0.68).round().clamp(11., 15.);
                return div()
                    .size(px(side))
                    .flex_none()
                    .rounded(px((side * 0.28).clamp(5., 6.)))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .size(px(inner_side))
                            .rounded(px(3.5))
                            .bg(rgb(0x336791))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg(
                                "0 0 24 24",
                                POSTGRES_P_SVG,
                                inner_side,
                                rgb(0xffffff).into(),
                            )),
                    )
                    .into_any_element();
            }
            if let Some(mark) = pro_app_artwork(app_name, (side * 0.62).round().max(11.), cx) {
                return div()
                    .size(px(side))
                    .flex_none()
                    .rounded(px((side * 0.28).clamp(5., 6.)))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(mark)
                    .into_any_element();
            }
            return pro_app_tile(app_name, side, cx);
        }

        let kv = kind_visuals(step.kind, step.failed, p);
        div()
            .size(px(side))
            .flex_none()
            .rounded(px((side * 0.28).clamp(5., 6.)))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_center()
            .child(pro_svg(
                "0 0 24 24",
                BRAIN_SVG,
                (side * 0.64).round().clamp(11., 14.),
                kv.ink,
            ))
            .into_any_element()
    }

    fn render_status_badge(label: SharedString, status: &'static str, p: &ProPalette) -> Div {
        let (bg, fg): (Hsla, Hsla) = match status {
            "running" => {
                if p.dark {
                    (hsla(207. / 360., 0.98, 0.65, 0.16), rgb(0x7cc3fe).into())
                } else {
                    (hsla(206. / 360., 0.99, 0.50, 0.12), rgb(0x0a6fc2).into())
                }
            }
            "failed" => {
                if p.dark {
                    (hsla(5. / 360., 1.0, 0.57, 0.17), p.red_ink)
                } else {
                    (hsla(10. / 360., 1.0, 0.48, 0.095), p.red_ink)
                }
            }
            "done" => {
                if p.dark {
                    (hsla(145. / 360., 0.48, 0.58, 0.15), rgb(0x7dcf9f).into())
                } else {
                    (hsla(147. / 360., 0.64, 0.33, 0.12), rgb(0x1e7a47).into())
                }
            }
            _ => (p.chip, p.muted),
        };
        div()
            .h(px(20.))
            .px(px(7.))
            .flex_none()
            .rounded(px(6.))
            .bg(bg)
            .text_color(fg)
            .text_size(px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .whitespace_nowrap()
            .child(label)
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let anim_ms = self.boot_instant.elapsed().as_secs_f32() * 1000.0;
        let skel_phase = (anim_ms / 1600.0).fract();

        // Advance replay if user clicked Rerun
        if let Some(play_start) = self.play_instant
            && self.base_elapsed.unwrap_or(0.) + play_start.elapsed().as_secs_f32() * 3.5 >= 48.0
        {
            self.play_instant = None;
            self.base_elapsed = None;
            self.live = false;
        }

        let is_live = self.is_live();
        if (is_live || self.loading) && !reduced {
            w.request_animation_frame();
        }

        let now_opt = self.effective_elapsed();
        let all_elapsed = self.elapsed_steps(now_opt);
        let vis_steps = self.visible_steps(now_opt);

        // Compute aggregate run metrics
        let max_step_end = all_elapsed.iter().fold(0.0_f32, |acc, s| {
            let end = match s.duration {
                Some(d) => s.start + d,
                None => now_opt.unwrap_or(s.start + 1.0).max(s.start),
            };
            acc.max(end)
        });
        let display_time = self.run.total_duration.or(now_opt).unwrap_or(max_step_end);
        let total_cost: f64 = all_elapsed.iter().filter_map(|s| s.cost).sum();
        let total_tokens: u64 = all_elapsed
            .iter()
            .filter_map(|s| s.tokens.map(StepTokens::total))
            .sum();
        let step_count = all_elapsed.len();
        let failed_count = all_elapsed.iter().filter(|s| s.failed).count();

        let (axis_max, axis_ticks) = compute_axis_ticks(display_time.max(max_step_end), is_live);

        let width = self.measured_width.get();
        let compact = width < 640.;
        let stacked = width < 480.;
        let label_col_w = if stacked {
            0.0
        } else if compact {
            156.0
        } else {
            248.0
        };

        let agent_color = if p.dark && self.run.agent_color == Hsla::from(rgb(0x0190fe)) {
            p.blue
        } else {
            self.run.agent_color
        };

        // Header
        let subtitle = match &self.run.trigger {
            Some(trig) => format!(
                "{} · {} · {} {} · {}",
                self.run.agent_name,
                self.run.model,
                tr(cx, "started", "始于"),
                self.run.started_at,
                trig
            ),
            None => format!(
                "{} · {} · {} {}",
                self.run.agent_name,
                self.run.model,
                tr(cx, "started", "始于"),
                self.run.started_at
            ),
        };
        // Keep exact reference subtitle when using the default PR #491 run
        let header_sub = if self.run.id.as_ref() == "run_0042" {
            format!(
                "{} · {} · {} {}",
                self.run.agent_name,
                self.run.model,
                tr(cx, "started", "始于"),
                self.run.started_at
            )
        } else {
            subtitle
        };

        let (run_badge_lbl, run_badge_key) = if self.empty {
            (SharedString::from(tr(cx, "Queued", "排队中")), "queued")
        } else if is_live {
            (SharedString::from(tr(cx, "Running", "运行中")), "running")
        } else if self.stopped {
            (SharedString::from(tr(cx, "Stopped", "已停止")), "stopped")
        } else if failed_count > 0 {
            (SharedString::from(tr(cx, "Failed", "失败")), "failed")
        } else {
            (SharedString::from(tr(cx, "Done", "已完成")), "done")
        };

        let head = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(if self.loading {
                        pro_skeleton_bar(22., 22., 6., skel_phase, cx).into_any_element()
                    } else {
                        pro_agent_avatar(&self.run.agent_id, agent_color, 22., is_live, anim_ms, cx)
                    })
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(1.))
                            .child(if self.loading {
                                pro_skeleton_bar(140., 14., 4., skel_phase, cx).into_any_element()
                            } else {
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(self.run.title.clone())
                                    .into_any_element()
                            })
                            .child(if self.loading {
                                div()
                                    .pt(px(3.))
                                    .child(pro_skeleton_bar(180., 11., 4., skel_phase, cx))
                                    .into_any_element()
                            } else {
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(header_sub)
                                    .into_any_element()
                            }),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .when(!self.loading, |d| {
                        d.child(Self::render_status_badge(run_badge_lbl, run_badge_key, &p))
                    })
                    .when(!self.loading && !self.empty, |d| {
                        if is_live {
                            let stop_lbl: SharedString = tr(cx, "Stop", "停止").into();
                            d.child(activate(
                                div()
                                    .id(pro_eid(&self.id, "btn-stop"))
                                    .role(Role::Button)
                                    .aria_label(stop_lbl.clone())
                                    .tab_index(0)
                                    .h(px(28.))
                                    .px(px(11.))
                                    .rounded(px(8.))
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
                                    .child(pro_svg("0 0 24 24", STOP_SQUARE_SVG, 13., p.muted))
                                    .child(stop_lbl),
                                cx,
                                |this, _, cx| this.stop_run(cx),
                            ))
                        } else {
                            let rerun_lbl: SharedString = tr(cx, "Rerun", "重新运行").into();
                            d.child(activate(
                                div()
                                    .id(pro_eid(&self.id, "btn-rerun"))
                                    .role(Role::Button)
                                    .aria_label(rerun_lbl.clone())
                                    .tab_index(0)
                                    .h(px(28.))
                                    .px(px(11.))
                                    .rounded(px(8.))
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
                                    .child(pro_icon(ProIcon::Refresh, 13., p.muted))
                                    .child(rerun_lbl),
                                cx,
                                |this, _, cx| this.rerun(cx),
                            ))
                        }
                    }),
            );

        // 4 Summary Metric Cards
        let steps_str = if self.empty {
            "–".to_string()
        } else if failed_count > 0 {
            format!("{step_count} · {failed_count} {}", tr(cx, "failed", "失败"))
        } else {
            step_count.to_string()
        };
        let stat_items: [(SharedString, String, bool); 4] = [
            (
                tr(cx, "Time", "耗时").into(),
                if self.empty {
                    "0s".to_string()
                } else {
                    format_run_seconds(display_time, false)
                },
                false,
            ),
            (
                tr(cx, "Cost", "费用").into(),
                if self.empty {
                    "–".to_string()
                } else {
                    format!("{}{total_cost:.2}", self.run.currency)
                },
                false,
            ),
            (
                tr(cx, "Tokens", "Token").into(),
                if self.empty {
                    "–".to_string()
                } else {
                    format_compact_tokens(total_tokens)
                },
                false,
            ),
            (
                tr(cx, "Steps", "步骤").into(),
                steps_str,
                failed_count > 0 && !self.empty,
            ),
        ];

        let is_loading = self.loading;
        let render_stat_card = |(lbl, val, is_err): (SharedString, String, bool), idx: usize| {
            div()
                .flex_1()
                .min_w_0()
                .px(px(14.))
                .py(px(12.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .child(lbl),
                )
                .child(if is_loading {
                    div()
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .child(pro_skeleton_bar(
                            54. + (idx as f32 * 11.),
                            14.,
                            4.,
                            skel_phase,
                            cx,
                        ))
                        .into_any_element()
                } else {
                    div()
                        .text_size(px(15.))
                        .line_height(px(22.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if is_err { p.red_ink } else { p.ink })
                        .truncate()
                        .child(val)
                        .into_any_element()
                })
        };

        let stats_grid = if compact {
            let [c0, c1, c2, c3] = stat_items;
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(10.))
                        .child(render_stat_card(c0, 0))
                        .child(render_stat_card(c1, 1)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(10.))
                        .child(render_stat_card(c2, 2))
                        .child(render_stat_card(c3, 3)),
                )
        } else {
            let mut row = div().w_full().flex().gap(px(10.));
            for (i, item) in stat_items.into_iter().enumerate() {
                row = row.child(render_stat_card(item, i));
            }
            row
        };

        // Interactive sliding filter & legend strip
        let flt_sensor = pro_selection_sensor();
        let active_flt_key: SharedString = self.step_filter.key().into();
        let filter_defs: [(StepFilter, &str, Option<KindVisuals>); 5] = [
            (StepFilter::All, tr(cx, "All", "全部"), None),
            (
                StepFilter::Llm,
                tr(cx, "Thinking", "思考推理"),
                Some(kind_visuals(RunStepKind::Llm, false, &p)),
            ),
            (
                StepFilter::Tool,
                tr(cx, "Tool call", "工具调用"),
                Some(kind_visuals(RunStepKind::Tool, false, &p)),
            ),
            (
                StepFilter::Agent,
                tr(cx, "Sub-agent", "子智能体"),
                Some(kind_visuals(RunStepKind::Agent, false, &p)),
            ),
            (
                StepFilter::Failed,
                tr(cx, "Failed", "失败"),
                Some(kind_visuals(RunStepKind::Tool, true, &p)),
            ),
        ];
        let mut pill_strip = div()
            .id(pro_eid(&self.id, "step-filter-tabs"))
            .role(Role::TabList)
            .relative()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(3.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "step-flt-ind"),
                active_flt_key.clone(),
                flt_sensor.clone(),
                6.,
                p.chip,
                None,
                false,
            ));
        for (flt, lbl, swatch) in filter_defs {
            let selected = self.step_filter == flt;
            pill_strip = pill_strip.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("step-flt-{}", flt.key())))
                    .role(Role::Tab)
                    .aria_label(lbl)
                    .tab_index(0)
                    .relative()
                    .h(px(26.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .hover(move |s| {
                        if selected {
                            s.text_color(p.ink)
                        } else {
                            s.bg(p.hover).text_color(p.ink)
                        }
                    })
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if selected { p.ink } else { p.muted })
                    .cursor_pointer()
                    .when(selected, |d| d.child(pro_mark_selected(&flt_sensor)))
                    .when_some(swatch, |d, kv| {
                        d.child(
                            div()
                                .w(px(10.))
                                .h(px(8.))
                                .rounded(px(2.))
                                .bg(kv.bg)
                                .border_1()
                                .border_color(kv.mark),
                        )
                    })
                    .child(lbl),
                cx,
                move |this, _, cx| {
                    if this.step_filter != flt {
                        this.step_filter = flt;
                        this.focused_idx = 0;
                        cx.notify();
                    }
                },
            ));
        }
        let legend = div().w_full().flex().justify_end().child(pill_strip);

        // Axis row
        let tick_count = axis_ticks.len();
        let mut axis_track = div().relative().flex_1().h(px(24.)).min_w_0();
        for (ti, &tv) in axis_ticks.iter().enumerate() {
            if stacked && ti % 2 == 1 && ti + 1 < tick_count {
                continue;
            }
            let frac = (tv / axis_max).clamp(0., 1.);
            let lbl = format_run_seconds(tv, false);
            let tick_el = if ti == 0 {
                div()
                    .absolute()
                    .top(px(4.))
                    .left_0()
                    .text_size(px(11.))
                    .text_color(p.faint)
                    .whitespace_nowrap()
                    .child(lbl)
            } else if ti + 1 == tick_count {
                div()
                    .absolute()
                    .top(px(4.))
                    .right_0()
                    .text_size(px(11.))
                    .text_color(p.faint)
                    .whitespace_nowrap()
                    .child(lbl)
            } else {
                div()
                    .absolute()
                    .top(px(4.))
                    .left(relative(frac))
                    .ml(px(-10.))
                    .text_size(px(11.))
                    .text_color(p.faint)
                    .whitespace_nowrap()
                    .child(lbl)
            };
            axis_track = axis_track.child(tick_el);
        }

        let axis_row = div()
            .w_full()
            .h(px(24.))
            .px(px(10.))
            .flex()
            .items_center()
            .when(!stacked, |d| {
                d.gap(px(12.)).child(div().w(px(label_col_w)).flex_none())
            })
            .child(axis_track);

        // Waterfall rows or empty/skeleton states
        let trace_body = if self.empty && !self.loading {
            div()
                .w_full()
                .py(px(24.))
                .child(pro_empty_state(
                    ProIcon::Activity,
                    tr(cx, "No steps recorded yet", "尚无执行步骤"),
                    tr(
                        cx,
                        "Steps will stream onto the timeline as soon as the agent starts thinking or calling tools.",
                        "智能体开始推理或调用工具后，执行步骤将实时呈现在时间轴上。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-start"),
                        tr(cx, "Start demo run", "开始示例运行").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.rerun(cx);
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else if self.loading {
            let skel_specs: [(f32, f32, f32, u8); 10] = [
                (118., 0.00, 0.08, 0),
                (96., 0.08, 0.05, 0),
                (126., 0.13, 0.07, 0),
                (138., 0.20, 0.14, 0),
                (110., 0.34, 0.04, 0),
                (144., 0.38, 0.16, 0),
                (108., 0.40, 0.10, 1),
                (98., 0.50, 0.05, 1),
                (114., 0.55, 0.06, 0),
                (152., 0.61, 0.18, 0),
            ];
            let mut rows = div().relative().w_full().flex().flex_col().gap(px(2.));
            for (si, (nw, s_frac, w_frac, depth)) in skel_specs.into_iter().enumerate() {
                rows = rows.child(
                    div()
                        .w_full()
                        .h(px(32.))
                        .px(px(10.))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(
                            div()
                                .w(px(label_col_w.max(120.)))
                                .flex_none()
                                .pl(px(if depth > 0 { 18. } else { 0. }))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(pro_skeleton_bar(18., 18., 5., skel_phase, cx))
                                .child(pro_skeleton_bar(
                                    nw.min((label_col_w - 32.).max(64.)),
                                    12.,
                                    4.,
                                    skel_phase + si as f32 * 0.05,
                                    cx,
                                )),
                        )
                        .child(
                            div().relative().flex_1().h_full().min_w_0().child(
                                div()
                                    .absolute()
                                    .top(px(9.))
                                    .left(relative(s_frac))
                                    .w(relative(w_frac))
                                    .h(px(14.))
                                    .rounded(px(4.))
                                    .bg(p.chip_hover.opacity(0.65)),
                            ),
                        ),
                );
            }
            rows.into_any_element()
        } else {
            // Background dashed grid lines + live blue `now` cursor
            let mut bg_track = div().relative().flex_1().h_full().min_w_0();
            for &tv in &axis_ticks {
                let frac = (tv / axis_max).clamp(0., 1.);
                bg_track = bg_track.child(
                    div()
                        .absolute()
                        .top(px(-4.))
                        .bottom_0()
                        .left(relative(frac))
                        .w(px(1.))
                        .bg(p.ink.opacity(0.055)),
                );
            }
            if is_live && let Some(now_s) = now_opt {
                let now_frac = (now_s / axis_max).clamp(0., 1.);
                let now_blue: Hsla = if p.dark {
                    rgb(0x4fb0fe).into()
                } else {
                    rgb(0x0190fe).into()
                };
                bg_track = bg_track.child(
                    div()
                        .absolute()
                        .top(px(-2.))
                        .bottom_0()
                        .left(relative(now_frac))
                        .w(px(1.5))
                        .rounded(px(1.))
                        .bg(now_blue)
                        .child(
                            div()
                                .absolute()
                                .top(px(-3.))
                                .left(px(-2.25))
                                .size(px(6.))
                                .rounded_full()
                                .bg(now_blue),
                        ),
                );
            }

            let grid_overlay = div()
                .absolute()
                .inset_0()
                .px(px(10.))
                .flex()
                .when(!stacked, |d| {
                    d.gap(px(12.)).child(div().w(px(label_col_w)).flex_none())
                })
                .child(bg_track);

            let mut step_list = div()
                .relative()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(grid_overlay);

            for (idx, step) in vis_steps.iter().enumerate() {
                let is_open = self.selected_step.as_ref() == Some(&step.id)
                    && self.shell.read(cx).is_panel_open();
                let step_running = step.duration.is_none() && is_live;
                let step_dur = step
                    .duration
                    .unwrap_or_else(|| (display_time - step.start).max(0.1));
                let start_frac = (step.start / axis_max).clamp(0., 0.99);
                let dur_frac = (step_dur / axis_max).clamp(0.008, (1.0 - start_frac).max(0.008));
                let end_frac = (start_frac + dur_frac).clamp(0., 1.);

                let kv = kind_visuals(step.kind, step.failed, &p);
                let tag_text = if step.failed {
                    tr(cx, "failed", "失败").to_string()
                } else {
                    format_run_seconds(step_dur, true)
                };
                let kind_word = match step.kind {
                    RunStepKind::Llm => tr(cx, "Thinking", "思考推理"),
                    RunStepKind::Tool => tr(cx, "Tool call", "工具调用"),
                    RunStepKind::Agent => tr(cx, "Sub-agent", "子智能体"),
                };
                let aria = match &step.app {
                    Some(app) => format!("{} – {kind_word}, {app}, {tag_text}", step.label),
                    None => format!("{} – {kind_word}, {tag_text}", step.label),
                };

                // Leading sheen highlight on live running bars
                let sheen_pos = ((anim_ms / 1500.0).fract()) * 0.85;
                let bar = div()
                    .absolute()
                    .top(px(if stacked { 3. } else { 9. }))
                    .left(relative(start_frac))
                    .w(relative(dur_frac))
                    .min_w(px(3.))
                    .h(px(14.))
                    .rounded(px(4.))
                    .overflow_hidden()
                    .bg(kv.bg)
                    .border_1()
                    .border_color(if is_open { kv.mark } else { kv.edge })
                    .when(step_running && !reduced, |d| {
                        d.child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(relative(sheen_pos))
                                .w(px(28.))
                                .bg(p.card.opacity(0.45)),
                        )
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .bottom_0()
                                .w(px(12.))
                                .bg(kv.edge),
                        )
                    });

                let place_before = end_frac > 0.86;
                let tag_el = if place_before {
                    div()
                        .absolute()
                        .top(px(if stacked { 1. } else { 7. }))
                        .right(relative((1.0 - start_frac + 0.012).clamp(0., 0.96)))
                        .h(px(18.))
                        .px(px(4.))
                        .rounded(px(4.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(if step.failed { p.red_ink } else { p.muted })
                        .whitespace_nowrap()
                        .child(tag_text)
                } else {
                    div()
                        .absolute()
                        .top(px(if stacked { 1. } else { 7. }))
                        .left(relative((end_frac + 0.012).clamp(0., 0.95)))
                        .h(px(18.))
                        .px(px(4.))
                        .rounded(px(4.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(if step.failed { p.red_ink } else { p.muted })
                        .whitespace_nowrap()
                        .child(tag_text)
                };

                let label_cell = div()
                    .relative()
                    .when(!stacked, |d| d.w(px(label_col_w)).flex_none())
                    .min_w_0()
                    .when(step.depth > 0 && !stacked, |d| {
                        let elbow_color = p.line_strong;
                        d.pl(px(18.)).child(
                            canvas(
                                |_, _, _| (),
                                move |b, _, window, _| {
                                    let x0 = b.left();
                                    let y0 = b.top();
                                    let x1 = b.right();
                                    let y1 = b.bottom();
                                    let mut pb = PathBuilder::stroke(px(1.));
                                    pb.move_to(point(x0, y0));
                                    pb.line_to(point(x0, y1 - px(4.)));
                                    pb.curve_to(point(x0 + px(4.), y1), point(x0, y1));
                                    pb.line_to(point(x1, y1));
                                    if let Ok(path) = pb.build() {
                                        window.paint_path(path, elbow_color);
                                    }
                                },
                            )
                            .absolute()
                            .left(px(7.))
                            .top(px(-8.))
                            .w(px(8.))
                            .h(px(19.)),
                        )
                    })
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(self.render_step_tile(step, 18., step_running, anim_ms, &p, cx))
                    .child(
                        div()
                            .min_w_0()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.ink)
                            .truncate()
                            .child(step.label.clone()),
                    );

                let track_cell = div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h(px(if stacked { 20. } else { 32. }))
                    .child(bar)
                    .child(tag_el);

                let row_bg = if is_open {
                    p.ink.opacity(0.04)
                } else {
                    transparent_black()
                };

                let row_btn = activate(
                    div()
                        .id(pro_eid(&self.id, format!("step-{}", step.id)))
                        .role(Role::Button)
                        .aria_label(aria)
                        .tab_index(0)
                        .w_full()
                        .px(px(10.))
                        .when(!stacked, |d| {
                            d.h(px(32.)).flex().items_center().gap(px(12.))
                        })
                        .when(stacked, |d| d.py(px(5.)).flex().flex_col().gap(px(2.)))
                        .rounded(px(8.))
                        .bg(row_bg)
                        .hover(move |s| s.bg(p.ink.opacity(0.035)))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .cursor_pointer()
                        .child(label_cell)
                        .child(track_cell),
                    cx,
                    move |this, _, cx| {
                        this.select_step_by_index(idx, true, cx);
                    },
                );

                step_list = step_list.child(row_btn);
            }

            if vis_steps.is_empty() {
                step_list = step_list.child(
                    div()
                        .py(px(24.))
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.5))
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "No steps match this filter",
                            "当前筛选条件下暂无步骤",
                        )),
                );
            }

            pro_view_transition(
                pro_eid(&self.id, "trace-flt-vtx"),
                active_flt_key,
                step_list,
            )
        };

        let measure_cell = self.measured_width.clone();
        div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .key_context("AgentRunDetailScreen")
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                let key = ev.keystroke.key.as_str();
                let vis_len = this.visible_steps(this.effective_elapsed()).len();
                if vis_len == 0 {
                    return;
                }
                match key {
                    "j" | "down" => {
                        let next = (this.focused_idx + 1).min(vis_len - 1);
                        this.select_step_by_index(next, true, cx);
                    }
                    "k" | "up" => {
                        let prev = this.focused_idx.saturating_sub(1);
                        this.select_step_by_index(prev, true, cx);
                    }
                    "home" => {
                        this.select_step_by_index(0, true, cx);
                    }
                    "end" => {
                        this.select_step_by_index(vis_len - 1, true, cx);
                    }
                    "enter" => {
                        this.select_step_by_index(this.focused_idx, true, cx);
                    }
                    "escape" => {
                        this.shell
                            .update(cx, |s, s_cx| s.set_panel_open(false, s_cx));
                        cx.emit(AgentRunDetailEvent::SelectStep(None));
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .relative()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
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
            .child(
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(if stacked { 12. } else { 20. }))
                    .pt(px(18.))
                    .pb(px(40.))
                    .flex()
                    .flex_col()
                    .gap(px(22.))
                    .child(head)
                    .child(stats_grid)
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(legend)
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_col()
                                    .child(axis_row)
                                    .child(trace_body),
                            ),
                    ),
            )
    }

    fn render_inspector(
        &mut self,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let p = ProPalette::get(cx);
        let anim_ms = self.boot_instant.elapsed().as_secs_f32() * 1000.0;
        let skel_phase = (anim_ms / 1600.0).fract();

        if self.loading {
            let skel_body = div()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(22.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(22., 22., 6., skel_phase, cx))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .child(pro_skeleton_bar(130., 13., 4., skel_phase, cx))
                                .child(pro_skeleton_bar(72., 10., 3., skel_phase, cx)),
                        )
                        .child(pro_skeleton_bar(44., 20., 6., skel_phase, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .children((0..4).map(|i| {
                            div()
                                .h(px(22.))
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .child(pro_skeleton_bar(52., 11., 3., skel_phase, cx))
                                .child(pro_skeleton_bar(
                                    80. + (i as f32 * 16.),
                                    12.,
                                    4.,
                                    skel_phase,
                                    cx,
                                ))
                        })),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(48., 11., 3., skel_phase, cx))
                        .child(pro_skeleton_bar(260., 76., 10., skel_phase, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(54., 11., 3., skel_phase, cx))
                        .child(pro_skeleton_bar(260., 92., 10., skel_phase, cx)),
                )
                .into_any_element();
            return Some((
                tr(cx, "Loading step…", "加载步骤…").into(),
                ProIcon::ClipboardDoc,
                skel_body,
            ));
        }

        if self.empty {
            return None;
        }

        let sel_id = self.selected_step.clone()?;
        let step = self.run.steps.iter().find(|s| s.id == sel_id)?.clone();

        let is_live = self.is_live();
        let now_s = self
            .effective_elapsed()
            .or(self.run.total_duration)
            .unwrap_or(43.0);
        let step_running = step.duration.is_none() && is_live;
        let step_dur = step
            .duration
            .unwrap_or_else(|| (now_s - step.start).max(0.1));

        let (badge_lbl, badge_key) = if step.failed {
            (SharedString::from(tr(cx, "Failed", "失败")), "failed")
        } else if step_running {
            (SharedString::from(tr(cx, "Running", "运行中")), "running")
        } else {
            (SharedString::from(tr(cx, "Done", "已完成")), "done")
        };

        let kind_sub = match step.kind {
            RunStepKind::Llm => tr(cx, "Thinking", "思考推理").to_string(),
            RunStepKind::Tool => match &step.app {
                Some(app) => format!("{} · {app}", tr(cx, "Tool call", "工具调用")),
                None => tr(cx, "Tool call", "工具调用").to_string(),
            },
            RunStepKind::Agent => tr(cx, "Sub-agent", "子智能体").to_string(),
        };

        let well_bg: Hsla = if p.dark {
            rgb(0x161619).into()
        } else {
            rgb(0xf6f6f7).into()
        };

        // Check whether copy confirmation is still active (< 1.8s)
        let copied_tag: Option<SharedString> = self.copied_key.as_ref().and_then(|(k, t)| {
            if t.elapsed().as_millis() < 1800 {
                Some(k.clone())
            } else {
                None
            }
        });

        let total_dur_ref = now_s.max(step_dur).max(0.1);
        let total_cost_ref: f64 = self
            .run
            .steps
            .iter()
            .filter_map(|s| s.cost)
            .sum::<f64>()
            .max(0.01);
        let step_kv = kind_visuals(step.kind, step.failed, &p);

        type PropBar = (&'static str, f32, Hsla);
        let mut props_col = div().flex().flex_col().gap(px(4.));
        let mut prop_rows: Vec<(SharedString, String, Option<PropBar>)> = vec![
            (
                tr(cx, "Started", "开始于").into(),
                format!("+{}", format_run_seconds(step.start, true)),
                None,
            ),
            (
                tr(cx, "Took", "耗时").into(),
                format_run_seconds(step_dur, true),
                Some((
                    "insp-dur-bar",
                    (step_dur / total_dur_ref).clamp(0.04, 1.0),
                    step_kv.mark,
                )),
            ),
        ];
        if let Some(tok) = step.tokens {
            prop_rows.push((
                tr(cx, "Tokens", "Token").into(),
                format!(
                    "{} {} · {} {}",
                    format_with_commas(tok.input),
                    tr(cx, "in", "输入"),
                    format_with_commas(tok.output),
                    tr(cx, "out", "输出")
                ),
                None,
            ));
        }
        if let Some(c) = step.cost {
            prop_rows.push((
                tr(cx, "Cost", "费用").into(),
                format!("{}{c:.2}", self.run.currency),
                Some((
                    "insp-cost-bar",
                    ((c / total_cost_ref) as f32).clamp(0.04, 1.0),
                    p.blue,
                )),
            ));
        }

        for (k, v, bar_spec) in prop_rows {
            props_col = props_col.child(
                div()
                    .min_h(px(24.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .child(div().w(px(68.)).flex_none().text_color(p.muted).child(k))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(p.ink)
                            .truncate()
                            .child(v),
                    )
                    .when_some(bar_spec, |d, (bar_id, ratio, fill_c)| {
                        d.child(div().w(px(68.)).flex_none().child(pro_spring_bar(
                            pro_eid(&self.id, bar_id),
                            ratio,
                            4.,
                            2.,
                            p.line_soft,
                            fill_c,
                        )))
                    }),
            );
        }

        let mut panel = div()
            .p(px(16.))
            .pb(px(28.))
            .flex()
            .flex_col()
            .gap(px(22.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(self.render_step_tile(&step, 22., step_running, anim_ms, &p, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(step.label.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(kind_sub),
                            ),
                    )
                    .child(Self::render_status_badge(badge_lbl, badge_key, &p)),
            )
            .child(props_col);

        // Failed error banner
        if step.failed
            && let Some(ref err_msg) = step.error
        {
            let kv = kind_visuals(step.kind, true, &p);
            panel = panel.child(
                div()
                    .px(px(12.))
                    .py(px(10.))
                    .rounded(px(10.))
                    .bg(kv.bg)
                    .border_1()
                    .border_color(kv.edge)
                    .text_size(px(12.5))
                    .line_height(px(19.))
                    .text_color(p.red_ink)
                    .child(err_msg.clone()),
            );
        }

        // Input section (collapsible on label click + Copy button)
        if let Some(ref input_txt) = step.input {
            let in_fold_key = SharedString::from(format!("{}-in", step.id));
            let in_folded = self.collapsed_io.contains(&in_fold_key);
            let copy_key = SharedString::from(format!("{}:input", step.id));
            let is_copied = copied_tag.as_ref() == Some(&copy_key);
            let step_id_cp = step.id.clone();
            let txt_cp = input_txt.clone();
            let txt_fold = input_txt.clone();
            let fold_toggle_key = in_fold_key.clone();

            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, format!("fold-in-{}", step.id)))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .cursor_pointer()
                                    .hover(move |s| s.text_color(p.ink))
                                    .child(tr(cx, "Input", "输入"))
                                    .child(pro_icon(
                                        if in_folded {
                                            ProIcon::ChevronRight
                                        } else {
                                            ProIcon::ChevronDown
                                        },
                                        11.,
                                        p.faint,
                                    )),
                                cx,
                                move |this, _, cx| {
                                    if !this.collapsed_io.remove(&fold_toggle_key) {
                                        this.collapsed_io.insert(fold_toggle_key.clone());
                                    }
                                    cx.notify();
                                },
                            ))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, format!("copy-in-{}", step.id)))
                                    .role(Role::Button)
                                    .aria_label(tr(cx, "Copy input", "复制输入"))
                                    .tab_index(0)
                                    .h(px(22.))
                                    .px(px(6.))
                                    .rounded(px(6.))
                                    .hover(move |s| s.bg(p.chip).text_color(p.ink))
                                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(12.))
                                    .text_color(if is_copied { p.green_ink } else { p.muted })
                                    .cursor_pointer()
                                    .child(pro_icon(
                                        if is_copied {
                                            ProIcon::Check
                                        } else {
                                            ProIcon::Copy
                                        },
                                        12.,
                                        if is_copied { p.green_ink } else { p.muted },
                                    ))
                                    .child(if is_copied {
                                        tr(cx, "Copied", "已复制")
                                    } else {
                                        tr(cx, "Copy", "复制")
                                    }),
                                cx,
                                move |this, _, cx| {
                                    clipboard::write(txt_cp.as_ref(), cx).detach();
                                    this.copied_key = Some((copy_key.clone(), Instant::now()));
                                    cx.emit(AgentRunDetailEvent::CopyPayload {
                                        step_id: step_id_cp.clone(),
                                        field: "input".into(),
                                    });
                                    cx.notify();
                                },
                            )),
                    )
                    .child(pro_fold(
                        pro_eid(&self.id, format!("in-fold-{}", step.id)),
                        !in_folded,
                        88.,
                        0.,
                        move || {
                            div()
                                .pt(px(8.))
                                .child(
                                    div()
                                        .px(px(14.))
                                        .py(px(12.))
                                        .rounded(px(10.))
                                        .bg(well_bg)
                                        .font_family("Menlo")
                                        .text_size(px(11.5))
                                        .line_height(px(18.))
                                        .text_color(p.text)
                                        .child(txt_fold),
                                )
                                .into_any_element()
                        },
                    )),
            );
        }

        // Output section (or live waiting indicator)
        if let Some(ref out_txt) = step.output {
            let out_fold_key = SharedString::from(format!("{}-out", step.id));
            let out_folded = self.collapsed_io.contains(&out_fold_key);
            let copy_key = SharedString::from(format!("{}:output", step.id));
            let is_copied = copied_tag.as_ref() == Some(&copy_key);
            let step_id_cp = step.id.clone();
            let txt_cp = out_txt.clone();
            let txt_fold = out_txt.clone();
            let fold_toggle_key = out_fold_key.clone();

            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, format!("fold-out-{}", step.id)))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .cursor_pointer()
                                    .hover(move |s| s.text_color(p.ink))
                                    .child(tr(cx, "Output", "输出"))
                                    .child(pro_icon(
                                        if out_folded {
                                            ProIcon::ChevronRight
                                        } else {
                                            ProIcon::ChevronDown
                                        },
                                        11.,
                                        p.faint,
                                    )),
                                cx,
                                move |this, _, cx| {
                                    if !this.collapsed_io.remove(&fold_toggle_key) {
                                        this.collapsed_io.insert(fold_toggle_key.clone());
                                    }
                                    cx.notify();
                                },
                            ))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, format!("copy-out-{}", step.id)))
                                    .role(Role::Button)
                                    .aria_label(tr(cx, "Copy output", "复制输出"))
                                    .tab_index(0)
                                    .h(px(22.))
                                    .px(px(6.))
                                    .rounded(px(6.))
                                    .hover(move |s| s.bg(p.chip).text_color(p.ink))
                                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(12.))
                                    .text_color(if is_copied { p.green_ink } else { p.muted })
                                    .cursor_pointer()
                                    .child(pro_icon(
                                        if is_copied {
                                            ProIcon::Check
                                        } else {
                                            ProIcon::Copy
                                        },
                                        12.,
                                        if is_copied { p.green_ink } else { p.muted },
                                    ))
                                    .child(if is_copied {
                                        tr(cx, "Copied", "已复制")
                                    } else {
                                        tr(cx, "Copy", "复制")
                                    }),
                                cx,
                                move |this, _, cx| {
                                    clipboard::write(txt_cp.as_ref(), cx).detach();
                                    this.copied_key = Some((copy_key.clone(), Instant::now()));
                                    cx.emit(AgentRunDetailEvent::CopyPayload {
                                        step_id: step_id_cp.clone(),
                                        field: "output".into(),
                                    });
                                    cx.notify();
                                },
                            )),
                    )
                    .child(pro_fold(
                        pro_eid(&self.id, format!("out-fold-{}", step.id)),
                        !out_folded,
                        92.,
                        0.,
                        move || {
                            div()
                                .pt(px(8.))
                                .child(
                                    div()
                                        .px(px(14.))
                                        .py(px(12.))
                                        .rounded(px(10.))
                                        .bg(well_bg)
                                        .font_family("Menlo")
                                        .text_size(px(11.5))
                                        .line_height(px(18.))
                                        .text_color(p.text)
                                        .child(txt_fold),
                                )
                                .into_any_element()
                        },
                    )),
            );
        } else if step_running {
            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Output", "输出")),
                    )
                    .child(
                        div()
                            .px(px(14.))
                            .py(px(12.))
                            .rounded(px(10.))
                            .bg(well_bg)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .text_color(p.muted)
                            .child(div().size(px(6.)).rounded_full().bg(p.blue))
                            .child(tr(cx, "Streaming step output…", "正在生成输出…")),
                    ),
            );
        }

        // Collapsible Reasoning disclosure (when step has chain-of-thought reasoning)
        if let Some(ref reason_txt) = step.reasoning {
            let open = self.reasoning_open.contains(&step.id);
            let toggle_id = step.id.clone();
            let reason_fold_txt = reason_txt.clone();
            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("reason-{}", step.id)))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .cursor_pointer()
                            .hover(move |s| s.text_color(p.ink))
                            .child(pro_icon(
                                if open {
                                    ProIcon::ChevronDown
                                } else {
                                    ProIcon::ChevronRight
                                },
                                12.,
                                p.faint,
                            ))
                            .child(tr(cx, "Reasoning trace", "推理摘要")),
                        cx,
                        move |this, _, cx| {
                            if !this.reasoning_open.remove(&toggle_id) {
                                this.reasoning_open.insert(toggle_id.clone());
                            }
                            cx.notify();
                        },
                    ))
                    .child(pro_fold(
                        pro_eid(&self.id, format!("reason-fold-{}", step.id)),
                        open,
                        84.,
                        0.,
                        move || {
                            div()
                                .pt(px(8.))
                                .child(
                                    div()
                                        .px(px(12.))
                                        .py(px(10.))
                                        .rounded(px(10.))
                                        .bg(well_bg)
                                        .border_1()
                                        .border_color(p.purple.opacity(0.32))
                                        .text_size(px(12.5))
                                        .line_height(px(19.))
                                        .text_color(p.text)
                                        .child(reason_fold_txt),
                                )
                                .into_any_element()
                        },
                    )),
            );
        }

        // Artifacts produced by this step
        if !step.artifacts.is_empty() {
            let mut chips = div().flex().flex_wrap().gap(px(6.));
            for art in &step.artifacts {
                let ic = if art.kind.as_ref() == "link" {
                    ProIcon::Link
                } else {
                    ProIcon::Code
                };
                let ic_color = if art.kind.as_ref() == "link" {
                    p.blue
                } else {
                    p.orange
                };
                chips = chips.child(
                    div()
                        .h(px(26.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(p.ink)
                        .child(pro_icon(ic, 13., ic_color))
                        .child(art.name.clone())
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.muted)
                                .child(art.summary.clone()),
                        ),
                );
            }
            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Artifacts", "产出文件")),
                    )
                    .child(chips),
            );
        }

        Some((
            step.label.clone(),
            ProIcon::ClipboardDoc,
            pro_view_transition(pro_eid(&self.id, "step-insp-vtx"), step.id.clone(), panel),
        ))
    }
}

impl Render for AgentRunDetailScreen {
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
        AgentRunRecord, StepTokens, compute_axis_ticks, format_compact_tokens, format_run_seconds,
        format_with_commas,
    };

    #[test]
    fn default_run_aggregates_match_migration_trace() {
        let run = AgentRunRecord::default();
        assert_eq!(run.steps.len(), 16);

        let failed: Vec<&str> = run
            .steps
            .iter()
            .filter(|s| s.failed)
            .map(|s| s.id.as_ref())
            .collect();
        assert_eq!(failed, vec!["dry-run"]);

        let total_tokens: u64 = run
            .steps
            .iter()
            .filter_map(|s| s.tokens.map(StepTokens::total))
            .sum();
        assert_eq!(format_compact_tokens(total_tokens), "46.6K");

        let total_cost: f64 = run.steps.iter().filter_map(|s| s.cost).sum();
        assert_eq!(format!("{total_cost:.2}"), "0.76");

        let write_step = run
            .steps
            .iter()
            .find(|s| s.id.as_ref() == "write")
            .expect("default run includes write step");
        let write_tok = write_step.tokens.expect("write step has token counts");
        assert_eq!(format_with_commas(write_tok.input), "18,640");
        assert_eq!(format_with_commas(write_tok.output), "2,104");
    }

    #[test]
    fn round_step_axis_scales_cleanly() {
        let (max_live, ticks_live) = compute_axis_ticks(43.0, true);
        assert_eq!(max_live, 70.0);
        assert_eq!(ticks_live.len(), 8);
        assert_eq!(format_run_seconds(ticks_live[0], false), "0s");
        assert_eq!(format_run_seconds(ticks_live[6], false), "1m");
        assert_eq!(format_run_seconds(ticks_live[7], false), "1m 10s");
    }
}
