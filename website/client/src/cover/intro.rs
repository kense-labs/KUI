//! KUI product-page entrance choreography: CAD drafting trace, twin-frame 3D isometric extrusion,
//! kinetic `KUI`/`GPUI` typography sweep, and floating hero coordinate handoff.

use super::{Frame, Result, css, element, flag, listen, viewport};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, KeyboardEvent, MediaQueryList, Window};

const SVG_NS: &str = "http://www.w3.org/2000/svg";

const DRAW_END_MS: f64 = 800.0;
const EXTRUDE_AT_MS: f64 = 800.0;
const EXTRUDE_DUR_MS: f64 = 420.0;
const HANDOFF_AT_MS: f64 = 1170.0;
const MOVE_DUR_MS: f64 = 700.0;
const REVEAL_DUR_MS: f64 = 320.0;
const HANDOFF_END_MS: f64 = HANDOFF_AT_MS + MOVE_DUR_MS;
const TOTAL_MS: f64 = HANDOFF_END_MS + REVEAL_DUR_MS;

/// Exact tangent and corner vertices of KUI's twin overlapping rounded frames + center core ring.
const KUI_EXTRUDE_VERTICES: [(f64, f64); 26] = [
    // Frame 1 (Desktop: x=116..316, y=76..276, rx=36)
    (152.0, 76.0),
    (280.0, 76.0),
    (305.46, 86.54),
    (316.0, 112.0),
    (316.0, 240.0),
    (305.46, 265.46),
    (280.0, 276.0),
    (152.0, 276.0),
    (126.54, 265.46),
    (116.0, 240.0),
    (116.0, 112.0),
    (126.54, 86.54),
    // Frame 2 (Web: x=204..404, y=164..364, rx=36)
    (240.0, 164.0),
    (368.0, 164.0),
    (393.46, 174.54),
    (404.0, 200.0),
    (404.0, 328.0),
    (393.46, 353.46),
    (368.0, 364.0),
    (240.0, 364.0),
    (214.54, 353.46),
    (204.0, 328.0),
    (204.0, 200.0),
    (214.54, 174.54),
    // Shared precision core ring (cx=260, cy=220, r=24)
    (243.03, 203.03),
    (276.97, 236.97),
];

fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

fn ease_out_cubic(t: f64) -> f64 {
    let u = 1.0 - clamp01(t);
    1.0 - u * u * u
}

fn ease_out_quart(t: f64) -> f64 {
    let u = 1.0 - clamp01(t);
    1.0 - u * u * u * u
}

fn ease_in_cubic(t: f64) -> f64 {
    let t = clamp01(t);
    t * t * t
}

fn ease_in_out(t: f64) -> f64 {
    let t = clamp01(t);
    t * t * (3.0 - 2.0 * t)
}

/// Smooth fast-out slow-in curve closely matching `cubic-bezier(0.4, 0, 0.2, 1)`.
fn ease_move(t: f64) -> f64 {
    let t = clamp01(t);
    if t < 0.38 {
        let u = t / 0.38;
        0.42 * u * u
    } else {
        let u = 1.0 - (t - 0.38) / 0.62;
        1.0 - 0.58 * u * u * u
    }
}

fn lerp_u8(a: f64, b: f64, t: f64) -> u8 {
    (a + (b - a) * t).round().clamp(0.0, 255.0) as u8
}

struct WordSpec {
    el: HtmlElement,
    right: bool,
    delay: f64,
    base_left: f64,
    base_right: f64,
}

struct ProductIntro {
    window: Window,
    document: Document,
    hero: Element,
    curtain: HtmlElement,
    backdrop: HtmlElement,
    stage: HtmlElement,
    base_group: Element,
    edges_group: Element,
    edge_lines: Vec<Element>,
    guide_paths: Vec<Element>,
    outline_paths: Vec<Element>,
    words: Vec<WordSpec>,
    target_title: HtmlElement,
    target_lead: HtmlElement,
    float_title: HtmlElement,
    float_title_prefix: Option<HtmlElement>,
    float_lead: HtmlElement,
    topbar: Option<HtmlElement>,
    topbar_h: f64,
    guide_bot: Option<HtmlElement>,
    body: Option<HtmlElement>,
    start: Option<f64>,
    settled: bool,
    done: bool,
}

impl ProductIntro {
    fn is_dark(&self) -> bool {
        self.document
            .document_element()
            .and_then(|el| el.get_attribute("data-theme"))
            .as_deref()
            == Some("dark")
    }

    fn clear_topbar_and_bot(&self) {
        if let Some(bar) = &self.topbar {
            let _ = bar.style().remove_property("opacity");
            let _ = bar.style().remove_property("transform");
            let _ = bar.style().remove_property("pointer-events");
        }
        if let Some(bot) = &self.guide_bot {
            let _ = bot.style().remove_property("opacity");
        }
    }

    fn finish(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        self.float_title.remove();
        self.float_lead.remove();
        self.curtain.set_hidden(true);
        let _ = self.curtain.style().remove_property("opacity");
        flag(&self.hero, "data-kui-intro-playing", false);
        flag(&self.hero, "data-kui-intro-settled", true);
        if let Some(body) = &self.body {
            flag(body, "data-kui-intro-active", false);
        }
        self.clear_topbar_and_bot();
    }

    fn tick(&mut self, now: f64) -> bool {
        if self.done {
            return false;
        }
        let start = *self.start.get_or_insert(now);
        let elapsed = (now - start).max(0.0);
        if elapsed >= TOTAL_MS {
            self.finish();
            return false;
        }

        let (vw, vh) = viewport(&self.window);
        let dark = self.is_dark();

        // 1. CAD blueprint construction lines (`0..800ms` trace, `800..1020ms` dissolve)
        let guide_fade = 1.0 - ease_out_cubic((elapsed - EXTRUDE_AT_MS) / 220.0);
        for (i, path) in self.guide_paths.iter().enumerate() {
            let delay = (i as f64) * 14.0;
            let p = ease_out_cubic((elapsed - delay) / 720.0);
            let alpha = ease_out_cubic((elapsed - delay) / 100.0) * 0.9 * guide_fade;
            let _ = path.set_attribute(
                "style",
                &format!("opacity:{alpha:.3};stroke-dashoffset:{:.4}", 1.0 - p),
            );
        }

        // 2. Twin-frame KUI emblem outline trace (`0..800ms`)
        for (i, path) in self.outline_paths.iter().enumerate() {
            let delay = ((i as f64) * 28.0).min(60.0);
            let p = ease_in_out((elapsed - delay) / (DRAW_END_MS - delay));
            let alpha = ease_out_cubic((elapsed - delay) / 90.0);
            let _ = path.set_attribute(
                "style",
                &format!("opacity:{alpha:.3};stroke-dashoffset:{:.4}", 1.0 - p),
            );
        }

        // 3. 3-stage kinetic `KUI` / `GPUI` background word sweep
        for w in &self.words {
            let start_x = if w.right {
                -w.base_right - 40.0
            } else {
                vw - w.base_left + 40.0
            };
            let drift_x = if w.right { vw * 0.045 } else { -vw * 0.045 };
            let exit_x = if w.right {
                vw - w.base_left + vw * 0.08
            } else {
                -w.base_right - vw * 0.08
            };
            let arrive_end = w.delay + 600.0;
            let tx = if elapsed <= arrive_end {
                let p = ease_out_quart((elapsed - w.delay) / 600.0);
                start_x + (0.0 - start_x) * p
            } else if elapsed <= HANDOFF_AT_MS {
                let p = clamp01((elapsed - arrive_end) / (HANDOFF_AT_MS - arrive_end).max(1.0));
                drift_x * p
            } else {
                let p = ease_in_cubic((elapsed - HANDOFF_AT_MS) / 600.0);
                drift_x + (exit_x - drift_x) * p
            };
            css(&w.el, "transform", &format!("translate3d({tx:.2}px,0,0)"));
        }

        // 4. Isometric 3D wireframe extrusion (`800..1220ms`) + stage dissolve (`1420..1870ms`)
        let ext = ease_out_cubic((elapsed - EXTRUDE_AT_MS) / EXTRUDE_DUR_MS);
        let dx = 26.0 * ext;
        let dy = -34.0 * ext;
        let _ = self.base_group.set_attribute(
            "style",
            &format!(
                "opacity:{:.3};transform:translate({dx:.2}px,{dy:.2}px)",
                ext * 0.92
            ),
        );
        let _ = self
            .edges_group
            .set_attribute("style", &format!("opacity:{:.3}", ext * 0.82));
        for (line, &(vx, vy)) in self.edge_lines.iter().zip(KUI_EXTRUDE_VERTICES.iter()) {
            let _ = line.set_attribute("x2", &format!("{:.2}", vx + dx));
            let _ = line.set_attribute("y2", &format!("{:.2}", vy + dy));
        }

        let stage_out = ease_out_cubic((elapsed - (HANDOFF_AT_MS - 30.0)) / 360.0);
        let stage_alpha = 1.0 - stage_out;
        let stage_scale = 1.0 + ext * 0.032 - stage_out * 0.085;
        let stage_y = -50.0 - stage_out * 12.0;
        css(
            &self.stage,
            "transform",
            &format!("translate(-50%, {stage_y:.2}%) scale({stage_scale:.4})"),
        );
        css(&self.stage, "opacity", &format!("{stage_alpha:.3}"));

        // 5. Floating `#cover-title` & `.lead` coordinate + color handoff (`800..1870ms`)
        let title_rect = self.target_title.get_bounding_client_rect();
        let lead_rect = self.target_lead.get_bounding_client_rect();
        let gap = (lead_rect.top() - title_rect.bottom()).clamp(18.0, 96.0);
        let stage_anchor = (vh * 0.75)
            .min(vh - title_rect.height() - lead_rect.height() - 56.0)
            .max(vh * 0.54);

        let appear = ease_out_cubic((elapsed - EXTRUDE_AT_MS) / 280.0);
        let move_p = ease_move((elapsed - HANDOFF_AT_MS) / MOVE_DUR_MS);
        let color_p = ease_in_out((elapsed - HANDOFF_AT_MS) / MOVE_DUR_MS);

        let title_dy = (stage_anchor - title_rect.top()) * (1.0 - move_p);
        let lead_dy =
            (stage_anchor + title_rect.height() + gap.min(28.0) - lead_rect.top()) * (1.0 - move_p);

        css(
            &self.float_title,
            "left",
            &format!("{:.2}px", title_rect.left()),
        );
        css(
            &self.float_title,
            "top",
            &format!("{:.2}px", title_rect.top()),
        );
        css(
            &self.float_title,
            "width",
            &format!("{:.2}px", title_rect.width()),
        );
        css(
            &self.float_title,
            "height",
            &format!("{:.2}px", title_rect.height()),
        );
        css(&self.float_title, "opacity", &format!("{appear:.3}"));
        css(
            &self.float_title,
            "transform",
            &format!("translate3d(0,{title_dy:.2}px,0)"),
        );

        let (tr_to, tg_to, tb_to) = if dark {
            (255.0, 255.0, 255.0)
        } else {
            (0.0, 0.0, 0.0)
        };
        let title_rgb = format!(
            "rgb({},{},{})",
            lerp_u8(245.0, tr_to, color_p),
            lerp_u8(247.0, tg_to, color_p),
            lerp_u8(250.0, tb_to, color_p),
        );
        css(&self.float_title, "color", &title_rgb);
        if let Some(prefix) = &self.float_title_prefix {
            css(prefix, "color", &title_rgb);
        }

        css(
            &self.float_lead,
            "left",
            &format!("{:.2}px", lead_rect.left()),
        );
        css(
            &self.float_lead,
            "top",
            &format!("{:.2}px", lead_rect.top()),
        );
        css(
            &self.float_lead,
            "width",
            &format!("{:.2}px", lead_rect.width()),
        );
        css(
            &self.float_lead,
            "height",
            &format!("{:.2}px", lead_rect.height()),
        );
        css(&self.float_lead, "opacity", &format!("{appear:.3}"));
        css(
            &self.float_lead,
            "transform",
            &format!("translate3d(0,{lead_dy:.2}px,0)"),
        );

        let (lr_to, lg_to, lb_to) = if dark {
            (163.0, 163.0, 163.0)
        } else {
            (82.0, 82.0, 82.0)
        };
        let lead_rgb = format!(
            "rgb({},{},{})",
            lerp_u8(216.0, lr_to, color_p),
            lerp_u8(220.0, lg_to, color_p),
            lerp_u8(228.0, lb_to, color_p),
        );
        css(&self.float_lead, "color", &lead_rgb);

        // 6. Backdrop cross-fade from `#171717` to page surface (`1170..1870ms`)
        let (bg_to_r, bg_to_g, bg_to_b) = if dark {
            (0.0, 0.0, 0.0)
        } else {
            (255.0, 255.0, 255.0)
        };
        let bg_rgb = format!(
            "rgb({},{},{})",
            lerp_u8(23.0, bg_to_r, color_p),
            lerp_u8(23.0, bg_to_g, color_p),
            lerp_u8(23.0, bg_to_b, color_p),
        );
        css(&self.backdrop, "background-color", &bg_rgb);

        // 7. Topbar slide-down & companion `.guide` fade-in (`1170..1770ms`)
        let bar_p = ease_out_cubic((elapsed - HANDOFF_AT_MS) / 600.0);
        if let Some(bar) = &self.topbar {
            if elapsed >= HANDOFF_AT_MS + 600.0 {
                let _ = bar.style().remove_property("opacity");
                let _ = bar.style().remove_property("transform");
                let _ = bar.style().remove_property("pointer-events");
            } else {
                let bar_y = -(self.topbar_h + 16.0) * (1.0 - bar_p);
                css(bar, "opacity", &format!("{bar_p:.3}"));
                css(bar, "transform", &format!("translate3d(0,{bar_y:.2}px,0)"));
                css(bar, "pointer-events", "none");
            }
        }
        if let Some(bot) = &self.guide_bot {
            if elapsed >= HANDOFF_AT_MS + 600.0 {
                let _ = bot.style().remove_property("opacity");
            } else {
                css(bot, "opacity", &format!("{bar_p:.3}"));
            }
        }

        // 8. Final seamless handoff & overlay dissolve (`1870..2190ms`)
        if elapsed >= HANDOFF_END_MS {
            if !self.settled {
                self.settled = true;
                flag(&self.hero, "data-kui-intro-playing", false);
                flag(&self.hero, "data-kui-intro-settled", true);
            }
            let curtain_alpha = 1.0 - ease_in_out((elapsed - HANDOFF_END_MS) / REVEAL_DUR_MS);
            css(&self.curtain, "opacity", &format!("{curtain_alpha:.3}"));
        }

        true
    }
}

pub(super) fn init(
    window: &Window,
    document: &Document,
    root: &Element,
    reduced: &MediaQueryList,
) -> Result {
    if reduced.matches() {
        return Ok(());
    }
    if window.scroll_y().unwrap_or_default() > 8.0
        || !window.location().hash().unwrap_or_default().is_empty()
    {
        return Ok(());
    }
    let Some(hero) = root.query_selector("[data-kui-intro-hero]")? else {
        return Ok(());
    };
    let Ok(curtain) = element::<HtmlElement>(root, "[data-kui-product-intro]") else {
        return Ok(());
    };

    let key = curtain
        .get_attribute("data-intro-key")
        .unwrap_or_else(|| "kui:product-intro:v1".into());
    if let Ok(Some(storage)) = window.session_storage() {
        let armed = storage
            .get_item("kui:product-intro:armed")
            .ok()
            .flatten()
            .as_deref()
            == Some("1");
        if armed {
            let _ = storage.remove_item("kui:product-intro:armed");
            let _ = storage.set_item(&key, "1");
        } else if storage.get_item(&key).ok().flatten().as_deref() == Some("1") {
            return Ok(());
        } else {
            let _ = storage.set_item(&key, "1");
        }
    }

    let backdrop: HtmlElement = element(&curtain, ".kui-intro-backdrop")?;
    let stage: HtmlElement = element(&curtain, ".kui-intro-stage")?;
    let base_group = curtain
        .query_selector(".kui-intro-extrude-base")?
        .ok_or("kui-extrude-base")?;
    let edges_group = curtain
        .query_selector(".kui-intro-extrude-edges")?
        .ok_or("kui-extrude-edges")?;

    let mut edge_lines = Vec::with_capacity(KUI_EXTRUDE_VERTICES.len());
    for &(vx, vy) in &KUI_EXTRUDE_VERTICES {
        let line = document.create_element_ns(Some(SVG_NS), "line")?;
        let x = format!("{vx:.2}");
        let y = format!("{vy:.2}");
        line.set_attribute("x1", &x)?;
        line.set_attribute("y1", &y)?;
        line.set_attribute("x2", &x)?;
        line.set_attribute("y2", &y)?;
        edges_group.append_child(&line)?;
        edge_lines.push(line);
    }

    let collect = |sel: &str| -> Result<Vec<Element>> {
        let list = curtain.query_selector_all(sel)?;
        Ok((0..list.length())
            .filter_map(|i| list.item(i)?.dyn_into().ok())
            .collect())
    };
    let guide_paths = collect(".kui-intro-guides [data-kui-trace]")?;
    let outline_paths = collect(".kui-intro-outline [data-kui-trace]")?;

    let target_title: HtmlElement = element(&hero, "[data-kui-intro-title]")?;
    let target_lead: HtmlElement = element(&hero, "[data-kui-intro-lead]")?;

    // Clone `#cover-title` deeply inside `.kui-cover-page` so its 3D `.reel` ticket badge matches 100%
    let float_title: HtmlElement = target_title.clone_node_with_deep(true)?.dyn_into()?;
    let _ = float_title.remove_attribute("id");
    let _ = float_title.remove_attribute("data-kui-intro-title");
    let _ = float_title.set_attribute("aria-hidden", "true");
    float_title.set_class_name("kui-intro-floater kui-intro-floater--title");
    let hits = float_title.query_selector_all("label")?;
    for i in 0..hits.length() {
        if let Some(lbl) = hits.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
            let _ = lbl.remove_attribute("for");
        }
    }
    let float_title_prefix = float_title
        .query_selector(".title-prefix")?
        .and_then(|n| n.dyn_into::<HtmlElement>().ok());

    let float_lead: HtmlElement = target_lead.clone_node_with_deep(true)?.dyn_into()?;
    let _ = float_lead.remove_attribute("data-kui-intro-lead");
    let _ = float_lead.set_attribute("aria-hidden", "true");
    float_lead.set_class_name("lead kui-intro-floater kui-intro-floater--lead");

    let topbar = document
        .query_selector("header.topbar")?
        .and_then(|n| n.dyn_into::<HtmlElement>().ok());
    let topbar_h = topbar
        .as_ref()
        .map_or(64.0, |b| b.get_bounding_client_rect().height().max(48.0));
    let guide_bot = root
        .query_selector(".guide")?
        .and_then(|n| n.dyn_into::<HtmlElement>().ok());

    let body = document.body();
    flag(&hero, "data-kui-intro-playing", true);
    if let Some(b) = &body {
        flag(b, "data-kui-intro-active", true);
    }
    curtain.set_hidden(false);
    curtain.append_child(&float_title)?;
    curtain.append_child(&float_lead)?;

    // Measure word rects after unhiding curtain so `base_left` / `base_right` are exact
    let word_nodes = curtain.query_selector_all("[data-kui-intro-word]")?;
    let mut words = Vec::new();
    for i in 0..word_nodes.length() {
        if let Some(el) = word_nodes
            .item(i)
            .and_then(|n| n.dyn_into::<HtmlElement>().ok())
        {
            let _ = el.style().remove_property("transform");
            let r = el.get_bounding_client_rect();
            let right = el.get_attribute("data-intro-direction").as_deref() == Some("right");
            words.push(WordSpec {
                el,
                right,
                delay: f64::from(i) * 70.0,
                base_left: r.left(),
                base_right: r.right(),
            });
        }
    }

    let state = Rc::new(RefCell::new(ProductIntro {
        window: window.clone(),
        document: document.clone(),
        hero,
        curtain,
        backdrop,
        stage,
        base_group,
        edges_group,
        edge_lines,
        guide_paths,
        outline_paths,
        words,
        target_title,
        target_lead,
        float_title,
        float_title_prefix,
        float_lead,
        topbar,
        topbar_h,
        guide_bot,
        body,
        start: None,
        settled: false,
        done: false,
    }));

    // Paint initial t=0 state synchronously so Frame 0 has zero jump
    if let Ok(mut guard) = state.try_borrow_mut()
        && let Some(perf) = window.performance()
    {
        let now = perf.now();
        let _ = guard.tick(now);
    }

    let frame = Frame::new(window, {
        let state = state.clone();
        move |now| state.borrow_mut().tick(now)
    });

    listen(window, "scroll", {
        let state = state.clone();
        let win = window.clone();
        move |_| {
            if win.scroll_y().unwrap_or_default() > 24.0 {
                state.borrow_mut().finish();
            }
        }
    })?;
    listen(window, "resize", {
        let state = state.clone();
        move |_| state.borrow_mut().finish()
    })?;
    listen(document, "keydown", {
        let state = state.clone();
        move |ev| {
            if let Some(k) = ev.dyn_ref::<KeyboardEvent>()
                && matches!(k.key().as_str(), "Escape" | "Tab")
            {
                state.borrow_mut().finish();
            }
        }
    })?;
    listen(reduced, "change", {
        let state = state.clone();
        let media = reduced.clone();
        move |_| {
            if media.matches() {
                state.borrow_mut().finish();
            }
        }
    })?;

    frame.request();
    std::mem::forget(frame);
    Ok(())
}
