//! Live GPUI workbench tab switching, focus escape, scroll-reveal choreography, and copy springs.

use crate::cover::{Frame, Result, css, flag, listen};
use js_sys::{Array, Function, Reflect};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
    Document, Element, HtmlElement, HtmlIFrameElement, IntersectionObserver,
    IntersectionObserverEntry, KeyboardEvent, MediaQueryList, MessageEvent, PointerEvent,
    ResizeObserver, Url, Window,
};

fn switch_gpui_story(window: &Window, frame: &HtmlIFrameElement, story: &str) {
    let lang = frame
        .get_attribute("data-lang")
        .unwrap_or_else(|| "zh".into());
    let _ = frame.set_attribute("data-story", story);
    let _ = frame.set_attribute("data-story-key", story);

    if let Some(cw) = frame.content_window()
        && let Ok(runtime) = Reflect::get(&cw, &"kuiPreview".into())
        && !runtime.is_undefined()
        && !runtime.is_null()
        && let Ok(show) = Reflect::get(&runtime, &"show".into())
        && let Ok(func) = show.dyn_into::<Function>()
        && func
            .call2(&runtime, &story.into(), &lang.clone().into())
            .is_ok()
    {
        return;
    }

    // If the lazy GPUI iframe has not booted `kuiPreview` yet, update its initial query string.
    if let Some(raw_src) = frame.get_attribute("src")
        && let Ok(base) = window.location().href()
        && let Ok(url) = Url::new_with_base(&raw_src, &base)
    {
        let next = format!("{}?story={story}&lang={lang}", url.pathname());
        if raw_src != next {
            let _ = frame.set_attribute("src", &next);
        }
    }
}

fn sync_tab_indicator(root: &Element) {
    let Ok(Some(bar_node)) = root.query_selector("[data-kense-tabs]") else {
        return;
    };
    let Ok(bar) = bar_node.dyn_into::<HtmlElement>() else {
        return;
    };
    let Ok(Some(active_node)) = bar.query_selector(".kense-tab.is-active") else {
        return;
    };
    let Ok(active) = active_node.dyn_into::<HtmlElement>() else {
        return;
    };
    let br = bar.get_bounding_client_rect();
    let ar = active.get_bounding_client_rect();
    if br.width() > 0.0 && ar.width() > 0.0 {
        let x = ar.left() - br.left();
        css(&bar, "--tab-x", &format!("{x:.1}px"));
        css(&bar, "--tab-w", &format!("{:.1}px", ar.width()));
        flag(&bar, "data-indicator-ready", true);
    }
}

fn activate_tab(window: &Window, root: &Element, key: &str, focus_tab: bool) {
    let Ok(tabs) = root.query_selector_all("[data-kense-tab]") else {
        return;
    };
    let mut story = None;
    for i in 0..tabs.length() {
        let Some(tab) = tabs.item(i).and_then(|n| n.dyn_into::<HtmlElement>().ok()) else {
            continue;
        };
        let active = tab.get_attribute("data-kense-tab").as_deref() == Some(key);
        let _ = tab.class_list().toggle_with_force("is-active", active);
        let _ = tab.set_attribute("aria-selected", if active { "true" } else { "false" });
        let _ = tab.set_attribute("tabindex", if active { "0" } else { "-1" });
        if active {
            story = tab.get_attribute("data-gpui-story");
            if focus_tab {
                let _ = tab.focus();
            }
        }
    }
    sync_tab_indicator(root);

    if let Ok(panels) = root.query_selector_all("[data-kense-panel]") {
        for i in 0..panels.length() {
            if let Some(panel) = panels
                .item(i)
                .and_then(|n| n.dyn_into::<HtmlElement>().ok())
            {
                let active = panel.get_attribute("data-kense-panel").as_deref() == Some(key);
                panel.set_hidden(!active);
                let _ = panel.class_list().toggle_with_force("is-active", active);
            }
        }
    }

    if let Some(story_id) = story
        && let Ok(Some(el)) = root.query_selector("[data-kense-gpui-frame]")
        && el.namespace_uri().as_deref() == Some("http://www.w3.org/1999/xhtml")
    {
        let frame: HtmlIFrameElement = el.unchecked_into();
        switch_gpui_story(window, &frame, &story_id);
    }
}

fn sync_hash_tab(window: &Window, root: &Element) {
    let hash = window.location().hash().unwrap_or_default();
    let tab = match hash.as_str() {
        "#experience-gateway" => Some("gateway"),
        "#experience-sandbox" => Some("sandbox"),
        "#experience-interface" => Some("interface"),
        "#experience-klang" => Some("klang"),
        _ => None,
    };
    if let Some(key) = tab {
        activate_tab(window, root, key, false);
    }
}

fn focusable_stops(scope: &Element) -> Vec<HtmlElement> {
    let Ok(list) = scope.query_selector_all(
        "a[href],button:not(:disabled),input:not(:disabled),select:not(:disabled),summary,iframe,[tabindex='0']",
    ) else {
        return Vec::new();
    };
    (0..list.length())
        .filter_map(|i| list.item(i)?.dyn_into::<HtmlElement>().ok())
        .filter(|el| {
            el.tab_index() >= 0
                && el.closest("[inert],[hidden]").ok().flatten().is_none()
                && el.get_bounding_client_rect().height() > 0.0
        })
        .collect()
}

fn bind_scroll_reveal(window: &Window, root: &Element, reduced: &MediaQueryList) -> Result {
    let reveals = root.query_selector_all(".kense-reveal")?;
    if reduced.matches() {
        for i in 0..reveals.length() {
            if let Some(el) = reveals.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                let _ = el.class_list().add_1("is-visible");
            }
        }
        return Ok(());
    }

    // Stagger sibling cards in grids while immediately revealing elements already in view.
    let vh = window
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);
    flag(root, "data-reveal-ready", true);

    let cb = Closure::<dyn FnMut(Array, IntersectionObserver)>::new(
        move |entries: Array, observer: IntersectionObserver| {
            for item in entries.iter() {
                if let Ok(entry) = item.dyn_into::<IntersectionObserverEntry>()
                    && entry.is_intersecting()
                {
                    let target = entry.target();
                    let _ = target.class_list().add_1("is-visible");
                    observer.unobserve(&target);
                }
            }
        },
    );
    let observer = IntersectionObserver::new(cb.as_ref().unchecked_ref())?;
    for i in 0..reveals.length() {
        let Some(el) = reveals
            .item(i)
            .and_then(|n| n.dyn_into::<HtmlElement>().ok())
        else {
            continue;
        };
        if let Some(parent) = el.parent_element()
            && parent
                .matches(".kense-bento,.kense-future-grid,.kense-access-routes")
                .unwrap_or(false)
        {
            let mut idx = 0_u32;
            let mut prev = el.previous_element_sibling();
            while let Some(p) = prev {
                idx += 1;
                prev = p.previous_element_sibling();
            }
            css(&el, "--reveal-delay", &format!("{}ms", (idx % 4) * 65));
        }
        if el.get_bounding_client_rect().top() < vh * 0.94 {
            let _ = el.class_list().add_1("is-visible");
        } else {
            observer.observe(&el);
        }
    }
    cb.forget();

    // Gate continuous SVG relief & topology animations to viewport intersection.
    let flows = root.query_selector_all("[data-kense-flow],[data-kense-bento]")?;
    let flow_cb = Closure::<dyn FnMut(Array)>::new({
        let media = reduced.clone();
        move |entries: Array| {
            let allow = !media.matches();
            for item in entries.iter() {
                if let Ok(entry) = item.dyn_into::<IntersectionObserverEntry>() {
                    flag(
                        &entry.target(),
                        "data-flow-running",
                        allow && entry.is_intersecting(),
                    );
                }
            }
        }
    });
    let flow_io = IntersectionObserver::new(flow_cb.as_ref().unchecked_ref())?;
    for i in 0..flows.length() {
        if let Some(el) = flows.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
            flow_io.observe(&el);
        }
    }
    flow_cb.forget();
    Ok(())
}

fn bind_copy_springs(window: &Window, root: &Element, reduced: &MediaQueryList) -> Result {
    struct Impulse {
        el: HtmlElement,
        y: f64,
        v: f64,
    }
    let active: Rc<RefCell<(Vec<Impulse>, f64)>> = Rc::new(RefCell::new((Vec::new(), 0.0)));
    let frame = Frame::new(window, {
        let active = active.clone();
        let media = reduced.clone();
        move |now| {
            let mut guard = active.borrow_mut();
            if media.matches() {
                for item in guard.0.drain(..) {
                    let _ = item.el.style().remove_property("transform");
                }
                guard.1 = 0.0;
                return false;
            }
            let dt = if guard.1 == 0.0 {
                0.016
            } else {
                ((now - guard.1) / 1000.0).clamp(0.001, 0.04)
            };
            guard.1 = now;
            let k = 420.0_f64;
            let c = 2.0 * k.sqrt();
            guard.0.retain_mut(|imp| {
                let acc = -k * imp.y - c * imp.v;
                imp.v += acc * dt;
                imp.y += imp.v * dt;
                if imp.y.abs() < 0.08 && imp.v.abs() < 1.2 {
                    let _ = imp.el.style().remove_property("transform");
                    false
                } else {
                    css(
                        &imp.el,
                        "transform",
                        &format!("translate3d(0,{:.2}px,0)", imp.y),
                    );
                    true
                }
            });
            if guard.0.is_empty() {
                guard.1 = 0.0;
                false
            } else {
                true
            }
        }
    });

    listen(root, "click", {
        let active = active.clone();
        let frame = frame.clone();
        let media = reduced.clone();
        move |ev| {
            if media.matches() {
                return;
            }
            let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                return;
            };
            let Ok(Some(btn)) = target.closest("[data-spring-copy]") else {
                return;
            };
            let Ok(el) = btn.dyn_into::<HtmlElement>() else {
                return;
            };
            let mut guard = active.borrow_mut();
            if let Some(existing) = guard.0.iter_mut().find(|i| i.el == el) {
                existing.v += 185.0;
            } else {
                guard.0.push(Impulse {
                    el,
                    y: 0.0,
                    v: 215.0,
                });
            }
            frame.request();
        }
    })
}

pub(super) fn init(
    window: &Window,
    document: &Document,
    root: &Element,
    reduced: &MediaQueryList,
) -> Result {
    bind_scroll_reveal(window, root, reduced)?;
    bind_copy_springs(window, root, reduced)?;

    // Subtle pointer parallax on the Hero architectural studio grid & light field
    if let Ok(Some(hero)) = root.query_selector("[data-kense-hero]")
        && let Ok(Some(optics_node)) = hero.query_selector("[data-kense-optics]")
        && let Ok(optics) = optics_node.dyn_into::<HtmlElement>()
    {
        listen(&hero, "pointermove", {
            let hero = hero.clone();
            let optics = optics.clone();
            let media = reduced.clone();
            move |ev| {
                let Some(pe) = ev.dyn_ref::<PointerEvent>() else {
                    return;
                };
                if pe.pointer_type() == "touch" || media.matches() {
                    return;
                }
                let r = hero.get_bounding_client_rect();
                if r.width() > 0.0 && r.height() > 0.0 {
                    let px = 20.0 * ((f64::from(pe.client_x()) - r.left()) / r.width() - 0.5);
                    let py = 14.0 * ((f64::from(pe.client_y()) - r.top()) / r.height() - 0.5);
                    css(&optics, "--hero-px", &format!("{px:.1}px"));
                    css(&optics, "--hero-py", &format!("{py:.1}px"));
                }
            }
        })?;
        listen(&hero, "pointerleave", move |_| {
            css(&optics, "--hero-px", "0px");
            css(&optics, "--hero-py", "0px");
        })?;
    }

    // Click & keyboard navigation on `#experience` capsule tabs
    listen(root, "click", {
        let win = window.clone();
        let root = root.clone();
        move |ev| {
            let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                return;
            };
            if let Ok(Some(tab)) = target.closest("[data-kense-tab]")
                && let Some(key) = tab.get_attribute("data-kense-tab")
            {
                activate_tab(&win, &root, &key, false);
            }
        }
    })?;

    listen(root, "keydown", {
        let win = window.clone();
        let root = root.clone();
        move |ev| {
            let Some(k) = ev.dyn_ref::<KeyboardEvent>() else {
                return;
            };
            if k.default_prevented() || k.is_composing() {
                return;
            }
            let Some(target) = ev.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                return;
            };
            if !target.has_attribute("data-kense-tab") {
                return;
            }
            let order = ["gateway", "sandbox", "interface", "klang"];
            let cur = target
                .get_attribute("data-kense-tab")
                .and_then(|v| order.iter().position(|&k| k == v))
                .unwrap_or(0);
            let next = match k.key().as_str() {
                "ArrowRight" => (cur + 1) % order.len(),
                "ArrowLeft" => (cur + order.len() - 1) % order.len(),
                "Home" => 0,
                "End" => order.len() - 1,
                _ => return,
            };
            ev.prevent_default();
            activate_tab(&win, &root, order[next], true);
        }
    })?;

    // Ensure lazy GPUI iframe syncs to active tab once booted
    if let Ok(Some(el)) = root.query_selector("[data-kense-gpui-frame]")
        && el.namespace_uri().as_deref() == Some("http://www.w3.org/1999/xhtml")
    {
        let frame: HtmlIFrameElement = el.unchecked_into();
        listen(&frame, "load", {
            let win = window.clone();
            let frame = frame.clone();
            move |_| {
                if let Some(story) = frame.get_attribute("data-story") {
                    switch_gpui_story(&win, &frame, &story);
                }
            }
        })?;
    }

    // Handle Tab / Shift+Tab focus escape from the live GPUI workbench iframe on `/`
    listen(window, "message", {
        let win = window.clone();
        let doc = document.clone();
        move |ev| {
            let Some(msg) = ev.dyn_ref::<MessageEvent>() else {
                return;
            };
            let Ok(origin) = win.location().origin() else {
                return;
            };
            if msg.origin() != origin {
                return;
            }
            let data = msg.data();
            let Some(kind) = Reflect::get(&data, &"type".into())
                .ok()
                .and_then(|v| v.as_string())
            else {
                return;
            };
            if kind != "kui:story-leave" {
                return;
            }
            let Some(back) = Reflect::get(&data, &"back".into())
                .ok()
                .and_then(|v| v.as_bool())
            else {
                return;
            };
            let Some(source) = msg.source() else { return };
            let Some(scope) = doc.document_element() else {
                return;
            };
            let stops = focusable_stops(&scope);
            let Some(idx) = stops.iter().position(|el| {
                el.dyn_ref::<HtmlIFrameElement>()
                    .filter(|f| f.has_attribute("data-kense-gpui-frame"))
                    .and_then(|f| f.content_window())
                    .is_some_and(|cw| js_sys::Object::is(cw.as_ref(), source.as_ref()))
            }) else {
                return;
            };
            let next = if back {
                let Some(prev) = idx.checked_sub(1) else {
                    return;
                };
                prev
            } else {
                idx + 1
            };
            if let Some(el) = stops.get(next) {
                let _ = el.focus();
            }
        }
    })?;

    listen(window, "hashchange", {
        let win = window.clone();
        let root = root.clone();
        move |_| sync_hash_tab(&win, &root)
    })?;
    sync_hash_tab(window, root);
    sync_tab_indicator(root);

    if let Ok(Some(bar)) = root.query_selector("[data-kense-tabs]") {
        let root_clone = root.clone();
        let ro_cb = Closure::<dyn FnMut()>::new(move || {
            sync_tab_indicator(&root_clone);
        });
        let ro = ResizeObserver::new(ro_cb.as_ref().unchecked_ref())?;
        ro.observe(&bar);
        ro_cb.forget();
    }

    Ok(())
}
