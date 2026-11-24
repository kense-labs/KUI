//! Isometric design-tool inspection scene of an autonomous agent card.
//!
//! Renders an agent execution card (and optional faded predecessor above it) tilted in
//! 3D perspective with a `#0190fe` Figma-style selection frame, diagonal padding hatch,
//! four corner resize handles, pixel dimension pills, and an animated `#ff3b30` spacing
//! guide walking row by row or pinning to pointer hover.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_ease, pro_eid, pro_empty_state, pro_skeleton_bar, pro_swap_label, tr,
    },
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_raw_svg, pro_svg},
    },
};
use gpui::{prelude::*, *};
use image::{Frame, RgbaImage};
use std::{
    cell::{Cell, RefCell},
    f32::consts::TAU,
    rc::Rc,
    sync::Arc,
};
use web_time::Instant;

const STAGE_W: usize = 436;
const STAGE_H: usize = 356;
const RASTER_SCALE: usize = 2;
const TEX_W: usize = STAGE_W * RASTER_SCALE;
const TEX_H: usize = STAGE_H * RASTER_SCALE;
const CARD_W: f32 = 320.0;
const MAIN_H: f32 = 176.0;
const ABOVE_H: f32 = 112.0;

const LAYER_TAG_SVG: &str = r#"<rect x="1.5" y="2" width="9" height="3" rx="1" fill="currentColor"></rect><rect x="1.5" y="7" width="6" height="3" rx="1" fill="currentColor"></rect>"#;

const CHECK_DONE_SVG: &str = r#"<path d="M5 14L8.5 17.5L19 6.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.4"></path>"#;

const HAND_CURSOR_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="44" height="44" viewBox="0 0 24 24"><path d="M9 11V4.5a1.5 1.5 0 0 1 3 0V10m0-1.5a1.5 1.5 0 0 1 3 0V11m0-1a1.5 1.5 0 0 1 3 0v1.5m0 0a1.5 1.5 0 0 1 3 0V15a7 7 0 0 1-7 7h-1.2a6 6 0 0 1-4.6-2.2L4.6 16.4a1.6 1.6 0 0 1 2.4-2.1L9 16V11" fill="#ffffff" stroke="#111111" stroke-width="1.35" stroke-linejoin="round"/></svg>"##;

/// Execution status badge displayed in the top-right corner of an [`InspectAgent`] card.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectAgentStatus {
    /// Active blue badge with a spinning progress arc (`Running`).
    #[default]
    Running,
    /// Completed green badge with a checkmark (`Done`).
    Done,
}

/// Inspected row slot inside [`InspectAgentCard`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectAgentRow {
    /// Top row containing the avatar, agent name, model subtitle, and status badge (`16px` top gap).
    #[default]
    Header,
    /// Middle row containing tool icons and the two-line task description (`12px` row gap).
    Task,
    /// Bottom row containing tabular dollar cost and elapsed duration (`8px` row gap).
    Meta,
}

impl InspectAgentRow {
    const ALL: [Self; 3] = [Self::Header, Self::Task, Self::Meta];

    fn from_index(idx: usize) -> Self {
        Self::ALL[idx % Self::ALL.len()]
    }

    fn index(self) -> usize {
        match self {
            Self::Header => 0,
            Self::Task => 1,
            Self::Meta => 2,
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Header => tr(cx, "Header", "顶栏"),
            Self::Task => tr(cx, "Task", "任务"),
            Self::Meta => tr(cx, "Meta", "统计"),
        }
    }
}

/// Events emitted by [`InspectAgentCard`] on row pin or playback toggle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectAgentCardEvent {
    /// Emitted when a row is pinned or unpinned by clicking.
    PinRow(Option<InspectAgentRow>),
    /// Emitted when playback is toggled.
    PlayingChanged(bool),
}

impl EventEmitter<InspectAgentCardEvent> for InspectAgentCard {}

/// Data model for a single agent card in [`InspectAgentCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectAgent {
    /// Agent role name (for example `"Coder"`).
    pub name: SharedString,
    /// Frontier model label (for example `"GPT-6 Sol"`).
    pub model: SharedString,
    /// Provider key for the 22x22 avatar mark (`"gpt"`, `"claude"`, `"gemini"`, `"kense"`).
    pub avatar: SharedString,
    /// Connected tool keys shown inline before the task text (for example `["github", "stripe"]`).
    pub tools: Vec<SharedString>,
    /// Short task summary line.
    pub task: SharedString,
    /// Execution state (`Running` or `Done`).
    pub status: InspectAgentStatus,
    /// Formatted cost string (for example `"$0.42"`).
    pub cost: SharedString,
    /// Elapsed runtime string (for example `"48s"`).
    pub time: SharedString,
}

impl InspectAgent {
    /// Create an [`InspectAgent`] with `name`, `model`, and `task`.
    pub fn new(
        name: impl Into<SharedString>,
        model: impl Into<SharedString>,
        task: impl Into<SharedString>,
    ) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            avatar: "gpt".into(),
            tools: Vec::new(),
            task: task.into(),
            status: InspectAgentStatus::Running,
            cost: "$0.42".into(),
            time: "48s".into(),
        }
    }

    /// Set the provider key for the avatar mark.
    pub fn with_avatar(mut self, avatar: impl Into<SharedString>) -> Self {
        self.avatar = avatar.into();
        self
    }

    /// Set the inline tool app keys (for example `["github", "stripe"]`).
    pub fn with_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.tools = tools.into_iter().take(4).map(Into::into).collect();
        self
    }

    /// Set the execution status badge (`Running` or `Done`).
    pub fn with_status(mut self, status: InspectAgentStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the formatted run cost (for example `"$0.42"`).
    pub fn with_cost(mut self, cost: impl Into<SharedString>) -> Self {
        self.cost = cost.into();
        self
    }

    /// Set the formatted elapsed duration (for example `"48s"`).
    pub fn with_time(mut self, time: impl Into<SharedString>) -> Self {
        self.time = time.into();
        self
    }
}

/// Default foreground `Coder` agent card matching the reference product scene.
pub fn default_main_agent() -> InspectAgent {
    InspectAgent::new("Coder", "GPT-6 Sol", "Build the page and wire up checkout")
        .with_avatar("gpt")
        .with_tools(["github", "stripe"])
        .with_status(InspectAgentStatus::Running)
        .with_cost("$0.42")
        .with_time("48s")
}

/// Default faded upper `Researcher` agent card matching the reference product scene.
pub fn default_above_agent() -> InspectAgent {
    InspectAgent::new(
        "Researcher",
        "Sonnet 5.5",
        "Compare the pricing of five competitors",
    )
    .with_avatar("claude")
    .with_tools(["notion", "slack"])
    .with_status(InspectAgentStatus::Done)
    .with_cost("$0.18")
    .with_time("55s")
}

#[derive(Clone, Copy)]
struct RowMetrics {
    frame_top: f32,
    frame_h: f32,
    content_w: f32,
    guide_y0: f32,
    guide_y1: f32,
    gap_label: &'static str,
}

fn row_metrics(row: InspectAgentRow) -> RowMetrics {
    match row {
        InspectAgentRow::Header => RowMetrics {
            frame_top: 0.0,
            frame_h: 52.0,
            content_w: 236.0,
            guide_y0: -16.0,
            guide_y1: 0.0,
            gap_label: "16px",
        },
        InspectAgentRow::Task => RowMetrics {
            frame_top: 44.0,
            frame_h: 56.0,
            content_w: 236.0,
            guide_y0: 42.0,
            guide_y1: 54.0,
            gap_label: "12px",
        },
        InspectAgentRow::Meta => RowMetrics {
            frame_top: 88.0,
            frame_h: 40.0,
            content_w: 236.0,
            guide_y0: 90.0,
            guide_y1: 98.0,
            gap_label: "8px",
        },
    }
}

#[derive(Clone, Copy)]
struct AgentTargetBox {
    fx: f32,
    fy: f32,
    fw: f32,
    fh: f32,
    pad: f32,
    gx: f32,
    gy0: f32,
    gy1: f32,
    gap_label: &'static str,
    dim_prefix: &'static str,
    dim_w: f32,
    sub_tag: Option<(&'static str, &'static str)>,
    row: InspectAgentRow,
}

impl AgentTargetBox {
    fn tag_label(self, cx: &App) -> &'static str {
        if let Some((en, zh)) = self.sub_tag {
            tr(cx, en, zh)
        } else {
            self.row.label(cx)
        }
    }
}

fn agent_target_box(target_idx: usize, mirror: bool) -> AgentTargetBox {
    let gx_edge = if mirror { CARD_W - 14.0 } else { 14.0 };
    match target_idx {
        0..=2 => {
            let row = InspectAgentRow::from_index(target_idx);
            let m = row_metrics(row);
            AgentTargetBox {
                fx: 0.0,
                fy: m.frame_top,
                fw: CARD_W,
                fh: m.frame_h,
                pad: 10.0,
                gx: gx_edge,
                gy0: m.guide_y0,
                gy1: m.guide_y1,
                gap_label: m.gap_label,
                dim_prefix: "W Fill",
                dim_w: m.content_w,
                sub_tag: None,
                row,
            }
        }
        3 => AgentTargetBox {
            fx: 6.0,
            fy: 6.0,
            fw: 158.0,
            fh: 34.0,
            pad: 4.0,
            gx: 42.0,
            gy0: 0.0,
            gy1: 6.0,
            gap_label: "8px",
            dim_prefix: "W Hug",
            dim_w: 150.0,
            sub_tag: Some(("Agent · Model", "智能体模型")),
            row: InspectAgentRow::Header,
        },
        4 => AgentTargetBox {
            fx: 184.0,
            fy: 6.0,
            fw: 68.0,
            fh: 26.0,
            pad: 4.0,
            gx: 218.0,
            gy0: 0.0,
            gy1: 6.0,
            gap_label: "6px",
            dim_prefix: "W Hug",
            dim_w: 60.0,
            sub_tag: Some(("Status Pill", "状态胶囊")),
            row: InspectAgentRow::Header,
        },
        5 => AgentTargetBox {
            fx: 6.0,
            fy: 49.0,
            fw: 42.0,
            fh: 24.0,
            pad: 4.0,
            gx: 27.0,
            gy0: 40.0,
            gy1: 49.0,
            gap_label: "6px",
            dim_prefix: "W Hug",
            dim_w: 34.0,
            sub_tag: Some(("Tool Chips", "工具图标")),
            row: InspectAgentRow::Task,
        },
        _ => AgentTargetBox {
            fx: 6.0,
            fy: 93.0,
            fw: 104.0,
            fh: 28.0,
            pad: 4.0,
            gx: 56.0,
            gy0: 84.0,
            gy1: 93.0,
            gap_label: "8px",
            dim_prefix: "W Hug",
            dim_w: 96.0,
            sub_tag: Some(("Cost · Time", "耗时与费用")),
            row: InspectAgentRow::Meta,
        },
    }
}

/// Perspective homography matching `perspective(1100px) rotateX(30deg) rotateY(+-14deg) rotateZ(+-7deg)`.
#[derive(Clone, Copy)]
struct TiltTransform {
    flat: bool,
    mirror: bool,
    ox: f32,
    oy: f32,
}

impl TiltTransform {
    fn new(flat: bool, mirror: bool) -> Self {
        let (ox, oy) = if flat {
            (58.0, 134.0)
        } else if mirror {
            (46.0, 138.0)
        } else {
            (76.0, 138.0)
        };
        Self {
            flat,
            mirror,
            ox,
            oy,
        }
    }

    fn project(self, u: f32, v: f32) -> (f32, f32) {
        if self.flat {
            return (self.ox + u, self.oy + v);
        }
        if self.mirror {
            let u_rel = u - CARD_W;
            let xp = 0.963_063 * u_rel + 0.118_249 * v;
            let yp = 0.014_517 * u_rel + 0.874_312 * v;
            let wp = (1.0 + 0.000_244 * u_rel - 0.000_428 * v).max(0.25);
            (self.ox + CARD_W + xp / wp, self.oy + yp / wp)
        } else {
            let xp = 0.963_063 * u - 0.118_249 * v;
            let yp = -0.014_517 * u + 0.874_312 * v;
            let wp = (1.0 - 0.000_244 * u - 0.000_428 * v).max(0.25);
            (self.ox + xp / wp, self.oy + yp / wp)
        }
    }

    fn unproject(self, sx: f32, sy: f32) -> (f32, f32) {
        if self.flat {
            return (sx - self.ox, sy - self.oy);
        }
        if self.mirror {
            let dx = sx - (self.ox + CARD_W);
            let dy = sy - self.oy;
            let a11 = 0.963_063 - 0.000_244 * dx;
            let a12 = 0.118_249 + 0.000_428 * dx;
            let a21 = 0.014_517 - 0.000_244 * dy;
            let a22 = 0.874_312 + 0.000_428 * dy;
            let det = a11 * a22 - a12 * a21;
            if det.abs() < 1e-6 {
                return (0., 0.);
            }
            let u_rel = (dx * a22 - dy * a12) / det;
            let v = (a11 * dy - a21 * dx) / det;
            (u_rel + CARD_W, v)
        } else {
            let dx = sx - self.ox;
            let dy = sy - self.oy;
            let a11 = 0.963_063 + 0.000_244 * dx;
            let a12 = -0.118_249 + 0.000_428 * dx;
            let a21 = -0.014_517 + 0.000_244 * dy;
            let a22 = 0.874_312 + 0.000_428 * dy;
            let det = a11 * a22 - a12 * a21;
            if det.abs() < 1e-6 {
                return (0., 0.);
            }
            let u = (dx * a22 - dy * a12) / det;
            let v = (a11 * dy - a21 * dx) / det;
            (u, v)
        }
    }
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// Scene-wide CSS `mask-image` alpha fading right (or left if `mirror`) and bottom/top edges.
fn stage_mask_alpha(sx: f32, sy: f32, mirror: bool) -> f32 {
    let mx = if mirror {
        smoothstep(4.0, STAGE_W as f32 * 0.26, sx)
    } else {
        1. - smoothstep(STAGE_W as f32 * 0.72, STAGE_W as f32 - 4.0, sx)
    };
    let my_bot = 1. - smoothstep(STAGE_H as f32 * 0.64, STAGE_H as f32 * 0.96, sy);
    let my_top = smoothstep(4.0, 56.0, sy);
    (mx * my_bot * my_top).clamp(0., 1.)
}

fn sd_round_rect(px_x: f32, px_y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    let hx = (x1 - x0) * 0.5 - r;
    let hy = (y1 - y0) * 0.5 - r;
    let dx = (px_x - cx).abs() - hx;
    let dy = (px_y - cy).abs() - hy;
    dx.max(0.).hypot(dy.max(0.)) + dx.max(dy).min(0.) - r
}

fn blend_over(dst: [f32; 4], src_rgb: [f32; 3], src_a: f32) -> [f32; 4] {
    let sa = src_a.clamp(0., 1.);
    if sa <= 1e-5 {
        return dst;
    }
    let inv = 1. - sa;
    [
        src_rgb[0] * sa + dst[0] * inv,
        src_rgb[1] * sa + dst[1] * inv,
        src_rgb[2] * sa + dst[2] * inv,
        sa + dst[3] * inv,
    ]
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct InspectPlateKey {
    dark: bool,
    has_above: bool,
    mirror: bool,
    flat: bool,
    fx_q: i32,
    ft_q: i32,
    fw_q: i32,
    fh_q: i32,
    pad_q: i32,
    gx_q: i32,
    gy0_q: i32,
    gy1_q: i32,
    reveal_q: u16,
}

struct InspectPlateCache {
    key: Option<InspectPlateKey>,
    image: Option<Arc<RenderImage>>,
}

impl InspectPlateCache {
    fn new() -> Self {
        Self {
            key: None,
            image: None,
        }
    }

    fn render(&mut self, key: InspectPlateKey, window: &mut Window) -> Arc<RenderImage> {
        if self.key == Some(key)
            && let Some(existing) = &self.image
        {
            return existing.clone();
        }
        let img = Arc::new(rasterize_inspect_plate(key));
        if let Some(prev) = self.image.replace(img.clone()) {
            let _ = window.drop_image(prev);
        }
        self.key = Some(key);
        img
    }
}

fn rasterize_inspect_plate(key: InspectPlateKey) -> RenderImage {
    let tf = TiltTransform::new(key.flat, key.mirror);
    let fx = key.fx_q as f32 / 10.0;
    let ft = key.ft_q as f32 / 10.0;
    let fw = (key.fw_q as f32 / 10.0).max(20.0);
    let fh = (key.fh_q as f32 / 10.0).max(16.0);
    let pad = (key.pad_q as f32 / 10.0).max(3.0);
    let gx = key.gx_q as f32 / 10.0;
    let gy0 = key.gy0_q as f32 / 10.0;
    let gy1 = key.gy1_q as f32 / 10.0;
    let rev = (f32::from(key.reveal_q) / 1000.0).clamp(0., 1.);

    let box_alpha = pro_ease(((rev - 0.10) / 0.45).clamp(0., 1.));
    let handle_alpha = pro_ease(((rev - 0.24) / 0.45).clamp(0., 1.));
    let guide_alpha = pro_ease(((rev - 0.42) / 0.48).clamp(0., 1.));

    let (card_rgb, border_rgb, border_a, sh_a, blue_rgb, hatch_a, handle_fill) = if key.dark {
        (
            [28. / 255., 28. / 255., 31. / 255.],
            [1.0, 1.0, 1.0],
            0.10,
            0.40,
            [0x3b as f32 / 255., 0x82 as f32 / 255., 0xf6 as f32 / 255.],
            0.30,
            [25. / 255., 25. / 255., 27. / 255.],
        )
    } else {
        (
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0],
            0.075,
            0.10,
            [0x01 as f32 / 255., 0x90 as f32 / 255., 0xfe as f32 / 255.],
            0.26,
            [1.0, 1.0, 1.0],
        )
    };
    let red_rgb = [1.0, 59. / 255., 48. / 255.];

    const BAYER4: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    let px_u = 1.0 / RASTER_SCALE as f32;
    let mut raw = vec![0u8; TEX_W * TEX_H * 4];
    let above_u_off = if key.mirror { 16.0 } else { -16.0 };
    let above_y0 = -(ABOVE_H + 16.0);
    let above_y1 = -16.0;

    for py in 0..TEX_H {
        let sy = (py as f32 + 0.5) * px_u;
        for px_i in 0..TEX_W {
            let sx = (px_i as f32 + 0.5) * px_u;
            let mask = stage_mask_alpha(sx, sy, key.mirror);
            if mask <= 0.002 {
                continue;
            }

            let (u, v) = tf.unproject(sx, sy);
            let mut c = [0.0f32; 4];

            // Optional faded predecessor card above the main card
            if key.has_above {
                let au = u - above_u_off;
                let above_fade = smoothstep(above_y0 + 8.0, above_y1 - 8.0, v);
                if above_fade > 0.005 {
                    let d_above = sd_round_rect(au, v, 0.0, above_y0, CARD_W, above_y1, 8.0);
                    let cov_ab = (0.5 - d_above / px_u).clamp(0., 1.) * above_fade;
                    if cov_ab > 0.0 {
                        c = blend_over(c, card_rgb, cov_ab);
                        let b_cov =
                            (0.5 - ((d_above + 0.5).abs() - 0.5) / px_u).clamp(0., 1.) * above_fade;
                        if b_cov > 0.0 {
                            c = blend_over(c, border_rgb, b_cov * border_a);
                        }
                    }
                }
            }

            // Soft elevation shadow beneath the main card
            let d_sh = sd_round_rect(u, v - 10.0, 4.0, 4.0, CARD_W - 4.0, MAIN_H - 4.0, 8.0);
            if d_sh < 24.0 {
                let s_cov = (1. - d_sh.max(0.) / 24.0).powi(2) * sh_a;
                c = blend_over(c, [0.0, 0.0, 0.0], s_cov);
            }

            // Main agent card surface + 1px Retina border
            let d_main = sd_round_rect(u, v, 0.0, 0.0, CARD_W, MAIN_H, 8.0);
            let cov_main = (0.5 - d_main / px_u).clamp(0., 1.);
            if cov_main > 0.0 {
                c = blend_over(c, card_rgb, cov_main);
                let b_cov = (0.5 - ((d_main + 0.5).abs() - 0.5) / px_u).clamp(0., 1.);
                if b_cov > 0.0 {
                    c = blend_over(c, border_rgb, b_cov * border_a);
                }
            }

            // Figma-style `#0190fe` selection frame, anti-aliased diagonal padding hatch, and inner dashed box
            if box_alpha > 0.005 {
                let fr = fx + fw;
                let fb = ft + fh;
                if (fx - 3.0..=fr + 3.0).contains(&u) && (ft - 3.0..=fb + 3.0).contains(&v) {
                    let in_outer = (fx..=fr).contains(&u) && (ft..=fb).contains(&v);
                    let in_inner =
                        (fx + pad..=fr - pad).contains(&u) && (ft + pad..=fb - pad).contains(&v);

                    if in_outer && !in_inner {
                        let stripe = ((u + v) * 0.85).rem_euclid(3.8);
                        let stripe_d = (stripe - 0.45).abs();
                        let stripe_cov = (0.5 - (stripe_d - 0.40) / (0.85 * px_u)).clamp(0., 1.);
                        if stripe_cov > 0.0 {
                            c = blend_over(c, blue_rgb, stripe_cov * hatch_a * box_alpha);
                        }
                    }

                    let d_in = sd_round_rect(u, v, fx + pad, ft + pad, fr - pad, fb - pad, 3.0);
                    let in_line = (0.5 - (d_in.abs() - 0.42) / px_u).clamp(0., 1.);
                    if in_line > 0.0 {
                        let along = (u + v).rem_euclid(5.0);
                        let dash_cov = (0.5 - ((along - 1.4).abs() - 1.35) / px_u).clamp(0., 1.);
                        if dash_cov > 0.0 {
                            c = blend_over(c, blue_rgb, in_line * dash_cov * 0.86 * box_alpha);
                        }
                    }

                    let d_out = sd_round_rect(u, v, fx, ft, fr, fb, 0.5);
                    let out_line = (0.5 - (d_out.abs() - 0.55) / px_u).clamp(0., 1.);
                    if out_line > 0.0 {
                        c = blend_over(c, blue_rgb, out_line * box_alpha);
                    }
                }
            }

            // 4 corner resize handles (`5x5` crisp anti-aliased squares on TL, TR, BR, BL)
            if handle_alpha > 0.005 {
                let fr = fx + fw;
                let fb = ft + fh;
                for (hx, hy) in [(fx, ft), (fr, ft), (fr, fb), (fx, fb)] {
                    let cheb = (u - hx).abs().max((v - hy).abs());
                    let outer_cov = (0.5 - (cheb - 2.55) / px_u).clamp(0., 1.);
                    if outer_cov > 0.0 {
                        let inner_cov = (0.5 - (cheb - 1.55) / px_u).clamp(0., 1.);
                        c = blend_over(c, blue_rgb, outer_cov * handle_alpha);
                        if inner_cov > 0.0 {
                            c = blend_over(c, handle_fill, inner_cov * handle_alpha);
                        }
                    }
                }
            }

            // Red vertical spacing guide (`#ff3b30` dashed line + 7px top & bottom T-bars)
            if guide_alpha > 0.005 && gy1 > gy0 {
                let g_end = gy0 + (gy1 - gy0) * guide_alpha;
                if (v >= gy0 - 1.0 && v <= g_end + 1.0) && (u - gx).abs() <= 4.2 {
                    let stem_cov = (0.5 - ((u - gx).abs() - 0.55) / px_u).clamp(0., 1.);
                    if stem_cov > 0.0 && v >= gy0 && v <= g_end {
                        let dash_a = (v - gy0).rem_euclid(5.0);
                        let d_cov = ((1.5 - (dash_a - 1.5).abs()) / (0.45 * px_u)).clamp(0., 1.);
                        if d_cov > 0.0 {
                            c = blend_over(c, red_rgb, stem_cov * d_cov * guide_alpha);
                        }
                    }
                    let cap_x = (0.5 - ((u - gx).abs() - 3.5) / px_u).clamp(0., 1.);
                    if cap_x > 0.0 {
                        let cap_t = (0.5 - ((v - gy0).abs() - 0.55) / px_u).clamp(0., 1.);
                        let cap_b = (0.5 - ((v - g_end).abs() - 0.55) / px_u).clamp(0., 1.);
                        let cap_cov = cap_t.max(cap_b) * cap_x;
                        if cap_cov > 0.0 {
                            c = blend_over(c, red_rgb, cap_cov * guide_alpha);
                        }
                    }
                }
            }

            let out_a = (c[3] * mask).clamp(0., 1.);
            if out_a > 1e-4 {
                let dither = (f32::from(BAYER4[(py & 3) * 4 + (px_i & 3)]) / 16.0 - 0.5) / 255.0;
                let r = (c[0] / c[3].max(1e-4) + dither).clamp(0., 1.);
                let g = (c[1] / c[3].max(1e-4) + dither).clamp(0., 1.);
                let b = (c[2] / c[3].max(1e-4) + dither).clamp(0., 1.);
                let dst = (py * TEX_W + px_i) * 4;
                raw[dst] = (b * 255.).round() as u8;
                raw[dst + 1] = (g * 255.).round() as u8;
                raw[dst + 2] = (r * 255.).round() as u8;
                raw[dst + 3] = (out_a * 255.).round() as u8;
            }
        }
    }

    for y in 1..TEX_H - 1 {
        let row = y * TEX_W;
        for x in 1..TEX_W - 1 {
            let idx = (row + x) * 4;
            if raw[idx + 3] != 0 {
                continue;
            }
            let mut best_a = 0u8;
            let mut best_bgr = [0u8; 3];
            for (dx, dy) in [
                (-1isize, 0isize),
                (1, 0),
                (0, -1),
                (0, 1),
                (-1, -1),
                (1, -1),
                (-1, 1),
                (1, 1),
            ] {
                let n = (((y as isize + dy) as usize) * TEX_W + (x as isize + dx) as usize) * 4;
                let na = raw[n + 3];
                if na > best_a {
                    best_a = na;
                    best_bgr = [raw[n], raw[n + 1], raw[n + 2]];
                }
            }
            if best_a > 0 {
                raw[idx] = best_bgr[0];
                raw[idx + 1] = best_bgr[1];
                raw[idx + 2] = best_bgr[2];
            }
        }
    }

    let buf = RgbaImage::from_raw(TEX_W as u32, TEX_H as u32, raw)
        .expect("TEX_W * TEX_H * 4 matches raw buffer length");
    RenderImage::new(vec![Frame::new(buf)])
}

/// Evaluate which row is inspected and its `0.0..=1.0` stagger reveal progress at `elapsed_ms`.
fn step_timeline(
    playing: bool,
    hold_ms: u64,
    delay_ms: u64,
    elapsed_ms: u64,
) -> (InspectAgentRow, f32) {
    if !playing {
        return (InspectAgentRow::Header, 1.0);
    }
    if elapsed_ms < delay_ms {
        return (InspectAgentRow::Header, 1.0);
    }
    let active_ms = elapsed_ms - delay_ms;
    let enter_ms = 520u64;
    let exit_ms = 180u64;
    let cycle_ms = enter_ms + hold_ms.max(400) + exit_ms;
    let row_idx = ((active_ms / cycle_ms) as usize + 1) % 3;
    let within = active_ms % cycle_ms;
    let reveal = if within < enter_ms {
        within as f32 / enter_ms as f32
    } else if within < enter_ms + hold_ms.max(400) {
        1.0
    } else {
        1.0 - ((within - enter_ms - hold_ms.max(400)) as f32 / exit_ms as f32).clamp(0., 1.)
    };
    (InspectAgentRow::from_index(row_idx), reveal)
}

fn render_status_pill(status: InspectAgentStatus, spin_turn: f32, dark: bool) -> AnyElement {
    match status {
        InspectAgentStatus::Running => {
            let bg: Hsla = if dark {
                hsla(217. / 360., 0.91, 0.60, 0.18)
            } else {
                hsla(205. / 360., 1.0, 0.48, 0.10)
            };
            let fg: Hsla = if dark {
                rgb(0x7ab4ff).into()
            } else {
                rgb(0x006dcb).into()
            };
            let turn = spin_turn;
            div()
                .h(px(18.))
                .pl(px(5.))
                .pr(px(6.))
                .rounded(px(4.))
                .bg(bg)
                .flex_none()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(
                    canvas(
                        |_, _, _| (),
                        move |b, _, window, _| {
                            let cx_p = b.origin.x + b.size.width * 0.5;
                            let cy_p = b.origin.y + b.size.height * 0.5;
                            let rad = 4.2f32;
                            let mut ring = PathBuilder::stroke(px(1.5));
                            for i in 0..=16 {
                                let a = (i as f32 / 16.) * TAU;
                                let pt = point(cx_p + px(a.cos() * rad), cy_p + px(a.sin() * rad));
                                if i == 0 {
                                    ring.move_to(pt);
                                } else {
                                    ring.line_to(pt);
                                }
                            }
                            if let Ok(path) = ring.build() {
                                window.paint_path(path, fg.opacity(0.26));
                            }
                            let mut arc = PathBuilder::stroke(px(1.6));
                            for i in 0..=8 {
                                let a = (turn + i as f32 / 24.) * TAU;
                                let pt = point(cx_p + px(a.cos() * rad), cy_p + px(a.sin() * rad));
                                if i == 0 {
                                    arc.move_to(pt);
                                } else {
                                    arc.line_to(pt);
                                }
                            }
                            if let Ok(path) = arc.build() {
                                window.paint_path(path, fg);
                            }
                        },
                    )
                    .size(px(12.))
                    .flex_none(),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(fg)
                        .child("Running"),
                )
                .into_any_element()
        }
        InspectAgentStatus::Done => {
            let bg: Hsla = if dark {
                hsla(142. / 360., 0.71, 0.45, 0.16)
            } else {
                hsla(139. / 360., 1.0, 0.32, 0.10)
            };
            let fg: Hsla = if dark {
                rgb(0x5fd38a).into()
            } else {
                rgb(0x007d2e).into()
            };
            div()
                .h(px(18.))
                .pl(px(4.))
                .pr(px(6.))
                .rounded(px(4.))
                .bg(bg)
                .flex_none()
                .flex()
                .items_center()
                .gap(px(3.))
                .child(pro_svg("0 0 24 24", CHECK_DONE_SVG, 11., fg))
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(fg)
                        .child("Done"),
                )
                .into_any_element()
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_card_rows(
    agent: &InspectAgent,
    tf: TiltTransform,
    u_offset: f32,
    v_offset: f32,
    spin_turn: f32,
    dark: bool,
    mirror: bool,
    extra_fade: f32,
    cx: &App,
) -> Vec<AnyElement> {
    let mut out = Vec::with_capacity(3);
    let strong_c: Hsla = if dark {
        rgb(0xffffff).into()
    } else {
        rgb(0x000000).into()
    };
    let ink_c: Hsla = if dark {
        hsla(0., 0., 1., 0.92)
    } else {
        hsla(0., 0., 0., 0.88)
    };
    let text_c: Hsla = if dark {
        hsla(0., 0., 1., 0.62)
    } else {
        hsla(0., 0., 0., 0.61)
    };
    let muted_c: Hsla = if dark {
        hsla(0., 0., 1., 0.45)
    } else {
        hsla(0., 0., 0., 0.45)
    };
    let tile_bg: Hsla = if dark {
        rgb(0x1c1c1f).into()
    } else {
        rgb(0xffffff).into()
    };
    let tile_bdr: Hsla = if dark {
        hsla(0., 0., 1., 0.11)
    } else {
        hsla(0., 0., 0., 0.08)
    };

    // Row 0: Header (`u = 10..246, v = 10`)
    let (hx, hy) = tf.project(10.0 + u_offset, 10.0 + v_offset);
    let h_alpha = stage_mask_alpha(hx + 80., hy + 14., mirror) * extra_fade;
    if h_alpha > 0.02 {
        let av_mark = pro_app_artwork(&agent.avatar, 13., cx).unwrap_or_else(|| {
            div()
                .text_size(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink_c)
                .child(
                    agent
                        .name
                        .chars()
                        .next()
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "A".to_string()),
                )
                .into_any_element()
        });
        out.push(
            div()
                .absolute()
                .left(px(hx))
                .top(px(hy))
                .w(px(236.))
                .flex()
                .items_center()
                .gap(px(8.))
                .opacity(h_alpha)
                .child(
                    div()
                        .size(px(22.))
                        .rounded(px(6.))
                        .bg(tile_bg)
                        .border_1()
                        .border_color(tile_bdr)
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(av_mark),
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
                                .line_height(px(16.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(strong_c)
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(agent.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .line_height(px(14.))
                                .text_color(muted_c)
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(agent.model.clone()),
                        ),
                )
                .child(render_status_pill(agent.status, spin_turn, dark))
                .into_any_element(),
        );
    }

    // Row 1: Task (`u = 10..246, v = 54` on main, `v = 48` on compact above card)
    let task_v = if v_offset < 0.0 { 46.0 } else { 54.0 };
    let (tx, ty) = tf.project(10.0 + u_offset, task_v + v_offset);
    let t_alpha = stage_mask_alpha(tx + 80., ty + 14., mirror) * extra_fade;
    if t_alpha > 0.02 {
        let mut tool_strip = div().flex().items_center().mr(px(6.)).flex_none();
        for (ti, tool_key) in agent.tools.iter().enumerate() {
            let t_art = pro_app_artwork(tool_key, 10., cx).unwrap_or_else(|| {
                div()
                    .text_size(px(8.))
                    .text_color(ink_c)
                    .child("•")
                    .into_any_element()
            });
            tool_strip = tool_strip.child(
                div()
                    .when(ti > 0, |d| d.ml(px(-3.)))
                    .size(px(16.))
                    .rounded(px(4.))
                    .bg(tile_bg)
                    .border_1()
                    .border_color(tile_bdr)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(t_art),
            );
        }
        out.push(
            div()
                .absolute()
                .left(px(tx))
                .top(px(ty))
                .w(px(236.))
                .flex()
                .items_start()
                .opacity(t_alpha)
                .when(!agent.tools.is_empty(), |d| d.child(tool_strip))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(text_c)
                        .child(agent.task.clone()),
                )
                .into_any_element(),
        );
    }

    // Row 2: Meta (`u = 10..246, v = 98` on main, `v = 84` on compact above card)
    let meta_v = if v_offset < 0.0 { 84.0 } else { 98.0 };
    let (mx, my) = tf.project(10.0 + u_offset, meta_v + v_offset);
    let m_alpha = stage_mask_alpha(mx + 40., my + 8., mirror) * extra_fade;
    if m_alpha > 0.02 {
        out.push(
            div()
                .absolute()
                .left(px(mx))
                .top(px(my))
                .w(px(236.))
                .h(px(20.))
                .flex()
                .items_center()
                .gap(px(4.))
                .opacity(m_alpha)
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ink_c)
                        .child(agent.cost.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(px(16.))
                        .text_color(muted_c)
                        .child(format!("· {}", agent.time)),
                )
                .into_any_element(),
        );
    }

    out
}

/// Design-tool inspection scene of an autonomous agent card with `#0190fe` Figma chrome.
///
/// # Examples
///
/// ```ignore
/// use kui::components::inspect_agent_card::InspectAgentCard;
///
/// let scene = cx.new(|cx| InspectAgentCard::new("hero-agent-inspect", cx));
/// ```
pub struct InspectAgentCard {
    id: ElementId,
    focus: FocusHandle,
    agent: InspectAgent,
    above: Option<InspectAgent>,
    playing: bool,
    hold_ms: u64,
    delay_ms: u64,
    mirror: bool,
    flat: bool,
    manual_target: Option<usize>,
    pinned_target: Option<usize>,
    pinned_row: Option<InspectAgentRow>,
    fx_slide: Slide,
    ft_slide: Slide,
    fw_slide: Slide,
    fh_slide: Slide,
    pad_slide: Slide,
    gx_slide: Slide,
    gy0_slide: Slide,
    gy1_slide: Slide,
    press_slide: Slide,
    loading: bool,
    empty: bool,
    clock: Instant,
    stage_bounds: Rc<Cell<Bounds<Pixels>>>,
    plate_cache: Rc<RefCell<InspectPlateCache>>,
}

impl InspectAgentCard {
    /// Construct an [`InspectAgentCard`] with the default `Coder` and faded `Researcher` cards.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let t0 = agent_target_box(0, false);
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            agent: default_main_agent(),
            above: Some(default_above_agent()),
            playing: true,
            hold_ms: 1600,
            delay_ms: 600,
            mirror: false,
            flat: false,
            manual_target: None,
            pinned_target: None,
            pinned_row: None,
            fx_slide: Slide::new(t0.fx),
            ft_slide: Slide::new(t0.fy),
            fw_slide: Slide::new(t0.fw),
            fh_slide: Slide::new(t0.fh),
            pad_slide: Slide::new(t0.pad),
            gx_slide: Slide::new(t0.gx),
            gy0_slide: Slide::new(t0.gy0),
            gy1_slide: Slide::new(t0.gy1),
            press_slide: Slide::snappy(0.),
            loading: false,
            empty: false,
            clock: cx.background_executor().now(),
            stage_bounds: Rc::new(Cell::new(Bounds::default())),
            plate_cache: Rc::new(RefCell::new(InspectPlateCache::new())),
        }
    }

    /// Override the foreground inspected [`InspectAgent`] card.
    pub fn with_agent(mut self, agent: InspectAgent) -> Self {
        self.agent = agent;
        self
    }

    /// Override or clear (`None`) the faded agent card above the main card.
    pub fn with_above(mut self, above: Option<InspectAgent>) -> Self {
        self.above = above;
        self
    }

    /// Control whether the selection walks through `Header`, `Task`, and `Meta` (`true` by default).
    pub fn with_playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Set how long the selection stays on each row in milliseconds (`1600` by default).
    pub fn with_hold(mut self, hold_ms: u64) -> Self {
        self.hold_ms = hold_ms.max(400);
        self
    }

    /// Set the initial pause on `Header` before stepping to `Task` in milliseconds (`600` by default).
    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }

    /// Mirror the 3D perspective tilt and fade to the left side (`false` by default).
    pub fn with_mirror(mut self, mirror: bool) -> Self {
        self.mirror = mirror;
        self
    }

    /// Render the card flat with zero 3D perspective tilt (`false` by default).
    pub fn with_flat(mut self, flat: bool) -> Self {
        self.flat = flat;
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

    /// Return whether row-walking playback is active.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Toggle row-walking playback at runtime.
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.playing != playing {
            self.playing = playing;
            self.clock = cx.background_executor().now();
            cx.emit(InspectAgentCardEvent::PlayingChanged(playing));
            cx.notify();
        }
    }

    /// Return the manually pinned row, if any.
    pub fn pinned_row(&self) -> Option<InspectAgentRow> {
        self.pinned_row
    }

    /// Pin or unpin inspection on a specific row at runtime.
    pub fn set_pinned_row(&mut self, row: Option<InspectAgentRow>, cx: &mut Context<Self>) {
        if self.pinned_row != row || self.pinned_target != row.map(InspectAgentRow::index) {
            self.pinned_row = row;
            self.pinned_target = row.map(InspectAgentRow::index);
            cx.emit(InspectAgentCardEvent::PinRow(row));
            cx.notify();
        }
    }

    /// Return whether mirrored perspective is enabled.
    pub fn is_mirror(&self) -> bool {
        self.mirror
    }

    /// Return whether flat (non-tilted) mode is enabled.
    pub fn is_flat(&self) -> bool {
        self.flat
    }

    /// Return whether skeleton loading is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle skeleton loading at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    fn hit_target_uv(&self, u: f32, v: f32) -> Option<usize> {
        if !(0.0..=CARD_W).contains(&u) || !(0.0..=MAIN_H).contains(&v) {
            return None;
        }
        if (184.0..=252.0).contains(&u) && (4.0..=34.0).contains(&v) {
            Some(4)
        } else if (6.0..=168.0).contains(&u) && (4.0..=40.0).contains(&v) {
            Some(3)
        } else if !self.agent.tools.is_empty()
            && (6.0..=50.0).contains(&u)
            && (48.0..=76.0).contains(&v)
        {
            Some(5)
        } else if (6.0..=114.0).contains(&u) && (92.0..=124.0).contains(&v) {
            Some(6)
        } else if v < 46.0 {
            Some(0)
        } else if v < 92.0 {
            Some(1)
        } else {
            Some(2)
        }
    }

    fn hit_target(&self, pos: Point<Pixels>) -> Option<usize> {
        let b = self.stage_bounds.get();
        if b.size.width <= px(2.) {
            return None;
        }
        let sx = f32::from(pos.x - b.origin.x);
        let sy = f32::from(pos.y - b.origin.y);
        let tf = TiltTransform::new(self.flat, self.mirror);
        let (u, v) = tf.unproject(sx, sy);
        self.hit_target_uv(u, v)
    }
}

impl Render for InspectAgentCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();
        let elapsed = now.saturating_duration_since(self.clock);

        if self.loading {
            let phase = if reduced {
                0.0
            } else {
                w.request_animation_frame();
                (elapsed.as_secs_f32() / 1.4).fract()
            };
            return div()
                .id(pro_eid(&self.id, "loading"))
                .w_full()
                .h(px(STAGE_H as f32))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(300.))
                        .p(px(14.))
                        .rounded(px(8.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(196., 22., 6., phase, cx))
                        .child(pro_skeleton_bar(236., 32., 6., phase, cx))
                        .child(pro_skeleton_bar(92., 14., 4., phase, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(pro_eid(&self.id, "empty"))
                .w_full()
                .child(pro_empty_state::<Self>(
                    ProIcon::Sliders,
                    tr(cx, "No agent card to inspect", "暂无可检视的智能体卡片"),
                    tr(
                        cx,
                        "Provide an InspectAgent to render the pixel-inspection scene.",
                        "配置 InspectAgent 后即可渲染设计工具像素检视场景。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let elapsed_ms = elapsed.as_millis() as u64;
        let spin_turn = if reduced {
            0.0
        } else {
            ((elapsed_ms % 900) as f32) / 900.0
        };

        let interactive_override = self
            .manual_target
            .or(self.pinned_target)
            .or_else(|| self.pinned_row.map(InspectAgentRow::index));
        let auto_animate = !reduced
            && ((self.playing && interactive_override.is_none())
                || self.agent.status == InspectAgentStatus::Running);
        let (active_target_idx, reveal) = if let Some(target_idx) = interactive_override {
            (target_idx, 1.0)
        } else if reduced || !self.playing {
            (0, 1.0)
        } else {
            let (row, rev) = step_timeline(self.playing, self.hold_ms, self.delay_ms, elapsed_ms);
            (row.index(), rev)
        };

        let tb = agent_target_box(active_target_idx, self.mirror);
        self.fx_slide.duration(tb.fx, 0.24, cx);
        self.ft_slide.duration(tb.fy, 0.24, cx);
        self.fw_slide.duration(tb.fw, 0.24, cx);
        self.fh_slide.duration(tb.fh, 0.24, cx);
        self.pad_slide.duration(tb.pad, 0.22, cx);
        self.gx_slide.duration(tb.gx, 0.24, cx);
        self.gy0_slide.duration(tb.gy0, 0.24, cx);
        self.gy1_slide.duration(tb.gy1, 0.24, cx);

        let (cur_fx, cur_ft, cur_fw, cur_fh, cur_pad, cur_gx, cur_gy0, cur_gy1, press_t) =
            if reduced {
                (
                    tb.fx, tb.fy, tb.fw, tb.fh, tb.pad, tb.gx, tb.gy0, tb.gy1, 0.0,
                )
            } else {
                (
                    self.fx_slide.get(w, cx),
                    self.ft_slide.get(w, cx),
                    self.fw_slide.get(w, cx),
                    self.fh_slide.get(w, cx),
                    self.pad_slide.get(w, cx),
                    self.gx_slide.get(w, cx),
                    self.gy0_slide.get(w, cx),
                    self.gy1_slide.get(w, cx),
                    self.press_slide.get(w, cx),
                )
            };

        let tf = TiltTransform::new(self.flat, self.mirror);
        let plate_key = InspectPlateKey {
            dark,
            has_above: self.above.is_some(),
            mirror: self.mirror,
            flat: self.flat,
            fx_q: (cur_fx * 10.).round() as i32,
            ft_q: (cur_ft * 10.).round() as i32,
            fw_q: (cur_fw * 10.).round() as i32,
            fh_q: (cur_fh * 10.).round() as i32,
            pad_q: (cur_pad * 10.).round() as i32,
            gx_q: (cur_gx * 10.).round() as i32,
            gy0_q: (cur_gy0 * 10.).round() as i32,
            gy1_q: (cur_gy1 * 10.).round() as i32,
            reveal_q: (reveal * 1000.).round() as u16,
        };

        let bounds_cell = self.stage_bounds.clone();
        let plate_cache = self.plate_cache.clone();

        let mut scene = div()
            .id(pro_eid(&self.id, "scene"))
            .relative()
            .w(px(STAGE_W as f32))
            .h(px(STAGE_H as f32))
            .flex_none()
            .overflow_hidden()
            .cursor_pointer()
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| {
                let hit = this.hit_target(e.position);
                if this.manual_target != hit {
                    let was_some = this.manual_target.is_some();
                    this.manual_target = hit;
                    if hit.is_none()
                        && was_some
                        && this.pinned_target.is_none()
                        && this.pinned_row.is_none()
                    {
                        this.clock = cx.background_executor().now();
                    }
                    cx.notify();
                }
            }))
            .on_hover(cx.listener(|this, over: &bool, _, cx| {
                if !*over && this.manual_target.take().is_some() {
                    this.press_slide.duration(0., 0.12, cx);
                    if this.pinned_target.is_none() && this.pinned_row.is_none() {
                        this.clock = cx.background_executor().now();
                    }
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    this.press_slide.duration(1., 0.08, cx);
                    let hit = this.hit_target(e.position);
                    let next_target = if this.pinned_target == hit { None } else { hit };
                    let next_row = next_target.map(|idx| agent_target_box(idx, this.mirror).row);
                    this.pinned_target = next_target;
                    this.pinned_row = next_row;
                    if next_target.is_none() && this.manual_target.is_none() {
                        this.clock = cx.background_executor().now();
                    }
                    cx.emit(InspectAgentCardEvent::PinRow(next_row));
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    this.press_slide.duration(0., 0.14, cx);
                    cx.notify();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    this.press_slide.duration(0., 0.14, cx);
                    cx.notify();
                }),
            )
            .child(
                canvas(
                    move |b, _, _| bounds_cell.set(b),
                    move |bounds, _, window, _| {
                        let image = plate_cache.borrow_mut().render(plate_key, window);
                        let _ =
                            window.paint_image(bounds, bounds, Corners::default(), image, 0, false);
                        if auto_animate {
                            window.request_animation_frame();
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );

        if let Some(above_agent) = &self.above {
            let u_off = if self.mirror { 16.0 } else { -16.0 };
            for el in render_card_rows(
                above_agent,
                tf,
                u_off,
                -(ABOVE_H + 16.0),
                spin_turn,
                dark,
                self.mirror,
                0.56,
                cx,
            ) {
                scene = scene.child(el);
            }
        }

        for el in render_card_rows(
            &self.agent,
            tf,
            0.0,
            0.0,
            spin_turn,
            dark,
            self.mirror,
            1.0,
            cx,
        ) {
            scene = scene.child(el);
        }

        // Figma inspection pills: layer tag, top & side blue padding badges, red spacing pill, dimension callout, and hand cursor
        let blue_pill: Hsla = if dark {
            rgb(0x3b82f6).into()
        } else {
            rgb(0x0190fe).into()
        };
        let red_pill: Hsla = rgb(0xff3b30).into();
        let white_ink: Hsla = rgb(0xffffff).into();

        let tag_a = pro_ease(((reveal - 0.30) / 0.45).clamp(0., 1.));
        let pad_a = pro_ease(((reveal - 0.38) / 0.45).clamp(0., 1.));
        let dim_a = pro_ease(((reveal - 0.48) / 0.45).clamp(0., 1.));
        let cur_a = pro_ease((reveal / 0.38).clamp(0., 1.));

        // Layer tag pill (`Header` / `Agent · Model` / `Status Pill` / `Tool Chips` / etc.)
        let tag_u = if self.mirror {
            (cur_fx + cur_fw - 68.0).max(cur_fx)
        } else {
            cur_fx
        };
        let (tag_x, tag_y) = tf.project(tag_u, cur_ft - 17.0 + press_t * 1.0);
        let is_pinned = self.pinned_target.is_some() || self.pinned_row.is_some();
        let tag_text = if is_pinned {
            format!("{} •", tb.tag_label(cx))
        } else {
            tb.tag_label(cx).to_string()
        };
        scene = scene.child(
            div()
                .absolute()
                .left(px(tag_x))
                .top(px(tag_y))
                .h(px(15.))
                .px(px(5.))
                .rounded(px(3.))
                .bg(blue_pill)
                .flex()
                .items_center()
                .gap(px(3.))
                .opacity(tag_a * stage_mask_alpha(tag_x + 20., tag_y + 7., self.mirror))
                .child(pro_svg("0 0 12 12", LAYER_TAG_SVG, 9., white_ink))
                .child(pro_swap_label(
                    pro_eid(&self.id, "layer-tag"),
                    tag_text,
                    9.,
                    FontWeight::MEDIUM,
                    white_ink,
                )),
        );

        // Top blue padding badge (`10` or `4`)
        let pad_label = format!("{:.0}", cur_pad.round());
        let pad_top_u = cur_fx + cur_fw * 0.62;
        let (pt_x, pt_y) = tf.project(pad_top_u, cur_ft - 2.0);
        scene = scene.child(
            div()
                .absolute()
                .left(px(pt_x))
                .top(px(pt_y))
                .h(px(13.))
                .px(px(3.5))
                .rounded(px(3.))
                .bg(blue_pill)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(8.5))
                .line_height(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(white_ink)
                .opacity(pad_a * stage_mask_alpha(pt_x, pt_y, self.mirror))
                .child(pad_label.clone()),
        );

        // Side blue padding badge
        let pad_side_u = if self.mirror {
            cur_fx + cur_fw - 14.0
        } else {
            cur_fx - 2.0
        };
        let (ps_x, ps_y) = tf.project(pad_side_u, cur_ft + cur_fh * 0.5 - 6.5);
        scene = scene.child(
            div()
                .absolute()
                .left(px(ps_x))
                .top(px(ps_y))
                .h(px(13.))
                .px(px(3.5))
                .rounded(px(3.))
                .bg(blue_pill)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(8.5))
                .line_height(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(white_ink)
                .opacity(pad_a * stage_mask_alpha(ps_x, ps_y, self.mirror))
                .child(pad_label),
        );

        // Red spacing guide badge (`16px` / `12px` / `8px` / `6px`) beside the red dashed guide
        let red_u = if self.mirror {
            (cur_gx - 32.0).max(12.0)
        } else {
            cur_gx + 6.0
        };
        let red_v = (cur_gy0 + cur_gy1) * 0.5 - 6.5;
        let (rx, ry) = tf.project(red_u, red_v);
        scene = scene.child(
            div()
                .absolute()
                .left(px(rx))
                .top(px(ry))
                .h(px(13.))
                .px(px(4.))
                .rounded(px(3.))
                .bg(red_pill)
                .flex()
                .items_center()
                .justify_center()
                .opacity(dim_a * stage_mask_alpha(rx, ry, self.mirror))
                .child(pro_swap_label(
                    pro_eid(&self.id, "gap-tag"),
                    tb.gap_label,
                    8.5,
                    FontWeight::MEDIUM,
                    white_ink,
                )),
        );

        // Dimension callout (`W Fill 236 px` / `W Hug 60 px`) below the frame
        let size_u = if self.mirror {
            (cur_fx + cur_fw - 88.0).max(cur_fx)
        } else {
            cur_fx + (cur_fw - 88.0).max(0.0)
        };
        let (sz_x, sz_y) = tf.project(size_u, cur_ft + cur_fh + 7.0);
        let dim_px_text = format!(
            "{:.0} × {:.0} px",
            (cur_fw - cur_pad * 2.0)
                .max(16.0)
                .min(tb.dim_w.max(cur_fw - cur_pad * 2.0)),
            cur_fh.round()
        );
        scene = scene.child(
            div()
                .absolute()
                .left(px(sz_x))
                .top(px(sz_y))
                .h(px(15.))
                .px(px(5.))
                .rounded(px(3.))
                .bg(blue_pill)
                .flex()
                .items_center()
                .gap(px(4.))
                .opacity(dim_a * stage_mask_alpha(sz_x + 25., sz_y, self.mirror))
                .child(
                    div()
                        .text_size(px(9.))
                        .line_height(px(14.))
                        .text_color(white_ink.opacity(0.76))
                        .child(tb.dim_prefix),
                )
                .child(pro_swap_label(
                    pro_eid(&self.id, "dim-px"),
                    dim_px_text,
                    9.,
                    FontWeight::MEDIUM,
                    white_ink,
                )),
        );

        // Crisp white/black hand pointer hovering over the selected region
        let cur_u = cur_fx + cur_fw * 0.68;
        let (cx_p, cy_p) = tf.project(cur_u, cur_ft + cur_fh - 16.0 + press_t * 1.5);
        let cur_slide = (1. - cur_a) * 6.0;
        scene = scene.child(
            div()
                .absolute()
                .left(px(cx_p + cur_slide))
                .top(px(cy_p + cur_slide))
                .opacity(cur_a * stage_mask_alpha(cx_p, cy_p, self.mirror))
                .child(pro_raw_svg(HAND_CURSOR_SVG, 19., 19.)),
        );

        let focus_ring = p.focus;
        let a11y = tr(
            cx,
            "Design-tool pixel inspection of a tilted autonomous agent card",
            "三维透视倾斜智能体卡片的像素间距检视场景",
        );

        div()
            .id(self.id.clone())
            .role(Role::Image)
            .aria_label(a11y)
            .track_focus(&self.focus)
            .tab_index(0)
            .w_full()
            .min_h(px(STAGE_H as f32))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(14.)).border_color(focus_ring))
            .on_key_down(cx.listener(
                |this, e: &KeyDownEvent, _, cx| match e.keystroke.key.as_str() {
                    "down" | "right" | "j" => {
                        let cur = this
                            .pinned_target
                            .or_else(|| this.pinned_row.map(InspectAgentRow::index))
                            .unwrap_or(0);
                        let next_idx = (cur + 1) % 7;
                        let next_row = agent_target_box(next_idx, this.mirror).row;
                        this.pinned_target = Some(next_idx);
                        this.pinned_row = Some(next_row);
                        cx.emit(InspectAgentCardEvent::PinRow(Some(next_row)));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "up" | "left" | "k" => {
                        let cur = this
                            .pinned_target
                            .or_else(|| this.pinned_row.map(InspectAgentRow::index))
                            .unwrap_or(0);
                        let next_idx = (cur + 6) % 7;
                        let next_row = agent_target_box(next_idx, this.mirror).row;
                        this.pinned_target = Some(next_idx);
                        this.pinned_row = Some(next_row);
                        cx.emit(InspectAgentCardEvent::PinRow(Some(next_row)));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "escape" => {
                        this.pinned_target = None;
                        this.pinned_row = None;
                        this.manual_target = None;
                        this.clock = cx.background_executor().now();
                        cx.emit(InspectAgentCardEvent::PinRow(None));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "enter" | "space" | " " => {
                        this.playing = !this.playing;
                        this.clock = cx.background_executor().now();
                        cx.emit(InspectAgentCardEvent::PlayingChanged(this.playing));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                },
            ))
            .child(scene)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CARD_W, InspectAgentCard, InspectAgentRow, InspectPlateKey, RASTER_SCALE, STAGE_H, TEX_W,
        TiltTransform, agent_target_box, default_above_agent, default_main_agent,
        rasterize_inspect_plate, row_metrics, step_timeline,
    };
    use gpui::{MouseButton, MouseDownEvent, MouseMoveEvent, TestAppContext, point, px, size};
    use std::time::Duration;

    #[test]
    fn perspective_homography_roundtrips_and_mirrors_cleanly() {
        let tf = TiltTransform::new(false, false);
        let (sx, sy) = tf.project(160.0, 72.0);
        let (u, v) = tf.unproject(sx, sy);
        assert!((u - 160.0).abs() < 0.05 && (v - 72.0).abs() < 0.05);

        let tf_m = TiltTransform::new(false, true);
        let (mx, my) = tf_m.project(160.0, 72.0);
        let (mu, mv) = tf_m.unproject(mx, my);
        assert!((mu - 160.0).abs() < 0.05 && (mv - 72.0).abs() < 0.05);

        let main = default_main_agent();
        let above = default_above_agent();
        assert_eq!(main.name.as_ref(), "Coder");
        assert_eq!(above.name.as_ref(), "Researcher");
    }

    #[test]
    fn timeline_walks_header_task_meta_and_plate_paints_selection_blue() {
        let (r0, rev0) = step_timeline(true, 1600, 600, 100);
        assert_eq!(r0, InspectAgentRow::Header);
        assert!((rev0 - 1.0).abs() < 1e-3);

        let (r1, rev1) = step_timeline(true, 1600, 600, 600 + 800);
        assert_eq!(r1, InspectAgentRow::Task);
        assert!((rev1 - 1.0).abs() < 1e-3);
        assert_eq!(row_metrics(r1).gap_label, "12px");

        let (r2, _) = step_timeline(true, 1600, 600, 600 + 2300 + 800);
        assert_eq!(r2, InspectAgentRow::Meta);
        assert_eq!(row_metrics(r2).gap_label, "8px");

        let tb = agent_target_box(0, false);
        let img = rasterize_inspect_plate(InspectPlateKey {
            dark: false,
            has_above: true,
            mirror: false,
            flat: true,
            fx_q: (tb.fx * 10.) as i32,
            ft_q: (tb.fy * 10.) as i32,
            fw_q: (tb.fw * 10.) as i32,
            fh_q: (tb.fh * 10.) as i32,
            pad_q: (tb.pad * 10.) as i32,
            gx_q: (tb.gx * 10.) as i32,
            gy0_q: (tb.gy0 * 10.) as i32,
            gy1_q: (tb.gy1 * 10.) as i32,
            reveal_q: 1000,
        });
        let bytes = img.as_bytes(0).expect("BGRA plate");
        // Sample right on the top `#0190fe` selection border (`x = 58 + 120, y = 134`)
        let idx = ((134 * RASTER_SCALE) * TEX_W + (58 + 120) * RASTER_SCALE) * 4;
        assert!(
            bytes[idx] > 200
                && bytes[idx + 2] < 120
                && bytes[idx] > bytes[idx + 2] + 100
                && STAGE_H > 300,
            "expected #0190fe blue selection line, got B={} R={}",
            bytes[idx],
            bytes[idx + 2]
        );
    }

    #[gpui::test]
    fn hovering_subregions_glides_devmode_box_and_resumes_tour_on_leave(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let exec = cx.executor();
        let (view, vcx) = cx.add_window_view(|_, cx| {
            InspectAgentCard::new("test-inspect-agent", cx)
                .with_flat(true)
                .with_delay(300)
                .with_hold(500)
        });
        vcx.simulate_resize(size(px(600.), px(460.)));
        vcx.run_until_parked();

        // Project Status Pill center `(u = 210, v = 16)` in flat coordinates
        let (stage_origin, pill_pt, chip_pt, out_pt) = vcx.update(|_, app| {
            let this = view.read(app);
            let b = this.stage_bounds.get();
            let tf = TiltTransform::new(true, false);
            let (px_u, py_v) = tf.project(210.0, 16.0);
            let (cx_u, cy_v) = tf.project(24.0, 60.0);
            (
                b.origin,
                point(b.origin.x + px(px_u), b.origin.y + px(py_v)),
                point(b.origin.x + px(cx_u), b.origin.y + px(cy_v)),
                point(b.origin.x + px(4.0), b.origin.y + px(4.0)),
            )
        });
        assert!(stage_origin.x >= px(0.));

        vcx.simulate_event(MouseMoveEvent {
            position: pill_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        vcx.run_until_parked();
        exec.advance_clock(Duration::from_millis(320));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        vcx.update(|_, app| {
            let this = view.read(app);
            assert_eq!(this.manual_target, Some(4));
            assert!((this.fx_slide.value(app) - 184.0).abs() < 4.0);
            assert!((this.fw_slide.value(app) - 68.0).abs() < 4.0);
        });

        // Hover over Tool Chips (`u = 24, v = 60` -> target 5) and click to pin
        vcx.simulate_event(MouseMoveEvent {
            position: chip_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        vcx.simulate_event(MouseDownEvent {
            position: chip_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.run_until_parked();
        vcx.update(|_, app| {
            let this = view.read(app);
            assert_eq!(this.pinned_target, Some(5));
            assert_eq!(this.pinned_row(), Some(InspectAgentRow::Task));
        });

        // Click again to unpin, then move pointer off card -> resumes auto tour cleanly
        vcx.simulate_event(MouseDownEvent {
            position: chip_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(MouseMoveEvent {
            position: out_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        vcx.run_until_parked();
        exec.advance_clock(Duration::from_millis(600));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();
        vcx.update(|_, app| {
            let this = view.read(app);
            assert_eq!(this.manual_target, None);
            assert_eq!(this.pinned_target, None);
            assert!((this.fw_slide.value(app) - CARD_W).abs() < 4.0);
        });
    }
}
