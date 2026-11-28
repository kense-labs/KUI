//! Spring-driven Hero inline product belt and 3D perspective flip-card tooltip (`#kense-hero-hint`).

use crate::cover::{Frame, Result, css, element, flag, listen, viewport};
use js_sys::Array;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
    AddEventListenerOptions, Document, Element, Event, HtmlElement, IntersectionObserver,
    IntersectionObserverEntry, KeyboardEvent, MediaQueryList, PointerEvent, ResizeObserver,
    WheelEvent, Window,
};

struct BeltItem {
    el: HtmlElement,
    domain: String,
    href: String,
    external: bool,
    prod: String,
    phrase: String,
    hint: String,
    width_px: f64,
}

struct FlipTransition {
    outgoing: HtmlElement,
    incoming: HtmlElement,
    start_ms: f64,
}

struct BeltState {
    window: Window,
    document: Document,
    reduced: MediaQueryList,
    anchor: HtmlElement,
    hint_root: HtmlElement,
    items: Vec<BeltItem>,
    pos: f64,
    vel: f64,
    goal: f64,
    step_y: f64,
    active_idx: usize,
    visible: bool,
    hovered: bool,
    focused: bool,
    hint_hovered: bool,
    scrubbing: bool,
    auto_timer: Option<i32>,
    snap_timer: Option<i32>,
    close_timer: Option<i32>,
    flip: Option<FlipTransition>,
    last_ms: f64,
}

fn smooth01(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn ease_out_expo(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

impl BeltState {
    fn now(&self) -> f64 {
        self.window
            .performance()
            .map(|p| p.now())
            .unwrap_or_default()
    }

    fn clear_timer(window: &Window, slot: &mut Option<i32>) {
        if let Some(id) = slot.take() {
            window.clear_timeout_with_handle(id);
        }
    }

    fn measure(&mut self) {
        let font_px = self
            .window
            .get_computed_style(&self.anchor)
            .ok()
            .flatten()
            .and_then(|s| s.get_property_value("font-size").ok())
            .and_then(|v| v.trim_end_matches("px").parse::<f64>().ok())
            .unwrap_or(36.0);
        self.step_y = (font_px * 1.66).max(40.0);

        for item in &mut self.items {
            let w = f64::from(item.el.offset_width());
            if w > 12.0 {
                item.width_px = w;
            }
        }
    }

    fn clear_flip(&mut self) {
        if let Some(f) = self.flip.take() {
            f.outgoing.remove();
            let _ = f.incoming.style().remove_property("opacity");
            let _ = f.incoming.style().remove_property("transform");
        }
    }

    fn position_hint(&self) {
        if !self.hint_root.has_attribute("data-open") {
            return;
        }
        let (vw, _) = viewport(&self.window);
        let r = self.anchor.get_bounding_client_rect();
        let hw = f64::from(self.hint_root.offset_width())
            .max(280.0)
            .min(vw - 32.0);
        let hh = f64::from(self.hint_root.offset_height()).max(118.0);
        let x = (r.left() + (r.width() - hw) * 0.5).clamp(16.0, (vw - hw - 16.0).max(16.0));
        let y = if r.top() - hh - 14.0 >= 64.0 {
            r.top() - hh - 14.0
        } else {
            r.bottom() + 14.0
        };
        css(&self.hint_root, "left", &format!("{x:.1}px"));
        css(&self.hint_root, "top", &format!("{y:.1}px"));
    }

    fn open_hint(&mut self) {
        Self::clear_timer(&self.window, &mut self.close_timer);
        flag(&self.hint_root, "data-open", true);
        let _ = self.hint_root.set_attribute("aria-hidden", "false");
        self.position_hint();
        if let Ok(ev) = Event::new("kense:belt-change") {
            let _ = self.document.dispatch_event(&ev);
        }
    }

    fn close_hint(&mut self) {
        Self::clear_timer(&self.window, &mut self.close_timer);
        if !self.hint_root.has_attribute("data-open") {
            return;
        }
        self.clear_flip();
        flag(&self.hint_root, "data-open", false);
        let _ = self.hint_root.set_attribute("aria-hidden", "true");
        if let Ok(ev) = Event::new("kense:belt-change") {
            let _ = self.document.dispatch_event(&ev);
        }
    }

    fn sync_hint_domain(&mut self, domain: &str, animate_3d: bool) {
        self.clear_flip();
        let cards = match self.hint_root.query_selector_all("[data-hint-card]") {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut prev_visible: Option<HtmlElement> = None;
        let mut next_card: Option<HtmlElement> = None;
        for i in 0..cards.length() {
            let Some(card) = cards.item(i).and_then(|n| n.dyn_into::<HtmlElement>().ok()) else {
                continue;
            };
            let is_match = card.get_attribute("data-hint-card").as_deref() == Some(domain);
            if !card.hidden() && !is_match {
                prev_visible = Some(card.clone());
            }
            card.set_hidden(!is_match);
            if is_match {
                next_card = Some(card);
            }
        }

        if animate_3d
            && !self.reduced.matches()
            && self.hint_root.has_attribute("data-open")
            && let (Some(prev), Some(incoming)) = (prev_visible, next_card)
            && let Ok(clone_node) = prev.clone_node_with_deep(true)
            && let Ok(outgoing) = clone_node.dyn_into::<HtmlElement>()
        {
            outgoing.set_hidden(false);
            let _ = outgoing.set_attribute("aria-hidden", "true");
            css(&outgoing, "position", "absolute");
            css(&outgoing, "left", "0");
            css(&outgoing, "top", "0");
            css(&outgoing, "pointer-events", "none");
            css(&outgoing, "z-index", "0");
            css(&incoming, "position", "relative");
            css(&incoming, "z-index", "1");
            let _ = self.hint_root.insert_before(&outgoing, Some(&incoming));
            self.flip = Some(FlipTransition {
                outgoing,
                incoming,
                start_ms: self.now(),
            });
        }
        self.position_hint();
    }

    fn apply_visuals(&mut self) {
        let n = self.items.len();
        if n == 0 {
            return;
        }
        let nf = n as f64;
        let base = self.pos.floor();
        let frac = self.pos - base;
        let t = smooth01(frac);
        let i0 = ((base as isize).rem_euclid(n as isize)) as usize;
        let i1 = (i0 + 1) % n;
        let w = self.items[i0].width_px + (self.items[i1].width_px - self.items[i0].width_px) * t;
        if w > 12.0 {
            css(&self.anchor, "width", &format!("{w:.1}px"));
        }

        let half = nf * 0.5;
        for (idx, item) in self.items.iter().enumerate() {
            let raw = (idx as f64 - self.pos).rem_euclid(nf);
            let rel = if raw >= half { raw - nf } else { raw };
            let dist = rel.abs().min(1.0);
            let fade = smooth01(dist);
            let opacity = if rel.abs() > 1.08 {
                0.0
            } else {
                (1.0 - 0.89 * fade).max(0.0)
            };
            flag(&item.el, "data-active", rel.abs() < 0.5);
            css(
                &item.el,
                "transform",
                &format!(
                    "translate3d(-50%, -50%, 0) translate3d(0, {:.2}px, 0)",
                    rel * self.step_y
                ),
            );
            css(&item.el, "opacity", &format!("{opacity:.3}"));
        }

        let nearest = ((self.pos.round() as isize).rem_euclid(n as isize)) as usize;
        if nearest != self.active_idx {
            self.active_idx = nearest;
            let domain = self.items[nearest].domain.clone();
            let href = self.items[nearest].href.clone();
            let external = self.items[nearest].external;
            let aria = format!(
                "{} · {}",
                self.items[nearest].phrase, self.items[nearest].prod
            );
            let hint = self.items[nearest].hint.clone();

            let _ = self.anchor.set_attribute("data-domain", &domain);
            let _ = self.anchor.set_attribute("href", &href);
            let _ = self.anchor.set_attribute("aria-label", &aria);
            let _ = self.anchor.set_attribute("data-kense-hint", &hint);
            if external {
                let _ = self.anchor.set_attribute("target", "_blank");
                let _ = self.anchor.set_attribute("rel", "noreferrer");
            } else {
                let _ = self.anchor.remove_attribute("target");
                let _ = self.anchor.remove_attribute("rel");
            }
            self.sync_hint_domain(&domain, true);
            if let Ok(ev) = Event::new("kense:belt-change") {
                let _ = self.document.dispatch_event(&ev);
            }
        } else if self.hint_root.has_attribute("data-open") {
            self.position_hint();
        }
    }

    fn tick(&mut self, now: f64) -> bool {
        if self.document.hidden() {
            self.last_ms = 0.0;
            return false;
        }
        let dt = if self.last_ms == 0.0 {
            0.016
        } else {
            ((now - self.last_ms) / 1000.0).clamp(0.001, 0.032)
        };
        self.last_ms = now;

        let mut keep = false;
        if self.reduced.matches() {
            self.pos = self.goal.round();
            self.vel = 0.0;
        } else {
            // Critically damped spring: omega = 15 rad/s
            let omega = 15.0;
            let err = self.pos - self.goal;
            let exp_term = (-omega * dt).exp();
            let k = self.vel + omega * err;
            self.pos = self.goal + (err + k * dt) * exp_term;
            self.vel = (self.vel - omega * k * dt) * exp_term;

            if !self.scrubbing && (self.pos - self.goal).abs() < 0.0006 && self.vel.abs() < 0.008 {
                self.pos = self.goal;
                self.vel = 0.0;
            } else {
                keep = true;
            }
        }

        self.apply_visuals();

        // Step 3D perspective card flip if active
        if let Some(f) = &self.flip {
            let elapsed = (now - f.start_ms).max(0.0);
            let p_out = (elapsed / 420.0).clamp(0.0, 1.0);
            let p_in = ((elapsed - 35.0) / 460.0).clamp(0.0, 1.0);
            let eo = smooth01(p_out);
            let ei = ease_out_expo(p_in);

            css(
                &f.outgoing,
                "opacity",
                &format!("{:.3}", (1.0 - eo * 1.15).max(0.0)),
            );
            css(
                &f.outgoing,
                "transform",
                &format!(
                    "translate3d(0, {:.1}px, {:.1}px) rotateX({:.2}deg)",
                    -18.0 * eo,
                    -96.0 * eo,
                    6.0 * eo
                ),
            );
            css(&f.incoming, "opacity", &format!("{ei:.3}"));
            css(
                &f.incoming,
                "transform",
                &format!(
                    "translate3d(0, {:.1}px, {:.1}px) rotateX({:.2}deg)",
                    26.0 * (1.0 - ei),
                    56.0 * (1.0 - ei),
                    -5.0 * (1.0 - ei)
                ),
            );

            if p_out >= 1.0 && p_in >= 1.0 {
                self.clear_flip();
            } else {
                keep = true;
            }
        }

        if !keep {
            self.last_ms = 0.0;
        }
        keep
    }
}

fn schedule_auto(state: &Rc<RefCell<BeltState>>, frame: &Rc<Frame>) {
    let should_arm = {
        let mut s = state.borrow_mut();
        let win = s.window.clone();
        BeltState::clear_timer(&win, &mut s.auto_timer);
        s.visible
            && !s.hovered
            && !s.focused
            && !s.hint_hovered
            && !s.scrubbing
            && !s.document.hidden()
            && !s.reduced.matches()
    };
    if !should_arm {
        return;
    }
    let st = state.clone();
    let fr = frame.clone();
    let cb = Closure::once_into_js(move || {
        {
            let mut s = st.borrow_mut();
            s.auto_timer = None;
            if !s.visible || s.hovered || s.focused || s.hint_hovered || s.scrubbing {
                return;
            }
            s.goal = s.pos.round() + 1.0;
        }
        fr.request();
        schedule_auto(&st, &fr);
    });
    let mut s = state.borrow_mut();
    s.auto_timer = s
        .window
        .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 2950)
        .ok();
}

pub(super) fn init(
    window: &Window,
    document: &Document,
    root: &Element,
    reduced: &MediaQueryList,
) -> Result {
    let Ok(region) = element::<HtmlElement>(root, "[data-hero-belt-region]") else {
        return Ok(());
    };
    let anchor: HtmlElement = element(&region, "[data-hero-solution-word]")?;
    let hint_root: HtmlElement = element(root, "[data-hero-hint]")?;

    // Move `#kense-hero-hint` to `document.body` so hero `overflow: clip` never clips the 3D flip card!
    if let Some(body) = document.body() {
        let _ = body.append_child(&hint_root);
    }

    let nodes = region.query_selector_all("[data-belt-item]")?;
    let mut items = Vec::with_capacity(nodes.length() as usize);
    for i in 0..nodes.length() {
        let Some(el) = nodes.item(i).and_then(|n| n.dyn_into::<HtmlElement>().ok()) else {
            continue;
        };
        let domain = el.get_attribute("data-belt-item").unwrap_or_default();
        let href = el.get_attribute("data-belt-href").unwrap_or_default();
        let external = el.get_attribute("data-belt-ext").as_deref() == Some("1");
        let prod = el.get_attribute("data-belt-prod").unwrap_or_default();
        let hint = el.get_attribute("data-belt-hint").unwrap_or_default();
        let phrase = el
            .query_selector(".kense-belt-phrase")
            .ok()
            .flatten()
            .and_then(|p| p.text_content())
            .unwrap_or_default();
        items.push(BeltItem {
            el,
            domain,
            href,
            external,
            prod,
            phrase,
            hint,
            width_px: 220.0,
        });
    }
    if items.is_empty() {
        return Ok(());
    }

    flag(&region, "data-belt-ready", true);

    let mut initial = BeltState {
        window: window.clone(),
        document: document.clone(),
        reduced: reduced.clone(),
        anchor: anchor.clone(),
        hint_root: hint_root.clone(),
        items,
        pos: 0.0,
        vel: 0.0,
        goal: 0.0,
        step_y: 56.0,
        active_idx: 0,
        visible: true,
        hovered: false,
        focused: false,
        hint_hovered: false,
        scrubbing: false,
        auto_timer: None,
        snap_timer: None,
        close_timer: None,
        flip: None,
        last_ms: 0.0,
    };
    initial.measure();
    initial.apply_visuals();

    let state = Rc::new(RefCell::new(initial));
    let frame = Frame::new(window, {
        let st = state.clone();
        move |now| st.borrow_mut().tick(now)
    });

    // ResizeObserver keeps inline width & drum spacing crisp across font load & viewport resize
    let ro_cb = Closure::<dyn FnMut(Array)>::new({
        let st = state.clone();
        move |_: Array| {
            if let Ok(mut s) = st.try_borrow_mut() {
                s.measure();
                s.apply_visuals();
            }
        }
    });
    if let Ok(ro) = ResizeObserver::new(ro_cb.as_ref().unchecked_ref()) {
        ro.observe(&region);
        std::mem::forget(ro);
    }
    ro_cb.forget();

    // IntersectionObserver pauses auto-rotation when scrolled past the Hero
    let io_cb = Closure::<dyn FnMut(Array)>::new({
        let st = state.clone();
        let fr = frame.clone();
        move |entries: Array| {
            if let Ok(mut s) = st.try_borrow_mut() {
                for entry in entries.iter() {
                    let Ok(e) = entry.dyn_into::<IntersectionObserverEntry>() else {
                        continue;
                    };
                    s.visible = e.is_intersecting();
                }
            }
            schedule_auto(&st, &fr);
        }
    });
    if let Ok(io) = IntersectionObserver::new(io_cb.as_ref().unchecked_ref()) {
        io.observe(&region);
        std::mem::forget(io);
    }
    io_cb.forget();

    // Hover & focus open the 3D flip-card tooltip
    let schedule_close = {
        let st = state.clone();
        let fr = frame.clone();
        move || {
            let win = st.borrow().window.clone();
            BeltState::clear_timer(&win, &mut st.borrow_mut().close_timer);
            let st2 = st.clone();
            let fr2 = fr.clone();
            let cb = Closure::once_into_js(move || {
                {
                    let mut s = st2.borrow_mut();
                    s.close_timer = None;
                    if !s.hovered && !s.focused && !s.hint_hovered {
                        s.close_hint();
                    }
                }
                schedule_auto(&st2, &fr2);
            });
            st.borrow_mut().close_timer = win
                .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 140)
                .ok();
        }
    };

    listen(&anchor, "pointerenter", {
        let st = state.clone();
        let fr = frame.clone();
        move |ev| {
            if ev
                .dyn_ref::<PointerEvent>()
                .is_some_and(|p| p.pointer_type() == "touch")
            {
                return;
            }
            {
                let mut s = st.borrow_mut();
                s.hovered = true;
                BeltState::clear_timer(&s.window.clone(), &mut s.auto_timer);
                s.open_hint();
            }
            fr.request();
        }
    })?;

    listen(&anchor, "pointerleave", {
        let st = state.clone();
        let close = schedule_close.clone();
        move |_| {
            st.borrow_mut().hovered = false;
            close();
        }
    })?;

    listen(&anchor, "focus", {
        let st = state.clone();
        move |_| {
            let mut s = st.borrow_mut();
            s.focused = true;
            BeltState::clear_timer(&s.window.clone(), &mut s.auto_timer);
            s.open_hint();
        }
    })?;

    listen(&anchor, "blur", {
        let st = state.clone();
        let close = schedule_close.clone();
        move |_| {
            st.borrow_mut().focused = false;
            close();
        }
    })?;

    listen(&hint_root, "pointerenter", {
        let st = state.clone();
        move |_| {
            let mut s = st.borrow_mut();
            s.hint_hovered = true;
            let win = s.window.clone();
            BeltState::clear_timer(&win, &mut s.close_timer);
            BeltState::clear_timer(&win, &mut s.auto_timer);
        }
    })?;

    listen(&hint_root, "pointerleave", {
        let st = state.clone();
        let close = schedule_close;
        move |_| {
            st.borrow_mut().hint_hovered = false;
            close();
        }
    })?;

    // Wheel scrubbing over `[data-hero-belt-region]` (`passive: false` so wheel scrubs belt smoothly)
    let wheel_cb = Closure::<dyn FnMut(Event)>::new({
        let st = state.clone();
        let fr = frame.clone();
        move |ev: Event| {
            let Some(we) = ev.dyn_ref::<WheelEvent>() else {
                return;
            };
            if we.ctrl_key() {
                return;
            }
            let raw = if we.delta_x().abs() > we.delta_y().abs() {
                we.delta_x()
            } else {
                we.delta_y()
            };
            let scale = match we.delta_mode() {
                1 => 16.0,
                2 => 400.0,
                _ => 1.0,
            };
            let delta = raw * scale;
            if delta.abs() < 0.5 {
                return;
            }
            ev.prevent_default();

            let win = {
                let mut s = st.borrow_mut();
                let win = s.window.clone();
                BeltState::clear_timer(&win, &mut s.auto_timer);
                BeltState::clear_timer(&win, &mut s.snap_timer);
                if s.reduced.matches() {
                    s.goal = s.pos.round() + delta.signum();
                    s.pos = s.goal;
                    s.apply_visuals();
                    return;
                }
                s.scrubbing = true;
                let step = (delta / 170.0).clamp(-0.68, 0.68);
                s.goal = (s.goal + step).clamp(s.pos - 1.6, s.pos + 1.6);
                win
            };
            fr.request();

            let st2 = st.clone();
            let fr2 = fr.clone();
            let snap = Closure::once_into_js(move || {
                {
                    let mut s = st2.borrow_mut();
                    s.snap_timer = None;
                    s.scrubbing = false;
                    s.goal = s.goal.round();
                }
                fr2.request();
                schedule_auto(&st2, &fr2);
            });
            st.borrow_mut().snap_timer = win
                .set_timeout_with_callback_and_timeout_and_arguments_0(snap.unchecked_ref(), 115)
                .ok();
        }
    });
    let opts = AddEventListenerOptions::new();
    opts.set_passive(false);
    region.add_event_listener_with_callback_and_add_event_listener_options(
        "wheel",
        wheel_cb.as_ref().unchecked_ref(),
        &opts,
    )?;
    wheel_cb.forget();

    listen(document, "keydown", {
        let st = state.clone();
        move |ev| {
            if ev
                .dyn_ref::<KeyboardEvent>()
                .is_some_and(|k| k.key() == "Escape")
            {
                st.borrow_mut().close_hint();
            }
        }
    })?;

    listen(window, "scroll", {
        let st = state.clone();
        move |_| st.borrow_mut().close_hint()
    })?;

    listen(document, "visibilitychange", {
        let st = state.clone();
        let fr = frame.clone();
        move |_| schedule_auto(&st, &fr)
    })?;

    schedule_auto(&state, &frame);
    Ok(())
}
