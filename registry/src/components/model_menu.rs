//! Product-scene frontier model picker with a gliding rose highlight band and edge fades.
//!
//! Displays recognizable model marks and labels inside a floating card whose right edge
//! and foot dissolve smoothly into the surrounding surface while a warm pink selection
//! band glides from model to model or follows pointer hover and keyboard input.

use crate::{
    components::pro_chrome::{ProPalette, pro_eid, pro_empty_state, pro_skeleton_bar, tr},
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use image::{Frame, RgbaImage};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use web_time::Instant;

const SCENE_W: usize = 384;
const SCENE_H: usize = 340;
const RASTER_SCALE: usize = 2;
const TEX_W: usize = SCENE_W * RASTER_SCALE;
const TEX_H: usize = SCENE_H * RASTER_SCALE;
const CARD_LEFT: f32 = 40.0;
const CARD_TOP: f32 = 34.0;
const CARD_W: f32 = 340.0;
const CARD_R: f32 = 12.0;
const ROW_H: f32 = 35.0;

const QWEN_SVG: &str = r#"<path fill="currentColor" d="M23.919 14.545 20.817 9.17l1.47-2.544a.56.56 0 0 0 0-.566l-1.633-2.83a.57.57 0 0 0-.49-.283h-6.207L12.487.402a.57.57 0 0 0-.49-.284H8.732a.56.56 0 0 0-.49.284L5.139 5.775h-2.94a.56.56 0 0 0-.49.284L.077 8.887a.56.56 0 0 0 0 .567L3.18 14.83l-1.47 2.545a.56.56 0 0 0 0 .566l1.634 2.83a.57.57 0 0 0 .49.283h6.205l1.47 2.545a.57.57 0 0 0 .49.284h3.266a.57.57 0 0 0 .49-.284l3.104-5.375h2.94a.57.57 0 0 0 .49-.283l1.634-2.828a.55.55 0 0 0-.004-.568M8.733.686l1.634 2.828-1.634 2.828H21.8L20.164 9.17H7.425L5.63 6.06Zm1.306 19.801-6.205-.002 1.634-2.83h3.265L2.201 6.344h3.267q3.182 5.517 6.367 11.032zm10.124-5.66L18.53 12l-6.532 11.315-1.634-2.83c2.129-3.673 4.25-7.351 6.373-11.028h3.592l3.102 5.374z"></path>"#;

type CustomMarkFn = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Single model row inside [`ModelMenu`].
#[derive(Clone)]
pub struct MenuModel {
    /// Stable identifier for this row.
    pub id: SharedString,
    /// Display name rendered beside the provider mark (for example `"Claude Opus"`).
    pub name: SharedString,
    /// Provider key used to resolve built-in artwork (`"deepseek"`, `"gpt"`, `"claude"`, `"gemini"`, `"mistral"`, `"qwen"`, `"meta"`).
    pub provider: SharedString,
    mark: Option<CustomMarkFn>,
}

impl std::fmt::Debug for MenuModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuModel")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("provider", &self.provider)
            .finish()
    }
}

impl MenuModel {
    /// Construct a model entry whose `provider` defaults to `id`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        let id_s = id.into();
        Self {
            provider: id_s.clone(),
            id: id_s,
            name: name.into(),
            mark: None,
        }
    }

    /// Specify the provider logo key explicitly.
    pub fn with_provider(mut self, provider: impl Into<SharedString>) -> Self {
        self.provider = provider.into();
        self
    }

    /// Supply a custom 16x16 logo element builder for this row.
    pub fn with_mark(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.mark = Some(Rc::new(render));
        self
    }
}

/// Default frontier model catalog matching the reference product scene.
pub fn default_menu_models() -> Vec<MenuModel> {
    vec![
        MenuModel::new("deepseek", "DeepSeek"),
        MenuModel::new("gpt", "GPT"),
        MenuModel::new("claude-opus", "Claude Opus").with_provider("claude"),
        MenuModel::new("gemini", "Gemini"),
        MenuModel::new("mistral-large", "Mistral Large").with_provider("mistral"),
        MenuModel::new("qwen", "Qwen"),
        MenuModel::new("llama", "Llama").with_provider("meta"),
    ]
}

/// Events emitted by [`ModelMenu`] when a model row is picked interactively.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelMenuEvent {
    /// Emitted when clicking or pressing Enter/Space picks a model row.
    Select {
        /// Zero-based index of the picked row.
        index: usize,
        /// Display name of the picked model.
        name: SharedString,
    },
}

impl EventEmitter<ModelMenuEvent> for ModelMenu {}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CardPlateKey {
    dark: bool,
    has_title: bool,
    row_count: u8,
    band_y_q: i32,
}

struct CardPlateCache {
    key: Option<CardPlateKey>,
    image: Option<Arc<RenderImage>>,
}

impl CardPlateCache {
    fn new() -> Self {
        Self {
            key: None,
            image: None,
        }
    }

    fn render(&mut self, key: CardPlateKey, window: &mut Window) -> Arc<RenderImage> {
        if self.key == Some(key)
            && let Some(existing) = &self.image
        {
            return existing.clone();
        }
        let img = Arc::new(rasterize_card_plate(key));
        if let Some(prev) = self.image.replace(img.clone()) {
            let _ = window.drop_image(prev);
        }
        self.key = Some(key);
        img
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// Combined right-edge and bottom-edge CSS mask opacity in `[0.0, 1.0]` for scene coordinates `(sx, sy)`.
fn scene_mask_alpha(sx: f32, sy: f32) -> f32 {
    let mx = 1. - smoothstep(SCENE_W as f32 - 62., SCENE_W as f32 - 4., sx);
    let my = 1. - smoothstep(SCENE_H as f32 * 0.56, SCENE_H as f32 * 0.95, sy);
    (mx * my).clamp(0., 1.)
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

fn rasterize_card_plate(key: CardPlateKey) -> RenderImage {
    let rows_top = CARD_TOP + 12.0 + if key.has_title { 20.0 } else { 0.0 };
    let card_h =
        12.0 + if key.has_title { 20.0 } else { 0.0 } + f32::from(key.row_count) * ROW_H + 56.0;
    let card_x0 = CARD_LEFT;
    let card_y0 = CARD_TOP;
    let card_x1 = card_x0 + CARD_W;
    let card_y1 = card_y0 + card_h;

    let band_top = rows_top + (key.band_y_q as f32 / 20.0);
    let band_bot = band_top + ROW_H;

    let (
        card_rgb,
        border_rgb,
        border_a,
        sh_a1,
        sh_a2,
        pick_rgb,
        pick_line_rgb,
        pick_a0,
        pick_a60,
        pick_line_a,
    ) = if key.dark {
        (
            [20. / 255., 20. / 255., 22. / 255.],
            [1.0, 1.0, 1.0],
            0.09,
            0.42,
            0.24,
            [204. / 255., 90. / 255., 142. / 255.],
            [236. / 255., 124. / 255., 176. / 255.],
            0.20,
            0.065,
            0.28,
        )
    } else {
        (
            [1.0, 1.0, 1.0],
            [0.0, 0.0, 0.0],
            0.08,
            0.10,
            0.055,
            [232. / 255., 54. / 255., 140. / 255.],
            [226. / 255., 46. / 255., 134. / 255.],
            0.11,
            0.035,
            0.20,
        )
    };

    const BAYER4: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    let px_u = 1.0 / RASTER_SCALE as f32;
    let mut raw = vec![0u8; TEX_W * TEX_H * 4];
    for py in 0..TEX_H {
        let sy = (py as f32 + 0.5) * px_u;
        for px_i in 0..TEX_W {
            let sx = (px_i as f32 + 0.5) * px_u;
            let mask = scene_mask_alpha(sx, sy);
            if mask <= 0.002 {
                continue;
            }

            // Soft multi-layer elevation shadow around the card
            let d_sh1 = sd_round_rect(
                sx,
                sy - 10.0,
                card_x0 + 4.0,
                card_y0 + 4.0,
                card_x1 - 4.0,
                card_y1 - 4.0,
                CARD_R,
            );
            let d_sh2 = sd_round_rect(sx, sy - 1.5, card_x0, card_y0, card_x1, card_y1, CARD_R);
            let s1 = (1. - (d_sh1.max(0.) / 24.0)).clamp(0., 1.).powi(2) * sh_a1;
            let s2 = (1. - (d_sh2.max(0.) / 3.0)).clamp(0., 1.).powi(2) * sh_a2;
            let mut c = blend_over([0.0; 4], [0.0, 0.0, 0.0], (s1 + s2).clamp(0., 0.55));

            let d_card = sd_round_rect(sx, sy, card_x0, card_y0, card_x1, card_y1, CARD_R);
            let cov_card = (0.5 - d_card / px_u).clamp(0., 1.);
            if cov_card > 0.0 {
                c = blend_over(c, card_rgb, cov_card);

                // Subtle warm plum-rose wash + crisp 1px top/bottom gradient hairlines (0% -> 62% -> 92%)
                if sy >= band_top - 0.6 && sy <= band_bot + 0.6 {
                    let u = ((sx - card_x0) / CARD_W).clamp(0., 1.);
                    if u < 0.92 {
                        let fill_a = if u <= 0.62 {
                            let t = u / 0.62;
                            pick_a0 + (pick_a60 - pick_a0) * t
                        } else {
                            let t = (u - 0.62) / 0.30;
                            pick_a60 * (1. - t)
                        };
                        let v_cov = ((sy - (band_top - 0.5 * px_u)) / px_u).clamp(0., 1.)
                            * (((band_bot + 0.5 * px_u) - sy) / px_u).clamp(0., 1.);
                        c = blend_over(c, pick_rgb, fill_a * v_cov * cov_card);

                        let line_fade = if u <= 0.58 {
                            1.0 - 0.25 * (u / 0.58)
                        } else {
                            0.75 * (1.0 - (u - 0.58) / 0.34).clamp(0., 1.)
                        };
                        let top_line =
                            (0.5 - ((sy - (band_top + 0.5)).abs() - 0.45) / px_u).clamp(0., 1.);
                        let bot_line =
                            (0.5 - ((sy - (band_bot - 0.5)).abs() - 0.45) / px_u).clamp(0., 1.);
                        let line_cov = top_line.max(bot_line);
                        if line_cov > 0.0 {
                            c = blend_over(
                                c,
                                pick_line_rgb,
                                line_cov * pick_line_a * line_fade * cov_card,
                            );
                        }
                    }
                }

                // Crisp 1px Retina outer card hairline
                let border_cov = (0.5 - ((d_card + 0.5).abs() - 0.5) / px_u).clamp(0., 1.);
                if border_cov > 0.0 {
                    c = blend_over(c, border_rgb, border_cov * border_a);
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

/// Compute the active auto-walked row index at `elapsed_ms` given `selected`, `walk`, `row_count`, and `hold_ms`.
fn active_walk_index(
    selected: usize,
    walk: usize,
    row_count: usize,
    hold_ms: u64,
    elapsed_ms: u64,
) -> usize {
    if row_count == 0 {
        return 0;
    }
    let base = selected.min(row_count - 1);
    let span = walk.min(row_count);
    if span <= 1 || hold_ms == 0 {
        return base;
    }
    let step_period = hold_ms.saturating_add(520);
    let steps = (elapsed_ms / step_period) as usize;
    if base < span {
        (base + steps) % span
    } else {
        (base + steps) % row_count
    }
}

fn render_model_mark(model: &MenuModel, dark: bool, w: &mut Window, cx: &mut App) -> AnyElement {
    if let Some(custom) = &model.mark {
        return custom(w, cx);
    }
    let key = model.provider.trim().to_ascii_lowercase();
    if key == "qwen" {
        let qwen_color: Hsla = if dark {
            rgb(0x8173f3).into()
        } else {
            rgb(0x7c6cf0).into()
        };
        return pro_svg("0 0 24 24", QWEN_SVG, 18., qwen_color);
    }
    if let Some(art) = pro_app_artwork(&key, 18., cx) {
        return art;
    }
    let fallback: Hsla = if dark {
        hsla(0., 0., 1., 0.85)
    } else {
        hsla(0., 0., 0., 0.82)
    };
    div()
        .size(px(18.))
        .rounded(px(4.5))
        .bg(fallback.opacity(0.12))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(10.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(fallback)
        .child(
            model
                .name
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "M".to_string()),
        )
        .into_any_element()
}

/// Product-scene model picker card with a warm rose selection band and soft edge fades.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use kui::components::model_menu::ModelMenu;
///
/// let menu = cx.new(|cx| ModelMenu::new("hero-models", cx).with_selected(1));
/// # }
/// ```
pub struct ModelMenu {
    id: ElementId,
    focus: FocusHandle,
    models: Vec<MenuModel>,
    title: Option<SharedString>,
    has_custom_title: bool,
    selected: usize,
    playing: bool,
    hold_ms: u64,
    walk: usize,
    hovered_row: Option<usize>,
    band_slide: Slide,
    row_press: [Slide; 9],
    loading: bool,
    empty: bool,
    clock: Instant,
    plate_cache: Rc<RefCell<CardPlateCache>>,
}

impl ModelMenu {
    /// Construct a [`ModelMenu`] with the 7 default frontier models and `"GPT"` (`1`) selected.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            models: default_menu_models(),
            title: None,
            has_custom_title: false,
            selected: 1,
            playing: true,
            hold_ms: 2200,
            walk: 3,
            hovered_row: None,
            band_slide: Slide::new(ROW_H),
            row_press: [
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
                Slide::snappy(0.),
            ],
            loading: false,
            empty: false,
            clock: cx.background_executor().now(),
            plate_cache: Rc::new(RefCell::new(CardPlateCache::new())),
        }
    }

    /// Replace the model rows rendered inside the menu card.
    pub fn with_models(mut self, models: Vec<MenuModel>) -> Self {
        if !models.is_empty() && self.selected >= models.len() {
            self.selected = 0;
            self.band_slide.reset(0.);
        }
        self.models = models;
        self
    }

    /// Set or clear the quiet header above the rows (`Some("Best models")` by default, `None` omits it).
    pub fn with_title(mut self, title: Option<SharedString>) -> Self {
        self.title = title;
        self.has_custom_title = true;
        self
    }

    /// Set the initially picked model row index (`1` for `"GPT"` by default).
    pub fn with_selected(mut self, selected: usize) -> Self {
        let clamped = if self.models.is_empty() {
            0
        } else {
            selected.min(self.models.len() - 1)
        };
        self.selected = clamped;
        self.band_slide.reset(clamped as f32 * ROW_H);
        self
    }

    /// Control whether the rose highlight band walks through the top rows automatically.
    pub fn with_playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Set how long the band rests on each model in milliseconds (`2200` by default).
    pub fn with_hold(mut self, hold_ms: u64) -> Self {
        self.hold_ms = hold_ms.max(200);
        self
    }

    /// Set how many rows from the top the auto-playing band cycles through (`3` by default).
    pub fn with_walk(mut self, walk: usize) -> Self {
        self.walk = walk.max(1);
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

    /// Return the currently selected base row index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Return whether auto-play is enabled.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Return the configured model slice.
    pub fn models(&self) -> &[MenuModel] {
        &self.models
    }

    /// Pick a model row at runtime and glide the rose highlight band to it.
    pub fn set_selected(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.models.is_empty() {
            return;
        }
        let next = index.min(self.models.len() - 1);
        self.selected = next;
        self.clock = cx.background_executor().now();
        self.band_slide.duration(next as f32 * ROW_H, 0.42, cx);
        cx.notify();
    }

    /// Toggle auto-play at runtime.
    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        if self.playing != playing {
            self.playing = playing;
            self.clock = cx.background_executor().now();
            cx.notify();
        }
    }

    /// Return whether loading skeletons are active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle loading skeletons at runtime.
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
}

impl Render for ModelMenu {
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
                .h(px(SCENE_H as f32))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(300.))
                        .p(px(16.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(84., 11., 4., phase, cx))
                        .children((0..5).map(|i| {
                            pro_skeleton_bar(
                                180. - (i as f32 * 14.),
                                20.,
                                6.,
                                (phase + i as f32 * 0.08).fract(),
                                cx,
                            )
                        })),
                )
                .into_any_element();
        }

        if self.empty || self.models.is_empty() {
            return div()
                .id(pro_eid(&self.id, "empty"))
                .w_full()
                .child(pro_empty_state::<Self>(
                    ProIcon::Sparkles,
                    tr(cx, "No models configured", "暂无可用模型"),
                    tr(
                        cx,
                        "Provide frontier models to preview the gliding picker scene.",
                        "添加模型条目后即可预览滑动高亮菜单。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let row_count = self.models.len().min(9);
        let auto_animate = self.playing && !reduced && self.hovered_row.is_none() && row_count > 1;
        let elapsed_ms = if auto_animate {
            elapsed.as_millis() as u64
        } else {
            0
        };

        let target_row = if let Some(h) = self.hovered_row {
            h.min(row_count - 1)
        } else if auto_animate {
            active_walk_index(
                self.selected,
                self.walk,
                row_count,
                self.hold_ms,
                elapsed_ms,
            )
        } else {
            self.selected.min(row_count - 1)
        };

        let target_y = target_row as f32 * ROW_H;
        self.band_slide.duration(
            target_y,
            if self.hovered_row.is_some() {
                0.34
            } else {
                0.52
            },
            cx,
        );
        let band_y = if reduced {
            target_y
        } else {
            self.band_slide.get(w, cx)
        };

        let resolved_title: Option<SharedString> = if self.has_custom_title {
            self.title.clone()
        } else {
            Some(tr(cx, "Best models", "精选模型").into())
        };
        let has_title = resolved_title.is_some();

        let plate_key = CardPlateKey {
            dark,
            has_title,
            row_count: row_count as u8,
            band_y_q: (band_y * 20.).round() as i32,
        };
        let plate_cache = self.plate_cache.clone();

        let rows_top = CARD_TOP + 12.0 + if has_title { 20.0 } else { 0.0 };
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
        let muted_c: Hsla = if dark {
            hsla(0., 0., 1., 0.45)
        } else {
            hsla(0., 0., 0., 0.45)
        };
        let faint_c: Hsla = if dark {
            hsla(0., 0., 1., 0.40)
        } else {
            hsla(0., 0., 0., 0.36)
        };

        let mut scene = div()
            .relative()
            .w(px(SCENE_W as f32))
            .h(px(SCENE_H as f32))
            .flex_none()
            .overflow_hidden()
            .child(
                canvas(
                    |_, _, _| (),
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

        if let Some(header_text) = resolved_title {
            let header_alpha = scene_mask_alpha(CARD_LEFT + 20., CARD_TOP + 20.);
            scene = scene.child(
                div()
                    .absolute()
                    .left(px(CARD_LEFT + 16.))
                    .top(px(CARD_TOP + 11.))
                    .h(px(16.))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(muted_c)
                    .opacity(header_alpha)
                    .child(header_text),
            );
        }

        let check_ink: Hsla = if dark {
            rgb(0xf4f4f5).into()
        } else {
            rgb(0x18181b).into()
        };

        let mut row_downs = [0.0f32; 9];
        for (i, d) in row_downs.iter_mut().enumerate().take(row_count) {
            *d = self.row_press[i].get(w, cx).clamp(0., 1.);
        }

        let mut rows_stack = div()
            .absolute()
            .left(px(CARD_LEFT + 6.))
            .top(px(rows_top))
            .w(px(264.))
            .flex()
            .flex_col();

        let visible_models: Vec<MenuModel> = self.models.iter().take(row_count).cloned().collect();
        let active_sel = self.selected.min(row_count - 1);
        for (idx, model) in visible_models.iter().enumerate() {
            let is_last = idx + 1 == row_count && row_count >= 5;
            let is_target = idx == target_row;
            let is_checked = idx == active_sel;
            let down = row_downs[idx];
            let row_top = rows_top + idx as f32 * ROW_H;
            let row_alpha = scene_mask_alpha(CARD_LEFT + 40., row_top + ROW_H * 0.5);
            let check_alpha = scene_mask_alpha(CARD_LEFT + 244., row_top + ROW_H * 0.5);
            let mark_el = render_model_mark(model, dark, w, cx);
            let model_name = model.name.clone();
            let emit_name = model.name.clone();
            let name_col = if is_last {
                faint_c
            } else if is_target || is_checked {
                strong_c
            } else {
                ink_c
            };

            rows_stack = rows_stack.child(
                div()
                    .id(pro_eid(&self.id, format!("row-{idx}")))
                    .role(Role::ListItem)
                    .aria_label(model_name.clone())
                    .aria_selected(is_checked)
                    .relative()
                    .w(px(264.))
                    .h(px(ROW_H))
                    .pl(px(10. + 2.2 * down))
                    .pr(px(12. + 1.5 * down))
                    .top(px(0.6 * down))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .cursor_pointer()
                    .opacity(row_alpha)
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        let next = if *hovered { Some(idx) } else { None };
                        if this.hovered_row != next {
                            if *hovered {
                                this.hovered_row = Some(idx);
                            } else if this.hovered_row == Some(idx) {
                                this.hovered_row = None;
                                this.row_press[idx].duration(0., 0.14, cx);
                            }
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.row_press[idx].duration(1., 0.08, cx);
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.row_press[idx].duration(0., 0.14, cx);
                            cx.notify();
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.row_press[idx].duration(0., 0.14, cx);
                            cx.notify();
                        }),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected = idx;
                        this.clock = cx.background_executor().now();
                        cx.emit(ModelMenuEvent::Select {
                            index: idx,
                            name: emit_name.clone(),
                        });
                        cx.notify();
                    }))
                    .child(
                        div()
                            .size(px(18.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .opacity(if is_last { 0.60 } else { 1.0 })
                            .child(mark_el),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_size(px(14.5))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(name_col)
                            .child(model_name),
                    )
                    .when(is_checked, |d| {
                        d.child(
                            div()
                                .size(px(14.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .opacity(check_alpha)
                                .child(pro_icon(ProIcon::Check, 14., check_ink)),
                        )
                    }),
            );
        }
        scene = scene.child(rows_stack);

        let focus_ring = p.focus;
        let a11y = tr(
            cx,
            "Frontier model picker showing DeepSeek, GPT, Claude Opus, Gemini, Mistral Large, Qwen, and Llama",
            "包含 DeepSeek、GPT、Claude Opus、Gemini、Mistral Large、Qwen 与 Llama 的前沿模型选择菜单场景",
        );

        div()
            .id(self.id.clone())
            .role(Role::ListBox)
            .aria_label(a11y)
            .track_focus(&self.focus)
            .tab_index(0)
            .w_full()
            .min_h(px(SCENE_H as f32))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(14.)).border_color(focus_ring))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
                let n = this.models.len().min(9);
                if n == 0 {
                    return;
                }
                match e.keystroke.key.as_str() {
                    "down" | "j" => {
                        this.selected = (this.selected + 1) % n;
                        this.clock = cx.background_executor().now();
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "up" | "k" => {
                        this.selected = (this.selected + n - 1) % n;
                        this.clock = cx.background_executor().now();
                        cx.stop_propagation();
                        cx.notify();
                    }
                    " " => {
                        this.playing = !this.playing;
                        this.clock = cx.background_executor().now();
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "enter" => {
                        let idx = this.selected.min(n - 1);
                        this.row_press[idx].reset(1.);
                        this.row_press[idx].duration(0., 0.16, cx);
                        let name = this.models[idx].name.clone();
                        cx.emit(ModelMenuEvent::Select { index: idx, name });
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(scene)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CARD_LEFT, CARD_TOP, CardPlateKey, ModelMenu, ModelMenuEvent, RASTER_SCALE, ROW_H, SCENE_H,
        SCENE_W, TEX_W, active_walk_index, default_menu_models, rasterize_card_plate,
        scene_mask_alpha,
    };
    use gpui::{
        MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, TestAppContext, point, px, size,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};

    #[test]
    fn default_models_and_walk_cycle_match_reference() {
        let models = default_menu_models();
        assert_eq!(models.len(), 7);
        assert_eq!(models[0].name.as_ref(), "DeepSeek");
        assert_eq!(models[1].name.as_ref(), "GPT");
        assert_eq!(models[6].name.as_ref(), "Llama");

        // Starts at selected=1 ("GPT"), holds for 2200ms, then cycles across top `walk=3` rows (1 -> 2 -> 0 -> 1).
        assert_eq!(active_walk_index(1, 3, 7, 2200, 0), 1);
        assert_eq!(active_walk_index(1, 3, 7, 2200, 2100), 1);
        assert_eq!(active_walk_index(1, 3, 7, 2200, 2800), 2);
        assert_eq!(active_walk_index(1, 3, 7, 2200, 5600), 0);
        assert_eq!(active_walk_index(1, 3, 7, 2200, 8300), 1);
    }

    #[test]
    fn card_plate_renders_rose_band_and_fades_right_and_bottom_edges() {
        assert!(scene_mask_alpha(CARD_LEFT + 40., CARD_TOP + 40.) > 0.99);
        assert!(scene_mask_alpha(SCENE_W as f32 - 2., CARD_TOP + 40.) < 0.02);
        assert!(scene_mask_alpha(CARD_LEFT + 40., SCENE_H as f32 - 4.) < 0.02);

        let img = rasterize_card_plate(CardPlateKey {
            dark: false,
            has_title: true,
            row_count: 7,
            band_y_q: (ROW_H * 20.) as i32,
        });
        let bytes = img.as_bytes(0).expect("BGRA plate bytes");
        // Row 0 ("DeepSeek") is pure white card (`R ~ G ~ B ~ 255`)
        let r0_y = (CARD_TOP + 32. + 16.) as usize * RASTER_SCALE;
        let r0_x = (CARD_LEFT + 30.) as usize * RASTER_SCALE;
        let p0 = (r0_y * TEX_W + r0_x) * 4;
        assert!(bytes[p0] > 245 && bytes[p0 + 1] > 245 && bytes[p0 + 2] > 245);

        // Row 1 ("GPT") has the warm pink band (`R > G` because rose tint reduces green on white)
        let r1_y = (CARD_TOP + 32. + ROW_H + 16.) as usize * RASTER_SCALE;
        let p1 = (r1_y * TEX_W + r0_x) * 4;
        assert!(
            bytes[p1 + 2] > bytes[p1 + 1] + 12,
            "expected warm rose tint on row 1, got R={} G={}",
            bytes[p1 + 2],
            bytes[p1 + 1]
        );

        // Far right edge of card is faded out (`alpha < 15`)
        let right_edge = (r1_y * TEX_W + (TEX_W - 6)) * 4;
        assert!(bytes[right_edge + 3] < 15);
    }

    #[gpui::test]
    fn hover_and_click_row_glides_liquid_band_and_selects_active_model(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let exec = cx.executor();
        let events: Rc<RefCell<Vec<ModelMenuEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let ev_sink = events.clone();

        let (view, vcx) = cx.add_window_view(|_, cx| {
            let menu = ModelMenu::new("mm-test", cx).with_playing(false);
            cx.subscribe_self(move |_, ev: &ModelMenuEvent, _| {
                ev_sink.borrow_mut().push(ev.clone());
            })
            .detach();
            menu
        });
        vcx.simulate_resize(size(px(480.), px(400.)));
        vcx.run_until_parked();

        // Scene is centered `(SCENE_W=384, SCENE_H=340)` inside `(480, 400)` -> ox=48, oy=30.
        // Row 3 ("Gemini") sits at `y = 30 + CARD_TOP(34) + 32 + 3 * ROW_H(32) + 16 = 208`
        let gemini_pt = point(
            px(48. + CARD_LEFT + 80.),
            px(30. + CARD_TOP + 32. + 3. * ROW_H + 16.),
        );
        vcx.simulate_event(MouseMoveEvent {
            position: gemini_pt,
            pressed_button: None,
            modifiers: Default::default(),
        });
        exec.advance_clock(Duration::from_millis(360));
        vcx.update(|w, app| w.simulate_next_frame(app));
        vcx.run_until_parked();

        vcx.update(|_, app| {
            let mm = view.read(app);
            assert_eq!(mm.hovered_row, Some(3));
            assert!((mm.band_slide.value(app) - 3. * ROW_H).abs() < 4.0);
        });

        vcx.simulate_event(MouseDownEvent {
            position: gemini_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
            first_mouse: false,
        });
        vcx.simulate_event(MouseUpEvent {
            position: gemini_pt,
            button: MouseButton::Left,
            modifiers: Default::default(),
            click_count: 1,
        });
        vcx.run_until_parked();

        assert_eq!(vcx.update(|_, app| view.read(app).selected()), 3);
        assert!(events.borrow().iter().any(|e| {
            *e == ModelMenuEvent::Select {
                index: 3,
                name: "Gemini".into(),
            }
        }));
    }
}
