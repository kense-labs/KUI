//! Browser interactions for the KUI cover.
mod flight;
mod guide;
mod motion;

use std::{
    cell::Cell,
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Document, Element, Event, EventTarget, HtmlElement, Window};

type Result<T = ()> = std::result::Result<T, JsValue>;

fn element<T: JsCast>(root: &Element, selector: &str) -> Result<T> {
    root.query_selector(selector)?
        .ok_or_else(|| JsValue::from_str(selector))?
        .dyn_into()
        .map_err(Into::into)
}

fn flag(el: &Element, attr: &str, enabled: bool) {
    if el.has_attribute(attr) != enabled {
        let _ = el.toggle_attribute(attr);
    }
}

fn css(el: &HtmlElement, name: &str, value: &str) {
    let _ = el.style().set_property(name, value);
}

fn listen(target: &EventTarget, event: &str, handler: impl FnMut(Event) + 'static) -> Result {
    let handler = Closure::<dyn FnMut(Event)>::new(handler);
    target.add_event_listener_with_callback(event, handler.as_ref().unchecked_ref())?;
    handler.forget(); // The static document owns the listener until navigation.
    Ok(())
}

fn viewport(window: &Window) -> (f64, f64) {
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
struct Frame {
    window: Window,
    pending: Cell<bool>,
    callback: Closure<dyn FnMut(f64)>,
}

impl Frame {
    fn new(window: &Window, mut tick: impl FnMut(f64) -> bool + 'static) -> Rc<Self> {
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
    fn request(&self) {
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
    let reduced = window
        .match_media("(prefers-reduced-motion: reduce)")?
        .ok_or("matchMedia")?;
    motion::init(&window, &document, &root, &reduced)?;
    guide::init(&window, &document, &root, &reduced)?;
    root.set_attribute("data-initialized", "")
}
