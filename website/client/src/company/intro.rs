//! Blueprint drafting, 3D isometric wireframe extrusion, and hero title handoff.

use crate::cover::{Frame, Result, css, element, flag, listen, viewport};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, KeyboardEvent, MediaQueryList, Window};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const DURATION_MS: f64 = 2360.0;

/// Architectural vertices of the Kense geometric `K` mark in `svg::intro_draft()` viewBox space.
const EXTRUDE_VERTICES: [(f64, f64); 21] = [
    // Vertical stem
    (124.0, 72.0),
    (144.0, 52.0),
    (184.0, 52.0),
    (204.0, 72.0),
    (204.0, 368.0),
    (184.0, 388.0),
    (144.0, 388.0),
    (124.0, 368.0),
    // Upper arm
    (224.0, 206.0),
    (344.0, 86.0),
    (372.3, 86.0),
    (398.0, 111.7),
    (398.0, 140.0),
    (302.0, 236.0),
    // Lower arm
    (224.0, 214.0),
    (298.0, 214.0),
    (400.0, 316.0),
    (400.0, 344.3),
    (374.3, 370.0),
    (346.0, 370.0),
    (224.0, 248.0),
];

fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

fn ease_out_cubic(t: f64) -> f64 {
    let u = 1.0 - clamp01(t);
    1.0 - u * u * u
}

fn ease_in_out(t: f64) -> f64 {
    let t = clamp01(t);
    t * t * (3.0 - 2.0 * t)
}

struct Intro {
    window: Window,
    hero: Element,
    curtain: HtmlElement,
    backdrop: HtmlElement,
    stage: HtmlElement,
    base_group: Element,
    edges_group: Element,
    edge_lines: Vec<Element>,
    guide_paths: Vec<Element>,
    outline_paths: Vec<Element>,
    words: Vec<(HtmlElement, f64)>,
    target_title: HtmlElement,
    target_sub: HtmlElement,
    float_title: HtmlElement,
    float_sub: HtmlElement,
    body: Option<HtmlElement>,
    start: Option<f64>,
    done: bool,
}

impl Intro {
    fn finish(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        self.float_title.remove();
        self.float_sub.remove();
        self.curtain.set_hidden(true);
        flag(&self.hero, "data-kense-intro-playing", false);
        if let Some(body) = &self.body {
            flag(body, "data-kense-intro-active", false);
        }
    }

    fn position_floater(
        &self,
        floater: &HtmlElement,
        target: &HtmlElement,
        stage_y: f64,
        handoff: f64,
        appear: f64,
    ) {
        let (vw, _) = viewport(&self.window);
        let rect = target.get_bounding_client_rect();
        let fw = f64::from(floater.offset_width()).max(rect.width());
        let fh = f64::from(floater.offset_height()).max(rect.height());
        let start_x = (vw - fw) * 0.5;
        let start_y = stage_y;
        let end_x = rect.left() + (rect.width() - fw) * 0.5;
        let end_y = rect.top() + (rect.height() - fh) * 0.5;
        let h = ease_in_out(handoff);
        let x = start_x + (end_x - start_x) * h;
        let y = start_y + (end_y - start_y) * h + (1.0 - appear) * 14.0;
        css(floater, "left", "0px");
        css(floater, "top", "0px");
        css(floater, "width", &format!("{fw:.1}px"));
        css(
            floater,
            "transform",
            &format!("translate3d({x:.2}px,{y:.2}px,0)"),
        );
        let alpha = appear * (1.0 - clamp01((handoff - 0.86) / 0.14));
        css(floater, "opacity", &format!("{alpha:.3}"));
        css(
            floater,
            "color",
            if handoff < 0.55 {
                "#ffffff"
            } else {
                "var(--ink)"
            },
        );
    }

    fn tick(&mut self, now: f64) -> bool {
        if self.done {
            return false;
        }
        let start = *self.start.get_or_insert(now);
        let elapsed = (now - start).max(0.0);
        if elapsed >= DURATION_MS {
            self.finish();
            return false;
        }

        // Phase 1: CAD guide & outline stroke trace (0..920ms)
        for (i, path) in self.guide_paths.iter().enumerate() {
            let delay = (i as f64) * 38.0;
            let p = ease_out_cubic((elapsed - delay) / 520.0);
            let _ = path.set_attribute(
                "style",
                &format!("opacity:{:.3};stroke-dashoffset:{:.4}", p * 0.85, 1.0 - p),
            );
        }
        for (i, path) in self.outline_paths.iter().enumerate() {
            let delay = 180.0 + (i as f64) * 95.0;
            let p = ease_out_cubic((elapsed - delay) / 620.0);
            let _ = path.set_attribute(
                "style",
                &format!("opacity:{:.3};stroke-dashoffset:{:.4}", p, 1.0 - p),
            );
        }

        // Kinetic background typography sweep
        let word_u = clamp01(elapsed / 1950.0);
        for (word, dir) in &self.words {
            let shift = (0.42 - word_u * 0.84) * 64.0 * (*dir);
            css(word, "transform", &format!("translate3d({shift:.2}vw,0,0)"));
        }

        // Phase 2: Isometric 3D wireframe extrusion (680..1520ms)
        let ext = ease_out_cubic((elapsed - 680.0) / 760.0);
        let dx = 24.0 * ext;
        let dy = -32.0 * ext;
        let _ = self.base_group.set_attribute(
            "style",
            &format!(
                "opacity:{:.3};transform:translate({dx:.2}px,{dy:.2}px)",
                ext * 0.9
            ),
        );
        let _ = self
            .edges_group
            .set_attribute("style", &format!("opacity:{:.3}", ext * 0.78));
        for (line, &(vx, vy)) in self.edge_lines.iter().zip(EXTRUDE_VERTICES.iter()) {
            let _ = line.set_attribute("x2", &format!("{:.2}", vx + dx));
            let _ = line.set_attribute("y2", &format!("{:.2}", vy + dy));
        }

        // Phase 3: Coordinate handoff & curtain dissolve (1520..2360ms)
        let title_appear = ease_out_cubic((elapsed - 620.0) / 480.0);
        let sub_appear = ease_out_cubic((elapsed - 740.0) / 480.0);
        let handoff = clamp01((elapsed - 1480.0) / 780.0);
        let curtain_fade = 1.0 - ease_in_out((elapsed - 1560.0) / 720.0);

        let (_, vh) = viewport(&self.window);
        let stage_bottom = vh * 0.44 + f64::from(self.stage.offset_height()) * 0.46;
        self.position_floater(
            &self.float_title,
            &self.target_title,
            stage_bottom + 8.0,
            handoff,
            title_appear,
        );
        self.position_floater(
            &self.float_sub,
            &self.target_sub,
            stage_bottom + 66.0,
            handoff,
            sub_appear,
        );

        let stage_scale = 1.0 + ext * 0.035 - ease_in_out(handoff) * 0.12;
        let stage_y = -50.0 - ease_in_out(handoff) * 14.0;
        css(
            &self.stage,
            "transform",
            &format!("translate(-50%, {stage_y:.2}%) scale({stage_scale:.4})"),
        );
        css(&self.stage, "opacity", &format!("{curtain_fade:.3}"));
        css(&self.backdrop, "opacity", &format!("{curtain_fade:.3}"));
        for (word, _) in &self.words {
            css(word, "opacity", &format!("{curtain_fade:.3}"));
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
    let Some(hero) = root.query_selector("[data-kense-hero]")? else {
        return Ok(());
    };
    let Ok(curtain) = element::<HtmlElement>(&hero, "[data-kense-intro]") else {
        return Ok(());
    };
    let key = curtain
        .get_attribute("data-intro-key")
        .unwrap_or_else(|| "kense:home-intro:v1".into());
    if let Ok(Some(storage)) = window.session_storage() {
        if storage.get_item(&key).ok().flatten().as_deref() == Some("1") {
            return Ok(());
        }
        let _ = storage.set_item(&key, "1");
    }

    let backdrop: HtmlElement = element(&curtain, ".kense-intro-backdrop")?;
    let stage: HtmlElement = element(&curtain, ".kense-intro-stage")?;
    let base_group = curtain
        .query_selector(".kense-intro-extrude-base")?
        .ok_or("extrude-base")?;
    let edges_group = curtain
        .query_selector(".kense-intro-extrude-edges")?
        .ok_or("extrude-edges")?;

    let mut edge_lines = Vec::with_capacity(EXTRUDE_VERTICES.len());
    for &(vx, vy) in &EXTRUDE_VERTICES {
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
    let guide_paths = collect(".kense-intro-guides [data-intro-trace]")?;
    let outline_paths = collect(".kense-intro-outline [data-intro-trace]")?;

    let word_nodes = curtain.query_selector_all("[data-intro-word]")?;
    let mut words = Vec::new();
    for i in 0..word_nodes.length() {
        if let Some(el) = word_nodes
            .item(i)
            .and_then(|n| n.dyn_into::<HtmlElement>().ok())
        {
            let dir = if el.get_attribute("data-intro-direction").as_deref() == Some("right") {
                -1.0
            } else {
                1.0
            };
            words.push((el, dir));
        }
    }

    let target_title: HtmlElement = element(&hero, "[data-intro-title]")?;
    let target_sub: HtmlElement = element(&hero, "[data-intro-name]")?;

    let prefix_txt = target_title
        .query_selector(".kense-solution-prefix")?
        .and_then(|e| e.text_content())
        .unwrap_or_default();
    let belt_txt = target_title
        .query_selector(".kense-belt-anchor")?
        .and_then(|e| e.text_content())
        .map(|s| s.trim().to_owned())
        .unwrap_or_default();
    let suffix_txt = target_title
        .query_selector(".kense-solution-suffix")?
        .and_then(|e| e.text_content())
        .unwrap_or_default();
    let clean_title = if prefix_txt.is_empty() && suffix_txt.is_empty() {
        target_title.text_content().unwrap_or_default()
    } else {
        format!("{prefix_txt} {belt_txt} {suffix_txt}")
    };

    let float_title: HtmlElement = document.create_element("div")?.dyn_into()?;
    float_title.set_class_name("kense-intro-floating-title");
    float_title.set_text_content(Some(&clean_title));
    curtain.append_child(&float_title)?;

    let float_sub: HtmlElement = document.create_element("div")?.dyn_into()?;
    float_sub.set_class_name("kense-intro-floating-title kense-intro-floating-sub");
    float_sub.set_text_content(target_sub.text_content().as_deref());
    curtain.append_child(&float_sub)?;

    let body = document.body();
    flag(&hero, "data-kense-intro-playing", true);
    if let Some(b) = &body {
        flag(b, "data-kense-intro-active", true);
    }
    curtain.set_hidden(false);

    let state = Rc::new(RefCell::new(Intro {
        window: window.clone(),
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
        target_sub,
        float_title,
        float_sub,
        body,
        start: None,
        done: false,
    }));

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
