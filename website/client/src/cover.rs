//! Browser interactions for the KUI cover.
pub(crate) mod flight;
mod guide;
mod intro;
mod motion;

use js_sys::{Array, Function, Reflect};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{
    Document, Element, Event, EventTarget, HtmlElement, HtmlIFrameElement, IntersectionObserver,
    IntersectionObserverEntry, ResizeObserver, Window,
};

pub(crate) type Result<T = ()> = std::result::Result<T, JsValue>;

pub(crate) fn element<T: JsCast>(root: &Element, selector: &str) -> Result<T> {
    root.query_selector(selector)?
        .ok_or_else(|| JsValue::from_str(selector))?
        .dyn_into()
        .map_err(Into::into)
}

pub(crate) fn flag(el: &Element, attr: &str, enabled: bool) {
    if el.has_attribute(attr) != enabled {
        let _ = el.toggle_attribute(attr);
    }
}

pub(crate) fn css(el: &HtmlElement, name: &str, value: &str) {
    let _ = el.style().set_property(name, value);
}

pub(crate) fn listen(
    target: &EventTarget,
    event: &str,
    handler: impl FnMut(Event) + 'static,
) -> Result {
    let handler = Closure::<dyn FnMut(Event)>::new(handler);
    target.add_event_listener_with_callback(event, handler.as_ref().unchecked_ref())?;
    handler.forget();
    Ok(())
}

pub(crate) fn viewport(window: &Window) -> (f64, f64) {
    (
        window
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or_default(),
        window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or_default(),
    )
}

/// At most one frame is queued. Idle effects wake through their event handlers.
pub(crate) struct Frame {
    window: Window,
    pending: Cell<bool>,
    callback: Closure<dyn FnMut(f64)>,
}

impl Frame {
    pub(crate) fn new(window: &Window, mut tick: impl FnMut(f64) -> bool + 'static) -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| {
            let weak = weak.clone();
            let callback = Closure::new(move |now: f64| {
                if let Some(frame) = weak.upgrade() {
                    frame.pending.set(false);
                    if tick(now) {
                        frame.request();
                    }
                }
            });
            Self {
                window: window.clone(),
                pending: Cell::new(false),
                callback,
            }
        })
    }
    pub(crate) fn request(&self) {
        if !self.pending.replace(true)
            && self
                .window
                .request_animation_frame(self.callback.as_ref().unchecked_ref())
                .is_err()
        {
            self.pending.set(false);
        }
    }
}

fn sync_cover_tab_indicator(root: &Element) {
    let Ok(Some(bar_node)) = root.query_selector("[data-cover-tabs]") else {
        return;
    };
    let Ok(bar) = bar_node.dyn_into::<HtmlElement>() else {
        return;
    };
    let Ok(Some(active_node)) = bar.query_selector(".live-tab[aria-pressed='true']") else {
        return;
    };
    let Ok(active) = active_node.dyn_into::<HtmlElement>() else {
        return;
    };
    let br = bar.get_bounding_client_rect();
    let ar = active.get_bounding_client_rect();
    if br.width() > 0.0 && ar.width() > 0.0 {
        let x = ar.left() - br.left();
        let y = ar.top() - br.top();
        css(&bar, "--cover-tab-x", &format!("{x:.1}px"));
        css(&bar, "--cover-tab-y", &format!("{y:.1}px"));
        css(&bar, "--cover-tab-w", &format!("{:.1}px", ar.width()));
        css(&bar, "--cover-tab-h", &format!("{:.1}px", ar.height()));
        flag(&bar, "data-indicator-ready", true);
    }
}

fn ensure_cover_frame_booted(iframe: &HtmlIFrameElement) {
    if iframe.has_attribute("src") {
        return;
    }
    if let Some(deferred) = iframe.get_attribute("data-src")
        && !deferred.is_empty()
    {
        let _ = iframe.set_attribute("src", &deferred);
    }
}

/// Registers cover effects on the localized landing pages.
pub fn init() -> Result {
    let window = web_sys::window().ok_or("window")?;
    let document: Document = window.document().ok_or("document")?;
    let Some(root) = document.query_selector(".kui-cover-page")? else {
        return Ok(());
    };
    if root.has_attribute("data-initialized") {
        return Ok(());
    }
    root.set_attribute("data-initialized", "")?;

    let reduced = window
        .match_media("(prefers-reduced-motion: reduce)")?
        .ok_or("matchMedia")?;
    let intro_playing = intro::init(&window, &document, &root, &reduced)?;
    guide::init(&window, &document, &root, &reduced)?;
    sync_cover_tab_indicator(&root);

    if let Ok(Some(bar)) = root.query_selector("[data-cover-tabs]") {
        let root_ro = root.clone();
        let ro_cb = Closure::<dyn FnMut()>::new(move || {
            sync_cover_tab_indicator(&root_ro);
        });
        if let Ok(ro) = ResizeObserver::new(ro_cb.as_ref().unchecked_ref()) {
            ro.observe(&bar);
            std::mem::forget(ro);
            ro_cb.forget();
        }
    }

    // Boot the hero light-ribbon motion only after the intro curtain completes so entrance never contends.
    let boot_motion = {
        let booted = Rc::new(Cell::new(false));
        let win = window.clone();
        let doc = document.clone();
        let root = root.clone();
        let reduced = reduced.clone();
        move || {
            if booted.replace(true) {
                return;
            }
            let _ = motion::init(&win, &doc, &root, &reduced);
        }
    };

    if intro_playing {
        let bm = boot_motion.clone();
        listen(&document, "kui:intro-done", move |_| bm())?;
    } else {
        boot_motion();
    }

    // Defer 56 MB GPUI WASM iframe boot until intro finishes AND `.live-showcase` nears the viewport.
    if let Some(shell) = root
        .query_selector(".live-showcase")
        .ok()
        .flatten()
        .or_else(|| {
            root.query_selector("[data-cover-live-shell]")
                .ok()
                .flatten()
        })
        && let Ok(Some(frame_el)) = root.query_selector("[data-cover-live-frame]")
        && let Ok(iframe) = frame_el.dyn_into::<HtmlIFrameElement>()
    {
        let near_view = Rc::new(Cell::new(false));
        let intro_done = Rc::new(Cell::new(!intro_playing));

        let maybe_boot = {
            let near_view = near_view.clone();
            let intro_done = intro_done.clone();
            let iframe = iframe.clone();
            move || {
                if near_view.get() && intro_done.get() {
                    ensure_cover_frame_booted(&iframe);
                }
            }
        };

        let io_cb = Closure::<dyn FnMut(Array, IntersectionObserver)>::new({
            let near_view = near_view.clone();
            let maybe_boot = maybe_boot.clone();
            let iframe_check = iframe.clone();
            move |entries: Array, observer: IntersectionObserver| {
                for item in entries.iter() {
                    if let Ok(entry) = item.dyn_into::<IntersectionObserverEntry>()
                        && entry.is_intersecting()
                    {
                        near_view.set(true);
                        maybe_boot();
                        if iframe_check.has_attribute("src") {
                            observer.disconnect();
                        }
                    }
                }
            }
        });
        if let Ok(io) = IntersectionObserver::new(io_cb.as_ref().unchecked_ref()) {
            io.observe(&shell);
            std::mem::forget(io);
            io_cb.forget();
        }

        let check_bounds = {
            let win = window.clone();
            let shell = shell.clone();
            let near_view = near_view.clone();
            let maybe_boot = maybe_boot.clone();
            let iframe_check = iframe.clone();
            move || {
                if iframe_check.has_attribute("src") {
                    return;
                }
                let vh = win
                    .inner_height()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(900.0);
                let r = shell.get_bounding_client_rect();
                if r.top() < vh + 360.0 && r.bottom() > -360.0 {
                    near_view.set(true);
                    maybe_boot();
                }
            }
        };
        check_bounds();
        listen(&window, "scroll", {
            let check_bounds = check_bounds.clone();
            move |_| check_bounds()
        })?;

        if intro_playing {
            let intro_done = intro_done.clone();
            let check_bounds = check_bounds.clone();
            let maybe_boot = maybe_boot.clone();
            listen(&document, "kui:intro-done", move |_| {
                intro_done.set(true);
                check_bounds();
                maybe_boot();
            })?;
        }
    }

    {
        let root_clone = root.clone();
        listen(&root, "click", move |event: Event| {
            let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
                return;
            };
            let Ok(Some(btn)) = target.closest("[data-cover-story]") else {
                return;
            };
            let Some(story) = btn.get_attribute("data-cover-story") else {
                return;
            };
            if let Ok(tabs) = root_clone.query_selector_all("[data-cover-story]") {
                for i in 0..tabs.length() {
                    if let Some(el) = tabs.get(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                        let _ = el.set_attribute(
                            "aria-pressed",
                            if el == btn { "true" } else { "false" },
                        );
                    }
                }
            }
            if let Ok(Some(bar)) = root_clone.query_selector("[data-cover-tabs]") {
                flag(&bar, "data-indicator-animate", true);
            }
            sync_cover_tab_indicator(&root_clone);
            if let Some(href) = btn.get_attribute("data-cover-href")
                && let Ok(links) = root_clone.query_selector_all("[data-cover-open-link]")
            {
                for i in 0..links.length() {
                    if let Some(el) = links.get(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                        let _ = el.set_attribute("href", &href);
                    }
                }
            }
            if let Ok(Some(frame)) = root_clone.query_selector("[data-cover-live-frame]")
                && let Ok(iframe) = frame.dyn_into::<HtmlIFrameElement>()
            {
                let lang = iframe
                    .get_attribute("data-lang")
                    .unwrap_or_else(|| "zh".into());
                let _ = iframe.set_attribute("data-story", &story);
                let _ = iframe.set_attribute("data-story-key", &story);
                let mut switched = false;
                if let Some(win) = iframe.content_window()
                    && let Ok(rt) = Reflect::get(&win, &"kuiPreview".into())
                    && !rt.is_undefined()
                    && !rt.is_null()
                    && let Ok(show) = Reflect::get(&rt, &"show".into())
                    && let Ok(func) = show.dyn_into::<Function>()
                {
                    switched = func.call2(&rt, &story.into(), &lang.into()).is_ok();
                }
                if !switched
                    && !iframe.has_attribute("src")
                    && let Some(src) = btn.get_attribute("data-cover-src")
                {
                    let _ = iframe.set_attribute("data-src", &src);
                }
                ensure_cover_frame_booted(&iframe);
            }
        })?;
    }
    Ok(())
}
