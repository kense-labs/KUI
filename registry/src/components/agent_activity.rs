//! Chronological agent activity feed with day groups, permission provenance, and step undo.
//!
//! Displays every step an autonomous agent took across connected apps as a plain sentence
//! grouped by day, showing whether it acted on its own or with user approval, why it was
//! permitted, and an inline `Undo` control while the action remains reversible.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_ease, pro_eid, pro_empty_state,
            pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_selection_sensor,
            pro_skeleton_bar, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// Authorization source indicating who permitted an [`ActivityItem`] to execute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivityActor {
    /// Executed autonomously under an active workspace permission rule.
    #[default]
    Auto,
    /// Explicitly approved by the user prior to execution.
    Approved,
}

/// Filter tab scope in [`AgentActivityScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivityFilter {
    /// Show every recorded step.
    #[default]
    All,
    /// Show steps the agent took on its own.
    Auto,
    /// Show steps approved by the user.
    Approved,
    /// Show steps that have been undone.
    Undone,
}

impl ActivityFilter {
    const ALL: [Self; 4] = [Self::All, Self::Auto, Self::Approved, Self::Undone];

    fn next(self) -> Self {
        match self {
            Self::All => Self::Auto,
            Self::Auto => Self::Approved,
            Self::Approved => Self::Undone,
            Self::Undone => Self::All,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::All => Self::Undone,
            Self::Auto => Self::All,
            Self::Approved => Self::Auto,
            Self::Undone => Self::Approved,
        }
    }
}

/// Connected third-party application metadata displayed on activity rows and in the step inspector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityApp {
    /// Stable identifier (for example `"gmail"` or `"stripe"`).
    pub id: SharedString,
    /// Human-readable application name (for example `"Gmail"`).
    pub name: SharedString,
    /// Optional connected account handle (for example `"alex@fieldwork.co"`).
    pub account: Option<SharedString>,
}

impl ActivityApp {
    /// Construct an [`ActivityApp`] with `id` and display `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            account: None,
        }
    }

    /// Attach a connected account handle shown under the step title.
    pub fn with_account(mut self, account: impl Into<SharedString>) -> Self {
        self.account = Some(account.into());
        self
    }
}

/// Key-value field inside the `What happened` inspector card of an [`ActivityItem`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityDetail {
    /// Field label (for example `"To"` or `"Subject"`).
    pub label: SharedString,
    /// Field value (for example `"priya.shah@northwind.com"`).
    pub value: SharedString,
}

impl ActivityDetail {
    /// Construct one key-value detail pair.
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// Reversibility window and mechanism for an [`ActivityItem`] that can still be undone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityUndoSpec {
    /// Plain-language description of what `Undo` performs.
    pub how: SharedString,
    /// Remaining window during which `Undo` stays available.
    pub until: SharedString,
}

impl ActivityUndoSpec {
    /// Construct an undo specification describing `how` it reverses the step and `until` when.
    pub fn new(how: impl Into<SharedString>, until: impl Into<SharedString>) -> Self {
        Self {
            how: how.into(),
            until: until.into(),
        }
    }
}

/// One recorded agent action in [`AgentActivityScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivityItem {
    /// Stable identifier (for example `"a1"`).
    pub id: SharedString,
    /// Day bucket heading (for example `"Today"` or `"Yesterday"`).
    pub day: SharedString,
    /// 24-hour clock timestamp (for example `"08:57"`).
    pub time: SharedString,
    /// Optional relative timestamp shown in the inspector (`"12 minutes ago"`).
    pub relative_time: Option<SharedString>,
    /// Application id or name matching an [`ActivityApp`].
    pub app: SharedString,
    /// Complete sentence describing what the agent did.
    pub text: SharedString,
    /// Optional parent task name.
    pub task: Option<SharedString>,
    /// Who permitted the action (`Auto` vs `Approved`).
    pub by: ActivityActor,
    /// Optional permission rule under which the step ran.
    pub rule: Option<SharedString>,
    /// Optional relative time when the user approved the step (`"14 minutes ago"`).
    pub approved_at: Option<SharedString>,
    /// Structured metadata fields shown in `What happened`.
    pub details: Vec<ActivityDetail>,
    /// Optional body/payload preview quote shown in `What happened`.
    pub preview: Option<SharedString>,
    /// Reversibility specification when the step can be undone.
    pub undo: Option<ActivityUndoSpec>,
    /// Explanation shown with a lock icon when the step is permanent (`Final`).
    pub final_reason: Option<SharedString>,
    /// Whether this step has already been undone.
    pub undone: bool,
    /// Optional formatted execution cost (`"€0.04"`).
    pub cost: Option<SharedString>,
}

impl ActivityItem {
    /// Construct a new [`ActivityItem`] with core sentence fields.
    pub fn new(
        id: impl Into<SharedString>,
        day: impl Into<SharedString>,
        time: impl Into<SharedString>,
        app: impl Into<SharedString>,
        text: impl Into<SharedString>,
        by: ActivityActor,
    ) -> Self {
        Self {
            id: id.into(),
            day: day.into(),
            time: time.into(),
            relative_time: None,
            app: app.into(),
            text: text.into(),
            task: None,
            by,
            rule: None,
            approved_at: None,
            details: Vec::new(),
            preview: None,
            undo: None,
            final_reason: None,
            undone: false,
            cost: None,
        }
    }

    /// Set the relative timestamp shown in the inspector `When` row.
    pub fn with_relative_time(mut self, rel: impl Into<SharedString>) -> Self {
        self.relative_time = Some(rel.into());
        self
    }

    /// Associate the step with a parent task.
    pub fn with_task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = Some(task.into());
        self
    }

    /// Set the permission rule under which the step executed.
    pub fn with_rule(mut self, rule: impl Into<SharedString>) -> Self {
        self.rule = Some(rule.into());
        self
    }

    /// Set when the user approved the step.
    pub fn with_approved_at(mut self, approved_at: impl Into<SharedString>) -> Self {
        self.approved_at = Some(approved_at.into());
        self
    }

    /// Set structured key-value details for `What happened`.
    pub fn with_details(mut self, details: Vec<ActivityDetail>) -> Self {
        self.details = details;
        self
    }

    /// Set the quoted payload preview in `What happened`.
    pub fn with_preview(mut self, preview: impl Into<SharedString>) -> Self {
        self.preview = Some(preview.into());
        self
    }

    /// Mark the step reversible with an [`ActivityUndoSpec`].
    pub fn with_undo(mut self, undo: ActivityUndoSpec) -> Self {
        self.undo = Some(undo);
        self
    }

    /// Mark the step permanent with a plain-language reason.
    pub fn with_final_reason(mut self, reason: impl Into<SharedString>) -> Self {
        self.final_reason = Some(reason.into());
        self
    }

    /// Set whether the step has already been undone.
    pub fn with_undone(mut self, undone: bool) -> Self {
        self.undone = undone;
        self
    }

    /// Set the formatted run cost (`"€0.04"`).
    pub fn with_cost(mut self, cost: impl Into<SharedString>) -> Self {
        self.cost = Some(cost.into());
        self
    }

    /// Return `true` while the step can still be undone.
    pub fn can_undo(&self) -> bool {
        !self.undone && self.undo.is_some()
    }

    /// Return `true` if this item belongs to `filter`.
    pub fn matches_filter(&self, filter: ActivityFilter) -> bool {
        match filter {
            ActivityFilter::All => true,
            ActivityFilter::Auto => self.by == ActivityActor::Auto,
            ActivityFilter::Approved => self.by == ActivityActor::Approved,
            ActivityFilter::Undone => self.undone,
        }
    }

    /// Return `true` if this item matches a case-insensitive search `query`.
    pub fn matches_query(&self, query: &str, resolved_app_name: &str) -> bool {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return true;
        }
        self.text.to_ascii_lowercase().contains(&q)
            || self.app.to_ascii_lowercase().contains(&q)
            || resolved_app_name.to_ascii_lowercase().contains(&q)
            || self.time.to_ascii_lowercase().contains(&q)
            || self
                .task
                .as_ref()
                .is_some_and(|t| t.to_ascii_lowercase().contains(&q))
            || self
                .rule
                .as_ref()
                .is_some_and(|r| r.to_ascii_lowercase().contains(&q))
            || self.details.iter().any(|d| {
                d.label.to_ascii_lowercase().contains(&q)
                    || d.value.to_ascii_lowercase().contains(&q)
            })
    }
}

/// Events emitted by [`AgentActivityScreen`].
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentActivityEvent {
    /// User triggered `Undo` on a reversible step (row is already marked `undone = true`).
    Undo(ActivityItem),
    /// User clicked the arrow next to a task to open it.
    OpenTask(SharedString),
    /// User selected/opened a step in the inspector.
    Select(SharedString),
    /// User navigated to another screen (`"Permissions"`, `"Tasks"`, or sidebar target).
    Navigate(SharedString),
}

impl EventEmitter<AgentActivityEvent> for AgentActivityScreen {}

/// Default seven connected apps used by [`AgentActivityScreen`].
pub fn default_activity_apps() -> Vec<ActivityApp> {
    vec![
        ActivityApp::new("gmail", "Gmail").with_account("alex@fieldwork.co"),
        ActivityApp::new("stripe", "Stripe").with_account("Fieldwork EU"),
        ActivityApp::new("slack", "Slack").with_account("fieldwork.slack.com"),
        ActivityApp::new("github", "GitHub").with_account("fieldwork-hq"),
        ActivityApp::new("google-calendar", "Google Calendar").with_account("alex@fieldwork.co"),
        ActivityApp::new("google-drive", "Google Drive").with_account("Fieldwork Shared"),
        ActivityApp::new("linear", "Linear").with_account("Fieldwork"),
    ]
}

/// Default nine chronological activity items across `Today` (6) and `Yesterday` (3).
pub fn default_activity_items() -> Vec<ActivityItem> {
    vec![
        ActivityItem::new(
            "a1",
            "Today",
            "08:57",
            "Gmail",
            "Sent the weekly update to Priya Shah",
            ActivityActor::Approved,
        )
        .with_relative_time("12 minutes ago")
        .with_task("Weekly update")
        .with_approved_at("14 minutes ago")
        .with_rule("Send email to clients")
        .with_cost("€0.04")
        .with_details(vec![
            ActivityDetail::new("To", "priya.shah@northwind.com"),
            ActivityDetail::new("Subject", "Northwind – week 46"),
        ])
        .with_preview(
            "Hi Priya – three things shipped this week, and one needs your answer: the v1 end date. Details below.",
        )
        .with_undo(ActivityUndoSpec::new(
            "Recalls the email – it's still unread",
            "Possible for 18 more minutes",
        )),
        ActivityItem::new(
            "a2",
            "Today",
            "08:31",
            "Stripe",
            "Refunded 38 € to Hannah Kim",
            ActivityActor::Auto,
        )
        .with_relative_time("38 minutes ago")
        .with_task("Billing webhooks")
        .with_rule("Refund duplicate charges under €50")
        .with_cost("€0.02")
        .with_details(vec![
            ActivityDetail::new("Customer", "Hannah Kim · hannah@lumen.studio"),
            ActivityDetail::new("Charge", "ch_3Q9kL2 · €38.00 (duplicate)"),
        ])
        .with_preview(
            "Duplicate webhook retry charged €38.00 twice within 4 seconds. Refunded the second capture automatically.",
        )
        .with_final_reason(
            "Card refunds settle through the bank network and cannot be reversed once issued.",
        ),
        ActivityItem::new(
            "a3",
            "Today",
            "08:05",
            "Slack",
            "Posted the staging link in #platform",
            ActivityActor::Auto,
        )
        .with_relative_time("1 hour ago")
        .with_task("Billing webhooks")
        .with_rule("Post build and staging updates in #platform")
        .with_cost("€0.01")
        .with_details(vec![
            ActivityDetail::new("Channel", "#platform"),
            ActivityDetail::new("Thread", "Billing webhook retry rollout"),
        ])
        .with_preview(
            "Staging deploy #1842 is live with idempotency keys enabled. Replay suite passed 48/48 webhook events.",
        )
        .with_undo(ActivityUndoSpec::new(
            "Deletes the message from #platform",
            "Possible while nobody has replied",
        )),
        ActivityItem::new(
            "a4",
            "Today",
            "07:34",
            "GitHub",
            "Opened pull request #482 – Retry failed billing webhooks",
            ActivityActor::Auto,
        )
        .with_relative_time("1 hour ago")
        .with_task("Billing webhooks")
        .with_rule("Open draft and review pull requests")
        .with_cost("€0.18")
        .with_details(vec![
            ActivityDetail::new("Repo", "fieldwork-hq/monorepo"),
            ActivityDetail::new("Branch", "fix/webhook-backoff -> main"),
            ActivityDetail::new("Files", "4 changed (+118, −19)"),
        ])
        .with_preview(
            "Adds exponential backoff with jitter and stores Stripe event IDs for 72h so retried deliveries never double-charge.",
        )
        .with_undo(ActivityUndoSpec::new(
            "Closes pull request #482 and deletes the branch",
            "Possible until a reviewer merges it",
        )),
        ActivityItem::new(
            "a5",
            "Today",
            "06:09",
            "Google Calendar",
            "Booked a review call with Tom Becker for Thursday 10:30",
            ActivityActor::Approved,
        )
        .with_relative_time("3 hours ago")
        .with_task("Onboarding emails")
        .with_approved_at("3 hours ago")
        .with_rule("Schedule external calls on your calendar")
        .with_cost("€0.03")
        .with_details(vec![
            ActivityDetail::new("When", "Thu 10:30–11:00 CET"),
            ActivityDetail::new("Guest", "tom.becker@northwind.com"),
        ])
        .with_preview(
            "30-minute walkthrough of the onboarding email sequence and sender domain verification.",
        )
        .with_undo(ActivityUndoSpec::new(
            "Cancels the invite and notifies Tom Becker",
            "Possible until 2 hours before the call",
        )),
        ActivityItem::new(
            "a6",
            "Today",
            "05:09",
            "Google Drive",
            "Created “Northwind – ingest design” in Clients",
            ActivityActor::Auto,
        )
        .with_relative_time("4 hours ago")
        .with_task("Ingest pipeline")
        .with_rule("Create and organize docs in Clients")
        .with_cost("€0.06")
        .with_details(vec![
            ActivityDetail::new("Folder", "Clients / Northwind"),
            ActivityDetail::new("Sharing", "Internal team only"),
        ])
        .with_preview(
            "Architecture notes covering schema validation, dead-letter queues, and S3 parquet partitioning for Northwind's daily feed.\nLine 2: Batch manifest verification and idempotency keys.\nLine 3: Backpressure limits for burst partner uploads.\nLine 4: Replay checkpoints and schema drift alerts.\nLine 5: Per-tenant KMS envelope encryption.\nLine 6: Staging cutover checklist for week 47.\nLine 7: Rollback playbook and on-call escalation.",
        )
        .with_undo(ActivityUndoSpec::new(
            "Moves the document to Google Drive trash",
            "Possible for 30 days",
        )),
        ActivityItem::new(
            "a7",
            "Yesterday",
            "07:09",
            "GitHub",
            "Merged #479 – Move our apps to API v2",
            ActivityActor::Approved,
        )
        .with_relative_time("Yesterday")
        .with_task("API migration")
        .with_approved_at("Yesterday at 07:08")
        .with_rule("Merge pull requests into main")
        .with_cost("€0.09")
        .with_details(vec![
            ActivityDetail::new("Repo", "fieldwork-hq/monorepo"),
            ActivityDetail::new("Commit", "9c41f0e on main"),
        ])
        .with_preview(
            "Switched all workspace connectors to REST API v2 endpoints and removed the deprecated v1 token adapter.",
        )
        .with_final_reason(
            "Merged into main and deployed to production – open a revert pull request instead.",
        ),
        ActivityItem::new(
            "a8",
            "Yesterday",
            "04:09",
            "Gmail",
            "Sent an invoice reminder to Lumen Studio",
            ActivityActor::Approved,
        )
        .with_relative_time("Yesterday")
        .with_task("Billing webhooks")
        .with_approved_at("Yesterday at 04:07")
        .with_rule("Send email to clients")
        .with_cost("€0.03")
        .with_details(vec![
            ActivityDetail::new("To", "billing@lumen.studio"),
            ActivityDetail::new("Invoice", "INV-2048 · €1,200.00"),
        ])
        .with_preview(
            "Friendly reminder that invoice INV-2048 is due this Friday. Let us know if you need an updated PO reference.",
        )
        .with_undo(ActivityUndoSpec::new(
            "Recalled the email before it was opened",
            "Taken back 4 minutes after sending",
        ))
        .with_undone(true),
        ActivityItem::new(
            "a9",
            "Yesterday",
            "02:09",
            "Slack",
            "Asked Tom Becker for the staging keys",
            ActivityActor::Approved,
        )
        .with_relative_time("Yesterday")
        .with_task("Billing webhooks")
        .with_approved_at("Yesterday at 02:08")
        .with_rule("Message external guests in Slack Connect")
        .with_cost("€0.02")
        .with_details(vec![
            ActivityDetail::new("Channel", "#fieldwork-northwind"),
            ActivityDetail::new("Recipient", "Tom Becker"),
        ])
        .with_preview(
            "Hey Tom – could you drop the rotated staging webhook signing secret in 1Password when you have a moment?",
        )
        .with_final_reason("Tom Becker already read and replied to this message in Slack Connect."),
    ]
}

fn count_matching(items: &[ActivityItem], filter: ActivityFilter) -> usize {
    items.iter().filter(|it| it.matches_filter(filter)).count()
}

fn lerp_hsla(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let t = t.clamp(0., 1.);
    let ra: Rgba = a.into();
    let rb: Rgba = b.into();
    Rgba {
        r: ra.r + (rb.r - ra.r) * t,
        g: ra.g + (rb.g - ra.g) * t,
        b: ra.b + (rb.b - ra.b) * t,
        a: ra.a + (rb.a - ra.a) * t,
    }
    .into()
}

/// Pro Agent Activity screen with day-grouped step rows, search/filter bar, and `Step` inspector.
pub struct AgentActivityScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    focus: FocusHandle,
    apps: Vec<ActivityApp>,
    items: Vec<ActivityItem>,
    filter: ActivityFilter,
    selected_id: Option<SharedString>,
    focused_id: Option<SharedString>,
    expanded_preview_id: Option<SharedString>,
    search_editor: Option<Entity<Editor>>,
    mobile_search_open: bool,
    loading: bool,
    empty: bool,
    last_announcement: Option<SharedString>,
    clock: Instant,
    undo_animations: Vec<(SharedString, Instant)>,
    measured_width: Rc<Cell<f32>>,
}

impl AgentActivityScreen {
    /// Construct an [`AgentActivityScreen`] with the default `Fieldwork` workspace framing and 9 sample steps.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle().tab_stop(true);
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Agent, shell_cx)
                .with_active("Activity")
                .with_work_area(true, true)
                .with_panel_width(360.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(AgentActivityEvent::Navigate(label.clone()));
            }
            AppShellEvent::PanelOpen(false) => {
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            standalone: false,
            focus,
            apps: default_activity_apps(),
            items: default_activity_items(),
            filter: ActivityFilter::All,
            selected_id: Some("a1".into()),
            focused_id: Some("a1".into()),
            expanded_preview_id: None,
            search_editor: None,
            mobile_search_open: false,
            loading: false,
            empty: false,
            last_announcement: None,
            clock: Instant::now(),
            undo_animations: Vec::new(),
            measured_width: Rc::new(Cell::new(560.)),
        }
    }

    /// Render only the inner activity page without the surrounding [`AppShell`].
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the connected application definitions.
    pub fn with_apps(mut self, apps: Vec<ActivityApp>) -> Self {
        self.apps = apps;
        self
    }

    /// Override the activity step items.
    pub fn with_items(mut self, items: Vec<ActivityItem>) -> Self {
        if self
            .selected_id
            .as_ref()
            .is_some_and(|sel| !items.iter().any(|it| &it.id == sel))
        {
            self.selected_id = items.first().map(|it| it.id.clone());
        }
        self.focused_id = self.selected_id.clone();
        self.items = items;
        self
    }

    /// Set the step initially open in the right-hand `Step` inspector (`Some("a1")` by default).
    pub fn with_selected(mut self, selected: Option<SharedString>, cx: &mut Context<Self>) -> Self {
        let has_sel = selected.is_some();
        self.selected_id = selected.clone();
        self.focused_id = selected;
        self.shell.update(cx, |s, cx| s.set_panel_open(has_sel, cx));
        self
    }

    /// Set the initial filter tab (`All`, `Auto`, `Approved`, `Undone`).
    pub fn with_filter(mut self, filter: ActivityFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the zero-activity empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle the skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether the screen is currently showing loading skeletons.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle the empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the screen is forced into its empty state.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Replace the activity list at runtime.
    pub fn set_items(&mut self, items: Vec<ActivityItem>, cx: &mut Context<Self>) {
        if self
            .selected_id
            .as_ref()
            .is_some_and(|sel| !items.iter().any(|it| &it.id == sel))
        {
            self.selected_id = items.first().map(|it| it.id.clone());
        }
        self.items = items;
        cx.notify();
    }

    /// Return the current slice of activity items.
    pub fn items(&self) -> &[ActivityItem] {
        &self.items
    }

    /// Return the id of the step currently open in the inspector.
    pub fn selected_id(&self) -> Option<&SharedString> {
        self.selected_id.as_ref()
    }

    /// Select and open a step in the right-hand inspector.
    pub fn select(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        self.selected_id = id.clone();
        if id.is_some() {
            self.focused_id = id.clone();
            self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        }
        if let Some(sel) = id {
            cx.emit(AgentActivityEvent::Select(sel));
        }
        cx.notify();
    }

    /// Optimistically mark `id` as undone if reversible, emitting [`AgentActivityEvent::Undo`].
    pub fn undo_item(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let Some(item) = self.items.iter_mut().find(|it| it.id.as_ref() == id) else {
            return false;
        };
        if !item.can_undo() {
            return false;
        }
        item.undone = true;
        let snapshot = item.clone();
        self.last_announcement = Some(format!("Undone: {}", snapshot.text).into());
        self.undo_animations.retain(|(aid, _)| aid.as_ref() != id);
        self.undo_animations
            .push((snapshot.id.clone(), Instant::now()));
        cx.emit(AgentActivityEvent::Undo(snapshot));
        cx.notify();
        true
    }

    /// Roll back an optimistic undo if the upstream undo call fails.
    pub fn revert_undo(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let Some(item) = self.items.iter_mut().find(|it| it.id.as_ref() == id) else {
            return false;
        };
        if !item.undone {
            return false;
        }
        item.undone = false;
        self.undo_animations.retain(|(aid, _)| aid.as_ref() != id);
        cx.notify();
        true
    }

    /// Access the hosting [`AppShell`] entity.
    pub fn shell(&self) -> &Entity<AppShell> {
        &self.shell
    }

    fn resolve_app(&self, app_ref: &str) -> Option<&ActivityApp> {
        let lower = app_ref.trim().to_ascii_lowercase();
        self.apps
            .iter()
            .find(|a| a.id.to_ascii_lowercase() == lower || a.name.to_ascii_lowercase() == lower)
    }

    fn app_display_name(&self, app_ref: &str) -> SharedString {
        self.resolve_app(app_ref)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| SharedString::from(app_ref.to_owned()))
    }

    fn ensure_search_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Search activity", "搜索活动").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed | EditEvent::Submitted => {
                    cx.notify();
                }
                EditEvent::Escape => {
                    let has_val = !ed_ent.read(cx).value().is_empty();
                    if has_val {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    } else {
                        this.mobile_search_open = false;
                        this.focus.focus(w, cx);
                    }
                    cx.notify();
                }
                EditEvent::Navigate(down) if *down => {
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }
    }

    fn current_query(&self, cx: &App) -> String {
        self.search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_owned())
            .unwrap_or_default()
    }

    fn filtered_indices(&self, query: &str) -> Vec<usize> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, it)| {
                let app_name = self.app_display_name(&it.app);
                it.matches_filter(self.filter) && it.matches_query(query, &app_name)
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    fn undo_anim_ms(&self, id: &str) -> Option<f32> {
        self.undo_animations
            .iter()
            .find(|(aid, _)| aid.as_ref() == id)
            .map(|(_, started)| started.elapsed().as_secs_f32() * 1000.)
    }

    fn translate_day<'a>(&self, day: &'a str, cx: &App) -> &'a str {
        match day {
            "Today" => tr(cx, "Today", "今天"),
            "Yesterday" => tr(cx, "Yesterday", "昨天"),
            other => other,
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.loading {
            return;
        }
        let search_focused = self
            .search_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus.is_focused(w));
        let key = ev.keystroke.key.as_str();
        let m = ev.keystroke.modifiers;

        if search_focused {
            if key == "down" {
                self.focus.focus(w, cx);
                cx.stop_propagation();
                cx.notify();
            } else if key == "escape" {
                if let Some(ed) = self.search_editor.clone() {
                    if !ed.read(cx).value().is_empty() {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                    } else {
                        self.mobile_search_open = false;
                        self.focus.focus(w, cx);
                    }
                }
                cx.stop_propagation();
                cx.notify();
            }
            return;
        }

        if m.platform || m.control || m.alt {
            return;
        }

        let query = self.current_query(cx);
        let visible = self.filtered_indices(&query);

        match key {
            "/" => {
                self.mobile_search_open = true;
                if let Some(ed) = self.search_editor.clone() {
                    ed.read(cx).focus.clone().focus(w, cx);
                }
                w.prevent_default();
                cx.stop_propagation();
                cx.notify();
            }
            "escape" => {
                if !query.is_empty() {
                    if let Some(ed) = self.search_editor.clone() {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                    self.mobile_search_open = false;
                    cx.stop_propagation();
                    cx.notify();
                } else if self.mobile_search_open {
                    self.mobile_search_open = false;
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            "left" => {
                self.filter = self.filter.prev();
                cx.stop_propagation();
                cx.notify();
            }
            "right" => {
                self.filter = self.filter.next();
                cx.stop_propagation();
                cx.notify();
            }
            "j" | "down" => {
                if !visible.is_empty() {
                    let cur_pos = self
                        .focused_id
                        .as_ref()
                        .or(self.selected_id.as_ref())
                        .and_then(|fid| visible.iter().position(|&idx| &self.items[idx].id == fid));
                    let next_pos = match cur_pos {
                        Some(p) => (p + 1).min(visible.len() - 1),
                        None => 0,
                    };
                    let next_id = self.items[visible[next_pos]].id.clone();
                    self.focused_id = Some(next_id);
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            "k" | "up" => {
                if !visible.is_empty() {
                    let cur_pos = self
                        .focused_id
                        .as_ref()
                        .or(self.selected_id.as_ref())
                        .and_then(|fid| visible.iter().position(|&idx| &self.items[idx].id == fid));
                    let prev_pos = match cur_pos {
                        Some(p) => p.saturating_sub(1),
                        None => 0,
                    };
                    let prev_id = self.items[visible[prev_pos]].id.clone();
                    self.focused_id = Some(prev_id);
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            "enter" => {
                let target = self
                    .focused_id
                    .clone()
                    .or_else(|| self.selected_id.clone())
                    .or_else(|| visible.first().map(|&idx| self.items[idx].id.clone()));
                if target.is_some() {
                    self.select(target, cx);
                    cx.stop_propagation();
                }
            }
            "u" | "U" => {
                let target = self.focused_id.clone().or_else(|| self.selected_id.clone());
                if let Some(tid) = target
                    && self.undo_item(&tid, cx)
                {
                    cx.stop_propagation();
                }
            }
            _ => {}
        }
    }

    fn render_header(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let width = self.measured_width.get();
        let narrow = width < 560.;
        let hide_counts = width < 720.;
        let panel_open = self.standalone || self.shell.read(cx).is_panel_open();
        let query = self.current_query(cx);
        let search_focused = self
            .search_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus.is_focused(w));
        let expand_mobile_search =
            narrow && (self.mobile_search_open || search_focused || !query.is_empty());

        let total_all = count_matching(&self.items, ActivityFilter::All);
        let total_auto = count_matching(&self.items, ActivityFilter::Auto);
        let total_approved = count_matching(&self.items, ActivityFilter::Approved);
        let total_undone = count_matching(&self.items, ActivityFilter::Undone);

        let tab_sensor = pro_selection_sensor();
        let selected_slug = match self.filter {
            ActivityFilter::All => "all",
            ActivityFilter::Auto => "auto",
            ActivityFilter::Approved => "approved",
            ActivityFilter::Undone => "undone",
        };
        let mut tabs_row = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(2.))
            .min_w_0()
            .overflow_hidden()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "filter-tabs"),
                selected_slug,
                tab_sensor.clone(),
                6.,
                p.chip,
                None,
                false,
            ));

        for tab in ActivityFilter::ALL {
            let selected = self.filter == tab;
            let (label, count, slug) = match tab {
                ActivityFilter::All => (tr(cx, "All", "全部"), total_all, "all"),
                ActivityFilter::Auto => (tr(cx, "On its own", "自动执行"), total_auto, "auto"),
                ActivityFilter::Approved => (
                    tr(cx, "You approved", "你已批准"),
                    total_approved,
                    "approved",
                ),
                ActivityFilter::Undone => (tr(cx, "Undone", "已撤销"), total_undone, "undone"),
            };
            tabs_row = tabs_row.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("tab-{slug}")))
                    .role(Role::Tab)
                    .aria_label(format!("{label} {count}"))
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .when(!selected, |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .when(selected, |d| d.child(pro_mark_selected(&tab_sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .text_color(if selected { p.ink } else { p.muted })
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .child(label)
                    .when(!hide_counts && !self.loading, |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(count.to_string()),
                        )
                    }),
                cx,
                move |this, _, cx| {
                    this.filter = tab;
                    cx.notify();
                },
            ));
        }

        let build_search_box = |full_width: bool, cx: &mut Context<Self>| {
            let search_ed = self.search_editor.clone();
            let ed_click = search_ed.clone();
            let ed_clear = search_ed.clone();
            let has_query = !query.is_empty();
            div()
                .id(pro_eid(&self.id, "search-box"))
                .when(full_width, |d| d.w_full().min_w_0())
                .when(!full_width, |d| d.w(px(210.)).min_w(px(136.)))
                .h(px(28.))
                .px(px(8.))
                .rounded(px(6.))
                .bg(if search_focused { p.card } else { p.chip })
                .border_1()
                .border_color(if search_focused {
                    p.line_strong
                } else {
                    transparent_black()
                })
                .flex()
                .items_center()
                .gap(px(6.))
                .cursor_text()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |_, _, w, cx| {
                        if let Some(ed) = &ed_click {
                            ed.read(cx).focus.clone().focus(w, cx);
                        }
                    }),
                )
                .child(pro_icon(ProIcon::Search, 14., p.faint))
                .when_some(search_ed, |d, ed| {
                    d.child(div().flex_1().min_w_0().overflow_hidden().child(ed))
                })
                .when(has_query || full_width, |d| {
                    d.child(activate(
                        div()
                            .id(pro_eid(&self.id, "search-clear"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Clear search", "清除搜索"))
                            .tab_index(0)
                            .size(px(18.))
                            .rounded(px(4.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child(pro_icon(ProIcon::Close, 11., p.muted)),
                        cx,
                        move |this, w, cx| {
                            if let Some(ed) = &ed_clear {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                            this.mobile_search_open = false;
                            this.focus.focus(w, cx);
                            cx.notify();
                        },
                    ))
                })
        };

        let (header_right_el, mobile_fold_el) = if narrow {
            let toggle_btn = pro_icon_btn(
                pro_eid(&self.id, "search-open"),
                tr(cx, "Search activity", "搜索活动"),
                ProIcon::Search,
                28.,
                expand_mobile_search,
                cx,
                |this, w, cx| {
                    this.mobile_search_open = !this.mobile_search_open;
                    if this.mobile_search_open {
                        if let Some(ed) = this.search_editor.clone() {
                            ed.read(cx).focus.clone().focus(w, cx);
                        }
                    } else {
                        if let Some(ed) = this.search_editor.clone() {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                        this.focus.focus(w, cx);
                    }
                    cx.notify();
                },
            )
            .into_any_element();
            let mobile_box = div()
                .w_full()
                .px(px(12.))
                .py(px(6.))
                .border_b_1()
                .border_color(p.line)
                .child(build_search_box(true, cx))
                .into_any_element();
            let fold = pro_fold(
                pro_eid(&self.id, "mobile-search-fold"),
                expand_mobile_search,
                41.,
                0.,
                move || mobile_box,
            );
            (toggle_btn, Some(fold))
        } else {
            (build_search_box(false, cx).into_any_element(), None)
        };

        div()
            .flex_none()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(48.))
                    .flex_none()
                    .pl(px(if narrow { 44. } else { 20. }))
                    .pr(px(if !panel_open { 52. } else { 12. }))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .when(!narrow, |d| {
                                d.child(
                                    div()
                                        .flex_none()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(tr(cx, "Activity", "活动记录")),
                                )
                            })
                            .child(tabs_row),
                    )
                    .child(header_right_el),
            )
            .when_some(mobile_fold_el, |d, f| d.child(f))
    }

    fn render_row(
        &self,
        item: &ActivityItem,
        narrow: bool,
        reduced_motion: bool,
        cx: &Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let is_selected = self.selected_id.as_ref() == Some(&item.id);
        let is_kbd_focused = self.focused_id.as_ref() == Some(&item.id);
        let app_name = self.app_display_name(&item.app);

        let anim_ms = if reduced_motion {
            None
        } else {
            self.undo_anim_ms(&item.id)
        };

        let (strike_frac, sentence_color, tag_alpha) = if item.undone {
            if let Some(ms) = anim_ms {
                let s_frac = pro_ease((ms / 520.).clamp(0., 1.));
                let c_frac = pro_ease(((ms - 220.) / 380.).clamp(0., 1.));
                let pop = pro_ease(((ms - 320.) / 340.).clamp(0., 1.));
                (s_frac, lerp_hsla(p.ink, p.faint, c_frac), pop)
            } else {
                (1.0, p.faint, 1.0)
            }
        } else {
            (0.0, p.ink, 1.0)
        };

        let actor_label = match item.by {
            ActivityActor::Auto => tr(cx, "On its own", "自动执行"),
            ActivityActor::Approved => tr(cx, "You approved", "你已批准"),
        };
        let dot_color = if item.undone {
            p.faint
        } else {
            match item.by {
                ActivityActor::Auto => p.blue,
                ActivityActor::Approved => p.amber,
            }
        };

        let mut full_aria = format!("{}, {}, in {}", item.time, item.text, app_name);
        if let Some(t) = &item.task {
            full_aria.push_str(&format!(", for {t}"));
        }
        full_aria.push_str(&format!(", {actor_label}"));
        if item.undone {
            full_aria.push_str(", Undone");
        }

        let item_id_select = item.id.clone();
        let item_id_undo = item.id.clone();

        let sentence_el = div().min_w_0().flex().items_center().child(
            div()
                .relative()
                .max_w_full()
                .overflow_hidden()
                .text_size(px(13.))
                .line_height(px(20.))
                .text_color(sentence_color)
                .when(!narrow, |d| d.whitespace_nowrap().text_ellipsis())
                .child(item.text.clone())
                .when(strike_frac > 0.01, |d| {
                    d.child(
                        div()
                            .absolute()
                            .left_0()
                            .top(px(10.5))
                            .h(px(1.))
                            .w(relative(strike_frac))
                            .bg(sentence_color),
                    )
                }),
        );

        let mut meta_row = div()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(5.))
            .text_size(px(12.))
            .line_height(px(16.))
            .text_color(p.muted)
            .whitespace_nowrap()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().size(px(6.)).rounded_full().bg(dot_color))
                    .child(actor_label),
            );

        if let Some(task_name) = item.task.clone() {
            meta_row = meta_row
                .child(div().flex_none().text_color(p.faint).child("·"))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(p.faint)
                        .child(task_name),
                );
        }

        let main_btn = activate(
            div()
                .id(pro_eid(&self.id, format!("row-main-{}", item.id)))
                .role(Role::Button)
                .aria_label(full_aria)
                .tab_index(0)
                .flex_1()
                .min_w_0()
                .min_h(px(56.))
                .py(px(8.))
                .pl(px(12.))
                .rounded(px(8.))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(12.))
                .cursor_pointer()
                .when(!narrow, |d| {
                    d.child(
                        div()
                            .w(px(38.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(item.time.clone()),
                    )
                })
                .child(pro_app_tile(&app_name, 26., cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(sentence_el)
                        .child(meta_row),
                ),
            cx,
            move |this, _, cx| {
                this.select(Some(item_id_select.clone()), cx);
            },
        );

        let end_el = if item.undone {
            div()
                .h(px(18.))
                .px(px(6.))
                .rounded(px(4.))
                .bg(p.chip)
                .opacity(tag_alpha)
                .flex()
                .items_center()
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted)
                .child(tr(cx, "Undone", "已撤销"))
                .into_any_element()
        } else if item.undo.is_some() {
            let aria_undo: SharedString =
                format!("{}: {}", tr(cx, "Undo", "撤销"), item.text).into();
            activate(
                div()
                    .id(pro_eid(&self.id, format!("row-undo-{}", item.id)))
                    .role(Role::Button)
                    .aria_label(aria_undo)
                    .tab_index(0)
                    .h(px(26.))
                    .when(narrow, |d| d.w(px(28.)).justify_center())
                    .when(!narrow, |d| d.pl(px(8.)).pr(px(10.)))
                    .rounded(px(6.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::Undo, 13., p.muted))
                    .when(!narrow, |d| d.child(tr(cx, "Undo", "撤销"))),
                cx,
                move |this, _, cx| {
                    this.undo_item(&item_id_undo, cx);
                },
            )
            .into_any_element()
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(12.))
                .text_color(p.faint)
                .child(pro_icon(ProIcon::Lock, 12., p.faint))
                .child(tr(cx, "Final", "不可撤销"))
                .into_any_element()
        };

        div()
            .w_full()
            .min_h(px(56.))
            .pr(px(10.))
            .rounded(px(8.))
            .bg(if is_selected {
                p.chip
            } else {
                transparent_black()
            })
            .when(is_kbd_focused && !is_selected, |d| {
                d.border_1().border_color(p.line_strong)
            })
            .hover(move |s| s.bg(if is_selected { p.chip } else { p.hover }))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(main_btn)
            .child(
                div()
                    .when(!narrow, |d| d.w(px(84.)))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .items_center()
                    .child(end_el),
            )
    }

    fn render_loading_list(&self, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let dhead_bg = if p.dark {
            hsla(0., 0., 1., 0.04)
        } else {
            hsla(0., 0., 0., 0.035)
        };
        let row_widths: [(f32, f32); 6] = [
            (220., 130.),
            (184., 118.),
            (204., 124.),
            (256., 132.),
            (192., 112.),
            (212., 126.),
        ];
        let build_skel_day = |count: usize, offset: usize| {
            let mut rows = div().flex().flex_col().gap(px(2.));
            for i in 0..count {
                let (w1, w2) = row_widths[(offset + i) % row_widths.len()];
                rows = rows.child(
                    div()
                        .min_h(px(56.))
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .child(pro_skeleton_bar(32., 11., 4., phase, cx))
                                .child(pro_skeleton_bar(26., 26., 7., phase, cx))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .gap(px(6.))
                                        .child(pro_skeleton_bar(w1, 13., 4., phase, cx))
                                        .child(pro_skeleton_bar(w2, 10., 4., phase, cx)),
                                ),
                        )
                        .child(pro_skeleton_bar(56., 22., 6., phase, cx)),
                );
            }
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .h(px(32.))
                        .px(px(12.))
                        .rounded(px(10.))
                        .bg(dhead_bg)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(54., 12., 4., phase, cx))
                        .child(pro_skeleton_bar(18., 16., 5., phase, cx)),
                )
                .child(rows)
        };

        div()
            .px(px(8.))
            .pt(px(10.))
            .pb(px(40.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(build_skel_day(4, 0))
            .child(build_skel_day(2, 4))
    }

    fn render_loading_inspector(&self, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        div()
            .px(px(16.))
            .pt(px(14.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .gap(px(22.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(12.))
                    .child(pro_skeleton_bar(36., 36., 7., phase, cx))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(180., 15., 4., phase, cx))
                            .child(pro_skeleton_bar(116., 11., 4., phase, cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children((0..3usize).map(|_| {
                        div()
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .child(pro_skeleton_bar(48., 11., 4., phase, cx))
                            .child(pro_skeleton_bar(130., 12., 4., phase, cx))
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(84., 11., 4., phase, cx))
                    .child(
                        div()
                            .p(px(12.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(190., 12., 4., phase, cx))
                            .child(pro_skeleton_bar(160., 12., 4., phase, cx))
                            .child(pro_skeleton_bar(210., 36., 5., phase, cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(72., 11., 4., phase, cx))
                    .child(
                        div()
                            .p(px(12.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(96., 12., 4., phase, cx))
                            .child(pro_skeleton_bar(176., 12., 4., phase, cx)),
                    ),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_search_editor(w, cx);
        let p = ProPalette::get(cx);
        let reduced_motion = Theme::global(cx).reduced_motion;

        self.undo_animations
            .retain(|(_, started)| started.elapsed().as_millis() < 900);
        let has_active_undo_anim = !reduced_motion && !self.undo_animations.is_empty();
        if (self.loading && !reduced_motion) || has_active_undo_anim {
            w.request_animation_frame();
        }
        let phase = (self.clock.elapsed().as_secs_f32() * 0.75).fract();
        let measure_cell = self.measured_width.clone();
        let narrow = measure_cell.get() < 560.;

        let header = self.render_header(w, cx);

        let body_content = if self.loading {
            self.render_loading_list(phase, cx).into_any_element()
        } else if self.empty || self.items.is_empty() {
            let cta_id = pro_eid(&self.id, "empty-permissions");
            let cta_label: SharedString = tr(cx, "Set permissions", "配置权限").into();
            div()
                .min_h_full()
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Activity,
                    tr(cx, "Nothing yet", "暂无活动记录"),
                    tr(
                        cx,
                        "Every step your agent takes in your apps shows up here as a sentence, with who let it and Undo while it can be taken back.",
                        "智能体在各应用中执行的每一步都会以完整句子记录于此，注明授权来源并支持在有效期内撤销。",
                    ),
                    Some((
                        cta_id,
                        cta_label,
                        Box::new(|_, _, cx| {
                            cx.emit(AgentActivityEvent::Navigate("Permissions".into()));
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else {
            let query = self.current_query(cx);
            let visible = self.filtered_indices(&query);
            if visible.is_empty() {
                let clear_id = pro_eid(&self.id, "clear-filters");
                let clear_label: SharedString = tr(cx, "Clear filters", "清除筛选").into();
                div()
                    .min_h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_empty_state(
                        ProIcon::Search,
                        tr(cx, "No matching steps", "没有匹配的步骤"),
                        tr(
                            cx,
                            "Try another search term or switch back to All steps.",
                            "请可改其他搜索关键词，或切换回全部步骤。",
                        ),
                        Some((
                            clear_id,
                            clear_label,
                            Box::new(|this: &mut Self, w, cx| {
                                this.filter = ActivityFilter::All;
                                this.mobile_search_open = false;
                                if let Some(ed) = this.search_editor.clone() {
                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                }
                                cx.notify();
                            }),
                        )),
                        cx,
                    ))
                    .into_any_element()
            } else {
                let mut groups: Vec<(SharedString, Vec<usize>)> = Vec::new();
                for idx in visible {
                    let day = self.items[idx].day.clone();
                    if let Some(last) = groups.last_mut().filter(|(d, _)| d == &day) {
                        last.1.push(idx);
                    } else {
                        groups.push((day, vec![idx]));
                    }
                }

                let dhead_bg = if p.dark {
                    hsla(0., 0., 1., 0.04)
                } else {
                    hsla(0., 0., 0., 0.035)
                };

                let mut list = div()
                    .px(px(8.))
                    .pt(px(10.))
                    .pb(px(40.))
                    .flex()
                    .flex_col()
                    .gap(px(16.));

                for (day, row_indices) in groups {
                    let day_label = self.translate_day(&day, cx).to_owned();
                    let count_str = row_indices.len().to_string();
                    let mut rows_col = div().flex().flex_col().gap(px(2.));
                    for idx in row_indices {
                        let item = &self.items[idx];
                        rows_col =
                            rows_col.child(self.render_row(item, narrow, reduced_motion, cx));
                    }
                    list = list.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .h(px(32.))
                                    .px(px(12.))
                                    .rounded(px(10.))
                                    .bg(dhead_bg)
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(day_label),
                                    )
                                    .child(
                                        div()
                                            .min_w(px(20.))
                                            .h(px(18.))
                                            .px(px(5.))
                                            .rounded(px(6.))
                                            .bg(p.card)
                                            .border_1()
                                            .border_color(p.line)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_size(px(11.5))
                                            .text_color(p.muted)
                                            .child(count_str),
                                    ),
                            )
                            .child(rows_col),
                    );
                }

                list.into_any_element()
            }
        };

        let body_content = if self.loading {
            body_content
        } else {
            let filter_key = format!("filter-{:?}-{}", self.filter, self.empty);
            pro_view_transition(pro_eid(&self.id, "list-vtx"), filter_key, body_content)
        };

        let status_label = self
            .last_announcement
            .clone()
            .unwrap_or_else(|| SharedString::from(""));

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .size_full()
            .min_h_0()
            .bg(p.panel)
            .overflow_hidden()
            .relative()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                this.handle_key_down(ev, w, cx);
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
            .child(
                div()
                    .id(pro_eid(&self.id, "list-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(body_content),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "sr-status"))
                    .role(Role::Status)
                    .aria_label(status_label)
                    .absolute()
                    .size_0(),
            )
    }

    fn render_inspector(
        &mut self,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let p = ProPalette::get(cx);
        if self.loading {
            let phase = (self.clock.elapsed().as_secs_f32() * 0.75).fract();
            return Some((
                tr(cx, "Step", "步骤").into(),
                ProIcon::Activity,
                self.render_loading_inspector(phase, cx).into_any_element(),
            ));
        }

        if self.empty || self.items.is_empty() {
            let empty_panel = div()
                .min_h(px(240.))
                .p(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(p.muted)
                .child(tr(cx, "Nothing open", "未打开任何步骤"));
            return Some((
                tr(cx, "Step", "步骤").into(),
                ProIcon::Activity,
                empty_panel.into_any_element(),
            ));
        }

        let Some(sel_id) = self.selected_id.clone() else {
            let empty_panel = div()
                .min_h(px(240.))
                .p(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "Select a step to inspect it",
                    "选择一个步骤查看详情",
                ));
            return Some((
                tr(cx, "Step", "步骤").into(),
                ProIcon::Activity,
                empty_panel.into_any_element(),
            ));
        };

        let item = self.items.iter().find(|it| it.id == sel_id).cloned()?;

        let app_meta = self.resolve_app(&item.app).cloned();
        let app_name = app_meta
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| item.app.clone());
        let subtitle: SharedString = match app_meta.and_then(|a| a.account) {
            Some(acc) => format!("{app_name} · {acc}").into(),
            None => app_name.clone(),
        };

        // 1. Header: App tile + sentence + app/account subtitle
        let head = div()
            .flex()
            .items_start()
            .gap(px(12.))
            .child(pro_app_tile(&app_name, 36., cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if item.undone { p.faint } else { p.ink })
                            .when(item.undone, |d| d.line_through())
                            .child(item.text.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.muted)
                            .truncate()
                            .child(subtitle),
                    ),
            );

        // 2. Props list: When / Task / Cost
        let day_str = self.translate_day(&item.day, cx).to_owned();
        let when_main = format!("{day_str} {}", item.time);
        let mut props = div().flex().flex_col().gap(px(2.)).child(
            div()
                .min_h(px(28.))
                .flex()
                .items_center()
                .child(
                    div()
                        .w(px(68.))
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(tr(cx, "When", "时间")),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .child(when_main)
                        .when_some(item.relative_time.clone(), |d, rel| {
                            d.child(
                                div()
                                    .truncate()
                                    .text_color(p.faint)
                                    .child(format!("· {rel}")),
                            )
                        }),
                ),
        );

        if let Some(task_name) = item.task.clone() {
            let task_emit = task_name.clone();
            props = props.child(
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(68.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Task", "关联任务")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .child(div().truncate().child(task_name.clone()))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "panel-open-task"))
                                    .role(Role::Button)
                                    .aria_label(format!(
                                        "{}: {task_name}",
                                        tr(cx, "Open task", "打开任务")
                                    ))
                                    .tab_index(0)
                                    .size(px(24.))
                                    .rounded(px(6.))
                                    .hover(move |s| s.bg(p.chip))
                                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .child(pro_icon(ProIcon::ChevronRight, 12., p.faint)),
                                cx,
                                move |_, _, cx| {
                                    cx.emit(AgentActivityEvent::OpenTask(task_emit.clone()));
                                },
                            )),
                    ),
            );
        }

        if let Some(cost) = item.cost.clone() {
            props = props.child(
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(68.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Cost", "花费")),
                    )
                    .child(div().text_size(px(13.)).text_color(p.ink).child(cost)),
            );
        }

        // 3. What happened section
        let what_section = if !item.details.is_empty() || item.preview.is_some() {
            let mut card = div()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .px(px(12.))
                .py(px(10.))
                .flex()
                .flex_col()
                .gap(px(10.));

            if !item.details.is_empty() {
                let mut dl = div().flex().flex_col().gap(px(4.));
                for det in &item.details {
                    dl = dl.child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .child(
                                div()
                                    .w(px(72.))
                                    .flex_none()
                                    .text_color(p.muted)
                                    .child(det.label.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(p.ink)
                                    .child(det.value.clone()),
                            ),
                    );
                }
                card = card.child(dl);
            }

            if let Some(preview_raw) = item.preview.clone() {
                let all_lines: Vec<&str> = preview_raw.lines().collect();
                let line_count = all_lines.len();
                let is_long = line_count > 6 || preview_raw.chars().count() > 230;
                let is_expanded = self.expanded_preview_id.as_ref() == Some(&item.id);
                let (head_preview, rest_preview): (SharedString, Option<SharedString>) = if is_long
                {
                    if line_count > 6 {
                        let head_str = all_lines[..6].join("\n");
                        let rest_str: SharedString = all_lines[6..].join("\n").into();
                        let shown_head: SharedString = if is_expanded {
                            head_str.into()
                        } else {
                            format!("{head_str}…").into()
                        };
                        (shown_head, Some(rest_str))
                    } else {
                        let chars: Vec<char> = preview_raw.chars().collect();
                        let split_at = chars[..220.min(chars.len())]
                            .iter()
                            .rposition(|c| c.is_whitespace())
                            .unwrap_or(220.min(chars.len()));
                        let head_str: String = chars[..split_at].iter().collect();
                        let rest_str: SharedString = chars[split_at..]
                            .iter()
                            .collect::<String>()
                            .trim_start()
                            .to_owned()
                            .into();
                        let shown_head: SharedString = if is_expanded {
                            head_str.into()
                        } else {
                            format!("{head_str}…").into()
                        };
                        (shown_head, Some(rest_str))
                    }
                } else {
                    (preview_raw, None)
                };
                let item_id_toggle = item.id.clone();

                let mut quote_box = div()
                    .w_full()
                    .pl(px(10.))
                    .border_l_2()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(head_preview);

                if let Some(rest_txt) = rest_preview {
                    let rest_el = div()
                        .w_full()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child(rest_txt)
                        .into_any_element();
                    quote_box = quote_box.child(pro_fold(
                        pro_eid(&self.id, format!("preview-fold-{}", item.id)),
                        is_expanded,
                        40.,
                        0.,
                        move || rest_el,
                    ));
                }

                let mut wrap = div()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(4.))
                    .child(quote_box);

                if is_long {
                    let toggle_lbl = if is_expanded {
                        tr(cx, "Show less", "收起")
                    } else {
                        tr(cx, "Show all", "展开全部")
                    };
                    wrap = wrap.child(activate(
                        div()
                            .id(pro_eid(&self.id, "panel-preview-more"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(22.))
                            .px(px(6.))
                            .ml(px(4.))
                            .rounded(px(6.))
                            .hover(move |s| s.bg(p.chip).text_color(p.ink))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .cursor_pointer()
                            .child(toggle_lbl),
                        cx,
                        move |this, _, cx| {
                            if this.expanded_preview_id.as_ref() == Some(&item_id_toggle) {
                                this.expanded_preview_id = None;
                            } else {
                                this.expanded_preview_id = Some(item_id_toggle.clone());
                            }
                            cx.notify();
                        },
                    ));
                }

                card = card.child(wrap);
            }

            Some(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "What happened", "执行详情")),
                    )
                    .child(card),
            )
        } else {
            None
        };

        // 4. Why it could section
        let (who_label, who_dot, why_sentence): (&str, Hsla, SharedString) = match item.by {
            ActivityActor::Auto => (
                tr(cx, "On its own", "自动执行"),
                if item.undone { p.faint } else { p.blue },
                tr(
                    cx,
                    "Allowed to run on its own under your workspace rules",
                    "依据工作区自动化权限规则直接执行",
                )
                .into(),
            ),
            ActivityActor::Approved => {
                let sentence: SharedString = match &item.approved_at {
                    Some(ts) => format!("You approved it {ts}").into(),
                    None => tr(cx, "You approved this step", "你已批准此步骤").into(),
                };
                (
                    tr(cx, "You approved", "你已批准"),
                    if item.undone { p.faint } else { p.amber },
                    sentence,
                )
            }
        };

        let mut why_card = div()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .px(px(12.))
            .py(px(10.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(div().size(px(6.)).rounded_full().bg(who_dot))
                    .child(who_label),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(why_sentence),
            );

        if let Some(rule_text) = item.rule.clone() {
            why_card = why_card.child(
                div()
                    .mt(px(2.))
                    .pt(px(8.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_icon(ProIcon::Shield, 13., p.muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .text_color(p.ink)
                            .child(rule_text),
                    )
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "panel-permissions-link"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(24.))
                            .pl(px(6.))
                            .pr(px(4.))
                            .rounded(px(6.))
                            .hover(move |s| s.bg(p.chip).text_color(p.ink))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .cursor_pointer()
                            .child(tr(cx, "Permissions", "权限策略"))
                            .child(pro_icon(ProIcon::ChevronRight, 12., p.muted)),
                        cx,
                        |_, _, cx| {
                            cx.emit(AgentActivityEvent::Navigate("Permissions".into()));
                        },
                    )),
            );
        }

        let why_section = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Why it could", "授权依据")),
            )
            .child(why_card);

        // 5. Taking it back section
        let undo_body = if item.undone {
            let note_text: SharedString = match &item.undo {
                Some(u) => format!("{} · {}", u.how, u.until).into(),
                None => tr(cx, "This step was taken back.", "此步骤已被撤销。").into(),
            };
            div()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .px(px(12.))
                .py(px(10.))
                .flex()
                .items_start()
                .gap(px(8.))
                .text_size(px(13.))
                .line_height(px(20.))
                .text_color(p.muted)
                .child(
                    div()
                        .mt(px(3.))
                        .child(pro_icon(ProIcon::Undo, 14., p.faint)),
                )
                .child(div().flex_1().min_w_0().child(note_text))
                .into_any_element()
        } else if let Some(u) = item.undo.clone() {
            let item_id_undo = item.id.clone();
            div()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .pl(px(12.))
                .pr(px(10.))
                .py(px(10.))
                .flex()
                .items_center()
                .gap(px(12.))
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
                                .text_color(p.ink)
                                .child(u.how),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.faint)
                                .child(u.until),
                        ),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "panel-undo-btn"),
                    tr(cx, "Undo", "撤销"),
                    Some(ProIcon::Undo),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    move |this, _, cx| {
                        this.undo_item(&item_id_undo, cx);
                    },
                ))
                .into_any_element()
        } else {
            let reason = item.final_reason.clone().unwrap_or_else(|| {
                tr(
                    cx,
                    "This step has already taken effect outside your workspace and cannot be undone.",
                    "此操作已在外部系统生效，无法自动撤回。",
                )
                .into()
            });
            div()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .px(px(12.))
                .py(px(10.))
                .flex()
                .items_start()
                .gap(px(8.))
                .text_size(px(13.))
                .line_height(px(20.))
                .text_color(p.muted)
                .child(
                    div()
                        .mt(px(3.))
                        .child(pro_icon(ProIcon::Lock, 14., p.faint)),
                )
                .child(div().flex_1().min_w_0().child(reason))
                .into_any_element()
        };

        let taking_back_section = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Taking it back", "撤销说明")),
            )
            .child(undo_body);

        let panel_body = div()
            .px(px(16.))
            .pt(px(14.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .gap(px(22.))
            .child(head)
            .child(props)
            .when_some(what_section, |d, ws| d.child(ws))
            .child(why_section)
            .child(taking_back_section);

        let panel_el = pro_view_transition(pro_eid(&self.id, "inspector-vtx"), sel_id, panel_body);

        Some((item.text, ProIcon::Activity, panel_el))
    }
}

impl Render for AgentActivityScreen {
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
        ActivityActor, ActivityFilter, ActivityUndoSpec, count_matching, default_activity_apps,
        default_activity_items,
    };

    #[test]
    fn default_catalog_counts_and_search_match_spec() {
        let apps = default_activity_apps();
        let items = default_activity_items();
        assert_eq!(apps.len(), 7);
        assert_eq!(count_matching(&items, ActivityFilter::All), 9);
        assert_eq!(count_matching(&items, ActivityFilter::Auto), 4);
        assert_eq!(count_matching(&items, ActivityFilter::Approved), 5);
        assert_eq!(count_matching(&items, ActivityFilter::Undone), 1);

        let today_n = items.iter().filter(|it| it.day.as_ref() == "Today").count();
        let yesterday_n = items
            .iter()
            .filter(|it| it.day.as_ref() == "Yesterday")
            .count();
        assert_eq!(today_n, 6);
        assert_eq!(yesterday_n, 3);

        let pr_matches: Vec<&str> = items
            .iter()
            .filter(|it| it.matches_query("#482", &it.app))
            .map(|it| it.id.as_ref())
            .collect();
        assert_eq!(pr_matches, vec!["a4"]);

        let webhook_auto: Vec<&str> = items
            .iter()
            .filter(|it| {
                it.matches_filter(ActivityFilter::Auto)
                    && it.matches_query("billing webhooks", &it.app)
            })
            .map(|it| it.id.as_ref())
            .collect();
        assert_eq!(webhook_auto, vec!["a2", "a3", "a4"]);
    }

    #[test]
    fn undo_eligibility_distinguishes_reversible_final_and_undone_steps() {
        let mut items = default_activity_items();
        // a1 has an undo window and is not yet undone
        assert!(items[0].can_undo());
        assert_eq!(items[0].by, ActivityActor::Approved);
        // a2 is Final (Stripe refund)
        assert!(!items[1].can_undo());
        assert!(items[1].final_reason.is_some());
        // a8 is already undone
        assert!(!items[7].can_undo());
        assert!(items[7].undone);

        // Optimistic undo + rollback simulation
        items[0].undone = true;
        assert!(!items[0].can_undo());
        assert_eq!(count_matching(&items, ActivityFilter::Undone), 2);

        items[0].undone = false;
        assert!(items[0].can_undo());
        assert_eq!(count_matching(&items, ActivityFilter::Undone), 1);

        let custom_undo = ActivityUndoSpec::new("Deletes draft", "Possible for 10m");
        assert_eq!(custom_undo.until.as_ref(), "Possible for 10m");
    }
}
