//! Recurring agent schedules screen with timeline activity chart and run inspector.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_eid, pro_empty_state, pro_icon_btn,
            pro_mark_selected, pro_mix_color, pro_pill_indicator, pro_presence, pro_progress_ring,
            pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_view_transition, tr,
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

const SVG_REPEAT: &str = r#"<path d="M16.3884 3L17.3913 3.97574C17.8393 4.41165 18.0633 4.62961 17.9844 4.81481C17.9056 5 17.5888 5 16.9552 5H9.19422C5.22096 5 2 8.13401 2 12C2 13.4872 2.47668 14.8662 3.2895 16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M7.61156 21L6.60875 20.0243C6.16074 19.5883 5.93673 19.3704 6.01557 19.1852C6.09441 19 6.4112 19 7.04478 19H14.8058C18.779 19 22 15.866 22 12C22 10.5128 21.5233 9.13383 20.7105 8" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const CHART_VB_W: f32 = 680.;
const CHART_PLOT_H: f32 = 114.;
const NOW_X_FRAC: f32 = 226.735 / CHART_VB_W;

const ACTIVITY_CURVE_PTS: [(f32, f32); 24] = [
    (9.4, 101.5),
    (18.8, 100.8),
    (28.3, 102.1),
    (37.7, 98.7),
    (47.2, 100.1),
    (56.6, 106.2),
    (66.1, 106.3),
    (75.5, 105.0),
    (85.0, 107.2),
    (94.4, 104.7),
    (103.8, 99.4),
    (113.3, 99.8),
    (122.7, 101.4),
    (132.2, 100.6),
    (141.6, 103.4),
    (151.1, 89.9),
    (160.5, 78.0),
    (170.0, 78.8),
    (179.4, 88.4),
    (188.8, 101.8),
    (198.3, 98.6),
    (207.7, 99.8),
    (217.2, 102.8),
    (226.7, 103.8),
];

const CHART_TICKS: [(f32, &str); 6] = [
    (47.175 / CHART_VB_W, "06:00"),
    (160.509 / CHART_VB_W, "08:00"),
    (273.842 / CHART_VB_W, "10:00"),
    (387.175 / CHART_VB_W, "12:00"),
    (500.509 / CHART_VB_W, "14:00"),
    (613.842 / CHART_VB_W, "16:00"),
];

/// Accent tone assigned to a recurring schedule mark and timeline stem.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScheduleTone {
    #[default]
    Orange,
    Purple,
    Teal,
    Blue,
    Amber,
    Pink,
    Green,
    Gray,
}

impl ScheduleTone {
    fn resolve(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Orange => p.orange,
            Self::Purple => p.purple,
            Self::Teal => p.teal,
            Self::Blue => p.blue,
            Self::Amber => p.amber,
            Self::Pink => p.pink,
            Self::Green => p.green,
            Self::Gray => {
                if p.dark {
                    hsla(0., 0., 1., 0.32)
                } else {
                    hsla(0., 0., 0., 0.30)
                }
            }
        }
    }
}

/// Outcome badge for the most recent or historical execution of a schedule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScheduleOutcome {
    #[default]
    Done,
    NeedsYou,
    Failed,
}

impl ScheduleOutcome {
    /// Localized short badge text for this outcome.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Done => tr(cx, "Done", "已完成"),
            Self::NeedsYou => tr(cx, "Needs you", "待确认"),
            Self::Failed => tr(cx, "Failed", "失败"),
        }
    }
}

/// One upcoming scheduled execution slot shown in the inspector's `Next runs` list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UpcomingRun {
    pub when: SharedString,
    pub relative: SharedString,
    pub skipped: bool,
}

impl UpcomingRun {
    /// Construct an upcoming run entry with formatted timestamp and relative hint.
    pub fn new(when: impl Into<SharedString>, relative: impl Into<SharedString>) -> Self {
        Self {
            when: when.into(),
            relative: relative.into(),
            skipped: false,
        }
    }

    /// Mark whether this upcoming run is skipped.
    pub fn with_skipped(mut self, skipped: bool) -> Self {
        self.skipped = skipped;
        self
    }
}

/// Historical run entry shown in the inspector's `Last runs` list.
#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleRunRecord {
    pub at: SharedString,
    pub outcome: ScheduleOutcome,
    pub summary: SharedString,
    pub cost_eur: Option<f32>,
}

impl ScheduleRunRecord {
    /// Construct a completed or failed historical run record.
    pub fn new(
        at: impl Into<SharedString>,
        outcome: ScheduleOutcome,
        summary: impl Into<SharedString>,
        cost_eur: Option<f32>,
    ) -> Self {
        Self {
            at: at.into(),
            outcome,
            summary: summary.into(),
            cost_eur,
        }
    }
}

/// Execution state of a run marker along the `Around now` timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelineRunState {
    Ran,
    Failed,
    Running,
    Next,
}

/// Interactive run marker plotted on the `Around now` timeline chart.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineMarker {
    pub schedule_id: SharedString,
    pub schedule_name: SharedString,
    pub time_label: SharedString,
    pub status_label: SharedString,
    pub tone: ScheduleTone,
    pub x_fraction: f32,
    pub state: TimelineRunState,
}

/// Recurring agent schedule model rendered in [`AgentSchedulesScreen`].
#[derive(Clone, Debug, PartialEq)]
pub struct AgentSchedule {
    pub id: SharedString,
    pub name: SharedString,
    pub project: Option<SharedString>,
    pub tone: ScheduleTone,
    pub cadence: SharedString,
    pub prompt: SharedString,
    pub next_when: SharedString,
    pub next_relative: SharedString,
    pub last_outcome: ScheduleOutcome,
    pub cost_eur: f32,
    pub duration_ms: u32,
    pub paused: bool,
    pub apps: Vec<SharedString>,
    pub upcoming: Vec<UpcomingRun>,
    pub history: Vec<ScheduleRunRecord>,
}

impl AgentSchedule {
    /// Construct a recurring schedule with human-readable cadence.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        cadence: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            project: None,
            tone: ScheduleTone::Orange,
            cadence: cadence.into(),
            prompt: SharedString::default(),
            next_when: "Tomorrow 09:00".into(),
            next_relative: "in 24 h".into(),
            last_outcome: ScheduleOutcome::Done,
            cost_eur: 0.25,
            duration_ms: 6_400,
            paused: false,
            apps: Vec::new(),
            upcoming: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Attach a workspace or project name displayed in the inspector subtitle.
    pub fn with_project(mut self, project: impl Into<SharedString>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Set the color tone for row marks and timeline markers.
    pub fn with_tone(mut self, tone: ScheduleTone) -> Self {
        self.tone = tone;
        self
    }

    /// Set the agent instructions executed on each run.
    pub fn with_prompt(mut self, prompt: impl Into<SharedString>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Set the formatted next run timestamp and relative countdown label.
    pub fn with_next(
        mut self,
        when: impl Into<SharedString>,
        relative: impl Into<SharedString>,
    ) -> Self {
        self.next_when = when.into();
        self.next_relative = relative.into();
        self
    }

    /// Set the badge outcome of the last run.
    pub fn with_last_outcome(mut self, outcome: ScheduleOutcome) -> Self {
        self.last_outcome = outcome;
        self
    }

    /// Set the average cost per run in euros.
    pub fn with_cost(mut self, cost_eur: f32) -> Self {
        self.cost_eur = cost_eur;
        self
    }

    /// Set the simulated run duration in milliseconds for `Run now` progress.
    pub fn with_duration_ms(mut self, duration_ms: u32) -> Self {
        self.duration_ms = duration_ms.max(400);
        self
    }

    /// Configure whether this schedule starts paused.
    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Attach connected app names rendered with [`pro_app_tile`].
    pub fn with_apps<I, S>(mut self, apps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.apps = apps.into_iter().map(Into::into).collect();
        self
    }

    /// Set the upcoming run occurrences shown in the right inspector.
    pub fn with_upcoming(mut self, upcoming: Vec<UpcomingRun>) -> Self {
        self.upcoming = upcoming;
        self
    }

    /// Set the historical run records shown in the right inspector.
    pub fn with_history(mut self, history: Vec<ScheduleRunRecord>) -> Self {
        self.history = history;
        self
    }
}

/// Format a euro run cost, shortening large figures (`>= 1000`) to `€1.3K`.
pub fn format_schedule_cost(cost_eur: f32) -> String {
    if cost_eur >= 1000. {
        format!("€{:.1}K", cost_eur / 1000.)
    } else {
        format!("€{cost_eur:.2}")
    }
}

/// Filter tab for [`AgentSchedulesScreen`] when toolbar filtering is shown or driven programmatically.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScheduleFilter {
    #[default]
    All,
    Active,
    Paused,
}

impl ScheduleFilter {
    fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Active => "active",
            Self::Paused => "paused",
        }
    }
}

/// Events emitted by [`AgentSchedulesScreen`].
#[derive(Clone, Debug, PartialEq)]
pub enum AgentSchedulesEvent {
    Select(SharedString),
    PauseChanged {
        id: SharedString,
        paused: bool,
    },
    RunNow(SharedString),
    SkipChanged {
        id: SharedString,
        when: SharedString,
        skipped: bool,
    },
    CreateRequested,
    Navigate(SharedString),
}

impl EventEmitter<AgentSchedulesEvent> for AgentSchedulesScreen {}

#[derive(Clone)]
struct ActiveManualRun {
    schedule_id: SharedString,
    started_at: Instant,
    duration_ms: u32,
}

/// Pro Agent Schedules screen with around-now activity line, schedule list, and run inspector.
pub struct AgentSchedulesScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    schedules: Vec<AgentSchedule>,
    markers: Vec<TimelineMarker>,
    selected_id: Option<SharedString>,
    filter: ScheduleFilter,
    show_toolbar_actions: bool,
    prompt_expanded: bool,
    create_modal_open: bool,
    draft_preset_idx: usize,
    draft_cadence_idx: usize,
    draft_tone: ScheduleTone,
    cadence_popover_open: bool,
    running: Option<ActiveManualRun>,
    clock_origin: Instant,
    loading: bool,
    empty: bool,
    standalone: bool,
    measured_width: Rc<Cell<f32>>,
}

impl Focusable for AgentSchedulesScreen {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AgentSchedulesScreen {
    /// Construct an [`AgentSchedulesScreen`] preloaded with the canonical `Fieldwork` schedules.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Agent, shell_cx)
                .with_active("schedules")
                .with_work_area(true, true);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => cx.emit(AgentSchedulesEvent::Navigate(label.clone())),
            AppShellEvent::PanelOpen(_) => cx.notify(),
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            focus: cx.focus_handle(),
            schedules: Self::default_schedules(),
            markers: Self::default_markers(),
            selected_id: Some("scan".into()),
            filter: ScheduleFilter::All,
            show_toolbar_actions: false,
            prompt_expanded: false,
            create_modal_open: false,
            draft_preset_idx: 0,
            draft_cadence_idx: 0,
            draft_tone: ScheduleTone::Blue,
            cadence_popover_open: false,
            running: None,
            clock_origin: Instant::now(),
            loading: false,
            empty: false,
            standalone: false,
            measured_width: Rc::new(Cell::new(760.)),
        }
    }

    /// Canonical 7 sample schedules for the `Fieldwork` agent workspace.
    pub fn default_schedules() -> Vec<AgentSchedule> {
        vec![
            AgentSchedule::new("scan", "Competitor scan", "Every Monday at 09:10")
                .with_project("Northwind")
                .with_tone(ScheduleTone::Orange)
                .with_prompt(
                    "Check the nine makers' sites and shops for new prices, models and campaigns – and tell me what changed since last week.",
                )
                .with_next("Today 09:10", "in a moment")
                .with_last_outcome(ScheduleOutcome::Done)
                .with_cost(3.10)
                .with_duration_ms(6_400)
                .with_upcoming(vec![
                    UpcomingRun::new("Today 09:10", "in a moment"),
                    UpcomingRun::new("Nov 30 09:10", "in 7 days"),
                ])
                .with_history(vec![
                    ScheduleRunRecord::new(
                        "Nov 16 09:10",
                        ScheduleOutcome::Done,
                        "2 makers cut prices, 1 new model",
                        Some(3.42),
                    ),
                    ScheduleRunRecord::new(
                        "Nov 9 09:10",
                        ScheduleOutcome::Done,
                        "Nothing changed",
                        Some(2.80),
                    ),
                    ScheduleRunRecord::new(
                        "Nov 2 09:10",
                        ScheduleOutcome::Failed,
                        "Three shops blocked the browser",
                        Some(1.20),
                    ),
                ]),
            AgentSchedule::new("triage", "Inbox triage", "Every 2 hours, 8:00 – 20:00")
                .with_project("Fieldwork")
                .with_tone(ScheduleTone::Purple)
                .with_prompt(
                    "Sort new mail into Needs reply, Waiting and FYI, draft short replies in our voice for routine questions, and flag anything urgent from active clients.",
                )
                .with_next("Today 10:00", "in 50 min")
                .with_last_outcome(ScheduleOutcome::Done)
                .with_cost(0.18)
                .with_duration_ms(4_800)
                .with_upcoming(vec![
                    UpcomingRun::new("Today 10:00", "in 50 min"),
                    UpcomingRun::new("Today 12:00", "in 3 h"),
                    UpcomingRun::new("Today 14:00", "in 5 h"),
                    UpcomingRun::new("Today 16:00", "in 7 h"),
                ])
                .with_history(vec![
                    ScheduleRunRecord::new(
                        "Today 08:00",
                        ScheduleOutcome::Done,
                        "Archived 14 newsletters, drafted 3 replies",
                        Some(0.19),
                    ),
                    ScheduleRunRecord::new(
                        "Yesterday 20:00",
                        ScheduleOutcome::Done,
                        "Flagged contract addendum from Northwind",
                        Some(0.21),
                    ),
                    ScheduleRunRecord::new(
                        "Yesterday 18:00",
                        ScheduleOutcome::Done,
                        "Quiet window – 4 FYI threads filed",
                        Some(0.14),
                    ),
                ]),
            AgentSchedule::new("standup", "Standup notes", "Every weekday at 16:30")
                .with_project("Fieldwork")
                .with_tone(ScheduleTone::Purple)
                .with_prompt(
                    "Collect merged pull requests, closed issues and Slack updates since yesterday afternoon, then post a calm five-line summary in #team-daily.",
                )
                .with_next("Today 16:30", "in 7 h")
                .with_last_outcome(ScheduleOutcome::Done)
                .with_cost(0.26)
                .with_duration_ms(5_200)
                .with_upcoming(vec![
                    UpcomingRun::new("Today 16:30", "in 7 h"),
                    UpcomingRun::new("Tomorrow 16:30", "in 31 h"),
                    UpcomingRun::new("Wed 16:30", "in 2 days"),
                ])
                .with_history(vec![
                    ScheduleRunRecord::new(
                        "Fri 16:30",
                        ScheduleOutcome::Done,
                        "Posted summary: 6 PRs merged, 2 blockers",
                        Some(0.28),
                    ),
                    ScheduleRunRecord::new(
                        "Thu 16:30",
                        ScheduleOutcome::Done,
                        "Posted summary: Q4 pricing QA green",
                        Some(0.24),
                    ),
                ]),
            AgentSchedule::new("numbers", "Northwind numbers", "Every weekday at 7:45")
                .with_project("Northwind")
                .with_tone(ScheduleTone::Orange)
                .with_prompt(
                    "Pull yesterday's orders, refunds and trial conversions from Stripe and Postgres, update the Northwind KPI sheet, and note any metric off by more than 15%.",
                )
                .with_next("Tomorrow 07:45", "in 23 h")
                .with_last_outcome(ScheduleOutcome::Done)
                .with_cost(0.42)
                .with_duration_ms(5_600)
                .with_upcoming(vec![
                    UpcomingRun::new("Tomorrow 07:45", "in 23 h"),
                    UpcomingRun::new("Wed 07:45", "in 2 days"),
                    UpcomingRun::new("Thu 07:45", "in 3 days"),
                ])
                .with_history(vec![
                    ScheduleRunRecord::new(
                        "Today 07:45",
                        ScheduleOutcome::Done,
                        "MRR +€1,420 · net retention steady at 104%",
                        Some(0.41),
                    ),
                    ScheduleRunRecord::new(
                        "Fri 07:45",
                        ScheduleOutcome::Done,
                        "Updated KPI sheet · 0 anomalies",
                        Some(0.44),
                    ),
                ]),
            AgentSchedule::new("weekly", "Weekly update", "Every Friday at 16:00")
                .with_project("Fieldwork")
                .with_tone(ScheduleTone::Orange)
                .with_prompt(
                    "Draft the Friday stakeholder memo from completed tasks, revenue highlights and open risks, then leave it in Inbox for approval before sending.",
                )
                .with_next("Fri 16:00", "in 4 days")
                .with_last_outcome(ScheduleOutcome::NeedsYou)
                .with_cost(0.55)
                .with_duration_ms(6_000)
                .with_upcoming(vec![
                    UpcomingRun::new("Fri 16:00", "in 4 days"),
                    UpcomingRun::new("Dec 4 16:00", "in 11 days"),
                ])
                .with_history(vec![
                    ScheduleRunRecord::new(
                        "Nov 20 16:00",
                        ScheduleOutcome::NeedsYou,
                        "Draft ready – waiting on Q4 hiring line",
                        Some(0.58),
                    ),
                    ScheduleRunRecord::new(
                        "Nov 13 16:00",
                        ScheduleOutcome::Done,
                        "Sent to 8 recipients after approval",
                        Some(0.52),
                    ),
                ]),
            AgentSchedule::new("invoices", "Invoice reminders", "On the 1st of every month")
                .with_project("Finance")
                .with_tone(ScheduleTone::Gray)
                .with_prompt(
                    "Check Stripe for invoices more than 7 days past due and prepare polite follow-up drafts in Gmail for each billing contact.",
                )
                .with_next("Dec 1 09:00", "in 8 days")
                .with_last_outcome(ScheduleOutcome::Failed)
                .with_cost(0.30)
                .with_duration_ms(5_000)
                .with_upcoming(vec![
                    UpcomingRun::new("Dec 1 09:00", "in 8 days"),
                    UpcomingRun::new("Jan 1 09:00", "in 39 days"),
                ])
                .with_history(vec![
                    ScheduleRunRecord::new(
                        "Nov 1 09:00",
                        ScheduleOutcome::Failed,
                        "Stripe token expired – reconnect in Permissions",
                        Some(0.08),
                    ),
                    ScheduleRunRecord::new(
                        "Oct 1 09:00",
                        ScheduleOutcome::Done,
                        "Prepared 3 reminder drafts, all paid within 48h",
                        Some(0.31),
                    ),
                ]),
            AgentSchedule::new("evals", "Nightly evals", "Every night at 2:00")
                .with_project("Engineering")
                .with_tone(ScheduleTone::Teal)
                .with_prompt(
                    "Run the 120-case regression suite against staging prompts, compare accuracy and latency against last night's baseline, and file an issue if pass rate drops below 97%.",
                )
                .with_next("Tomorrow 02:00", "in 17 h")
                .with_last_outcome(ScheduleOutcome::Done)
                .with_cost(4.80)
                .with_duration_ms(7_200)
                .with_paused(true)
                .with_upcoming(vec![
                    UpcomingRun::new("Tomorrow 02:00", "in 17 h"),
                    UpcomingRun::new("Wed 02:00", "in 41 h"),
                ])
                .with_history(vec![
                    ScheduleRunRecord::new(
                        "Nov 18 02:00",
                        ScheduleOutcome::Done,
                        "118 / 120 passed (98.3%) · p95 1.8s",
                        Some(4.80),
                    ),
                    ScheduleRunRecord::new(
                        "Nov 17 02:00",
                        ScheduleOutcome::Done,
                        "119 / 120 passed (99.2%) · p95 1.7s",
                        Some(4.74),
                    ),
                ]),
        ]
    }

    fn default_markers() -> Vec<TimelineMarker> {
        vec![
            TimelineMarker {
                schedule_id: "numbers".into(),
                schedule_name: "Northwind numbers".into(),
                time_label: "07:45".into(),
                status_label: "Done".into(),
                tone: ScheduleTone::Orange,
                x_fraction: 146.342 / CHART_VB_W,
                state: TimelineRunState::Ran,
            },
            TimelineMarker {
                schedule_id: "triage".into(),
                schedule_name: "Inbox triage".into(),
                time_label: "08:00".into(),
                status_label: "Done".into(),
                tone: ScheduleTone::Purple,
                x_fraction: 160.509 / CHART_VB_W,
                state: TimelineRunState::Ran,
            },
            TimelineMarker {
                schedule_id: "scan".into(),
                schedule_name: "Competitor scan".into(),
                time_label: "09:10".into(),
                status_label: "in a moment".into(),
                tone: ScheduleTone::Orange,
                x_fraction: 226.735 / CHART_VB_W,
                state: TimelineRunState::Next,
            },
            TimelineMarker {
                schedule_id: "triage".into(),
                schedule_name: "Inbox triage".into(),
                time_label: "10:00".into(),
                status_label: "in 50 min".into(),
                tone: ScheduleTone::Purple,
                x_fraction: 273.842 / CHART_VB_W,
                state: TimelineRunState::Next,
            },
            TimelineMarker {
                schedule_id: "triage".into(),
                schedule_name: "Inbox triage".into(),
                time_label: "12:00".into(),
                status_label: "in 3 h".into(),
                tone: ScheduleTone::Purple,
                x_fraction: 387.175 / CHART_VB_W,
                state: TimelineRunState::Next,
            },
            TimelineMarker {
                schedule_id: "triage".into(),
                schedule_name: "Inbox triage".into(),
                time_label: "14:00".into(),
                status_label: "in 5 h".into(),
                tone: ScheduleTone::Purple,
                x_fraction: 500.509 / CHART_VB_W,
                state: TimelineRunState::Next,
            },
            TimelineMarker {
                schedule_id: "triage".into(),
                schedule_name: "Inbox triage".into(),
                time_label: "16:00".into(),
                status_label: "in 7 h".into(),
                tone: ScheduleTone::Purple,
                x_fraction: 613.842 / CHART_VB_W,
                state: TimelineRunState::Next,
            },
            TimelineMarker {
                schedule_id: "standup".into(),
                schedule_name: "Standup notes".into(),
                time_label: "16:30".into(),
                status_label: "in 7 h".into(),
                tone: ScheduleTone::Purple,
                x_fraction: 642.175 / CHART_VB_W,
                state: TimelineRunState::Next,
            },
        ]
    }

    /// Replace the displayed schedules list.
    pub fn with_schedules(mut self, schedules: Vec<AgentSchedule>) -> Self {
        if self
            .selected_id
            .as_ref()
            .is_none_or(|sel| !schedules.iter().any(|s| &s.id == sel))
        {
            self.selected_id = schedules.first().map(|s| s.id.clone());
        }
        self.schedules = schedules;
        self
    }

    /// Override the initially selected schedule open in the right inspector.
    pub fn with_selected(mut self, selected: Option<impl Into<SharedString>>) -> Self {
        self.selected_id = selected.map(Into::into);
        self
    }

    /// Set the active status filter (`All`, `Active`, or `Paused`).
    pub fn with_filter(mut self, filter: ScheduleFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Show optional filter tabs and `+ New schedule` button in the page header.
    pub fn with_toolbar_actions(mut self, show: bool) -> Self {
        self.show_toolbar_actions = show;
        self
    }

    /// Configure skeleton loading placeholder mode.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure explicit empty state mode.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Render only the schedules canvas without the outer [`AppShell`] chrome.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Update loading mode dynamically.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Update empty state mode dynamically.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Return the current schedules slice.
    pub fn schedules(&self) -> &[AgentSchedule] {
        &self.schedules
    }

    /// Return the currently selected schedule id.
    pub fn selected_id(&self) -> Option<&SharedString> {
        self.selected_id.as_ref()
    }

    /// Count schedules that are currently active (not paused).
    pub fn active_count(&self) -> usize {
        count_active_schedules(&self.schedules)
    }

    /// Select a schedule and open the right-hand inspector.
    pub fn select_schedule(&mut self, id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let id = id.into();
        if self.selected_id.as_ref() != Some(&id) {
            self.prompt_expanded = false;
            self.cadence_popover_open = false;
        }
        self.selected_id = Some(id.clone());
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentSchedulesEvent::Select(id));
        cx.notify();
    }

    /// Toggle the paused state of a schedule by id.
    pub fn toggle_pause(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(sched) = self.schedules.iter_mut().find(|s| s.id.as_ref() == id) {
            sched.paused = !sched.paused;
            let paused = sched.paused;
            let sid = sched.id.clone();
            cx.emit(AgentSchedulesEvent::PauseChanged { id: sid, paused });
            cx.notify();
        }
    }

    /// Start an immediate manual execution of a schedule on the timeline.
    pub fn trigger_run_now(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(sched) = self.schedules.iter().find(|s| s.id.as_ref() == id) else {
            return;
        };
        let sid = sched.id.clone();
        let duration_ms = sched.duration_ms;
        if Theme::global(cx).reduced_motion {
            self.complete_manual_run(&sid);
        } else {
            self.running = Some(ActiveManualRun {
                schedule_id: sid.clone(),
                started_at: Instant::now(),
                duration_ms,
            });
        }
        cx.emit(AgentSchedulesEvent::RunNow(sid));
        cx.notify();
    }

    /// Toggle whether a specific upcoming run index is skipped.
    pub fn toggle_skip_run(&mut self, id: &str, run_index: usize, cx: &mut Context<Self>) {
        if let Some(sched) = self.schedules.iter_mut().find(|s| s.id.as_ref() == id)
            && let Some(run) = sched.upcoming.get_mut(run_index)
        {
            run.skipped = !run.skipped;
            let sid = sched.id.clone();
            let when = run.when.clone();
            let skipped = run.skipped;
            cx.emit(AgentSchedulesEvent::SkipChanged {
                id: sid,
                when,
                skipped,
            });
            cx.notify();
        }
    }

    /// Append a new schedule and open it in the right inspector.
    pub fn add_schedule(&mut self, schedule: AgentSchedule, cx: &mut Context<Self>) {
        let id = schedule.id.clone();
        self.schedules.insert(0, schedule);
        self.selected_id = Some(id.clone());
        self.empty = false;
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentSchedulesEvent::Select(id));
        cx.notify();
    }

    fn complete_manual_run(&mut self, schedule_id: &str) {
        if let Some(sched) = self
            .schedules
            .iter_mut()
            .find(|s| s.id.as_ref() == schedule_id)
        {
            sched.last_outcome = ScheduleOutcome::Done;
            sched.history.insert(
                0,
                ScheduleRunRecord::new(
                    "Just now",
                    ScheduleOutcome::Done,
                    "Completed manual run",
                    Some(sched.cost_eur),
                ),
            );
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        filter_schedule_indices(&self.schedules, self.filter)
    }

    fn step_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let indices = self.visible_indices();
        if indices.is_empty() {
            return;
        }
        let cur_pos = self
            .selected_id
            .as_ref()
            .and_then(|sel| {
                indices
                    .iter()
                    .position(|&idx| &self.schedules[idx].id == sel)
            })
            .unwrap_or(0);
        let next_pos = (cur_pos as isize + delta).clamp(0, indices.len() as isize - 1) as usize;
        let next_id = self.schedules[indices[next_pos]].id.clone();
        if self.selected_id.as_ref() != Some(&next_id) {
            self.prompt_expanded = false;
            self.selected_id = Some(next_id.clone());
            cx.emit(AgentSchedulesEvent::Select(next_id));
            cx.notify();
        }
    }

    fn on_key_down(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if self.loading || self.empty {
            return;
        }
        let key = ev.keystroke.key.as_str();
        let plain = !ev.keystroke.modifiers.platform
            && !ev.keystroke.modifiers.control
            && !ev.keystroke.modifiers.alt;
        if !plain {
            return;
        }
        match key {
            "j" | "down" => {
                cx.stop_propagation();
                self.step_selection(1, cx);
            }
            "k" | "up" => {
                cx.stop_propagation();
                self.step_selection(-1, cx);
            }
            "enter" => {
                if let Some(sel) = self.selected_id.clone() {
                    cx.stop_propagation();
                    self.select_schedule(sel, cx);
                }
            }
            " " | "space" => {
                if let Some(sel) = self.selected_id.clone() {
                    cx.stop_propagation();
                    self.toggle_pause(sel.as_ref(), cx);
                }
            }
            "r" => {
                if let Some(sel) = self.selected_id.clone() {
                    cx.stop_propagation();
                    self.trigger_run_now(sel.as_ref(), cx);
                }
            }
            "escape" => {
                if self.create_modal_open {
                    cx.stop_propagation();
                    self.create_modal_open = false;
                    cx.notify();
                } else if self.cadence_popover_open {
                    cx.stop_propagation();
                    self.cadence_popover_open = false;
                    cx.notify();
                } else if self.shell.read(cx).is_panel_open() {
                    cx.stop_propagation();
                    self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
                }
            }
            _ => {}
        }
    }

    fn sync_manual_run(&mut self, w: &mut Window) -> Option<(SharedString, SharedString, f32)> {
        let run = self.running.clone()?;
        let elapsed_ms = run.started_at.elapsed().as_secs_f32() * 1000.;
        let pct = (elapsed_ms / run.duration_ms.max(1) as f32).clamp(0., 1.);
        if pct >= 1.0 {
            let sid = run.schedule_id.clone();
            self.running = None;
            self.complete_manual_run(sid.as_ref());
            w.request_animation_frame();
            return None;
        }
        w.request_animation_frame();
        let name = self
            .schedules
            .iter()
            .find(|s| s.id == run.schedule_id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| run.schedule_id.clone());
        Some((run.schedule_id, name, pct))
    }

    fn render_header(&self, compact_shell: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let panel_open = self.standalone || self.shell.read(cx).is_panel_open();
        let active_n = self.active_count();
        let total_n = self.schedules.len();

        let mut head = div()
            .h(px(48.))
            .w_full()
            .flex_none()
            .pl(px(if compact_shell && !self.standalone {
                46.
            } else {
                20.
            }))
            .pr(px(if !panel_open && !self.standalone {
                52.
            } else {
                16.
            }))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(tr(cx, "Schedules", "定时计划")),
            );

        if !self.loading && !self.empty && !self.schedules.is_empty() {
            head = head.child(
                div()
                    .text_size(px(13.))
                    .text_color(p.faint)
                    .child(format!("{active_n} / {total_n}")),
            );
        }

        if self.show_toolbar_actions && !self.loading {
            let pill_row = self.render_filter_pills("flt", cx).ml_auto();
            head = head.child(pill_row).child(pro_btn(
                pro_eid(&self.id, "new-schedule"),
                tr(cx, "New schedule", "新建计划"),
                Some(ProIcon::Plus),
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, _, cx| {
                    this.create_modal_open = true;
                    this.cadence_popover_open = false;
                    cx.emit(AgentSchedulesEvent::CreateRequested);
                    cx.notify();
                },
            ));
        }

        head
    }

    fn render_filter_pills(&self, id_prefix: &'static str, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let sensor = pro_selection_sensor();
        let active_key: SharedString = self.filter.key().into();
        let filters = [
            (ScheduleFilter::All, tr(cx, "All", "全部")),
            (ScheduleFilter::Active, tr(cx, "Active", "启用中")),
            (ScheduleFilter::Paused, tr(cx, "Paused", "已暂停")),
        ];
        let mut pill_row = div()
            .id(pro_eid(&self.id, format!("{id_prefix}-tablist")))
            .role(Role::TabList)
            .relative()
            .flex()
            .items_center()
            .gap(px(3.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, format!("{id_prefix}-pill-ind")),
                active_key,
                sensor.clone(),
                6.,
                p.chip,
                None,
                false,
            ));
        for (flt, label) in filters {
            let selected = self.filter == flt;
            pill_row = pill_row.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("{id_prefix}-{label}")))
                    .role(Role::Tab)
                    .aria_label(label)
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
                    .text_size(px(12.5))
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if selected { p.ink } else { p.muted })
                    .cursor_pointer()
                    .when(selected, |d| d.child(pro_mark_selected(&sensor)))
                    .child(label),
                cx,
                move |this, _, cx| {
                    this.filter = flt;
                    cx.notify();
                },
            ));
        }
        div().child(pill_row)
    }

    fn render_create_modal(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = ProPalette::get(cx);
        let presets: [(&str, &str, &str, [&str; 2], f32); 3] = [
            (
                tr(cx, "Morning digest", "晨间简报"),
                "Fieldwork",
                "Summarize overnight customer replies and open blockers into a short morning brief.",
                ["Gmail", "Slack"],
                0.24,
            ),
            (
                tr(cx, "PR & CI watch", "PR 与构建巡检"),
                "Engineering",
                "Check open pull requests with failing checks or stale reviews and post a digest in #eng.",
                ["GitHub", "Slack"],
                0.31,
            ),
            (
                tr(cx, "Revenue pulse", "营收异动监控"),
                "Finance",
                "Compare daily trials, upgrades and failed charges against the 14-day baseline.",
                ["Stripe", "Linear"],
                0.28,
            ),
        ];
        let cadences: [(&str, &str, &str); 3] = [
            (
                tr(cx, "Weekdays 08:30", "工作日 08:30"),
                "Every weekday at 08:30",
                "Tomorrow 08:30",
            ),
            (
                tr(cx, "Every 2 hours", "每 2 小时"),
                "Every 2 hours, 8:00 – 20:00",
                "Today 10:00",
            ),
            (
                tr(cx, "Mondays 09:00", "每周一 09:00"),
                "Every Monday at 09:00",
                "Mon 09:00",
            ),
        ];

        let preset_sensor = pro_selection_sensor();
        let mut preset_tabs = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "modal-preset-ind"),
                format!("preset-{}", self.draft_preset_idx),
                preset_sensor.clone(),
                6.,
                p.card,
                Some(p.line),
                true,
            ));
        for (pi, (ptitle, _, _, _, _)) in presets.iter().enumerate() {
            let sel = self.draft_preset_idx == pi;
            preset_tabs = preset_tabs.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("modal-preset-{pi}")))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .flex_1()
                    .h(px(26.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .font_weight(if sel {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if sel { p.ink } else { p.muted })
                    .cursor_pointer()
                    .when(sel, |d| d.child(pro_mark_selected(&preset_sensor)))
                    .child(*ptitle),
                cx,
                move |this, _, cx| {
                    this.draft_preset_idx = pi;
                    cx.notify();
                },
            ));
        }

        let cad_sensor = pro_selection_sensor();
        let mut cad_tabs = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "modal-cad-ind"),
                format!("cad-{}", self.draft_cadence_idx),
                cad_sensor.clone(),
                6.,
                p.card,
                Some(p.line),
                true,
            ));
        for (ci, (clbl, _, _)) in cadences.iter().enumerate() {
            let sel = self.draft_cadence_idx == ci;
            cad_tabs = cad_tabs.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("modal-cad-{ci}")))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .flex_1()
                    .h(px(26.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .font_weight(if sel {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if sel { p.ink } else { p.muted })
                    .cursor_pointer()
                    .when(sel, |d| d.child(pro_mark_selected(&cad_sensor)))
                    .child(*clbl),
                cx,
                move |this, _, cx| {
                    this.draft_cadence_idx = ci;
                    cx.notify();
                },
            ));
        }

        let tones = [
            ScheduleTone::Blue,
            ScheduleTone::Orange,
            ScheduleTone::Purple,
            ScheduleTone::Teal,
            ScheduleTone::Green,
        ];
        let mut tone_row = div().flex().items_center().gap(px(6.));
        for (ti, tone) in tones.into_iter().enumerate() {
            let active = self.draft_tone == tone;
            let tc = tone.resolve(&p);
            tone_row = tone_row.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("modal-tone-{ti}")))
                    .role(Role::RadioButton)
                    .tab_index(0)
                    .size(px(22.))
                    .rounded_full()
                    .when(active, |d| d.border_2().border_color(tc))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(div().size(px(12.)).rounded_full().bg(tc)),
                cx,
                move |this, _, cx| {
                    this.draft_tone = tone;
                    cx.notify();
                },
            ));
        }

        let cur_preset = presets[self.draft_preset_idx.min(presets.len() - 1)];
        let cur_cad = cadences[self.draft_cadence_idx.min(cadences.len() - 1)];

        let dialog = div()
            .w_full()
            .max_w(px(440.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .px(px(16.))
                    .pt(px(14.))
                    .pb(px(10.))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "New schedule", "新建定时计划")),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "sched-modal-close"),
                        tr(cx, "Close", "关闭"),
                        ProIcon::Close,
                        28.,
                        false,
                        cx,
                        |this, _, cx| {
                            this.create_modal_open = false;
                            cx.notify();
                        },
                    )),
            )
            .child(
                div()
                    .p(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Workflow template", "工作流模板")),
                            )
                            .child(preset_tabs),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Cadence", "执行频率")),
                            )
                            .child(cad_tabs),
                    )
                    .child(
                        div()
                            .p(px(10.))
                            .rounded(px(8.))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.line)
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.text)
                            .child(cur_preset.2),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Accent color", "标记颜色")),
                            )
                            .child(tone_row),
                    ),
            )
            .child(
                div()
                    .px(px(16.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(pro_btn(
                        pro_eid(&self.id, "sched-modal-cancel"),
                        tr(cx, "Cancel", "取消"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, _, cx| {
                            this.create_modal_open = false;
                            cx.notify();
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "sched-modal-submit"),
                        tr(cx, "Create schedule", "创建计划"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| {
                            let idx = this.schedules.len() + 1;
                            let created = AgentSchedule::new(
                                format!("custom-{idx}"),
                                format!("{} #{idx}", cur_preset.0),
                                cur_cad.1,
                            )
                            .with_project(cur_preset.1)
                            .with_tone(this.draft_tone)
                            .with_apps(cur_preset.3)
                            .with_prompt(cur_preset.2)
                            .with_next(cur_cad.2, "in 23 h")
                            .with_cost(cur_preset.4)
                            .with_upcoming(vec![
                                UpcomingRun::new(cur_cad.2, "in 23 h"),
                                UpcomingRun::new("Next week", "in 7 days"),
                            ]);
                            this.create_modal_open = false;
                            this.add_schedule(created, cx);
                        },
                    )),
            );

        let backdrop = div()
            .id(pro_eid(&self.id, "sched-modal-scrim"))
            .size_full()
            .p(px(16.))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.create_modal_open = false;
                    cx.notify();
                }),
            )
            .child(dialog);

        let scrim_base = if p.dark { 0.44 } else { 0.18 };
        pro_presence(
            pro_eid(&self.id, "create-sched-pres"),
            self.create_modal_open,
            0.22,
            0.16,
            move |amount| {
                div()
                    .absolute()
                    .inset_0()
                    .bg(hsla(0., 0., 0., scrim_base * amount))
                    .child(
                        div()
                            .size_full()
                            .relative()
                            .top(px(8. * (1. - amount)))
                            .opacity(amount)
                            .child(backdrop),
                    )
                    .into_any_element()
            },
        )
    }

    fn render_around_now_section(
        &self,
        live_run: Option<(SharedString, SharedString, f32)>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let ambient_secs = self.clock_origin.elapsed().as_secs_f32();

        // Filter out coming markers whose schedule is currently paused.
        let mut active_markers: Vec<TimelineMarker> = self
            .markers
            .iter()
            .filter(|m| {
                if m.state != TimelineRunState::Next {
                    return true;
                }
                self.schedules
                    .iter()
                    .find(|s| s.id == m.schedule_id)
                    .is_none_or(|s| !s.paused)
            })
            .cloned()
            .collect();

        if let Some((ref run_id, ref run_name, pct)) = live_run {
            let tone = self
                .schedules
                .iter()
                .find(|s| &s.id == run_id)
                .map(|s| s.tone)
                .unwrap_or(ScheduleTone::Blue);
            let mut found = false;
            for m in &mut active_markers {
                if &m.schedule_id == run_id && (m.x_fraction - NOW_X_FRAC).abs() < 0.02 {
                    m.state = TimelineRunState::Running;
                    m.status_label = format!("{}%", (pct * 100.).round() as u32).into();
                    found = true;
                    break;
                }
            }
            if !found {
                active_markers.push(TimelineMarker {
                    schedule_id: run_id.clone(),
                    schedule_name: run_name.clone(),
                    time_label: "Now".into(),
                    status_label: format!("{}%", (pct * 100.).round() as u32).into(),
                    tone,
                    x_fraction: NOW_X_FRAC,
                    state: TimelineRunState::Running,
                });
            }
        }

        let rate_num = if live_run.is_some() { 4 } else { 1 };
        let rate_unit = if rate_num == 1 {
            tr(cx, "action / 10 min", "次动作 / 10 分钟")
        } else {
            tr(cx, "actions / 10 min", "次动作 / 10 分钟")
        };

        let paint_markers = active_markers.clone();
        let run_lift = live_run
            .as_ref()
            .map(|(_, _, pct)| (pct * std::f32::consts::PI).sin())
            .unwrap_or(0.);
        let glow_pulse = if reduced {
            1.0
        } else {
            0.5 + 0.5 * ((ambient_secs / 2.2) * std::f32::consts::TAU).sin()
        };
        if !reduced && live_run.is_some() {
            w.request_animation_frame();
        }

        let plot_height = 96.0_f32;
        let marker_top_px = (32. / CHART_PLOT_H) * plot_height;

        let mut plot_box = div().relative().w_full().h(px(plot_height)).child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let bw = f32::from(bounds.size.width);
                    let bh = f32::from(bounds.size.height);
                    if bw <= 8. || bh <= 8. {
                        return;
                    }
                    let sx = |vx: f32| bounds.left() + px((vx / CHART_VB_W) * bw);
                    let sy = |vy: f32| bounds.top() + px((vy / CHART_PLOT_H) * bh);

                    // Horizontal subtle grid lines
                    for gy in [36.0_f32, 62.0, 88.0] {
                        let mut gl = PathBuilder::stroke(px(1.));
                        let y = sy(gy);
                        gl.move_to(point(bounds.left(), y));
                        gl.line_to(point(bounds.right(), y));
                        if let Ok(path) = gl.build() {
                            window.paint_path(path, p.line.opacity(0.42));
                        }
                    }

                    // Solid baseline
                    let base_y = sy(CHART_PLOT_H - 0.5);
                    let mut base = PathBuilder::stroke(px(1.));
                    base.move_to(point(bounds.left(), base_y));
                    base.line_to(point(bounds.right(), base_y));
                    if let Ok(path) = base.build() {
                        window.paint_path(path, p.line);
                    }

                    // Vertical run markers (stems + top glyphs)
                    let top_y = sy(32.);
                    for m in &paint_markers {
                        let mx = bounds.left() + px(m.x_fraction * bw);
                        let tone_c = if m.state == TimelineRunState::Failed {
                            p.red
                        } else {
                            m.tone.resolve(&p)
                        };
                        match m.state {
                            TimelineRunState::Next => {
                                paint_dashed_vertical(
                                    window,
                                    mx,
                                    top_y + px(3.5),
                                    base_y,
                                    tone_c.opacity(0.45),
                                );
                                paint_disc(window, point(mx, top_y), 3.6, p.card);
                                paint_circle_stroke(window, point(mx, top_y), 3.0, 1.5, tone_c);
                            }
                            TimelineRunState::Running => {
                                let mut stem = PathBuilder::stroke(px(1.4));
                                stem.move_to(point(mx, top_y));
                                stem.line_to(point(mx, base_y));
                                if let Ok(path) = stem.build() {
                                    window.paint_path(path, tone_c.opacity(0.9));
                                }
                                paint_disc(window, point(mx, top_y), 4.5, tone_c.opacity(0.24));
                                paint_disc(window, point(mx, top_y), 2.8, tone_c);
                            }
                            TimelineRunState::Ran | TimelineRunState::Failed => {
                                let alpha = if m.state == TimelineRunState::Failed {
                                    0.8
                                } else {
                                    0.55
                                };
                                let mut stem = PathBuilder::stroke(px(1.));
                                stem.move_to(point(mx, top_y));
                                stem.line_to(point(mx, base_y));
                                if let Ok(path) = stem.build() {
                                    window.paint_path(path, tone_c.opacity(alpha));
                                }
                                paint_square(window, point(mx, top_y), 2.5, tone_c);
                            }
                        }
                    }

                    // Smooth interpolated activity curve up to `now`
                    let mut adjusted: Vec<(f32, f32)> = ACTIVITY_CURVE_PTS.to_vec();
                    if run_lift > 0.001 {
                        let n = adjusted.len();
                        for (i, pt) in adjusted.iter_mut().enumerate() {
                            if i + 5 >= n {
                                let w_tail = (i + 5 - n + 1) as f32 / 5.0;
                                pt.1 -= 32.0 * run_lift * w_tail * w_tail;
                            }
                        }
                    }
                    let samples = sample_smooth_curve(&adjusted, 5);
                    if samples.len() >= 2 {
                        let first_pt = point(sx(samples[0].0), sy(samples[0].1));
                        let last_raw = samples[samples.len() - 1];
                        let head_pt = point(sx(last_raw.0), sy(last_raw.1));

                        let mut area = PathBuilder::fill();
                        area.move_to(point(first_pt.x, base_y));
                        area.line_to(first_pt);
                        for &(vx, vy) in samples.iter().skip(1) {
                            area.line_to(point(sx(vx), sy(vy)));
                        }
                        area.line_to(point(head_pt.x, base_y));
                        area.close();
                        if let Ok(path) = area.build() {
                            window.paint_path(
                                path,
                                linear_gradient(
                                    180.,
                                    linear_color_stop(p.blue.opacity(0.15), 0.),
                                    linear_color_stop(p.blue.opacity(0.), 1.),
                                ),
                            );
                        }

                        let mut line = PathBuilder::stroke(px(1.75));
                        line.move_to(first_pt);
                        for &(vx, vy) in samples.iter().skip(1) {
                            line.line_to(point(sx(vx), sy(vy)));
                        }
                        if let Ok(path) = line.build() {
                            window.paint_path(path, p.blue);
                        }

                        if !reduced {
                            let glow_r = 7.0 + 4.0 * glow_pulse;
                            let glow_a = 0.22 - 0.12 * glow_pulse;
                            paint_disc(window, head_pt, glow_r, p.blue.opacity(glow_a));
                        }
                        paint_disc(window, head_pt, 4.8, p.card);
                        paint_disc(window, head_pt, 3.3, p.blue);
                    }
                },
            )
            .size_full(),
        );

        // Interactive keyboard/pointer buttons on each run marker
        for (mi, marker) in active_markers.iter().enumerate() {
            let sid = marker.schedule_id.clone();
            let aria = format!(
                "{}, {}, {}",
                marker.schedule_name, marker.time_label, marker.status_label
            );
            plot_box = plot_box.child(activate(
                div()
                    .id(pro_eid(
                        &self.id,
                        format!("mark-{mi}-{}", marker.schedule_id),
                    ))
                    .role(Role::Button)
                    .aria_label(aria)
                    .tab_index(0)
                    .absolute()
                    .left(relative(marker.x_fraction))
                    .top(px((marker_top_px - 10.).max(0.)))
                    .ml(px(-10.))
                    .size(px(20.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus)),
                cx,
                move |this, _, cx| {
                    this.select_schedule(sid.clone(), cx);
                },
            ));
        }

        // Floating progress chip beside the glowing head while a run is active
        if let Some((_, ref run_name, pct)) = live_run {
            let pct_num = (pct * 100.).round() as u32;
            plot_box = plot_box.child(
                div()
                    .absolute()
                    .left(relative(NOW_X_FRAC))
                    .ml(px(10.))
                    .top(px(8.))
                    .max_w(px(220.))
                    .px(px(8.))
                    .py(px(3.))
                    .rounded(px(7.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .child(pro_progress_ring(pct, 13., 1.8, p.blue, p.line_strong))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(run_name.clone()),
                    )
                    .child(div().text_color(p.muted).child(format!("{pct_num}%"))),
            );
        }

        // X-axis clock ticks row
        let mut ticks_row = div().relative().w_full().h(px(16.)).mt(px(3.));
        for (tx, label) in CHART_TICKS {
            ticks_row = ticks_row.child(
                div()
                    .absolute()
                    .left(relative(tx))
                    .ml(px(-18.))
                    .w(px(36.))
                    .flex()
                    .justify_center()
                    .text_size(px(11.))
                    .text_color(p.faint)
                    .child(label),
            );
        }

        // Status line under the chart
        let status_row = if let Some((_, run_name, pct)) = live_run {
            let pct_num = (pct * 100.).round() as u32;
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .min_w_0()
                .text_size(px(13.))
                .child(div().size(px(7.)).flex_none().rounded_full().bg(p.blue))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(tr(cx, "Running now:", "正在运行：")),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(p.text)
                        .child(run_name),
                )
                .child(div().text_color(p.faint).child(format!("· {pct_num}%")))
        } else if let Some(next_sched) = self.schedules.iter().find(|s| !s.paused) {
            let next_time = short_time_part(&next_sched.next_when);
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .min_w_0()
                .text_size(px(13.))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(tr(cx, "Next:", "下次运行：")),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(p.text)
                        .child(next_sched.name.clone()),
                )
                .child(
                    div()
                        .text_color(p.faint)
                        .child(format!("· {next_time}, {}", next_sched.next_relative)),
                )
        } else {
            div().text_size(px(13.)).text_color(p.faint).child(tr(
                cx,
                "All schedules paused",
                "所有定时计划均已暂停",
            ))
        };

        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Around now", "当前时段")),
            )
            .child(
                div()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .px(px(16.))
                    .py(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.))
                            .text_size(px(12.))
                            .child(div().text_color(p.muted).child(tr(
                                cx,
                                "Agent activity",
                                "智能体活动",
                            )))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(rate_num.to_string()),
                                    )
                                    .child(div().text_color(p.muted).child(rate_unit)),
                            ),
                    )
                    .child(div().flex().flex_col().child(plot_box).child(ticks_row))
                    .child(status_row),
            )
    }

    fn render_schedule_row(
        &self,
        sched: &AgentSchedule,
        narrow: bool,
        tiny: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let selected = self.selected_id.as_ref() == Some(&sched.id);
        let sid = sched.id.clone();
        let sid_switch = sched.id.clone();
        let tone_c = sched.tone.resolve(&p);

        let mark = schedule_tone_mark(tone_c, sched.paused);
        let row_bg = if selected {
            if p.dark {
                hsla(0., 0., 1., 0.045)
            } else {
                hsla(0., 0., 0., 0.038)
            }
        } else {
            transparent_black()
        };

        let aria_row = if sched.paused {
            format!(
                "{}, {}, {}, last run {}",
                sched.name,
                sched.cadence,
                tr(cx, "Paused", "已暂停"),
                sched.last_outcome.label(cx)
            )
        } else {
            format!(
                "{}, {}, next {}, last run {}",
                sched.name,
                sched.cadence,
                sched.next_when,
                sched.last_outcome.label(cx)
            )
        };

        let mut main_btn = div()
            .id(pro_eid(&self.id, format!("row-{}", sched.id)))
            .role(Role::Button)
            .aria_label(aria_row)
            .tab_index(0)
            .flex_1()
            .min_w_0()
            .min_h(px(if tiny { 56. } else { 42. }))
            .pl(px(12.))
            .py(px(if tiny { 8. } else { 0. }))
            .rounded(px(10.))
            .flex()
            .items_center()
            .gap(px(12.))
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus));

        if tiny {
            let next_str = if sched.paused {
                tr(cx, "Paused", "已暂停").to_string()
            } else {
                sched.next_when.to_string()
            };
            main_btn = main_btn
                .items_start()
                .child(div().mt(px(6.)).child(mark))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if sched.paused { p.text } else { p.ink })
                                .child(sched.name.clone()),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(sched.cadence.clone()),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(if sched.paused { p.faint } else { p.ink })
                                .child(next_str),
                        ),
                );
        } else {
            let mut text_group = div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(
                    div()
                        .flex_none()
                        .max_w(px(200.))
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if sched.paused { p.text } else { p.ink })
                        .child(sched.name.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .child(sched.cadence.clone()),
                );

            if !sched.apps.is_empty() {
                let mut app_strip = div().flex_none().flex().items_center().gap(px(4.));
                for app in sched.apps.iter().take(3) {
                    app_strip = app_strip.child(pro_app_tile(app, 18., cx));
                }
                text_group = text_group.child(app_strip);
            }

            let next_col = if sched.paused {
                div()
                    .flex_none()
                    .when(!narrow, |d| d.w(px(190.)))
                    .flex()
                    .justify_end()
                    .text_size(px(12.5))
                    .text_color(p.faint)
                    .child(tr(cx, "Paused", "已暂停"))
            } else {
                div()
                    .flex_none()
                    .when(!narrow, |d| d.w(px(190.)))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(6.))
                    .whitespace_nowrap()
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .child(sched.next_when.clone()),
                    )
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(sched.next_relative.clone()),
                        )
                    })
            };

            main_btn = main_btn.child(mark).child(text_group).child(next_col);

            if !narrow {
                main_btn = main_btn
                    .child(
                        div()
                            .flex_none()
                            .w(px(78.))
                            .flex()
                            .justify_end()
                            .child(outcome_badge(sched.last_outcome, cx)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .w(px(60.))
                            .flex()
                            .justify_end()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(format_schedule_cost(sched.cost_eur)),
                    );
            }
        }

        let switch_aria = format!(
            "{}: {}",
            sched.name,
            if sched.paused {
                tr(cx, "Resume", "恢复")
            } else {
                tr(cx, "Pause", "暂停")
            }
        );

        div()
            .pr(px(10.))
            .rounded(px(10.))
            .bg(row_bg)
            .hover(move |s| s.bg(if selected { row_bg } else { p.hover }))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(activate(main_btn, cx, move |this, _, cx| {
                this.select_schedule(sid.clone(), cx);
            }))
            .child(schedule_capsule_switch(
                pro_eid(&self.id, format!("sw-{}", sched.id)),
                switch_aria,
                !sched.paused,
                cx,
                move |this, _, cx| {
                    this.toggle_pause(sid_switch.as_ref(), cx);
                },
            ))
    }

    fn render_loading_body(&self, narrow: bool, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        let phase = 0.25_f32;
        let mut rows = div().mx(px(-10.)).flex().flex_col().gap(px(2.));
        let widths = [
            (116., 148., 126.),
            (92., 162., 118.),
            (104., 140., 112.),
            (128., 136., 130.),
            (98., 128., 108.),
            (120., 154., 122.),
            (90., 118., 74.),
        ];
        for (i, (nw, cw, xw)) in widths.into_iter().enumerate() {
            let ph = phase + i as f32 * 0.07;
            rows = rows.child(
                div()
                    .h(px(42.))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(pro_skeleton_bar(8., 8., 2., ph, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(pro_skeleton_bar(nw, 12., 4., ph, cx))
                            .child(pro_skeleton_bar(cw, 11., 4., ph + 0.1, cx)),
                    )
                    .child(pro_skeleton_bar(xw, 11., 4., ph + 0.15, cx))
                    .when(!narrow, |d| {
                        d.child(pro_skeleton_bar(48., 18., 4., ph + 0.2, cx))
                            .child(pro_skeleton_bar(42., 11., 4., ph + 0.25, cx))
                    })
                    .child(pro_skeleton_bar(30., 18., 9., ph + 0.3, cx)),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap(px(28.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(72., 11., 4., phase, cx))
                    .child(
                        div()
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .px(px(16.))
                            .py(px(14.))
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(pro_skeleton_bar(84., 11., 4., phase, cx))
                                    .child(pro_skeleton_bar(96., 11., 4., phase + 0.1, cx)),
                            )
                            .child(pro_skeleton_bar(320., 84., 6., phase + 0.15, cx))
                            .child(pro_skeleton_bar(196., 12., 4., phase + 0.2, cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(64., 11., 4., phase, cx))
                    .child(rows),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let live_run = self.sync_manual_run(w);

        let measure_cell = self.measured_width.clone();
        let page_w = measure_cell.get();
        let narrow = page_w <= 640.;
        let tiny = page_w <= 440.;
        let compact_shell = page_w <= 560.;

        let header = self.render_header(compact_shell, cx);

        let content = if self.loading {
            self.render_loading_body(narrow, cx).into_any_element()
        } else if self.empty || self.schedules.is_empty() {
            pro_empty_state(
                ProIcon::Calendar,
                tr(cx, "No schedules yet", "暂无定时计划"),
                tr(
                    cx,
                    "Ask your agent in Thread to repeat any task – every morning, every weekday or on the first of the month.",
                    "在会话中让智能体按固定节奏重复执行任务——每天早晨、每个工作日或每月一日。",
                ),
                Some((
                    pro_eid(&self.id, "empty-ask-thread"),
                    tr(cx, "Ask in Thread", "在会话中创建").into(),
                    Box::new(|this: &mut Self, _, cx| {
                        this.shell.update(cx, |s, _| s.set_active("thread"));
                        cx.emit(AgentSchedulesEvent::Navigate("Thread".into()));
                        cx.notify();
                    }),
                )),
                cx,
            )
            .into_any_element()
        } else {
            let visible = self.visible_indices();
            let mut list = div().mx(px(-10.)).flex().flex_col().gap(px(2.));
            if visible.is_empty() {
                list = list.child(
                    div()
                        .py(px(20.))
                        .px(px(12.))
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "No schedules match this filter.",
                            "当前筛选条件下暂无定时计划。",
                        )),
                );
            } else {
                for idx in visible {
                    let sched = self.schedules[idx].clone();
                    list = list.child(self.render_schedule_row(&sched, narrow, tiny, cx));
                }
            }

            let list_vtx =
                pro_view_transition(pro_eid(&self.id, "sched-list-vtx"), self.filter.key(), list);

            let mut sec_head = div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(tr(cx, "Schedules", "计划列表")),
                );
            if !self.show_toolbar_actions {
                sec_head = sec_head.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(self.render_filter_pills("sec-flt", cx))
                        .child(pro_btn(
                            pro_eid(&self.id, "sec-new-schedule"),
                            tr(cx, "New", "新建"),
                            Some(ProIcon::Plus),
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, _, cx| {
                                this.create_modal_open = true;
                                this.cadence_popover_open = false;
                                cx.emit(AgentSchedulesEvent::CreateRequested);
                                cx.notify();
                            },
                        )),
                );
            }

            div()
                .flex()
                .flex_col()
                .gap(px(if narrow { 24. } else { 28. }))
                .child(self.render_around_now_section(live_run, w, cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(sec_head)
                        .child(list_vtx),
                )
                .into_any_element()
        };

        let modal_layer = self.render_create_modal(cx);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .relative()
            .size_full()
            .bg(p.panel)
            .flex()
            .flex_col()
            .overflow_hidden()
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
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        div().w_full().flex().justify_center().child(
                            div()
                                .w_full()
                                .max_w(px(760.))
                                .px(px(if narrow { 16. } else { 24. }))
                                .pt(px(if narrow { 16. } else { 24. }))
                                .pb(px(if narrow { 40. } else { 48. }))
                                .child(content),
                        ),
                    ),
            )
            .child(modal_layer)
    }

    fn render_inspector(
        &mut self,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let p = ProPalette::get(cx);

        if self.loading {
            let ph = 0.2_f32;
            let skel = div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(10.))
                        .child(div().mt(px(6.)).child(pro_skeleton_bar(8., 8., 2., ph, cx)))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(pro_skeleton_bar(136., 15., 4., ph, cx))
                                .child(pro_skeleton_bar(178., 11., 4., ph + 0.1, cx)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(82., 28., 6., ph + 0.15, cx))
                        .child(pro_skeleton_bar(70., 28., 6., ph + 0.2, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(72., 11., 4., ph, cx))
                        .child(pro_skeleton_bar(240., 46., 6., ph + 0.1, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(64., 11., 4., ph, cx))
                        .child(pro_skeleton_bar(240., 80., 10., ph + 0.15, cx)),
                )
                .into_any_element();
            return Some((tr(cx, "Schedule", "计划").into(), ProIcon::Refresh, skel));
        }

        if self.empty || self.schedules.is_empty() {
            let empty_panel = div()
                .min_h(px(240.))
                .p(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(p.faint)
                .child(tr(cx, "Nothing open", "未打开任何计划"))
                .into_any_element();
            return Some((
                tr(cx, "Schedule", "计划").into(),
                ProIcon::Refresh,
                empty_panel,
            ));
        }

        let sel_id = self.selected_id.as_ref()?;
        let sched = self.schedules.iter().find(|s| &s.id == sel_id)?.clone();
        let tone_c = sched.tone.resolve(&p);
        let sid_run = sched.id.clone();
        let sid_pause = sched.id.clone();

        let is_running_this = self
            .running
            .as_ref()
            .is_some_and(|r| r.schedule_id == sched.id);
        let run_pct = self
            .running
            .as_ref()
            .filter(|r| r.schedule_id == sched.id)
            .map(|r| {
                (r.started_at.elapsed().as_secs_f32() * 1000. / r.duration_ms.max(1) as f32)
                    .clamp(0., 1.)
            })
            .unwrap_or(0.);

        let subtitle = match &sched.project {
            Some(proj) => format!("{} · {proj}", sched.cadence),
            None => sched.cadence.to_string(),
        };

        // Run now button (`ui-ags-btn--dark`)
        let run_btn = activate(
            div()
                .id(pro_eid(&self.id, "panel-run-now"))
                .role(Role::Button)
                .aria_label(tr(cx, "Run now", "立即运行"))
                .tab_index(0)
                .h(px(28.))
                .pl(px(8.))
                .pr(px(10.))
                .rounded(px(6.))
                .bg(p.primary)
                .hover(move |s| s.bg(p.primary.opacity(0.88)))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.on_primary)
                .cursor_pointer()
                .child(if is_running_this {
                    pro_progress_ring(run_pct, 13., 1.8, p.on_primary, p.on_primary.opacity(0.32))
                } else {
                    pro_icon(ProIcon::Play, 13., p.on_primary)
                })
                .child(if is_running_this {
                    tr(cx, "Running…", "运行中…")
                } else {
                    tr(cx, "Run now", "立即运行")
                }),
            cx,
            move |this, _, cx| {
                this.trigger_run_now(sid_run.as_ref(), cx);
            },
        );

        let pause_label = if sched.paused {
            tr(cx, "Resume", "恢复")
        } else {
            tr(cx, "Pause", "暂停")
        };
        let pause_icon = if sched.paused {
            ProIcon::Play
        } else {
            ProIcon::Pause
        };
        let pause_btn = activate(
            div()
                .id(pro_eid(&self.id, "panel-pause"))
                .role(Role::Button)
                .aria_label(pause_label)
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
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .cursor_pointer()
                .child(pro_icon(pause_icon, 13., p.muted))
                .child(pause_label),
            cx,
            move |this, _, cx| {
                this.toggle_pause(sid_pause.as_ref(), cx);
            },
        );

        // Prompt section (`What it does`)
        let long_prompt = sched.prompt.chars().count() > 220;
        let shown_prompt: SharedString = if long_prompt && !self.prompt_expanded {
            let prefix: String = sched.prompt.chars().take(220).collect();
            format!("{prefix}…").into()
        } else {
            sched.prompt.clone()
        };

        let mut prompt_sec = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "What it does", "执行说明")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .child(
                        div()
                            .w(px(2.))
                            .flex_none()
                            .rounded(px(1.))
                            .bg(p.line_strong),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.text)
                            .child(shown_prompt),
                    ),
            );

        if long_prompt {
            let toggle_txt = if self.prompt_expanded {
                tr(cx, "Show less", "收起")
            } else {
                tr(cx, "Show all", "展开全部")
            };
            prompt_sec = prompt_sec.child(activate(
                div()
                    .id(pro_eid(&self.id, "toggle-prompt"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(24.))
                    .px(px(6.))
                    .rounded(px(5.))
                    .hover(move |s| s.bg(p.chip).text_color(p.ink))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .cursor_pointer()
                    .child(toggle_txt),
                cx,
                |this, _, cx| {
                    this.prompt_expanded = !this.prompt_expanded;
                    cx.notify();
                },
            ));
        }

        // Next runs section
        let next_runs_body = if sched.paused {
            div()
                .text_size(px(13.))
                .text_color(p.faint)
                .child(tr(
                    cx,
                    "Paused – resume to schedule upcoming runs.",
                    "已暂停——恢复后将继续安排后续运行。",
                ))
                .into_any_element()
        } else if sched.upcoming.is_empty() {
            div()
                .text_size(px(13.))
                .text_color(p.faint)
                .child(tr(cx, "No upcoming runs", "暂无后续运行"))
                .into_any_element()
        } else {
            let mut card = div()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .px(px(12.))
                .py(px(2.))
                .flex()
                .flex_col();

            for (ri, run) in sched.upcoming.iter().enumerate() {
                let sid_skip = sched.id.clone();
                let skipped = run.skipped;
                let action_lbl = if skipped {
                    tr(cx, "Restore", "恢复")
                } else {
                    tr(cx, "Skip", "跳过")
                };
                let when_el = div()
                    .relative()
                    .flex_none()
                    .text_size(px(13.))
                    .text_color(if skipped { p.faint } else { p.ink })
                    .child(run.when.clone())
                    .when(skipped, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .top(px(9.5))
                                .h(px(1.))
                                .bg(p.faint),
                        )
                    });

                card = card.child(
                    div()
                        .min_h(px(40.))
                        .when(ri > 0, |d| d.border_t_1().border_color(p.line_soft))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(when_el)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(if skipped {
                                    SharedString::from(tr(cx, "Skipped", "已跳过"))
                                } else {
                                    run.relative.clone()
                                }),
                        )
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("skip-{}-{ri}", sched.id)))
                                .role(Role::Button)
                                .aria_label(format!("{action_lbl}: {}", run.when))
                                .tab_index(0)
                                .h(px(26.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(p.chip).text_color(p.ink))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .flex()
                                .items_center()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .cursor_pointer()
                                .child(action_lbl),
                            cx,
                            move |this, _, cx| {
                                this.toggle_skip_run(sid_skip.as_ref(), ri, cx);
                            },
                        )),
                );
            }
            card.into_any_element()
        };

        // Last runs history section
        let history_body = if sched.history.is_empty() {
            div()
                .text_size(px(13.))
                .text_color(p.faint)
                .child(tr(cx, "No past runs yet", "暂无历史运行记录"))
                .into_any_element()
        } else {
            let mut card = div()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .px(px(12.))
                .py(px(2.))
                .flex()
                .flex_col();

            for (hi, rec) in sched.history.iter().enumerate() {
                let meta_line = match rec.cost_eur {
                    Some(c) => format!("{} · {}", rec.at, format_schedule_cost(c)),
                    None => rec.at.to_string(),
                };
                card = card.child(
                    div()
                        .py(px(8.))
                        .when(hi > 0, |d| d.border_t_1().border_color(p.line_soft))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(1.))
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.ink)
                                        .child(rec.summary.clone()),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.faint)
                                        .child(meta_line),
                                ),
                        )
                        .child(outcome_badge(rec.outcome, cx)),
                );
            }
            card.into_any_element()
        };

        let cadence_presets = [
            "Every Monday at 09:10",
            "Every weekday at 08:30",
            "Every 2 hours, 8:00 – 20:00",
            "Every Friday at 16:00",
            "Every night at 02:00",
        ];
        let sid_cad = sched.id.clone();
        let cad_menu = div()
            .id(pro_eid(&self.id, "cadence-menu"))
            .role(Role::Menu)
            .w(px(224.))
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_md()
            .flex()
            .flex_col()
            .gap(px(1.))
            .children(cadence_presets.into_iter().enumerate().map(|(ci, cad)| {
                let is_cur = sched.cadence.as_ref() == cad;
                let sid_opt = sid_cad.clone();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("cad-opt-{ci}")))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .text_color(if is_cur { p.ink } else { p.text })
                        .cursor_pointer()
                        .child(div().truncate().child(cad))
                        .when(is_cur, |d| d.child(pro_icon(ProIcon::Check, 12., p.ink))),
                    cx,
                    move |this, _, cx| {
                        if let Some(s) = this.schedules.iter_mut().find(|x| x.id == sid_opt) {
                            s.cadence = cad.into();
                        }
                        this.cadence_popover_open = false;
                        cx.notify();
                    },
                )
            }));

        let cad_pop_layer = pro_presence(
            pro_eid(&self.id, "cad-pop-pres"),
            self.cadence_popover_open,
            0.18,
            0.14,
            move |amount| {
                div()
                    .relative()
                    .top(px(-4. * (1. - amount)))
                    .opacity(amount)
                    .child(cad_menu)
                    .into_any_element()
            },
        );

        let mut panel = div()
            .p(px(14.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(
                div()
                    .relative()
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .child(
                        div()
                            .mt(px(7.))
                            .child(schedule_tone_mark(tone_c, sched.paused)),
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
                                    .text_size(px(15.))
                                    .line_height(px(22.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(sched.name.clone()),
                            )
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "panel-edit-cadence"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .ml(px(-4.))
                                    .px(px(4.))
                                    .h(px(20.))
                                    .rounded(px(5.))
                                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .min_w_0()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .cursor_pointer()
                                    .child(pro_svg("0 0 24 24", SVG_REPEAT, 12., p.muted))
                                    .child(div().min_w_0().truncate().child(subtitle))
                                    .child(pro_icon(ProIcon::ChevronDown, 11., p.faint)),
                                cx,
                                |this, _, cx| {
                                    this.cadence_popover_open = !this.cadence_popover_open;
                                    cx.notify();
                                },
                            )),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(px(18.))
                            .top(px(48.))
                            .child(cad_pop_layer),
                    ),
            )
            .child(
                div()
                    .mt(px(-6.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(run_btn)
                    .child(pause_btn)
                    .child(
                        div()
                            .ml_auto()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(format!(
                                "{} {}",
                                format_schedule_cost(sched.cost_eur),
                                tr(cx, "per run", "每次")
                            )),
                    ),
            );

        if !sched.apps.is_empty() {
            let mut apps_row = div().flex().flex_wrap().items_center().gap(px(6.));
            for app in &sched.apps {
                apps_row = apps_row.child(
                    div()
                        .h(px(26.))
                        .pl(px(5.))
                        .pr(px(9.))
                        .rounded(px(7.))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(p.ink)
                        .child(pro_app_tile(app, 18., cx))
                        .child(app.clone()),
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
                            .child(tr(cx, "Connected apps", "已连接应用")),
                    )
                    .child(apps_row),
            );
        }

        panel = panel
            .child(prompt_sec)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Next runs", "后续运行")),
                    )
                    .child(next_runs_body),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Last runs", "历史记录")),
                    )
                    .child(history_body),
            );

        Some((
            sched.name.clone(),
            ProIcon::Refresh,
            pro_view_transition(pro_eid(&self.id, "sched-insp-vtx"), sched.id.clone(), panel),
        ))
    }
}

impl Render for AgentSchedulesScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

/// Count how many schedules in `schedules` are currently active (not paused).
pub fn count_active_schedules(schedules: &[AgentSchedule]) -> usize {
    schedules.iter().filter(|s| !s.paused).count()
}

/// Filter schedule indices matching [`ScheduleFilter`].
pub fn filter_schedule_indices(schedules: &[AgentSchedule], filter: ScheduleFilter) -> Vec<usize> {
    schedules
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let keep = match filter {
                ScheduleFilter::All => true,
                ScheduleFilter::Active => !s.paused,
                ScheduleFilter::Paused => s.paused,
            };
            keep.then_some(i)
        })
        .collect()
}

fn short_time_part(when: &str) -> &str {
    when.split_whitespace().next_back().unwrap_or(when)
}

fn schedule_tone_mark(color: Hsla, paused: bool) -> Div {
    let base = div().size(px(8.)).flex_none().rounded(px(2.));
    if paused {
        base.border_1().border_color(color)
    } else {
        base.bg(color)
    }
}

fn outcome_badge(outcome: ScheduleOutcome, cx: &App) -> Div {
    let p = ProPalette::get(cx);
    let (bg, fg) = match outcome {
        ScheduleOutcome::Done => (p.chip, p.muted),
        ScheduleOutcome::NeedsYou => (hsla(37. / 360., 1.0, 0.50, 0.14), p.amber_ink),
        ScheduleOutcome::Failed => (p.red_soft, p.red_ink),
    };
    div()
        .h(px(18.))
        .px(px(6.))
        .flex_none()
        .rounded(px(4.))
        .bg(bg)
        .flex()
        .items_center()
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(fg)
        .whitespace_nowrap()
        .child(outcome.label(cx))
}

#[derive(IntoElement)]
struct ScheduleSwitchTrack {
    state_id: ElementId,
    checked: bool,
}

impl RenderOnce for ScheduleSwitchTrack {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let t = pro_spring_f32(
            self.state_id,
            if self.checked { 1.0 } else { 0.0 },
            0.24,
            w,
            cx,
        )
        .clamp(0., 1.);
        let off_bg: Hsla = if p.dark {
            rgb(0x3a3a3f).into()
        } else {
            rgb(0xd9d9dc).into()
        };
        let track_bg = pro_mix_color(off_bg, p.blue, t);
        div()
            .relative()
            .w(px(30.))
            .h(px(18.))
            .rounded_full()
            .bg(track_bg)
            .child(
                div()
                    .absolute()
                    .top(px(2.))
                    .left(px(2. + 12. * t))
                    .size(px(14.))
                    .rounded_full()
                    .bg(rgb(0xffffff)),
            )
    }
}

fn schedule_capsule_switch<V: 'static>(
    id: impl Into<ElementId>,
    aria: impl Into<SharedString>,
    checked: bool,
    cx: &Context<V>,
    on_toggle: impl Fn(&mut V, &mut Window, &mut Context<V>) + Clone + 'static,
) -> Stateful<Div> {
    let p = ProPalette::get(cx);
    let id = id.into();
    let state_id = pro_eid(&id, "sw-spring");
    let el = div()
        .id(id)
        .role(Role::Switch)
        .aria_label(aria.into())
        .tab_index(0)
        .w(px(30.))
        .h(px(18.))
        .flex_none()
        .rounded_full()
        .cursor_pointer()
        .focus_visible(move |s| s.border_1().border_color(p.focus))
        .child(ScheduleSwitchTrack { state_id, checked });
    activate(el, cx, on_toggle)
}

fn sample_smooth_curve(pts: &[(f32, f32)], subdiv: usize) -> Vec<(f32, f32)> {
    if pts.len() < 2 {
        return pts.to_vec();
    }
    let steps = subdiv.max(1);
    let mut out = Vec::with_capacity(pts.len() * steps);
    for (i, &p1) in pts[..pts.len() - 1].iter().enumerate() {
        let p0 = if i == 0 { pts[0] } else { pts[i - 1] };
        let p2 = pts[i + 1];
        let p3 = if i + 2 < pts.len() {
            pts[i + 2]
        } else {
            pts[pts.len() - 1]
        };
        for s in 0..steps {
            let t = s as f32 / steps as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let x = 0.5
                * ((2. * p1.0)
                    + (-p0.0 + p2.0) * t
                    + (2. * p0.0 - 5. * p1.0 + 4. * p2.0 - p3.0) * t2
                    + (-p0.0 + 3. * p1.0 - 3. * p2.0 + p3.0) * t3);
            let y = 0.5
                * ((2. * p1.1)
                    + (-p0.1 + p2.1) * t
                    + (2. * p0.1 - 5. * p1.1 + 4. * p2.1 - p3.1) * t2
                    + (-p0.1 + 3. * p1.1 - 3. * p2.1 + p3.1) * t3);
            out.push((x, y));
        }
    }
    if let Some(&last) = pts.last() {
        out.push(last);
    }
    out
}

fn paint_dashed_vertical(
    window: &mut Window,
    x: Pixels,
    y_start: Pixels,
    y_end: Pixels,
    color: Hsla,
) {
    let mut cur = f32::from(y_start);
    let end = f32::from(y_end);
    while cur < end {
        let seg_end = (cur + 2.2).min(end);
        let mut seg = PathBuilder::stroke(px(1.));
        seg.move_to(point(x, px(cur)));
        seg.line_to(point(x, px(seg_end)));
        if let Ok(path) = seg.build() {
            window.paint_path(path, color);
        }
        cur += 5.2;
    }
}

fn paint_disc(window: &mut Window, center: Point<Pixels>, radius: f32, color: Hsla) {
    let steps = 24usize;
    let mut pb = PathBuilder::fill();
    for i in 0..steps {
        let a = (i as f32 / steps as f32) * std::f32::consts::TAU;
        let pt = center + point(px(radius * a.cos()), px(radius * a.sin()));
        if i == 0 {
            pb.move_to(pt);
        } else {
            pb.line_to(pt);
        }
    }
    pb.close();
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

fn paint_circle_stroke(
    window: &mut Window,
    center: Point<Pixels>,
    radius: f32,
    stroke: f32,
    color: Hsla,
) {
    let steps = 24usize;
    let mut pb = PathBuilder::stroke(px(stroke));
    for i in 0..=steps {
        let a = (i as f32 / steps as f32) * std::f32::consts::TAU;
        let pt = center + point(px(radius * a.cos()), px(radius * a.sin()));
        if i == 0 {
            pb.move_to(pt);
        } else {
            pb.line_to(pt);
        }
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

fn paint_square(window: &mut Window, center: Point<Pixels>, half: f32, color: Hsla) {
    let mut pb = PathBuilder::fill();
    pb.move_to(center + point(px(-half), px(-half)));
    pb.line_to(center + point(px(half), px(-half)));
    pb.line_to(center + point(px(half), px(half)));
    pb.line_to(center + point(px(-half), px(half)));
    pb.close();
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentSchedulesScreen, ScheduleFilter, ScheduleOutcome, count_active_schedules,
        filter_schedule_indices, format_schedule_cost,
    };

    #[test]
    fn default_schedules_counts_and_filter_slices_match_reference() {
        let mut schedules = AgentSchedulesScreen::default_schedules();
        assert_eq!(schedules.len(), 7);
        assert_eq!(count_active_schedules(&schedules), 6);

        let active = filter_schedule_indices(&schedules, ScheduleFilter::Active);
        let paused = filter_schedule_indices(&schedules, ScheduleFilter::Paused);
        assert_eq!(active.len(), 6);
        assert_eq!(paused.len(), 1);
        assert_eq!(schedules[paused[0]].id.as_ref(), "evals");

        schedules[0].paused = true;
        assert_eq!(count_active_schedules(&schedules), 5);
        assert_eq!(schedules[4].last_outcome, ScheduleOutcome::NeedsYou);
    }

    #[test]
    fn cost_formatting_shortens_thousand_euro_runs() {
        assert_eq!(format_schedule_cost(3.10), "€3.10");
        assert_eq!(format_schedule_cost(0.18), "€0.18");
        assert_eq!(format_schedule_cost(1300.0), "€1.3K");
    }
}
