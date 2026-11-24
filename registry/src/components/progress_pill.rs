//! Slim floating agent progress pill with twinkling pixel mark, step shimmer, and under-glow.
//!
//! Depicts an autonomous agent assembling a multi-step report inside a capsule pill
//! that fades into the page on the right, complete with a 3×3 mirrored pixel mark,
//! tabular percentage, crossfading shimmer step subtitle, sheen progress bar, and
//! drifting blue-purple-pink atmospheric glow beneath.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_ease, pro_eid, pro_empty_state, pro_shimmer_text, pro_skeleton_bar,
        pro_spring_f32, pro_swap_label, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells},
    },
};
use gpui::{prelude::*, *};
use std::sync::{Arc, OnceLock};
use web_time::Instant;

const STEP_TICKS: [f32; 4] = [4.0, 11.0, 9.0, 6.0];

/// Default three report-building steps shown by [`ProgressPill`].
pub fn default_progress_steps(cx: &App) -> Vec<SharedString> {
    vec![
        tr(cx, "Read 24 sources", "阅读 24 份来源资料").into(),
        tr(cx, "Pull the numbers", "提取核心指标数据").into(),
        tr(cx, "Draft the report", "撰写分析报告初稿").into(),
    ]
}

/// Events emitted by [`ProgressPill`] when interactive or cycling through steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgressPillEvent {
    /// Active step changed (either via auto-cycle or user click).
    StepChanged {
        /// Zero-based step index (`steps.len()` when in the final `"Report ready"` beat).
        index: usize,
        /// Visible subtitle text for this phase.
        label: SharedString,
        /// `true` when displaying the completed `"Report ready"` beat.
        done: bool,
    },
}

impl EventEmitter<ProgressPillEvent> for ProgressPill {}

/// Evaluated visual snapshot of the pill's step & progress state at a given instant.
#[derive(Clone, Debug, PartialEq)]
struct PillSnapshot {
    step_index: usize,
    done: bool,
    ratio: f32,
}

fn evaluate_pill_snapshot(
    step_count: usize,
    initial_step: usize,
    hold_ms: u64,
    playing: bool,
    reduced_motion: bool,
    elapsed_ms: f64,
    controlled_progress: Option<f32>,
) -> PillSnapshot {
    let n = step_count.max(1);
    let clamped_init = initial_step.min(n - 1);

    if let Some(p) = controlled_progress {
        let r = p.clamp(0.0, 1.0);
        let done = r >= 0.999;
        let idx = if done {
            n - 1
        } else {
            ((r * n as f32).floor() as usize).min(n - 1)
        };
        return PillSnapshot {
            step_index: idx,
            done,
            ratio: r,
        };
    }

    if !playing || reduced_motion || hold_ms == 0 {
        // Static resting pose matching reference (`step = 1` -> ~36% on 3 steps)
        let tick_cap = STEP_TICKS[clamped_init % STEP_TICKS.len()] + 2.0;
        let sub = (1.0 / tick_cap).clamp(0.0, 0.92);
        let ratio = ((clamped_init as f32 + sub) / n as f32).min(0.97);
        return PillSnapshot {
            step_index: clamped_init,
            done: false,
            ratio,
        };
    }

    let step_dur = hold_ms.max(400) as f64;
    let done_dur = step_dur * 0.8;
    let total_cycle = step_dur * (n as f64) + done_dur;
    // Start at `clamped_init` so initial frame shows the configured step (default step 1 = 36%)
    let shifted = (elapsed_ms + (clamped_init as f64) * step_dur).rem_euclid(total_cycle);
    let active_span = step_dur * (n as f64);

    if shifted >= active_span {
        PillSnapshot {
            step_index: n - 1,
            done: true,
            ratio: 1.0,
        }
    } else {
        let idx = ((shifted / step_dur).floor() as usize).min(n - 1);
        let within_ms = (shifted - (idx as f64) * step_dur).max(0.0);
        // Discrete 250ms ticks matching reference `(b / (a[g] + 2))` starting from 1 tick (`36%`)
        let ticks = 1.0 + (within_ms / 250.0).floor() as f32;
        let cap = STEP_TICKS[idx % STEP_TICKS.len()] + 2.0;
        let sub = (ticks / cap).min(1.0);
        let ratio = ((idx as f32 + sub) / n as f32).min(0.97);
        PillSnapshot {
            step_index: idx,
            done: false,
            ratio,
        }
    }
}

fn under_glow_svg(dark: bool) -> Arc<Image> {
    static LIGHT_GLOW: OnceLock<Arc<Image>> = OnceLock::new();
    static DARK_GLOW: OnceLock<Arc<Image>> = OnceLock::new();
    let cache = if dark { &DARK_GLOW } else { &LIGHT_GLOW };
    cache
        .get_or_init(|| {
            let (blue, purple, pink) = if dark {
                ("#4fb0fe", "#b98af0", "#f06db5")
            } else {
                ("#0190fe", "#a651f0", "#ee3e9e")
            };
            let doc = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="560" height="140" viewBox="0 0 280 70" fill="none"><defs><filter id="pplg" x="-30%" y="-60%" width="160%" height="220%"><feGaussianBlur stdDeviation="14"/></filter></defs><g filter="url(#pplg)"><ellipse cx="58" cy="30" rx="46" ry="13" fill="{blue}" fill-opacity="0.72"/><ellipse cx="142" cy="28" rx="48" ry="13" fill="{purple}" fill-opacity="0.68"/><ellipse cx="222" cy="26" rx="44" ry="12" fill="{pink}" fill-opacity="0.64"/></g></svg>"##
            );
            Arc::new(Image::from_bytes(ImageFormat::Svg, doc.into_bytes()))
        })
        .clone()
}

fn render_circular_pixel_mark(
    seed: &str,
    mark_color: Hsla,
    tile_bg: Hsla,
    working: bool,
    reduced_motion: bool,
    elapsed_ms: f32,
) -> AnyElement {
    let cells = agent_pixel_cells(seed);
    let sq = 3.75f32;
    let gap = 1.9f32;
    let mut grid = div().flex().flex_col().gap(px(gap));
    for r in 0..3usize {
        let mut row_el = div().flex().items_center().gap(px(gap));
        for c in 0..3usize {
            let i = r * 3 + c;
            let on = cells[i];
            let alpha = if working && !reduced_motion {
                let phase = ((elapsed_ms + AGENT_WAVE_MS[i]) / 1200.0) * std::f32::consts::TAU;
                let wave = 0.60 + 0.40 * phase.cos();
                if on { wave } else { 0.16 * wave }
            } else if on {
                1.0
            } else {
                0.0
            };
            row_el = row_el.child(
                div()
                    .size(px(sq))
                    .rounded(px(0.95))
                    .bg(mark_color.opacity(alpha)),
            );
        }
        grid = grid.child(row_el);
    }

    div()
        .size(px(48.0))
        .flex_none()
        .rounded_full()
        .bg(tile_bg)
        .flex()
        .items_center()
        .justify_center()
        .child(grid)
        .into_any_element()
}

/// Paint the `68px` capsule card whose right `135px` tail smoothly fades to transparency.
fn paint_fading_capsule(card_bg: Hsla, border_col: Hsla, shadow_col: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let bw = f32::from(bounds.size.width);
            let bh = f32::from(bounds.size.height);
            if bw <= 40.0 || bh <= 10.0 {
                return;
            }
            let r = bh * 0.5;
            let fade_w = 135.0f32.min(bw * 0.35);
            let solid_w = (bw - fade_w).max(r + 8.0);
            let x0 = f32::from(bounds.left());
            let y0 = f32::from(bounds.top());

            // Subtle ambient drop shadow under solid portion
            window.paint_quad(quad(
                Bounds::new(
                    point(px(x0 + 2.0), px(y0 + 5.0)),
                    size(px(solid_w), px((bh - 2.0).max(1.0))),
                ),
                Corners {
                    top_left: px(r),
                    bottom_left: px(r),
                    top_right: px(0.0),
                    bottom_right: px(0.0),
                },
                shadow_col,
                Edges::all(px(0.0)),
                transparent_black(),
                BorderStyle::Solid,
            ));

            // Solid left capsule body (with left, top, and bottom 1px hairline border)
            window.paint_quad(quad(
                Bounds::new(point(px(x0), px(y0)), size(px(solid_w), px(bh))),
                Corners {
                    top_left: px(r),
                    bottom_left: px(r),
                    top_right: px(0.0),
                    bottom_right: px(0.0),
                },
                card_bg,
                Edges {
                    top: px(1.0),
                    bottom: px(1.0),
                    left: px(1.0),
                    right: px(0.0),
                },
                border_col,
                BorderStyle::Solid,
            ));

            // Smooth horizontal fade-out tail on the right
            let slices = 28usize;
            let slice_w = fade_w / slices as f32;
            for i in 0..slices {
                let u = (i as f32 + 0.5) / slices as f32;
                let fade = (1.0 - u) * (1.0 - u);
                if fade <= 0.004 {
                    continue;
                }
                let sx = x0 + solid_w + (i as f32) * slice_w;
                let sw = slice_w + 0.4;
                window.paint_quad(fill(
                    Bounds::new(
                        point(px(sx), px(y0 + 5.0)),
                        size(px(sw), px((bh - 2.0).max(1.0))),
                    ),
                    shadow_col.opacity(shadow_col.a * fade),
                ));
                window.paint_quad(quad(
                    Bounds::new(point(px(sx), px(y0)), size(px(sw), px(bh))),
                    Corners::all(px(0.0)),
                    card_bg.opacity(card_bg.a * fade),
                    Edges {
                        top: px(1.0),
                        bottom: px(1.0),
                        left: px(0.0),
                        right: px(0.0),
                    },
                    border_col.opacity(border_col.a * fade),
                    BorderStyle::Solid,
                ));
            }
        },
    )
    .absolute()
    .inset_0()
}

/// Product-scene illustration of an agent building a report as a slim floating capsule pill.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::progress_pill::ProgressPill;
///
/// # fn example(cx: &mut gpui::App) {
/// let _pill = cx.new(|cx| {
///     ProgressPill::new("hero-report-pill", cx)
///         .with_title("Q3 revenue report")
///         .with_name("Analyst")
/// });
/// # }
/// ```
pub struct ProgressPill {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    name: Option<SharedString>,
    steps: Option<Vec<SharedString>>,
    agent: SharedString,
    playing: bool,
    user_paused: bool,
    paused_elapsed_ms: f64,
    hovered: bool,
    hover_slide: Slide,
    press_slide: Slide,
    step: usize,
    hold_ms: u64,
    controlled_progress: Option<f32>,
    show_step_count: bool,
    loading: bool,
    empty: bool,
    started_at: Instant,
    step_entered_at: Instant,
    last_observed_key: (usize, bool),
}

impl ProgressPill {
    /// Create a [`ProgressPill`] with `"Q3 revenue report"`, `"Analyst"`, and the 3 default steps.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = cx.background_executor().now();
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            title: None,
            name: None,
            steps: None,
            agent: "analyst".into(),
            playing: true,
            user_paused: false,
            paused_elapsed_ms: 0.0,
            hovered: false,
            hover_slide: Slide::new(0.),
            press_slide: Slide::snappy(0.),
            step: 1,
            hold_ms: 2600,
            controlled_progress: None,
            show_step_count: false,
            loading: false,
            empty: false,
            started_at: now,
            step_entered_at: now,
            last_observed_key: (1, false),
        }
    }

    /// Set the report or task title shown on the top row (`"Q3 revenue report"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the agent display name shown before the current step subtitle (`"Analyst"` by default).
    pub fn with_name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Override the ordered step subtitles cycled through while building the report.
    pub fn with_steps<I, S>(mut self, steps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        let list: Vec<SharedString> = steps.into_iter().map(Into::into).collect();
        if !list.is_empty() {
            self.steps = Some(list);
        }
        self
    }

    /// Set the agent identifier whose hash determines the 3×3 mirrored pixel mark (`"analyst"` default).
    pub fn with_agent(mut self, agent: impl Into<SharedString>) -> Self {
        self.agent = agent.into();
        self
    }

    /// Control whether the pill cycles automatically from step to step (`true` by default).
    pub fn with_playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Set the zero-based initial or paused step index (`1` by default, `"Pull the numbers"`).
    pub fn with_step(mut self, step: usize) -> Self {
        self.step = step;
        self.last_observed_key = (step, false);
        self
    }

    /// Set how long each step holds before advancing, in milliseconds (`2600` default).
    pub fn with_hold(mut self, hold_ms: u64) -> Self {
        self.hold_ms = hold_ms.max(400);
        self
    }

    /// Alias for [`Self::with_hold`] setting step hold duration in milliseconds.
    pub fn with_hold_ms(self, hold_ms: u64) -> Self {
        self.with_hold(hold_ms)
    }

    /// Pin an explicit controlled progress ratio (`0.0..=1.0`), overriding the internal timer.
    pub fn with_progress(mut self, progress: Option<f32>) -> Self {
        self.controlled_progress = progress.map(|p| p.clamp(0.0, 1.0));
        self
    }

    /// Show `"step/total · pct%"` instead of `"pct%"` on the top-right readout.
    pub fn with_show_step_count(mut self, show: bool) -> Self {
        self.show_step_count = show;
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return whether automatic step cycling is enabled.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Pause or resume automatic step cycling at runtime.
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.playing != playing {
            let now = cx.background_executor().now();
            if !playing {
                self.paused_elapsed_ms +=
                    now.saturating_duration_since(self.started_at).as_secs_f64() * 1000.0;
                self.user_paused = true;
            } else {
                self.started_at = now;
                self.user_paused = false;
            }
            self.playing = playing;
            cx.notify();
        }
    }

    fn toggle_or_replay(&mut self, is_done: bool, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        if is_done {
            self.step = 0;
            self.paused_elapsed_ms = 0.0;
            self.started_at = now;
            self.step_entered_at = now;
            self.playing = true;
            self.user_paused = false;
            cx.notify();
        } else {
            self.set_playing(!self.playing, cx);
        }
    }

    /// Update controlled progress (`0.0..=1.0`) at runtime.
    pub fn set_progress(&mut self, progress: Option<f32>, cx: &mut Context<Self>) {
        let next = progress.map(|p| p.clamp(0.0, 1.0));
        if self.controlled_progress != next {
            self.controlled_progress = next;
            cx.notify();
        }
    }

    /// Advance manually to the next step (and reset the cycle anchor).
    pub fn advance_step(&mut self, step_count: usize, cx: &mut Context<Self>) {
        let n = step_count.max(1);
        let now = cx.background_executor().now();
        self.step = (self.step + 1) % n;
        self.paused_elapsed_ms = 0.0;
        self.started_at = now;
        self.step_entered_at = now;
        cx.notify();
    }

    /// Toggle skeleton loading mode at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether skeleton loading mode is active.
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

    /// Return whether the empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }
}

impl Render for ProgressPill {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();
        let live_delta_ms = if self.playing {
            now.saturating_duration_since(self.started_at).as_secs_f64() * 1000.0
        } else {
            0.0
        };
        let elapsed_ms = self.paused_elapsed_ms + live_delta_ms;
        let elapsed_secs = elapsed_ms / 1000.0;

        if self.loading {
            let phase = ((elapsed_secs as f32) / 1.4).fract();
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .py(px(32.0))
                .flex()
                .items_center()
                .justify_center()
                .child(pro_skeleton_bar(360.0, 68.0, 34.0, phase, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "No active report run", "暂无运行中的报告任务"),
                    tr(
                        cx,
                        "Provide report steps to preview the floating agent progress pill.",
                        "配置步骤列表以预览悬浮智能体进度胶囊。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let steps = self
            .steps
            .clone()
            .unwrap_or_else(|| default_progress_steps(cx));
        let step_count = steps.len().max(1);
        let eval_playing = self.playing || self.user_paused;
        let snap = evaluate_pill_snapshot(
            step_count,
            self.step,
            self.hold_ms,
            eval_playing,
            reduced,
            elapsed_ms,
            self.controlled_progress,
        );
        let snap_done = snap.done;

        let cur_key = (snap.step_index, snap.done);
        if self.last_observed_key != cur_key {
            self.last_observed_key = cur_key;
            self.step_entered_at = now;
            let label: SharedString = if snap.done {
                tr(cx, "Report ready", "报告已就绪").into()
            } else {
                steps.get(snap.step_index).cloned().unwrap_or_default()
            };
            cx.emit(ProgressPillEvent::StepChanged {
                index: if snap.done {
                    step_count
                } else {
                    snap.step_index
                },
                label,
                done: snap.done,
            });
        }

        self.hover_slide.duration(
            if self.hovered { 1.0 } else { 0.0 },
            if self.hovered { 0.22 } else { 0.26 },
            cx,
        );
        let hov_t = self.hover_slide.get(w, cx).clamp(0., 1.);
        let down_t = self.press_slide.get(w, cx).clamp(0., 1.);

        let live_motion = self.playing && !reduced;
        if live_motion {
            w.request_animation_frame();
        }

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Q3 revenue report", "Q3 营收分析报告").into());
        let agent_name: SharedString = self
            .name
            .clone()
            .unwrap_or_else(|| tr(cx, "Analyst", "分析师").into());
        let step_label: SharedString = if snap.done {
            tr(cx, "Report ready", "报告已就绪").into()
        } else {
            steps.get(snap.step_index).cloned().unwrap_or_default()
        };

        let sprung_ratio = pro_spring_f32(
            pro_eid(&self.id, "bar-fill"),
            snap.ratio,
            if reduced { 0.001 } else { 0.48 },
            w,
            cx,
        )
        .clamp(0.0, 1.0);
        let pct_int = (sprung_ratio * 100.0).round() as u32;
        let shown_step = (snap.step_index + 1).min(step_count);
        let readout_text = if self.show_step_count {
            format!("{shown_step}/{step_count} · {pct_int}%")
        } else {
            format!("{pct_int}%")
        };
        let state_badge = if snap.done {
            tr(cx, "Replay", "重播")
        } else if !self.playing {
            tr(cx, "Paused", "已暂停")
        } else {
            tr(cx, "Live", "运行中")
        };
        let hover_detail = format!("{shown_step}/{step_count} · {state_badge}");

        let dark = p.dark;
        let card_bg: Hsla = if dark {
            rgb(0x161618).into()
        } else {
            rgb(0xffffff).into()
        };
        let border_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.10)
        } else {
            hsla(0.0, 0.0, 0.0, 0.08)
        };
        let shadow_col: Hsla = if dark {
            hsla(0.0, 0.0, 0.0, 0.42)
        } else {
            hsla(0.0, 0.0, 0.0, 0.055)
        };
        let ink_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.88)
        } else {
            hsla(0.0, 0.0, 0.0, 0.84)
        };
        let pct_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.64)
        } else {
            hsla(0.0, 0.0, 0.0, 0.62)
        };
        let muted_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.40)
        } else {
            hsla(0.0, 0.0, 0.0, 0.40)
        };
        let shine_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.92)
        } else {
            hsla(0.0, 0.0, 0.0, 0.88)
        };
        let tile_bg: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.05)
        } else {
            hsla(0.0, 0.0, 0.0, 0.035)
        };
        let mark_col: Hsla = if dark {
            hsla(218.0 / 360.0, 1.0, 0.65, 0.85)
        } else {
            hsla(215.0 / 360.0, 1.0, 0.44, 0.78)
        };
        let track_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.08)
        } else {
            hsla(0.0, 0.0, 0.0, 0.06)
        };
        let fill_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.82)
        } else {
            hsla(0.0, 0.0, 0.0, 0.78)
        };
        let sheen_on_fill: Hsla = if dark {
            hsla(0.0, 0.0, 0.0, 0.42)
        } else {
            hsla(0.0, 0.0, 1.0, 0.72)
        };
        let glint_on_track: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.16)
        } else {
            hsla(0.0, 0.0, 0.0, 0.12)
        };

        // Step entrance animation (`420ms` upward micro-glide + fade)
        let step_age = now
            .saturating_duration_since(self.step_entered_at)
            .as_secs_f32();
        let enter_t = if reduced || elapsed_secs < 0.08 {
            1.0
        } else {
            pro_ease((step_age / 0.42).clamp(0.0, 1.0))
        };
        let step_dy = 5.0 * (1.0 - enter_t);

        let subtitle_el =
            div()
                .relative()
                .top(px(step_dy))
                .opacity(enter_t)
                .child(pro_shimmer_text(
                    pro_eid(&self.id, "step-shimmer"),
                    step_label,
                    pct_col,
                    shine_col,
                    11.5,
                    FontWeight::NORMAL,
                    live_motion && !snap.done,
                ));

        // 4px progress bar with synchronized 1.8s sheen over both track and fill
        let sheen_phase = if live_motion && !snap.done {
            Some(((elapsed_secs as f32) / 1.8).fract())
        } else {
            None
        };
        let bar_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 2.0 || bh <= 1.0 {
                    return;
                }
                let r = px(bh * 0.5);
                window.paint_quad(fill(bounds, track_col).corner_radii(r));

                let fill_w = (bw * sprung_ratio).clamp(0.0, bw);
                if fill_w > 1.0 {
                    window.paint_quad(
                        fill(
                            Bounds::new(bounds.origin, size(px(fill_w), bounds.size.height)),
                            fill_col,
                        )
                        .corner_radii(r),
                    );
                }

                if let Some(sp) = sheen_phase {
                    let sheen_w = 72.0f32;
                    let center_x = -sheen_w * 0.5 + sp * (bw + sheen_w);
                    let segs = 12usize;
                    let seg_w = sheen_w / segs as f32;
                    for k in 0..segs {
                        let u = (k as f32 + 0.5) / segs as f32;
                        let bell = (u * std::f32::consts::PI).sin();
                        let bell = bell * bell;
                        let sx0 = (center_x - sheen_w * 0.5 + (k as f32) * seg_w).max(0.0);
                        let sx1 = (sx0 + seg_w + 0.3).min(bw);
                        if sx1 <= sx0 + 0.2 {
                            continue;
                        }
                        let mid = (sx0 + sx1) * 0.5;
                        let col = if mid <= fill_w {
                            sheen_on_fill.opacity(sheen_on_fill.a * bell)
                        } else {
                            glint_on_track.opacity(glint_on_track.a * bell)
                        };
                        window.paint_quad(fill(
                            Bounds::new(
                                point(bounds.left() + px(sx0), bounds.top()),
                                size(px(sx1 - sx0), bounds.size.height),
                            ),
                            col,
                        ));
                    }
                }
            },
        );
        let bar_h = 4.0 + 1.5 * hov_t;
        let bar_canvas = bar_canvas.w_full().h(px(bar_h));

        // Atmospheric drifting blue-purple-pink glow beneath the capsule
        let drift_x = if live_motion {
            6.0 - 6.0 * (((elapsed_secs as f32) / 9.0) * std::f32::consts::PI).cos()
        } else {
            0.0
        };
        let glow_img = under_glow_svg(dark);
        let glow_opacity = ((if dark { 0.52 } else { 0.70 }) + 0.14 * hov_t).clamp(0.0, 0.92);

        let pixel_tile = render_circular_pixel_mark(
            &self.agent,
            mark_col,
            tile_bg,
            !snap.done && self.playing,
            reduced,
            elapsed_ms as f32,
        );

        let card_lift_y = -1.8 * hov_t * (1.0 - down_t) + 1.5 * down_t;
        let content_w = 210.0 + 38.0 * hov_t;
        let detail_max_w = 88.0 * hov_t;

        let pill_card = div()
            .id(pro_eid(&self.id, "pill-card"))
            .role(Role::Button)
            .aria_label(title_str.clone())
            .relative()
            .top(px(card_lift_y))
            .w(px(452.0))
            .h(px(68.0))
            .pl(px(10.0 + 1.2 * down_t))
            .pr(px(20.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .cursor_pointer()
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if this.hovered != *inside {
                    this.hovered = *inside;
                    if !*inside {
                        this.press_slide.duration(0., 0.14, cx);
                    }
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.press_slide.duration(1., 0.08, cx);
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.press_slide.duration(0., 0.14, cx);
                    cx.notify();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.press_slide.duration(0., 0.14, cx);
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.toggle_or_replay(snap_done, cx);
            }))
            .child(paint_fading_capsule(card_bg, border_col, shadow_col))
            .child(div().relative().child(pixel_tile))
            .child(
                div()
                    .relative()
                    .w(px(content_w))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.0))
                                    .line_height(px(18.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(ink_col)
                                    .child(title_str),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0 * hov_t))
                                    .when(hov_t > 0.02, |d| {
                                        d.child(
                                            div()
                                                .max_w(px(detail_max_w))
                                                .overflow_hidden()
                                                .opacity(hov_t)
                                                .px(px(5.5 * hov_t))
                                                .h(px(16.0))
                                                .rounded_full()
                                                .bg(tile_bg)
                                                .flex()
                                                .items_center()
                                                .child(pro_swap_label(
                                                    pro_eid(&self.id, "hover-detail"),
                                                    hover_detail,
                                                    10.0,
                                                    FontWeight::MEDIUM,
                                                    muted_col,
                                                )),
                                        )
                                    })
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_size(px(11.5))
                                            .line_height(px(18.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(pct_col)
                                            .child(readout_text),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(17.0))
                            .overflow_hidden()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_size(px(11.5))
                            .line_height(px(16.0))
                            .text_color(muted_col)
                            .child(format!("{agent_name} ·"))
                            .child(subtitle_el),
                    )
                    .child(div().mt(px(7.0)).w_full().child(bar_canvas)),
            );

        let stack = div()
            .relative()
            .w(px(452.0))
            .pt(px(20.0))
            .pb(px(48.0))
            .child(
                div()
                    .absolute()
                    .left(px(6.0 + drift_x))
                    .top(px(64.0))
                    .w(px(360.0))
                    .h(px(62.0))
                    .opacity(glow_opacity)
                    .child(img(glow_img).w(px(360.0)).h(px(62.0))),
            )
            .child(pill_card);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .w_full()
            .min_h(px(168.0))
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(14.)).border_color(p.focus))
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "space" | " " | "enter" => {
                        this.toggle_or_replay(snap_done, cx);
                        cx.stop_propagation();
                    }
                    "right" | "n" => {
                        this.advance_step(step_count, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(stack)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{ProgressPill, evaluate_pill_snapshot};
    use gpui::{
        MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext, point, px, size,
    };
    use std::time::Duration;

    #[test]
    fn initial_and_static_step_match_reference_percentage() {
        // Default 3 steps, step = 1 ("Pull the numbers") -> 36%
        let snap = evaluate_pill_snapshot(3, 1, 2600, false, false, 0.0, None);
        assert_eq!(snap.step_index, 1);
        assert!(!snap.done);
        let pct = (snap.ratio * 100.0).round() as u32;
        assert_eq!(pct, 36);

        let reduced = evaluate_pill_snapshot(3, 1, 2600, true, true, 5400.0, None);
        assert_eq!(reduced.step_index, 1);
        assert_eq!((reduced.ratio * 100.0).round() as u32, 36);
    }

    #[test]
    fn live_cycle_advances_through_steps_done_beat_and_loops() {
        // Starting at initial_step = 1 with hold_ms = 2600:
        // elapsed 0ms -> step 1 (~36%)
        let s0 = evaluate_pill_snapshot(3, 1, 2600, true, false, 0.0, None);
        assert_eq!(s0.step_index, 1);
        assert!(!s0.done);

        // elapsed 2700ms -> step 2 ("Draft the report")
        let s1 = evaluate_pill_snapshot(3, 1, 2600, true, false, 2700.0, None);
        assert_eq!(s1.step_index, 2);
        assert!(!s1.done);
        assert!(s1.ratio > s0.ratio);

        // elapsed 5300ms -> shifted = 7900ms >= 7800ms -> "Report ready" done beat (100%)
        let done = evaluate_pill_snapshot(3, 1, 2600, true, false, 5300.0, None);
        assert!(done.done);
        assert!((done.ratio - 1.0).abs() < 1e-5);

        // Controlled progress override clamps cleanly
        let ctrl = evaluate_pill_snapshot(4, 0, 2600, true, false, 1200.0, Some(0.75));
        assert!(!ctrl.done);
        assert!((ctrl.ratio - 0.75).abs() < 1e-5);
        assert_eq!(ctrl.step_index, 3);
    }

    #[gpui::test]
    fn hover_expands_detail_readout_and_click_toggles_pause_or_replays_done(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let exec = cx.executor();
        let (view, vcx) =
            cx.add_window_view(|_, cx| ProgressPill::new("pp-test", cx).with_hold(1000));
        vcx.simulate_resize(size(px(560.), px(220.)));
        vcx.run_until_parked();

        // Pill is centered inside `(560, 220)` -> center `(220, 96)` hits `pill-card`
        let pill_pt = point(px(220.), px(96.));
        vcx.simulate_event(MouseMoveEvent {
            position: pill_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        exec.advance_clock(Duration::from_millis(240));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        vcx.update(|_, app| {
            let pp = view.read(app);
            assert!(pp.hovered);
            assert!(pp.hover_slide.value(app) > 0.75);
            assert!(pp.is_playing());
        });

        // Click pill -> pauses while preserving accumulated progress
        vcx.simulate_event(MouseDownEvent {
            position: pill_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(MouseUpEvent {
            position: pill_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        });
        vcx.run_until_parked();
        assert!(!vcx.update(|_, app| view.read(app).is_playing()));

        // Click again -> resumes, then advance into `done` beat and click to replay from step 0!
        vcx.simulate_event(MouseDownEvent {
            position: pill_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(MouseUpEvent {
            position: pill_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        });
        exec.advance_clock(Duration::from_millis(2150));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();
        assert!(vcx.update(|_, app| view.read(app).last_observed_key.1));

        vcx.simulate_event(MouseDownEvent {
            position: pill_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(MouseUpEvent {
            position: pill_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        });
        vcx.run_until_parked();
        assert_eq!(vcx.update(|_, app| view.read(app).step), 0);
        assert!(vcx.update(|_, app| view.read(app).is_playing()));
    }
}
