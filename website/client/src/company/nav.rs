//! Company topbar mega-popovers, sliding column highlights, logo springs, and product badge cards.

use crate::cover::{Frame, Result, css, flag, listen, viewport};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
    Document, Element, Event, HtmlElement, KeyboardEvent, MediaQueryList, PointerEvent, Window,
};

struct LogoSpring {
    el: HtmlElement,
    scale: f64,
    vel: f64,
}

struct NavState {
    window: Window,
    document: Document,
    cluster: Option<HtmlElement>,
    veil: Option<HtmlElement>,
    open_menu: Option<String>,
    menu_close_timer: Option<i32>,
    island_timer: Option<i32>,
    popover: Option<HtmlElement>,
    active_badge: Option<Element>,
    popover_hovered: bool,
    pop_close_timer: Option<i32>,
    springs: Vec<LogoSpring>,
    last_ms: f64,
}

impl NavState {
    fn clear_menu_timer(&mut self) {
        if let Some(id) = self.menu_close_timer.take() {
            self.window.clear_timeout_with_handle(id);
        }
    }

    fn clear_pop_timer(&mut self) {
        if let Some(id) = self.pop_close_timer.take() {
            self.window.clear_timeout_with_handle(id);
        }
    }

    fn pulse_island(&mut self, reduced: bool) {
        if reduced {
            return;
        }
        let Some(cluster) = &self.cluster else {
            return;
        };
        if let Some(id) = self.island_timer.take() {
            self.window.clear_timeout_with_handle(id);
        }
        flag(cluster, "data-island-press", true);
        let cluster_clone = cluster.clone();
        let cb = Closure::once_into_js(move || {
            flag(&cluster_clone, "data-island-press", false);
        });
        self.island_timer = self
            .window
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 150)
            .ok();
    }

    fn set_menu(&mut self, key: Option<&str>, reduced: bool) {
        self.clear_menu_timer();
        let next = key.map(str::to_owned);
        if self.open_menu == next {
            return;
        }
        if next.is_some() {
            self.close_popover();
            self.pulse_island(reduced);
        }
        self.open_menu = next.clone();

        if let Some(cluster) = &self.cluster {
            flag(cluster, "data-menu-active", next.is_some());
            if let Ok(toggles) = cluster.query_selector_all("[data-kense-menu-toggle]") {
                for i in 0..toggles.length() {
                    if let Some(btn) = toggles.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                        let active = btn.get_attribute("data-kense-menu-toggle").as_deref() == key;
                        let _ = btn
                            .set_attribute("aria-expanded", if active { "true" } else { "false" });
                    }
                }
            }
            if let Ok(panels) = cluster.query_selector_all("[data-kense-mega]") {
                for i in 0..panels.length() {
                    if let Some(panel) = panels
                        .item(i)
                        .and_then(|n| n.dyn_into::<HtmlElement>().ok())
                    {
                        let active = panel.get_attribute("data-kense-mega").as_deref() == key;
                        flag(&panel, "data-open", active);
                        flag(&panel, "inert", !active);
                        let _ = panel
                            .set_attribute("aria-hidden", if active { "false" } else { "true" });
                        if !active {
                            clear_col_highlights(&panel);
                        }
                    }
                }
            }
        }
        if let Some(veil) = &self.veil {
            flag(veil, "data-open", next.is_some());
        }
        if let Some(root) = self.document.document_element() {
            flag(&root, "data-mega-open", next.is_some());
        }
        if let Ok(ev) = Event::new("kense:menu-change") {
            let _ = self.document.dispatch_event(&ev);
        }
    }

    fn open_popover(&mut self, badge: Element) {
        self.clear_pop_timer();
        if self.open_menu.is_some() {
            return;
        }
        let Some(popover) = self.popover.clone() else {
            return;
        };
        let Some(kind) = badge.get_attribute("data-product-badge") else {
            return;
        };

        if let Some(prev) = &self.active_badge
            && prev != &badge
        {
            flag(prev, "data-badge-active", false);
        }
        flag(&badge, "data-badge-active", true);
        self.active_badge = Some(badge.clone());

        let mut matched = false;
        if let Ok(cards) = popover.query_selector_all("[data-popover-card]") {
            for i in 0..cards.length() {
                if let Some(card) = cards.item(i).and_then(|n| n.dyn_into::<HtmlElement>().ok()) {
                    let is_match =
                        card.get_attribute("data-popover-card").as_deref() == Some(kind.as_str());
                    card.set_hidden(!is_match);
                    matched |= is_match;
                }
            }
        }
        if !matched {
            return;
        }

        self.position_popover(&badge, &popover);
        flag(&popover, "inert", false);
        let _ = popover.set_attribute("aria-hidden", "false");
        flag(&popover, "data-open", true);
    }

    fn position_popover(&self, badge: &Element, popover: &HtmlElement) {
        let (vw, vh) = viewport(&self.window);
        let br = badge.get_bounding_client_rect();
        let pw = f64::from(popover.offset_width()).max(320.0).min(vw - 24.0);
        let ph = f64::from(popover.offset_height()).max(188.0);

        let center_x = br.left() + br.width() * 0.5;
        let x = (center_x - pw * 0.5).clamp(12.0, (vw - pw - 12.0).max(12.0));
        let arrow_x = (center_x - x).clamp(20.0, pw - 20.0);

        let below_y = br.bottom() + 10.0;
        let above = below_y + ph > vh - 14.0 && br.top() - ph - 10.0 > 58.0;
        let y = if above {
            (br.top() - ph - 10.0).max(60.0)
        } else {
            below_y.min((vh - ph - 12.0).max(60.0))
        };

        flag(popover, "data-above", above);
        css(popover, "--pop-x", &format!("{x:.1}px"));
        css(popover, "--pop-y", &format!("{y:.1}px"));
        css(popover, "--pop-arrow-x", &format!("{arrow_x:.1}px"));
        css(
            popover,
            "--pop-origin-x",
            &format!("{:.1}%", (arrow_x / pw * 100.0).clamp(10.0, 90.0)),
        );
        css(popover, "--pop-origin-y", if above { "100%" } else { "0%" });
    }

    fn close_popover(&mut self) {
        self.clear_pop_timer();
        self.popover_hovered = false;
        if let Some(prev) = self.active_badge.take() {
            flag(&prev, "data-badge-active", false);
        }
        if let Some(popover) = &self.popover {
            flag(popover, "data-open", false);
            flag(popover, "inert", true);
            let _ = popover.set_attribute("aria-hidden", "true");
        }
    }

    fn kick_logo(&mut self, logo: HtmlElement, reduced: bool) {
        if reduced {
            return;
        }
        if let Some(existing) = self.springs.iter_mut().find(|s| s.el == logo) {
            existing.vel = -4.2;
        } else {
            self.springs.push(LogoSpring {
                el: logo,
                scale: 0.94,
                vel: -4.2,
            });
        }
    }

    fn tick(&mut self, now: f64, reduced: bool) -> bool {
        if reduced || self.springs.is_empty() {
            for s in self.springs.drain(..) {
                let _ = s.el.style().remove_property("transform");
            }
            self.last_ms = 0.0;
            return false;
        }
        let dt = if self.last_ms == 0.0 {
            0.016
        } else {
            ((now - self.last_ms) / 1000.0).clamp(0.001, 0.04)
        };
        self.last_ms = now;
        let k = 720.0_f64;
        let c = 34.0_f64;

        self.springs.retain_mut(|s| {
            let disp = s.scale - 1.0;
            let acc = -k * disp - c * s.vel;
            s.vel += acc * dt;
            s.scale = (s.scale + s.vel * dt).clamp(0.76, 1.18);
            if (s.scale - 1.0).abs() < 0.002 && s.vel.abs() < 0.02 {
                let _ = s.el.style().remove_property("transform");
                false
            } else {
                css(&s.el, "transform", &format!("scale({:.4})", s.scale));
                true
            }
        });
        if self.springs.is_empty() {
            self.last_ms = 0.0;
            false
        } else {
            true
        }
    }
}

fn clear_col_highlights(scope: &Element) {
    if let Ok(cols) = scope.query_selector_all("[data-mega-col]") {
        for i in 0..cols.length() {
            if let Some(col) = cols.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                flag(&col, "data-has-highlight", false);
                if let Ok(Some(hl)) = col.query_selector(".kense-mega-highlight")
                    && let Ok(hl) = hl.dyn_into::<HtmlElement>()
                {
                    css(&hl, "opacity", "0");
                }
            }
        }
    }
}

fn update_col_highlight(entry: &HtmlElement) {
    let Ok(Some(col)) = entry.closest("[data-mega-col]") else {
        return;
    };
    let Ok(col_el) = col.dyn_into::<HtmlElement>() else {
        return;
    };

    // Clear highlights in all sibling columns so moving across columns never leaves stale shadows!
    if let Ok(Some(panel)) = col_el.closest(".kense-mega-panel")
        && let Ok(cols) = panel.query_selector_all("[data-mega-col]")
    {
        for i in 0..cols.length() {
            if let Some(other) = cols.item(i).and_then(|n| n.dyn_into::<HtmlElement>().ok())
                && other != col_el
            {
                flag(&other, "data-has-highlight", false);
                if let Ok(Some(other_hl)) = other.query_selector(".kense-mega-highlight")
                    && let Ok(other_hl) = other_hl.dyn_into::<HtmlElement>()
                {
                    css(&other_hl, "opacity", "0");
                }
            }
        }
    }

    let Ok(Some(hl_node)) = col_el.query_selector(".kense-mega-highlight") else {
        return;
    };
    let Ok(hl) = hl_node.dyn_into::<HtmlElement>() else {
        return;
    };
    let was_active = col_el.has_attribute("data-has-highlight");
    let cr = col_el.get_bounding_client_rect();
    let er = entry.get_bounding_client_rect();
    let x = er.left() - cr.left();
    let y = er.top() - cr.top();

    if !was_active {
        css(&hl, "transition", "opacity 0.14s ease");
    }
    flag(&col_el, "data-has-highlight", true);
    css(&hl, "width", &format!("{:.1}px", er.width()));
    css(&hl, "height", &format!("{:.1}px", er.height()));
    css(
        &hl,
        "transform",
        &format!("translate3d({x:.1}px,{y:.1}px,0)"),
    );
    if !was_active {
        let _ = hl.offset_width();
        let _ = hl.style().remove_property("transition");
    }
    css(&hl, "opacity", "1");
}

pub(super) fn init(window: &Window, document: &Document, reduced: &MediaQueryList) -> Result {
    let cluster = document
        .query_selector("[data-kense-nav-cluster]")?
        .and_then(|e| e.dyn_into::<HtmlElement>().ok());
    let veil = document
        .query_selector("[data-kense-nav-veil]")?
        .and_then(|e| e.dyn_into::<HtmlElement>().ok());
    let popover = document
        .query_selector("[data-kense-popover]")?
        .and_then(|e| e.dyn_into::<HtmlElement>().ok());

    if cluster.is_none() && popover.is_none() {
        return Ok(());
    }
    if let Some(c) = &cluster
        && c.has_attribute("data-nav-bound")
    {
        return Ok(());
    }
    if let Some(c) = &cluster {
        c.set_attribute("data-nav-bound", "")?;
    }

    let state = Rc::new(RefCell::new(NavState {
        window: window.clone(),
        document: document.clone(),
        cluster: cluster.clone(),
        veil: veil.clone(),
        open_menu: None,
        menu_close_timer: None,
        island_timer: None,
        popover: popover.clone(),
        active_badge: None,
        popover_hovered: false,
        pop_close_timer: None,
        springs: Vec::new(),
        last_ms: 0.0,
    }));

    let frame = Frame::new(window, {
        let state = state.clone();
        let media = reduced.clone();
        move |now| state.borrow_mut().tick(now, media.matches())
    });

    // Bind Header Mega-Menu hover, click, sliding highlight & Escape
    if let Some(cluster_el) = cluster {
        listen(&cluster_el, "pointerover", {
            let state = state.clone();
            let frame = frame.clone();
            let media = reduced.clone();
            move |ev| {
                let Some(pe) = ev.dyn_ref::<PointerEvent>() else {
                    return;
                };
                if pe.pointer_type() == "touch" {
                    return;
                }
                let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                    return;
                };
                let mut s = state.borrow_mut();
                s.clear_menu_timer();

                if let Ok(Some(btn)) = target.closest("[data-kense-menu-toggle]")
                    && let Some(key) = btn.get_attribute("data-kense-menu-toggle")
                {
                    if let Some(c) = &s.cluster {
                        clear_col_highlights(c);
                    }
                    let (vw, _) = viewport(&s.window);
                    if vw >= 760.0 {
                        s.set_menu(Some(&key), media.matches());
                    }
                    return;
                }

                if target.closest(".company-nav-link").ok().flatten().is_some() {
                    s.set_menu(None, media.matches());
                    return;
                }

                if let Ok(Some(entry)) = target.closest("[data-mega-entry]")
                    && let Ok(entry_el) = entry.dyn_into::<HtmlElement>()
                {
                    update_col_highlight(&entry_el);
                    if let Ok(Some(logo)) = entry_el.query_selector("[data-mega-logo]")
                        && let Ok(logo_el) = logo.dyn_into::<HtmlElement>()
                    {
                        s.kick_logo(logo_el, media.matches());
                        frame.request();
                    }
                } else if let Some(c) = &s.cluster {
                    clear_col_highlights(c);
                }
            }
        })?;

        listen(&cluster_el, "pointerleave", {
            let state = state.clone();
            let media = reduced.clone();
            move |ev| {
                if ev
                    .dyn_ref::<PointerEvent>()
                    .is_some_and(|p| p.pointer_type() == "touch")
                {
                    return;
                }
                let mut s = state.borrow_mut();
                if let Some(c) = &s.cluster {
                    clear_col_highlights(c);
                }
                if s.open_menu.is_none() {
                    return;
                }
                s.clear_menu_timer();
                let st = state.clone();
                let md = media.clone();
                let cb = Closure::once_into_js(move || {
                    st.borrow_mut().set_menu(None, md.matches());
                });
                s.menu_close_timer = s
                    .window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 175)
                    .ok();
            }
        })?;

        listen(&cluster_el, "click", {
            let state = state.clone();
            let media = reduced.clone();
            move |ev| {
                let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                    return;
                };
                if let Ok(Some(btn)) = target.closest("[data-kense-menu-toggle]")
                    && let Some(key) = btn.get_attribute("data-kense-menu-toggle")
                {
                    ev.prevent_default();
                    let mut s = state.borrow_mut();
                    let next = if s.open_menu.as_deref() == Some(key.as_str()) {
                        None
                    } else {
                        Some(key)
                    };
                    s.set_menu(next.as_deref(), media.matches());
                    return;
                }
                if target
                    .closest(".kense-mega-panel a")
                    .ok()
                    .flatten()
                    .is_some()
                {
                    state.borrow_mut().set_menu(None, media.matches());
                }
            }
        })?;

        listen(&cluster_el, "focusin", {
            let cluster = cluster_el.clone();
            move |ev| {
                if let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok())
                    && let Ok(Some(entry)) = target.closest("[data-mega-entry]")
                    && let Ok(entry_el) = entry.dyn_into::<HtmlElement>()
                {
                    update_col_highlight(&entry_el);
                } else {
                    clear_col_highlights(&cluster);
                }
            }
        })?;
    }

    if let Some(veil_el) = veil {
        listen(&veil_el, "click", {
            let state = state.clone();
            let media = reduced.clone();
            move |_| {
                state.borrow_mut().set_menu(None, media.matches());
            }
        })?;
    }

    // Bind Product Badge Floating Preview Popover across the page
    if let Some(pop_el) = popover {
        listen(document, "pointerover", {
            let state = state.clone();
            move |ev| {
                let Some(pe) = ev.dyn_ref::<PointerEvent>() else {
                    return;
                };
                if pe.pointer_type() == "touch" {
                    return;
                }
                let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                    return;
                };
                let mut s = state.borrow_mut();
                let (vw, _) = viewport(&s.window);
                if vw < 760.0 {
                    return;
                }
                if let Some(pop) = &s.popover
                    && pop.contains(Some(&target))
                {
                    s.popover_hovered = true;
                    s.clear_pop_timer();
                    return;
                }
                s.popover_hovered = false;
                if let Ok(Some(badge)) = target.closest("[data-product-badge]") {
                    s.open_popover(badge);
                } else if s.active_badge.is_some() && s.pop_close_timer.is_none() {
                    let st = state.clone();
                    let cb = Closure::once_into_js(move || {
                        let mut g = st.borrow_mut();
                        if !g.popover_hovered {
                            g.close_popover();
                        }
                    });
                    s.pop_close_timer = s
                        .window
                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                            cb.unchecked_ref(),
                            180,
                        )
                        .ok();
                }
            }
        })?;

        listen(&pop_el, "pointerleave", {
            let state = state.clone();
            move |_| {
                let mut s = state.borrow_mut();
                s.popover_hovered = false;
                s.clear_pop_timer();
                let st = state.clone();
                let cb = Closure::once_into_js(move || {
                    st.borrow_mut().close_popover();
                });
                s.pop_close_timer = s
                    .window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 160)
                    .ok();
            }
        })?;

        listen(document, "focusin", {
            let state = state.clone();
            move |ev| {
                let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                    return;
                };
                let mut s = state.borrow_mut();
                if let Some(pop) = &s.popover
                    && pop.contains(Some(&target))
                {
                    s.clear_pop_timer();
                    return;
                }
                if target.matches(":focus-visible").unwrap_or(false)
                    && let Ok(Some(badge)) = target.closest("[data-product-badge]")
                {
                    s.open_popover(badge);
                } else {
                    s.close_popover();
                }
            }
        })?;
    }

    // Global Escape & Scroll dismissal
    listen(document, "keydown", {
        let state = state.clone();
        let media = reduced.clone();
        move |ev| {
            let Some(k) = ev.dyn_ref::<KeyboardEvent>() else {
                return;
            };
            if k.key() == "Escape" {
                let mut s = state.borrow_mut();
                if let Some(open_key) = s.open_menu.clone() {
                    ev.prevent_default();
                    s.set_menu(None, media.matches());
                    if let Some(cluster) = &s.cluster
                        && let Ok(Some(btn)) = cluster
                            .query_selector(&format!("[data-kense-menu-toggle=\"{open_key}\"]"))
                        && let Ok(el) = btn.dyn_into::<HtmlElement>()
                    {
                        let _ = el.focus();
                    }
                } else if s.active_badge.is_some() {
                    s.close_popover();
                }
            }
        }
    })?;

    listen(window, "scroll", {
        let state = state.clone();
        move |_| {
            let mut s = state.borrow_mut();
            if s.active_badge.is_some() {
                s.close_popover();
            }
        }
    })?;

    listen(document, "pointerdown", {
        let state = state.clone();
        let media = reduced.clone();
        move |ev| {
            let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                return;
            };
            let mut s = state.borrow_mut();
            if s.open_menu.is_some()
                && s.cluster
                    .as_ref()
                    .is_none_or(|c| !c.contains(Some(&target)))
            {
                s.set_menu(None, media.matches());
            }
            if s.active_badge.is_some()
                && s.popover
                    .as_ref()
                    .is_none_or(|p| !p.contains(Some(&target)))
                && target
                    .closest("[data-product-badge]")
                    .ok()
                    .flatten()
                    .is_none()
            {
                s.close_popover();
            }
        }
    })?;

    Ok(())
}
