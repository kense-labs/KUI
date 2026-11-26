//! Client workspace block with live KPI numbers, switcher, and chronological context timeline.
//!
//! Combines a client switcher dropdown (with inline client creation), four rolling KPI
//! cards with sparklines, sectioned tabs (`Context`, `Reports`, `Financials`, `Settings`),
//! search and type filter pills, an `@`-mention note/file composer, and a shared timeline.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_card_shadow, pro_check_draw, pro_ease,
        pro_eid, pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_mix_color,
        pro_person_avatar, pro_pill_indicator, pro_popover_presence, pro_rolling_number,
        pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_swap_label, pro_switch,
        pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const CHEVRON_SWITCH_SVG: &str = r#"<path d="m8 9 4-4 4 4M8 15l4 4 4-4" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>"#;

/// Document type badge for attached files in [`ClientContextBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContextFileKind {
    /// Portable Document Format (`PDF`).
    #[default]
    Pdf,
    /// Spreadsheet workbook (`XLSX`).
    Xlsx,
    /// Comma-separated data export (`CSV`).
    Csv,
    /// Written brief or agreement (`DOC`).
    Doc,
}

impl ContextFileKind {
    fn badge_text(self) -> &'static str {
        match self {
            Self::Pdf => "PDF",
            Self::Xlsx => "XLSX",
            Self::Csv => "CSV",
            Self::Doc => "DOC",
        }
    }
}

/// Attached file metadata on a [`ContextEntry`] or staged in the composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextFile {
    /// File name (for example `"Northwind_Q3_Actuals.xlsx"`).
    pub name: SharedString,
    /// Document badge kind.
    pub kind: ContextFileKind,
}

impl ContextFile {
    /// Construct a [`ContextFile`] with `name` and `kind`.
    pub fn new(name: impl Into<SharedString>, kind: ContextFileKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }
}

/// Entry classification in the shared client timeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContextEntryKind {
    /// Written team note, decision, or `@Report` annotation.
    #[default]
    Note,
    /// Synced call recording or meeting recap.
    Meeting,
    /// Synced customer email thread.
    Email,
    /// Uploaded client spreadsheet, deck, or agreement.
    File,
    /// Generated financial or operational report snapshot.
    Report,
    /// Connected live data source event.
    Integration,
}

/// Filter pill scope for the `Context` timeline in [`ClientContextBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContextTypeFilter {
    /// Show every entry in chronological order.
    #[default]
    All,
    /// Show meeting recaps and call transcripts.
    Meetings,
    /// Show logged customer email threads.
    Emails,
    /// Show entries with uploaded documents or spreadsheets.
    Files,
    /// Show written notes and decisions.
    Notes,
}

impl ContextTypeFilter {
    const ALL: [Self; 5] = [
        Self::All,
        Self::Meetings,
        Self::Emails,
        Self::Files,
        Self::Notes,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Meetings => "meetings",
            Self::Emails => "emails",
            Self::Files => "files",
            Self::Notes => "notes",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All", "全部"),
            Self::Meetings => tr(cx, "Meetings", "会议"),
            Self::Emails => tr(cx, "Emails", "邮件"),
            Self::Files => tr(cx, "Files", "文件"),
            Self::Notes => tr(cx, "Notes", "笔记"),
        }
    }

    fn matches(self, entry: &ContextEntry) -> bool {
        match self {
            Self::All => true,
            Self::Meetings => entry.kind == ContextEntryKind::Meeting,
            Self::Emails => entry.kind == ContextEntryKind::Email,
            Self::Files => entry.kind == ContextEntryKind::File || !entry.files.is_empty(),
            Self::Notes => entry.kind == ContextEntryKind::Note,
        }
    }
}

/// Primary workspace section tab in [`ClientContextBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClientSectionTab {
    /// Chronological context composer and timeline.
    #[default]
    Context,
    /// Expandable client reports and executive packs.
    Reports,
    /// Live financial rows with period-over-period deltas.
    Financials,
    /// Per-client AI context and sync switches.
    Settings,
}

impl ClientSectionTab {
    const ALL: [Self; 4] = [
        Self::Context,
        Self::Reports,
        Self::Financials,
        Self::Settings,
    ];

    fn key(self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Reports => "reports",
            Self::Financials => "financials",
            Self::Settings => "settings",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Context => tr(cx, "Context", "上下文"),
            Self::Reports => tr(cx, "Reports", "报告"),
            Self::Financials => tr(cx, "Financials", "财务"),
            Self::Settings => tr(cx, "Settings", "设置"),
        }
    }
}

/// One live KPI card specification for a [`ContextClient`].
#[derive(Clone, Debug, PartialEq)]
pub struct ClientKpi {
    /// Metric identifier (for example `"mrr"`).
    pub id: SharedString,
    /// Metric label (for example `"MRR"` or `"Gross margin"`).
    pub label: SharedString,
    /// Numeric value animated via [`pro_rolling_number`].
    pub value: f64,
    /// Decimal places shown.
    pub decimals: usize,
    /// Leading currency or unit symbol (for example `"€"` or `"$"`).
    pub prefix: SharedString,
    /// Trailing unit suffix (for example `"k"` or `"%"`).
    pub suffix: SharedString,
    /// Period delta badge shown in the `Financials` tab (for example `"+8.4%"`).
    pub delta: SharedString,
    /// Whether the period trend is favorable.
    pub favorable: bool,
    /// Normalized sparkline points (`0.0..=1.0`).
    pub spark: Vec<f32>,
}

impl ClientKpi {
    /// Construct a [`ClientKpi`] card definition.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        value: f64,
        decimals: usize,
        prefix: impl Into<SharedString>,
        suffix: impl Into<SharedString>,
        delta: impl Into<SharedString>,
        favorable: bool,
        spark: Vec<f32>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            decimals,
            prefix: prefix.into(),
            suffix: suffix.into(),
            delta: delta.into(),
            favorable,
            spark,
        }
    }
}

/// Expandable report item listed under `Reports` and referenceable via `@` in notes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientReport {
    /// Stable report id.
    pub id: SharedString,
    /// Report title (for example `"Q3 Board Pack"`).
    pub title: SharedString,
    /// Period and update subtitle.
    pub updated: SharedString,
    /// Executive summary disclosed when expanded.
    pub summary: SharedString,
}

impl ClientReport {
    /// Construct a [`ClientReport`].
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        updated: impl Into<SharedString>,
        summary: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            updated: updated.into(),
            summary: summary.into(),
        }
    }
}

/// Single chronological timeline item inside a [`ContextClient`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextEntry {
    /// Stable entry id.
    pub id: SharedString,
    /// Entry category (`Note`, `Meeting`, `Email`, `File`, `Report`, `Integration`).
    pub kind: ContextEntryKind,
    /// Person or connector name attributed on the headline.
    pub author: SharedString,
    /// Action phrase shown after the author (for example `"added a note"`).
    pub action: SharedString,
    /// Relative timestamp (`"12m ago"`).
    pub when: SharedString,
    /// Optional note body or excerpt (`@Report` tokens render as mention pills).
    pub body: Option<SharedString>,
    /// Optional referenced `@Report` title highlighted inline.
    pub mention: Option<SharedString>,
    /// Attached files on this entry.
    pub files: Vec<ContextFile>,
    /// Optional connected app name rendered via [`pro_app_tile`].
    pub app: Option<SharedString>,
}

impl ContextEntry {
    /// Construct a [`ContextEntry`].
    pub fn new(
        id: impl Into<SharedString>,
        kind: ContextEntryKind,
        author: impl Into<SharedString>,
        action: impl Into<SharedString>,
        when: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            author: author.into(),
            action: action.into(),
            when: when.into(),
            body: None,
            mention: None,
            files: Vec::new(),
            app: None,
        }
    }

    /// Attach note or transcript body text.
    pub fn with_body(mut self, body: impl Into<SharedString>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Attach a highlighted `@Report` mention pill.
    pub fn with_mention(mut self, mention: impl Into<SharedString>) -> Self {
        self.mention = Some(mention.into());
        self
    }

    /// Attach document files to this timeline entry.
    pub fn with_files(mut self, files: Vec<ContextFile>) -> Self {
        self.files = files;
        self
    }

    /// Associate a third-party app mark (`"Stripe"`, `"Zoom"`, `"Gmail"`, etc.).
    pub fn with_app(mut self, app: impl Into<SharedString>) -> Self {
        self.app = Some(app.into());
        self
    }

    fn matches_query(&self, query: &str) -> bool {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return true;
        }
        self.author.to_ascii_lowercase().contains(&q)
            || self.action.to_ascii_lowercase().contains(&q)
            || self
                .body
                .as_ref()
                .is_some_and(|b| b.to_ascii_lowercase().contains(&q))
            || self
                .mention
                .as_ref()
                .is_some_and(|m| m.to_ascii_lowercase().contains(&q))
            || self
                .files
                .iter()
                .any(|f| f.name.to_ascii_lowercase().contains(&q))
            || self
                .app
                .as_ref()
                .is_some_and(|a| a.to_ascii_lowercase().contains(&q))
    }
}

/// Complete per-client workspace model in [`ClientContextBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct ContextClient {
    /// Stable client id (for example `"northwind"`).
    pub id: SharedString,
    /// Display name (for example `"Northwind Logistics"`).
    pub name: SharedString,
    /// Legal entity, currency, and fiscal metadata line.
    pub meta: SharedString,
    /// Brand squircle fill color.
    pub mark_color: Hsla,
    /// Relative last-sync label (`"Synced 3m ago"`).
    pub synced_label: SharedString,
    /// Four live KPI cards.
    pub kpis: Vec<ClientKpi>,
    /// Client reports available in `Reports` and `@` mentions.
    pub reports: Vec<ClientReport>,
    /// Chronological context timeline entries.
    pub context: Vec<ContextEntry>,
}

impl ContextClient {
    /// Construct a [`ContextClient`].
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        mark_color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            meta: meta.into(),
            mark_color,
            synced_label: "Synced 3m ago".into(),
            kpis: Vec::new(),
            reports: Vec::new(),
            context: Vec::new(),
        }
    }

    /// Set the four-card KPI summary for this client.
    pub fn with_kpis(mut self, kpis: Vec<ClientKpi>) -> Self {
        self.kpis = kpis;
        self
    }

    /// Set the expandable reports list for this client.
    pub fn with_reports(mut self, reports: Vec<ClientReport>) -> Self {
        self.reports = reports;
        self
    }

    /// Set the initial chronological timeline entries for this client.
    pub fn with_context(mut self, context: Vec<ContextEntry>) -> Self {
        self.context = context;
        self
    }
}

/// Typed events emitted by [`ClientContextBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientContextEvent {
    /// Active client workspace changed.
    ClientSwitched(SharedString),
    /// User created a new client from the switcher popover.
    ClientCreated {
        id: SharedString,
        name: SharedString,
    },
    /// User posted a new note and/or attached files to the active client's timeline.
    ContextAdded {
        client_id: SharedString,
        text: SharedString,
        files: Vec<ContextFile>,
    },
    /// User removed an entry from the active client's timeline.
    ContextRemoved {
        client_id: SharedString,
        entry_id: SharedString,
    },
    /// Timeline type filter pill changed.
    FilterChanged(ContextTypeFilter),
    /// Workspace section tab changed.
    TabChanged(ClientSectionTab),
    /// User triggered a live sync for the active client.
    Synced(SharedString),
}

fn default_clients() -> Vec<ContextClient> {
    vec![
        ContextClient::new(
            "northwind",
            "Northwind Logistics",
            "Northwind Logistics GmbH · EUR · Fiscal year ends Dec 31",
            rgb(0x0190fe).into(),
        )
        .with_kpis(vec![
            ClientKpi::new(
                "mrr",
                "MRR",
                48.2,
                1,
                "€",
                "k",
                "+8.4%",
                true,
                vec![0.14, 0.28, 0.22, 0.42, 0.48, 0.42, 0.62, 0.76, 0.82, 0.96],
            ),
            ClientKpi::new(
                "cash",
                "Cash",
                612.0,
                0,
                "€",
                "k",
                "-3.1%",
                true,
                vec![0.92, 0.84, 0.84, 0.68, 0.58, 0.68, 0.40, 0.30, 0.20, 0.14],
            ),
            ClientKpi::new(
                "margin",
                "Gross margin",
                64.1,
                1,
                "",
                "%",
                "+2.3pp",
                true,
                vec![0.14, 0.34, 0.14, 0.54, 0.34, 0.74, 0.54, 0.74, 0.94, 0.94],
            ),
            ClientKpi::new(
                "burn",
                "Net burn",
                36.0,
                1,
                "€",
                "k",
                "-11.2%",
                true,
                vec![0.86, 0.78, 0.94, 0.70, 0.62, 0.54, 0.62, 0.38, 0.28, 0.14],
            ),
        ])
        .with_reports(vec![
            ClientReport::new(
                "r-q3-board",
                "Q3 Board Pack",
                "Updated 2 days ago · 14 slides",
                "DACH freight volume grew 19% QoQ while warehouse automation cut unit handling cost by €0.42 per shipment.",
            ),
            ClientReport::new(
                "r-unit-econ",
                "Unit Economics Deep Dive",
                "Updated Oct 12 · Finance & Ops",
                "Contribution margin reached 64.1% across enterprise annual contracts with 14-month payback.",
            ),
            ClientReport::new(
                "r-cash-runway",
                "18-Month Cash Runway Model",
                "Updated Oct 4 · Scenario B",
                "At €36.0k net monthly burn, cash reserves of €612k fund 17.0 months before expansion credit lines.",
            ),
        ])
        .with_context(vec![
            ContextEntry::new("c1", ContextEntryKind::Note, "Lena Hart", "added a note", "12m ago")
                .with_mention("Q3 Board Pack")
                .with_body(
                    "Board approved expanding the DACH freight lane budget by €18k/mo starting November provided gross margin stays above 62%.",
                ),
            ContextEntry::new(
                "c2",
                ContextEntryKind::Meeting,
                "Zoom",
                "synced meeting recap · Quarterly CFO Review (42m)",
                "2h ago",
            )
            .with_app("Zoom")
            .with_body(
                "Confirmed net-30 enterprise billing terms and Q4 warehouse automation milestones with Claire Vogel.",
            ),
            ContextEntry::new(
                "c3",
                ContextEntryKind::File,
                "Theo Park",
                "uploaded 2 files",
                "Yesterday",
            )
            .with_files(vec![
                ContextFile::new("Northwind_Q3_Actuals.xlsx", ContextFileKind::Xlsx),
                ContextFile::new("Carrier_SLA_Addendum.pdf", ContextFileKind::Pdf),
            ]),
            ContextEntry::new(
                "c4",
                ContextEntryKind::Email,
                "Gmail",
                "logged thread with claire.vogel@northwind.de",
                "2d ago",
            )
            .with_app("Gmail")
            .with_body("Re: Updated VAT reverse-charge schedule & October PO #NW-884."),
            ContextEntry::new(
                "c5",
                ContextEntryKind::Integration,
                "Stripe",
                "connected live EUR subscription & invoice feed",
                "4d ago",
            )
            .with_app("Stripe"),
        ]),
        ContextClient::new(
            "lumen",
            "Lumen Studio",
            "Lumen Design Systems Ltd · USD · Fiscal year ends Mar 31",
            rgb(0x13b3a0).into(),
        )
        .with_kpis(vec![
            ClientKpi::new(
                "mrr",
                "MRR",
                82.5,
                1,
                "$",
                "k",
                "+14.2%",
                true,
                vec![0.18, 0.24, 0.32, 0.38, 0.48, 0.56, 0.66, 0.74, 0.86, 0.95],
            ),
            ClientKpi::new(
                "cash",
                "Cash",
                1240.0,
                0,
                "$",
                "k",
                "+6.8%",
                true,
                vec![0.42, 0.46, 0.50, 0.54, 0.60, 0.66, 0.72, 0.78, 0.84, 0.90],
            ),
            ClientKpi::new(
                "margin",
                "Gross margin",
                78.4,
                1,
                "",
                "%",
                "+1.6pp",
                true,
                vec![0.60, 0.64, 0.62, 0.68, 0.72, 0.76, 0.78, 0.82, 0.86, 0.92],
            ),
            ClientKpi::new(
                "burn",
                "Net burn",
                14.5,
                1,
                "$",
                "k",
                "-22.0%",
                true,
                vec![0.76, 0.70, 0.64, 0.58, 0.50, 0.44, 0.38, 0.32, 0.24, 0.18],
            ),
        ])
        .with_reports(vec![
            ClientReport::new(
                "r-lumen-arr",
                "Enterprise Seat Expansion Review",
                "Updated yesterday · 9 slides",
                "Net dollar retention climbed to 126% driven by multi-brand design system upgrades.",
            ),
            ClientReport::new(
                "r-lumen-tax",
                "UK R&D Tax Relief Filing",
                "Updated Sep 28 · Approved",
                "Qualified engineering payroll schedule finalized with external tax counsel.",
            ),
        ])
        .with_context(vec![
            ContextEntry::new("l1", ContextEntryKind::Note, "Lena Hart", "added a note", "35m ago")
                .with_mention("Enterprise Seat Expansion Review")
                .with_body("Annual multi-seat renewals now include dedicated SSO onboarding at $24k ARR minimum."),
            ContextEntry::new(
                "l2",
                ContextEntryKind::Meeting,
                "Google Calendar",
                "synced recap · Annual Renewal Prep (30m)",
                "5h ago",
            )
            .with_app("Google Calendar")
            .with_body("Reviewed Q1 roadmap commitments and enterprise SLA uptime guarantees."),
            ContextEntry::new(
                "l3",
                ContextEntryKind::File,
                "Maya Lin",
                "attached renewal schedule",
                "2d ago",
            )
            .with_files(vec![ContextFile::new(
                "Lumen_FY27_Cohort_Model.csv",
                ContextFileKind::Csv,
            )]),
        ]),
        ContextClient::new(
            "vesper",
            "Vesper Bio",
            "Vesper Therapeutics, Inc. · USD · Series B · Ends Dec 31",
            rgb(0xa651f0).into(),
        )
        .with_kpis(vec![
            ClientKpi::new(
                "mrr",
                "MRR",
                129.0,
                1,
                "$",
                "k",
                "+19.5%",
                true,
                vec![0.12, 0.22, 0.30, 0.40, 0.52, 0.60, 0.72, 0.80, 0.88, 0.98],
            ),
            ClientKpi::new(
                "cash",
                "Cash",
                4850.0,
                0,
                "$",
                "k",
                "-2.4%",
                true,
                vec![0.94, 0.90, 0.86, 0.82, 0.78, 0.74, 0.70, 0.68, 0.64, 0.60],
            ),
            ClientKpi::new(
                "margin",
                "Gross margin",
                71.8,
                1,
                "",
                "%",
                "+3.1pp",
                true,
                vec![0.40, 0.46, 0.50, 0.56, 0.62, 0.66, 0.72, 0.76, 0.82, 0.88],
            ),
            ClientKpi::new(
                "burn",
                "Net burn",
                94.0,
                1,
                "$",
                "k",
                "+4.2%",
                false,
                vec![0.36, 0.40, 0.44, 0.48, 0.52, 0.56, 0.60, 0.64, 0.68, 0.74],
            ),
        ])
        .with_reports(vec![ClientReport::new(
            "r-vesper-gpu",
            "Lab Compute & Cloud Spend Audit",
            "Updated 3 days ago · Ops",
            "Reserved GPU cluster commitments reduced spot training spend by 28% in Q3.",
        )])
        .with_context(vec![
            ContextEntry::new(
                "v1",
                ContextEntryKind::Email,
                "Gmail",
                "logged CFO approval from d.chen@vesper.bio",
                "1h ago",
            )
            .with_app("Gmail")
            .with_body("Approved multi-year wet-lab equipment lease cap at $420k."),
            ContextEntry::new(
                "v2",
                ContextEntryKind::File,
                "Samir Rao",
                "uploaded audit memo",
                "3d ago",
            )
            .with_files(vec![ContextFile::new(
                "SeriesB_Board_Covenants.doc",
                ContextFileKind::Doc,
            )]),
        ]),
    ]
}

fn sample_attachment_pool() -> [ContextFile; 4] {
    [
        ContextFile::new("Q4_Forecast_Model.xlsx", ContextFileKind::Xlsx),
        ContextFile::new("Signed_MSA_Addendum.pdf", ContextFileKind::Pdf),
        ContextFile::new("Cohort_Retention_Export.csv", ContextFileKind::Csv),
        ContextFile::new("Executive_Decision_Brief.doc", ContextFileKind::Doc),
    ]
}

#[derive(IntoElement)]
struct TimelineEntryEntrance {
    id: ElementId,
    animate_in: bool,
    parent_gap: f32,
    content: AnyElement,
}

impl RenderOnce for TimelineEntryEntrance {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.animate_in {
            return div()
                .id(self.id)
                .w_full()
                .child(self.content)
                .into_any_element();
        }
        let st = w.use_keyed_state(pro_eid(&self.id, "tl-enter"), cx, |_, inner_cx| {
            let mut slide = Slide::new(0.);
            slide.duration(1., 0.26, inner_cx);
            (slide, Rc::new(Cell::new(px(56.))))
        });
        let (amt, h_cell) = st.update(cx, |s, cx| (s.0.get(w, cx).clamp(0., 1.), s.1.clone()));
        let h = h_cell.get();
        div()
            .id(self.id)
            .mt(px(-self.parent_gap * (1. - amt)))
            .w_full()
            .flex_none()
            .when(amt < 0.998, |v| v.h(h * amt).overflow_hidden())
            .opacity(amt)
            .child(
                div()
                    .relative()
                    .top(px(7. * (1. - amt)))
                    .w_full()
                    .flex_none()
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first()
                            && b.size.height > px(0.)
                        {
                            h_cell.set(b.size.height);
                        }
                    })
                    .child(self.content),
            )
            .into_any_element()
    }
}

#[derive(IntoElement)]
struct SyncSpinIcon {
    angle_rad: f32,
    side: f32,
    color: Hsla,
}

impl RenderOnce for SyncSpinIcon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        if self.angle_rad.abs() <= 0.005 {
            return pro_icon(ProIcon::Refresh, self.side, self.color);
        }
        let ang = self.angle_rad;
        let color = self.color;
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let c = bounds.center();
                let r = (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) * 0.37)
                    .max(2.);
                for base_a in [0.25_f32, std::f32::consts::PI + 0.25] {
                    let a0 = base_a + ang;
                    let span = std::f32::consts::PI * 0.68;
                    let steps = 12usize;
                    let mut arc = PathBuilder::stroke(px(1.35));
                    for i in 0..=steps {
                        let a = a0 + span * (i as f32 / steps as f32);
                        let pt = c + point(px(r * a.cos()), px(r * a.sin()));
                        if i == 0 {
                            arc.move_to(pt);
                        } else {
                            arc.line_to(pt);
                        }
                    }
                    if let Ok(path) = arc.build() {
                        window.paint_path(path, color);
                    }
                }
            },
        )
        .size(px(self.side))
        .flex_none()
        .into_any_element()
    }
}

/// Client context workspace block with switcher, live rolling KPIs, search, type filter pills, and timeline composer.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::client_context::{ClientContextBlock, ClientContextEvent};
///
/// let workspace = cx.new(|cx| {
///     ClientContextBlock::new("client-context", cx)
///         .with_me("Lena Hart")
/// });
/// cx.subscribe(&workspace, |_, event: &ClientContextEvent, _| {
///     let _ = event;
/// }).detach();
/// # }
/// ```
pub struct ClientContextBlock {
    id: ElementId,
    focus: FocusHandle,
    clients: Vec<ContextClient>,
    active_client_id: SharedString,
    me_name: SharedString,
    section_tab: ClientSectionTab,
    type_filter: ContextTypeFilter,
    selected_kpi: Option<usize>,
    open_report_id: Option<SharedString>,
    switcher_open: bool,
    mention_menu_open: bool,
    staged_files: Vec<ContextFile>,
    next_sample_file_idx: usize,
    entry_seq: usize,
    added_entry_ids: Vec<SharedString>,
    sync_clock: PhaseClock,
    hovered_entry_id: Option<SharedString>,
    auto_sync_emails: bool,
    auto_sync_meetings: bool,
    ground_ai_answers: bool,
    search_editor: Option<Entity<Editor>>,
    composer_editor: Option<Entity<Editor>>,
    new_client_editor: Option<Entity<Editor>>,
    loading: bool,
    empty: bool,
    toast: Option<(SharedString, Instant)>,
    clock: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl EventEmitter<ClientContextEvent> for ClientContextBlock {}

impl ClientContextBlock {
    /// Construct a [`ClientContextBlock`] preloaded with 3 client workspaces (`Northwind Logistics` active).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let clients = default_clients();
        let active_client_id = clients
            .first()
            .map(|c| c.id.clone())
            .unwrap_or_else(|| "northwind".into());

        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            clients,
            active_client_id,
            me_name: "Lena Hart".into(),
            section_tab: ClientSectionTab::Context,
            type_filter: ContextTypeFilter::All,
            selected_kpi: None,
            open_report_id: Some("r-q3-board".into()),
            switcher_open: false,
            mention_menu_open: false,
            staged_files: Vec::new(),
            next_sample_file_idx: 0,
            entry_seq: 100,
            added_entry_ids: Vec::new(),
            sync_clock: PhaseClock::default(),
            hovered_entry_id: None,
            auto_sync_emails: true,
            auto_sync_meetings: true,
            ground_ai_answers: true,
            search_editor: None,
            composer_editor: None,
            new_client_editor: None,
            loading: false,
            empty: false,
            toast: None,
            clock: Instant::now(),
            measured_width: Rc::new(Cell::new(894.)),
        }
    }

    /// Override the list of client workspaces.
    pub fn with_clients(mut self, clients: Vec<ContextClient>) -> Self {
        if !clients.iter().any(|c| c.id == self.active_client_id) {
            self.active_client_id = clients
                .first()
                .map(|c| c.id.clone())
                .unwrap_or_else(|| "".into());
        }
        self.clients = clients;
        self
    }

    /// Select which client workspace starts active.
    pub fn with_active_client(mut self, client_id: impl Into<SharedString>) -> Self {
        self.active_client_id = client_id.into();
        self
    }

    /// Set the current author name used when adding timeline notes or files (`"Lena Hart"` by default).
    pub fn with_me(mut self, name: impl Into<SharedString>) -> Self {
        self.me_name = name.into();
        self
    }

    /// Set the initial timeline type filter pill (`All`, `Meetings`, `Emails`, `Files`, `Notes`).
    pub fn with_filter(mut self, filter: ContextTypeFilter) -> Self {
        self.type_filter = filter;
        self
    }

    /// Set the initial workspace section tab (`Context`, `Reports`, `Financials`, `Settings`).
    pub fn with_tab(mut self, tab: ClientSectionTab) -> Self {
        self.section_tab = tab;
        self
    }

    /// Configure skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently selected client identifier.
    pub fn active_client_id(&self) -> &SharedString {
        &self.active_client_id
    }

    /// Return the configured client workspaces.
    pub fn clients(&self) -> &[ContextClient] {
        &self.clients
    }

    /// Switch to another client workspace by `client_id`.
    pub fn switch_client(&mut self, client_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let cid = client_id.into();
        if self.clients.iter().any(|c| c.id == cid) && self.active_client_id != cid {
            self.active_client_id = cid.clone();
            self.selected_kpi = None;
            self.switcher_open = false;
            self.mention_menu_open = false;
            self.open_report_id = self
                .active_client()
                .and_then(|c| c.reports.first())
                .map(|r| r.id.clone());
            cx.emit(ClientContextEvent::ClientSwitched(cid));
            cx.notify();
        }
    }

    /// Programmatically append a new context entry to the active client.
    pub fn add_context_entry(
        &mut self,
        text: impl Into<SharedString>,
        files: Vec<ContextFile>,
        cx: &mut Context<Self>,
    ) -> bool {
        let text: SharedString = text.into();
        if text.trim().is_empty() && files.is_empty() {
            return false;
        }
        let cid = self.active_client_id.clone();
        let reports = self
            .active_client()
            .map(|c| c.reports.clone())
            .unwrap_or_default();
        let detected_mention = reports
            .iter()
            .find(|r| text.contains(&format!("@{}", r.title)))
            .map(|r| r.title.clone());

        self.entry_seq += 1;
        let entry_id: SharedString = format!("ctx-{}", self.entry_seq).into();
        let kind = if !files.is_empty() && text.trim().is_empty() {
            ContextEntryKind::File
        } else {
            ContextEntryKind::Note
        };
        let action: SharedString = if !files.is_empty() && text.trim().is_empty() {
            format!("uploaded {} file(s)", files.len()).into()
        } else {
            "added a note".into()
        };

        let mut entry = ContextEntry::new(
            entry_id.clone(),
            kind,
            self.me_name.clone(),
            action,
            "Just now",
        )
        .with_files(files.clone());
        if !text.trim().is_empty() {
            entry = entry.with_body(text.clone());
        }
        if let Some(m) = detected_mention {
            entry = entry.with_mention(m);
        }

        if let Some(client) = self.clients.iter_mut().find(|c| c.id == cid) {
            self.added_entry_ids.push(entry_id);
            client.context.insert(0, entry);
            cx.emit(ClientContextEvent::ContextAdded {
                client_id: cid,
                text,
                files,
            });
            cx.notify();
            true
        } else {
            false
        }
    }

    /// Toggle skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether loading skeletons are active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether empty state is forced on.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    fn active_client(&self) -> Option<&ContextClient> {
        self.clients.iter().find(|c| c.id == self.active_client_id)
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let search_ph: SharedString = tr(cx, "Search context...", "搜索上下文...").into();
        if let Some(ed) = &self.search_editor {
            ed.update(cx, |e, _| e.set_placeholder(search_ph));
        } else {
            let ed = cx.new(|cx| {
                let mut e = Editor::new(search_ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(search_ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed | EditEvent::Submitted => cx.notify(),
                EditEvent::Escape => {
                    if !ed_ent.read(cx).value().is_empty() {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    } else {
                        this.focus.focus(w, cx);
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }

        let comp_ph: SharedString = tr(
            cx,
            "Add a client note, decision, or type @ to mention a report...",
            "添加客户备注、关键决策，或输入 @ 引用财务报告...",
        )
        .into();
        if let Some(ed) = &self.composer_editor {
            ed.update(cx, |e, _| e.set_placeholder(comp_ph));
        } else {
            let ed = cx.new(|cx| {
                let mut e = Editor::new(comp_ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(comp_ph);
                e.set_command_submit(true);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    let val = ed_ent.read(cx).value();
                    this.mention_menu_open = val.ends_with('@');
                    cx.notify();
                }
                EditEvent::Escape => {
                    if this.mention_menu_open {
                        this.mention_menu_open = false;
                    } else {
                        this.focus.focus(w, cx);
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            cx.subscribe_in(
                &ed,
                w,
                |this, _, ev: &EditorShortcutEvent, w, cx| match ev {
                    EditorShortcutEvent::CommandSubmit => {
                        this.submit_composer(w, cx);
                    }
                },
            )
            .detach();
            self.composer_editor = Some(ed);
        }

        let create_ph: SharedString = tr(cx, "New client name...", "新客户名称...").into();
        if let Some(ed) = &self.new_client_editor {
            ed.update(cx, |e, _| e.set_placeholder(create_ph));
        } else {
            let ed = cx.new(|cx| {
                let mut e = Editor::new(create_ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(create_ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Submitted => {
                    this.create_client_from_editor(w, cx);
                }
                EditEvent::Escape => {
                    this.switcher_open = false;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            self.new_client_editor = Some(ed);
        }
    }

    fn submit_composer(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let text = self
            .composer_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        let files = std::mem::take(&mut self.staged_files);
        if text.is_empty() && files.is_empty() {
            return;
        }
        if let Some(ed) = &self.composer_editor {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        self.mention_menu_open = false;
        if self.add_context_entry(text, files, cx) {
            self.toast = Some((
                tr(cx, "Added to client timeline", "已添加到客户上下文时间线").into(),
                Instant::now(),
            ));
        }
    }

    fn insert_report_mention(
        &mut self,
        report_title: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ed) = self.composer_editor.clone() {
            let cur = ed.read(cx).value().to_owned();
            let base = cur.strip_suffix('@').unwrap_or(&cur);
            let sep = if base.is_empty() || base.ends_with(' ') {
                ""
            } else {
                " "
            };
            let next = format!("{base}{sep}@{report_title} ");
            ed.update(cx, |e, cx| e.set_value(next, w, cx));
            ed.read(cx).focus.clone().focus(w, cx);
        }
        self.mention_menu_open = false;
        cx.notify();
    }

    fn create_client_from_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.new_client_editor.clone() else {
            return;
        };
        let raw = ed.read(cx).value().trim().to_owned();
        if raw.is_empty() {
            return;
        }
        let slug: SharedString = format!("client-{}", self.clients.len() + 1).into();
        let name: SharedString = raw.into();
        let palette_colors = [
            rgb(0x0190fe),
            rgb(0x13b3a0),
            rgb(0xa651f0),
            rgb(0xf28a2e),
            rgb(0xee3e9e),
        ];
        let color: Hsla = palette_colors[self.clients.len() % palette_colors.len()].into();
        let new_client = ContextClient::new(
            slug.clone(),
            name.clone(),
            "USD · Fiscal year ends Dec 31",
            color,
        )
        .with_kpis(vec![
            ClientKpi::new(
                "mrr",
                "MRR",
                12.0,
                1,
                "$",
                "k",
                "New",
                true,
                vec![0.2, 0.3, 0.4, 0.5, 0.6, 0.7],
            ),
            ClientKpi::new(
                "cash",
                "Cash",
                250.0,
                0,
                "$",
                "k",
                "—",
                true,
                vec![0.5, 0.5, 0.52, 0.54, 0.55, 0.58],
            ),
            ClientKpi::new(
                "margin",
                "Gross margin",
                68.0,
                1,
                "",
                "%",
                "—",
                true,
                vec![0.6, 0.62, 0.64, 0.65, 0.67, 0.68],
            ),
            ClientKpi::new(
                "burn",
                "Net burn",
                8.5,
                1,
                "$",
                "k",
                "—",
                true,
                vec![0.4, 0.38, 0.36, 0.35, 0.34, 0.32],
            ),
        ])
        .with_context(vec![ContextEntry::new(
            format!("init-{slug}"),
            ContextEntryKind::Note,
            self.me_name.clone(),
            "created client workspace",
            "Just now",
        )]);

        ed.update(cx, |e, cx| e.set_value("", w, cx));
        self.clients.push(new_client);
        self.active_client_id = slug.clone();
        self.switcher_open = false;
        self.selected_kpi = None;
        self.toast = Some((
            if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("已创建客户 {name}").into()
            } else {
                format!("Created {name}").into()
            },
            Instant::now(),
        ));
        cx.emit(ClientContextEvent::ClientCreated { id: slug, name });
        cx.notify();
    }

    fn render_kpi_spark(points: &[f32], color: Hsla, card_bg: Hsla) -> impl IntoElement {
        let pts = points.to_vec();
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 4. || bh <= 4. || pts.len() < 2 {
                    return;
                }
                let pad = 3.;
                let map_pt = |idx: usize, val: f32| -> Point<Pixels> {
                    let tx = idx as f32 / (pts.len() - 1) as f32;
                    let ty = 1. - val.clamp(0., 1.);
                    point(
                        bounds.left() + px(pad + tx * (bw - 2. * pad)),
                        bounds.top() + px(pad + ty * (bh - 2. * pad)),
                    )
                };
                let mut stroke = PathBuilder::stroke(px(1.5));
                let mut end_pt = map_pt(0, pts[0]);
                for (i, &v) in pts.iter().enumerate() {
                    let pt = map_pt(i, v);
                    if i == 0 {
                        stroke.move_to(pt);
                    } else {
                        stroke.line_to(pt);
                    }
                    end_pt = pt;
                }
                if let Ok(path) = stroke.build() {
                    window.paint_path(path, color);
                }
                let ring_rect = Bounds::new(
                    point(end_pt.x - px(2.5), end_pt.y - px(2.5)),
                    size(px(5.), px(5.)),
                );
                window.paint_quad(quad(
                    ring_rect,
                    Corners::all(px(99.)),
                    card_bg,
                    Edges::all(px(1.5)),
                    color,
                    BorderStyle::Solid,
                ));
            },
        )
        .w(px(52.))
        .h(px(20.))
        .flex_none()
    }

    fn render_file_badge(kind: ContextFileKind, p: &ProPalette) -> Div {
        let (bg, fg) = match kind {
            ContextFileKind::Pdf => (p.red_soft, p.red_ink),
            ContextFileKind::Xlsx | ContextFileKind::Csv => (p.green_soft, p.green_ink),
            ContextFileKind::Doc => (p.accent_soft, p.accent_info),
        };
        div()
            .h(px(16.))
            .px(px(4.5))
            .rounded(px(4.))
            .bg(bg)
            .text_color(fg)
            .text_size(px(9.5))
            .font_weight(FontWeight::SEMIBOLD)
            .flex_none()
            .flex()
            .items_center()
            .child(kind.badge_text())
    }

    fn render_switcher_menu(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let ed_click = self.new_client_editor.clone();
        let mut menu = div()
            .id(pro_eid(&self.id, "client-switcher-menu"))
            .role(Role::ListBox)
            .w(px(268.))
            .p(px(5.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .flex()
            .flex_col()
            .gap(px(2.));

        for client in &self.clients {
            let active = client.id == self.active_client_id;
            let cid = client.id.clone();
            let count = client.context.len();
            let initial = client
                .name
                .chars()
                .next()
                .map(|c| c.to_uppercase().collect::<String>())
                .unwrap_or_else(|| "C".to_owned());

            menu = menu.child(
                pro_tactile(
                    pro_eid(&self.id, format!("sw-client-{cid}")),
                    client.name.clone(),
                    6.,
                    if active { p.hover } else { transparent_black() },
                    p.hover,
                    transparent_black(),
                    transparent_black(),
                    active,
                    div()
                        .h(px(32.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .size(px(18.))
                                .rounded(px(5.))
                                .bg(client.mark_color)
                                .text_color(rgb(0xffffff))
                                .text_size(px(10.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(initial),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(13.))
                                .font_weight(if active {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(p.ink)
                                .truncate()
                                .child(client.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.muted)
                                .child(format!("{count}")),
                        )
                        .child(pro_check_draw(
                            pro_eid(&self.id, format!("sw-client-chk-{cid}")),
                            active,
                            14.,
                            false,
                            p.accent_info,
                        )),
                    cx,
                    move |this, _, cx| {
                        this.switch_client(cid.clone(), cx);
                    },
                )
                .w_full(),
            );
        }

        menu.child(div().my(px(4.)).h(px(1.)).bg(p.line)).child(
            div()
                .px(px(3.))
                .py(px(2.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(28.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .bg(p.chip)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .cursor_text()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |_, _, w, cx| {
                                if let Some(ed) = &ed_click {
                                    ed.read(cx).focus.clone().focus(w, cx);
                                }
                            }),
                        )
                        .when_some(self.new_client_editor.clone(), |d, ed| {
                            d.child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(20.))
                                    .overflow_hidden()
                                    .child(ed),
                            )
                        }),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "create-client-btn"),
                    tr(cx, "Add", "新建"),
                    Some(ProIcon::Plus),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, w, cx| {
                        this.create_client_from_editor(w, cx);
                    },
                )),
        )
    }

    fn build_type_filter_pills(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let filter_sensor = pro_selection_sensor();
        let mut filter_pills = div()
            .id(pro_eid(&self.id, "ctx-filter-tabs"))
            .role(Role::TabList)
            .aria_label(tr(cx, "Filter context by type", "按类型筛选上下文"))
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .relative()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(1.))
            .self_start()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "ctx-type-pill"),
                self.type_filter.key(),
                filter_sensor.clone(),
                6.,
                p.card,
                Some(p.line),
                true,
            ));

        for tf in ContextTypeFilter::ALL {
            let active = self.type_filter == tf;
            let tf_lbl = tf.label(cx);
            filter_pills = filter_pills.child(
                pro_tactile(
                    pro_eid(&self.id, format!("ctx-filter-{}", tf.key())),
                    tf_lbl,
                    6.,
                    transparent_black(),
                    if active { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    active,
                    div()
                        .h(px(24.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if active { p.strong } else { p.text })
                        .child(tf_lbl),
                    cx,
                    move |this, _, cx| {
                        if this.type_filter != tf {
                            this.type_filter = tf;
                            cx.emit(ClientContextEvent::FilterChanged(tf));
                            cx.notify();
                        }
                    },
                )
                .relative()
                .flex_none()
                .when(active, |d| d.child(pro_mark_selected(&filter_sensor))),
            );
        }
        filter_pills
    }

    fn render_context_panel(
        &self,
        client: &ContextClient,
        compact: bool,
        show_inline_filters: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let query = self
            .search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_owned())
            .unwrap_or_default();

        let filter_pills_opt = if show_inline_filters {
            Some(self.build_type_filter_pills(cx))
        } else {
            None
        };

        // Composer card
        let comp_click = self.composer_editor.clone();
        let mut attached_strip = div().flex().flex_wrap().gap(px(6.));
        for (f_idx, f) in self.staged_files.iter().enumerate() {
            attached_strip = attached_strip.child(
                div()
                    .id(pro_eid(&self.id, format!("staged-file-{f_idx}")))
                    .h(px(24.))
                    .pl(px(7.))
                    .pr(px(3.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(Self::render_file_badge(f.kind, &p))
                    .child(
                        div()
                            .max_w(px(180.))
                            .text_size(px(12.))
                            .text_color(p.ink)
                            .truncate()
                            .child(f.name.clone()),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, format!("rm-staged-{f_idx}")),
                        tr(cx, "Remove attachment", "移除附件"),
                        ProIcon::Close,
                        18.,
                        false,
                        cx,
                        move |this, _, cx| {
                            if f_idx < this.staged_files.len() {
                                this.staged_files.remove(f_idx);
                                cx.notify();
                            }
                        },
                    )),
            );
        }

        let mut mention_box = div()
            .p(px(4.))
            .rounded(px(8.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .px(px(4.))
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .child(tr(cx, "Mention report:", "引用报告：")),
            );
        for rep in &client.reports {
            let r_title = rep.title.clone();
            mention_box = mention_box.child(
                pro_tactile(
                    pro_eid(&self.id, format!("mention-opt-{}", rep.id)),
                    format!("Insert @{}", rep.title),
                    5.,
                    p.accent_soft,
                    p.accent_info.opacity(0.22),
                    transparent_black(),
                    p.accent_info.opacity(0.32),
                    false,
                    div()
                        .h(px(22.))
                        .px(px(7.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.accent_info)
                        .child(pro_icon(ProIcon::UsageChart, 11., p.accent_info))
                        .child(format!("@{}", rep.title)),
                    cx,
                    move |this, w, cx| {
                        this.insert_report_mention(&r_title, w, cx);
                    },
                )
                .flex_none(),
            );
        }

        let mention_open = self.mention_menu_open && !client.reports.is_empty();
        let composer_card = div()
            .flex_1()
            .min_w_0()
            .rounded(px(10.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .min_h(px(34.))
                    .px(px(12.))
                    .pt(px(8.))
                    .pb(px(2.))
                    .cursor_text()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_, _, w, cx| {
                            if let Some(ed) = &comp_click {
                                ed.read(cx).focus.clone().focus(w, cx);
                            }
                        }),
                    )
                    .when_some(self.composer_editor.clone(), |d, ed| {
                        d.child(
                            div()
                                .w_full()
                                .min_w_0()
                                .h(px(26.))
                                .overflow_hidden()
                                .child(ed),
                        )
                    }),
            )
            .when(!self.staged_files.is_empty(), |d| {
                d.child(div().px(px(12.)).pt(px(4.)).child(attached_strip))
            })
            .child(pro_fold(
                pro_eid(&self.id, "mention-menu-fold"),
                mention_open,
                40.,
                0.,
                move || {
                    div()
                        .px(px(10.))
                        .pt(px(6.))
                        .child(mention_box)
                        .into_any_element()
                },
            ))
            .child(
                div()
                    .px(px(8.))
                    .py(px(5.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "attach-file-btn"),
                        tr(cx, "Attach file", "添加附件文件"),
                        ProIcon::Paperclip,
                        26.,
                        false,
                        cx,
                        |this, _, cx| {
                            let pool = sample_attachment_pool();
                            let file = pool[this.next_sample_file_idx % pool.len()].clone();
                            this.next_sample_file_idx += 1;
                            if !this.staged_files.iter().any(|f| f.name == file.name) {
                                this.staged_files.push(file);
                            }
                            cx.notify();
                        },
                    ))
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, "mention-report-btn"),
                            tr(cx, "Mention client report", "引用客户报告"),
                            6.,
                            if self.mention_menu_open {
                                p.accent_soft
                            } else {
                                transparent_black()
                            },
                            p.hover,
                            transparent_black(),
                            transparent_black(),
                            self.mention_menu_open,
                            div()
                                .h(px(26.))
                                .px(px(7.))
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .whitespace_nowrap()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if self.mention_menu_open {
                                    p.accent_info
                                } else {
                                    p.muted
                                })
                                .child("@")
                                .child(tr(cx, "Report", "报告")),
                            cx,
                            |this, _, cx| {
                                this.mention_menu_open = !this.mention_menu_open;
                                cx.notify();
                            },
                        )
                        .flex_none(),
                    )
                    .child(div().flex_1())
                    .when(!compact, |d| {
                        d.child(
                            div()
                                .mr(px(4.))
                                .flex()
                                .items_center()
                                .gap(px(3.))
                                .child(
                                    div()
                                        .h(px(20.))
                                        .px(px(5.))
                                        .rounded(px(4.))
                                        .bg(p.chip)
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .flex()
                                        .items_center()
                                        .child("⌘"),
                                )
                                .child(
                                    div()
                                        .h(px(20.))
                                        .px(px(5.))
                                        .rounded(px(4.))
                                        .bg(p.chip)
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .flex()
                                        .items_center()
                                        .child(pro_svg(
                                            "0 0 16 16",
                                            r#"<path d="M12.5 3.5v4a2 2 0 0 1-2 2H4m0 0 2.8-2.8M4 9.5l2.8 2.8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#,
                                            11.,
                                            p.muted,
                                        )),
                                ),
                        )
                    })
                    .child(pro_btn(
                        pro_eid(&self.id, "submit-ctx-btn"),
                        tr(cx, "Add context", "添加上下文"),
                        Some(ProIcon::Plus),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, w, cx| {
                            this.submit_composer(w, cx);
                        },
                    )),
            );

        let composer_row = div()
            .mt(px(if filter_pills_opt.is_some() { 10. } else { 2. }))
            .flex()
            .items_start()
            .gap(px(10.))
            .child(
                div()
                    .mt(px(4.))
                    .child(pro_person_avatar(&self.me_name, 24., false, None, cx)),
            )
            .child(composer_card);

        // Filtered chronological timeline
        let visible_entries: Vec<&ContextEntry> = client
            .context
            .iter()
            .filter(|e| self.type_filter.matches(e) && e.matches_query(&query))
            .collect();

        let timeline_el = if visible_entries.is_empty() {
            div()
                .mt(px(16.))
                .rounded(px(10.))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line)
                .py(px(22.))
                .px(px(16.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .text_size(px(13.))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "No context entries match the current filter.",
                    "当前筛选条件下暂无匹配的上下文记录。",
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "clear-ctx-filter"),
                    tr(cx, "Show all entries", "查看全部记录"),
                    None,
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, w, cx| {
                        this.type_filter = ContextTypeFilter::All;
                        if let Some(ed) = &this.search_editor {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                        cx.notify();
                    },
                ))
                .into_any_element()
        } else {
            let mut list = div().mt(px(14.)).flex().flex_col().gap(px(12.));
            for entry in visible_entries {
                let eid = entry.id.clone();
                let hover_id = entry.id.clone();
                let rm_id = entry.id.clone();
                let client_id = client.id.clone();
                let is_hovered = self
                    .hovered_entry_id
                    .as_ref()
                    .is_some_and(|h| h == &entry.id);
                let is_new_entry = self.added_entry_ids.contains(&entry.id);

                let node = if let Some(app_name) = &entry.app {
                    pro_app_tile(app_name, 24., cx)
                } else if entry.kind == ContextEntryKind::Note {
                    pro_person_avatar(&entry.author, 24., false, None, cx)
                } else {
                    let ic = match entry.kind {
                        ContextEntryKind::Meeting => ProIcon::Calendar,
                        ContextEntryKind::Email => ProIcon::Mail,
                        ContextEntryKind::File => ProIcon::Folder,
                        ContextEntryKind::Report => ProIcon::UsageChart,
                        ContextEntryKind::Integration => ProIcon::Link,
                        ContextEntryKind::Note => ProIcon::Note,
                    };
                    div()
                        .size(px(24.))
                        .rounded(px(6.5))
                        .bg(if p.dark {
                            rgb(0x27272b).into()
                        } else {
                            p.subtle
                        })
                        .border_1()
                        .border_color(p.line)
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ic, 13., p.muted))
                        .into_any_element()
                };

                let mut files_row = div().mt(px(5.)).flex().flex_wrap().gap(px(6.));
                for f in &entry.files {
                    files_row = files_row.child(
                        div()
                            .h(px(23.))
                            .px(px(7.))
                            .rounded(px(5.5))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .child(Self::render_file_badge(f.kind, &p))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(f.name.clone()),
                            ),
                    );
                }

                let row_inner = div()
                    .id(pro_eid(&self.id, format!("tline-hit-{eid}")))
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.hovered_entry_id = Some(hover_id.clone());
                        } else if this
                            .hovered_entry_id
                            .as_ref()
                            .is_some_and(|h| h == &hover_id)
                        {
                            this.hovered_entry_id = None;
                        }
                        cx.notify();
                    }))
                    .child(node)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .pt(px(1.))
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
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_wrap()
                                            .items_center()
                                            .gap_x(px(5.))
                                            .text_size(px(12.5))
                                            .line_height(px(19.))
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(p.strong)
                                                    .child(entry.author.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_color(p.text)
                                                    .child(entry.action.clone()),
                                            )
                                            .when_some(entry.mention.clone(), |d, m_title| {
                                                d.child(activate(
                                                    div()
                                                        .id(pro_eid(
                                                            &self.id,
                                                            format!("Jump-{eid}"),
                                                        ))
                                                        .role(Role::Button)
                                                        .tab_index(0)
                                                        .h(px(19.))
                                                        .px(px(6.))
                                                        .rounded(px(5.))
                                                        .bg(p.accent_soft)
                                                        .text_color(p.accent_info)
                                                        .text_size(px(11.5))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(3.))
                                                        .cursor_pointer()
                                                        .child(pro_icon(
                                                            ProIcon::UsageChart,
                                                            11.,
                                                            p.accent_info,
                                                        ))
                                                        .child(format!("@{m_title}")),
                                                    cx,
                                                    |this, _, cx| {
                                                        this.section_tab =
                                                            ClientSectionTab::Reports;
                                                        cx.emit(ClientContextEvent::TabChanged(
                                                            ClientSectionTab::Reports,
                                                        ));
                                                        cx.notify();
                                                    },
                                                ))
                                            })
                                            .child(
                                                div()
                                                    .text_size(px(11.5))
                                                    .text_color(p.muted)
                                                    .child(format!("· {}", entry.when)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .opacity(if is_hovered || compact { 1. } else { 0. })
                                            .child(pro_icon_btn(
                                                pro_eid(&self.id, format!("rm-entry-{eid}")),
                                                tr(cx, "Remove context entry", "移除此条记录"),
                                                ProIcon::Close,
                                                20.,
                                                false,
                                                cx,
                                                move |this, _, cx| {
                                                    if let Some(cl) = this
                                                        .clients
                                                        .iter_mut()
                                                        .find(|c| c.id == client_id)
                                                    {
                                                        cl.context.retain(|e| e.id != rm_id);
                                                        cx.emit(
                                                            ClientContextEvent::ContextRemoved {
                                                                client_id: client_id.clone(),
                                                                entry_id: rm_id.clone(),
                                                            },
                                                        );
                                                        cx.notify();
                                                    }
                                                },
                                            )),
                                    ),
                            )
                            .when_some(entry.body.clone(), |d, body| {
                                d.child(
                                    div()
                                        .mt(px(2.))
                                        .text_size(px(12.5))
                                        .line_height(px(19.))
                                        .text_color(p.ink)
                                        .child(body),
                                )
                            })
                            .when(!entry.files.is_empty(), |d| d.child(files_row)),
                    );

                list = list.child(TimelineEntryEntrance {
                    id: pro_eid(&self.id, format!("tline-{eid}")),
                    animate_in: is_new_entry,
                    parent_gap: 12.,
                    content: row_inner.into_any_element(),
                });
            }
            list.into_any_element()
        };

        div()
            .pt(px(10.))
            .flex()
            .flex_col()
            .when_some(filter_pills_opt, |d, fp| d.child(fp))
            .child(composer_row)
            .child(pro_view_transition(
                pro_eid(&self.id, "tline-vtx"),
                format!("{}-{}", client.id, self.type_filter.key()),
                timeline_el,
            ))
            .into_any_element()
    }

    fn render_reports_panel(&self, client: &ContextClient, cx: &mut Context<Self>) -> AnyElement {
        let p = ProPalette::get(cx);
        if client.reports.is_empty() {
            return div()
                .pt(px(24.))
                .text_size(px(13.))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "No reports published yet.",
                    "暂无已发布的客户报告。",
                ))
                .into_any_element();
        }
        let mut list = div().pt(px(16.)).flex().flex_col().gap(px(6.));
        for rep in &client.reports {
            let is_open = self.open_report_id.as_ref() == Some(&rep.id);
            let rid = rep.id.clone();
            let r_title_mention = rep.title.clone();
            let summary = rep.summary.clone();

            let body_card = div()
                .pl(px(44.))
                .pr(px(12.))
                .pb(px(12.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child(summary),
                )
                .child(div().self_start().child(pro_btn(
                    pro_eid(&self.id, format!("rep-cite-{rid}")),
                    tr(cx, "Cite in note", "在笔记中引用"),
                    None,
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    move |this, w, cx| {
                        this.section_tab = ClientSectionTab::Context;
                        this.insert_report_mention(&r_title_mention, w, cx);
                    },
                )))
                .into_any_element();

            list = list.child(
                div()
                    .rounded(px(9.))
                    .bg(if is_open {
                        p.hover
                    } else {
                        transparent_black()
                    })
                    .border_1()
                    .border_color(if is_open { p.line } else { transparent_black() })
                    .flex()
                    .flex_col()
                    .child(
                        pro_tactile(
                            pro_eid(&self.id, format!("rep-head-{}", rep.id)),
                            rep.title.clone(),
                            9.,
                            transparent_black(),
                            p.hover,
                            transparent_black(),
                            transparent_black(),
                            is_open,
                            div()
                                .min_h(px(52.))
                                .px(px(10.))
                                .py(px(8.))
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .child(
                                    div()
                                        .size(px(32.))
                                        .rounded(px(8.))
                                        .bg(p.card)
                                        .border_1()
                                        .border_color(p.line)
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_icon(ProIcon::UsageChart, 15., p.accent_info)),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .truncate()
                                                .child(rep.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.muted)
                                                .truncate()
                                                .child(rep.updated.clone()),
                                        ),
                                )
                                .child(pro_icon(
                                    if is_open {
                                        ProIcon::ChevronUp
                                    } else {
                                        ProIcon::ChevronDown
                                    },
                                    14.,
                                    p.muted,
                                )),
                            cx,
                            move |this, _, cx| {
                                if this.open_report_id.as_ref() == Some(&rid) {
                                    this.open_report_id = None;
                                } else {
                                    this.open_report_id = Some(rid.clone());
                                }
                                cx.notify();
                            },
                        )
                        .w_full(),
                    )
                    .child(pro_fold(
                        pro_eid(&self.id, format!("rep-fold-{}", rep.id)),
                        is_open,
                        84.,
                        0.,
                        move || body_card,
                    )),
            );
        }
        list.into_any_element()
    }

    fn render_financials_panel(
        &self,
        client: &ContextClient,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let mut rows = div().pt(px(16.)).flex().flex_col().gap(px(6.));
        for (idx, kpi) in client.kpis.iter().enumerate() {
            let active = self.selected_kpi == Some(idx);
            let (badge_bg, badge_fg) = if kpi.favorable {
                (p.green_soft, p.green_ink)
            } else {
                (p.red_soft, p.red_ink)
            };
            rows = rows.child(
                pro_tactile(
                    pro_eid(&self.id, format!("fin-row-{}", kpi.id)),
                    kpi.label.clone(),
                    9.,
                    if active {
                        p.select
                    } else {
                        transparent_black()
                    },
                    p.hover,
                    if active { p.line } else { transparent_black() },
                    p.line,
                    active,
                    div()
                        .min_h(px(54.))
                        .px(px(10.))
                        .py(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(kpi.label.clone()),
                                )
                                .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                                    cx,
                                    "Trailing 10-period ledger",
                                    "近 10 期账本序列",
                                ))),
                        )
                        .when(!compact, |d| {
                            d.child(Self::render_kpi_spark(&kpi.spark, p.accent_info, p.card))
                        })
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .flex_col()
                                .items_end()
                                .gap(px(2.))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, format!("fin-num-{}", kpi.id)),
                                    kpi.value,
                                    kpi.decimals,
                                    kpi.prefix.clone(),
                                    kpi.suffix.clone(),
                                    13.5,
                                    FontWeight::MEDIUM,
                                    p.strong,
                                ))
                                .child(
                                    div()
                                        .h(px(19.))
                                        .px(px(6.))
                                        .rounded(px(5.))
                                        .bg(badge_bg)
                                        .text_color(badge_fg)
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .flex()
                                        .items_center()
                                        .child(kpi.delta.clone()),
                                ),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.selected_kpi = if this.selected_kpi == Some(idx) {
                            None
                        } else {
                            Some(idx)
                        };
                        cx.notify();
                    },
                )
                .w_full(),
            );
        }
        rows.into_any_element()
    }

    fn render_settings_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = ProPalette::get(cx);
        let row = |id_sfx: &'static str,
                   title: &'static str,
                   desc: &'static str,
                   checked: bool,
                   on_toggle: fn(&mut Self, &mut Window, &mut Context<Self>)| {
            div()
                .min_h(px(56.))
                .px(px(8.))
                .py(px(8.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(16.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(title),
                        )
                        .child(div().text_size(px(12.)).text_color(p.muted).child(desc)),
                )
                .child(pro_switch(
                    pro_eid(&self.id, id_sfx),
                    title,
                    checked,
                    cx,
                    on_toggle,
                ))
        };

        div()
            .pt(px(14.))
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(row(
                "sw-ground-ai",
                tr(cx, "Ground AI answers in client context", "使用客户上下文增强 AI 回答"),
                tr(
                    cx,
                    "Include notes, @Reports, and uploaded files automatically in workspace threads.",
                    "在工作区对话中自动注入此客户的笔记、@报告与已上传文件。",
                ),
                self.ground_ai_answers,
                |this, _, cx| {
                    this.ground_ai_answers = !this.ground_ai_answers;
                    cx.notify();
                },
            ))
            .child(row(
                "sw-sync-meetings",
                tr(cx, "Auto-import meeting recaps", "自动导入会议纪要"),
                tr(
                    cx,
                    "Append Zoom and Google Calendar call summaries matching this client's domain.",
                    "自动将匹配此客户域名的 Zoom 与日历通话纪要同步至时间线。",
                ),
                self.auto_sync_meetings,
                |this, _, cx| {
                    this.auto_sync_meetings = !this.auto_sync_meetings;
                    cx.notify();
                },
            ))
            .child(row(
                "sw-sync-emails",
                tr(cx, "Log stakeholder email threads", "归档关键联系人邮件"),
                tr(
                    cx,
                    "Capture billing and executive threads from connected Gmail inboxes.",
                    "从已连接的 Gmail 收件箱归档账单与高管往来会话。",
                ),
                self.auto_sync_emails,
                |this, _, cx| {
                    this.auto_sync_emails = !this.auto_sync_emails;
                    cx.notify();
                },
            ))
            .into_any_element()
    }

    fn render_loading(&self, compact: bool, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        if !disabled_motion(cx) {
            w.request_animation_frame();
        }
        let phase = (self.clock.elapsed().as_secs_f32() * 0.75).fract();
        let kpi_n = if compact { 2usize } else { 4usize };
        let mut kpis = div().flex().gap(px(12.));
        for i in 0..kpi_n {
            kpis = kpis.child(
                div()
                    .flex_1()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .p(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(pro_skeleton_bar(68., 12., 4., phase + i as f32 * 0.08, cx))
                    .child(pro_skeleton_bar(92., 20., 5., phase + i as f32 * 0.1, cx)),
            );
        }

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(pro_skeleton_bar(210., 26., 6., phase, cx))
                    .child(pro_skeleton_bar(140., 26., 6., phase + 0.1, cx)),
            )
            .child(kpis)
            .child(
                div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .p(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(pro_skeleton_bar(260., 26., 6., phase + 0.2, cx))
                    .child(pro_skeleton_bar(420., 68., 8., phase + 0.3, cx))
                    .child(pro_skeleton_bar(340., 36., 6., phase + 0.4, cx)),
            )
    }
}

impl Render for ClientContextBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let measure_cell = self.measured_width.clone();
        let container_w = measure_cell.get().max(320.);
        let compact = container_w <= 680.;

        let sensor = canvas(
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
        .left_0()
        .right_0()
        .top_0()
        .h(px(0.));

        if self.loading {
            return div()
                .id(self.id.clone())
                .w_full()
                .py(px(8.))
                .relative()
                .child(sensor)
                .child(self.render_loading(compact, w, cx))
                .into_any_element();
        }

        let Some(client) = self.active_client().cloned() else {
            return div()
                .id(self.id.clone())
                .w_full()
                .relative()
                .rounded(px(12.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(sensor)
                .child(pro_empty_state(
                    ProIcon::Building,
                    tr(cx, "No client workspace selected", "尚未选择客户工作区"),
                    tr(
                        cx,
                        "Create or load a client to track live KPIs, notes, and shared files.",
                        "创建或载入客户档案即可在此追踪实时指标、笔记与共享文件。",
                    ),
                    Some((
                        pro_eid(&self.id, "restore-clients"),
                        tr(cx, "Load sample clients", "载入示例客户").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.clients = default_clients();
                            this.active_client_id = "northwind".into();
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        };

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .relative()
                .rounded(px(12.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(sensor)
                .child(pro_empty_state(
                    ProIcon::Building,
                    tr(cx, "Client workspace is empty", "客户上下文为空"),
                    tr(
                        cx,
                        "Restore default client records to preview live KPIs and timeline history.",
                        "恢复默认客户档案即可预览实时财务指标与上下文时间线。",
                    ),
                    Some((
                        pro_eid(&self.id, "unempty-clients"),
                        tr(cx, "Show client workspace", "显示客户工作区").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let mark_initial = client
            .name
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect::<String>())
            .unwrap_or_else(|| "C".to_owned());

        let who_el = pro_view_transition(
            pro_eid(&self.id, "client-who-vtx"),
            client.id.clone(),
            div()
                .flex()
                .items_center()
                .gap(px(11.))
                .min_w_0()
                .child(
                    div()
                        .size(px(32.))
                        .rounded(px(8.))
                        .bg(client.mark_color)
                        .border_1()
                        .border_color(hsla(0., 0., 0., 0.10))
                        .text_color(rgb(0xffffff))
                        .text_size(px(15.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(mark_initial),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(if compact { 18. } else { 20. }))
                                .line_height(px(if compact { 24. } else { 26. }))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.strong)
                                .truncate()
                                .child(client.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .truncate()
                                .child(client.meta.clone()),
                        ),
                ),
        );

        let sync_cid = client.id.clone();
        let zh_locale = UiLanguage::current(cx) == UiLanguage::Chinese;
        let synced_text: SharedString =
            if zh_locale && client.synced_label.as_ref() == "Synced 3m ago" {
                "3 分钟前已同步".into()
            } else {
                client.synced_label.clone()
            };
        let sync_age = self.sync_clock.time(0.48, w, cx);
        let spin_angle = if sync_age < 0.48 && !disabled_motion(cx) {
            pro_ease((sync_age / 0.48).clamp(0., 1.)) * std::f32::consts::TAU
        } else {
            0.
        };

        let sync_btn = pro_tactile(
            pro_eid(&self.id, "sync-now-btn"),
            tr(cx, "Sync now", "立即同步"),
            6.,
            p.card,
            p.chip_hover,
            p.line_strong,
            p.line_strong,
            false,
            div()
                .h(px(28.))
                .px(px(10.))
                .flex()
                .items_center()
                .gap(px(7.))
                .whitespace_nowrap()
                .child(div().size(px(6.)).rounded_full().bg(p.green))
                .child(pro_swap_label(
                    pro_eid(&self.id, "sync-now-lbl"),
                    synced_text,
                    12.5,
                    FontWeight::MEDIUM,
                    p.text,
                ))
                .child(SyncSpinIcon {
                    angle_rad: spin_angle,
                    side: 13.,
                    color: p.muted,
                }),
            cx,
            move |this, _, cx| {
                if let Some(cl) = this.clients.iter_mut().find(|c| c.id == sync_cid) {
                    cl.synced_label = tr(cx, "Synced just now", "刚刚同步").into();
                }
                this.sync_clock.restart(cx);
                this.toast = Some((
                    tr(cx, "Refreshed live client metrics", "已刷新客户实时指标").into(),
                    Instant::now(),
                ));
                cx.emit(ClientContextEvent::Synced(sync_cid.clone()));
                cx.notify();
            },
        )
        .flex_none();

        let sw_open = self.switcher_open;
        let switcher_btn = pro_tactile(
            pro_eid(&self.id, "switch-client-btn"),
            tr(cx, "Switch client", "切换客户"),
            6.,
            if sw_open { p.accent_soft } else { p.card },
            if sw_open {
                p.accent_info.opacity(0.20)
            } else {
                p.chip_hover
            },
            if sw_open {
                p.accent_info.opacity(0.30)
            } else {
                p.line_strong
            },
            if sw_open {
                p.accent_info.opacity(0.38)
            } else {
                p.line_strong
            },
            sw_open,
            div()
                .h(px(28.))
                .pl(px(10.))
                .pr(px(8.))
                .flex()
                .items_center()
                .gap(px(6.))
                .whitespace_nowrap()
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if sw_open { p.accent_info } else { p.strong })
                .child(tr(cx, "Switch client", "切换客户"))
                .child(pro_svg(
                    "0 0 24 24",
                    CHEVRON_SWITCH_SVG,
                    12.5,
                    if sw_open { p.accent_info } else { p.muted },
                )),
            cx,
            |this, _, cx| {
                this.switcher_open = !this.switcher_open;
                cx.notify();
            },
        )
        .flex_none();

        let switcher_menu = self.render_switcher_menu(cx);
        let switcher_pop = div()
            .absolute()
            .top(px(34.))
            .right_0()
            .child(pro_popover_presence(
                pro_eid(&self.id, "switcher-pres"),
                self.switcher_open,
                move || switcher_menu.into_any_element(),
            ));

        let header = div()
            .pb(px(14.))
            .flex()
            .when(compact, |d| d.flex_col().gap(px(10.)))
            .when(!compact, |d| {
                d.items_center().justify_between().gap(px(16.))
            })
            .child(div().flex_1().min_w_0().child(who_el))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(sync_btn)
                    .child(div().relative().child(switcher_btn).child(switcher_pop)),
            );

        // 4 KPI cards (1x4 on desktop, 2x2 below 680px)
        let has_picked_kpi = self.selected_kpi.is_some();
        let mut kpi_cards: Vec<AnyElement> = Vec::with_capacity(client.kpis.len());
        for (idx, kpi) in client.kpis.iter().enumerate() {
            let picked = self.selected_kpi == Some(idx);
            let dim = has_picked_kpi && !picked;
            let card_alpha = pro_spring_f32(
                pro_eid(&self.id, format!("kpi-dim-{}", kpi.id)),
                if dim { 0.56 } else { 1.0 },
                0.20,
                w,
                cx,
            )
            .clamp(0.3, 1.0);
            let kpi_lbl = if zh_locale {
                match kpi.label.as_ref() {
                    "MRR" => SharedString::from("月经常性收入 MRR"),
                    "Cash" => SharedString::from("现金储备"),
                    "Gross margin" => SharedString::from("毛利率"),
                    "Net burn" => SharedString::from("月净消耗"),
                    _ => kpi.label.clone(),
                }
            } else {
                kpi.label.clone()
            };

            kpi_cards.push(
                pro_tactile(
                    pro_eid(&self.id, format!("kpi-card-{}", kpi.id)),
                    kpi_lbl.clone(),
                    10.,
                    p.card,
                    pro_mix_color(p.card, p.ink, 0.022),
                    if picked {
                        p.accent_info.opacity(0.45)
                    } else {
                        p.line
                    },
                    if picked {
                        p.accent_info.opacity(0.62)
                    } else {
                        p.line_strong
                    },
                    picked,
                    div()
                        .p(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(5.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .truncate()
                                .child(kpi_lbl),
                        )
                        .child(
                            div()
                                .flex()
                                .items_end()
                                .justify_between()
                                .gap(px(8.))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, format!("kpi-roll-{}", kpi.id)),
                                    kpi.value,
                                    kpi.decimals,
                                    kpi.prefix.clone(),
                                    kpi.suffix.clone(),
                                    18.5,
                                    FontWeight::SEMIBOLD,
                                    p.strong,
                                ))
                                .child(Self::render_kpi_spark(&kpi.spark, p.accent_info, p.card)),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.selected_kpi = if this.selected_kpi == Some(idx) {
                            None
                        } else {
                            Some(idx)
                        };
                        cx.notify();
                    },
                )
                .flex_1()
                .min_w_0()
                .rounded(px(10.))
                .shadow(pro_card_shadow(p.dark))
                .opacity(card_alpha)
                .into_any_element(),
            );
        }

        let kpis_grid = if compact && kpi_cards.len() >= 4 {
            let mut it = kpi_cards.into_iter();
            let c0 = it.next();
            let c1 = it.next();
            let c2 = it.next();
            let c3 = it.next();
            div()
                .mb(px(10.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .when_some(c0, |d, el| d.child(el))
                        .when_some(c1, |d, el| d.child(el)),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(8.))
                        .when_some(c2, |d, el| d.child(el))
                        .when_some(c3, |d, el| d.child(el)),
                )
        } else {
            div().mb(px(10.)).flex().gap(px(10.)).children(kpi_cards)
        };

        // Main card toolbar: Section tabs + inline type filters (on wide desktop) + Search box
        let tab_sensor = pro_selection_sensor();
        let seg_thumb: Hsla = if p.dark {
            rgb(0x2c2c2e).into()
        } else {
            rgb(0xffffff).into()
        };
        let mut section_seg = div()
            .id(pro_eid(&self.id, "sec-tabs"))
            .role(Role::TabList)
            .aria_label(tr(cx, "Client sections", "客户工作区分栏"))
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .relative()
            .flex()
            .items_center()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "sec-tab-pill"),
                self.section_tab.key(),
                tab_sensor.clone(),
                6.,
                seg_thumb,
                Some(p.line),
                true,
            ));

        for st in ClientSectionTab::ALL {
            let active = self.section_tab == st;
            let st_lbl = st.label(cx);
            section_seg = section_seg.child(
                pro_tactile(
                    pro_eid(&self.id, format!("sec-tab-{}", st.key())),
                    st_lbl,
                    6.,
                    transparent_black(),
                    if active { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    active,
                    div()
                        .when(!compact, |d| d.px(px(12.)))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .whitespace_nowrap()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if active { p.strong } else { p.text })
                        .child(st_lbl),
                    cx,
                    move |this, _, cx| {
                        if this.section_tab != st {
                            this.section_tab = st;
                            cx.emit(ClientContextEvent::TabChanged(st));
                            cx.notify();
                        }
                    },
                )
                .relative()
                .when(compact, |d| d.flex_1())
                .when(!compact, |d| d.flex_none())
                .when(active, |d| d.child(pro_mark_selected(&tab_sensor))),
            );
        }

        let search_click = self.search_editor.clone();
        let search_clear = self.search_editor.clone();
        let has_q = self
            .search_editor
            .as_ref()
            .is_some_and(|e| !e.read(cx).value().is_empty());

        let wide_toolbar = !compact && container_w >= 760.;
        let search_box = div()
            .when(compact, |d| d.w_full())
            .when(!compact, |d| {
                d.w(px(if wide_toolbar { 154. } else { 196. }))
            })
            .h(px(28.))
            .px(px(8.))
            .rounded(px(6.))
            .bg(p.chip)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(6.))
            .cursor_text()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_, _, w, cx| {
                    if let Some(ed) = &search_click {
                        ed.read(cx).focus.clone().focus(w, cx);
                    }
                }),
            )
            .child(pro_icon(ProIcon::Search, 13., p.muted))
            .when_some(self.search_editor.clone(), |d, ed| {
                d.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(20.))
                        .overflow_hidden()
                        .child(ed),
                )
            })
            .when(has_q, |d| {
                d.child(pro_icon_btn(
                    pro_eid(&self.id, "ctx-search-clear"),
                    tr(cx, "Clear search", "清空搜索"),
                    ProIcon::Close,
                    18.,
                    false,
                    cx,
                    move |_, w, cx| {
                        if let Some(ed) = &search_clear {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                        cx.notify();
                    },
                ))
            });

        let toolbar_filters = if wide_toolbar && self.section_tab == ClientSectionTab::Context {
            Some(self.build_type_filter_pills(cx))
        } else {
            None
        };

        let card_toolbar = div()
            .flex()
            .when(compact, |d| d.flex_col().items_stretch().gap(px(10.)))
            .when(!compact, |d| {
                d.items_center().justify_between().gap(px(10.))
            })
            .child(section_seg)
            .when(self.section_tab == ClientSectionTab::Context, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(search_box)
                        .when_some(toolbar_filters, |r, fp| r.child(fp)),
                )
            });

        let tab_panel = match self.section_tab {
            ClientSectionTab::Context => {
                self.render_context_panel(&client, compact, !wide_toolbar, cx)
            }
            ClientSectionTab::Reports => self.render_reports_panel(&client, cx),
            ClientSectionTab::Financials => self.render_financials_panel(&client, compact, cx),
            ClientSectionTab::Settings => self.render_settings_panel(cx),
        };

        let main_card = div()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow(pro_card_shadow(p.dark))
            .p(px(if compact { 12. } else { 14. }))
            .flex()
            .flex_col()
            .child(card_toolbar)
            .child(pro_view_transition(
                pro_eid(&self.id, "sec-panel-vtx"),
                format!("{}-{}", client.id, self.section_tab.key()),
                tab_panel,
            ));

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(client.name)
            .w_full()
            .px(px(if compact { 16. } else { 28. }))
            .py(px(if compact { 16. } else { 20. }))
            .relative()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key.as_str() == "escape"
                    && (this.switcher_open || this.mention_menu_open)
                {
                    this.switcher_open = false;
                    this.mention_menu_open = false;
                    this.focus.focus(w, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .child(sensor)
            .child(header)
            .child(kpis_grid)
            .child(main_card)
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ClientContextBlock, ContextFile, ContextFileKind, ContextTypeFilter, default_clients,
    };
    use gpui::TestAppContext;

    #[test]
    fn default_clients_include_northwind_kpis_and_timeline_types() {
        let clients = default_clients();
        assert_eq!(clients.len(), 3);
        let nw = &clients[0];
        assert_eq!(nw.id.as_ref(), "northwind");
        assert_eq!(nw.kpis.len(), 4);
        assert_eq!(nw.kpis[0].value, 48.2);
        assert_eq!(nw.kpis[1].value, 612.0);
        assert_eq!(nw.kpis[2].value, 64.1);
        assert_eq!(nw.kpis[3].value, 36.0);
        assert_eq!(nw.context.len(), 5);
        assert_eq!(
            nw.context
                .iter()
                .filter(|e| ContextTypeFilter::Meetings.matches(e))
                .count(),
            1
        );
        assert_eq!(
            nw.context
                .iter()
                .filter(|e| ContextTypeFilter::Emails.matches(e))
                .count(),
            1
        );
        assert_eq!(
            nw.context
                .iter()
                .filter(|e| ContextTypeFilter::Files.matches(e))
                .count(),
            1
        );
    }

    #[test]
    fn switching_clients_and_adding_context_detects_report_mentions() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| ClientContextBlock::new("test-client-ctx", cx));

        w.update(|_, cx| {
            view.update(cx, |blk, cx| {
                assert_eq!(blk.active_client_id().as_ref(), "northwind");
                let added = blk.add_context_entry(
                    "Reviewed @Q3 Board Pack margin assumptions with finance.",
                    vec![ContextFile::new("Variance.xlsx", ContextFileKind::Xlsx)],
                    cx,
                );
                assert!(added);
                let nw = blk.active_client().expect("northwind exists");
                assert_eq!(nw.context.len(), 6);
                assert_eq!(nw.context[0].mention.as_deref(), Some("Q3 Board Pack"));
                assert_eq!(nw.context[0].files.len(), 1);

                blk.switch_client("lumen", cx);
                assert_eq!(blk.active_client_id().as_ref(), "lumen");
            });
        });
    }
}
