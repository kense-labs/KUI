//! Blueprint drafting, 3D isometric wireframe extrusion, and zero-reflow hero title handoff.

use crate::cover::{Frame, Result, css, element, flag, listen, viewport};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlElement, KeyboardEvent, MediaQueryList, Window};

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const DURATION_MS: f64 = 2080.0;
const EXTRUDE_DX: f64 = 24.0;
const EXTRUDE_DY: f64 = -32.0;

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

#[derive(Clone, Copy)]
struct FloaterTrack {
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
}

struct TracePath {
    el: Element,
    settled: bool,
}

struct Intro {
    document: Document,
    hero: Element,
    curtain: HtmlElement,
    backdrop: HtmlElement,
    stage: HtmlElement,
    base_group: Element,
    edges_group: Element,
    guide_paths: Vec<TracePath>,
    outline_paths: Vec<TracePath>,
    words: Vec<(HtmlElement, f64)>,
    float_title: HtmlElement,
    float_sub: HtmlElement,
    title_track: FloaterTrack,
    sub_track: FloaterTrack,
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
        if let Ok(ev) = Event::new("kense:intro-done") {
            let _ = self.document.dispatch_event(&ev);
        }
    }

    fn apply_floater(floater: &HtmlElement, track: FloaterTrack, handoff: f64, appear: f64) {
        let h = ease_in_out(handoff);
        let x = track.start_x + (track.end_x - track.start_x) * h;
        let y = track.start_y + (track.end_y - track.start_y) * h + (1.0 - appear) * 14.0;
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

        // Phase 1: CAD guide & outline stroke trace (0..820ms) — zero updates once settled
        for (i, item) in self.guide_paths.iter_mut().enumerate() {
            if item.settled {
                continue;
            }
            let delay = (i as f64) * 32.0;
            if elapsed < delay {
                continue;
            }
            let p = ease_out_cubic((elapsed - delay) / 460.0);
            let _ = item.el.set_attribute(
                "style",
                &format!("opacity:{:.3};stroke-dashoffset:{:.4}", p * 0.85, 1.0 - p),
            );
            if p >= 0.999 {
                item.settled = true;
            }
        }
        for (i, item) in self.outline_paths.iter_mut().enumerate() {
            if item.settled {
                continue;
            }
            let delay = 150.0 + (i as f64) * 80.0;
            if elapsed < delay {
                continue;
            }
            let p = ease_out_cubic((elapsed - delay) / 540.0);
            let _ = item.el.set_attribute(
                "style",
                &format!("opacity:{:.3};stroke-dashoffset:{:.4}", p, 1.0 - p),
            );
            if p >= 0.999 {
                item.settled = true;
            }
        }

        // Kinetic background typography sweep (GPU transform + opacity only)
        let word_u = clamp01(elapsed / 1760.0);
        let curtain_fade = 1.0 - ease_in_out((elapsed - 1360.0) / 640.0);
        for (word, dir) in &self.words {
            let shift = (0.42 - word_u * 0.84) * 64.0 * (*dir);
            css(word, "transform", &format!("translate3d({shift:.2}vw,0,0)"));
            css(word, "opacity", &format!("{curtain_fade:.3}"));
        }

        // Phase 2: Isometric 3D wireframe extrusion (580..1280ms) — 2 group writes instead of 42 line writes
        let ext = ease_out_cubic((elapsed - 580.0) / 680.0);
        let dx = EXTRUDE_DX * ext;
        let dy = EXTRUDE_DY * ext;
        let _ = self.base_group.set_attribute(
            "style",
            &format!(
                "opacity:{:.3};transform:translate3d({dx:.2}px,{dy:.2}px,0)",
                ext * 0.9
            ),
        );
        let _ = self.edges_group.set_attribute(
            "style",
            &format!(
                "opacity:{:.3};stroke-dashoffset:{:.4}",
                ext * 0.78,
                1.0 - ext
            ),
        );

        // Phase 3: Zero-reflow coordinate handoff & curtain dissolve (1280..2080ms)
        let title_appear = ease_out_cubic((elapsed - 540.0) / 420.0);
        let sub_appear = ease_out_cubic((elapsed - 650.0) / 420.0);
        let handoff = clamp01((elapsed - 1280.0) / 700.0);

        Self::apply_floater(&self.float_title, self.title_track, handoff, title_appear);
        Self::apply_floater(&self.float_sub, self.sub_track, handoff, sub_appear);

        let stage_scale = 1.0 + ext * 0.035 - ease_in_out(handoff) * 0.12;
        let stage_y = -50.0 - ease_in_out(handoff) * 14.0;
        css(
            &self.stage,
            "transform",
            &format!("translate3d(-50%, {stage_y:.2}%, 0) scale({stage_scale:.4})"),
        );
        css(&self.stage, "opacity", &format!("{curtain_fade:.3}"));
        css(&self.backdrop, "opacity", &format!("{curtain_fade:.3}"));

        true
    }
}

pub(super) fn init(
    window: &Window,
    document: &Document,
    root: &Element,
    reduced: &MediaQueryList,
) -> Result<bool> {
    if reduced.matches() {
        return Ok(false);
    }
    if window.scroll_y().unwrap_or_default() > 8.0
        || !window.location().hash().unwrap_or_default().is_empty()
    {
        return Ok(false);
    }
    let Some(hero) = root.query_selector("[data-kense-hero]")? else {
        return Ok(false);
    };
    let Ok(curtain) = element::<HtmlElement>(&hero, "[data-kense-intro]") else {
        return Ok(false);
    };
    let key = curtain
        .get_attribute("data-intro-key")
        .unwrap_or_else(|| "kense:home-intro:v1".into());
    if let Ok(Some(storage)) = window.session_storage() {
        if storage.get_item(&key).ok().flatten().as_deref() == Some("1") {
            return Ok(false);
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
    edges_group.set_attribute("stroke-dasharray", "1")?;
    edges_group.set_attribute("stroke-dashoffset", "1")?;

    for &(vx, vy) in &EXTRUDE_VERTICES {
        let line = document.create_element_ns(Some(SVG_NS), "line")?;
        line.set_attribute("x1", &format!("{vx:.2}"))?;
        line.set_attribute("y1", &format!("{vy:.2}"))?;
        line.set_attribute("x2", &format!("{:.2}", vx + EXTRUDE_DX))?;
        line.set_attribute("y2", &format!("{:.2}", vy + EXTRUDE_DY))?;
        line.set_attribute("pathLength", "1")?;
        edges_group.append_child(&line)?;
    }

    let collect = |sel: &str| -> Result<Vec<TracePath>> {
        let list = curtain.query_selector_all(sel)?;
        Ok((0..list.length())
            .filter_map(|i| {
                Some(TracePath {
                    el: list.item(i)?.dyn_into().ok()?,
                    settled: false,
                })
            })
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
    css(&float_title, "left", "0px");
    css(&float_title, "top", "0px");
    curtain.append_child(&float_title)?;

    let float_sub: HtmlElement = document.create_element("div")?.dyn_into()?;
    float_sub.set_class_name("kense-intro-floating-title kense-intro-floating-sub");
    float_sub.set_text_content(target_sub.text_content().as_deref());
    css(&float_sub, "left", "0px");
    css(&float_sub, "top", "0px");
    curtain.append_child(&float_sub)?;

    let body = document.body();
    flag(&hero, "data-kense-intro-playing", true);
    if let Some(b) = &body {
        flag(b, "data-kense-intro-active", true);
    }
    curtain.set_hidden(false);

    // Measure once before the animation loop starts so `tick` performs 0 layout reads.
    let (vw, vh) = viewport(window);
    let stage_h = f64::from(stage.offset_height());
    let stage_bottom = vh * 0.44 + stage_h * 0.46;

    let measure_track =
        |floater: &HtmlElement, target: &HtmlElement, start_y: f64| -> FloaterTrack {
            let rect = target.get_bounding_client_rect();
            let fw = f64::from(floater.offset_width()).max(rect.width());
            let fh = f64::from(floater.offset_height()).max(rect.height());
            css(floater, "width", &format!("{fw:.1}px"));
            FloaterTrack {
                start_x: (vw - fw) * 0.5,
                start_y,
                end_x: rect.left() + (rect.width() - fw) * 0.5,
                end_y: rect.top() + (rect.height() - fh) * 0.5,
            }
        };
    let title_track = measure_track(&float_title, &target_title, stage_bottom + 8.0);
    let sub_track = measure_track(&float_sub, &target_sub, stage_bottom + 66.0);

    let state = Rc::new(RefCell::new(Intro {
        document: document.clone(),
        hero,
        curtain,
        backdrop,
        stage,
        base_group,
        edges_group,
        guide_paths,
        outline_paths,
        words,
        float_title,
        float_sub,
        title_track,
        sub_track,
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

    {
        let state = state.clone();
        let safety = wasm_bindgen::closure::Closure::once(move || {
            state.borrow_mut().finish();
        });
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            safety.as_ref().unchecked_ref(),
            (DURATION_MS as i32) + 320,
        );
        safety.forget();
    }

    frame.request();
    std::mem::forget(frame);
    Ok(true)
}
