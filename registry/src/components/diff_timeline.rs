//! Pull-request edit-by-edit timeline with dual overlaid diff curves, per-file bars, and hover diff preview.
//!
//! Plots cumulative lines `+added` (green) and `−removed` (red) plus net delta over time via
//! [`PathBuilder`], lights up per-file segmented diff bars as the agent reaches each file, and
//! reveals an interactive monospace diff snippet on pointer scrub or arrow-key stepping.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_agent_avatar, pro_ease, pro_eid, pro_empty_state, pro_presence,
        pro_rolling_number, pro_skeleton_bar, pro_spring_f32, tr,
    },
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, f32::consts::TAU, rc::Rc};
use web_time::Instant;

const CHART_H: f32 = 156.0;
const SCALE_W: f32 = 40.0;
const FILE_ROW_H: f32 = 30.0;

/// Kind of a single line inside a [`DiffSnippetLine`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffLineKind {
    /// Unchanged surrounding context line (`' '`).
    #[default]
    Context,
    /// Inserted line (`'+'`).
    Add,
    /// Deleted line (`'-'`).
    Remove,
}

/// One line in an edit's monospace diff preview card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSnippetLine {
    /// Line kind (`Context`, `Add`, or `Remove`).
    pub kind: DiffLineKind,
    /// Source code text without leading `+`/`-` marker.
    pub code: SharedString,
}

impl DiffSnippetLine {
    /// Create a context line (`' '`).
    pub fn context(code: impl Into<SharedString>) -> Self {
        Self {
            kind: DiffLineKind::Context,
            code: code.into(),
        }
    }

    /// Create an added line (`'+'`).
    pub fn add(code: impl Into<SharedString>) -> Self {
        Self {
            kind: DiffLineKind::Add,
            code: code.into(),
        }
    }

    /// Create a removed line (`'-'`).
    pub fn remove(code: impl Into<SharedString>) -> Self {
        Self {
            kind: DiffLineKind::Remove,
            code: code.into(),
        }
    }
}

/// One atomic file edit performed by the agent at `at_secs` into the run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffEdit {
    /// Timestamp in seconds since the start of the run.
    pub at_secs: u32,
    /// Repository-relative file path (`"components/pricing-table.tsx"`).
    pub file: SharedString,
    /// Lines added in this edit.
    pub added: u32,
    /// Lines removed in this edit.
    pub removed: u32,
    /// Optional concise summary shown inside the hover diff popover.
    pub note: Option<SharedString>,
    /// Optional Chinese summary for bilingual display.
    pub note_zh: Option<SharedString>,
    /// Optional monospace diff lines previewed when inspecting this edit.
    pub diff: Vec<DiffSnippetLine>,
}

impl DiffEdit {
    /// Create a [`DiffEdit`] at `at_secs` for `file` with `added` and `removed` line counts.
    pub fn new(at_secs: u32, file: impl Into<SharedString>, added: u32, removed: u32) -> Self {
        Self {
            at_secs,
            file: file.into(),
            added,
            removed,
            note: None,
            note_zh: None,
            diff: Vec::new(),
        }
    }

    /// Attach a short human-readable note to this edit.
    pub fn with_note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Attach a localized Chinese note.
    pub fn with_note_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.note_zh = Some(zh.into());
        self
    }

    /// Attach monospace diff lines previewed on hover or keyboard focus.
    pub fn with_diff(mut self, diff: Vec<DiffSnippetLine>) -> Self {
        self.diff = diff;
        self
    }
}

/// Agent identity shown in the [`DiffTimeline`] header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffTimelineAgent {
    /// Deterministic seed for the 3x3 pixel avatar (`"frontend"`).
    pub id: SharedString,
    /// Display name (`"Frontend agent"`).
    pub name: SharedString,
}

impl DiffTimelineAgent {
    /// Create a [`DiffTimelineAgent`].
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// Typed events emitted by [`DiffTimeline`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffTimelineEvent {
    /// User inspected a specific edit index (`None` when inspection cleared).
    InspectEdit(Option<usize>),
    /// Playback or live mode restarted/toggled.
    ReplayStarted,
}

impl EventEmitter<DiffTimelineEvent> for DiffTimeline {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AggregatedFileRow {
    pub file: SharedString,
    pub added: u32,
    pub removed: u32,
    pub edit_indices: Vec<usize>,
}

/// Aggregate visible edits (`at_secs <= elapsed_secs`) into ordered per-file rows (first-touched order).
pub(crate) fn aggregate_visible_files(
    edits: &[DiffEdit],
    elapsed_secs: f32,
) -> Vec<AggregatedFileRow> {
    let mut rows: Vec<AggregatedFileRow> = Vec::new();
    for (idx, ed) in edits.iter().enumerate() {
        if (ed.at_secs as f32) > elapsed_secs + 0.001 {
            continue;
        }
        if let Some(existing) = rows.iter_mut().find(|r| r.file == ed.file) {
            existing.added += ed.added;
            existing.removed += ed.removed;
            existing.edit_indices.push(idx);
        } else {
            rows.push(AggregatedFileRow {
                file: ed.file.clone(),
                added: ed.added,
                removed: ed.removed,
                edit_indices: vec![idx],
            });
        }
    }
    rows
}

/// Compute a clean ceiling (`100`, `200`, `400`, etc.) for the Y-axis from ALL edits so the scale never jumps.
pub(crate) fn compute_y_scale_max(edits: &[DiffEdit]) -> u32 {
    let total_add: u32 = edits.iter().map(|e| e.added).sum();
    let total_del: u32 = edits.iter().map(|e| e.removed).sum();
    let peak = total_add.max(total_del).max(20);
    if peak <= 50 {
        50
    } else if peak <= 100 {
        100
    } else if peak <= 200 {
        200
    } else if peak <= 400 {
        400
    } else {
        peak.div_ceil(200) * 200
    }
}

fn split_dir_and_file(path: &str) -> (&str, &str) {
    if let Some(slash) = path.rfind('/') {
        (&path[..=slash], &path[slash + 1..])
    } else {
        ("", path)
    }
}

fn format_mm_ss(secs: u32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

fn file_icon_and_color(path: &str, p: &ProPalette) -> (ProIcon, Hsla) {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".svg") || lower.ends_with(".png") || lower.ends_with(".webp") {
        (ProIcon::Sparkles, p.pink)
    } else if lower.ends_with(".json") || lower.ends_with(".sql") || lower.ends_with(".csv") {
        (ProIcon::Database, p.green)
    } else if lower.ends_with('/') {
        (ProIcon::Folder, p.amber)
    } else {
        (ProIcon::Code, p.blue)
    }
}

fn default_pr_edits() -> Vec<DiffEdit> {
    vec![
        DiffEdit::new(18, "components/hero-banner.tsx", 52, 8)
            .with_note("Replace static callout with annual savings pill")
            .with_note_zh("将静态提示替换为年付折扣胶囊")
            .with_diff(vec![
                DiffSnippetLine::remove("  <span className=\"badge\">New</span>"),
                DiffSnippetLine::add("  <BillingBadge savePct={20} annual={annual} />"),
                DiffSnippetLine::context("  <h1 className=\"hero-title\">{heading}</h1>"),
            ]),
        DiffEdit::new(46, "lib/billing/tiers.ts", 64, 18)
            .with_note("Normalize monthly & annual Stripe price IDs")
            .with_note_zh("统一月付与年付 Stripe 价格标识结构")
            .with_diff(vec![
                DiffSnippetLine::remove("export const PRO_PRICE = \"price_monthly\";"),
                DiffSnippetLine::add("export const TIER_PRICES: Record<PlanId, CycleMap> = {"),
                DiffSnippetLine::add("  pro: { monthly: 29, annual: 24 },"),
            ]),
        DiffEdit::new(78, "components/pricing-table.tsx", 88, 24)
            .with_note("Build 3-column comparison grid with seat slider")
            .with_note_zh("构建三列套餐对比矩阵与席位滑杆")
            .with_diff(vec![
                DiffSnippetLine::context("export function PricingTable({ cycle }: Props) {"),
                DiffSnippetLine::remove("  return <LegacyPlanList plans={PLANS} />;"),
                DiffSnippetLine::add("  return <TierGrid plans={PLANS} seats={seats} />;"),
            ]),
        DiffEdit::new(112, "styles/pricing.css", 46, 31)
            .with_note("Drop legacy float rules for CSS subgrid")
            .with_note_zh("移除旧版浮动布局并迁移至 CSS subgrid")
            .with_diff(vec![
                DiffSnippetLine::remove(".plan-col { float: left; width: 33.3%; }"),
                DiffSnippetLine::add(".tier-grid { display: grid; gap: 16px; }"),
                DiffSnippetLine::add(".tier-card { grid-row: span 4; subgrid: rows; }"),
            ]),
        DiffEdit::new(142, "public/icons/enterprise-shield.svg", 24, 0)
            .with_note("Add crisp 16x16 SOC2 shield vector")
            .with_note_zh("新增 16x16 SOC2 安全盾牌矢量图标")
            .with_diff(vec![
                DiffSnippetLine::add("<svg viewBox=\"0 0 16 16\" fill=\"none\">"),
                DiffSnippetLine::add("  <path d=\"M8 1.5L13.5 4v4.2c0 3.4-2.4 5.6-5.5 6.3\" />"),
                DiffSnippetLine::add("</svg>"),
            ]),
        DiffEdit::new(168, "components/pricing-table.tsx", 56, 19)
            .with_note("Wire keyboard roving focus across billing toggle")
            .with_note_zh("为计费周期切换接入键盘方向键焦点漫游")
            .with_diff(vec![
                DiffSnippetLine::remove("  const onToggle = () => setAnnual(!annual);"),
                DiffSnippetLine::add("  const onKey = useRovingCycle(annual, setAnnual);"),
                DiffSnippetLine::add("  <BillingToggle onKeyDown={onKey} value={annual} />"),
            ]),
        DiffEdit::new(196, "tests/pricing-table.spec.ts", 54, 12)
            .with_note("Cover proration math and annual toggle snapshots")
            .with_note_zh("补充按比例计费与年付切换快照测试")
            .with_diff(vec![
                DiffSnippetLine::context("test(\"annual toggle updates seat total\", () => {"),
                DiffSnippetLine::add("  expect(calcAnnualTotal(5, \"pro\")).toBe(1440);"),
                DiffSnippetLine::add("});"),
            ]),
    ]
}

/// Pull-request edit-by-edit timeline card with dual area curves and per-file diff bars.
pub struct DiffTimeline {
    id: ElementId,
    agent: DiffTimelineAgent,
    context: Option<SharedString>,
    edits: Vec<DiffEdit>,
    duration_secs: u32,
    base_elapsed_secs: f32,
    live: bool,
    max_rows: usize,
    inspected_edit: Option<usize>,
    hovered_file: Option<SharedString>,
    chart_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    started_at: Instant,
    replay_from_zero: bool,
    loading: bool,
    empty: bool,
}

impl DiffTimeline {
    /// Create a [`DiffTimeline`] for `"Frontend agent · PR #214"` with 7 edits across 6 files.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            agent: DiffTimelineAgent::new("frontend", "Frontend agent"),
            context: Some("PR #214".into()),
            edits: default_pr_edits(),
            duration_secs: 240,
            base_elapsed_secs: 204.0,
            live: true,
            max_rows: 7,
            inspected_edit: None,
            hovered_file: None,
            chart_bounds: Rc::new(Cell::new(None)),
            started_at: Instant::now(),
            replay_from_zero: false,
            loading: false,
            empty: false,
        }
    }

    /// Set the agent identity (`id` seeds the 3x3 pixel mark).
    pub fn with_agent(mut self, agent: DiffTimelineAgent) -> Self {
        self.agent = agent;
        self
    }

    /// Set or clear the quiet context string after the agent name (`"PR #214"`).
    pub fn with_context(mut self, context: Option<SharedString>) -> Self {
        self.context = context;
        self
    }

    /// Supply ordered file edits (`at_secs`, `file`, `added`, `removed`, `diff`).
    pub fn with_edits(mut self, edits: Vec<DiffEdit>) -> Self {
        let last_at = edits.iter().map(|e| e.at_secs).max().unwrap_or(60);
        self.duration_secs = last_at.div_ceil(60).max(1) * 60;
        self.base_elapsed_secs = last_at as f32;
        self.edits = edits;
        self
    }

    /// Set the current elapsed run time in seconds (edits beyond `elapsed` stay hidden).
    pub fn with_elapsed(mut self, elapsed_secs: f32) -> Self {
        self.base_elapsed_secs = elapsed_secs.max(0.);
        self.replay_from_zero = false;
        self
    }

    /// Set the total X-axis duration in seconds (`240` = `4:00`).
    pub fn with_duration(mut self, duration_secs: u32) -> Self {
        self.duration_secs = duration_secs.max(30);
        self
    }

    /// Enable or disable live mode (twinkling agent mark, breathing curve tips, and live status line).
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Cap visible file rows before vertical scrolling (`7` by default).
    pub fn with_max_rows(mut self, max_rows: usize) -> Self {
        self.max_rows = max_rows.max(2);
        self
    }

    /// Render skeleton placeholder bars.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state view.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Current effective elapsed seconds into the run.
    pub fn current_elapsed_secs(&self, reduced: bool) -> f32 {
        let dur = self.duration_secs as f32;
        if self.replay_from_zero && !reduced {
            let speed = 36.0; // replays 4-minute PR in ~6.6 seconds
            (self.started_at.elapsed().as_secs_f32() * speed).min(dur)
        } else if self.live && !reduced {
            (self.base_elapsed_secs + self.started_at.elapsed().as_secs_f32() * 1.5).min(dur)
        } else {
            self.base_elapsed_secs.min(dur)
        }
    }

    /// Select an edit index for diff inspection (`None` clears).
    pub fn set_inspected_edit(&mut self, edit_idx: Option<usize>, cx: &mut Context<Self>) {
        if self.inspected_edit != edit_idx {
            self.inspected_edit = edit_idx;
            cx.emit(DiffTimelineEvent::InspectEdit(edit_idx));
            cx.notify();
        }
    }

    /// Restart animated playback of the edit timeline from `0:00`.
    pub fn replay(&mut self, cx: &mut Context<Self>) {
        self.replay_from_zero = true;
        self.live = true;
        self.inspected_edit = None;
        self.started_at = Instant::now();
        cx.emit(DiffTimelineEvent::ReplayStarted);
        cx.notify();
    }
}

#[allow(clippy::type_complexity)]
fn build_cumulative_series(
    edits: &[DiffEdit],
    elapsed_secs: f32,
) -> (Vec<(f32, f32)>, Vec<(f32, f32)>, Vec<(f32, f32)>) {
    let mut add_pts = vec![(0.0f32, 0.0f32)];
    let mut del_pts = vec![(0.0f32, 0.0f32)];
    let mut net_pts = vec![(0.0f32, 0.0f32)];

    let mut cum_add = 0.0f32;
    let mut cum_del = 0.0f32;
    for ed in edits {
        let t = ed.at_secs as f32;
        if t > elapsed_secs + 0.001 {
            break;
        }
        // Subtle 6s ramp before each edit lands so the cumulative step slope reads crisply
        let pre_t = (t - 6.0).max(add_pts.last().map(|p| p.0).unwrap_or(0.0));
        if pre_t > add_pts.last().map(|p| p.0).unwrap_or(0.0) + 0.5 {
            add_pts.push((pre_t, cum_add));
            del_pts.push((pre_t, cum_del));
            net_pts.push((pre_t, (cum_add - cum_del).max(0.0)));
        }
        cum_add += ed.added as f32;
        cum_del += ed.removed as f32;
        add_pts.push((t, cum_add));
        del_pts.push((t, cum_del));
        net_pts.push((t, (cum_add - cum_del).max(0.0)));
    }

    if elapsed_secs > add_pts.last().map(|p| p.0).unwrap_or(0.0) + 0.2 {
        add_pts.push((elapsed_secs, cum_add));
        del_pts.push((elapsed_secs, cum_del));
        net_pts.push((elapsed_secs, (cum_add - cum_del).max(0.0)));
    }
    (add_pts, del_pts, net_pts)
}

impl Render for DiffTimeline {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let dur_f = (self.duration_secs.max(1)) as f32;
        let elapsed_s = self.current_elapsed_secs(reduced);
        let still_animating =
            (self.live || self.replay_from_zero) && elapsed_s < dur_f - 0.05 && !reduced;
        if still_animating || (self.loading && !reduced) || (self.live && !reduced) {
            window.request_animation_frame();
        }
        if self.replay_from_zero && elapsed_s >= dur_f - 0.05 {
            self.replay_from_zero = false;
            self.base_elapsed_secs = dur_f;
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let pop_bg: Hsla = if p.dark {
            rgb(0x222226).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let add_c = p.green;
        let add_ink = p.green_ink;
        let add_bg = p.green_soft;
        let del_c = p.red;
        let del_ink = p.red_ink;
        let del_bg = p.red_soft;
        let agent_mark_c: Hsla = if p.dark {
            rgba(0x4d8dffd9).into()
        } else {
            rgba(0x0060e2c7).into()
        };

        let visible_edit_indices: Vec<usize> = self
            .edits
            .iter()
            .enumerate()
            .filter(|(_, e)| (e.at_secs as f32) <= elapsed_s + 0.001)
            .map(|(i, _)| i)
            .collect();

        let file_rows = aggregate_visible_files(&self.edits, elapsed_s);
        let total_added: u32 = file_rows.iter().map(|r| r.added).sum();
        let total_removed: u32 = file_rows.iter().map(|r| r.removed).sum();
        let net_delta: i64 = i64::from(total_added) - i64::from(total_removed);

        let wall_ms = self.started_at.elapsed().as_secs_f32() * 1000.0;
        let is_live_now = self.live && elapsed_s < dur_f - 0.05;

        let latest_file: Option<&str> = visible_edit_indices
            .last()
            .map(|&i| self.edits[i].file.as_ref());

        let status_line: AnyElement = if let Some(fp) = latest_file {
            let (dir_part, base_part) = split_dir_and_file(fp);
            if is_live_now {
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .child(
                        div()
                            .text_color(p.muted)
                            .child(tr(cx, "Editing", "正在编辑")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .min_w_0()
                            .truncate()
                            .child(div().text_color(p.faint).child(dir_part.to_string()))
                            .child(div().text_color(p.ink).child(base_part.to_string())),
                    )
                    .into_any_element()
            } else {
                let last_at = visible_edit_indices
                    .last()
                    .map(|&i| self.edits[i].at_secs)
                    .unwrap_or(0);
                div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.muted)
                    .child(format!(
                        "{} {}",
                        tr(cx, "Completed at", "完成于"),
                        format_mm_ss(last_at)
                    ))
                    .into_any_element()
            }
        } else {
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(p.muted)
                .child(tr(cx, "Waiting for the first edit…", "等待首次代码修改…"))
                .into_any_element()
        };

        let files_count = file_rows.len();
        let across_text = if tr(cx, "en", "zh") == "zh" {
            format!("涉及 {files_count} 个文件")
        } else if files_count == 1 {
            "across 1 file".to_string()
        } else {
            format!("across {files_count} files")
        };

        let header = div()
            .pt(px(14.))
            .px(px(16.))
            .flex()
            .items_center()
            .gap(px(10.))
            .child(pro_agent_avatar(
                self.agent.id.as_ref(),
                agent_mark_c,
                24.,
                is_live_now,
                wall_ms,
                cx,
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(if self.agent.name.as_ref() == "Frontend agent" {
                                        tr(cx, "Frontend agent", "前端智能体").into()
                                    } else {
                                        self.agent.name.clone()
                                    }),
                            )
                            .when_some(self.context.clone(), |d, ctx_txt| {
                                d.child(div().text_color(p.muted).child(format!("· {ctx_txt}")))
                            }),
                    )
                    .child(status_line),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .child(
                        div()
                            .h(px(20.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(add_bg)
                            .flex()
                            .items_center()
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "tot-add"),
                                f64::from(total_added),
                                0,
                                "+",
                                "",
                                12.,
                                FontWeight::MEDIUM,
                                add_ink,
                            )),
                    )
                    .child(
                        div()
                            .h(px(20.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(del_bg)
                            .flex()
                            .items_center()
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "tot-del"),
                                f64::from(total_removed),
                                0,
                                "−",
                                "",
                                12.,
                                FontWeight::MEDIUM,
                                del_ink,
                            )),
                    )
                    .child(div().ml(px(2.)).text_color(p.muted).child(across_text)),
            );

        if self.loading {
            let phase = if reduced {
                0.5
            } else {
                (wall_ms / 1100.0).fract()
            };
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(580.))
                        .rounded(px(10.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .when(!p.dark, |d| d.shadow_sm())
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .pb(px(16.))
                        .child(header)
                        .child(
                            div()
                                .px(px(16.))
                                .child(pro_skeleton_bar(540., 150., 8., phase, cx)),
                        )
                        .child(
                            div()
                                .px(px(16.))
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(pro_skeleton_bar(520., 22., 6., phase + 0.1, cx))
                                .child(pro_skeleton_bar(480., 22., 6., phase + 0.2, cx))
                                .child(pro_skeleton_bar(500., 22., 6., phase + 0.3, cx)),
                        ),
                )
                .into_any_element();
        }

        if self.empty || self.edits.is_empty() {
            let empty_el = pro_empty_state(
                ProIcon::Code,
                tr(cx, "No pull request edits", "暂无代码修改记录"),
                tr(
                    cx,
                    "Line additions, removals, and per-file diff bars appear here as edits land.",
                    "智能体修改文件时，增删行数曲线与单文件进度条将实时呈现。",
                ),
                None,
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
                        .max_w(px(580.))
                        .rounded(px(10.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .when(!p.dark, |d| d.shadow_sm())
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(empty_el),
                )
                .into_any_element();
        }

        let y_max = compute_y_scale_max(&self.edits);
        let y_max_f = y_max as f32;

        // Left Y-axis scale (`0, 25%, 50%, 75%, 100%`)
        let mut y_scale_col = div().w(px(SCALE_W)).h(px(CHART_H)).relative().flex_none();
        for step in 0..=4u32 {
            let val = (y_max * (4 - step)) / 4;
            let top_px = (step as f32 / 4.0) * CHART_H - 8.0;
            y_scale_col = y_scale_col.child(
                div()
                    .absolute()
                    .right(px(10.))
                    .top(px(top_px))
                    .text_size(px(11.))
                    .line_height(px(16.))
                    .text_color(p.faint)
                    .child(val.to_string()),
            );
        }

        let (add_series, del_series, net_series) = build_cumulative_series(&self.edits, elapsed_s);
        let hovered_file_pins: Vec<(f32, bool)> = if let Some(hf) = &self.hovered_file {
            self.edits
                .iter()
                .filter(|e| &e.file == hf && (e.at_secs as f32) <= elapsed_s + 0.001)
                .map(|e| (e.at_secs as f32 / dur_f, e.added >= e.removed))
                .collect()
        } else {
            Vec::new()
        };

        let inspected_x_frac: Option<f32> = self
            .inspected_edit
            .and_then(|idx| self.edits.get(idx))
            .filter(|e| (e.at_secs as f32) <= elapsed_s + 0.001)
            .map(|e| (e.at_secs as f32 / dur_f).clamp(0., 1.));

        let bounds_cell = self.chart_bounds.clone();
        let grid_c = p.line;
        let base_c = p.line_strong;
        let net_c = p.faint;
        let now_frac = (elapsed_s / dur_f).clamp(0., 1.);
        let glow_phase = if reduced {
            0.0
        } else {
            (wall_ms / 1600.0).fract()
        };

        let chart_canvas = canvas(
            move |bounds, _, _| {
                bounds_cell.set(Some(bounds));
            },
            move |bounds, _, win, _| {
                let cw = f32::from(bounds.size.width);
                let ch = f32::from(bounds.size.height);
                if cw <= 4.0 || ch <= 4.0 {
                    return;
                }
                let ox = f32::from(bounds.left());
                let oy = f32::from(bounds.top());
                let map_xy = |t_sec: f32, val: f32| -> Point<Pixels> {
                    let ux = (t_sec / dur_f).clamp(0., 1.);
                    let uy = 1.0 - (val / y_max_f).clamp(0., 1.);
                    point(px(ox + ux * cw), px(oy + uy * ch))
                };

                // Horizontal dashed gridlines (`0..4`) + solid baseline at bottom
                for g in 0..4usize {
                    let gy = oy + (g as f32 / 4.0) * ch;
                    let dashes = ((cw / 6.0) as usize).max(1);
                    for d in 0..dashes {
                        let x0 = ox + (d as f32) * 6.0;
                        let x1 = (x0 + 2.2).min(ox + cw);
                        let mut seg = PathBuilder::stroke(px(1.0));
                        seg.move_to(point(px(x0), px(gy)));
                        seg.line_to(point(px(x1), px(gy)));
                        if let Ok(path) = seg.build() {
                            win.paint_path(path, grid_c);
                        }
                    }
                }
                let mut base = PathBuilder::stroke(px(1.0));
                base.move_to(point(px(ox), px(oy + ch)));
                base.line_to(point(px(ox + cw), px(oy + ch)));
                if let Ok(path) = base.build() {
                    win.paint_path(path, base_c);
                }

                // Vertical dashed "now" cursor while live
                if is_live_now && now_frac > 0.01 && now_frac < 0.995 {
                    let nx = ox + now_frac * cw;
                    let vdashes = ((ch / 5.0) as usize).max(1);
                    for d in 0..vdashes {
                        let y0 = oy + (d as f32) * 5.0;
                        let y1 = (y0 + 2.0).min(oy + ch);
                        let mut seg = PathBuilder::stroke(px(1.0));
                        seg.move_to(point(px(nx), px(y0)));
                        seg.line_to(point(px(nx), px(y1)));
                        if let Ok(path) = seg.build() {
                            win.paint_path(path, base_c);
                        }
                    }
                }

                // Helper closure to paint area + stroke for a series
                let paint_series =
                    |pts: &[(f32, f32)], color: Hsla, fill_top_alpha: f32, win: &mut Window| {
                        if pts.len() < 2 {
                            return;
                        }
                        if fill_top_alpha > 0.001 {
                            let mut area = PathBuilder::fill();
                            let first = map_xy(pts[0].0, pts[0].1);
                            let last = map_xy(pts[pts.len() - 1].0, pts[pts.len() - 1].1);
                            area.move_to(point(first.x, px(oy + ch)));
                            area.line_to(first);
                            for &(t, v) in pts.iter().skip(1) {
                                area.line_to(map_xy(t, v));
                            }
                            area.line_to(point(last.x, px(oy + ch)));
                            area.close();
                            if let Ok(path) = area.build() {
                                win.paint_path(
                                    path,
                                    linear_gradient(
                                        180.,
                                        linear_color_stop(color.opacity(fill_top_alpha), 0.),
                                        linear_color_stop(color.opacity(0.), 1.),
                                    ),
                                );
                            }
                        }
                        let mut line = PathBuilder::stroke(px(1.65));
                        for (i, &(t, v)) in pts.iter().enumerate() {
                            let pt = map_xy(t, v);
                            if i == 0 {
                                line.move_to(pt);
                            } else {
                                line.line_to(pt);
                            }
                        }
                        if let Ok(path) = line.build() {
                            win.paint_path(path, color);
                        }
                    };

                // 1) Removed curve + gradient wash
                paint_series(&del_series, del_c, if p.dark { 0.18 } else { 0.14 }, win);
                // 2) Added curve + gradient wash
                paint_series(&add_series, add_c, if p.dark { 0.22 } else { 0.19 }, win);

                // 3) Dashed Net delta curve
                if net_series.len() >= 2 {
                    for w in net_series.windows(2) {
                        let p0 = map_xy(w[0].0, w[0].1);
                        let p1 = map_xy(w[1].0, w[1].1);
                        let dx = f32::from(p1.x - p0.x);
                        let dy = f32::from(p1.y - p0.y);
                        let seg_len = dx.hypot(dy);
                        let steps = ((seg_len / 6.0).ceil() as usize).max(1);
                        for k in 0..steps {
                            let u0 = k as f32 / steps as f32;
                            let u1 = ((k as f32 + 0.5) / steps as f32).min(1.0);
                            let mut dash = PathBuilder::stroke(px(1.1));
                            dash.move_to(point(p0.x + px(dx * u0), p0.y + px(dy * u0)));
                            dash.line_to(point(p0.x + px(dx * u1), p0.y + px(dy * u1)));
                            if let Ok(path) = dash.build() {
                                win.paint_path(path, net_c);
                            }
                        }
                    }
                }

                // Vertical inspection hairline when hovering/scrubbing an edit
                if let Some(ix) = inspected_x_frac {
                    let gx = ox + ix * cw;
                    let mut gline = PathBuilder::stroke(px(1.0));
                    gline.move_to(point(px(gx), px(oy)));
                    gline.line_to(point(px(gx), px(oy + ch)));
                    if let Ok(path) = gline.build() {
                        win.paint_path(path, base_c);
                    }
                }

                // Baseline edit pins for a hovered file row
                for &(fx, mostly_add) in &hovered_file_pins {
                    let pc = point(px(ox + fx * cw), px(oy + ch));
                    let col = if mostly_add { add_c } else { del_c };
                    paint_tip_dot(pc, 2.8, col, card_bg, 1.8, win);
                }

                // End dots (+ breathing glow rings while live!)
                if let Some(&(lt, lv)) = net_series.last() {
                    paint_tip_dot(map_xy(lt, lv), 2.5, net_c, card_bg, 1.4, win);
                }
                if let Some(&(lt, lv)) = del_series.last() {
                    let tip = map_xy(lt, lv);
                    if is_live_now && !reduced {
                        let gp = (glow_phase + 0.25).fract();
                        let gr = 3.0 + 6.5 * pro_ease(gp);
                        paint_tip_dot(
                            tip,
                            gr,
                            del_c.opacity((1.0 - gp) * 0.30),
                            transparent_black(),
                            0.,
                            win,
                        );
                    }
                    paint_tip_dot(tip, 3.2, del_c, card_bg, 1.6, win);
                }
                if let Some(&(lt, lv)) = add_series.last() {
                    let tip = map_xy(lt, lv);
                    if is_live_now && !reduced {
                        let gr = 3.0 + 6.5 * pro_ease(glow_phase);
                        paint_tip_dot(
                            tip,
                            gr,
                            add_c.opacity((1.0 - glow_phase) * 0.32),
                            transparent_black(),
                            0.,
                            win,
                        );
                    }
                    paint_tip_dot(tip, 3.2, add_c, card_bg, 1.6, win);
                }
            },
        )
        .w_full()
        .h(px(CHART_H));

        // X-axis (`0:00`, `1:00`, `2:00`, `3:00`, `4:00`)
        let mut x_axis = div()
            .w_full()
            .h(px(24.))
            .flex()
            .items_center()
            .justify_between();
        for step in 0..=4u32 {
            let secs = (self.duration_secs * step) / 4;
            x_axis = x_axis.child(
                div()
                    .text_size(px(11.))
                    .line_height(px(16.))
                    .text_color(p.faint)
                    .child(format_mm_ss(secs)),
            );
        }

        // Floating edit diff preview popover (`pro_presence`)
        let active_edit_opt = self
            .inspected_edit
            .and_then(|i| self.edits.get(i).cloned().map(|ed| (i, ed)));
        let is_zh = tr(cx, "en", "zh") == "zh";
        let added_lbl = tr(cx, "Added", "新增").to_string();
        let removed_lbl = tr(cx, "Removed", "删除").to_string();
        let tip_layer = pro_presence(
            pro_eid(&self.id, "diff-tip"),
            active_edit_opt.is_some(),
            0.16,
            0.12,
            move |amt| {
                let Some((_, ed)) = active_edit_opt.as_ref() else {
                    return div().into_any_element();
                };
                let u_x = (ed.at_secs as f32 / dur_f).clamp(0., 1.);
                // Position tooltip cleanly inside chart width without overflowing right edge
                let align_right = u_x > 0.56;
                let (dir_p, file_p) = split_dir_and_file(ed.file.as_ref());
                let note_txt = match (&ed.note, &ed.note_zh) {
                    (Some(en), Some(zh)) => Some(if is_zh { zh.clone() } else { en.clone() }),
                    (Some(en), None) => Some(en.clone()),
                    _ => None,
                };

                let mut diff_box = div()
                    .mt(px(7.))
                    .rounded(px(5.))
                    .bg(p.hover)
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .font_family("Menlo")
                    .text_size(px(10.5))
                    .line_height(px(17.));

                for dl in &ed.diff {
                    let (row_bg, sign, sign_c, code_c) = match dl.kind {
                        DiffLineKind::Add => (add_bg, "+", add_ink, p.ink),
                        DiffLineKind::Remove => (del_bg, "−", del_ink, p.ink),
                        DiffLineKind::Context => (transparent_black(), " ", p.faint, p.muted),
                    };
                    diff_box = diff_box.child(
                        div()
                            .w_full()
                            .pr(px(8.))
                            .bg(row_bg)
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .w(px(18.))
                                    .flex_none()
                                    .text_center()
                                    .text_color(sign_c)
                                    .child(sign),
                            )
                            .child(div().truncate().text_color(code_c).child(dl.code.clone())),
                    );
                }

                let mut card = div()
                    .w(px(258.))
                    .p(px(9.))
                    .rounded(px(10.))
                    .bg(pop_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_md()
                    .opacity(amt)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .font_weight(FontWeight::MEDIUM)
                                    .flex()
                                    .items_center()
                                    .child(div().text_color(p.faint).child(dir_p.to_string()))
                                    .child(div().text_color(p.ink).child(file_p.to_string())),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_color(p.muted)
                                    .child(format_mm_ss(ed.at_secs)),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(6.))
                            .pt(px(6.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .text_size(px(12.))
                            .child(
                                div()
                                    .pl(px(7.))
                                    .border_l_2()
                                    .border_color(add_c)
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(div().text_color(p.muted).child(added_lbl))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(add_ink)
                                            .child(format!("+{}", ed.added)),
                                    ),
                            )
                            .child(
                                div()
                                    .pl(px(7.))
                                    .border_l_2()
                                    .border_color(del_c)
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(div().text_color(p.muted).child(removed_lbl))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(del_ink)
                                            .child(format!("−{}", ed.removed)),
                                    ),
                            ),
                    );

                if let Some(nt) = note_txt {
                    card = card.child(
                        div()
                            .mt(px(6.))
                            .text_size(px(11.5))
                            .line_height(px(16.))
                            .text_color(p.muted)
                            .child(nt),
                    );
                }
                if !ed.diff.is_empty() {
                    card = card.child(diff_box);
                }

                let wrapper = div().absolute().top(px(-8. + 4. * (1. - amt)));
                if align_right {
                    wrapper.right(px(6.)).child(card).into_any_element()
                } else {
                    wrapper.left(px(6.)).child(card).into_any_element()
                }
            },
        );

        let bounds_for_move = self.chart_bounds.clone();
        let vis_for_move = visible_edit_indices.clone();
        let vis_for_keys = visible_edit_indices.clone();

        let chart_main = div()
            .id(pro_eid(&self.id, "chart-main"))
            .role(Role::Group)
            .aria_label(tr(
                cx,
                "Lines added and removed over time; use Left and Right arrow keys to step through edits",
                "随时间累加的增删代码行数曲线；使用左右方向键逐步检视每次修改",
            ))
            .tab_index(0)
            .flex_1()
            .min_w_0()
            .relative()
            .rounded(px(4.))
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                let Some(b) = bounds_for_move.get() else {
                    return;
                };
                if vis_for_move.is_empty() {
                    return;
                }
                let bw = f32::from(b.size.width).max(1.);
                let rel_x = (f32::from(ev.position.x - b.left()) / bw).clamp(0., 1.);
                let hover_sec = rel_x * (this.duration_secs as f32);
                let nearest = vis_for_move.iter().copied().min_by(|&a, &b_idx| {
                    let da = (this.edits[a].at_secs as f32 - hover_sec).abs();
                    let db = (this.edits[b_idx].at_secs as f32 - hover_sec).abs();
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                });
                this.set_inspected_edit(nearest, cx);
            }))
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if !*inside && this.inspected_edit.is_some() {
                    this.set_inspected_edit(None, cx);
                }
            }))
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                if vis_for_keys.is_empty() {
                    return;
                }
                let cur_pos = this
                    .inspected_edit
                    .and_then(|ei| vis_for_keys.iter().position(|&x| x == ei));
                match ev.keystroke.key.as_str() {
                    "left" | "arrowleft" => {
                        let next_pos = match cur_pos {
                            Some(pos) => pos.saturating_sub(1),
                            None => vis_for_keys.len() - 1,
                        };
                        this.set_inspected_edit(Some(vis_for_keys[next_pos]), cx);
                        cx.stop_propagation();
                    }
                    "right" | "arrowright" => {
                        let next_pos = match cur_pos {
                            Some(pos) => (pos + 1).min(vis_for_keys.len() - 1),
                            None => 0,
                        };
                        this.set_inspected_edit(Some(vis_for_keys[next_pos]), cx);
                        cx.stop_propagation();
                    }
                    "home" => {
                        this.set_inspected_edit(Some(vis_for_keys[0]), cx);
                        cx.stop_propagation();
                    }
                    "end" => {
                        if let Some(&last) = vis_for_keys.last() {
                            this.set_inspected_edit(Some(last), cx);
                        }
                        cx.stop_propagation();
                    }
                    "escape" => {
                        this.set_inspected_edit(None, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(chart_canvas)
            .child(x_axis)
            .child(tip_layer);

        let chart_body = div()
            .w_full()
            .pt(px(22.))
            .pr(px(16.))
            .pb(px(6.))
            .flex()
            .items_start()
            .child(y_scale_col)
            .child(chart_main);

        // Per-file diff progress bars (`ui-dtlc__files`)
        let max_file_churn: u32 = self
            .edits
            .iter()
            .fold(
                std::collections::HashMap::<&str, u32>::new(),
                |mut acc, e| {
                    *acc.entry(e.file.as_ref()).or_insert(0) += e.added + e.removed;
                    acc
                },
            )
            .into_values()
            .max()
            .unwrap_or(1)
            .max(1);

        let active_file_name: Option<SharedString> = self
            .inspected_edit
            .and_then(|ei| self.edits.get(ei).map(|e| e.file.clone()))
            .or_else(|| self.hovered_file.clone())
            .or_else(|| {
                if is_live_now {
                    latest_file.map(|s| SharedString::from(s.to_string()))
                } else {
                    None
                }
            });

        let active_row_idx = active_file_name
            .as_ref()
            .and_then(|af| file_rows.iter().position(|r| &r.file == af));

        let wash_y = pro_spring_f32(
            pro_eid(&self.id, "file-wash-y"),
            active_row_idx.unwrap_or(0) as f32 * FILE_ROW_H,
            0.28,
            window,
            cx,
        );

        let any_file_hovered = self.hovered_file.is_some();
        let mut files_list =
            div()
                .w_full()
                .relative()
                .flex()
                .flex_col()
                .when(active_row_idx.is_some(), |d| {
                    d.child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .top(px(wash_y))
                            .h(px(FILE_ROW_H))
                            .rounded(px(7.))
                            .bg(p.hover),
                    )
                });

        for frow in &file_rows {
            let fname = frow.file.clone();
            let (glyph_icon, glyph_color) = file_icon_and_color(fname.as_ref(), &p);
            let (dir_str, base_str) = split_dir_and_file(fname.as_ref());
            let is_this_on = active_file_name.as_ref() == Some(&fname);
            let row_opacity = if any_file_hovered && !is_this_on {
                0.42
            } else {
                1.0
            };

            let file_churn = (frow.added + frow.removed).max(1);
            let target_bar_ratio = (file_churn as f32 / max_file_churn as f32).clamp(0.06, 1.0);
            let bar_ratio = pro_spring_f32(
                pro_eid(&self.id, format!("fbar-{fname}")),
                target_bar_ratio,
                0.38,
                window,
                cx,
            );

            let mut seg_bar = div()
                .w(relative(bar_ratio))
                .min_w(px(8.))
                .h(px(6.))
                .flex()
                .items_center()
                .gap(px(2.));

            for &e_idx in &frow.edit_indices {
                let ed = &self.edits[e_idx];
                let ed_total = (ed.added + ed.removed).max(1) as f32;
                let add_share = (ed.added as f32 / ed_total).clamp(0., 1.);
                let del_share = (ed.removed as f32 / ed_total).clamp(0., 1.);
                let is_inspected = self.inspected_edit == Some(e_idx);
                let dim_piece = self.inspected_edit.is_some() && is_this_on && !is_inspected;

                seg_bar = seg_bar.child(
                    div()
                        .id(pro_eid(&self.id, format!("epiece-{fname}-{e_idx}")))
                        .flex_grow(1.0)
                        .min_w(px(4.))
                        .h_full()
                        .rounded(px(2.))
                        .overflow_hidden()
                        .opacity(if dim_piece { 0.32 } else { 1.0 })
                        .flex()
                        .cursor_pointer()
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.set_inspected_edit(Some(e_idx), cx);
                            }
                        }))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            let next = if this.inspected_edit == Some(e_idx) {
                                None
                            } else {
                                Some(e_idx)
                            };
                            this.set_inspected_edit(next, cx);
                        }))
                        .when(add_share > 0.001, |d| {
                            d.child(
                                div()
                                    .h_full()
                                    .w(relative(add_share))
                                    .bg(add_c.opacity(0.82)),
                            )
                        })
                        .when(del_share > 0.001, |d| {
                            d.child(
                                div()
                                    .h_full()
                                    .w(relative(del_share))
                                    .bg(del_c.opacity(0.82)),
                            )
                        }),
                );
            }

            let hov_fname = fname.clone();
            let first_edit_for_file = frow.edit_indices.last().copied();
            files_list = files_list.child(
                div()
                    .id(pro_eid(&self.id, format!("frow-{fname}")))
                    .role(Role::ListItem)
                    .w_full()
                    .h(px(FILE_ROW_H))
                    .px(px(8.))
                    .rounded(px(7.))
                    .relative()
                    .opacity(row_opacity)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        let next = if *inside {
                            Some(hov_fname.clone())
                        } else {
                            None
                        };
                        if this.hovered_file != next {
                            this.hovered_file = next;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.set_inspected_edit(first_edit_for_file, cx);
                    }))
                    .child(pro_icon(glyph_icon, 14., glyph_color))
                    .child(
                        div()
                            .w(px(196.))
                            .flex_none()
                            .min_w_0()
                            .truncate()
                            .flex()
                            .items_center()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .child(
                                div()
                                    .truncate()
                                    .text_color(p.faint)
                                    .child(dir_str.to_string()),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_color(p.ink)
                                    .child(base_str.to_string()),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .child(seg_bar),
                    )
                    .child(
                        div()
                            .w(px(76.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(
                                div()
                                    .text_color(if frow.added == 0 { p.faint } else { add_ink })
                                    .child(format!("+{}", frow.added)),
                            )
                            .child(
                                div()
                                    .text_color(if frow.removed == 0 { p.faint } else { del_ink })
                                    .child(format!("−{}", frow.removed)),
                            ),
                    ),
            );
        }

        let files_scroll = div()
            .id(pro_eid(&self.id, "files-scroll"))
            .role(Role::List)
            .aria_label(tr(cx, "Modified files", "已修改文件列表"))
            .mx(px(8.))
            .mt(px(2.))
            .mb(px(8.))
            .min_h(px((self.edits.len().min(self.max_rows) as f32) * FILE_ROW_H))
            .max_h(px((self.max_rows as f32) * FILE_ROW_H))
            .when(file_rows.len() > self.max_rows, |d| d.overflow_y_scroll())
            .child(files_list);

        let net_str = if net_delta >= 0 {
            format!("+{net_delta}")
        } else {
            format!("−{}", net_delta.unsigned_abs())
        };

        let footer = div()
            .px(px(16.))
            .pt(px(6.))
            .pb(px(14.))
            .flex()
            .items_center()
            .gap(px(16.))
            .text_size(px(12.))
            .line_height(px(16.))
            .text_color(p.muted)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().w(px(12.)).h(px(2.)).rounded(px(1.)).bg(add_c))
                    .child(tr(cx, "Added", "新增")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().w(px(12.)).h(px(2.)).rounded(px(1.)).bg(del_c))
                    .child(tr(cx, "Removed", "删除")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().w(px(12.)).h(px(1.)).border_t_1().border_color(net_c))
                    .child(tr(cx, "Net", "净增"))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(net_str),
                    ),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "replay-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Replay edit timeline", "重播修改时间线"))
                    .ml_auto()
                    .h(px(22.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .cursor_pointer()
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.replay(cx);
                    }))
                    .child(pro_icon(ProIcon::Refresh, 11.5, p.muted))
                    .child(tr(cx, "Replay", "重放")),
            );

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(580.))
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .when(!p.dark, |d| d.shadow_sm())
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(chart_body)
                    .child(files_scroll)
                    .child(footer),
            )
            .into_any_element()
    }
}

fn paint_tip_dot(
    center: Point<Pixels>,
    radius: f32,
    fill_c: Hsla,
    ring_c: Hsla,
    ring_w: f32,
    win: &mut Window,
) {
    if radius <= 0.3 {
        return;
    }
    let steps = 20usize;
    let mut disc = PathBuilder::fill();
    for i in 0..=steps {
        let a = (i as f32 / steps as f32) * TAU;
        let pt = point(
            center.x + px(radius * a.cos()),
            center.y + px(radius * a.sin()),
        );
        if i == 0 {
            disc.move_to(pt);
        } else {
            disc.line_to(pt);
        }
    }
    disc.close();
    if let Ok(path) = disc.build() {
        win.paint_path(path, fill_c);
    }
    if ring_w > 0.1 {
        let mut ring = PathBuilder::stroke(px(ring_w));
        for i in 0..=steps {
            let a = (i as f32 / steps as f32) * TAU;
            let pt = point(
                center.x + px(radius * a.cos()),
                center.y + px(radius * a.sin()),
            );
            if i == 0 {
                ring.move_to(pt);
            } else {
                ring.line_to(pt);
            }
        }
        if let Ok(path) = ring.build() {
            win.paint_path(path, ring_c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DiffEdit, DiffTimeline, aggregate_visible_files, compute_y_scale_max, default_pr_edits,
    };
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn scale_stays_constant_and_files_aggregate_only_up_to_elapsed() {
        let edits = default_pr_edits();
        // Total added across all 7 default edits = 384 -> scale ceiling = 400
        assert_eq!(compute_y_scale_max(&edits), 400);

        let early = aggregate_visible_files(&edits, 80.0);
        assert_eq!(early.len(), 3);
        assert_eq!(early[0].file.as_ref(), "components/hero-banner.tsx");
        assert_eq!(early[2].file.as_ref(), "components/pricing-table.tsx");
        assert_eq!(early[2].added, 88);

        // At 180s, second edit to `components/pricing-table.tsx` merges into the existing row
        let later = aggregate_visible_files(&edits, 180.0);
        assert_eq!(later.len(), 5);
        assert_eq!(later[2].added, 88 + 56);
        assert_eq!(later[2].edit_indices.len(), 2);
    }

    #[gpui::test]
    fn diff_timeline_steps_edits_and_replays(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| {
            DiffTimeline::new("test-diff-timeline", cx).with_edits(vec![
                DiffEdit::new(30, "src/a.ts", 40, 5),
                DiffEdit::new(90, "src/b.ts", 22, 10),
            ])
        });

        view.update(cx, |v, cx| {
            v.set_inspected_edit(Some(1), cx);
            assert_eq!(v.inspected_edit, Some(1));
            v.replay(cx);
            assert!(v.replay_from_zero);
            assert_eq!(v.inspected_edit, None);
        });
    }
}
