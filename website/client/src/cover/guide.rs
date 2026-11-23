//! KUI companion bot (`#cover-guide`) upgraded to Kense studio standards while preserving KUI's signature eyes.
//!
//! Combines KUI's iconic slanted body and expressive rectangular eyes with silky tapered comet-ribbon
//! flight trails, parabolic hop arcs, damped landing cushion springs, smooth `--shell-w`/`--shell-h`
//! morphing, staggered character speech, copy celebration feedback, and strict topbar/mega non-collision.

use super::{
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

struct Guide {
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
    docks: Vec<HtmlElement>,
    library: HtmlElement,
    install: HtmlElement,
    header: Option<Element>,
    dock: usize,
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
    mode: Mode,
    flight: Flight,
    last_ms: f64,
    dirty: bool,
}

impl Guide {
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
                r#"<span class="speech-char" style="--char-delay:{delay}ms">"#
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

    fn speak(&mut self) {
        let Some(target) = &self.target else { return };
        let Some(hint) = target.get_attribute("data-guide-hint") else {
            return;
        };
        if hint.is_empty() {
            return;
        }
        self.set_title_animated(&hint);
        let body = target
            .get_attribute("data-guide-body")
            .filter(|s| !s.is_empty());
        self.body.set_hidden(body.is_none());
        self.body.set_text_content(body.as_deref());
        if let Some(source) = target.query_selector(".sample-copy").ok().flatten()
            && let Some(payload) = source.get_attribute("data-copy")
        {
            let _ = self.copy.set_attribute("data-copy", &payload);
            if let Some(label) = source.get_attribute("aria-label") {
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

    fn act(&mut self, target: Option<Element>) {
        self.leave_at = None;
        self.mode = Mode::Pointer;
        if self.target != target {
            if self.feedback_until.is_none() {
                self.clear_speech();
            }
            self.target = target;
            if self
                .target
                .as_ref()
                .is_some_and(|el| el.has_attribute("data-guide-prompt"))
            {
                self.speak();
            }
        }
        self.dirty = true;
    }

    fn menu(&mut self, open: bool) {
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
        if let Some(s) = self.trigger.get_attribute(key) {
            let _ = self.trigger.set_attribute("aria-label", &s);
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
        let centre = r.x() + r.width() / 2.0;
        if el == self.docks[0].as_ref() {
            return Point {
                x: centre,
                y: r.bottom() - 11.0,
            };
        }
        if el == self.docks[1].as_ref() {
            return Point {
                x: r.left() - 38.0,
                y: r.top() + r.height() / 2.0 + 15.0,
            };
        }
        let gap = if el.has_attribute("data-guide-prompt") {
            13.0
        } else {
            14.0
        };
        Point {
            x: centre,
            y: r.top() - gap,
        }
    }

    fn destination(&mut self) -> Point {
        let (vw, vh) = viewport(&self.window);
        let top = self.header_bottom() + 14.0;
        let home = self.perch(&self.docks[0]);
        let scroll = self.window.scroll_y().unwrap_or_default();
        let reduced = self.media.matches();
        let p = if matches!(self.mode, Mode::Menu | Mode::Held) {
            self.flight.position
        } else if let Some(target) = &self.target {
            self.perch(target)
        } else if reduced {
            home
        } else if self.mode == Mode::Scroll || scroll > 64.0 {
            let library = self.library.get_bounding_client_rect();
            let install = self.install.get_bounding_client_rect();
            let beside = |r: &web_sys::DomRect| Point {
                x: (r.right() + 41.0).clamp(48.0, vw - 48.0),
                y: r.top() + r.height() / 2.0 + 16.0,
            };
            let max = (f64::from(
                self.page
                    .document_element()
                    .map_or(0, |e| e.scroll_height()),
            ) - vh)
                .max(1.0);
            let arrival = (library.top() + scroll - vh * 0.55).clamp(max * 0.16, max * 0.78);
            let end = (install.top() + scroll - vh * 0.54)
                .min(max)
                .max(arrival + 1.0);
            if scroll < arrival {
                home.blend(beside(&library), scroll / arrival)
            } else {
                beside(&library).blend(beside(&install), (scroll - arrival) / (end - arrival))
            }
        } else if let Some(pointer) = self.pointer.filter(|_| vw >= 720.0) {
            let mut best = home;
            let mut score = f64::INFINITY;
            let mut selected = 0;
            for (i, el) in self.docks.iter().enumerate() {
                let at = self.perch(el);
                let cost = ((at.x - pointer.x) * 0.42).hypot(at.y - pointer.y)
                    - if self.dock == i { 26.0 } else { 0.0 };
                if cost < score {
                    score = cost;
                    best = at;
                    selected = i;
                }
            }
            self.dock = selected;
            best
        } else {
            home
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
            x: p.x.min(vw - width / 2.0 - 12.0).max(width / 2.0 + 12.0),
            y,
        }
    }

    fn step_trail(&mut self, now: f64, x: f64, y: f64, flying: bool, reduced: bool) -> bool {
        let (Some(canvas), Some(ctx)) = (&self.trail_canvas, &self.trail_ctx) else {
            return false;
        };
        let intro_playing = self
            .page
            .body()
            .is_some_and(|b| b.has_attribute("data-kui-intro-active"));
        if reduced || self.guide.has_attribute("data-concealed") || intro_playing {
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

        if self.dirty {
            let goal = self.destination();
            let jump = self.flight.position.distance(goal);
            if self.mode == Mode::Pointer && !self.media.matches() && jump > 32.0 {
                self.hop_start = now;
                self.hop_dur = (0.18 + jump.sqrt() * 0.014).clamp(0.20, 0.46) * 1000.0;
                self.hop_amp = (jump * 0.14).clamp(8.0, 34.0);
            }
            self.flight.aim(goal, self.mode, now);
            self.dirty = false;
        }

        let reduced = self.media.matches();
        let rest = self.flight.step(now, self.mode, reduced);
        let x = self.flight.position.x;
        let mut y = self.flight.position.y;

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

fn event_target(event: &web_sys::Event) -> Option<Element> {
    event.target()?.dyn_into().ok()
}

fn hint(el: &Element) -> Option<Element> {
    if el
        .closest("header.topbar,.kense-mega-panel")
        .ok()
        .flatten()
        .is_some()
    {
        return None;
    }
    el.closest(".kui-cover-page [data-guide-hint]")
        .ok()
        .flatten()
}

fn pointer(e: &PointerEvent) -> Point {
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
    let guide: HtmlElement = element(root, "#cover-guide")?;
    let shell: HtmlElement = element(&guide, ".shell")?;
    let trigger: HtmlElement = element(&guide, ".trigger")?;
    let choices: HtmlElement = element(&guide, ".choices")?;
    let trail_canvas = root
        .query_selector("[data-guide-trail]")?
        .and_then(|n| n.dyn_into::<HtmlCanvasElement>().ok());
    let trail_ctx = trail_canvas.as_ref().and_then(|c| {
        c.get_context("2d")
            .ok()
            .flatten()
            .and_then(|ctx| ctx.dyn_into::<CanvasRenderingContext2d>().ok())
    });

    let docks = [
        "[data-guide-home]",
        ".title-prefix",
        ".cta-row .primary",
        ".cta-row .secondary",
    ]
    .into_iter()
    .map(|sel| element(root, sel))
    .collect::<Result<_>>()?;

    let mut state = Guide {
        window: window.clone(),
        page: page.clone(),
        media: media.clone(),
        guide: guide.clone(),
        shell,
        trigger: trigger.clone(),
        choices: choices.clone(),
        speech: element(&guide, ".speech")?,
        title: element(&guide, ".speech-title")?,
        body: element(&guide, ".speech-body")?,
        copy: element(&guide, ".speech-copy")?,
        trail_canvas,
        trail_ctx,
        trail_nodes: VecDeque::with_capacity(32),
        trail_dirty: false,
        docks,
        library: element(root, "#collection-heading")?,
        install: element(root, "#start-heading")?,
        header: page.query_selector("header.topbar,header")?,
        dock: 0,
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
        mode: Mode::Pointer,
        flight: Flight::default(),
        last_ms: 0.0,
        dirty: true,
    };
    let start = state.destination();
    state.flight.place(start);
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
            let Some(el) = event_target(&ev) else { return };
            if e.pointer_type() == "touch" {
                return;
            }
            let mut g = state.borrow_mut();
            if g.mode == Mode::Menu {
                return;
            }
            let p = pointer(e);
            g.pointer = Some(p);
            if let Some(start) = g.press {
                g.dragged |= p.distance(start) > 8.0;
            } else if g.guide.contains(Some(&el)) {
                g.leave_at = None;
                if g.target.is_none() {
                    g.mode = Mode::Held;
                }
            } else if let Some(target) = hint(&el) {
                g.act(Some(target));
            } else if g.target.is_some() {
                if g.leave_at.is_none() {
                    g.leave_at = Some(g.now() + 680.0);
                }
            } else {
                g.act(None);
            }
            frame.request();
        })?;
    }

    listen(page, "pointerout", {
        let (s, f) = (state.clone(), frame.clone());
        move |ev| {
            if let Some(e) = ev.dyn_ref::<PointerEvent>()
                && e.related_target().is_none()
            {
                let mut g = s.borrow_mut();
                if g.mode != Mode::Menu && g.press.is_none() {
                    g.pointer = None;
                    g.act(None);
                    f.request();
                }
            }
        }
    })?;

    listen(page, "pointerdown", {
        let (s, f) = (state.clone(), frame.clone());
        move |ev| {
            let Some(e) = ev.dyn_ref::<PointerEvent>() else {
                return;
            };
            let Some(el) = event_target(&ev) else { return };
            let mut g = s.borrow_mut();
            if g.trigger.contains(Some(&el)) && e.button() == 0 {
                g.press = Some(pointer(e));
                g.dragged = false;
                g.leave_at = None;
                flag(&g.guide, "data-pressed", true);
            } else if g.mode == Mode::Menu && !g.guide.contains(Some(&el)) {
                g.menu(false);
                g.act(hint(&el));
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
            if event_target(&ev)
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
            let Some(el) = event_target(&ev) else { return };
            let mut g = s.borrow_mut();
            if g.guide.contains(Some(&el)) {
                g.leave_at = None;
            } else {
                if g.mode == Mode::Menu {
                    g.menu(false);
                }
                if el.matches(":focus-visible").unwrap_or(false) {
                    g.pointer = None;
                    g.act(hint(&el));
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
            let on_bubble = under_pointer
                .as_ref()
                .is_some_and(|e| g.guide.contains(Some(e)));
            if on_bubble && g.target.is_some() {
                g.leave_at = None;
                g.dirty = true;
            } else if let Some(target) = under_pointer.as_ref().and_then(hint) {
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
            if mega_open && g.mode == Mode::Menu {
                g.menu(false);
            }
            flag(&g.guide, "data-concealed", mega_open);
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
