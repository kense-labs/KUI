//! Unified in-place morphing `#kense-guide` companion for the Kense homepage.
//!
//! Combines KUI's signature in-place shell morphing (idle 56x38 slanted bot -> rich speech bubble
//! or 5-row command card with live top-left eyes) with silky comet-ribbon flight trails, curved hop
//! trajectories, velocity-coupled stretch/shear, staggered character speech, and strict topbar/mega
//! non-collision guarantees.

use crate::cover::{
    Frame, Result, css, element, flag,
    flight::{Flight, Mode, Point},
    listen, viewport,
};
use js_sys::Reflect;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{
    CanvasRenderingContext2d, Document, Element, HtmlCanvasElement, HtmlElement, KeyboardEvent,
    MediaQueryList, PointerEvent, ResizeObserver, Window,
};

const REST_W: f64 = 56.0;
const REST_H: f64 = 36.0;
const MENU_W: f64 = 296.0;
const MENU_H: f64 = 274.0;
const TRAIL_WINDOW_MS: f64 = 220.0;

#[derive(Clone, Copy)]
struct TrailNode {
    x: f64,
    y: f64,
    at_ms: f64,
}

struct KenseGuide {
    window: Window,
    page: Document,
    media: MediaQueryList,
    guide: HtmlElement,
    shell: HtmlElement,
    trigger: HtmlElement,
    choices: HtmlElement,
    speech: HtmlElement,
    title: HtmlElement,
    body: HtmlElement,
    copy: HtmlElement,
    trail_canvas: Option<HtmlCanvasElement>,
    trail_ctx: Option<CanvasRenderingContext2d>,
    trail_nodes: VecDeque<TrailNode>,
    trail_dirty: bool,
    home_dock: HtmlElement,
    family_berths: Vec<Element>,
    gpui_slot: Option<Element>,
    anchors: Vec<HtmlElement>,
    header: Option<Element>,
    target: Option<Element>,
    pointer: Option<Point>,
    press: Option<Point>,
    dragged: bool,
    leave_at: Option<f64>,
    feedback_until: Option<f64>,
    hop_start: f64,
    hop_dur: f64,
    hop_amp: f64,
    cushion: f64,
    cushion_vel: f64,
    was_flying: bool,
    fused_scene: Option<usize>,
    dive_from: Point,
    dive_start: f64,
    dive_attached: bool,
    mode: Mode,
    flight: Flight,
    last_ms: f64,
    dirty: bool,
}

impl KenseGuide {
    fn now(&self) -> f64 {
        self.window
            .performance()
            .map(|p| p.now())
            .unwrap_or_default()
    }

    fn reset_shell_size(&self) {
        if self.mode != Mode::Menu {
            css(&self.shell, "--shell-w", &format!("{REST_W}px"));
            css(&self.shell, "--shell-h", &format!("{REST_H}px"));
        } else {
            let _ = self.shell.style().remove_property("--shell-w");
            let _ = self.shell.style().remove_property("--shell-h");
        }
    }

    fn clear_speech(&mut self) {
        self.feedback_until = None;
        self.speech.set_hidden(true);
        self.body.set_hidden(true);
        self.copy.set_hidden(true);
        let _ = self.copy.remove_attribute("data-copy");
        let _ = self.copy.remove_attribute("data-copied");
        flag(&self.guide, "data-hint", false);
        flag(&self.guide, "data-below", false);
        self.reset_shell_size();
    }

    fn sync_speech_box(&self) {
        if !self.speech.hidden() {
            let w = f64::from(self.speech.offset_width()).max(148.0);
            let h = f64::from(self.speech.offset_height()).max(40.0);
            css(&self.shell, "--shell-w", &format!("{w:.1}px"));
            css(&self.shell, "--shell-h", &format!("{h:.1}px"));
        }
    }

    fn set_title_animated(&self, text: &str) {
        if self.media.matches() {
            self.title.set_text_content(Some(text));
            return;
        }
        let mut html = String::with_capacity(text.len() * 48);
        for (idx, ch) in text.chars().enumerate() {
            let delay = (idx * 10).min(240);
            html.push_str(&format!(
                r#"<span class="kense-speech-char" style="--char-delay:{delay}ms">"#
            ));
            match ch {
                '&' => html.push_str("&amp;"),
                '<' => html.push_str("&lt;"),
                '>' => html.push_str("&gt;"),
                '"' => html.push_str("&quot;"),
                _ => html.push(ch),
            }
            html.push_str("</span>");
        }
        self.title.set_inner_html(&html);
    }

    fn speak(&mut self) {
        let Some(target) = &self.target else { return };
        // Product badges & Hero belt pill already open their own dedicated floating cards!
        if target.has_attribute("data-product-badge")
            || target.has_attribute("data-hero-solution-word")
            || target.has_attribute("data-kense-solid")
        {
            return;
        }
        let Some(hint) = target.get_attribute("data-kense-hint") else {
            return;
        };
        if hint.is_empty() {
            return;
        }
        self.set_title_animated(&hint);
        let body = target
            .get_attribute("data-kense-body")
            .filter(|s| !s.is_empty());
        self.body.set_hidden(body.is_none());
        self.body.set_text_content(body.as_deref());

        if let Some(payload) = target.get_attribute("data-copy") {
            let _ = self.copy.set_attribute("data-copy", &payload);
            if let Some(kind) = target.get_attribute("data-kind") {
                let _ = self.copy.set_attribute("data-kind", &kind);
            }
            if let Some(label) = target.get_attribute("aria-label") {
                let _ = self.copy.set_attribute("aria-label", &label);
            }
            self.copy.set_hidden(false);
        } else {
            self.copy.set_hidden(true);
        }
        self.speech.set_hidden(false);
        flag(&self.guide, "data-hint", true);
        self.sync_speech_box();
        self.dirty = true;
    }

    fn speak_feedback(&mut self, attr: &str) {
        if self.mode == Mode::Menu {
            return;
        }
        self.exit_fusion();
        let Some(msg) = self.guide.get_attribute(attr) else {
            return;
        };
        self.set_title_animated(&msg);
        self.body.set_hidden(true);
        self.copy.set_hidden(true);
        self.speech.set_hidden(false);
        flag(&self.guide, "data-hint", true);
        self.sync_speech_box();
        self.feedback_until = Some(self.now() + 2100.0);
        self.cushion_vel = 2.6;
        self.dirty = true;
    }

    fn scene_anchor(&self, scene_idx: usize) -> Option<(f64, f64, f64)> {
        if let Some(proj) = super::fusion::projection(scene_idx) {
            return Some((proj.x, proj.y, proj.width));
        }
        let kind = match scene_idx {
            0 => "runtime",
            1 => "ui",
            2 => "agent",
            3 => "clay-vault",
            4 => "clay-shield",
            5 => "clay-release",
            6 => "clay-rust",
            7 => "studio",
            _ => return None,
        };
        let el = self
            .page
            .query_selector(&format!(r#"[data-kense-solid="{kind}"]"#))
            .ok()
            .flatten()?;
        let r = el.get_bounding_client_rect();
        (r.width() > 20.0 && r.height() > 20.0).then_some((
            r.x() + r.width() * 0.5,
            r.y() + r.height() * 0.5,
            52.0,
        ))
    }

    fn enter_fusion(&mut self, scene_idx: usize) {
        if self.fused_scene == Some(scene_idx) {
            self.leave_at = None;
            return;
        }
        let now = self.now();
        if let Some(prev) = self.fused_scene
            && self.dive_attached
            && let Some((px, py, _)) = self.scene_anchor(prev)
        {
            self.flight.place(Point {
                x: px,
                y: py + REST_H * 0.5,
            });
        }
        self.clear_speech();
        self.target = None;
        self.leave_at = None;
        self.mode = Mode::Pointer;
        self.fused_scene = Some(scene_idx);
        self.dive_from = self.flight.position;
        self.dive_start = now;
        if self.media.matches() {
            self.dive_attached = true;
            flag(&self.guide, "data-flying", false);
            flag(&self.guide, "data-fused", true);
            super::fusion::set_attached(scene_idx);
        } else {
            self.dive_attached = false;
            flag(&self.guide, "data-fused", false);
            super::fusion::set_diving(scene_idx, 0.0);
        }
        self.dirty = false;
    }

    fn exit_fusion(&mut self) {
        let Some(prev) = self.fused_scene.take() else {
            return;
        };
        if let Some((px, py, _)) = self.scene_anchor(prev) {
            self.flight.place(Point {
                x: px,
                y: py + REST_H * 0.5,
            });
        }
        self.dive_attached = false;
        flag(&self.guide, "data-fused", false);
        self.cushion_vel = 2.4;
        super::fusion::clear();
        self.dirty = true;
    }

    fn act(&mut self, target: Option<Element>) {
        self.exit_fusion();
        self.leave_at = None;
        self.mode = Mode::Pointer;
        if self.target != target {
            if self.feedback_until.is_none() {
                self.clear_speech();
            }
            self.target = target;
        }
        self.dirty = true;
    }

    fn menu(&mut self, open: bool) {
        self.exit_fusion();
        self.leave_at = None;
        self.target = None;
        self.feedback_until = None;
        self.speech.set_hidden(true);
        self.copy.set_hidden(true);
        flag(&self.guide, "data-hint", false);
        self.choices.set_hidden(!open);
        self.mode = if open { Mode::Menu } else { Mode::Held };
        flag(&self.guide, "data-open", open);
        self.reset_shell_size();
        let _ = self
            .trigger
            .set_attribute("aria-expanded", if open { "true" } else { "false" });
        let key = if open {
            "data-close-label"
        } else {
            "data-open-label"
        };
        if let Some(label) = self.trigger.get_attribute(key) {
            let _ = self.trigger.set_attribute("aria-label", &label);
        }
        if !open {
            self.cushion_vel = -1.8;
        }
        self.dirty = true;
    }

    fn header_bottom(&self) -> f64 {
        self.header
            .as_ref()
            .map_or(56.0, |el| el.get_bounding_client_rect().bottom().max(0.0))
    }

    fn perch(&self, el: &Element) -> Point {
        let r = el.get_bounding_client_rect();
        if el == self.home_dock.as_ref() {
            return Point {
                x: r.x() + r.width() * 0.5,
                y: r.bottom() - 6.0,
            };
        }
        // When `#kense-hero-hint` 3D flip-card is open above the Hero belt pill, perch cleanly above its top border!
        if el.has_attribute("data-hero-solution-word")
            && let Ok(Some(hint_card)) = self.page.query_selector("[data-hero-hint][data-open]")
        {
            let hr = hint_card.get_bounding_client_rect();
            return Point {
                x: hr.right() - 28.0,
                y: hr.top() - 15.0,
            };
        }
        // If `#kense-product-popover` opened ABOVE this badge near bottom of viewport, perch cleanly above the popover!
        if el.has_attribute("data-product-badge")
            && let Ok(Some(pop)) = self
                .page
                .query_selector("[data-kense-popover][data-open][data-above]")
        {
            let pr = pop.get_bounding_client_rect();
            return Point {
                x: r.x() + r.width() * 0.5,
                y: pr.top() - 16.0,
            };
        }
        if el.has_attribute("data-kense-solid") || r.height() > 160.0 {
            return Point {
                x: r.left() + r.width() * 0.78,
                y: r.top() + 34.0,
            };
        }
        Point {
            x: r.x() + r.width() * 0.5,
            y: r.top() - 15.0,
        }
    }

    fn scroll_perch(&self, vw: f64, vh: f64) -> Point {
        let home = self.perch(&self.home_dock);
        let scroll = self.window.scroll_y().unwrap_or_default();
        if scroll <= 96.0 {
            return home;
        }

        // 1. Live GPUI Workbench corner berth
        if let Some(gpui) = &self.gpui_slot {
            let gr = gpui.get_bounding_client_rect();
            if gr.top() < vh * 0.62 && gr.bottom() > vh * 0.36 {
                return Point {
                    x: (gr.right() - 44.0).clamp(48.0, vw - 48.0),
                    y: gr.top() - 15.0,
                };
            }
        }

        // 2. Active `#products` 3D solid corner berth
        for item in &self.family_berths {
            let ir = item.get_bounding_client_rect();
            if ir.top() < vh * 0.58
                && ir.bottom() > vh * 0.32
                && let Ok(Some(solid)) = item.query_selector("[data-kense-solid]")
            {
                let sr = solid.get_bounding_client_rect();
                return Point {
                    x: (sr.left() + sr.width() * 0.78).clamp(48.0, vw - 48.0),
                    y: sr.top() + 32.0,
                };
            }
        }

        if self.anchors.is_empty() {
            return home;
        }
        let band = vh * 0.32;
        let mut chosen = &self.anchors[0];
        let mut best = f64::INFINITY;
        for anchor in &self.anchors {
            let r = anchor.get_bounding_client_rect();
            let dist = (r.top() - band).abs();
            if dist < best {
                best = dist;
                chosen = anchor;
            }
        }
        let r = chosen.get_bounding_client_rect();
        // Always dock in the outer studio margin so `#kense-guide` never covers centered headings!
        let margin_x = (vw * 0.5 + 436.0).clamp(48.0, vw - 56.0);
        Point {
            x: margin_x,
            y: r.top() + r.height() * 0.5 + 14.0,
        }
    }

    fn destination(&mut self) -> Point {
        let (vw, vh) = viewport(&self.window);
        // Strict ceiling below sticky topbar so `#kense-guide` NEVER overlaps header buttons!
        let top = self.header_bottom() + 14.0;
        let home = self.perch(&self.home_dock);
        let reduced = self.media.matches();

        let p = if matches!(self.mode, Mode::Menu | Mode::Held) {
            self.flight.position
        } else if let Some(target) = &self.target {
            self.perch(target)
        } else if reduced || vw < 860.0 {
            home
        } else {
            self.scroll_perch(vw, vh)
        };

        let menu = self.mode == Mode::Menu;
        let speech = !self.speech.hidden();
        let (width, height) = if menu {
            ((vw - 32.0).min(MENU_W), MENU_H)
        } else if speech {
            (
                f64::from(self.speech.offset_width()).max(148.0),
                f64::from(self.speech.offset_height()).max(40.0),
            )
        } else {
            (REST_W, REST_H)
        };

        let below = (menu || speech) && p.y - height < top;
        flag(&self.guide, "data-below", below);
        let y = if below && speech {
            self.target
                .as_ref()
                .map(|t| t.get_bounding_client_rect().bottom() + 10.0)
                .unwrap_or(p.y)
                .min(vh - height - 10.0)
                .max(top)
        } else if below {
            p.y.min(vh - height + REST_H - 10.0).max(top + REST_H)
        } else {
            p.y.min(vh - 18.0).max(top + height)
        };
        Point {
            x: p.x.min(vw - width * 0.5 - 12.0).max(width * 0.5 + 12.0),
            y,
        }
    }

    fn step_trail(&mut self, now: f64, x: f64, y: f64, flying: bool, reduced: bool) -> bool {
        let (Some(canvas), Some(ctx)) = (&self.trail_canvas, &self.trail_ctx) else {
            return false;
        };
        if reduced || self.guide.has_attribute("data-concealed") {
            self.trail_nodes.clear();
            if self.trail_dirty {
                let _ = ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
                ctx.clear_rect(
                    0.0,
                    0.0,
                    f64::from(canvas.width()),
                    f64::from(canvas.height()),
                );
                self.trail_dirty = false;
            }
            return false;
        }

        let cy = y - REST_H * 0.5;
        if flying {
            let push = self
                .trail_nodes
                .back()
                .is_none_or(|last| (last.x - x).hypot(last.y - cy) > 2.2);
            if push {
                if self.trail_nodes.len() >= 28 {
                    self.trail_nodes.pop_front();
                }
                self.trail_nodes.push_back(TrailNode {
                    x,
                    y: cy,
                    at_ms: now,
                });
            }
        }

        while self
            .trail_nodes
            .front()
            .is_some_and(|n| now - n.at_ms > TRAIL_WINDOW_MS)
        {
            self.trail_nodes.pop_front();
        }

        if self.trail_nodes.len() < 2 {
            if self.trail_dirty {
                let _ = ctx.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
                ctx.clear_rect(
                    0.0,
                    0.0,
                    f64::from(canvas.width()),
                    f64::from(canvas.height()),
                );
                self.trail_dirty = false;
            }
            return !self.trail_nodes.is_empty();
        }

        let (vw, vh) = viewport(&self.window);
        let dpr = self.window.device_pixel_ratio().clamp(1.0, 2.0);
        let pw = (vw * dpr).round().max(1.0) as u32;
        let ph = (vh * dpr).round().max(1.0) as u32;
        if canvas.width() != pw || canvas.height() != ph {
            canvas.set_width(pw);
            canvas.set_height(ph);
        }
        let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
        ctx.clear_rect(0.0, 0.0, vw, vh);

        let dark = self
            .page
            .document_element()
            .and_then(|e| e.get_attribute("data-theme"))
            .as_deref()
            == Some("dark");
        let n = self.trail_nodes.len();
        let tail = self.trail_nodes[0];
        let head = self.trail_nodes[n - 1];
        if (head.x - tail.x).hypot(head.y - tail.y) < 3.0 {
            return !self.trail_nodes.is_empty();
        }

        // Build one continuous tapered ribbon polygon (zero overlapping stroke caps / dots!)
        let mut left = Vec::with_capacity(n);
        let mut right = Vec::with_capacity(n);
        for i in 0..n {
            let cur = self.trail_nodes[i];
            let prev = self.trail_nodes[i.saturating_sub(1)];
            let next = self.trail_nodes[(i + 1).min(n - 1)];
            let tx = next.x - prev.x;
            let ty = next.y - prev.y;
            let len = tx.hypot(ty).max(0.001);
            let nx = -ty / len;
            let ny = tx / len;

            let age01 = ((now - cur.at_ms) / TRAIL_WINDOW_MS).clamp(0.0, 1.0);
            let prog = (i as f64) / ((n - 1) as f64);
            let half_w = 10.5 * prog.powf(1.35) * (1.0 - age01);
            left.push((cur.x + nx * half_w, cur.y + ny * half_w));
            right.push((cur.x - nx * half_w, cur.y - ny * half_w));
        }

        let head_age = ((now - head.at_ms) / TRAIL_WINDOW_MS).clamp(0.0, 1.0);
        let max_alpha = (1.0 - head_age).powi(2) * if dark { 0.34 } else { 0.18 };
        if max_alpha > 0.005 {
            let grad = ctx.create_linear_gradient(tail.x, tail.y, head.x, head.y);
            let rgb = if dark { "255, 255, 255" } else { "23, 23, 23" };
            let _ = grad.add_color_stop(0.0, &format!("rgba({rgb}, 0)"));
            let _ = grad.add_color_stop(0.45, &format!("rgba({rgb}, {:.3})", max_alpha * 0.35));
            let _ = grad.add_color_stop(1.0, &format!("rgba({rgb}, {max_alpha:.3})"));
            let _ = Reflect::set(ctx.as_ref(), &JsValue::from_str("fillStyle"), grad.as_ref());

            ctx.begin_path();
            ctx.move_to(tail.x, tail.y);
            for &(lx, ly) in &left[1..] {
                ctx.line_to(lx, ly);
            }
            for &(rx, ry) in right[1..].iter().rev() {
                ctx.line_to(rx, ry);
            }
            ctx.close_path();
            ctx.fill();
            self.trail_dirty = true;
        }
        true
    }

    fn tick(&mut self, now: f64) -> bool {
        if self.page.hidden() || !self.guide.is_connected() {
            self.last_ms = 0.0;
            return false;
        }
        let dt = if self.last_ms == 0.0 {
            0.016
        } else {
            ((now - self.last_ms) / 1000.0).clamp(0.001, 0.05)
        };
        self.last_ms = now;

        if self.leave_at.is_some_and(|at| now >= at) {
            self.act(None);
        }
        if self.feedback_until.is_some_and(|at| now >= at) {
            self.clear_speech();
            if self.target.is_some() {
                self.speak();
            }
        }

        let reduced = self.media.matches();

        // Active 3D Fusion: dive into the hovered 3D model, vanish 2D shell on impact, and drive 3D KUI eyes!
        if let Some(scene_idx) = self.fused_scene
            && let Some((ax, ay, aw)) = self.scene_anchor(scene_idx)
        {
            let (dx, dy) = self.pointer.map_or((0.0, 0.0), |p| (p.x - ax, p.y - ay));
            let dist = dx.hypot(dy).max(70.0);
            let blink = if reduced {
                1.0
            } else {
                let phase = (now % 5400.0) / 5400.0;
                if (0.44..=0.48).contains(&phase) {
                    let t = ((phase - 0.46).abs() / 0.02).clamp(0.0, 1.0);
                    0.14 + 0.86 * (t * t * (3.0 - 2.0 * t)) as f32
                } else {
                    1.0
                }
            };
            super::fusion::set_gaze((dx / dist) as f32, (dy / dist) as f32, blink);

            if !self.dive_attached {
                let u = ((now - self.dive_start) / 215.0).clamp(0.0, 1.0);
                let ease = u * u * (3.0 - 2.0 * u);
                let end_scale = (aw / REST_W).clamp(0.48, 0.86);
                let scale = 1.0 + (end_scale - 1.0) * ease;
                let start_cx = self.dive_from.x;
                let start_cy = self.dive_from.y - REST_H * 0.5;
                let cx = start_cx + (ax - start_cx) * ease;
                let arc = 4.0 * u * (1.0 - u) * 20.0;
                let cy = start_cy + (ay - start_cy) * ease - arc;
                let gy = cy + REST_H * 0.5 * scale;

                let vx = ax - start_cx;
                let vy = ay - start_cy;
                let vlen = vx.hypot(vy).max(1.0);
                let env = 4.0 * u * (1.0 - u);
                let stretch = 0.16 * env * ((vx.abs() - vy.abs()) / vlen);
                let shear = 0.12 * env * (vx * vy) / (vlen * vlen);
                let sx = scale * (1.0 + stretch).max(0.76);
                let sy = scale * (1.0 - stretch * 0.85).max(0.76);
                let b = scale * shear;
                css(
                    &self.guide,
                    "--flight-pose",
                    &format!("matrix({sx:.5},{b:.5},{b:.5},{sy:.5},0,0)"),
                );
                css(&self.guide, "--guide-x", &format!("{cx:.2}px"));
                css(&self.guide, "--guide-y", &format!("{gy:.2}px"));
                flag(&self.guide, "data-flying", true);
                flag(&self.guide, "data-awake", !reduced);
                flag(&self.guide, "data-placed", true);
                let _ = self.step_trail(now, cx, cy + REST_H * 0.5, true, reduced);

                if u >= 1.0 {
                    self.dive_attached = true;
                    self.flight.place(Point {
                        x: ax,
                        y: ay + REST_H * 0.5,
                    });
                    flag(&self.guide, "data-flying", false);
                    flag(&self.guide, "data-fused", true);
                    super::fusion::set_attached(scene_idx);
                } else {
                    super::fusion::set_diving(scene_idx, u);
                }
                return true;
            }

            let rest_pt = Point {
                x: ax,
                y: ay + REST_H * 0.5,
            };
            self.flight.place(rest_pt);
            css(&self.guide, "--guide-x", &format!("{:.2}px", rest_pt.x));
            css(&self.guide, "--guide-y", &format!("{:.2}px", rest_pt.y));
            let trail_active = self.step_trail(now, rest_pt.x, rest_pt.y, false, reduced);
            let keep = !reduced || trail_active;
            if !keep {
                self.last_ms = 0.0;
            }
            return keep;
        }

        if self.dirty {
            let goal = self.destination();
            let jump = self.flight.position.distance(goal);
            if self.mode == Mode::Pointer && !reduced && jump > 32.0 {
                self.hop_start = now;
                self.hop_dur = (0.18 + jump.sqrt() * 0.014).clamp(0.20, 0.46) * 1000.0;
                self.hop_amp = (jump * 0.14).clamp(8.0, 34.0);
            }
            self.flight.aim(goal, self.mode, now);
            self.dirty = false;
        }

        let rest = self.flight.step(now, self.mode, reduced);
        let mut x = self.flight.position.x;
        let mut y = self.flight.position.y;

        // Add parabolic hop lift during discrete pointer jumps so movement feels buoyant & alive!
        let mut hop_active = false;
        if !reduced && self.hop_dur > 0.0 && now < self.hop_start + self.hop_dur {
            let u = ((now - self.hop_start) / self.hop_dur).clamp(0.0, 1.0);
            let arc = 4.0 * u * (1.0 - u);
            y = (y - self.hop_amp * arc).max(self.header_bottom() + REST_H + 10.0);
            hop_active = true;
        }

        let flying = !reduced
            && self.mode != Mode::Menu
            && (hop_active
                || (!rest && self.flight.position.distance(Point { x, y }) > 2.0)
                || self.flight.stretch.abs() > 0.028);
        flag(&self.guide, "data-flying", flying);
        if self.was_flying && !flying && self.mode != Mode::Menu {
            self.cushion_vel = -1.9;
        }
        self.was_flying = flying;

        // Cushion landing spring
        if reduced {
            self.cushion = 0.0;
            self.cushion_vel = 0.0;
        } else {
            let acc = -440.0 * self.cushion - 26.0 * self.cushion_vel;
            self.cushion_vel += acc * dt;
            self.cushion = (self.cushion + self.cushion_vel * dt).clamp(-0.14, 0.14);
            if self.cushion.abs() < 0.002 && self.cushion_vel.abs() < 0.02 {
                self.cushion = 0.0;
                self.cushion_vel = 0.0;
            }
        }

        let a = (1.0 + self.flight.stretch - self.cushion * 0.7).max(0.75);
        let b = self.flight.shear;
        let d = (1.0 + b * b) / a;
        css(
            &self.guide,
            "--flight-pose",
            &format!("matrix({a:.5},{b:.5},{b:.5},{d:.5},0,0)"),
        );
        x = x.round() + (x - x.round());
        css(&self.guide, "--guide-x", &format!("{x:.2}px"));
        css(&self.guide, "--guide-y", &format!("{y:.2}px"));

        let (dx, dy) = self
            .pointer
            .map_or((0.0, 0.0), |p| (p.x - x, p.y - y + 19.0));
        let gaze = dx.hypot(dy).max(55.0);
        css(
            &self.guide,
            "--look-x",
            &format!("{:.2}px", dx * 4.0 / gaze),
        );
        css(
            &self.guide,
            "--look-y",
            &format!("{:.2}px", dy * 3.0 / gaze),
        );

        if rest && self.mode == Mode::Pointer && self.target.is_some() && self.speech.hidden() {
            self.speak();
        }
        self.sync_speech_box();

        flag(&self.guide, "data-awake", !reduced);
        flag(&self.guide, "data-placed", true);

        let trail_active = self.step_trail(now, x, y, flying, reduced);

        let keep = self.dirty
            || !rest
            || hop_active
            || trail_active
            || self.flight.stretch.abs() + self.flight.shear.abs() > 0.0006
            || self.cushion.abs() > 0.002
            || self.cushion_vel.abs() > 0.02
            || self.leave_at.is_some()
            || self.feedback_until.is_some();
        if !keep {
            self.last_ms = 0.0;
        }
        keep
    }
}

fn ev_el(event: &web_sys::Event) -> Option<Element> {
    event.target()?.dyn_into().ok()
}

/// Detect 3D fusion zones (`.kense-bento-card` sculptures and `#products` `[data-kense-solid]` stages).
fn fusion_zone(el: &Element) -> Option<usize> {
    if el
        .closest("header.topbar,.kense-mega-panel")
        .ok()
        .flatten()
        .is_some()
    {
        return None;
    }
    if let Some(card) = el.closest(".kense-bento-card").ok().flatten()
        && let Ok(Some(solid)) = card.query_selector("[data-kense-solid]")
        && let Some(kind) = solid.get_attribute("data-kense-solid")
    {
        return super::fusion::parse_scene_idx(&kind);
    }
    if let Some(solid) = el.closest("[data-kense-solid]").ok().flatten()
        && let Some(kind) = solid.get_attribute("data-kense-solid")
    {
        return super::fusion::parse_scene_idx(&kind);
    }
    None
}

/// Only match interactive targets inside `<main class="kense-home">` — NEVER inside `header.topbar`!
fn hint_el(el: &Element) -> Option<Element> {
    if el
        .closest("header.topbar,.kense-mega-panel")
        .ok()
        .flatten()
        .is_some()
    {
        return None;
    }
    el.closest(".kense-home [data-kense-hint],.kense-home [data-product-badge]")
        .ok()
        .flatten()
}

fn ptr_point(e: &PointerEvent) -> Point {
    Point {
        x: f64::from(e.client_x()),
        y: f64::from(e.client_y()),
    }
}

pub(super) fn init(
    window: &Window,
    page: &Document,
    root: &Element,
    media: &MediaQueryList,
) -> Result {
    let Ok(guide) = element::<HtmlElement>(root, "#kense-guide") else {
        return Ok(());
    };
    let shell: HtmlElement = element(&guide, ".kense-guide-shell")?;
    let trigger: HtmlElement = element(&guide, ".kense-guide-trigger")?;
    let choices: HtmlElement = element(&guide, ".kense-guide-choices")?;
    let home_dock: HtmlElement = element(root, "[data-kense-guide-home]")?;
    let trail_canvas = root
        .query_selector("[data-guide-trail]")?
        .and_then(|n| n.dyn_into::<HtmlCanvasElement>().ok());
    let trail_ctx = trail_canvas.as_ref().and_then(|c| {
        c.get_context("2d")
            .ok()
            .flatten()
            .and_then(|ctx| ctx.dyn_into::<CanvasRenderingContext2d>().ok())
    });

    let family_list = root.query_selector_all("[data-family-berth]")?;
    let family_berths = (0..family_list.length())
        .filter_map(|i| family_list.item(i)?.dyn_into::<Element>().ok())
        .collect();

    let mut anchors = Vec::new();
    for sel in [
        "#kense-family-title",
        "#kense-exp-title",
        "#kense-cap-title",
        "#kense-arch-title",
        "#kense-future-title",
        "#kense-start-title",
    ] {
        if let Ok(el) = element::<HtmlElement>(root, sel) {
            anchors.push(el);
        }
    }

    let mut state = KenseGuide {
        window: window.clone(),
        page: page.clone(),
        media: media.clone(),
        guide: guide.clone(),
        shell,
        trigger: trigger.clone(),
        choices: choices.clone(),
        speech: element(&guide, ".kense-guide-speech")?,
        title: element(&guide, ".kense-speech-title")?,
        body: element(&guide, ".kense-speech-body")?,
        copy: element(&guide, ".kense-guide-copy")?,
        trail_canvas,
        trail_ctx,
        trail_nodes: VecDeque::with_capacity(32),
        trail_dirty: false,
        home_dock,
        family_berths,
        gpui_slot: root.query_selector("[data-kense-gpui-slot]")?,
        anchors,
        header: page.query_selector("header.topbar")?,
        target: None,
        pointer: None,
        press: None,
        dragged: false,
        leave_at: None,
        feedback_until: None,
        hop_start: 0.0,
        hop_dur: 0.0,
        hop_amp: 0.0,
        cushion: 0.0,
        cushion_vel: 0.0,
        was_flying: false,
        fused_scene: None,
        dive_from: Point::default(),
        dive_start: 0.0,
        dive_attached: false,
        mode: Mode::Pointer,
        flight: Flight::default(),
        last_ms: 0.0,
        dirty: true,
    };
    let initial = state.destination();
    state.flight.place(initial);
    let state = Rc::new(RefCell::new(state));
    let frame = Frame::new(window, {
        let state = state.clone();
        move |now| state.borrow_mut().tick(now)
    });

    for event in ["pointerover", "pointermove"] {
        let (state, frame) = (state.clone(), frame.clone());
        listen(page, event, move |ev| {
            let Some(e) = ev.dyn_ref::<PointerEvent>() else {
                return;
            };
            if e.pointer_type() == "touch" {
                return;
            }
            let Some(el) = ev_el(&ev) else { return };
            let mut g = state.borrow_mut();
            if g.mode == Mode::Menu {
                return;
            }
            let p = ptr_point(e);
            g.pointer = Some(p);
            if let Some(start) = g.press {
                g.dragged |= p.distance(start) > 8.0;
            } else if !g.guide.has_attribute("data-fused") && g.guide.contains(Some(&el)) {
                g.leave_at = None;
                if g.target.is_none() {
                    g.mode = Mode::Held;
                }
            } else if let Some(scene_idx) = fusion_zone(&el) {
                g.enter_fusion(scene_idx);
            } else {
                if g.fused_scene.is_some() {
                    g.exit_fusion();
                }
                if let Some(target) = hint_el(&el) {
                    g.act(Some(target));
                } else if g.target.is_some() {
                    if g.leave_at.is_none() {
                        g.leave_at = Some(g.now() + 620.0);
                    }
                } else {
                    g.act(None);
                }
            }
            frame.request();
        })?;
    }

    if let Some(doc_el) = page.document_element() {
        let (s, f) = (state.clone(), frame.clone());
        listen(&doc_el, "mouseleave", move |_| {
            let mut g = s.borrow_mut();
            if g.fused_scene.is_some() {
                g.exit_fusion();
                f.request();
            }
        })?;
    }

    listen(page, "pointerdown", {
        let (s, f) = (state.clone(), frame.clone());
        move |ev| {
            let Some(e) = ev.dyn_ref::<PointerEvent>() else {
                return;
            };
            let Some(el) = ev_el(&ev) else { return };
            let mut g = s.borrow_mut();
            if g.trigger.contains(Some(&el)) && e.button() == 0 {
                g.press = Some(ptr_point(e));
                g.dragged = false;
                g.leave_at = None;
                flag(&g.guide, "data-pressed", true);
            } else if g.mode == Mode::Menu && !g.guide.contains(Some(&el)) {
                g.menu(false);
                g.act(hint_el(&el));
            }
            f.request();
        }
    })?;

    for (target, event) in [
        (page.as_ref(), "pointerup"),
        (page.as_ref(), "pointercancel"),
        (window.as_ref(), "blur"),
    ] {
        let (s, f) = (state.clone(), frame.clone());
        listen(target, event, move |_| {
            let mut g = s.borrow_mut();
            g.press = None;
            flag(&g.guide, "data-pressed", false);
            f.request();
        })?;
    }

    listen(&trigger, "click", {
        let (s, f) = (state.clone(), frame.clone());
        move |ev| {
            let mut g = s.borrow_mut();
            if g.dragged
                && ev
                    .dyn_ref::<web_sys::MouseEvent>()
                    .is_some_and(|e| e.detail() != 0)
            {
                g.dragged = false;
                return;
            }
            let open = g.mode != Mode::Menu;
            g.menu(open);
            f.request();
        }
    })?;

    listen(&choices, "click", {
        let (s, f) = (state.clone(), frame.clone());
        move |ev| {
            if ev_el(&ev)
                .and_then(|el| el.closest("a").ok().flatten())
                .is_some()
            {
                s.borrow_mut().menu(false);
                f.request();
            }
        }
    })?;

    listen(&guide, "keydown", {
        let (s, f, trigger, choices) = (
            state.clone(),
            frame.clone(),
            trigger.clone(),
            choices.clone(),
        );
        move |ev| {
            let Some(e) = ev.dyn_ref::<KeyboardEvent>() else {
                return;
            };
            let key = e.key();
            match key.as_str() {
                "Escape" => {
                    e.prevent_default();
                    e.stop_propagation();
                    s.borrow_mut().menu(false);
                    let _ = trigger.focus();
                }
                "ArrowDown" | "ArrowUp" => {
                    e.prevent_default();
                    s.borrow_mut().menu(true);
                    if let Ok(links) = choices.query_selector_all("a") {
                        let active = choices.owner_document().and_then(|d| d.active_element());
                        let current = (0..links.length())
                            .find(|&i| links.item(i).and_then(|n| n.dyn_into().ok()) == active);
                        let n = i64::from(links.length());
                        if n > 0 {
                            let next = match (current, key.as_str()) {
                                (None, "ArrowUp") => n - 1,
                                (None, _) => 0,
                                (Some(i), "ArrowUp") => (i64::from(i) - 1).rem_euclid(n),
                                (Some(i), _) => (i64::from(i) + 1) % n,
                            };
                            if let Some(el) = links
                                .item(next as u32)
                                .and_then(|v| v.dyn_into::<HtmlElement>().ok())
                            {
                                let _ = el.focus();
                            }
                        }
                    }
                }
                _ => return,
            }
            f.request();
        }
    })?;

    listen(page, "focusin", {
        let (s, f) = (state.clone(), frame.clone());
        move |ev| {
            let Some(el) = ev_el(&ev) else { return };
            let mut g = s.borrow_mut();
            if g.guide.contains(Some(&el)) {
                g.leave_at = None;
            } else {
                if g.mode == Mode::Menu {
                    g.menu(false);
                }
                if el.matches(":focus-visible").unwrap_or(false) {
                    g.pointer = None;
                    g.act(hint_el(&el));
                }
            }
            f.request();
        }
    })?;

    listen(window, "scroll", {
        let (s, f) = (state.clone(), frame.clone());
        move |_| {
            let mut g = s.borrow_mut();
            if g.mode == Mode::Menu {
                return;
            }
            let under_pointer = g
                .pointer
                .and_then(|p| g.page.element_from_point(p.x as f32, p.y as f32));
            let on_bubble = !g.guide.has_attribute("data-fused")
                && under_pointer
                    .as_ref()
                    .is_some_and(|e| g.guide.contains(Some(e)));
            if let Some(scene_idx) = under_pointer.as_ref().and_then(fusion_zone) {
                g.enter_fusion(scene_idx);
            } else if on_bubble && g.target.is_some() {
                g.leave_at = None;
                g.dirty = true;
            } else if let Some(target) = under_pointer.as_ref().and_then(hint_el) {
                g.act(Some(target));
            } else {
                g.act(None);
                g.mode = Mode::Scroll;
            }
            f.request();
        }
    })?;

    for (ev_name, attr) in [
        ("kense:copied-cmd", "data-copy-cmd-msg"),
        ("kense:copied-text", "data-copy-cmd-msg"),
        ("kense:copied-prompt", "data-copy-prompt-msg"),
    ] {
        let (s, f) = (state.clone(), frame.clone());
        listen(page, ev_name, move |_| {
            s.borrow_mut().speak_feedback(attr);
            f.request();
        })?;
    }

    listen(page, "kense:menu-change", {
        let (s, f) = (state.clone(), frame.clone());
        move |_| {
            let mut g = s.borrow_mut();
            let mega_open = g
                .page
                .document_element()
                .is_some_and(|e| e.has_attribute("data-mega-open"));
            if mega_open {
                g.exit_fusion();
                if g.mode == Mode::Menu {
                    g.menu(false);
                }
            }
            flag(&g.guide, "data-concealed", mega_open);
            f.request();
        }
    })?;

    listen(page, "kense:belt-change", {
        let (s, f) = (state.clone(), frame.clone());
        move |_| {
            s.borrow_mut().dirty = true;
            f.request();
        }
    })?;

    for (target, event) in [
        (window.as_ref(), "resize"),
        (page.as_ref(), "visibilitychange"),
        (media.as_ref(), "change"),
        (page.fonts().as_ref(), "loadingdone"),
    ] {
        let (s, f) = (state.clone(), frame.clone());
        listen(target, event, move |_| {
            s.borrow_mut().dirty = true;
            f.request();
        })?;
    }

    let resized = Closure::<dyn FnMut()>::new({
        let (s, f) = (state.clone(), frame.clone());
        move || {
            s.borrow_mut().dirty = true;
            f.request();
        }
    });
    let observer = ResizeObserver::new(resized.as_ref().unchecked_ref())?;
    observer.observe(root);
    observer.observe(&state.borrow().speech);
    resized.forget();

    frame.request();
    Ok(())
}
