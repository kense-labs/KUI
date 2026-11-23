//! Connected execution pipeline stepper paired with a docked step log console.

use crate::{
    foundation::{Theme, surface_radius},
    primitives::{
        control::UiLanguage,
        family::Paint,
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};

/// Lifecycle state of an execution stage in [`StepConsole`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StepStatus {
    #[default]
    Pending,
    Running,
    Succeeded,
    Warning,
    Failed,
    Skipped,
}

impl StepStatus {
    fn label(self, lang: UiLanguage) -> &'static str {
        match self {
            Self::Pending => lang.text("Pending", "等待中"),
            Self::Running => lang.text("Running", "运行中"),
            Self::Succeeded => lang.text("Succeeded", "已完成"),
            Self::Warning => lang.text("Warning", "警告"),
            Self::Failed => lang.text("Failed", "失败"),
            Self::Skipped => lang.text("Skipped", "已跳过"),
        }
    }

    fn is_active(self) -> bool {
        matches!(self, Self::Running)
    }

    fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// Severity level for a log entry displayed inside [`StepConsole`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StepLogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl StepLogLevel {
    fn tag(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// Single stage in an execution pipeline.
#[derive(Clone, Debug, PartialEq)]
pub struct StepStage {
    pub id: SharedString,
    pub label: SharedString,
    pub status: StepStatus,
    pub offset_ms: Option<f64>,
    pub duration_ms: Option<f64>,
    pub summary: SharedString,
    pub detail: SharedString,
    pub metrics: Vec<(SharedString, SharedString)>,
}

impl StepStage {
    /// Create a pipeline stage with an identifier, display label, and status.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        status: StepStatus,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            status,
            offset_ms: None,
            duration_ms: None,
            summary: SharedString::default(),
            detail: SharedString::default(),
            metrics: Vec::new(),
        }
    }

    /// Attach a relative start offset in milliseconds.
    pub fn with_offset_ms(mut self, offset_ms: f64) -> Self {
        if offset_ms.is_finite() {
            self.offset_ms = Some(offset_ms.max(0.0));
        }
        self
    }

    /// Attach an observed duration in milliseconds.
    pub fn with_duration_ms(mut self, duration_ms: f64) -> Self {
        if duration_ms.is_finite() {
            self.duration_ms = Some(duration_ms.max(0.0));
        }
        self
    }

    /// Set a concise single-line summary shown in the console header.
    pub fn with_summary(mut self, summary: impl Into<SharedString>) -> Self {
        self.summary = summary.into();
        self
    }

    /// Set an evidence or diagnostic note displayed above the step logs.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Append a labeled sub-step or metric chip shown for this stage.
    pub fn with_metric(
        mut self,
        name: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        self.metrics.push((name.into(), value.into()));
        self
    }
}

/// Timestamped console log entry optionally associated with a stage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepLogLine {
    pub id: SharedString,
    pub stage_id: Option<SharedString>,
    pub timestamp: SharedString,
    pub level: StepLogLevel,
    pub source: SharedString,
    pub message: SharedString,
}

impl StepLogLine {
    /// Create a log line for [`StepConsole`].
    pub fn new(
        id: impl Into<SharedString>,
        timestamp: impl Into<SharedString>,
        level: StepLogLevel,
        source: impl Into<SharedString>,
        message: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            stage_id: None,
            timestamp: timestamp.into(),
            level,
            source: source.into(),
            message: message.into(),
        }
    }

    /// Bind this log line to a stage identifier.
    pub fn with_stage(mut self, stage_id: impl Into<SharedString>) -> Self {
        self.stage_id = Some(stage_id.into());
        self
    }
}

/// Emitted when the user selects a pipeline stage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepSelected(pub SharedString);

/// Split workbench pairing a vertical execution stepper with a live step log console.
pub struct StepConsole {
    id: ElementId,
    title: SharedString,
    stages: Vec<StepStage>,
    logs: Vec<StepLogLine>,
    selected_stage: Option<SharedString>,
    auto_follow: bool,
    show_all_logs: bool,
    errors_only: bool,
    stage_scroll: ScrollHandle,
    log_scroll: ScrollHandle,
}

impl EventEmitter<StepSelected> for StepConsole {}

impl StepConsole {
    /// Create a pipeline console with initial stages and log lines.
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        stages: Vec<StepStage>,
        logs: Vec<StepLogLine>,
    ) -> Self {
        let selected_stage = Self::pick_follow_stage(&stages);
        Self {
            id: id.into(),
            title: title.into(),
            stages,
            logs,
            selected_stage,
            auto_follow: true,
            show_all_logs: false,
            errors_only: false,
            stage_scroll: ScrollHandle::new(),
            log_scroll: ScrollHandle::new(),
        }
    }

    /// Configure whether stage selection tracks the latest active or failed stage.
    pub fn with_auto_follow(mut self, enabled: bool) -> Self {
        self.auto_follow = enabled;
        self
    }

    /// Replace the pipeline stages and advance auto-follow selection when enabled.
    pub fn set_stages(&mut self, stages: Vec<StepStage>, cx: &mut Context<Self>) {
        self.stages = stages;
        if self.auto_follow
            || self
                .selected_stage
                .as_ref()
                .is_none_or(|id| !self.stages.iter().any(|s| &s.id == id))
        {
            self.selected_stage = Self::pick_follow_stage(&self.stages);
        }
        cx.notify();
    }

    /// Replace all console log entries.
    pub fn set_logs(&mut self, logs: Vec<StepLogLine>, cx: &mut Context<Self>) {
        self.logs = logs;
        cx.notify();
    }

    /// Append a single live log line to the console stream.
    pub fn push_log(&mut self, entry: StepLogLine, cx: &mut Context<Self>) {
        self.logs.push(entry);
        cx.notify();
    }

    /// Programmatically select a stage by identifier.
    pub fn select_stage(&mut self, stage_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let next = stage_id.into();
        if self.stages.iter().any(|s| s.id == next) {
            self.selected_stage = Some(next.clone());
            self.auto_follow = false;
            cx.emit(StepSelected(next));
            cx.notify();
        }
    }

    /// Return the currently selected stage identifier, if any.
    pub fn selected_stage_id(&self) -> Option<&str> {
        self.selected_stage.as_deref()
    }

    /// Return the visible logs under the current stage and severity filters.
    pub fn visible_logs(&self) -> Vec<&StepLogLine> {
        let selected = self.selected_stage.as_deref();
        self.logs
            .iter()
            .filter(|line| {
                let stage_ok = self.show_all_logs
                    || selected.is_none()
                    || line.stage_id.as_deref() == selected;
                let level_ok = !self.errors_only || line.level == StepLogLevel::Error;
                stage_ok && level_ok
            })
            .collect()
    }

    fn pick_follow_stage(stages: &[StepStage]) -> Option<SharedString> {
        stages
            .iter()
            .find(|s| s.status.is_failed())
            .or_else(|| stages.iter().rev().find(|s| s.status.is_active()))
            .or_else(|| {
                stages
                    .iter()
                    .rev()
                    .find(|s| s.status != StepStatus::Pending)
            })
            .or_else(|| stages.first())
            .map(|s| s.id.clone())
    }

    fn status_color(status: StepStatus, cx: &App) -> Hsla {
        let c = &Theme::global(cx).tokens.colors;
        match status {
            StepStatus::Succeeded => c.success,
            StepStatus::Running => c.accent,
            StepStatus::Warning => c.warning,
            StepStatus::Failed => c.danger,
            StepStatus::Pending | StepStatus::Skipped => c.text_muted,
        }
    }

    fn level_color(level: StepLogLevel, cx: &App) -> Hsla {
        let c = &Theme::global(cx).tokens.colors;
        match level {
            StepLogLevel::Error => c.danger,
            StepLogLevel::Warn => c.warning,
            StepLogLevel::Info => c.accent,
            StepLogLevel::Debug => c.text_muted,
        }
    }

    fn format_ms(val: Option<f64>) -> String {
        match val {
            Some(ms) if ms.is_finite() => {
                if ms >= 1000.0 {
                    format!("{:.2}s", ms / 1000.0)
                } else {
                    format!("{ms:.0}ms")
                }
            }
            _ => "—".to_owned(),
        }
    }
}

impl Render for StepConsole {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).clone();
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let radius = surface_radius(cx);

        let selected_stage = self
            .selected_stage
            .as_deref()
            .and_then(|id| self.stages.iter().find(|s| s.id.as_ref() == id))
            .or_else(|| self.stages.first())
            .cloned();

        let max_ms = self
            .stages
            .iter()
            .filter_map(|s| s.duration_ms)
            .fold(1.0_f64, f64::max);

        let mut stepper_list = div()
            .id((self.id.clone(), "stages"))
            .flex()
            .flex_col()
            .gap_1p5()
            .overflow_y_scroll()
            .track_scroll(&self.stage_scroll)
            .max_h(px(360.));

        for (idx, stage) in self.stages.iter().enumerate() {
            let sid = stage.id.clone();
            let is_selected = selected_stage.as_ref().is_some_and(|s| s.id == stage.id);
            let dot_color = Self::status_color(stage.status, cx);
            let ratio = stage
                .duration_ms
                .map(|d| (d / max_ms).clamp(0.05, 1.0))
                .unwrap_or(0.0);
            let stage_logs = self
                .logs
                .iter()
                .filter(|l| l.stage_id.as_ref() == Some(&stage.id))
                .count();
            let stage_errs = self
                .logs
                .iter()
                .filter(|l| {
                    l.stage_id.as_ref() == Some(&stage.id) && l.level == StepLogLevel::Error
                })
                .count();

            stepper_list = stepper_list.child(
                div()
                    .id((self.id.clone(), SharedString::from(format!("stage-{idx}"))))
                    .cursor_pointer()
                    .p_2p5()
                    .rounded(radius)
                    .border_1()
                    .border_color(if is_selected {
                        t.tokens.colors.accent
                    } else {
                        t.tokens.colors.border
                    })
                    .bg(if is_selected {
                        t.tokens.colors.accent_subtle
                    } else {
                        t.tokens.colors.surface
                    })
                    .hover(|s| s.border_color(t.tokens.colors.border_strong))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_stage(sid.clone(), cx);
                    }))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().size(px(8.)).rounded_full().bg(dot_color))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(t.tokens.colors.foreground)
                                            .child(stage.label.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(t.tokens.colors.foreground)
                                    .child(Self::format_ms(stage.duration_ms)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(11.))
                            .text_color(t.tokens.colors.text_secondary)
                            .child(format!(
                                "+{} · {} · {} {}",
                                Self::format_ms(stage.offset_ms),
                                stage.status.label(lang),
                                stage_logs,
                                lang.text("logs", "日志")
                            ))
                            .when(stage_errs > 0, |d| {
                                d.child(
                                    div()
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(t.tokens.colors.danger)
                                        .child(format!("{stage_errs} err")),
                                )
                            }),
                    )
                    .when(ratio > 0.0, |d| {
                        d.child(
                            div()
                                .h(px(3.))
                                .w_full()
                                .rounded_full()
                                .bg(t.tokens.colors.border)
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative(ratio as f32))
                                        .rounded_full()
                                        .bg(dot_color),
                                ),
                        )
                    }),
            );
        }

        let visible_logs: Vec<StepLogLine> = self.visible_logs().into_iter().cloned().collect();
        let mut log_rows = div()
            .id((self.id.clone(), "console"))
            .flex()
            .flex_col()
            .gap(px(3.))
            .p_2p5()
            .rounded(radius)
            .bg(t.tokens.colors.surface_muted)
            .border_1()
            .border_color(t.tokens.colors.border)
            .overflow_y_scroll()
            .track_scroll(&self.log_scroll)
            .min_h(px(220.))
            .max_h(px(320.));

        if visible_logs.is_empty() {
            log_rows = log_rows.child(
                div()
                    .p_4()
                    .text_size(px(12.))
                    .text_color(t.tokens.colors.text_muted)
                    .child(lang.text(
                        "No logs match the current stage filter.",
                        "当前步骤暂无匹配日志。",
                    )),
            );
        } else {
            for line in visible_logs {
                let lc = Self::level_color(line.level, cx);
                log_rows = log_rows.child(
                    div()
                        .flex()
                        .items_start()
                        .gap_2()
                        .py(px(2.))
                        .text_size(px(12.))
                        .child(
                            div()
                                .flex_none()
                                .w(px(72.))
                                .text_color(t.tokens.colors.text_muted)
                                .child(line.timestamp),
                        )
                        .child(
                            div()
                                .flex_none()
                                .w(px(42.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(lc)
                                .child(line.level.tag()),
                        )
                        .child(
                            div()
                                .flex_none()
                                .w(px(96.))
                                .truncate()
                                .text_color(t.tokens.colors.text_secondary)
                                .child(line.source),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_color(t.tokens.colors.foreground)
                                .child(line.message),
                        ),
                );
            }
        }

        type StepToggleAction = Box<dyn Fn(&mut StepConsole, &mut Context<StepConsole>)>;
        let toggle_chip = |id: (&'static str, usize),
                           label: &'static str,
                           active: bool,
                           on_click: StepToggleAction| {
            div()
                .id((self.id.clone(), id.0))
                .cursor_pointer()
                .px_2p5()
                .py_1()
                .rounded(radius)
                .border_1()
                .border_color(if active {
                    t.tokens.colors.accent
                } else {
                    t.tokens.colors.border
                })
                .bg(if active {
                    t.tokens.colors.accent
                } else {
                    t.tokens.colors.surface
                })
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if active {
                    t.tokens.colors.accent_foreground
                } else {
                    t.tokens.colors.text_secondary
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    on_click(this, cx);
                    cx.notify();
                }))
                .child(label)
        };

        div()
            .id(self.id.clone())
            .w_full()
            .max_w(px(920.))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .rounded(radius)
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(icon(Glyph::Rocket, 15., t.tokens.colors.foreground))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(t.tokens.colors.foreground)
                                    .child(self.title.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(toggle_chip(
                                ("follow", 0),
                                lang.text("Auto-follow", "自动跟随"),
                                self.auto_follow,
                                Box::new(|this, _| {
                                    this.auto_follow = !this.auto_follow;
                                    if this.auto_follow {
                                        this.selected_stage = Self::pick_follow_stage(&this.stages);
                                    }
                                }),
                            ))
                            .child(toggle_chip(
                                ("scope", 1),
                                if self.show_all_logs {
                                    lang.text("All logs", "全链路")
                                } else {
                                    lang.text("Current step", "当前步骤")
                                },
                                self.show_all_logs,
                                Box::new(|this, _| {
                                    this.show_all_logs = !this.show_all_logs;
                                }),
                            ))
                            .child(toggle_chip(
                                ("errors", 2),
                                lang.text("Errors", "仅错误"),
                                self.errors_only,
                                Box::new(|this, _| {
                                    this.errors_only = !this.errors_only;
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .items_start()
                    .child(div().w(px(280.)).flex_none().child(stepper_list))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(320.))
                            .flex()
                            .flex_col()
                            .gap_2()
                            .when_some(selected_stage, |d, st| {
                                let mut metrics_row = div().flex().flex_wrap().gap_1p5();
                                for (k, v) in st.metrics {
                                    metrics_row = metrics_row.child(
                                        div()
                                            .px_2()
                                            .py(px(3.))
                                            .rounded(radius)
                                            .bg(t.tokens.colors.surface_muted)
                                            .border_1()
                                            .border_color(t.tokens.colors.border)
                                            .flex()
                                            .items_center()
                                            .gap_1p5()
                                            .text_size(px(11.))
                                            .child(
                                                div()
                                                    .text_color(t.tokens.colors.text_secondary)
                                                    .child(k),
                                            )
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(t.tokens.colors.accent)
                                                    .child(v),
                                            ),
                                    );
                                }
                                d.child(
                                    div()
                                        .p_2p5()
                                        .rounded(radius)
                                        .bg(t.tokens.colors.surface_muted)
                                        .border_1()
                                        .border_color(t.tokens.colors.border)
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(t.tokens.colors.foreground)
                                                        .child(st.label),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(Self::status_color(
                                                            st.status, cx,
                                                        ))
                                                        .child(st.status.label(lang)),
                                                ),
                                        )
                                        .when(!st.summary.is_empty(), |box_d| {
                                            box_d.child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(t.tokens.colors.text_secondary)
                                                    .child(st.summary),
                                            )
                                        })
                                        .when(!st.detail.is_empty(), |box_d| {
                                            box_d.child(
                                                div()
                                                    .text_size(px(11.))
                                                    .text_color(t.tokens.colors.text_muted)
                                                    .child(st.detail),
                                            )
                                        }),
                                )
                                .child(metrics_row)
                            })
                            .child(log_rows),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{StepConsole, StepLogLevel, StepLogLine, StepStage, StepStatus};

    #[test]
    fn follow_stage_prefers_failed_then_running_then_latest_completed() {
        let running = vec![
            StepStage::new("s1", "Build", StepStatus::Succeeded),
            StepStage::new("s2", "Boot", StepStatus::Running),
            StepStage::new("s3", "Verify", StepStatus::Pending),
        ];
        assert_eq!(
            StepConsole::pick_follow_stage(&running).as_deref(),
            Some("s2")
        );

        let failed = vec![
            StepStage::new("s1", "Build", StepStatus::Failed),
            StepStage::new("s2", "Boot", StepStatus::Running),
        ];
        assert_eq!(
            StepConsole::pick_follow_stage(&failed).as_deref(),
            Some("s1")
        );
    }

    #[test]
    fn visible_logs_respects_stage_and_error_filters() {
        let stages = vec![
            StepStage::new("a", "A", StepStatus::Succeeded),
            StepStage::new("b", "B", StepStatus::Running),
        ];
        let logs = vec![
            StepLogLine::new("1", "10:00:01", StepLogLevel::Info, "cli", "ok").with_stage("a"),
            StepLogLine::new("2", "10:00:02", StepLogLevel::Info, "vmm", "boot").with_stage("b"),
            StepLogLine::new("3", "10:00:03", StepLogLevel::Error, "vmm", "boom").with_stage("b"),
        ];
        let mut console = StepConsole::new("test", "Pipeline", stages, logs);
        assert_eq!(console.selected_stage_id(), Some("b"));
        assert_eq!(console.visible_logs().len(), 2);

        console.errors_only = true;
        assert_eq!(console.visible_logs().len(), 1);
        assert_eq!(console.visible_logs()[0].id.as_ref(), "3");

        console.errors_only = false;
        console.show_all_logs = true;
        assert_eq!(console.visible_logs().len(), 3);
    }
}
