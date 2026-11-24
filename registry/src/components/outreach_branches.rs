//! Branching channel lines to Gmail, Slack, and Notion with flowing light pulses.
//!
//! Renders a `3 : 2` transparent card illustration (`320 × 240` logical artboard)
//! where a single source apex fans out along smooth cubic curves into three app
//! tiles while staggered pastel light pulses travel down each branch and wrap
//! around the destination cards on arrival.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_eid, pro_empty_state, pro_mix_color, pro_skeleton_bar, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_raw_svg, pro_svg_doc},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{Arc, OnceLock},
};
use web_time::Instant;

const ART_W: f32 = 320.0;
const ART_H: f32 = 240.0;
const LOOP_SECS: f32 = 4.2;
const STAGGER_SECS: [f32; 3] = [0.6, 0.0, 1.2];

/// One destination channel tile in [`OutreachBranches`].
#[derive(Clone, Debug, PartialEq)]
pub struct OutreachChannel {
    /// Application identifier (such as `"gmail"`, `"slack"`, or `"notion"`).
    pub id: SharedString,
    /// Accessible display name for the destination channel.
    pub name: SharedString,
    /// Accent glow tint when a light pulse lands on this tile.
    pub accent: Hsla,
}

impl OutreachChannel {
    /// Construct a channel definition with `id` and display `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        let id_s: SharedString = id.into();
        let accent = default_channel_accent(&id_s);
        Self {
            id: id_s,
            name: name.into(),
            accent,
        }
    }

    /// Override the landing glow tint for this channel tile.
    pub fn with_accent(mut self, accent: Hsla) -> Self {
        self.accent = accent;
        self
    }
}

fn default_channel_accent(id: &str) -> Hsla {
    match id.trim().to_ascii_lowercase().as_str() {
        "gmail" | "google mail" => rgb(0xff8a96).into(),
        "slack" => rgb(0xc8a8e5).into(),
        "notion" => rgb(0x8e9dff).into(),
        _ => rgb(0xa8b5ff).into(),
    }
}

/// Default `[Gmail, Slack, Notion]` destination channels for [`OutreachBranches`].
pub fn default_outreach_channels() -> [OutreachChannel; 3] {
    [
        OutreachChannel::new("gmail", "Gmail").with_accent(rgb(0xff8a96).into()),
        OutreachChannel::new("slack", "Slack").with_accent(rgb(0xc8a8e5).into()),
        OutreachChannel::new("notion", "Notion").with_accent(rgb(0x8e9dff).into()),
    ]
}

/// Interaction events emitted by [`OutreachBranches`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutreachBranchesEvent {
    /// User clicked a destination tile (or triggered a branch pulse).
    TriggerBranch {
        /// Zero-based branch index (`0 = left`, `1 = center`, `2 = right`).
        index: usize,
        /// Channel identifier for that branch.
        channel: SharedString,
    },
    /// User clicked the top source node to fire pulses across all three branches.
    TriggerAll,
    /// User hovered or focused a specific branch (`None` when cleared).
    FocusBranch(Option<usize>),
}

impl EventEmitter<OutreachBranchesEvent> for OutreachBranches {}

#[derive(Clone, Copy, Debug)]
struct TileSpec {
    x: f32,
    y: f32,
    side: f32,
    rx: f32,
}

const TILE_SPECS: [TileSpec; 3] = [
    TileSpec {
        x: 44.0,
        y: 130.0,
        side: 56.0,
        rx: 12.0,
    },
    TileSpec {
        x: 128.0,
        y: 120.0,
        side: 64.0,
        rx: 14.0,
    },
    TileSpec {
        x: 220.0,
        y: 130.0,
        side: 56.0,
        rx: 12.0,
    },
];

/// Evaluate `cubic-bezier(0.7, 0.0, 0.3, 1.0)` matching the reference pulse easing.
fn pulse_bezier(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 || t >= 1.0 {
        return t;
    }
    let mut u = t;
    for _ in 0..7 {
        let inv = 1.0 - u;
        let x = 3.0 * inv * inv * u * 0.7 + 3.0 * inv * u * u * 0.3 + u * u * u;
        let dx = 3.0 * inv * inv * 0.7 + 6.0 * inv * u * (0.3 - 0.7) + 3.0 * u * u * (1.0 - 0.3);
        if dx.abs() < 1e-5 {
            break;
        }
        u = (u - (x - t) / dx).clamp(0.0, 1.0);
    }
    let inv = 1.0 - u;
    3.0 * inv * u * u + u * u * u
}

/// Sample the horizontal 4-stop pulse gradient (`x` in `40.0..=280.0`).
fn pulse_gradient_color(art_x: f32) -> Hsla {
    let u = ((art_x - 40.0) / 240.0).clamp(0.0, 1.0);
    let c0: Hsla = rgb(0xff8a96).into();
    let c1: Hsla = rgb(0xffa77f).into();
    let c2: Hsla = rgb(0xb98ee5).into();
    let c3: Hsla = rgb(0x8e9dff).into();
    if u < 0.33 {
        pro_mix_color(c0, c1, u / 0.33)
    } else if u < 0.66 {
        pro_mix_color(c1, c2, (u - 0.33) / 0.33)
    } else {
        pro_mix_color(c2, c3, (u - 0.66) / 0.34)
    }
}

/// Sample the center tile bottom rim pastel gradient (`u` in `0.0..=1.0`).
fn center_edge_color(u: f32, dark: bool) -> Hsla {
    let gray: Hsla = if dark {
        rgb(0x3a3a3f).into()
    } else {
        rgb(0xe4e7ec).into()
    };
    let s1: Hsla = rgb(0xffb5bc).into();
    let s2: Hsla = rgb(0xffc5ab).into();
    let s3: Hsla = rgb(0xd5bbef).into();
    let s4: Hsla = rgb(0xb5c0ff).into();
    let u = u.clamp(0.0, 1.0);
    if u < 0.18 {
        pro_mix_color(gray, s1, u / 0.18)
    } else if u < 0.40 {
        pro_mix_color(s1, s2, (u - 0.18) / 0.22)
    } else if u < 0.60 {
        pro_mix_color(s2, s3, (u - 0.40) / 0.20)
    } else if u < 0.82 {
        pro_mix_color(s3, s4, (u - 0.60) / 0.22)
    } else {
        pro_mix_color(s4, gray, (u - 0.82) / 0.18)
    }
}

/// Sample one of the three branch curves (`0 = left`, `1 = center`, `2 = right`) in `320×240` artboard units.
fn sample_branch_line(branch: usize, steps: usize) -> Vec<(f32, f32)> {
    let steps = steps.max(2);
    let mut out = Vec::with_capacity(steps + 1);
    let (x0, y0) = (160.0f32, 52.0f32);
    match branch {
        1 => {
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                out.push((x0, y0 + (120.0 - y0) * t));
            }
        }
        0 | 2 => {
            let tx = if branch == 0 { 72.0 } else { 248.0 };
            let ty = 130.0;
            let (c1x, c1y) = (160.0, 86.0);
            let (c2x, c2y) = (tx, 86.0);
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let inv = 1.0 - t;
                let x = inv * inv * inv * x0
                    + 3.0 * inv * inv * t * c1x
                    + 3.0 * inv * t * t * c2x
                    + t * t * t * tx;
                let y = inv * inv * inv * y0
                    + 3.0 * inv * inv * t * c1y
                    + 3.0 * inv * t * t * c2y
                    + t * t * t * ty;
                out.push((x, y));
            }
        }
        _ => {}
    }
    out
}

/// Sample half of a destination tile's rounded-rectangle perimeter from top-center to bottom-center.
fn sample_tile_half_perimeter(tile: TileSpec, left_half: bool) -> Vec<(f32, f32)> {
    let mut pts = Vec::with_capacity(36);
    let cx_mid = tile.x + tile.side * 0.5;
    let r = tile.rx;
    pts.push((cx_mid, tile.y));
    if left_half {
        for i in 1..=6 {
            let u = i as f32 / 6.0;
            pts.push((cx_mid + (tile.x + r - cx_mid) * u, tile.y));
        }
        let arc1_cx = tile.x + r;
        let arc1_cy = tile.y + r;
        for i in 1..=8 {
            let a = -std::f32::consts::FRAC_PI_2 - (i as f32 / 8.0) * std::f32::consts::FRAC_PI_2;
            pts.push((arc1_cx + r * a.cos(), arc1_cy + r * a.sin()));
        }
        for i in 1..=10 {
            let u = i as f32 / 10.0;
            pts.push((tile.x, tile.y + r + (tile.side - 2.0 * r) * u));
        }
        let arc2_cx = tile.x + r;
        let arc2_cy = tile.y + tile.side - r;
        for i in 1..=8 {
            let a = std::f32::consts::PI - (i as f32 / 8.0) * std::f32::consts::FRAC_PI_2;
            pts.push((arc2_cx + r * a.cos(), arc2_cy + r * a.sin()));
        }
        for i in 1..=6 {
            let u = i as f32 / 6.0;
            pts.push((tile.x + r + (cx_mid - (tile.x + r)) * u, tile.y + tile.side));
        }
    } else {
        let right_edge = tile.x + tile.side;
        for i in 1..=6 {
            let u = i as f32 / 6.0;
            pts.push((cx_mid + (right_edge - r - cx_mid) * u, tile.y));
        }
        let arc1_cx = right_edge - r;
        let arc1_cy = tile.y + r;
        for i in 1..=8 {
            let a = -std::f32::consts::FRAC_PI_2 + (i as f32 / 8.0) * std::f32::consts::FRAC_PI_2;
            pts.push((arc1_cx + r * a.cos(), arc1_cy + r * a.sin()));
        }
        for i in 1..=10 {
            let u = i as f32 / 10.0;
            pts.push((right_edge, tile.y + r + (tile.side - 2.0 * r) * u));
        }
        let arc2_cx = right_edge - r;
        let arc2_cy = tile.y + tile.side - r;
        for i in 1..=8 {
            let a = (i as f32 / 8.0) * std::f32::consts::FRAC_PI_2;
            pts.push((arc2_cx + r * a.cos(), arc2_cy + r * a.sin()));
        }
        for i in 1..=6 {
            let u = i as f32 / 6.0;
            pts.push((
                right_edge - r + (cx_mid - (right_edge - r)) * u,
                tile.y + tile.side,
            ));
        }
    }
    pts
}

fn cumulative_lengths(pts: &[(f32, f32)]) -> (Vec<f32>, f32) {
    let mut cum = Vec::with_capacity(pts.len());
    let mut total = 0.0f32;
    cum.push(0.0);
    for i in 1..pts.len() {
        let dx = pts[i].0 - pts[i - 1].0;
        let dy = pts[i].1 - pts[i - 1].1;
        total += (dx * dx + dy * dy).sqrt();
        cum.push(total);
    }
    (cum, total.max(1e-4))
}

/// Paint a 4-layer tapered comet pulse along `pts` matching the reference `strokeDasharray` stack.
fn paint_comet_along(
    pts: &[(f32, f32)],
    progress: f32,
    max_opacity: f32,
    stroke_w: f32,
    map_pt: &dyn Fn(f32, f32) -> Point<Pixels>,
    window: &mut Window,
) {
    if pts.len() < 2 || max_opacity <= 0.004 {
        return;
    }
    let (cum, total) = cumulative_lengths(pts);
    // Reference layers: [[36, 0.18], [25, 0.38], [15, 0.70], [7, 1.0]] on pathLength=100.
    // Center travels from -18 to +118 across progress 0..=1 so entry and exit taper smoothly.
    let head_pct = -18.0 + progress.clamp(0.0, 1.0) * 136.0;
    const LAYERS: [(f32, f32); 4] = [(36.0, 0.18), (25.0, 0.38), (15.0, 0.70), (7.0, 1.0)];

    for (dash_pct, rel_alpha) in LAYERS {
        let half = dash_pct * 0.5;
        let s0 = ((head_pct - half) / 100.0 * total).clamp(0.0, total);
        let s1 = ((head_pct + half) / 100.0 * total).clamp(0.0, total);
        if s1 - s0 <= 0.25 {
            continue;
        }
        let mid_s = (s0 + s1) * 0.5;
        let mut sample_x = pts[0].0;
        let mut builder = PathBuilder::stroke(px(stroke_w));
        let mut started = false;
        for i in 0..(pts.len() - 1) {
            let a_s = cum[i];
            let b_s = cum[i + 1];
            if a_s <= mid_s && mid_s <= b_s {
                let u = ((mid_s - a_s) / (b_s - a_s).max(1e-5)).clamp(0.0, 1.0);
                sample_x = pts[i].0 + (pts[i + 1].0 - pts[i].0) * u;
            }
            if b_s <= s0 || a_s >= s1 {
                continue;
            }
            let seg_len = (b_s - a_s).max(1e-5);
            let u0 = ((s0 - a_s) / seg_len).clamp(0.0, 1.0);
            let u1 = ((s1 - a_s) / seg_len).clamp(0.0, 1.0);
            let p_start = map_pt(
                pts[i].0 + (pts[i + 1].0 - pts[i].0) * u0,
                pts[i].1 + (pts[i + 1].1 - pts[i].1) * u0,
            );
            let p_end = map_pt(
                pts[i].0 + (pts[i + 1].0 - pts[i].0) * u1,
                pts[i].1 + (pts[i + 1].1 - pts[i].1) * u1,
            );
            if !started {
                builder.move_to(p_start);
                started = true;
            }
            builder.line_to(p_end);
        }
        if started && let Ok(path) = builder.build() {
            let col =
                pulse_gradient_color(sample_x).opacity((rel_alpha * max_opacity).clamp(0.0, 1.0));
            window.paint_path(path, col);
        }
    }
}

/// Evaluate `(line_progress, wrap_progress, land_glow)` for one branch at `cycle_t` in `0.0..1.0`.
fn branch_phase_state(cycle_t: f32) -> (Option<f32>, Option<f32>, f32) {
    let t = cycle_t.rem_euclid(1.0);
    let line = if t <= 0.40 {
        Some(pulse_bezier(t / 0.40))
    } else {
        None
    };
    let wrap = if (0.233..=0.618).contains(&t) {
        Some(pulse_bezier((t - 0.233) / (0.618 - 0.233)))
    } else {
        None
    };
    let glow = if (0.20..=0.72).contains(&t) {
        let u = (t - 0.20) / 0.52;
        (u * std::f32::consts::PI).sin().clamp(0.0, 1.0)
    } else {
        0.0
    };
    (line, wrap, glow)
}

fn ambient_pastel_glow_svg() -> Arc<Image> {
    static GLOW: OnceLock<Arc<Image>> = OnceLock::new();
    const MARKUP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="280" height="120" viewBox="0 0 140 60" fill="none"><defs><filter id="obg" x="-50%" y="-80%" width="200%" height="260%"><feGaussianBlur stdDeviation="10"/></filter><linearGradient id="obgrad" x1="0%" y1="0%" x2="100%" y2="0%"><stop offset="0%" stop-color="#FFA0AB" stop-opacity="0.62"/><stop offset="33%" stop-color="#FFB89E" stop-opacity="0.62"/><stop offset="66%" stop-color="#C8A8E5" stop-opacity="0.62"/><stop offset="100%" stop-color="#A8B5FF" stop-opacity="0.62"/></linearGradient></defs><ellipse cx="70" cy="28" rx="38" ry="12" fill="url(#obgrad)" filter="url(#obg)"/></svg>"##;
    GLOW.get_or_init(|| {
        Arc::new(Image::from_bytes(
            ImageFormat::Svg,
            MARKUP.as_bytes().to_vec(),
        ))
    })
    .clone()
}

fn outreach_channel_mark(channel_id: &str, width: f32, dark: bool, cx: &App) -> AnyElement {
    static NOTION_LIGHT_SVG: OnceLock<Arc<[u8]>> = OnceLock::new();
    static NOTION_DARK_SVG: OnceLock<Arc<[u8]>> = OnceLock::new();

    const GMAIL_MARKUP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="76" viewBox="0 0 56.4108 44.8722" fill="none"><path d="M44.2255 5.76727H56.4051V41.0239C56.4051 43.148 54.683 44.8701 52.5589 44.8701H46.1486C45.8961 44.8701 45.646 44.8204 45.4127 44.7237C45.1794 44.6271 44.9674 44.4854 44.7888 44.3069C44.6102 44.1283 44.4686 43.9163 44.3719 43.683C44.2753 43.4497 44.2255 43.1996 44.2255 42.947V5.76727Z" fill="url(#obgm0)"/><path d="M12.1796 5.76727H0V41.0239C0 43.148 1.72213 44.8701 3.84618 44.8701H10.2565C10.509 44.8701 10.7591 44.8204 10.9924 44.7237C11.2257 44.6271 11.4377 44.4854 11.6163 44.3069C11.7949 44.1283 11.9365 43.9163 12.0332 43.683C12.1298 43.4497 12.1796 43.1996 12.1796 42.947V5.76727Z" fill="#FC413D"/><path d="M10.0085 1.42778C7.43377 -0.736342 3.5924 -0.403647 1.42828 2.17105C-0.735843 4.74543 -0.403148 8.58681 2.17155 10.7512L26.5557 31.2479C27.0178 31.6362 27.602 31.8492 28.2056 31.8492C28.8091 31.8492 29.3934 31.6362 29.8554 31.2479L54.2396 10.7509C56.814 8.58681 57.1467 4.74543 54.9825 2.17073C52.8184 -0.403648 48.977 -0.736342 46.4027 1.42778L28.2054 16.724L10.0085 1.42778Z" fill="url(#obgm1)"/><defs><linearGradient id="obgm0" x1="50.3153" y1="5.76727" x2="50.3153" y2="44.8701" gradientUnits="userSpaceOnUse"><stop stop-color="#60D673"/><stop offset="0.17" stop-color="#42C868"/><stop offset="0.39" stop-color="#0EBC5F"/><stop offset="0.62" stop-color="#00A9BB"/><stop offset="0.86" stop-color="#3C90FF"/><stop offset="1" stop-color="#3186FF"/></linearGradient><linearGradient id="obgm1" x1="0" y1="6.45153" x2="56.4107" y2="6.45153" gradientUnits="userSpaceOnUse"><stop offset="0.08" stop-color="#FF63A0"/><stop offset="0.3" stop-color="#FC413D"/><stop offset="0.65" stop-color="#FC413D"/><stop offset="0.72" stop-color="#FC5C30"/><stop offset="0.86" stop-color="#FEB10C"/><stop offset="0.91" stop-color="#FEC700"/><stop offset="0.96" stop-color="#FFDB0F"/></linearGradient></defs></svg>"##;

    const SLACK_MARKUP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 49.883 49.877" fill="none"><path d="M18.3549 10.4784C15.4713 10.4784 13.1157 8.12279 13.1157 5.2392C13.1157 2.35561 15.4713 0 18.3549 0C21.2385 0 23.5941 2.35561 23.5941 5.2392V10.4784H18.3549Z" fill="#36C5F0"/><path d="M18.3575 13.12C21.2411 13.12 23.5967 15.4756 23.5967 18.3592C23.5967 21.2428 21.2411 23.5984 18.3575 23.5984H5.2392C2.35561 23.5984 0 21.2428 0 18.3592C0 15.4756 2.35561 13.12 5.2392 13.12H18.3575Z" fill="#36C5F0"/><g transform="translate(26.28 0)"><path d="M13.1244 18.3592C13.1244 15.4756 15.48 13.12 18.3636 13.12C21.2472 13.12 23.6028 15.4756 23.6028 18.3592C23.6028 21.2428 21.2472 23.5984 18.3636 23.5984H13.1244V18.3592Z" fill="#2EB67D"/><path d="M10.4785 18.3575C10.4785 21.2411 8.12286 23.5967 5.23927 23.5967C2.35568 23.5967 0 21.2411 0 18.3575V5.2392C0 2.35561 2.35568 0 5.23927 0C8.12286 0 10.4785 2.35561 10.4785 5.2392V18.3575Z" fill="#2EB67D"/></g><g transform="translate(0 26.28)"><path d="M10.4784 5.2392C10.4784 8.12279 8.12279 10.4784 5.2392 10.4784C2.35561 10.4784 0 8.12279 0 5.2392C0 2.35561 2.35561 0 5.2392 0H10.4784V5.2392Z" fill="#E01E5A"/><path d="M13.1157 5.2392C13.1157 2.35561 15.4713 0 18.3549 0C21.2385 0 23.5941 2.35561 23.5941 5.2392V18.3575C23.5941 21.2411 21.2385 23.5967 18.3549 23.5967C15.4713 23.5967 13.1157 21.2411 13.1157 18.3575V5.2392Z" fill="#E01E5A"/></g><g transform="translate(26.28 26.28)"><path d="M5.23927 13.1157C8.12286 13.1157 10.4785 15.4713 10.4785 18.3549C10.4785 21.2385 8.12286 23.5941 5.23927 23.5941C2.35568 23.5941 0 21.2385 0 18.3549V13.1157H5.23927Z" fill="#ECB22E"/><path d="M5.23927 10.4784C2.35568 10.4784 0 8.12279 0 5.2392C0 2.35561 2.35568 0 5.23927 0H18.3576C21.2412 0 23.5968 2.35561 23.5968 5.2392C23.5968 8.12279 21.2412 10.4784 18.3576 10.4784H5.23927Z" fill="#ECB22E"/></g></svg>"##;

    const NOTION_BODY: &str = r##"M45.393 0.182995L4.49201 3.25648C1.18211 3.4929 0 5.73891 0 8.33954V53.2597C0 55.2693 0.709265 57.0424 2.48243 59.2884L12.0575 71.8188C13.5942 73.8284 15.131 74.3012 18.0863 74.0648L65.607 71.2277C69.6262 70.9913 70.8083 69.0999 70.8083 65.9082V15.314C70.8083 13.659 70.2173 13.1862 68.2077 11.7677C68.0895 11.6494 67.9712 11.6494 67.853 11.5312L54.8498 2.429C51.7764 0.0647844 50.476 -0.171637 45.393 0.182995ZM19.1502 14.4865C15.2492 14.7229 14.4217 14.8411 12.1757 12.9498L6.61981 8.57597C6.02875 7.98491 6.26518 7.27565 7.80192 7.15744L47.1661 4.32038C50.476 4.08395 52.131 5.14785 53.4313 6.21175L60.1693 11.0584C60.524 11.1766 61.2332 12.0041 60.2875 12.0041L19.623 14.4865H19.1502ZM14.6581 65.4354V22.5248C14.6581 20.6335 15.2492 19.806 16.9042 19.6878L63.5974 16.9689C65.1342 16.8507 65.8435 17.7964 65.8435 19.6878V62.2437C65.8435 64.1351 65.4888 65.6718 63.0064 65.79L18.3227 68.3907C15.8403 68.6271 14.6581 67.6814 14.6581 65.4354ZM58.7508 24.8891C58.9872 26.1894 58.7508 27.4897 57.4505 27.6079L55.3227 28.0808V59.7613C53.4313 60.8252 51.7764 61.298 50.3578 61.298C48.1118 61.298 47.5208 60.5887 45.7476 58.461L31.5623 36.2373V57.6335L36.0543 58.6974C36.0543 58.6974 36.0543 61.298 32.508 61.298L22.5783 61.8891C22.3419 61.298 22.5783 59.8795 23.524 59.6431L26.1246 58.9338V30.5632L22.5783 30.2086C22.3419 28.9082 23.0511 27.0169 25.0607 26.8987L35.6997 26.1894L50.3578 48.6494V28.79L46.5751 28.3172C46.3387 26.7804 47.4026 25.5983 48.8211 25.4801L58.7508 24.8891Z"##;
    const NOTION_PAGE: &str = r##"M4.49207 3.25638L45.393 0.182895C50.3579 -0.289949 51.6582 0.0646841 54.8499 2.31069L67.9713 11.5311C70.0991 13.0679 70.8084 13.5407 70.8084 15.3139V65.7899C70.8084 68.9816 69.6263 70.873 65.6071 71.1094L18.0863 73.9465C15.0128 74.0647 13.5943 73.7101 12.0576 71.7005L2.48248 59.2883C0.709318 56.9241 0 55.2692 0 53.2596V8.33945C0 5.73881 1.18216 3.4928 4.49207 3.25638Z"##;

    let lower = channel_id.trim().to_ascii_lowercase();
    match lower.as_str() {
        "gmail" | "google mail" => pro_raw_svg(GMAIL_MARKUP, width, width * (44.8722 / 56.4108)),
        "slack" => pro_raw_svg(SLACK_MARKUP, width, width),
        "notion" => {
            let cache = if dark {
                &NOTION_DARK_SVG
            } else {
                &NOTION_LIGHT_SVG
            };
            let bytes = cache
                .get_or_init(|| {
                    let (page, mark) = if dark {
                        ("#191919", "#ededed")
                    } else {
                        ("#ffffff", "#171717")
                    };
                    let doc = format!(
                        r#"<svg xmlns="http://www.w3.org/2000/svg" width="92" height="96" viewBox="0 0 70.808 74.12" fill="none"><path fill="{page}" d="{NOTION_PAGE}"/><path fill="{mark}" fill-rule="evenodd" clip-rule="evenodd" d="{NOTION_BODY}"/></svg>"#
                    );
                    Arc::from(doc.into_bytes())
                })
                .clone();
            let key = 0x4e4f_5449_4f4e_0000u64 | u64::from(dark);
            pro_svg_doc(key, bytes, width, width * (74.12 / 70.808))
        }
        _ => {
            if let Some(mark) = pro_app_artwork(channel_id, width, cx) {
                mark
            } else {
                pro_app_tile(channel_id, width, cx)
            }
        }
    }
}

/// Card illustration of three branching lines fanning out to `Gmail`, `Slack`, and `Notion` tiles.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::outreach_branches::OutreachBranches;
///
/// # fn example(cx: &mut gpui::App) {
/// let _branches = cx.new(|cx| OutreachBranches::new("feature-branches", cx));
/// # }
/// ```
pub struct OutreachBranches {
    id: ElementId,
    focus: FocusHandle,
    label: Option<SharedString>,
    channels: [OutreachChannel; 3],
    playing: bool,
    focused_branch: Option<usize>,
    hovered_branch: Option<usize>,
    source_hovered: bool,
    branch_illum: [Slide; 3],
    tile_press: [Slide; 3],
    apex_illum: Slide,
    apex_press: Slide,
    height: Option<Pixels>,
    loading: bool,
    empty: bool,
    started_at: Instant,
    bursts: [Option<Instant>; 3],
    measured_size: Rc<Cell<(f32, f32)>>,
}

impl OutreachBranches {
    /// Create an [`OutreachBranches`] illustration with the default `Gmail`, `Slack`, and `Notion` tiles.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            label: None,
            channels: default_outreach_channels(),
            playing: true,
            focused_branch: None,
            hovered_branch: None,
            source_hovered: false,
            branch_illum: [Slide::snappy(0.), Slide::snappy(0.), Slide::snappy(0.)],
            tile_press: [Slide::snappy(0.), Slide::snappy(0.), Slide::snappy(0.)],
            apex_illum: Slide::snappy(0.),
            apex_press: Slide::snappy(0.),
            height: None,
            loading: false,
            empty: false,
            started_at: cx.background_executor().now(),
            bursts: [None, None, None],
            measured_size: Rc::new(Cell::new((480.0, 320.0))),
        }
    }

    /// Override the accessible image description (`aria-label`).
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Customize the three destination channel tiles (`[left, center, right]`).
    pub fn with_channels(mut self, channels: [OutreachChannel; 3]) -> Self {
        self.channels = channels;
        self
    }

    /// Control whether light pulses animate continuously (`true` by default).
    pub fn with_playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Pin focus to a specific branch (`0..3`), dimming sibling branches slightly.
    pub fn with_focused_branch(mut self, branch: Option<usize>) -> Self {
        self.focused_branch = branch.filter(|&b| b < 3);
        self
    }

    /// Override the explicit container height (otherwise scales `3 : 2` from width, `320px` default).
    pub fn with_height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }

    /// Configure the skeleton loading placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the three configured destination channels.
    pub fn channels(&self) -> &[OutreachChannel; 3] {
        &self.channels
    }

    /// Return whether continuous pulse playback is active.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Toggle continuous pulse playback at runtime.
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.playing != playing {
            self.playing = playing;
            cx.notify();
        }
    }

    /// Return the currently focused branch (`0..3`), if any.
    pub fn focused_branch(&self) -> Option<usize> {
        self.focused_branch
    }

    /// Focus or clear focus on a specific branch at runtime.
    pub fn set_focused_branch(&mut self, branch: Option<usize>, cx: &mut Context<Self>) {
        let next = branch.filter(|&b| b < 3);
        if self.focused_branch != next {
            self.focused_branch = next;
            cx.emit(OutreachBranchesEvent::FocusBranch(next));
            cx.notify();
        }
    }

    /// Fire an immediate light pulse down `branch` (`0..3`).
    pub fn trigger_branch(&mut self, branch: usize, cx: &mut Context<Self>) {
        if branch >= 3 {
            return;
        }
        self.bursts[branch] = Some(cx.background_executor().now());
        self.focused_branch = if self.focused_branch == Some(branch) {
            None
        } else {
            Some(branch)
        };
        let channel = self.channels[branch].id.clone();
        cx.emit(OutreachBranchesEvent::TriggerBranch {
            index: branch,
            channel,
        });
        cx.notify();
    }

    /// Fire synchronized light pulses down all three branches from the top source node.
    pub fn trigger_all(&mut self, cx: &mut Context<Self>) {
        let now = cx.background_executor().now();
        self.bursts = [Some(now), Some(now), Some(now)];
        self.focused_branch = None;
        cx.emit(OutreachBranchesEvent::TriggerAll);
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

impl Render for OutreachBranches {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();
        let elapsed = now.saturating_duration_since(self.started_at).as_secs_f32();

        if self.loading {
            let phase = (elapsed / 1.4).fract();
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .h(self.height.unwrap_or(px(300.0)))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(22.0))
                .child(pro_skeleton_bar(2.0, 64.0, 1.0, phase, cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(28.0))
                        .child(pro_skeleton_bar(68.0, 68.0, 14.0, phase, cx))
                        .child(pro_skeleton_bar(78.0, 78.0, 16.0, phase, cx))
                        .child(pro_skeleton_bar(68.0, 68.0, 14.0, phase, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::GitBranch,
                    tr(cx, "No outreach channels", "暂无外发渠道"),
                    tr(
                        cx,
                        "Connect messaging destinations to preview fan-out routing.",
                        "连接消息目标应用以预览多路分发。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let animate_loop = self.playing && !reduced;
        let mut any_burst_active = false;
        let mut branch_states: [(Option<f32>, Option<f32>, f32); 3] = [(None, None, 0.0); 3];

        for b in 0..3 {
            if let Some(t0) = self.bursts[b] {
                let age = now.saturating_duration_since(t0).as_secs_f32();
                if age < 2.6 && !reduced {
                    any_burst_active = true;
                    branch_states[b] = branch_phase_state(age / LOOP_SECS);
                    continue;
                } else {
                    self.bursts[b] = None;
                }
            }
            if animate_loop {
                let shifted = (elapsed - STAGGER_SECS[b]) / LOOP_SECS;
                if shifted >= 0.0 {
                    branch_states[b] = branch_phase_state(shifted);
                }
            }
        }

        if animate_loop || any_burst_active {
            w.request_animation_frame();
        }

        let aria_label: SharedString = self.label.clone().unwrap_or_else(|| {
            tr(
                cx,
                "Lines branching out to Gmail, Slack and Notion tiles",
                "分支连接至 Gmail、Slack 与 Notion 卡片的动态流向插画",
            )
            .into()
        });

        let active_focus = self.hovered_branch.or(self.focused_branch);
        let mut illums = [0.0f32; 3];
        let mut downs = [0.0f32; 3];
        for b in 0..3 {
            let target_illum = if self.hovered_branch == Some(b) || self.source_hovered {
                1.0
            } else if self.focused_branch == Some(b) {
                0.85
            } else if self.bursts[b].is_some() {
                0.65
            } else {
                0.0
            };
            self.branch_illum[b].duration(target_illum, 0.18, cx);
            illums[b] = self.branch_illum[b].get(w, cx).clamp(0., 1.);
            downs[b] = self.tile_press[b].get(w, cx).clamp(0., 1.);
        }
        self.apex_illum
            .duration(if self.source_hovered { 1.0 } else { 0.0 }, 0.18, cx);
        let apex_illum_v = self.apex_illum.get(w, cx).clamp(0., 1.);
        let apex_down_v = self.apex_press.get(w, cx).clamp(0., 1.);
        let accents = [
            self.channels[0].accent,
            self.channels[1].accent,
            self.channels[2].accent,
        ];

        let size_cell = self.measured_size.clone();
        let (meas_w, meas_h) = size_cell.get();
        let resolved_h = self
            .height
            .unwrap_or_else(|| px((meas_w * (2.0 / 3.0)).clamp(220.0, 420.0)));
        let eff_h = f32::from(resolved_h).max(meas_h.min(220.0));
        let scale = (meas_w / ART_W).min(eff_h / ART_H).max(0.5);
        let stage_w = ART_W * scale;
        let stage_h = ART_H * scale;

        let dark = p.dark;
        let base_line_col: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.14)
        } else {
            rgb(0xe4e7ec).into()
        };

        let canvas_layer = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 4.0 || bh <= 4.0 {
                    return;
                }
                let s = (bw / ART_W).min(bh / ART_H);
                let ox = f32::from(bounds.left()) + (bw - ART_W * s) * 0.5;
                let oy = f32::from(bounds.top()) + (bh - ART_H * s) * 0.5;
                let map_pt =
                    |ax: f32, ay: f32| -> Point<Pixels> { point(px(ox + ax * s), px(oy + ay * s)) };

                // 1. Base branching curves with `Slide::snappy` illumination & tint
                for b in 0..3 {
                    let pts = sample_branch_line(b, 44);
                    let illum = illums[b];
                    let dim = match active_focus {
                        Some(fb) if fb != b && illum < 0.05 => 0.48,
                        _ => 1.0,
                    };
                    let stroke_w = ((1.2 + 0.9 * illum) * s).max(1.0);
                    let mut pb = PathBuilder::stroke(px(stroke_w));
                    for (idx, &(ax, ay)) in pts.iter().enumerate() {
                        let pt = map_pt(ax, ay);
                        if idx == 0 {
                            pb.move_to(pt);
                        } else {
                            pb.line_to(pt);
                        }
                    }
                    if let Ok(path) = pb.build() {
                        let base_op = (base_line_col.a * dim).clamp(0.05, 0.42);
                        let lit_col = pro_mix_color(
                            base_line_col.opacity(base_op),
                            accents[b].opacity(if dark { 0.65 } else { 0.78 }),
                            illum * 0.85,
                        );
                        window.paint_path(path, lit_col);
                    }
                }

                // 2. Smooth source apex halo on hover / press
                if apex_illum_v > 0.01 || apex_down_v > 0.01 {
                    let apex = map_pt(160.0, 52.0);
                    let r = px((4.0 + 2.8 * apex_illum_v - 1.0 * apex_down_v) * s);
                    window.paint_quad(
                        fill(
                            Bounds::new(point(apex.x - r, apex.y - r), size(r * 2.0, r * 2.0)),
                            rgb(0xb98ee5).opacity(
                                (0.36 * apex_illum_v + 0.22 * apex_down_v).clamp(0., 0.65),
                            ),
                        )
                        .corner_radii(r),
                    );
                }

                // 3. Center Slack tile bottom pastel rim highlight (`M128 170 A14 14 0 0 0 142 184 L178 184 A14 14 0 0 0 192 170`)
                let mut rim_pts = Vec::with_capacity(28);
                for k in 0..=8 {
                    let a = std::f32::consts::PI - (k as f32 / 8.0) * std::f32::consts::FRAC_PI_2;
                    rim_pts.push((142.0 + 14.0 * a.cos(), 170.0 + 14.0 * a.sin()));
                }
                for k in 1..=10 {
                    let u = k as f32 / 10.0;
                    rim_pts.push((142.0 + 36.0 * u, 184.0));
                }
                for k in 1..=8 {
                    let a = std::f32::consts::FRAC_PI_2
                        - (k as f32 / 8.0) * std::f32::consts::FRAC_PI_2;
                    rim_pts.push((178.0 + 14.0 * a.cos(), 170.0 + 14.0 * a.sin()));
                }
                for i in 0..(rim_pts.len() - 1) {
                    let u = ((rim_pts[i].0 + rim_pts[i + 1].0) * 0.5 - 128.0) / 64.0;
                    let mut seg = PathBuilder::stroke(px((0.75 * s).max(0.75)));
                    seg.move_to(map_pt(rim_pts[i].0, rim_pts[i].1));
                    seg.line_to(map_pt(rim_pts[i + 1].0, rim_pts[i + 1].1));
                    if let Ok(path) = seg.build() {
                        window.paint_path(path, center_edge_color(u, dark));
                    }
                }

                // 4. Flowing colored light pulses & tile perimeter wraps
                for (b, &(line_prog, wrap_prog, _)) in branch_states.iter().enumerate() {
                    let focus_scale = match active_focus {
                        Some(fb) if fb != b => 0.35,
                        _ => 1.0,
                    };
                    if let Some(lp) = line_prog {
                        let pts = sample_branch_line(b, 48);
                        paint_comet_along(
                            &pts,
                            lp,
                            0.82 * focus_scale,
                            (1.15 * s).max(1.0),
                            &map_pt,
                            window,
                        );
                    }
                    if let Some(wp) = wrap_prog {
                        let tile = TILE_SPECS[b];
                        let left_pts = sample_tile_half_perimeter(tile, true);
                        let right_pts = sample_tile_half_perimeter(tile, false);
                        paint_comet_along(
                            &left_pts,
                            wp,
                            0.52 * focus_scale,
                            (0.95 * s).max(0.85),
                            &map_pt,
                            window,
                        );
                        paint_comet_along(
                            &right_pts,
                            wp,
                            0.52 * focus_scale,
                            (0.95 * s).max(0.85),
                            &map_pt,
                            window,
                        );
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        let glow_img = ambient_pastel_glow_svg();
        let tile_bg: Hsla = if dark {
            rgb(0x232326).into()
        } else {
            rgb(0xffffff).into()
        };
        let tile_border: Hsla = if dark {
            hsla(0.0, 0.0, 1.0, 0.08)
        } else {
            rgb(0xe4e7ec).into()
        };
        let sliver_col: Hsla = if dark {
            rgb(0x1a1a1c).into()
        } else {
            rgb(0xeaecef).into()
        };
        let shadow_col: Hsla = if dark {
            hsla(0.0, 0.0, 0.0, 0.40)
        } else {
            hsla(0.0, 0.0, 0.0, 0.06)
        };

        let mut art_stage = div().relative().w(px(stage_w)).h(px(stage_h)).flex_none();

        // Ambient center Slack pastel under-glow + landing blooms on each tile
        let center_glow_w = 136.0 * scale;
        let center_glow_h = 58.0 * scale;
        let center_glow_alpha =
            (if dark { 0.72 } else { 0.92 } + 0.18 * branch_states[1].2).clamp(0.0, 1.0);
        art_stage = art_stage.child(
            div()
                .absolute()
                .left(px(160.0 * scale - center_glow_w * 0.5))
                .top(px(184.0 * scale - center_glow_h * 0.46))
                .w(px(center_glow_w))
                .h(px(center_glow_h))
                .opacity(center_glow_alpha)
                .child(
                    img(glow_img.clone())
                        .w(px(center_glow_w))
                        .h(px(center_glow_h)),
                ),
        );

        for b in [0usize, 2usize] {
            let bloom = branch_states[b].2 + if active_focus == Some(b) { 0.45 } else { 0.0 };
            if bloom > 0.02 {
                let spec = TILE_SPECS[b];
                let gw = 104.0 * scale;
                let gh = 44.0 * scale;
                let cx_art = spec.x + spec.side * 0.5;
                let cy_art = spec.y + spec.side;
                art_stage = art_stage.child(
                    div()
                        .absolute()
                        .left(px(cx_art * scale - gw * 0.5))
                        .top(px(cy_art * scale - gh * 0.45))
                        .w(px(gw))
                        .h(px(gh))
                        .opacity((bloom * if dark { 0.55 } else { 0.72 }).clamp(0.0, 0.85))
                        .child(img(glow_img.clone()).w(px(gw)).h(px(gh))),
                );
            }
        }

        // Three tactile app tiles positioned on the 320x240 artboard
        for b in 0..3 {
            let spec = TILE_SPECS[b];
            let ch = &self.channels[b];
            let illum = illums[b];
            let down = downs[b];
            let lift = (-2.0 * illum * (1.0 - down) + 1.6 * down) * scale;
            let press_inset = 1.1 * down * scale;
            let tx = spec.x * scale + press_inset;
            let ty = spec.y * scale + lift + press_inset;
            let t_side = (spec.side * scale - press_inset * 2.0).max(16.0);
            let t_rx = spec.rx * scale;
            let mark_w = (if b == 1 { 28.0 } else { 30.0 }) * scale * (1.0 - 0.06 * down);
            let land_glow = branch_states[b].2;
            let border_mix = (land_glow * 0.75 + illum * 0.72 + down * 0.35).clamp(0.0, 1.0);
            let border_c = if border_mix > 0.02 {
                pro_mix_color(
                    tile_border,
                    ch.accent.opacity(if dark { 0.58 } else { 0.78 }),
                    border_mix,
                )
            } else {
                tile_border
            };

            // Soft drop shadow beneath tile
            art_stage = art_stage.child(
                div()
                    .absolute()
                    .left(px(tx + 1.0 * scale))
                    .top(px(ty + (3.0 - 1.2 * down) * scale))
                    .size(px((t_side - 2.0 * scale).max(4.0)))
                    .rounded(px(t_rx))
                    .bg(shadow_col),
            );

            // Tile body with inner bottom bevel lip + crisp vector mark
            art_stage = art_stage.child(
                div()
                    .id(pro_eid(&self.id, format!("tile-{b}")))
                    .role(Role::Button)
                    .aria_label(ch.name.clone())
                    .tab_index(0)
                    .absolute()
                    .left(px(tx))
                    .top(px(ty))
                    .size(px(t_side))
                    .rounded(px(t_rx))
                    .bg(sliver_col)
                    .border_1()
                    .border_color(border_c)
                    .overflow_hidden()
                    .cursor_pointer()
                    .focus_visible(move |s| s.border_color(p.focus))
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        let next = if *inside { Some(b) } else { None };
                        if *inside {
                            if this.hovered_branch != Some(b) {
                                this.hovered_branch = Some(b);
                                this.branch_illum[b].duration(1.0, 0.18, cx);
                                // Immediately launch a traveling comet packet along this branch on hover!
                                this.bursts[b] = Some(cx.background_executor().now());
                                cx.emit(OutreachBranchesEvent::FocusBranch(Some(b)));
                                cx.notify();
                            }
                        } else if this.hovered_branch == Some(b) {
                            this.hovered_branch = next;
                            this.branch_illum[b].duration(0.0, 0.18, cx);
                            this.tile_press[b].duration(0., 0.14, cx);
                            cx.emit(OutreachBranchesEvent::FocusBranch(this.focused_branch));
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.tile_press[b].duration(1., 0.08, cx);
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.tile_press[b].duration(0., 0.14, cx);
                            cx.notify();
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.tile_press[b].duration(0., 0.14, cx);
                            cx.notify();
                        }),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.trigger_branch(b, cx);
                    }))
                    .child(
                        div()
                            .w_full()
                            .h(px((t_side - (2.4 - 1.4 * down) * scale).max(1.0)))
                            .rounded(px((t_rx - 0.8 * scale).max(2.0)))
                            .bg(tile_bg)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(outreach_channel_mark(&ch.id, mark_w, dark, cx)),
                    ),
            );
        }

        // Interactive source apex button at `(160, 52)` to trigger all three branches
        let apex_hit = (22.0 * scale).max(18.0);
        art_stage = art_stage.child(
            div()
                .id(pro_eid(&self.id, "apex-trigger"))
                .role(Role::Button)
                .aria_label(tr(cx, "Send pulse to all channels", "向所有渠道发送脉冲"))
                .tab_index(0)
                .absolute()
                .left(px(160.0 * scale - apex_hit * 0.5))
                .top(px(52.0 * scale - apex_hit * 0.5))
                .size(px(apex_hit))
                .rounded_full()
                .cursor_pointer()
                .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                    if this.source_hovered != *inside {
                        this.source_hovered = *inside;
                        if !*inside {
                            this.apex_press.duration(0., 0.14, cx);
                        }
                        cx.notify();
                    }
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.apex_press.duration(1., 0.08, cx);
                        cx.notify();
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.apex_press.duration(0., 0.14, cx);
                        cx.notify();
                    }),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.trigger_all(cx);
                })),
        );

        let measure_sensor = canvas(
            move |bounds, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw > 8.0 && bh > 8.0 {
                    let prev = size_cell.get();
                    if (bw - prev.0).abs() > 1.0 || (bh - prev.1).abs() > 1.0 {
                        size_cell.set((bw, bh));
                        window.request_animation_frame();
                    }
                }
            },
            |_, _, _, _| (),
        )
        .absolute()
        .inset_0();

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .role(Role::Image)
            .aria_label(aria_label)
            .relative()
            .w_full()
            .h(resolved_h)
            .overflow_hidden()
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(14.)).border_color(p.focus))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "right" | "down" => {
                        let next = this.focused_branch.map_or(0, |b| (b + 1) % 3);
                        this.set_focused_branch(Some(next), cx);
                        this.bursts[next] = Some(cx.background_executor().now());
                        cx.stop_propagation();
                    }
                    "left" | "up" => {
                        let prev = this.focused_branch.map_or(2, |b| (b + 2) % 3);
                        this.set_focused_branch(Some(prev), cx);
                        this.bursts[prev] = Some(cx.background_executor().now());
                        cx.stop_propagation();
                    }
                    "enter" | "space" | " " => {
                        if let Some(b) = this.focused_branch {
                            this.trigger_branch(b, cx);
                        } else {
                            this.trigger_all(cx);
                        }
                        cx.stop_propagation();
                    }
                    "escape" => {
                        this.set_focused_branch(None, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(measure_sensor)
            .child(art_stage.child(canvas_layer))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ART_H, ART_W, OutreachBranches, OutreachBranchesEvent, TILE_SPECS, branch_phase_state,
        cumulative_lengths, default_outreach_channels, pulse_bezier, pulse_gradient_color,
        sample_branch_line, sample_tile_half_perimeter,
    };
    use gpui::{
        MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext, point, px, size,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};

    #[test]
    fn default_channels_and_branch_endpoints_align_with_tiles() {
        let channels = default_outreach_channels();
        assert_eq!(channels[0].id.as_ref(), "gmail");
        assert_eq!(channels[1].id.as_ref(), "slack");
        assert_eq!(channels[2].id.as_ref(), "notion");

        for (b, spec) in TILE_SPECS.iter().enumerate() {
            let line = sample_branch_line(b, 32);
            let first = line[0];
            let last = *line.last().expect("non-empty branch polyline");
            assert!((first.0 - 160.0).abs() < 1e-3 && (first.1 - 52.0).abs() < 1e-3);
            assert!((last.0 - (spec.x + spec.side * 0.5)).abs() < 1e-2);
            assert!((last.1 - spec.y).abs() < 1e-2);

            let left_wrap = sample_tile_half_perimeter(*spec, true);
            let right_wrap = sample_tile_half_perimeter(*spec, false);
            let (_, len_l) = cumulative_lengths(&left_wrap);
            let (_, len_r) = cumulative_lengths(&right_wrap);
            assert!((len_l - len_r).abs() < 0.5);
            let end_l = *left_wrap.last().expect("left perimeter points");
            assert!((end_l.0 - (spec.x + spec.side * 0.5)).abs() < 1e-2);
            assert!((end_l.1 - (spec.y + spec.side)).abs() < 1e-2);
        }
    }

    #[test]
    fn pulse_phase_and_bezier_handoff_from_branch_to_tile_wrap() {
        assert!((pulse_bezier(0.0) - 0.0).abs() < 1e-4);
        assert!((pulse_bezier(0.5) - 0.5).abs() < 2e-2);
        assert!((pulse_bezier(1.0) - 1.0).abs() < 1e-4);

        let (line_early, wrap_early, _) = branch_phase_state(0.10);
        assert!(line_early.is_some() && wrap_early.is_none());

        let (line_handoff, wrap_handoff, glow_mid) = branch_phase_state(0.32);
        assert!(line_handoff.is_some() && wrap_handoff.is_some());
        assert!(glow_mid > 0.4);

        let (line_late, wrap_late, glow_rest) = branch_phase_state(0.85);
        assert!(line_late.is_none() && wrap_late.is_none());
        assert_eq!(glow_rest, 0.0);

        let c_left = pulse_gradient_color(40.0);
        let c_right = pulse_gradient_color(280.0);
        assert_ne!(c_left, c_right);
    }

    #[gpui::test]
    fn hover_and_click_on_channel_tile_illuminates_bezier_and_fires_comet_packet(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let exec = cx.executor();
        let events: Rc<RefCell<Vec<OutreachBranchesEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let ev_sink = events.clone();

        let (view, vcx) = cx.add_window_view(|_, cx| {
            let ob = OutreachBranches::new("ob-test", cx).with_playing(false);
            cx.subscribe_self(move |_, ev: &OutreachBranchesEvent, _| {
                ev_sink.borrow_mut().push(ev.clone());
            })
            .detach();
            ob
        });
        vcx.simulate_resize(size(px(480.), px(320.)));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        // Compute Gmail tile (`branch = 0`) center on the centered `320x240` scaled stage
        let gmail_pt = vcx.update(|_, app| {
            let (mw, mh) = view.read(app).measured_size.get();
            let s = (mw / ART_W).min(mh / ART_H).max(0.5);
            let ox = (mw - ART_W * s) * 0.5;
            let oy = (mh - ART_H * s) * 0.5;
            let spec = TILE_SPECS[0];
            point(
                px(ox + (spec.x + spec.side * 0.5) * s),
                px(oy + (spec.y + spec.side * 0.5) * s),
            )
        });

        vcx.simulate_event(MouseMoveEvent {
            position: gmail_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        exec.advance_clock(Duration::from_millis(160));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        vcx.update(|_, app| {
            let ob = view.read(app);
            assert_eq!(ob.hovered_branch, Some(0));
            assert!(ob.bursts[0].is_some());
            assert!(ob.branch_illum[0].value(app) > 0.65);
        });

        vcx.simulate_event(MouseDownEvent {
            position: gmail_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(MouseUpEvent {
            position: gmail_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        });
        vcx.run_until_parked();

        assert!(events.borrow().iter().any(|e| {
            matches!(
                e,
                OutreachBranchesEvent::TriggerBranch { index: 0, channel } if channel.as_ref() == "gmail"
            )
        }));
    }
}
