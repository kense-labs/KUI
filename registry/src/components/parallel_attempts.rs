//! Best-of-N parallel agent attempts racing with live progress bars, judge scores, and diff inspection.
//!
//! Displays multiple simultaneous model/strategy attempts for one task, crowns and auto-selects
//! the highest-scoring finished attempt, and lets users inspect unified diffs or switch the winner.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_check_draw, pro_eid, pro_empty_state, pro_fold,
        pro_mix_color, pro_rolling_number, pro_shimmer_text, pro_skeleton_bar, pro_spring_bar,
        pro_spring_f32, pro_swap_label, pro_tactile, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, person_initials, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use web_time::Instant;

const PANEL_MAX_W: f32 = 756.0;
const ROW_H: f32 = 62.0;

const PARALLEL_MARK_SVG: &str = r#"<rect x="1.5" y="2.5" width="9" height="2.4" rx="1.2" fill="currentColor"></rect><rect x="1.5" y="6.8" width="12" height="2.4" rx="1.2" fill="currentColor" opacity="0.55"></rect><rect x="1.5" y="11.1" width="6" height="2.4" rx="1.2" fill="currentColor" opacity="0.3"></rect>"#;

/// Execution status of one [`ParallelAttempt`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttemptStatus {
    /// Actively executing with a growing spring bar and sweeping shine.
    #[default]
    Running,
    /// Completed and evaluated with an optional `0..=100` judge score.
    Done,
    /// Stopped early due to test failure or runtime error.
    Failed,
}

/// One candidate attempt in [`ParallelAttempts`].
#[derive(Clone, Debug, PartialEq)]
pub struct ParallelAttempt {
    /// Stable attempt identifier.
    pub id: SharedString,
    /// Model or agent display name (for example `"Claude Opus 4.6"`).
    pub name: SharedString,
    /// Strategy summary shown beside the model name.
    pub approach: SharedString,
    /// Optional Chinese strategy summary.
    pub approach_zh: Option<SharedString>,
    /// Provider artwork key passed to [`pro_app_artwork`] (`"claude"`, `"openai"`, `"gemini"`, `"deepseek"`).
    pub icon: Option<SharedString>,
    /// Current status (`Running`, `Done`, or `Failed`).
    pub status: AttemptStatus,
    /// Completion ratio in `0.0..=1.0` (retained where a `Failed` attempt stopped).
    pub progress: f32,
    /// Live activity status shown while `Running`.
    pub step: Option<SharedString>,
    /// Optional Chinese live activity status.
    pub step_zh: Option<SharedString>,
    /// Judge or eval score (`0..=100`) once `Done`; highest score earns `Best`.
    pub score: Option<u32>,
    /// One-line outcome summary or failure reason once finished.
    pub note: Option<SharedString>,
    /// Optional Chinese outcome summary or failure reason.
    pub note_zh: Option<SharedString>,
    /// Elapsed milliseconds taken by this attempt.
    pub took_ms: Option<u64>,
    /// Incremental attempt cost in USD.
    pub cost: Option<f64>,
    /// Target file path shown in the diff/output inspector.
    pub diff_file: Option<SharedString>,
    /// Unified diff or output lines shown when this attempt is inspected.
    pub diff_lines: Vec<SharedString>,
}

impl ParallelAttempt {
    /// Create a [`ParallelAttempt`] with `id`, `name`, and `approach`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        approach: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            approach: approach.into(),
            approach_zh: None,
            icon: None,
            status: AttemptStatus::Running,
            progress: 0.0,
            step: None,
            step_zh: None,
            score: None,
            note: None,
            note_zh: None,
            took_ms: None,
            cost: None,
            diff_file: None,
            diff_lines: Vec::new(),
        }
    }

    /// Provide a localized Chinese approach label.
    pub fn with_approach_zh(mut self, approach_zh: impl Into<SharedString>) -> Self {
        self.approach_zh = Some(approach_zh.into());
        self
    }

    /// Set the provider logo key (`"claude"`, `"openai"`, `"gemini"`, `"deepseek"`, `"mistral"`, etc.).
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the attempt status (`Running`, `Done`, or `Failed`).
    pub fn with_status(mut self, status: AttemptStatus) -> Self {
        self.status = status;
        if status == AttemptStatus::Done && self.progress <= 0.0 {
            self.progress = 1.0;
        }
        self
    }

    /// Set the progress fraction in `0.0..=1.0`.
    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = progress.clamp(0.0, 1.0);
        self
    }

    /// Set the in-flight step description shown while `Running`.
    pub fn with_step(mut self, step: impl Into<SharedString>) -> Self {
        self.step = Some(step.into());
        self
    }

    /// Provide a localized Chinese in-flight step description.
    pub fn with_step_zh(mut self, step_zh: impl Into<SharedString>) -> Self {
        self.step_zh = Some(step_zh.into());
        self
    }

    /// Set the judge score (`0..=100`) used to crown `Best`.
    pub fn with_score(mut self, score: u32) -> Self {
        self.score = Some(score.min(100));
        self
    }

    /// Set the outcome note or error line.
    pub fn with_note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Provide a localized Chinese outcome note or error line.
    pub fn with_note_zh(mut self, note_zh: impl Into<SharedString>) -> Self {
        self.note_zh = Some(note_zh.into());
        self
    }

    /// Set the runtime duration in milliseconds.
    pub fn with_took_ms(mut self, took_ms: u64) -> Self {
        self.took_ms = Some(took_ms);
        self
    }

    /// Set the USD cost of this attempt.
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Attach inspectable diff lines for `file`.
    pub fn with_diff(
        mut self,
        file: impl Into<SharedString>,
        lines: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.diff_file = Some(file.into());
        self.diff_lines = lines.into_iter().map(Into::into).collect();
        self
    }

    fn display_approach<'a>(&'a self, cx: &App) -> &'a str {
        match &self.approach_zh {
            Some(zh) => tr(cx, self.approach.as_ref(), zh.as_ref()),
            None => self.approach.as_ref(),
        }
    }

    fn display_step<'a>(&'a self, cx: &App) -> Option<&'a str> {
        let en = self.step.as_deref()?;
        Some(match &self.step_zh {
            Some(zh) => tr(cx, en, zh.as_ref()),
            None => en,
        })
    }

    fn display_note<'a>(&'a self, cx: &App) -> Option<&'a str> {
        let en = self.note.as_deref()?;
        Some(match &self.note_zh {
            Some(zh) => tr(cx, en, zh.as_ref()),
            None => en,
        })
    }
}

/// Find the highest-scoring `Done` attempt id (ties resolved by earlier order).
pub fn find_best_attempt_id(attempts: &[ParallelAttempt]) -> Option<SharedString> {
    let mut best: Option<(u32, &SharedString)> = None;
    for a in attempts {
        if a.status == AttemptStatus::Done {
            let sc = a.score.unwrap_or(0);
            if best.is_none_or(|(cur, _)| sc > cur) {
                best = Some((sc, &a.id));
            }
        }
    }
    best.map(|(_, id)| id.clone())
}

/// Typed events emitted by [`ParallelAttempts`].
#[derive(Clone, Debug, PartialEq)]
pub enum ParallelAttemptsEvent {
    /// User switched the chosen winning attempt.
    Pick(ParallelAttempt),
    /// User clicked an attempt row to inspect its diff/output.
    Inspect(ParallelAttempt),
    /// User re-triggered the live parallel race.
    RaceRestarted,
}

impl EventEmitter<ParallelAttemptsEvent> for ParallelAttempts {}

/// Best-of-N parallel attempt racer with live spring bars, automatic winner crowning, and diff inspector.
pub struct ParallelAttempts {
    id: ElementId,
    title: Option<SharedString>,
    title_zh: Option<SharedString>,
    task: SharedString,
    task_zh: SharedString,
    attempts: Vec<ParallelAttempt>,
    final_targets: Vec<ParallelAttempt>,
    picked_override: Option<SharedString>,
    inspected_id: Option<SharedString>,
    last_diff_attempt: Option<ParallelAttempt>,
    hovered_id: Option<SharedString>,
    race_started: Option<Instant>,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl ParallelAttempts {
    /// Create a [`ParallelAttempts`] card preloaded with a 4-model race on `"Fix the flaky checkout test"`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let targets = default_checkout_attempts();
        Self {
            id: id.into(),
            title: None,
            title_zh: None,
            task: SharedString::from("Fix the flaky checkout test"),
            task_zh: SharedString::from("修复结账页偶发失败测试"),
            attempts: targets.clone(),
            final_targets: targets,
            picked_override: None,
            inspected_id: None,
            last_diff_attempt: None,
            hovered_id: None,
            race_started: None,
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Override the heading title (`"Best of N"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Provide a localized Chinese heading override.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Set the shared task summary displayed beside the heading.
    pub fn with_task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = task.into();
        self
    }

    /// Provide a localized Chinese task summary.
    pub fn with_task_zh(mut self, task_zh: impl Into<SharedString>) -> Self {
        self.task_zh = task_zh.into();
        self
    }

    /// Supply the parallel attempts in display order.
    pub fn with_attempts(mut self, attempts: Vec<ParallelAttempt>) -> Self {
        self.final_targets = attempts.clone();
        self.attempts = attempts;
        self.race_started = None;
        self
    }

    /// Pre-select a specific winning attempt id (otherwise the highest-scoring `Done` attempt wins).
    pub fn with_picked(mut self, picked: Option<SharedString>) -> Self {
        self.picked_override = picked;
        self
    }

    /// Start immediately in a live animated race that settles onto the final judge scores.
    pub fn with_live_race(mut self, active: bool) -> Self {
        self.race_started = active.then(Instant::now);
        self
    }

    /// Toggle skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state placeholder.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the current slice of attempts.
    pub fn attempts(&self) -> &[ParallelAttempt] {
        &self.attempts
    }

    /// Update the attempts slice in place.
    pub fn set_attempts(&mut self, attempts: Vec<ParallelAttempt>, cx: &mut Context<Self>) {
        self.final_targets = attempts.clone();
        self.attempts = attempts;
        self.race_started = None;
        cx.notify();
    }

    /// Return the id of the highest-scoring `Done` attempt (`Best`).
    pub fn best_attempt_id(&self) -> Option<SharedString> {
        find_best_attempt_id(&self.attempts)
    }

    /// Return the currently active winning attempt id (explicit pick or auto-crowned `Best`).
    pub fn effective_picked_id(&self) -> Option<SharedString> {
        if let Some(ov) = &self.picked_override
            && self
                .attempts
                .iter()
                .any(|a| &a.id == ov && a.status == AttemptStatus::Done)
        {
            return Some(ov.clone());
        }
        self.best_attempt_id()
    }

    /// Return the currently chosen winning attempt, if any.
    pub fn picked_attempt(&self) -> Option<&ParallelAttempt> {
        let id = self.effective_picked_id()?;
        self.attempts.iter().find(|a| a.id == id)
    }

    /// Return the attempt currently shown in the diff/output inspector.
    pub fn inspected_attempt(&self) -> Option<&ParallelAttempt> {
        let target = self
            .inspected_id
            .clone()
            .or_else(|| self.effective_picked_id())
            .or_else(|| self.attempts.first().map(|a| a.id.clone()))?;
        self.attempts.iter().find(|a| a.id == target)
    }

    /// Switch the chosen winner to `id` (must be `Done`), focus its diff, and emit [`ParallelAttemptsEvent::Pick`].
    pub fn pick_attempt(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let Some(found) = self.attempts.iter().find(|a| a.id.as_ref() == id).cloned() else {
            return false;
        };
        self.inspected_id = Some(found.id.clone());
        cx.emit(ParallelAttemptsEvent::Inspect(found.clone()));
        if found.status == AttemptStatus::Done {
            self.picked_override = Some(found.id.clone());
            cx.emit(ParallelAttemptsEvent::Pick(found));
            cx.notify();
            true
        } else {
            cx.notify();
            false
        }
    }

    /// Replay the parallel race from `0%` across all configured attempts.
    pub fn start_race(&mut self, cx: &mut Context<Self>) {
        if self.final_targets.is_empty() {
            return;
        }
        self.picked_override = None;
        self.race_started = Some(Instant::now());
        cx.emit(ParallelAttemptsEvent::RaceRestarted);
        cx.notify();
    }

    fn advance_live_race(&mut self, window: &mut Window, reduced: bool) {
        let Some(started) = self.race_started else {
            return;
        };
        let elapsed = started.elapsed().as_secs_f32();
        if reduced || elapsed >= 2.85 {
            self.attempts = self.final_targets.clone();
            self.race_started = None;
            return;
        }
        window.request_animation_frame();
        let durations = [2.15_f32, 1.75, 2.65, 1.45];
        for (idx, target) in self.final_targets.iter().enumerate() {
            let dur = durations[idx % durations.len()];
            let Some(slot) = self.attempts.get_mut(idx) else {
                continue;
            };
            if elapsed >= dur {
                *slot = target.clone();
            } else {
                let frac = (elapsed / dur).clamp(0.04, 0.98);
                let end_p = if target.status == AttemptStatus::Failed {
                    target.progress.max(0.45)
                } else {
                    1.0
                };
                slot.status = AttemptStatus::Running;
                slot.progress = (frac * end_p).clamp(0.05, 0.96);
                slot.score = None;
                let step_n = ((frac * 20.0).ceil() as usize).clamp(1, 19);
                slot.step = Some(SharedString::from(format!("Running tests {step_n}/20")));
                slot.step_zh = Some(SharedString::from(format!("正在运行测试 {step_n}/20")));
                if let Some(c) = target.cost {
                    slot.cost = Some((c * frac as f64).max(0.02));
                }
            }
        }
    }
}

fn model_tile(icon: Option<&str>, fallback_name: &str, p: &ProPalette, cx: &App) -> AnyElement {
    let tile_bg: Hsla = if p.dark { rgb(0x232326).into() } else { p.card };
    let key = icon.unwrap_or(fallback_name);
    let inner = if let Some(art) = pro_app_artwork(key, 15.0, cx) {
        art
    } else {
        let init = person_initials(fallback_name);
        div()
            .text_size(px(11.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink)
            .child(if init.is_empty() {
                "M".to_string()
            } else {
                init
            })
            .into_any_element()
    };
    div()
        .size(px(28.))
        .flex_none()
        .rounded(px(7.))
        .bg(tile_bg)
        .border_1()
        .border_color(p.line_strong)
        .flex()
        .items_center()
        .justify_center()
        .child(inner)
        .into_any_element()
}

fn format_took(ms: u64) -> String {
    if ms < 1_000 {
        format!("{ms}ms")
    } else {
        let s = ms as f64 / 1_000.0;
        if s >= 60.0 {
            let mins = (s / 60.0).floor() as u64;
            let rem = (s % 60.0).round() as u64;
            format!("{mins}m {rem:02}s")
        } else {
            format!("{s:.0}s")
        }
    }
}

impl Render for ParallelAttempts {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        self.advance_live_race(window, reduced);

        let elapsed_s = self.started_at.elapsed().as_secs_f32();
        let panel_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            p.panel
        };
        let panel_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.06)
        };

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_s / 1.4).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(PANEL_MAX_W))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(pro_skeleton_bar(240., 16., 4., phase, cx))
                .child(pro_skeleton_bar(PANEL_MAX_W, ROW_H, 10., phase + 0.1, cx))
                .child(pro_skeleton_bar(PANEL_MAX_W, ROW_H, 10., phase + 0.2, cx))
                .child(pro_skeleton_bar(PANEL_MAX_W, ROW_H, 10., phase + 0.3, cx))
                .into_any_element();
        }

        if self.empty || self.attempts.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(PANEL_MAX_W))
                .rounded(px(10.))
                .bg(panel_bg)
                .border_1()
                .border_color(panel_border)
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "No parallel attempts running", "暂无并行尝试"),
                    tr(
                        cx,
                        "Launch multiple models or strategies simultaneously to compare scores and diffs.",
                        "同时发起多模型或多策略求解，自动评选最高分结果并对比代码差异。",
                    ),
                    Some((
                        pro_eid(&self.id, "load-demo"),
                        SharedString::from(tr(cx, "Run Best-of-4 demo", "启动 4 路竞速示例")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            if this.attempts.is_empty() {
                                let d = default_checkout_attempts();
                                this.final_targets = d.clone();
                                this.attempts = d;
                            }
                            this.start_race(cx);
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let any_running = self
            .attempts
            .iter()
            .any(|a| a.status == AttemptStatus::Running);
        if any_running && !reduced {
            window.request_animation_frame();
        }

        let n_total = self.attempts.len();
        let done_count = self
            .attempts
            .iter()
            .filter(|a| matches!(a.status, AttemptStatus::Done | AttemptStatus::Failed))
            .count();
        let total_cost: f64 = self.attempts.iter().filter_map(|a| a.cost).sum();

        let default_en = format!("Best of {n_total}");
        let default_zh = format!("{n_total} 路择优");
        let heading = match (&self.title, &self.title_zh) {
            (Some(en), Some(zh)) => tr(cx, en.as_ref(), zh.as_ref()).to_string(),
            (Some(en), None) => en.to_string(),
            _ => tr(cx, &default_en, &default_zh).to_string(),
        };

        let mark_c: Hsla = if p.dark {
            p.blue
        } else {
            hsla(215. / 360., 1.0, 0.44, 0.78)
        };
        let header = div()
            .w_full()
            .mb(px(12.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(pro_svg("0 0 16 16", PARALLEL_MARK_SVG, 16., mark_c))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(heading.clone()),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(tr(cx, self.task.as_ref(), self.task_zh.as_ref()).to_string()),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "done-cnt"),
                                done_count as f64,
                                0,
                                "",
                                "",
                                12.,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(
                                div().text_size(px(12.)).text_color(p.muted).child(
                                    tr(
                                        cx,
                                        &format!("of {n_total} done"),
                                        &format!("/ {n_total} 已完成"),
                                    )
                                    .to_string(),
                                ),
                            ),
                    )
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "tot-cost"),
                        total_cost,
                        2,
                        "$",
                        "",
                        12.,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "replay-race"),
                        tr(cx, "Re-run", "重跑"),
                        Some(ProIcon::Refresh),
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, _, cx| {
                            this.start_race(cx);
                        },
                    )),
            );

        let best_id = self.best_attempt_id();
        let picked_id = self.effective_picked_id();
        let inspected_id = self
            .inspected_id
            .clone()
            .or_else(|| picked_id.clone())
            .or_else(|| self.attempts.first().map(|a| a.id.clone()));

        let mut row_opacities = Vec::with_capacity(self.attempts.len());
        for attempt in &self.attempts {
            let is_picked = picked_id.as_ref() == Some(&attempt.id);
            let is_inspected = inspected_id.as_ref() == Some(&attempt.id);
            let is_hovered = self.hovered_id.as_ref() == Some(&attempt.id);
            let quiet = picked_id.is_some() && !is_picked && !is_inspected && !is_hovered;
            row_opacities.push(pro_spring_f32(
                pro_eid(&self.id, format!("pa-op-{}", attempt.id)),
                if quiet { 0.56 } else { 1.0 },
                0.22,
                window,
                cx,
            ));
        }

        let track_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.055)
        } else {
            hsla(0., 0., 0., 0.04)
        };
        let running_ink: Hsla = if p.dark {
            rgb(0x8bcbfe).into()
        } else {
            p.accent_info
        };

        let mut rows_col = div()
            .id(pro_eid(&self.id, "radiogroup"))
            .role(Role::Group)
            .aria_label(heading)
            .w_full()
            .flex()
            .flex_col()
            .gap(px(8.));

        for (idx, attempt) in self.attempts.iter().enumerate() {
            let is_best = best_id.as_ref() == Some(&attempt.id);
            let is_picked = picked_id.as_ref() == Some(&attempt.id);
            let is_inspected = inspected_id.as_ref() == Some(&attempt.id);
            let is_hovered = self.hovered_id.as_ref() == Some(&attempt.id);
            let row_op = row_opacities[idx];

            let row_bg = if is_picked {
                pro_mix_color(panel_bg, p.green, if p.dark { 0.075 } else { 0.045 })
            } else {
                panel_bg
            };
            let row_hover_bg = pro_mix_color(row_bg, p.hover, 0.85);
            let row_border = if is_picked {
                p.green.opacity(if p.dark { 0.45 } else { 0.36 })
            } else if is_inspected {
                p.blue.opacity(0.45)
            } else {
                panel_border
            };
            let row_hover_border = if is_picked {
                p.green.opacity(0.65)
            } else {
                p.line_strong
            };

            let bar_fill = match attempt.status {
                AttemptStatus::Running => p.blue.opacity(if p.dark { 0.48 } else { 0.36 }),
                AttemptStatus::Done if is_best => p.green.opacity(if p.dark { 0.56 } else { 0.46 }),
                AttemptStatus::Done => p.green.opacity(if p.dark { 0.36 } else { 0.30 }),
                AttemptStatus::Failed => p.red.opacity(if p.dark { 0.45 } else { 0.38 }),
            };

            let sub_line = match attempt.status {
                AttemptStatus::Running => {
                    let s_txt = attempt
                        .display_step(cx)
                        .unwrap_or_else(|| tr(cx, "Running…", "运行中…"));
                    pro_shimmer_text(
                        pro_eid(&self.id, format!("step-{}", attempt.id)),
                        s_txt.to_string(),
                        running_ink,
                        p.strong,
                        12.0,
                        FontWeight::NORMAL,
                        true,
                    )
                }
                AttemptStatus::Done => {
                    let n_txt = attempt
                        .display_note(cx)
                        .unwrap_or_else(|| tr(cx, "Completed cleanly", "顺利通过"));
                    div()
                        .truncate()
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.text)
                        .child(n_txt.to_string())
                        .into_any_element()
                }
                AttemptStatus::Failed => {
                    let e_txt = attempt
                        .display_note(cx)
                        .unwrap_or_else(|| tr(cx, "Attempt failed", "尝试失败"));
                    div()
                        .truncate()
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.red_ink)
                        .child(e_txt.to_string())
                        .into_any_element()
                }
            };

            let is_running = attempt.status == AttemptStatus::Running;
            let prog = attempt.progress.clamp(0.0, 1.0);
            let dark = p.dark;
            let bar_el = div()
                .relative()
                .w(px(210.))
                .flex_none()
                .h(px(8.))
                .rounded(px(4.))
                .overflow_hidden()
                .child(pro_spring_bar(
                    pro_eid(&self.id, format!("bar-{}", attempt.id)),
                    prog,
                    8.0,
                    4.0,
                    track_c,
                    bar_fill,
                ))
                .when(is_running && !reduced, |d| {
                    let sweep = ((elapsed_s + idx as f32 * 0.23) / 1.35).fract();
                    d.child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, window, _| {
                                let fill_w = f32::from(bounds.size.width) * prog;
                                if fill_w <= 6.0 {
                                    return;
                                }
                                let band = 38.0_f32.min(fill_w * 0.65);
                                let bx = -band + sweep * (fill_w + band);
                                let cl_l = bx.max(0.0);
                                let cl_r = (bx + band).min(fill_w);
                                if cl_r > cl_l + 1.0 {
                                    let r = Bounds::new(
                                        point(bounds.left() + px(cl_l), bounds.top()),
                                        size(px(cl_r - cl_l), bounds.size.height),
                                    );
                                    window.paint_quad(
                                        fill(r, hsla(0., 0., 1., if dark { 0.25 } else { 0.55 }))
                                            .corner_radii(px(4.)),
                                    );
                                }
                            },
                        )
                        .absolute()
                        .inset_0(),
                    )
                });

            let score_line = match attempt.status {
                AttemptStatus::Done => {
                    let sc = attempt.score.unwrap_or(0);
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(6.))
                        .when(is_best, |d| {
                            d.child(
                                div()
                                    .h(px(18.))
                                    .px(px(6.))
                                    .rounded(px(4.))
                                    .bg(p.green_soft)
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.green_ink)
                                    .flex()
                                    .items_center()
                                    .child(tr(cx, "Best", "最佳")),
                            )
                        })
                        .child(pro_rolling_number(
                            pro_eid(&self.id, format!("sc-{}", attempt.id)),
                            sc as f64,
                            0,
                            "",
                            "",
                            13.,
                            FontWeight::SEMIBOLD,
                            p.ink,
                        ))
                        .into_any_element()
                }
                AttemptStatus::Running => pro_rolling_number(
                    pro_eid(&self.id, format!("pct-{}", attempt.id)),
                    (prog * 100.0) as f64,
                    0,
                    "",
                    " %",
                    13.,
                    FontWeight::NORMAL,
                    p.muted,
                ),
                AttemptStatus::Failed => div()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.red_ink)
                    .child(tr(cx, "Failed", "失败"))
                    .into_any_element(),
            };

            let mut meta_parts = Vec::new();
            if let Some(ms) = attempt.took_ms
                && attempt.status != AttemptStatus::Running
            {
                meta_parts.push(format_took(ms));
            }
            if let Some(cost) = attempt.cost {
                meta_parts.push(format!("${cost:.2}"));
            }
            let meta_text = meta_parts.join(" · ");

            let bottom_right = if is_picked {
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(5.))
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("pick-chk-{}", attempt.id)),
                        true,
                        13.,
                        false,
                        p.green,
                    ))
                    .child(pro_swap_label(
                        pro_eid(&self.id, format!("pick-lbl-{}", attempt.id)),
                        tr(cx, "Using", "已采用"),
                        12.,
                        FontWeight::MEDIUM,
                        p.green_ink,
                    ))
                    .into_any_element()
            } else if is_hovered && attempt.status == AttemptStatus::Done {
                div()
                    .px(px(6.))
                    .h(px(20.))
                    .rounded(px(5.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(tr(cx, "Use this one", "选用此方案"))
                    .into_any_element()
            } else {
                div()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(meta_text)
                    .into_any_element()
            };

            let id_click = attempt.id.clone();
            let id_hover = attempt.id.clone();
            let row_aria = format!("{} — {}", attempt.name, attempt.display_approach(cx));
            let tile = model_tile(attempt.icon.as_deref(), attempt.name.as_ref(), &p, cx);

            let row_inner = div()
                .w_full()
                .min_h(px(ROW_H))
                .pl(px(12.))
                .pr(px(16.))
                .py(px(11.))
                .flex()
                .items_center()
                .gap(px(14.))
                .child(tile)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .gap(px(6.))
                                .min_w_0()
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(attempt.name.clone()),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(13.))
                                        .text_color(p.faint)
                                        .child("·"),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(13.))
                                        .text_color(p.muted)
                                        .child(attempt.display_approach(cx).to_string()),
                                ),
                        )
                        .child(sub_line),
                )
                .child(bar_el)
                .child(
                    div()
                        .w(px(110.))
                        .flex_none()
                        .flex()
                        .flex_col()
                        .items_end()
                        .gap(px(2.))
                        .child(score_line)
                        .child(bottom_right),
                );

            let row = pro_tactile(
                pro_eid(&self.id, format!("row-{}", attempt.id)),
                row_aria,
                10.,
                row_bg,
                row_hover_bg,
                row_border,
                row_hover_border,
                is_picked || is_inspected,
                row_inner,
                cx,
                move |this, _, cx| {
                    this.pick_attempt(id_click.as_ref(), cx);
                },
            )
            .role(Role::RadioButton)
            .w_full()
            .when(!p.dark, |d| d.shadow_sm())
            .opacity(row_op)
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                if *inside {
                    this.hovered_id = Some(id_hover.clone());
                } else if this.hovered_id.as_ref() == Some(&id_hover) {
                    this.hovered_id = None;
                }
                cx.notify();
            }));

            rows_col = rows_col.child(row);
        }

        // Diff / output preview for the inspected or winning attempt (with bidirectional fold)
        let active_diff_att = self
            .inspected_attempt()
            .filter(|a| !a.diff_lines.is_empty())
            .cloned();
        if let Some(att) = &active_diff_att {
            self.last_diff_attempt = Some(att.clone());
        }
        let has_diff_open = active_diff_att.is_some();
        let diff_att = active_diff_att.or_else(|| self.last_diff_attempt.clone());

        let inspector = diff_att.map(|att| {
            let is_winner = picked_id.as_ref() == Some(&att.id);
            let file = att
                .diff_file
                .as_deref()
                .unwrap_or("src/checkout.rs")
                .to_string();
            let adds = att.diff_lines.iter().filter(|l| l.starts_with('+')).count();
            let dels = att.diff_lines.iter().filter(|l| l.starts_with('-')).count();

            let mut code_col = div()
                .w_full()
                .px(px(12.))
                .py(px(8.))
                .flex()
                .flex_col()
                .gap(px(2.));
            for line in &att.diff_lines {
                let txt = line.as_ref();
                let (bg, fg) = if txt.starts_with('+') {
                    (p.green_soft.opacity(0.65), p.green_ink)
                } else if txt.starts_with('-') {
                    (p.red_soft.opacity(0.65), p.red_ink)
                } else {
                    (transparent_black(), p.text)
                };
                code_col = code_col.child(
                    div()
                        .w_full()
                        .px(px(6.))
                        .py(px(1.5))
                        .rounded(px(4.))
                        .bg(bg)
                        .font_family("Menlo")
                        .text_size(px(11.5))
                        .line_height(px(16.5))
                        .text_color(fg)
                        .truncate()
                        .child(line.clone()),
                );
            }

            let active_lbl = tr(cx, "Active patch", "当前采用补丁").to_string();
            let chk_el = pro_check_draw(
                pro_eid(&self.id, "active-patch-chk"),
                is_winner,
                12.,
                false,
                p.green,
            );
            let adds_num = pro_rolling_number(
                pro_eid(&self.id, "diff-adds"),
                adds as f64,
                0,
                "+",
                "",
                11.5,
                FontWeight::MEDIUM,
                p.green_ink,
            );
            let dels_num = pro_rolling_number(
                pro_eid(&self.id, "diff-dels"),
                dels as f64,
                0,
                "-",
                "",
                11.5,
                FontWeight::MEDIUM,
                p.red_ink,
            );
            let card = div()
                .w_full()
                .mt(px(10.))
                .rounded(px(10.))
                .bg(panel_bg)
                .border_1()
                .border_color(panel_border)
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .px(px(12.))
                        .py(px(8.))
                        .bg(p.subtle)
                        .border_b_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_icon(ProIcon::Code, 13.5, p.muted))
                        .child(
                            div()
                                .font_family("Menlo")
                                .text_size(px(12.))
                                .text_color(p.ink)
                                .child(file),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(format!("— {}", att.name)),
                        )
                        .child(
                            div()
                                .ml_auto()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(adds_num)
                                .child(dels_num)
                                .when(is_winner, |d| {
                                    d.child(
                                        div()
                                            .h(px(19.))
                                            .px(px(6.))
                                            .rounded(px(4.))
                                            .bg(p.green_soft)
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.green_ink)
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(chk_el)
                                            .child(active_lbl),
                                    )
                                }),
                        ),
                )
                .child(code_col);

            let vtx_card = pro_view_transition(pro_eid(&self.id, "diff-vtx"), att.id.clone(), card);
            pro_fold(
                pro_eid(&self.id, "diff-fold"),
                has_diff_open,
                148.0,
                0.0,
                move || vtx_card,
            )
        });

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(PANEL_MAX_W))
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(rows_col)
                    .children(inspector),
            )
            .into_any_element()
    }
}

fn default_checkout_attempts() -> Vec<ParallelAttempt> {
    vec![
        ParallelAttempt::new(
            "att-claude",
            "Claude Opus 4.6",
            "Idempotency key + jittered webhook retry",
        )
        .with_approach_zh("幂等键去重 + 带抖动 Webhook 重试")
        .with_icon("claude")
        .with_status(AttemptStatus::Done)
        .with_progress(1.0)
        .with_score(96)
        .with_took_ms(21_000)
        .with_cost(0.26)
        .with_note("20/20 stress runs green · zero duplicate charges")
        .with_note_zh("20/20 压测全绿 · 零重复扣款")
        .with_diff(
            "tests/checkout_webhook.rs",
            [
                "  let key = IdempotencyKey::from_order(order.id);",
                "- gateway.charge(&order).await?;",
                "+ retry_with_jitter(3, Duration::from_millis(80), || {",
                "+     gateway.charge_idempotent(&order, &key)",
                "+ }).await?;",
                "  assert_eq!(ledger.settled_count(order.id), 1);",
            ],
        ),
        ParallelAttempt::new(
            "att-openai",
            "GPT-5",
            "Deterministic mock clock for Stripe webhook",
        )
        .with_approach_zh("注入确定性虚拟时钟模拟回调")
        .with_icon("openai")
        .with_status(AttemptStatus::Done)
        .with_progress(1.0)
        .with_score(89)
        .with_took_ms(17_000)
        .with_cost(0.19)
        .with_note("20/20 unit runs green · skips live socket path")
        .with_note_zh("20/20 单测通过 · 绕过真实套接字路径")
        .with_diff(
            "tests/checkout_webhook.rs",
            [
                "- tokio::time::sleep(Duration::from_millis(250)).await;",
                "+ tokio::time::pause();",
                "+ tokio::time::advance(Duration::from_secs(2)).await;",
                "+ mock_stripe.flush_pending_webhooks().await;",
                "  assert!(checkout.is_confirmed());",
            ],
        ),
        ParallelAttempt::new(
            "att-gemini",
            "Gemini 2.5 Pro",
            "Wait for outbox barrier before assertion",
        )
        .with_approach_zh("断言前等待发件箱排空屏障")
        .with_icon("gemini")
        .with_status(AttemptStatus::Done)
        .with_progress(1.0)
        .with_score(84)
        .with_took_ms(24_000)
        .with_cost(0.16)
        .with_note("19/20 runs green · +40ms tail latency in CI")
        .with_note_zh("19/20 通过 · CI 尾延迟增加 40ms")
        .with_diff(
            "tests/checkout_webhook.rs",
            [
                "  checkout.submit_payment(&card).await?;",
                "+ outbox.wait_until_drained(Duration::from_secs(3)).await?;",
                "  assert_eq!(checkout.status(), OrderStatus::Paid);",
            ],
        ),
        ParallelAttempt::new(
            "att-deepseek",
            "DeepSeek R1",
            "Serialize checkout handler behind process mutex",
        )
        .with_approach_zh("使用进程级互斥锁串行化结账回调")
        .with_icon("deepseek")
        .with_status(AttemptStatus::Failed)
        .with_progress(0.62)
        .with_took_ms(14_000)
        .with_cost(0.09)
        .with_note("Deadlocked under concurrent webhook burst (test 13/20)")
        .with_note_zh("并发回调峰值下触发死锁（第 13/20 轮超时）")
        .with_diff(
            "src/checkout/handler.rs",
            [
                "- let mut tx = pool.begin().await?;",
                "+ let _guard = CHECKOUT_MUTEX.lock().await;",
                "+ let mut tx = pool.begin().await?; // blocks re-entrant ack",
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{AttemptStatus, ParallelAttempt, ParallelAttempts, find_best_attempt_id};
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn crowns_highest_scored_done_attempt_and_ignores_failed() {
        let items = vec![
            ParallelAttempt::new("a", "Model A", "Approach 1")
                .with_status(AttemptStatus::Done)
                .with_score(84),
            ParallelAttempt::new("b", "Model B", "Approach 2")
                .with_status(AttemptStatus::Done)
                .with_score(95),
            ParallelAttempt::new("c", "Model C", "Approach 3")
                .with_status(AttemptStatus::Failed)
                .with_score(99),
        ];
        assert_eq!(find_best_attempt_id(&items).as_deref(), Some("b"));
    }

    #[gpui::test]
    fn auto_crowns_winner_and_switches_on_pick(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| ParallelAttempts::new("test-pa", cx));
        view.update(cx, |v, cx| {
            assert_eq!(v.best_attempt_id().as_deref(), Some("att-claude"));
            assert_eq!(v.effective_picked_id().as_deref(), Some("att-claude"));

            // Switching to another finished attempt updates effective winner
            assert!(v.pick_attempt("att-openai", cx));
            assert_eq!(v.effective_picked_id().as_deref(), Some("att-openai"));
            assert_eq!(v.best_attempt_id().as_deref(), Some("att-claude"));

            // Clicking a failed attempt inspects its diff without crowning it winner
            assert!(!v.pick_attempt("att-deepseek", cx));
            assert_eq!(v.effective_picked_id().as_deref(), Some("att-openai"));
            assert_eq!(
                v.inspected_attempt().map(|a| a.id.as_ref()),
                Some("att-deepseek")
            );
        });
    }
}
