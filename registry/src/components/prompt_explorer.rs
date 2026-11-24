//! AI prompt and keyword explorer screen with drifting atmospheric hero and trend sparklines.
//!
//! Renders [`PromptExplorerScreen`] inside [`AppShell`] (`"Northwind"` workspace, active
//! `"Conversations"`), combining a multi-layer drifting atmospheric light hero, combobox
//! prompt composer with single and bulk-line analysis, live search completions, rising
//! analysis breakdown rows, and side-by-side popular and trending keyword lists.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellLayout, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProPalette, pro_eid, pro_empty_state, pro_icon_btn, pro_presence, pro_skeleton_bar,
            pro_spring_bar, pro_switch, pro_toast_banner, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        pro_icons::{ProIcon, pro_icon, pro_svg},
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

const ARROW_UP_SVG: &str = r#"<path d="M12 5.5V19" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"></path><path d="M18 11C18 11 13.5811 5.00001 12 5C10.4188 4.99999 6 11 6 11" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"></path>"#;

const ARROW_DOWN_SVG: &str = r#"<path d="M12 18.502V5.00195" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"></path><path d="M18 13.002C18 13.002 13.5811 19.0019 12 19.002C10.4188 19.002 6 13.002 6 13.002" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"></path>"#;

const CHART_RISING_SVG: &str = r#"<path d="M21 21H10C6.70017 21 5.05025 21 4.02513 19.9749C3 18.9497 3 17.2998 3 14V3" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path><path d="M7.99707 16.999C11.5286 16.999 18.9122 15.5348 18.6979 6.43269M16.4886 8.04302L18.3721 6.14612C18.5656 5.95127 18.8798 5.94981 19.0751 6.14286L20.9971 8.04302" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const CHART_FALLING_SVG: &str = r#"<path d="M21 21H10C6.70017 21 5.05025 21 4.02513 19.9749C3 18.9497 3 17.2998 3 14V3" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path><path d="M6.99707 5.99902C10.5286 5.999 17.9122 7.46328 17.6979 16.5654M15.4886 14.955L17.3721 16.8519C17.5656 17.0468 17.8798 17.0482 18.0751 16.8552L19.9971 14.955" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const HERO_TEX_W: usize = 160;
const HERO_TEX_H: usize = 88;

/// Input analysis target mode in [`PromptExplorerScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PromptAnalyseMode {
    #[default]
    Keyword,
    Url,
}

impl PromptAnalyseMode {
    /// Return both analysis modes in toolbar order.
    pub const fn all() -> [Self; 2] {
        [Self::Keyword, Self::Url]
    }

    /// Stable slug token for element ids and events.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Url => "url",
        }
    }

    /// Localized accessible tooltip label for the mode radio button.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Keyword => tr(cx, "Prompts and keywords", "提问与关键词"),
            Self::Url => tr(cx, "A page's address", "网页地址"),
        }
    }
}

/// Active direction list shown in the Trending Keywords column of [`PromptExplorerScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PromptTrendDirection {
    #[default]
    Rising,
    Falling,
}

impl PromptTrendDirection {
    /// Return both trend directions in switcher order.
    pub const fn all() -> [Self; 2] {
        [Self::Rising, Self::Falling]
    }

    /// Stable slug token for element ids.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Rising => "rising",
            Self::Falling => "falling",
        }
    }

    /// Localized accessible label for the direction switcher.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Rising => tr(cx, "Rising", "上升趋势"),
            Self::Falling => tr(cx, "Falling", "下降趋势"),
        }
    }
}

/// One prompt suggestion or keyword entry with monthly prompt volume, delta, and 12-month sparkline.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PromptKeywordItem {
    pub id: SharedString,
    pub text: SharedString,
    pub volume: u64,
    pub change: i64,
    pub trend: Vec<u64>,
}

impl PromptKeywordItem {
    /// Construct a keyword item with deterministic 12-month sparkline values derived from `(text, volume, change)`.
    pub fn new(
        id: impl Into<SharedString>,
        text: impl Into<SharedString>,
        volume: u64,
        change: i64,
    ) -> Self {
        let text = text.into();
        let trend = deterministic_trend(&text, volume, change);
        Self {
            id: id.into(),
            text,
            volume,
            change,
            trend,
        }
    }

    /// Override the 12-month trend sparkline values.
    pub fn with_trend(mut self, trend: Vec<u64>) -> Self {
        self.trend = trend;
        self
    }
}

/// Analysed prompt row returned when the user runs `"Analyse"` in [`PromptExplorerScreen`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PromptAnalysisResult {
    pub text: SharedString,
    pub volume: u64,
    pub change: i64,
    pub trend: Vec<u64>,
    /// Share of assistant answers mentioning the workspace brand (`0.0..=1.0`).
    pub share: f32,
}

impl PromptAnalysisResult {
    /// Construct one prompt analysis row.
    pub fn new(
        text: impl Into<SharedString>,
        volume: u64,
        change: i64,
        trend: Vec<u64>,
        share: f32,
    ) -> Self {
        Self {
            text: text.into(),
            volume,
            change,
            trend,
            share: share.clamp(0., 1.),
        }
    }
}

/// Typed events emitted by [`PromptExplorerScreen`].
#[derive(Clone, Debug, PartialEq)]
pub enum PromptExplorerEvent {
    Navigate(SharedString),
    ModeChanged(PromptAnalyseMode),
    BulkToggled(bool),
    TrendDirectionChanged(PromptTrendDirection),
    Analyse {
        prompts: Vec<SharedString>,
        mode: PromptAnalyseMode,
        results: Vec<PromptAnalysisResult>,
    },
    ClearAnalysis,
}

fn js_u32(val: f64) -> u32 {
    let rem = val % 4_294_967_296.0;
    let wrapped = if rem < 0.0 {
        rem + 4_294_967_296.0
    } else {
        rem
    };
    wrapped as u32
}

/// Generate the canonical 12-point monthly sparkline series for `(text, volume, change)`.
fn deterministic_trend(text: &str, volume: u64, change: i64) -> Vec<u64> {
    let mut r: u32 = 0;
    for unit in text.encode_utf16() {
        r = js_u32(f64::from(r) * 31.0 + f64::from(unit));
    }
    let mut next_unit = || -> f64 {
        r = js_u32(f64::from(r) * 1_103_515_245.0 + 12_345.0);
        f64::from(r) / 4_294_967_296.0
    };
    let vol_f = volume as f64;
    let start = (vol_f - (change as f64) * 6.0).max(1.0);
    (0..12)
        .map(|n| {
            let frac = n as f64 / 11.0;
            let jitter = (next_unit() - 0.5) * vol_f * 0.12;
            let raw = (start + (vol_f - start) * frac + jitter + 0.5).floor();
            if raw <= 0.0 { 0 } else { raw as u64 }
        })
        .collect()
}

fn trim_dot_zero(s: String) -> String {
    if let Some(stripped) = s.strip_suffix(".0") {
        stripped.to_owned()
    } else {
        s
    }
}

/// Format an integer volume (`205_000 -> "205k"`, `88_100 -> "88.1k"`, `2_100 -> "2.1k"`, `1_200_000 -> "1.2M"`).
pub fn format_prompt_volume(value: i64) -> String {
    let abs = value.unsigned_abs() as f64;
    if abs >= 999_950_000.0 {
        format!(
            "{}B",
            trim_dot_zero(format!("{:.1}", abs / 1_000_000_000.0))
        )
    } else if abs >= 999_950.0 {
        format!("{}M", trim_dot_zero(format!("{:.1}", abs / 1_000_000.0)))
    } else if abs >= 100_000.0 {
        format!("{:.0}k", abs / 1_000.0)
    } else if abs >= 1_000.0 {
        format!("{}k", trim_dot_zero(format!("{:.1}", abs / 1_000.0)))
    } else {
        format!("{}", abs.round() as u64)
    }
}

fn format_signed_delta(change: i64) -> String {
    if change > 0 {
        format!("+{}", format_prompt_volume(change))
    } else if change < 0 {
        format!("−{}", format_prompt_volume(change))
    } else {
        "0".to_owned()
    }
}

fn format_share_pct(share: f32) -> String {
    format!("{}%", (share.clamp(0., 1.) * 100.0).round() as u32)
}

fn synthesize_prompt_result(query: &str, catalog: &[PromptKeywordItem]) -> PromptAnalysisResult {
    let lower = query.trim().to_ascii_lowercase();
    let matched = catalog
        .iter()
        .find(|item| item.text.to_ascii_lowercase() == lower);
    let mut hash: u32 = 0;
    for unit in query.trim().encode_utf16() {
        hash = hash.wrapping_mul(33).wrapping_add(u32::from(unit));
    }
    let volume = matched
        .map(|m| m.volume)
        .unwrap_or_else(|| 800 + u64::from(hash % 64_000));
    let change = matched
        .map(|m| m.change)
        .unwrap_or_else(|| i64::from((hash >> 5) % 4_000) - 1_400);
    let trend = matched
        .map(|m| m.trend.clone())
        .unwrap_or_else(|| deterministic_trend(query.trim(), volume, change));
    let share = (((hash >> 9) % 70) as f32) / 100.0 + 0.04;
    PromptAnalysisResult::new(query.trim().to_owned(), volume, change, trend, share)
}

fn rank_prompt_suggestions(
    query: &str,
    mode: PromptAnalyseMode,
    catalog: &[PromptKeywordItem],
) -> Vec<PromptKeywordItem> {
    let trimmed = query.trim();
    if trimmed.is_empty() || mode == PromptAnalyseMode::Url {
        return Vec::new();
    }
    let q_lower = trimmed.to_ascii_lowercase();
    let mut prefix = Vec::new();
    let mut contains = Vec::new();
    for item in catalog {
        let t_lower = item.text.to_ascii_lowercase();
        if t_lower.starts_with(&q_lower) {
            prefix.push(item.clone());
        } else if t_lower.contains(&q_lower) {
            contains.push(item.clone());
        }
    }
    prefix.into_iter().chain(contains).take(5).collect()
}

fn default_prompt_catalog() -> Vec<PromptKeywordItem> {
    let kw = |s: &'static str, vol: u64, chg: i64| PromptKeywordItem::new(s, s, vol, chg);
    vec![
        kw("northwind ai visibility", 549_400, 76_200),
        kw("northwind ai", 35_000, -2_700),
        kw("northwind against semrush", 6_400, 1_500),
        kw("northwind pricing", 18_900, 2_100),
        kw("northwind vs semrush", 195_000, 2_100),
        kw("northwind review", 12_300, 900),
        kw("northwind alternatives", 9_800, -400),
        kw("generative ai tools", 135_000, 2_100),
        kw("generative ai", 97_000, 1_800),
        kw("generative engine optimization", 41_200, 6_900),
        kw("geo", 133_000, -2_100),
        kw("geo vs seo", 28_400, 3_300),
        kw("aeo", 120_000, 1_400),
        kw("aeo tools", 22_700, 4_100),
        kw("answer engine optimization", 58_600, 5_200),
        kw("ai search visibility", 47_300, 7_800),
        kw("ai content optimization", 64_000, 6_400),
        kw("ai overviews tracking", 15_600, 2_600),
        kw("google ai overviews", 70_000, 2_900),
        kw("boost ai visibility", 72_000, 3_400),
        kw("best seo tools 2026", 88_100, -5_600),
        kw("seo", 42_000, 7_100),
        kw("seo vs aeo", 108_000, -2_100),
        kw("chatgpt brand mentions", 19_800, 4_400),
        kw("chatgpt rank tracking", 11_400, 2_300),
        kw("perplexity citations", 8_900, 1_900),
    ]
}

fn default_popular_keywords() -> Vec<PromptKeywordItem> {
    let kw = |s: &'static str, vol: u64, chg: i64| PromptKeywordItem::new(s, s, vol, chg);
    vec![
        kw("Northwind ai", 205_000, 2_100),
        kw("northwind vs semrush", 195_000, 2_100),
        kw("northwind ai visibility", 147_000, 2_100),
        kw("geo", 133_000, -2_100),
        kw("aeo", 131_000, 2_100),
        kw("seo vs aeo", 108_000, -2_100),
        kw("boost ai visibility", 114_000, 2_100),
    ]
}

fn default_rising_keywords() -> Vec<PromptKeywordItem> {
    let kw = |s: &'static str, vol: u64, chg: i64| PromptKeywordItem::new(s, s, vol, chg);
    vec![
        kw("generative ai tools", 135_000, 2_100),
        kw("aeo", 120_000, 1_400),
        kw("generative ai", 97_000, 1_800),
        kw("boost ai visibility", 72_000, 3_400),
        kw("google ai overviews", 70_000, 2_900),
        kw("ai content optimization", 64_000, 6_400),
        kw("seo", 42_000, 7_100),
    ]
}

fn default_falling_keywords() -> Vec<PromptKeywordItem> {
    let kw = |s: &'static str, vol: u64, chg: i64| PromptKeywordItem::new(s, s, vol, chg);
    vec![
        kw("best seo tools 2026", 88_100, -5_600),
        kw("geo", 133_000, -2_100),
        kw("seo vs aeo", 108_000, -2_100),
        kw("northwind ai", 35_000, -2_700),
        kw("keyword research tool", 51_200, -1_900),
        kw("backlink checker", 33_800, -1_400),
        kw("rank tracker", 27_500, -1_100),
    ]
}

fn build_northwind_sidebar(cx: &App) -> Vec<ShellNavGroup> {
    vec![ShellNavGroup::primary(vec![
        ShellNavItem::new("overview", tr(cx, "Overview", "概览"), ProIcon::Grid),
        ShellNavItem::new(
            "answer-engine-insights",
            tr(cx, "Answer Engine Insights", "答案引擎洞察"),
            ProIcon::Bolt,
        ),
        ShellNavItem::new(
            "my-website",
            tr(cx, "My Website", "我的网站"),
            ProIcon::Globe,
        ),
        ShellNavItem::new(
            "conversations",
            tr(cx, "Conversations", "对话探索"),
            ProIcon::Thread,
        ),
        ShellNavItem::new("actions", tr(cx, "Actions", "自动化动作"), ProIcon::Refresh),
    ])]
}

struct HeroGlowSurface {
    dark: Option<bool>,
    panel_rgb: [u8; 3],
    static_pose: bool,
    last_step: u32,
    image: Option<Arc<RenderImage>>,
}

impl HeroGlowSurface {
    fn new() -> Self {
        Self {
            dark: None,
            panel_rgb: [0, 0, 0],
            static_pose: false,
            last_step: u32::MAX,
            image: None,
        }
    }

    fn frame(
        &mut self,
        dark: bool,
        panel_color: Hsla,
        elapsed_secs: f32,
        static_pose: bool,
        window: &mut Window,
    ) -> Arc<RenderImage> {
        let rgba: Rgba = panel_color.into();
        let panel_rgb = [
            (rgba.r.clamp(0., 1.) * 255.).round() as u8,
            (rgba.g.clamp(0., 1.) * 255.).round() as u8,
            (rgba.b.clamp(0., 1.) * 255.).round() as u8,
        ];
        // Quantize to ~30 Hz steps so drifting light stays silky without redundant allocations.
        let step = if static_pose {
            0
        } else {
            (elapsed_secs * 30.0).floor() as u32
        };
        if self.dark == Some(dark)
            && self.panel_rgb == panel_rgb
            && self.static_pose == static_pose
            && self.last_step == step
            && let Some(existing) = &self.image
        {
            return existing.clone();
        }

        let rendered = Arc::new(rasterize_hero_glow(
            dark,
            panel_rgb,
            if static_pose { 0.0 } else { elapsed_secs },
        ));
        if let Some(prev) = self.image.replace(rendered.clone()) {
            let _ = window.drop_image(prev);
        }
        self.dark = Some(dark);
        self.panel_rgb = panel_rgb;
        self.static_pose = static_pose;
        self.last_step = step;
        rendered
    }
}

fn blend_over(dst: [f32; 3], src: [f32; 3], alpha: f32) -> [f32; 3] {
    let a = alpha.clamp(0., 1.);
    let inv = 1.0 - a;
    [
        dst[0] * inv + src[0] * a,
        dst[1] * inv + src[1] * a,
        dst[2] * inv + src[2] * a,
    ]
}

fn radial_falloff(u: f32, v: f32, cx: f32, cy: f32, rx: f32, ry: f32, stop: f32) -> f32 {
    let dx = (u - cx) / rx.max(0.01);
    let dy = (v - cy) / ry.max(0.01);
    let dist = (dx * dx + dy * dy).sqrt() / stop.max(0.01);
    let t = (1.0 - dist).clamp(0., 1.);
    t * t * (3.0 - 2.0 * t)
}

fn rasterize_hero_glow(dark: bool, panel_rgb: [u8; 3], elapsed: f32) -> RenderImage {
    // Three smooth incommensurate loops matching the 9s / 11s / 13s alternate drift cycles.
    let wa = (elapsed / 9.0) * TAU * 0.5;
    let wb = (elapsed / 11.0) * TAU * 0.5;
    let wc = (elapsed / 13.0) * TAU * 0.5;

    let ax = 0.10 * wa.sin();
    let ay = -0.045 * wa.cos();
    let ascale = 1.0 + 0.06 * (wa * 1.3).sin();

    let bx = -0.08 * wb.sin();
    let by = 0.04 * wb.cos();
    let brot = 0.09 * wb.sin();

    let cx_off = -0.09 * wc.cos();
    let cy_off = 0.045 * wc.sin();

    let page = [
        panel_rgb[0] as f32 / 255.0,
        panel_rgb[1] as f32 / 255.0,
        panel_rgb[2] as f32 / 255.0,
    ];

    let img_buf = RgbaImage::from_fn(HERO_TEX_W as u32, HERO_TEX_H as u32, |x, y| {
        let u = (x as f32 + 0.5) / HERO_TEX_W as f32;
        let v = (y as f32 + 0.5) / HERO_TEX_H as f32;

        let ua = 0.5 + (u - 0.5 - ax) / ascale;
        let va = 0.5 + (v - 0.5 - ay) / ascale;

        let du = u - 0.5 - bx;
        let dv = v - 0.5 - by;
        let cos_r = brot.cos();
        let sin_r = brot.sin();
        let ub = 0.5 + du * cos_r - dv * sin_r;
        let vb = 0.5 + du * sin_r + dv * cos_r;

        let uc = u - cx_off;
        let vc = v - cy_off;

        let hero_rgb = if dark {
            // Base #131527 with cobalt/cyan/lavender drifting nocturnal layers
            let mut col = [19.0 / 255.0, 21.0 / 255.0, 41.0 / 255.0];
            let base_lin = [
                (20.0 + 5.0 * va) / 255.0,
                (22.0 + 6.0 * va) / 255.0,
                (44.0 + 6.0 * va) / 255.0,
            ];
            col = blend_over(col, base_lin, 0.80);

            let blue_l = radial_falloff(ua, va, 0.14, 0.58, 0.70, 0.88, 0.76);
            col = blend_over(
                col,
                [68.0 / 255.0, 120.0 / 255.0, 232.0 / 255.0],
                0.52 * blue_l,
            );

            let cyan_r = radial_falloff(ua, va, 0.86, 0.56, 0.62, 0.78, 0.76);
            col = blend_over(
                col,
                [58.0 / 255.0, 166.0 / 255.0, 204.0 / 255.0],
                0.32 * cyan_r,
            );

            // Diagonal atmospheric silk beams (screen-like lift on left/right flanks)
            let flank = ((ub - 0.50).abs() * 1.65).clamp(0.15, 1.0);
            let silk_wave =
                0.5 + 0.5 * ((ub * 4.2 + vb * 2.1).sin() * 0.6 + (ub * 7.5 - vb * 3.4).cos() * 0.4);
            let silk = flank * silk_wave * (1.0 - (vb - 0.38).abs() * 1.1).clamp(0., 1.);
            col = [
                (col[0] + 0.12 * silk).clamp(0., 1.),
                (col[1] + 0.19 * silk).clamp(0., 1.),
                (col[2] + 0.34 * silk).clamp(0., 1.),
            ];

            let lav_br = radial_falloff(uc, vc, 0.85, 0.82, 1.15, 1.15, 0.68);
            col = blend_over(
                col,
                [118.0 / 255.0, 90.0 / 255.0, 220.0 / 255.0],
                0.34 * lav_br,
            );

            let top_tl = radial_falloff(uc, vc, 0.14, 0.24, 1.10, 1.10, 0.62);
            col = blend_over(
                col,
                [62.0 / 255.0, 96.0 / 255.0, 196.0 / 255.0],
                0.30 * top_tl,
            );
            col
        } else {
            // Base #eef2ff with luminous sky-blue, cyan, and soft lavender drifting layers
            let mut col = [232.0 / 255.0, 240.0 / 255.0, 1.0];
            let white_lin = [1.0 - 0.02 * va, 1.0 - 0.015 * va, 1.0 - 0.005 * va];
            col = blend_over(col, white_lin, 0.52);

            let sky_l = radial_falloff(ua, va, 0.12, 0.62, 0.72, 0.92, 0.76);
            col = blend_over(
                col,
                [104.0 / 255.0, 166.0 / 255.0, 244.0 / 255.0],
                0.56 * sky_l,
            );

            let cyan_r = radial_falloff(ua, va, 0.86, 0.58, 0.64, 0.82, 0.76);
            col = blend_over(
                col,
                [118.0 / 255.0, 212.0 / 255.0, 238.0 / 255.0],
                0.44 * cyan_r,
            );

            // Soft diagonal atmospheric silk ribbons with calm bright center behind heading
            let flank = ((ub - 0.50).abs() * 1.75).clamp(0.18, 1.0);
            let silk_wave = 0.5
                + 0.5 * ((ub * 4.4 + vb * 2.0).sin() * 0.65 + (ub * 7.2 - vb * 3.1).cos() * 0.35);
            let silk = flank * silk_wave * (1.0 - (vb - 0.42).abs() * 1.05).clamp(0., 1.);
            col = blend_over(
                col,
                [154.0 / 255.0, 192.0 / 255.0, 250.0 / 255.0],
                0.36 * silk,
            );

            let lav_br = radial_falloff(uc, vc, 0.85, 0.84, 1.18, 1.18, 0.66);
            col = blend_over(col, [218.0 / 255.0, 200.0 / 255.0, 1.0], 0.46 * lav_br);

            let ice_tl = radial_falloff(uc, vc, 0.14, 0.24, 1.12, 1.12, 0.62);
            col = blend_over(col, [204.0 / 255.0, 229.0 / 255.0, 1.0], 0.48 * ice_tl);
            col
        };

        // Smooth bottom mask: 1.0 from v=0..0.50, easing cleanly to 0.0 at v=0.94
        let mask = if v <= 0.50 {
            1.0
        } else if v >= 0.94 {
            0.0
        } else {
            let t = ((0.94 - v) / 0.44).clamp(0., 1.);
            t * t * (3.0 - 2.0 * t)
        };
        let out = blend_over(page, hero_rgb, mask);

        // GPUI RenderImage expects BGRA byte ordering inside image::Rgba
        image::Rgba([
            (out[2].clamp(0., 1.) * 255.0).round() as u8,
            (out[1].clamp(0., 1.) * 255.0).round() as u8,
            (out[0].clamp(0., 1.) * 255.0).round() as u8,
            255,
        ])
    });
    RenderImage::new(vec![Frame::new(img_buf)])
}

fn render_mini_sparkline(values: &[u64], width: f32, height: f32, color: Hsla) -> AnyElement {
    let vals: Vec<f32> = values.iter().map(|&v| v as f32).collect();
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            if vals.len() < 2 || w <= 2. || h <= 2. {
                return;
            }
            let mut min_v = f32::INFINITY;
            let mut max_v = f32::NEG_INFINITY;
            for &v in &vals {
                min_v = min_v.min(v);
                max_v = max_v.max(v);
            }
            let span = max_v - min_v;
            let mut builder = PathBuilder::stroke(px(1.4));
            for (i, &v) in vals.iter().enumerate() {
                let x = bounds.left() + px(1.0 + (i as f32 / (vals.len() - 1) as f32) * (w - 2.0));
                let y_off = if span.abs() < 1e-4 {
                    h * 0.5
                } else {
                    h - 1.5 - ((v - min_v) / span) * (h - 3.0)
                };
                let pt = point(x, bounds.top() + px(y_off));
                if i == 0 {
                    builder.move_to(pt);
                } else {
                    builder.line_to(pt);
                }
            }
            if let Ok(path) = builder.build() {
                window.paint_path(path, color);
            }
        },
    )
    .w(px(width))
    .h(px(height))
    .flex_none()
    .into_any_element()
}

/// AI Search / Prompt Explorer workspace screen (`prompt-explorer`).
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::prompt_explorer::{PromptExplorerEvent, PromptExplorerScreen};
///
/// # fn mount(cx: &mut gpui::App) {
/// let screen = cx.new(|cx| PromptExplorerScreen::new("prompt-explorer", cx));
/// cx.subscribe(&screen, |_, ev: &PromptExplorerEvent, _| {
///     let _ = ev;
/// })
/// .detach();
/// # }
/// ```
pub struct PromptExplorerScreen {
    id: ElementId,
    standalone: bool,
    layout: AppShellLayout,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    custom_title: Option<SharedString>,
    prompts: Vec<PromptKeywordItem>,
    popular: Vec<PromptKeywordItem>,
    rising: Vec<PromptKeywordItem>,
    falling: Vec<PromptKeywordItem>,
    mode: PromptAnalyseMode,
    bulk: bool,
    trend_dir: PromptTrendDirection,
    editor: Option<Entity<Editor>>,
    editor_focused: bool,
    dropdown_open: bool,
    active_suggestion: usize,
    hovered_tip: Option<SharedString>,
    results: Option<Vec<PromptAnalysisResult>>,
    analysis_gen: usize,
    loading: bool,
    empty: bool,
    toast: Option<(SharedString, Instant)>,
    hero_surface: Rc<RefCell<HeroGlowSurface>>,
    composer_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    page_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    measured_width: Rc<Cell<f32>>,
    boot_at: Instant,
}

impl EventEmitter<PromptExplorerEvent> for PromptExplorerScreen {}

impl Focusable for PromptExplorerScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl PromptExplorerScreen {
    /// Construct a [`PromptExplorerScreen`] framed inside [`AppShell`] (`"Northwind"` workspace).
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self::build(id.into(), false, cx)
    }

    /// Construct [`PromptExplorerScreen`] without the outer [`AppShell`] sidebar.
    pub fn standalone(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self::build(id.into(), true, cx)
    }

    fn build(id_str: SharedString, standalone: bool, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let active: SharedString = tr(shell_cx, "Conversations", "对话探索").into();
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_workspace("Northwind")
                .with_user("Alex Morgan", "alex@northwind.ai")
                .with_groups(build_northwind_sidebar(shell_cx))
                .with_active(active)
                .with_side_cards(true)
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });

        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| {
            if let AppShellEvent::Navigate(label) = ev {
                cx.emit(PromptExplorerEvent::Navigate(label.clone()));
                cx.notify();
            }
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            standalone,
            layout: AppShellLayout::Inset,
            shell,
            focus: cx.focus_handle().tab_stop(true),
            workspace: "Northwind".into(),
            user_name: "Alex Morgan".into(),
            user_email: "alex@northwind.ai".into(),
            custom_title: None,
            prompts: default_prompt_catalog(),
            popular: default_popular_keywords(),
            rising: default_rising_keywords(),
            falling: default_falling_keywords(),
            mode: PromptAnalyseMode::Keyword,
            bulk: false,
            trend_dir: PromptTrendDirection::Rising,
            editor: None,
            editor_focused: false,
            dropdown_open: false,
            active_suggestion: 0,
            hovered_tip: None,
            results: None,
            analysis_gen: 0,
            loading: false,
            empty: false,
            toast: None,
            hero_surface: Rc::new(RefCell::new(HeroGlowSurface::new())),
            composer_bounds: Rc::new(Cell::new(None)),
            page_bounds: Rc::new(Cell::new(None)),
            measured_width: Rc::new(Cell::new(960.)),
            boot_at: Instant::now(),
        }
    }

    /// Configure whether [`PromptExplorerScreen`] renders standalone (`true`) or inside [`AppShell`] (`false`).
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Choose `Inset` or `Flat` framing on the outer [`AppShell`].
    pub fn with_layout(mut self, layout: AppShellLayout, cx: &mut Context<Self>) -> Self {
        self.layout = layout;
        self.refresh_shell(cx);
        self
    }

    /// Override the workspace brand name in the sidebar and `"Mentions you"` tooltip.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.workspace = workspace.into();
        self.refresh_shell(cx);
        self
    }

    /// Override the sidebar account display name and email.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.user_name = name.into();
        self.user_email = email.into();
        self.refresh_shell(cx);
        self
    }

    /// Override the hero heading above the prompt composer.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.custom_title = Some(title.into());
        self
    }

    /// Supply custom autocomplete prompt entries.
    pub fn with_prompts(mut self, prompts: Vec<PromptKeywordItem>) -> Self {
        self.prompts = prompts;
        self
    }

    /// Supply custom Popular Keywords entries on the left column.
    pub fn with_popular(mut self, popular: Vec<PromptKeywordItem>) -> Self {
        self.popular = popular;
        self
    }

    /// Supply custom Rising Trending Keywords entries on the right column.
    pub fn with_rising(mut self, rising: Vec<PromptKeywordItem>) -> Self {
        self.rising = rising;
        self
    }

    /// Supply custom Falling Trending Keywords entries on the right column.
    pub fn with_falling(mut self, falling: Vec<PromptKeywordItem>) -> Self {
        self.falling = falling;
        self
    }

    /// Set the initial composer mode (`Keyword` or `Url`).
    pub fn with_mode(mut self, mode: PromptAnalyseMode) -> Self {
        self.mode = mode;
        if mode == PromptAnalyseMode::Url {
            self.bulk = false;
        }
        self
    }

    /// Set whether multi-line `"Bulk analysis"` starts enabled.
    pub fn with_bulk_analysis(mut self, bulk: bool) -> Self {
        self.bulk = bulk && self.mode == PromptAnalyseMode::Keyword;
        self
    }

    /// Set the initial direction (`Rising` or `Falling`) for Trending Keywords.
    pub fn with_trend_direction(mut self, direction: PromptTrendDirection) -> Self {
        self.trend_dir = direction;
        self
    }

    /// Configure whether synchronized skeleton rows are shown in the keyword lists.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure whether the no-keywords empty state is shown.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return whether skeleton placeholders are currently active.
    pub const fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle skeleton loading rows at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            self.boot_at = Instant::now();
            cx.notify();
        }
    }

    /// Return whether the empty keyword state is currently active.
    pub const fn is_empty(&self) -> bool {
        self.empty
    }

    /// Toggle the empty keyword state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return the active composer analysis mode.
    pub const fn mode(&self) -> PromptAnalyseMode {
        self.mode
    }

    /// Switch between `Keyword` and `Url` composer modes.
    pub fn set_mode(&mut self, mode: PromptAnalyseMode, cx: &mut Context<Self>) {
        if self.mode != mode {
            self.mode = mode;
            if mode == PromptAnalyseMode::Url {
                self.bulk = false;
                self.dropdown_open = false;
            }
            cx.emit(PromptExplorerEvent::ModeChanged(mode));
            cx.notify();
        }
    }

    /// Return whether `"Bulk analysis"` multi-line mode is active.
    pub const fn is_bulk_analysis(&self) -> bool {
        self.bulk && matches!(self.mode, PromptAnalyseMode::Keyword)
    }

    /// Toggle `"Bulk analysis"` multi-line mode.
    pub fn set_bulk_analysis(&mut self, bulk: bool, cx: &mut Context<Self>) {
        if self.mode == PromptAnalyseMode::Url {
            return;
        }
        if self.bulk != bulk {
            self.bulk = bulk;
            cx.emit(PromptExplorerEvent::BulkToggled(bulk));
            cx.notify();
        }
    }

    /// Return the active direction of the Trending Keywords list.
    pub const fn trend_direction(&self) -> PromptTrendDirection {
        self.trend_dir
    }

    /// Switch Trending Keywords between `Rising` and `Falling`.
    pub fn set_trend_direction(&mut self, dir: PromptTrendDirection, cx: &mut Context<Self>) {
        if self.trend_dir != dir {
            self.trend_dir = dir;
            cx.emit(PromptExplorerEvent::TrendDirectionChanged(dir));
            cx.notify();
        }
    }

    /// Return the currently displayed prompt analysis rows, if any.
    pub fn results(&self) -> Option<&[PromptAnalysisResult]> {
        self.results.as_deref()
    }

    /// Dismiss the current analysis results card.
    pub fn clear_analysis(&mut self, cx: &mut Context<Self>) {
        if self.results.take().is_some() {
            cx.emit(PromptExplorerEvent::ClearAnalysis);
            cx.notify();
        }
    }

    /// Analyse one or more query strings and display the resulting breakdown card.
    pub fn analyse_queries(&mut self, raw_queries: Vec<String>, cx: &mut Context<Self>) {
        let cleaned: Vec<SharedString> = raw_queries
            .into_iter()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .map(SharedString::from)
            .collect();
        if cleaned.is_empty() {
            let msg = match self.mode {
                PromptAnalyseMode::Url => {
                    tr(cx, "Paste a page's address first", "请先粘贴网页链接")
                }
                PromptAnalyseMode::Keyword => tr(cx, "Type a prompt first", "请先输入提问或关键词"),
            };
            self.toast = Some((msg.into(), Instant::now()));
            cx.notify();
            return;
        }
        self.dropdown_open = false;
        let computed: Vec<PromptAnalysisResult> = cleaned
            .iter()
            .map(|q| synthesize_prompt_result(q, &self.prompts))
            .collect();
        self.analysis_gen = self.analysis_gen.wrapping_add(1);
        self.results = Some(computed.clone());
        cx.emit(PromptExplorerEvent::Analyse {
            prompts: cleaned,
            mode: self.mode,
            results: computed,
        });
        cx.notify();
    }

    fn refresh_shell(&self, cx: &mut Context<Self>) {
        let ws = self.workspace.clone();
        let name = self.user_name.clone();
        let email = self.user_email.clone();
        let layout = self.layout;
        let shell_id = format!("{}-shell", self.id);
        let this = cx.entity();
        self.shell.update(cx, |s, shell_cx| {
            let active: SharedString = tr(shell_cx, "Conversations", "对话探索").into();
            let mut next = AppShell::new(shell_id, shell_cx)
                .with_layout(layout)
                .with_workspace(ws)
                .with_user(name, email)
                .with_groups(build_northwind_sidebar(shell_cx))
                .with_active(active)
                .with_side_cards(true)
                .with_work_area(false, false);
            next.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            *s = next;
        });
    }

    fn current_editor_value(&self, cx: &App) -> String {
        self.editor
            .as_ref()
            .map(|ed| ed.read(cx).value().to_owned())
            .unwrap_or_default()
    }

    fn current_query_token(&self, cx: &App) -> String {
        let full = self.current_editor_value(cx);
        if self.is_bulk_analysis() {
            full.split('\n').next_back().unwrap_or("").trim().to_owned()
        } else {
            full.trim().to_owned()
        }
    }

    fn current_suggestions(&self, cx: &App) -> Vec<PromptKeywordItem> {
        let token = self.current_query_token(cx);
        rank_prompt_suggestions(&token, self.mode, &self.prompts)
    }

    fn submit_from_editor(&mut self, cx: &mut Context<Self>) {
        let val = self.current_editor_value(cx);
        let queries = if self.is_bulk_analysis() {
            val.split('\n').map(str::to_owned).collect()
        } else {
            vec![val]
        };
        self.analyse_queries(queries, cx);
    }

    fn pick_keyword(&mut self, text: SharedString, w: &mut Window, cx: &mut Context<Self>) {
        let full = self.current_editor_value(cx);
        let next_val = if self.is_bulk_analysis() {
            let mut lines: Vec<&str> = full.split('\n').collect();
            if !lines.is_empty() {
                lines.pop();
            }
            lines.push(text.as_ref());
            lines.join("\n")
        } else {
            text.to_string()
        };
        if let Some(ed) = self.editor.clone() {
            ed.update(cx, |e, cx| {
                e.set_value(next_val.clone(), w, cx);
                e.focus(w, cx);
            });
        }
        self.dropdown_open = false;
        let queries = if self.is_bulk_analysis() {
            next_val.split('\n').map(str::to_owned).collect()
        } else {
            vec![text.to_string()]
        };
        self.analyse_queries(queries, cx);
    }

    fn ensure_editor(&mut self, mobile: bool, w: &mut Window, cx: &mut Context<Self>) {
        let placeholder: SharedString = match self.mode {
            PromptAnalyseMode::Url => tr(
                cx,
                "Paste a page – see which prompts it answers",
                "粘贴网页链接 — 查看它回答了哪些提问",
            )
            .into(),
            PromptAnalyseMode::Keyword if self.is_bulk_analysis() => {
                tr(cx, "One prompt per line…", "每行输入一条提问或关键词…").into()
            }
            PromptAnalyseMode::Keyword => {
                tr(cx, "Type a prompt or keyword…", "输入提问或关键词…").into()
            }
        };
        let label: SharedString = match self.mode {
            PromptAnalyseMode::Url => tr(cx, "A page's address", "网页地址").into(),
            PromptAnalyseMode::Keyword if self.is_bulk_analysis() => {
                tr(cx, "Prompts, one per line", "多行批量提问").into()
            }
            PromptAnalyseMode::Keyword => tr(cx, "A prompt or keyword", "提问或关键词").into(),
        };
        let target_mode = if self.is_bulk_analysis() {
            InputMode::Multi
        } else {
            InputMode::Combobox
        };
        let target_fs = if mobile { 16.0 } else { 14.0 };

        if self.editor.is_none() {
            let ed = cx.new(|cx| {
                let mut e = Editor::new(label.clone(), target_mode, w, cx);
                e.set_placeholder(placeholder.clone());
                e.set_command_submit(true);
                e.font_size = target_fs;
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    this.dropdown_open = true;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => {
                    this.dropdown_open = true;
                    this.active_suggestion = 0;
                    cx.notify();
                }
                EditEvent::Navigate(down) => {
                    let suggestions = this.current_suggestions(cx);
                    if !suggestions.is_empty() {
                        if !this.dropdown_open {
                            this.dropdown_open = true;
                            this.active_suggestion = 0;
                        } else if *down {
                            this.active_suggestion =
                                (this.active_suggestion + 1) % suggestions.len();
                        } else {
                            this.active_suggestion = (this.active_suggestion + suggestions.len()
                                - 1)
                                % suggestions.len();
                        }
                        cx.notify();
                    }
                }
                EditEvent::Submitted => {
                    let suggestions = this.current_suggestions(cx);
                    if this.dropdown_open && !suggestions.is_empty() {
                        let idx = this.active_suggestion.min(suggestions.len() - 1);
                        let chosen = suggestions[idx].text.clone();
                        this.pick_keyword(chosen, w, cx);
                    } else if !this.is_bulk_analysis() {
                        this.submit_from_editor(cx);
                    }
                }
                EditEvent::Escape => {
                    if this.dropdown_open {
                        this.dropdown_open = false;
                        cx.notify();
                    }
                }
            })
            .detach();

            cx.subscribe_in(&ed, w, |this, _, ev: &EditorShortcutEvent, _, cx| {
                let EditorShortcutEvent::CommandSubmit = ev;
                this.submit_from_editor(cx);
            })
            .detach();

            self.editor = Some(ed);
        }

        let is_open = self.dropdown_open && !self.current_suggestions(cx).is_empty();
        if let Some(ed) = &self.editor {
            ed.update(cx, |e, _| {
                e.mode = target_mode;
                e.font_size = target_fs;
                e.set_placeholder(placeholder);
                e.set_command_submit(true);
                e.expanded = (target_mode == InputMode::Combobox).then_some(is_open);
            });
        }
    }

    fn render_change_badge(
        change: i64,
        with_arrow: bool,
        up_color: Hsla,
        down_color: Hsla,
        muted: Hsla,
    ) -> Div {
        let color = if change > 0 {
            up_color
        } else if change < 0 {
            down_color
        } else {
            muted
        };
        let mut row = div()
            .flex()
            .items_center()
            .justify_end()
            .gap(px(4.))
            .text_size(px(13.))
            .text_color(color);
        if with_arrow && change != 0 {
            let svg = if change > 0 {
                ARROW_UP_SVG
            } else {
                ARROW_DOWN_SVG
            };
            row = row.child(pro_svg("0 0 24 24", svg, 13.5, color));
            row = row.child(format_prompt_volume(change));
        } else {
            row = row.child(format_signed_delta(change));
        }
        row
    }

    fn render_highlighted_completion(text: &str, typed: &str, p: ProPalette) -> Div {
        let lower_text = text.to_ascii_lowercase();
        let lower_typed = typed.to_ascii_lowercase();
        let mut wrap = div()
            .min_w_0()
            .flex_1()
            .overflow_hidden()
            .whitespace_nowrap()
            .flex()
            .items_center()
            .text_size(px(13.));
        if !lower_typed.is_empty()
            && let Some(pos) = lower_text.find(&lower_typed)
            && text.is_char_boundary(pos)
            && text.is_char_boundary(pos + lower_typed.len())
        {
            let before = text[..pos].to_owned();
            let mid = text[pos..pos + lower_typed.len()].to_owned();
            let after = text[pos + lower_typed.len()..].to_owned();
            if !before.is_empty() {
                wrap = wrap.child(div().text_color(p.muted).child(before));
            }
            wrap = wrap.child(
                div()
                    .text_color(p.ink)
                    .font_weight(FontWeight::MEDIUM)
                    .child(mid),
            );
            if !after.is_empty() {
                wrap = wrap.child(div().text_color(p.muted).child(after));
            }
            return wrap;
        }
        wrap.text_color(p.muted).child(text.to_owned())
    }

    #[allow(clippy::too_many_arguments)]
    fn render_keyword_column(
        &self,
        col_key: &'static str,
        items: &[PromptKeywordItem],
        with_spark: bool,
        mobile: bool,
        skel_phase: f32,
        up_color: Hsla,
        down_color: Hsla,
        cx: &Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let show_spark = with_spark && !mobile;
        if self.loading {
            let widths = [180., 140., 200., 120., 160., 110., 150.];
            let mut list = div().mx(px(-8.)).flex().flex_col();
            for (idx, &bar_w) in widths.iter().enumerate() {
                list = list.child(
                    div()
                        .h(px(44.))
                        .px(px(8.))
                        .when(idx > 0, |d| d.border_t_1().border_color(p.line))
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(pro_skeleton_bar(bar_w, 10., 4., skel_phase, cx)),
                        )
                        .when(show_spark, |d| {
                            d.child(pro_skeleton_bar(48., 8., 4., skel_phase, cx))
                        })
                        .child(
                            div()
                                .w(px(if mobile { 64. } else { 72. }))
                                .flex()
                                .justify_end()
                                .child(pro_skeleton_bar(44., 10., 4., skel_phase, cx)),
                        )
                        .child(
                            div()
                                .w(px(if mobile { 48. } else { 52. }))
                                .flex()
                                .justify_end()
                                .child(pro_skeleton_bar(36., 10., 4., skel_phase, cx)),
                        ),
                );
            }
            return list.into_any_element();
        }

        if self.empty || items.is_empty() {
            let (title, caption) = if self.empty || !with_spark {
                (
                    tr(cx, "No keywords yet", "暂无关键词数据"),
                    tr(
                        cx,
                        "Once your prompts are tracked for a few days, the keywords people ask most show up here.",
                        "连续追踪几天提问后，用户最常询问的关键词将显示在这里。",
                    ),
                )
            } else {
                match self.trend_dir {
                    PromptTrendDirection::Rising => (
                        tr(cx, "Nothing rising right now", "当前暂无上升关键词"),
                        tr(
                            cx,
                            "Check back in a few days – it updates every morning.",
                            "每天早晨自动更新，请几天后再查看。",
                        ),
                    ),
                    PromptTrendDirection::Falling => (
                        tr(cx, "Nothing falling right now", "当前暂无下降关键词"),
                        tr(
                            cx,
                            "Check back in a few days – it updates every morning.",
                            "每天早晨自动更新，请几天后再查看。",
                        ),
                    ),
                }
            };
            return pro_empty_state(ProIcon::Search, title, caption, None, cx).into_any_element();
        }

        let mut list = div().mx(px(-8.)).flex().flex_col();
        for (idx, item) in items.iter().enumerate() {
            let text = item.text.clone();
            let vol_str = format_prompt_volume(item.volume as i64);
            let dir_word = if item.change >= 0 { "up" } else { "down" };
            let aria = format!(
                "{}, {} prompts a month, {} {} – analyse",
                item.text,
                vol_str,
                dir_word,
                format_prompt_volume(item.change),
            );
            let row = activate(
                div()
                    .id(pro_eid(&self.id, format!("kw-{col_key}-{idx}")))
                    .role(Role::Button)
                    .aria_label(aria)
                    .tab_index(0)
                    .h(px(44.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .child(div().flex_1().min_w_0().truncate().child(item.text.clone()))
                    .when(show_spark, |d| {
                        d.child(
                            div()
                                .w(px(56.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(render_mini_sparkline(&item.trend, 48., 16., p.faint)),
                        )
                    })
                    .child(
                        div()
                            .w(px(if mobile { 64. } else { 72. }))
                            .flex_none()
                            .flex()
                            .justify_end()
                            .child(Self::render_change_badge(
                                item.change,
                                true,
                                up_color,
                                down_color,
                                p.muted,
                            )),
                    )
                    .child(
                        div()
                            .w(px(if mobile { 48. } else { 52. }))
                            .flex_none()
                            .text_right()
                            .text_color(p.ink)
                            .child(vol_str),
                    ),
                cx,
                move |this, w, cx| {
                    this.pick_keyword(text.clone(), w, cx);
                },
            );
            list = list.child(
                div()
                    .when(idx > 0, |d| d.border_t_1().border_color(p.line))
                    .child(row),
            );
        }
        list.into_any_element()
    }

    fn render_results_card(
        &self,
        rows: &[PromptAnalysisResult],
        compact_cols: bool,
        up_color: Hsla,
        down_color: Hsla,
        on_blue: Hsla,
        cx: &Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let heading: SharedString = if rows.len() > 1 {
            format!("{} {}", rows.len(), tr(cx, "prompts", "条提问")).into()
        } else {
            tr(cx, "Analysis", "分析结果").into()
        };

        let mut table = div().px(px(12.)).pb(px(8.)).flex().flex_col().child(
            div()
                .h(px(28.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(16.))
                .text_size(px(10.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted)
                .child(div().flex_1().min_w_0().child(tr(cx, "PROMPT", "提问")))
                .child(
                    div()
                        .w(px(if compact_cols { 80. } else { 96. }))
                        .flex_none()
                        .text_right()
                        .child(tr(cx, "PROMPTS / MO", "月提问量")),
                )
                .child(
                    div()
                        .w(px(if compact_cols { 70. } else { 80. }))
                        .flex_none()
                        .text_right()
                        .child(tr(cx, "CHANGE", "环比变化")),
                )
                .when(!compact_cols, |d| {
                    d.child(div().w(px(96.)).flex_none().text_center().child(tr(
                        cx,
                        "12 MONTHS",
                        "近 12 个月",
                    )))
                    .child(div().w(px(150.)).flex_none().child(tr(
                        cx,
                        "MENTIONS YOU",
                        "品牌提及率",
                    )))
                }),
        );

        for (ri, row) in rows.iter().enumerate() {
            let share_str = format_share_pct(row.share);
            table = table.child(
                div()
                    .h(px(44.))
                    .px(px(8.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .child(div().flex_1().min_w_0().truncate().child(row.text.clone()))
                    .child(
                        div()
                            .w(px(if compact_cols { 80. } else { 96. }))
                            .flex_none()
                            .text_right()
                            .child(format_prompt_volume(row.volume as i64)),
                    )
                    .child(
                        div()
                            .w(px(if compact_cols { 70. } else { 80. }))
                            .flex_none()
                            .flex()
                            .justify_end()
                            .child(Self::render_change_badge(
                                row.change, true, up_color, down_color, p.muted,
                            )),
                    )
                    .when(!compact_cols, |d| {
                        d.child(
                            div()
                                .w(px(96.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(render_mini_sparkline(&row.trend, 72., 18., p.faint)),
                        )
                        .child(
                            div()
                                .w(px(150.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .text_color(p.text)
                                .child(div().flex_1().min_w_0().child(pro_spring_bar(
                                    pro_eid(
                                        &self.id,
                                        format!("res-share-{}-{ri}", self.analysis_gen),
                                    ),
                                    row.share,
                                    6.,
                                    3.,
                                    p.chip,
                                    on_blue,
                                )))
                                .child(div().w(px(34.)).flex_none().text_right().child(share_str)),
                        )
                    }),
            );
        }

        div()
            .w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .pt(px(14.))
                    .pr(px(16.))
                    .pb(px(4.))
                    .pl(px(20.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(24.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(heading),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "clear-analysis"),
                        tr(cx, "Clear the analysis", "关闭分析结果"),
                        ProIcon::Close,
                        28.,
                        false,
                        cx,
                        |this, _, cx| this.clear_analysis(cx),
                    )),
            )
            .child(table)
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let width = self.measured_width.get();
        let stack_cols = width < 860.0;
        let mobile = width < 560.0;

        self.ensure_editor(mobile, w, cx);

        let elapsed = self.boot_at.elapsed().as_secs_f32();
        let skel_phase = if reduced {
            0.25
        } else {
            (elapsed / 1.4).fract()
        };
        if self.loading && !reduced {
            w.request_animation_frame();
        }

        let up_color: Hsla = if p.dark {
            rgb(0x3dd68c).into()
        } else {
            rgb(0x12a150).into()
        };
        let down_color: Hsla = if p.dark {
            rgb(0xff6369).into()
        } else {
            rgb(0xe5484d).into()
        };
        let on_blue: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgb(0x0a84ff).into()
        };
        let hero_ink: Hsla = if p.dark {
            hsla(0., 0., 1., 0.94)
        } else {
            hsla(0., 0., 0., 0.88)
        };
        let box_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };

        let suggestions = self.current_suggestions(cx);
        let show_dropdown = self.dropdown_open && self.editor_focused && !suggestions.is_empty();
        let active_idx = if suggestions.is_empty() {
            0
        } else {
            self.active_suggestion.min(suggestions.len() - 1)
        };
        let typed_token = self.current_query_token(cx);

        let full_val = self.current_editor_value(cx);
        let bulk_line_count = if self.is_bulk_analysis() {
            full_val
                .split('\n')
                .filter(|l| !l.trim().is_empty())
                .count()
        } else {
            0
        };
        let raw_line_rows = full_val.split('\n').count().max(2);
        let field_h = if self.is_bulk_analysis() {
            ((raw_line_rows as f32) * 22.0 + 18.0).clamp(68.0, 148.0)
        } else {
            44.0
        };

        // Atmospheric drifting hero background canvas
        let surface_ref = self.hero_surface.clone();
        let dark = p.dark;
        let panel_bg = p.panel;
        let boot_at = self.boot_at;
        let hero_bg_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, cx| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 2. || bh <= 2. {
                    return;
                }
                let animate = !Theme::global(cx).reduced_motion && window.is_visible();
                let secs = if animate {
                    boot_at.elapsed().as_secs_f32()
                } else {
                    0.0
                };
                let image = surface_ref
                    .borrow_mut()
                    .frame(dark, panel_bg, secs, !animate, window);
                let corners = Corners {
                    top_left: px(12.),
                    top_right: px(12.),
                    bottom_left: px(0.),
                    bottom_right: px(0.),
                };
                let _ = window.paint_image(bounds, bounds, corners, image, 0, false);
                if animate {
                    window.request_animation_frame();
                }
            },
        )
        .absolute()
        .inset_0();

        // Hero title
        let title_el = if let Some(custom) = &self.custom_title {
            div()
                .max_w(px(520.))
                .text_size(px(if mobile { 22. } else { 28. }))
                .line_height(px(if mobile { 28. } else { 36. }))
                .font_weight(FontWeight::MEDIUM)
                .text_color(hero_ink)
                .text_center()
                .child(custom.clone())
        } else if mobile {
            div()
                .max_w(px(460.))
                .text_size(px(22.))
                .line_height(px(28.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(hero_ink)
                .text_center()
                .child(tr(
                    cx,
                    "Explore what people are prompting ChatGPT",
                    "探索大家正在向 ChatGPT 提问什么",
                ))
        } else {
            div()
                .max_w(px(520.))
                .flex()
                .flex_col()
                .items_center()
                .text_size(px(28.))
                .line_height(px(36.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(hero_ink)
                .text_center()
                .child(tr(cx, "Explore what people are", "探索大家正在向"))
                .child(tr(cx, "prompting ChatGPT", "ChatGPT 提问什么"))
        };

        // Mode switcher (Prompts vs Page)
        let mode_group = div()
            .id(pro_eid(&self.id, "mode-group"))
            .role(Role::RadioGroup)
            .aria_label(tr(cx, "Analyse", "分析模式"))
            .flex()
            .items_center()
            .gap(px(2.))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if matches!(ev.keystroke.key.as_str(), "left" | "right") {
                    let next = match this.mode {
                        PromptAnalyseMode::Keyword => PromptAnalyseMode::Url,
                        PromptAnalyseMode::Url => PromptAnalyseMode::Keyword,
                    };
                    this.set_mode(next, cx);
                    cx.stop_propagation();
                }
            }))
            .children(PromptAnalyseMode::all().into_iter().map(|m| {
                let checked = self.mode == m;
                let tip: SharedString = m.label(cx).into();
                let tip_key: SharedString = format!("mode-{}", m.slug()).into();
                let show_tip = self.hovered_tip.as_ref() == Some(&tip_key);
                let icon = match m {
                    PromptAnalyseMode::Keyword => ProIcon::Search,
                    PromptAnalyseMode::Url => ProIcon::Link,
                };
                let fg = if checked { p.ink } else { p.faint };
                let btn = activate(
                    div()
                        .id(pro_eid(&self.id, format!("mode-{}", m.slug())))
                        .role(Role::RadioButton)
                        .aria_label(tip.clone())
                        .aria_toggled(checked.into())
                        .tab_index(if checked { 0 } else { -1 })
                        .size(px(28.))
                        .rounded(px(6.))
                        .when(checked, |d| d.bg(p.chip))
                        .hover(move |s| {
                            if checked {
                                s.text_color(p.ink)
                            } else {
                                s.text_color(p.ink).bg(p.hover)
                            }
                        })
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_hover(cx.listener({
                            let tip_key = tip_key.clone();
                            move |this, inside: &bool, _, cx| {
                                if *inside {
                                    this.hovered_tip = Some(tip_key.clone());
                                } else if this.hovered_tip.as_ref() == Some(&tip_key) {
                                    this.hovered_tip = None;
                                }
                                cx.notify();
                            }
                        }))
                        .child(pro_icon(icon, 15., fg)),
                    cx,
                    move |this, _, cx| this.set_mode(m, cx),
                );
                div().relative().child(btn).when(show_tip, |d| {
                    d.child(
                        div()
                            .absolute()
                            .bottom(px(34.))
                            .left(px(-16.))
                            .px(px(7.))
                            .py(px(3.))
                            .rounded(px(6.))
                            .bg(hsla(0., 0., 0., 0.86))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0xffffff))
                            .whitespace_nowrap()
                            .child(tip),
                    )
                })
            }));

        // Bulk analysis switch + label + Analyse button
        let url_disabled = self.mode == PromptAnalyseMode::Url;
        let bulk_checked = self.is_bulk_analysis();
        let bulk_label: SharedString = tr(cx, "Bulk analysis", "批量分析").into();
        let bulk_control = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .when(url_disabled, |d| d.opacity(0.45))
            .child(pro_switch(
                pro_eid(&self.id, "bulk-switch"),
                bulk_label.clone(),
                bulk_checked,
                cx,
                |this, w, cx| {
                    if this.mode == PromptAnalyseMode::Keyword {
                        let next = !this.bulk;
                        this.set_bulk_analysis(next, cx);
                        if let Some(ed) = this.editor.clone() {
                            ed.update(cx, |e, cx| e.focus(w, cx));
                        }
                    }
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "bulk-label"))
                    .role(Role::Button)
                    .tab_index(-1)
                    .text_size(px(13.))
                    .text_color(p.text)
                    .when(!url_disabled, |d| d.cursor_pointer())
                    .child(bulk_label),
                cx,
                |this, w, cx| {
                    if this.mode == PromptAnalyseMode::Keyword {
                        let next = !this.bulk;
                        this.set_bulk_analysis(next, cx);
                        if let Some(ed) = this.editor.clone() {
                            ed.update(cx, |e, cx| e.focus(w, cx));
                        }
                    }
                },
            ));

        let analyse_btn = activate(
            div()
                .id(pro_eid(&self.id, "analyse-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "Analyse", "分析"))
                .tab_index(0)
                .h(px(28.))
                .px(px(10.))
                .rounded(px(6.))
                .bg(p.primary)
                .text_color(p.on_primary)
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.opacity(0.90))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(tr(cx, "Analyse", "分析")),
            cx,
            |this, _, cx| this.submit_from_editor(cx),
        );

        let composer_bounds_cell = self.composer_bounds.clone();
        let firm_edge = self.editor_focused || show_dropdown;
        let box_border = if firm_edge {
            p.line_strong
        } else if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.07)
        };
        let box_corners = if show_dropdown {
            Corners {
                top_left: px(10.),
                top_right: px(10.),
                bottom_left: px(0.),
                bottom_right: px(0.),
            }
        } else {
            Corners::all(px(10.))
        };

        let mut composer_box = div()
            .id(pro_eid(&self.id, "composer-box"))
            .relative()
            .w_full()
            .max_w(px(600.))
            .rounded_tl(box_corners.top_left)
            .rounded_tr(box_corners.top_right)
            .rounded_bl(box_corners.bottom_left)
            .rounded_br(box_corners.bottom_right)
            .bg(box_bg)
            .border_1()
            .border_color(box_border)
            .shadow_lg()
            .flex()
            .flex_col()
            .capture_key_down(cx.listener(move |this, ev: &KeyDownEvent, w, cx| {
                // In Bulk mode, let ArrowDown/ArrowUp/Enter/Escape drive active completion popup when open
                if !this.is_bulk_analysis() {
                    return;
                }
                let suggestions = this.current_suggestions(cx);
                let popup_active = this.dropdown_open && !suggestions.is_empty();
                if !popup_active {
                    return;
                }
                match ev.keystroke.key.as_str() {
                    "down" => {
                        this.active_suggestion = (this.active_suggestion + 1) % suggestions.len();
                        w.prevent_default();
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "up" => {
                        this.active_suggestion =
                            (this.active_suggestion + suggestions.len() - 1) % suggestions.len();
                        w.prevent_default();
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "escape" => {
                        this.dropdown_open = false;
                        w.prevent_default();
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "enter"
                        if !ev.keystroke.modifiers.platform
                            && !ev.keystroke.modifiers.control
                            && !ev.keystroke.modifiers.shift =>
                    {
                        let idx = this.active_suggestion.min(suggestions.len() - 1);
                        let chosen = suggestions[idx].text.clone();
                        this.pick_keyword(chosen, w, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(
                canvas(
                    move |bounds, _, _| composer_bounds_cell.set(Some(bounds)),
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            );

        if let Some(ed) = &self.editor {
            composer_box = composer_box.child(
                div()
                    .w_full()
                    .h(px(field_h))
                    .pt(px(14.))
                    .px(px(16.))
                    .pb(px(4.))
                    .child(ed.clone()),
            );
        }

        composer_box = composer_box.child(
            div()
                .pt(px(8.))
                .pr(px(10.))
                .pb(px(10.))
                .pl(px(12.))
                .flex()
                .items_center()
                .flex_wrap()
                .gap(px(if mobile { 8. } else { 12. }))
                .child(mode_group)
                .child(div().flex_1())
                .when(bulk_checked && bulk_line_count > 0, |d| {
                    let count_lbl = if bulk_line_count == 1 {
                        format!("1 {}", tr(cx, "prompt", "条提问"))
                    } else {
                        format!("{bulk_line_count} {}", tr(cx, "prompts", "条提问"))
                    };
                    d.child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(count_lbl),
                    )
                })
                .child(bulk_control)
                .child(analyse_btn),
        );

        let hero_section = div()
            .relative()
            .w_full()
            .pt(px(if mobile { 40. } else { 72. }))
            .px(px(if mobile { 12. } else { 24. }))
            .pb(px(if mobile { 20. } else { 88. }))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(if mobile { 22. } else { 28. }))
            .child(hero_bg_canvas)
            .child(div().relative().child(title_el))
            .child(composer_box);

        // Trending switcher (Rising / Falling)
        let trend_switcher = div()
            .id(pro_eid(&self.id, "trend-group"))
            .role(Role::RadioGroup)
            .aria_label(tr(cx, "Trend", "趋势方向"))
            .flex()
            .items_center()
            .gap(px(2.))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if matches!(ev.keystroke.key.as_str(), "left" | "right") {
                    let next = match this.trend_dir {
                        PromptTrendDirection::Rising => PromptTrendDirection::Falling,
                        PromptTrendDirection::Falling => PromptTrendDirection::Rising,
                    };
                    this.set_trend_direction(next, cx);
                    cx.stop_propagation();
                }
            }))
            .children(PromptTrendDirection::all().into_iter().map(|dir| {
                let checked = self.trend_dir == dir;
                let tip: SharedString = dir.label(cx).into();
                let tip_key: SharedString = format!("trend-{}", dir.slug()).into();
                let show_tip = self.hovered_tip.as_ref() == Some(&tip_key);
                let (svg, active_fg) = match dir {
                    PromptTrendDirection::Rising => (CHART_RISING_SVG, up_color),
                    PromptTrendDirection::Falling => (CHART_FALLING_SVG, down_color),
                };
                let fg = if checked { active_fg } else { p.faint };
                let btn = activate(
                    div()
                        .id(pro_eid(&self.id, format!("trend-{}", dir.slug())))
                        .role(Role::RadioButton)
                        .aria_label(tip.clone())
                        .aria_toggled(checked.into())
                        .tab_index(if checked { 0 } else { -1 })
                        .w(px(32.))
                        .h(px(28.))
                        .rounded(px(6.))
                        .when(checked, |d| d.bg(p.chip))
                        .hover(move |s| when_not_checked_bg(s, checked, p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_hover(cx.listener({
                            let tip_key = tip_key.clone();
                            move |this, inside: &bool, _, cx| {
                                if *inside {
                                    this.hovered_tip = Some(tip_key.clone());
                                } else if this.hovered_tip.as_ref() == Some(&tip_key) {
                                    this.hovered_tip = None;
                                }
                                cx.notify();
                            }
                        }))
                        .child(pro_svg("0 0 24 24", svg, 16., fg)),
                    cx,
                    move |this, _, cx| this.set_trend_direction(dir, cx),
                );
                div().relative().child(btn).when(show_tip, |d| {
                    d.child(
                        div()
                            .absolute()
                            .bottom(px(34.))
                            .right_0()
                            .px(px(7.))
                            .py(px(3.))
                            .rounded(px(6.))
                            .bg(hsla(0., 0., 0., 0.86))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0xffffff))
                            .whitespace_nowrap()
                            .child(tip),
                    )
                })
            }));

        let pop_body = self.render_keyword_column(
            "popular",
            &self.popular,
            false,
            mobile,
            skel_phase,
            up_color,
            down_color,
            cx,
        );
        let active_trending = match self.trend_dir {
            PromptTrendDirection::Rising => &self.rising,
            PromptTrendDirection::Falling => &self.falling,
        };
        let trend_col_key = match self.trend_dir {
            PromptTrendDirection::Rising => "rising",
            PromptTrendDirection::Falling => "falling",
        };
        let trend_body = pro_view_transition(
            pro_eid(&self.id, "trend-list-tx"),
            trend_col_key,
            self.render_keyword_column(
                trend_col_key,
                active_trending,
                true,
                mobile,
                skel_phase,
                up_color,
                down_color,
                cx,
            ),
        );

        let col_px = if mobile { 14.0 } else { 20.0 };
        let left_col = div()
            .flex_1()
            .min_w_0()
            .pt(px(16.))
            .px(px(col_px))
            .pb(px(10.))
            .flex()
            .flex_col()
            .child(
                div()
                    .min_h(px(28.))
                    .mb(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(24.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "Popular Keywords", "热门关键词")),
                    ),
            )
            .child(pop_body);

        let right_col = div()
            .flex_1()
            .min_w_0()
            .pt(px(16.))
            .px(px(col_px))
            .pb(px(10.))
            .when(!stack_cols, |d| d.border_l_1().border_color(p.line_strong))
            .when(stack_cols, |d| d.border_t_1().border_color(p.line_strong))
            .flex()
            .flex_col()
            .child(
                div()
                    .min_h(px(28.))
                    .mb(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(24.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "Trending Keywords", "趋势关键词")),
                    )
                    .child(trend_switcher),
            )
            .child(trend_body);

        let keywords_card = div()
            .w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .when(stack_cols, |d| d.flex_col())
            .child(left_col)
            .child(right_col);

        let has_results = self.results.is_some();
        let results_el = self.results.as_deref().map(|rows| {
            self.render_results_card(rows, stack_cols, up_color, down_color, on_blue, cx)
        });
        let results_presence = pro_presence(
            pro_eid(&self.id, "results-pres"),
            has_results,
            0.22,
            0.14,
            move |a| {
                if let Some(card) = results_el {
                    div()
                        .w_full()
                        .relative()
                        .top(px(6. * (1. - a)))
                        .opacity(a)
                        .child(card)
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            },
        );

        // Width & page bounds sensor
        let mw_cell = self.measured_width.clone();
        let pb_cell = self.page_bounds.clone();
        let width_sensor = canvas(
            move |bounds, _, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 40.0 {
                    mw_cell.set(bw);
                }
                pb_cell.set(Some(bounds));
            },
            |_, _, _, _| (),
        )
        .w_full()
        .h(px(0.));

        let mut scroll = div()
            .id(pro_eid(&self.id, "scroll"))
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .pt(px(12.))
            .px(px(if mobile { 8. } else { 12. }))
            .pb(px(if mobile { 16. } else { 24. }))
            .flex()
            .flex_col()
            .gap(px(if mobile { 12. } else { 16. }))
            .child(width_sensor)
            .child(hero_section);

        if has_results {
            scroll = scroll.child(results_presence);
        }
        scroll = scroll.child(keywords_card);

        let mut page = div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h_0()
            .bg(p.panel)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if this.editor_focused {
                    return;
                }
                if ev.keystroke.key == "/"
                    && !ev.keystroke.modifiers.platform
                    && !ev.keystroke.modifiers.control
                    && let Some(ed) = this.editor.clone()
                {
                    ed.update(cx, |e, cx| e.focus(w, cx));
                    cx.stop_propagation();
                } else if ev.keystroke.key == "escape" && this.results.is_some() {
                    this.clear_analysis(cx);
                    cx.stop_propagation();
                }
            }))
            .child(scroll);

        // Floating combobox listbox painted above all cards via `deferred`
        if show_dropdown
            && let (Some(cb), Some(pb)) = (self.composer_bounds.get(), self.page_bounds.get())
        {
            let left_px = f32::from(cb.left() - pb.left()).max(8.0);
            let top_px = f32::from(cb.bottom() - pb.top()) - 1.0;
            let list_w = f32::from(cb.size.width).max(220.0);
            let mut popup = div()
                .id(pro_eid(&self.id, "relevant-listbox"))
                .role(Role::ListBox)
                .aria_label(tr(cx, "Relevant searches", "相关搜索"))
                .occlude()
                .absolute()
                .left(px(left_px))
                .top(px(top_px))
                .w(px(list_w))
                .p(px(6.))
                .rounded_b(px(10.))
                .bg(box_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_xl()
                .flex()
                .flex_col()
                .child(
                    div()
                        .px(px(10.))
                        .pt(px(6.))
                        .pb(px(5.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(tr(cx, "Relevant searches", "相关搜索")),
                );

            for (si, opt) in suggestions.iter().enumerate() {
                let selected = si == active_idx;
                let opt_text = opt.text.clone();
                popup = popup.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("relevant-opt-{si}")))
                        .role(Role::ListBoxOption)
                        .aria_selected(selected)
                        .h(px(34.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .when(selected, |d| d.bg(p.hover))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(16.))
                        .cursor_pointer()
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside && this.active_suggestion != si {
                                this.active_suggestion = si;
                                cx.notify();
                            }
                        }))
                        .child(Self::render_highlighted_completion(
                            &opt.text,
                            &typed_token,
                            p,
                        ))
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(13.))
                                .text_color(p.ink)
                                .child(format_prompt_volume(opt.volume as i64))
                                .child(Self::render_change_badge(
                                    opt.change, false, up_color, down_color, p.muted,
                                )),
                        ),
                    cx,
                    move |this, w, cx| this.pick_keyword(opt_text.clone(), w, cx),
                ));
            }

            page = page.child(deferred(popup).with_priority(6));
        }

        page = page.child(pro_toast_banner(self.toast.clone()));
        page
    }
}

fn when_not_checked_bg(style: StyleRefinement, checked: bool, hover: Hsla) -> StyleRefinement {
    if checked { style } else { style.bg(hover) }
}

impl Render for PromptExplorerScreen {
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
        PromptAnalyseMode, PromptExplorerScreen, PromptTrendDirection, default_prompt_catalog,
        deterministic_trend, format_prompt_volume, rank_prompt_suggestions,
        synthesize_prompt_result,
    };
    use gpui::TestAppContext;

    #[test]
    fn volume_formatter_and_sparkline_series_match_reference_numbers() {
        assert_eq!(format_prompt_volume(205_000), "205k");
        assert_eq!(format_prompt_volume(88_100), "88.1k");
        assert_eq!(format_prompt_volume(2_100), "2.1k");
        assert_eq!(format_prompt_volume(35_000), "35k");
        assert_eq!(format_prompt_volume(1_260_000), "1.3M");

        let trend = deterministic_trend("generative ai tools", 135_000, 2_100);
        assert_eq!(trend.len(), 12);
        let min_v = *trend.iter().min().unwrap();
        let max_v = *trend.iter().max().unwrap();
        assert!(max_v > min_v);
    }

    #[test]
    fn suggestion_ranking_prioritizes_prefix_matches_and_blocks_url_mode() {
        let catalog = default_prompt_catalog();
        let hits = rank_prompt_suggestions("aeo", PromptAnalyseMode::Keyword, &catalog);
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].text.as_ref(), "aeo");
        assert_eq!(hits[1].text.as_ref(), "aeo tools");
        assert_eq!(hits[2].text.as_ref(), "seo vs aeo");

        let url_hits = rank_prompt_suggestions("aeo", PromptAnalyseMode::Url, &catalog);
        assert!(url_hits.is_empty());

        let result = synthesize_prompt_result("northwind vs semrush", &catalog);
        assert_eq!(result.volume, 195_000);
        assert_eq!(result.change, 2_100);
        assert!((0.04..=0.75).contains(&result.share));
    }

    #[test]
    fn screen_switches_modes_and_analyses_bulk_prompts() {
        let mut cx = TestAppContext::single();
        cx.update(crate::init);
        let (view, w) = cx.add_window_view(|_, cx| {
            PromptExplorerScreen::new("test-pex", cx)
                .with_bulk_analysis(true)
                .with_trend_direction(PromptTrendDirection::Falling)
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert!(s.is_bulk_analysis());
                assert_eq!(s.trend_direction(), PromptTrendDirection::Falling);
                s.analyse_queries(
                    vec!["Northwind ai".to_owned(), "geo".to_owned(), "  ".to_owned()],
                    cx,
                );
                let res = s.results().expect("two analysed rows");
                assert_eq!(res.len(), 2);
                assert_eq!(res[0].text.as_ref(), "Northwind ai");
                assert_eq!(res[1].volume, 133_000);
                s.set_mode(PromptAnalyseMode::Url, cx);
                assert!(!s.is_bulk_analysis());
                s.clear_analysis(cx);
                assert!(s.results().is_none());
            });
        });
    }
}
