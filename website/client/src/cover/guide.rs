//! The guide follows hover, focus and scroll and exposes each selected task.
use super::{
    Frame, Result, css, element, flag,
    flight::{Flight, Mode, Point},
    listen, viewport,
};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
    Document, Element, HtmlElement, KeyboardEvent, MediaQueryList, PointerEvent, ResizeObserver,
    Window,
};

struct Guide {
    window: Window,
    page: Document,
    media: MediaQueryList,
    guide: HtmlElement,
    trigger: HtmlElement,
    choices: HtmlElement,
    speech: HtmlElement,
    title: HtmlElement,
    body: HtmlElement,
    copy: HtmlElement,
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
    mode: Mode,
    flight: Flight,
    dirty: bool,
}

impl Guide {
    fn now(&self) -> f64 {
        self.window
            .performance()
            .map(|p| p.now())
            .unwrap_or_default()
    }
    fn clear_speech(&self) {
        self.speech.set_hidden(true);
        self.copy.set_hidden(true);
        let _ = self.copy.remove_attribute("data-copy");
        let _ = self.copy.remove_attribute("data-copied");
        flag(&self.guide, "data-hint", false);
        flag(&self.guide, "data-below", false);
    }
    fn speak(&mut self) {
        let Some(target) = &self.target else { return };
        let Some(hint) = target.get_attribute("data-guide-hint") else {
            return;
        };
        self.title.set_text_content(Some(&hint));
        let body = target.get_attribute("data-guide-body");
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
        }
        self.speech.set_hidden(false);
        flag(&self.guide, "data-hint", true);
        self.dirty = true;
    }
    fn act(&mut self, target: Option<Element>) {
        self.leave_at = None;
        self.mode = Mode::Pointer;
        if self.target != target {
            self.clear_speech();
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
        self.clear_speech();
        self.choices.set_hidden(!open);
        self.mode = if open { Mode::Menu } else { Mode::Held };
        flag(&self.guide, "data-open", open);
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
        self.dirty = true;
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
            22.0
        };
        Point {
            x: centre,
            y: r.top() - gap,
        }
    }

    fn destination(&mut self) -> Point {
        let (vw, vh) = viewport(&self.window);
        let top = self
            .header
            .as_ref()
            .map_or(0.0, |el| el.get_bounding_client_rect().bottom().max(0.0))
            + 8.0;
        let home = self.perch(&self.docks[0]);
        let scroll = self.window.scroll_y().unwrap_or_default();
        let reduced = self.media.matches();
        let p = if self.mode == Mode::Menu || self.mode == Mode::Held {
            self.flight.position
        } else if let Some(target) = &self.target {
            self.perch(target)
        } else if reduced {
            home
        } else if self.mode == Mode::Scroll || scroll > 64.0 {
            let library = self.library.get_bounding_client_rect();
            let install = self.install.get_bounding_client_rect();
            let beside = |r: &web_sys::DomRect| Point {
                x: r.right() + 41.0,
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
            ((vw - 36.0).min(278.0), 184.0)
        } else if speech {
            (
                f64::from(self.speech.offset_width()),
                f64::from(self.speech.offset_height()),
            )
        } else {
            (56.0, 36.0)
        };
        let below = (menu || speech) && p.y - height < top;
        flag(&self.guide, "data-below", below);
        let y = if below && speech {
            self.target
                .as_ref()
                .map(|t| t.get_bounding_client_rect().bottom() + 10.0)
                .unwrap_or(p.y)
                .min(vh - height - 9.0)
                .max(top)
        } else if below {
            p.y.min(vh - height + 36.0 - 9.0).max(top + 36.0)
        } else {
            p.y.min(vh - 18.0).max(top + height)
        };
        Point {
            x: p.x.min(vw - width / 2.0 - 9.0).max(width / 2.0 + 9.0),
            y,
        }
    }

    fn tick(&mut self, now: f64) -> bool {
        if self.page.hidden() || !self.guide.is_connected() {
            return false;
        }
        if self.leave_at.is_some_and(|at| now >= at) {
            self.act(None);
        }
        if self.dirty {
            let goal = self.destination();
            self.flight.aim(goal, self.mode, now);
            self.dirty = false;
        }
        let rest = self.flight.step(now, self.mode, self.media.matches());
        let Point { x, y } = self.flight.position;
        let a = 1.0 + self.flight.stretch;
        let b = self.flight.shear;
        css(
            &self.guide,
            "--flight-pose",
            &format!("matrix({a:.5},{b:.5},{b:.5},{:.5},0,0)", (1.0 + b * b) / a),
        );
        css(&self.guide, "--guide-x", &format!("{x:.2}px"));
        css(&self.guide, "--guide-y", &format!("{y:.2}px"));
        let (dx, dy) = self
            .pointer
            .map_or((0.0, 0.0), |p| (p.x - x, p.y - y + 18.0));
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
        flag(&self.guide, "data-awake", !self.media.matches());
        flag(&self.guide, "data-placed", true);
        self.dirty
            || !rest
            || self.flight.stretch.abs() + self.flight.shear.abs() > 0.0006
            || self.leave_at.is_some()
    }
}

fn event_target(event: &web_sys::Event) -> Option<Element> {
    event.target()?.dyn_into().ok()
}
fn hint(el: &Element) -> Option<Element> {
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
    let trigger: HtmlElement = element(&guide, ".trigger")?;
    let choices: HtmlElement = element(&guide, ".choices")?;
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
        trigger: trigger.clone(),
        choices: choices.clone(),
        speech: element(&guide, ".speech")?,
        title: element(&guide, ".speech-title")?,
        body: element(&guide, ".speech-body")?,
        copy: element(&guide, ".speech-copy")?,
        docks,
        library: element(root, "#collection-heading")?,
        install: element(root, "#start-heading")?,
        header: page.query_selector("header")?,
        dock: 0,
        target: None,
        pointer: None,
        press: None,
        dragged: false,
        leave_at: None,
        mode: Mode::Pointer,
        flight: Flight::default(),
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
                    g.leave_at = Some(g.now() + 760.0);
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
    listen(&guide, "keydown", {
        let (s, f, trigger) = (state.clone(), frame.clone(), trigger.clone());
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
            // Scrolling can be delivered after pointerover on the new card, especially
            // when focus or the browser itself scrolls a control into view.
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
