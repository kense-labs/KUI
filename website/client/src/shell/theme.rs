//! Synchronizes document and GPU appearance, including interrupted theme gestures.

use super::surface::{Opening, Surface};
use super::*;
use js_sys::Promise;
use std::cell::{Cell, RefCell};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Animation, MessageEvent, MouseEvent, PointerEvent};

struct Wipe {
    layer: Element,
    animations: Vec<Animation>,
    to: bool,
    intent: bool,
}
struct Press {
    owner: Element,
    prior: bool,
    mode: String,
    hole: Opening,
    pointer: Option<i32>,
}
thread_local! {
    static WIPE:RefCell<Option<Wipe>>=const{RefCell::new(None)};
    static PRESS:RefCell<Option<Press>>=const{RefCell::new(None)};
    static VERSION:Cell<u32>=const{Cell::new(0)};
    static SYSTEM:Cell<bool>=const{Cell::new(true)};
}
fn name(dark: bool) -> &'static str {
    if dark { "dark" } else { "light" }
}
fn is_dark() -> bool {
    root().get_attribute("data-theme").as_deref() == Some("dark")
}
fn intent() -> bool {
    WIPE.with_borrow(|s| s.as_ref().map(|w| w.intent))
        .unwrap_or_else(is_dark)
}
fn version() -> u32 {
    VERSION.with(|v| {
        v.set(v.get().wrapping_add(1));
        v.get()
    })
}
fn fresh(v: u32) -> bool {
    VERSION.with(Cell::get) == v
}

fn show_icon(dark: bool) {
    attr(&root(), "data-kui-theme-intent", name(dark));
    spawn_local(async move {
        if intent() != dark {
            return;
        }
        for f in frames()
            .into_iter()
            .filter(|f| f.get_attribute("data-story").as_deref() == Some("theme-switch"))
        {
            preview(&f, "iconIntent", &Array::of1(&dark.into()));
        }
    });
}

async fn paint(dark: bool) {
    if is_dark() != dark {
        return;
    }
    let pending = Array::new();
    let motion = reduced();
    for frame in frames() {
        let wait = surface::visible(&frame);
        let result = preview(
            &frame,
            "preferences",
            &Array::of3(&dark.into(), &motion.into(), &wait.into()),
        );
        if wait && let Some(p) = result {
            pending.push(&p);
        }
    }
    if pending.length() > 0 {
        let mut timer = None;
        let timeout = Promise::new(&mut |resolve, _| {
            timer = window()
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 350)
                .ok();
        });
        let race = Array::of2(&Promise::all(&pending), &timeout);
        let _ = JsFuture::from(Promise::race(&race)).await;
        if let Some(t) = timer {
            window().clear_timeout_with_handle(t);
        }
    }
}
fn apply(dark: bool) {
    let root = root();
    attr(&root, "data-theme", name(dark));
    if let Some(m) = q(r#"meta[name="theme-color"]"#) {
        attr(&m, "content", if dark { "#000000" } else { "#ffffff" });
    }
}
fn sync(dark: bool) {
    apply(dark);
    // Update GPUI windows after returning from any originating GPUI callback.
    spawn_local(paint(dark));
}
fn release() {
    if let Some(wipe) = WIPE.with_borrow_mut(Option::take) {
        wipe.layer.remove();
        for a in wipe.animations {
            a.cancel();
        }
    }
    let r = root();
    remove(&r, "data-kui-theme-intent");
    remove(&r, "data-kui-theme-transition");
}
fn settle() {
    if WIPE.with_borrow(Option::is_some) {
        let dark = intent();
        version();
        release();
        sync(dark);
    }
}
fn watch(animation: &Animation, epoch: u32, instant: bool, reversing: bool, dark: bool) {
    let finished = if instant {
        None
    } else {
        animation.finished().ok()
    };
    spawn_local(async move {
        if let Some(p) = finished
            && JsFuture::from(p).await.is_err()
        {
            return;
        }
        if !fresh(epoch) {
            return;
        }
        if reversing {
            // The outgoing pixels now cover the viewport. Wait for the GPU to
            // restore that theme before dropping the frozen surface.
            apply(dark);
            paint(dark).await;
        }
        if fresh(epoch) {
            release();
        }
    });
}
fn move_to(dark: bool, mode: &str, hole: Option<Opening>) {
    if dark == intent() {
        return;
    }
    let epoch = version();
    let reversed = WIPE.with_borrow_mut(|s| {
        let w = s.as_mut()?;
        w.intent = dark;
        let reversing = dark != w.to;
        let instant = reversing && w.animations[0].current_time().unwrap_or(0.) <= 0.;
        for a in &w.animations {
            if instant {
                let _ = a.pause();
                a.set_current_time(Some(0.));
            } else {
                a.update_playback_rate(if reversing { -1. } else { 1. });
                let _ = a.play();
            }
        }
        Some((w.animations[0].clone(), reversing, instant))
    });
    if let Some((a, reversing, instant)) = reversed {
        show_icon(dark);
        if !reversing && is_dark() != dark {
            sync(dark);
        }
        watch(&a, epoch, instant, reversing, dark);
        return;
    }
    if reduced() || document().hidden() {
        sync(dark);
        return;
    }
    let Ok(frozen) = Surface::freeze(name(!dark), hole) else {
        sync(dark);
        return;
    };
    let Ok(animations) = frozen.reveal(mode) else {
        frozen.layer.remove();
        sync(dark);
        return;
    };
    let animation = animations[0].clone();
    WIPE.with_borrow_mut(|s| {
        *s = Some(Wipe {
            layer: frozen.layer,
            animations,
            to: dark,
            intent: dark,
        })
    });
    attr(&root(), "data-kui-theme-transition", "playing");
    sync(dark);
    show_icon(dark);
    watch(&animation, epoch, false, false, dark);
}
fn choose(dark: bool, mode: &str, hole: Option<Opening>) {
    SYSTEM.set(false);
    store("theme", name(dark));
    move_to(dark, mode, hole);
}
fn preference(p: &str, persist: bool) {
    let dark = match p {
        "light" => false,
        "dark" => true,
        "system" => media("(prefers-color-scheme: dark)"),
        _ => return,
    };
    SYSTEM.set(p == "system");
    if persist {
        store("theme", p);
    }
    move_to(dark, "reveal", None);
}
fn finish(owner: &Element, commit: bool) {
    let press = PRESS.with_borrow_mut(|s| {
        if s.as_ref().is_some_and(|p| p.owner == *owner) {
            s.take()
        } else {
            None
        }
    });
    if let Some(p) = press
        && !commit
    {
        choose(p.prior, &p.mode, Some(p.hole));
    }
}
fn begin(owner: Element, mode: String, hole: Opening, pointer: Option<i32>) {
    let prev = PRESS.with_borrow(|s| s.as_ref().map(|p| p.owner.clone()));
    if let Some(prev) = prev {
        finish(&prev, false);
    }
    let dark = intent();
    choose(!dark, &mode, Some(hole));
    PRESS.with_borrow_mut(|p| {
        *p = Some(Press {
            owner,
            prior: dark,
            mode,
            hole,
            pointer,
        })
    });
}
fn mode(value: &JsValue) -> Option<String> {
    value
        .as_string()
        .filter(|s| matches!(s.as_str(), "reveal" | "split" | "rise" | "eclipse"))
}
fn hole(frame: &HtmlIFrameElement, obj: &JsValue, w: &str, h: &str) -> Option<Opening> {
    let mut values = [0.; 4];
    for (i, key) in ["x", "y", w, h].iter().enumerate() {
        values[i] = prop(obj, key).as_f64().filter(|v| v.is_finite())?;
    }
    let [x, y, w, h] = values;
    Some(Opening::in_frame(frame, x, y, w, h))
}
fn prepare_pictures(wanted: bool) {
    if wanted && (reduced() || document().hidden()) {
        return;
    }
    for f in frames() {
        if !wanted || surface::visible(&f) {
            preview(&f, "preparePicture", &Array::of1(&wanted.into()));
        }
    }
}

fn gesture(ev: Event) -> Option<()> {
    // GPUI dispatches the event in the iframe's JS realm. Its CustomEvent
    // constructor differs from the parent window's even on the same origin.
    let detail = prop(ev.as_ref(), "detail");
    let source = prop(&detail, "frame");
    let frame = frames()
        .into_iter()
        .find(|f| f.as_ref() as &JsValue == &source)?;
    if frame.get_attribute("data-story").as_deref() != Some("theme-switch") {
        return None;
    }
    match prop(&detail, "phase").as_string()?.as_str() {
        "begin" => {
            let opening = hole(&frame, &prop(&detail, "bounds"), "width", "height")?;
            begin(frame.into(), mode(&prop(&detail, "mode"))?, opening, None);
        }
        "warm" => prepare_pictures(true),
        "cool" => prepare_pictures(false),
        "commit" => finish(&frame, true),
        "cancel" => finish(&frame, false),
        _ => {}
    };
    Some(())
}
fn message(ev: Event) -> Option<()> {
    let e = ev.dyn_ref::<MessageEvent>()?;
    if e.origin() != window().location().origin().ok()? {
        return None;
    }
    let source = e.source()?;
    let f = frames().into_iter().find(|f| {
        f.content_window()
            .is_some_and(|w| JsValue::from(w) == JsValue::from(source.clone()))
    })?;
    let data = e.data();
    let m = prop(&data, "mode");
    match (
        f.get_attribute("data-story")?.as_str(),
        prop(&data, "type").as_string()?.as_str(),
    ) {
        ("theme-switch", "kui:family-theme") => {
            prop(&data, "dark").as_bool()?;
            choose(!intent(), &mode(&m)?, Some(hole(&f, &data, "w", "h")?));
        }
        ("user-menu", "kui:account-appearance") => preference(&m.as_string()?, true),
        _ => {}
    };
    Some(())
}
pub(super) fn refresh() {
    PRESS.with_borrow_mut(Option::take);
    settle();
    sync(is_dark());
}
pub(super) fn init() -> Result<(), JsValue> {
    let win = window();
    let doc = document();
    SYSTEM.set(!matches!(
        stored("theme").as_deref(),
        Some("light" | "dark")
    ));
    if let Some(os) = win.match_media("(prefers-color-scheme: dark)")? {
        on(os.as_ref(), "change", false, |_| {
            if SYSTEM.get() {
                preference("system", false);
            }
        })?;
    }
    if let Some(m) = win.match_media("(prefers-reduced-motion: reduce)")? {
        on(m.as_ref(), "change", false, |_| {
            settle();
            sync(is_dark());
            accent::refresh();
        })?;
    }
    on(win.as_ref(), "message", false, |e| {
        message(e);
    })?;
    on(win.as_ref(), "kui:theme-gesture", false, |e| {
        gesture(e);
    })?;
    on(win.as_ref(), "storage", false, |e| {
        let k = prop(e.as_ref(), "key");
        if k.is_null() || k.as_string().as_deref() == Some("theme") {
            preference(stored("theme").as_deref().unwrap_or("system"), false);
        }
    })?;
    for (event, wanted) in [
        ("pointerenter", true),
        ("focus", true),
        ("pointerleave", false),
        ("blur", false),
    ] {
        on(doc.as_ref(), event, true, move |e| {
            if target(&e).is_some_and(|t| t.has_attribute("data-theme-toggle")) {
                prepare_pictures(wanted);
            }
        })?;
    }
    on(doc.as_ref(), "pointerdown", false, |e| {
        let Some(p) = e.dyn_ref::<PointerEvent>() else {
            return;
        };
        if !p.is_primary() || p.button() != 0 {
            return;
        }
        let Some(t) = target(&e).and_then(|t| closest(&t, "[data-theme-toggle]")) else {
            return;
        };
        let _ = t.set_pointer_capture(p.pointer_id());
        let opening = Opening::from_element(&t);
        begin(t, "reveal".into(), opening, Some(p.pointer_id()));
    })?;
    for name in ["pointerup", "lostpointercapture", "pointercancel"] {
        on(doc.as_ref(), name, false, move |e| {
            let Some(p) = e.dyn_ref::<PointerEvent>() else {
                return;
            };
            let owner = PRESS.with_borrow(|s| {
                s.as_ref()
                    .filter(|v| v.pointer == Some(p.pointer_id()))
                    .map(|v| v.owner.clone())
            });
            if let Some(t) = owner {
                let hit = document().element_from_point(p.client_x() as f32, p.client_y() as f32);
                finish(
                    &t,
                    name == "pointerup" && hit.as_ref().is_some_and(|h| t.contains(Some(h))),
                );
            }
        })?;
    }
    on(doc.as_ref(), "click", false, |e| {
        if let Some(m) = e.dyn_ref::<MouseEvent>()
            && m.detail() == 0
            && let Some(t) = target(&e).and_then(|t| closest(&t, "[data-theme-toggle]"))
        {
            choose(!intent(), "reveal", Some(Opening::from_element(&t)));
        }
    })?;
    on(win.as_ref(), "blur", false, |_| {
        let t = PRESS.with_borrow(|s| {
            s.as_ref()
                .filter(|v| v.pointer.is_some())
                .map(|v| v.owner.clone())
        });
        if let Some(t) = t {
            finish(&t, false);
        }
    })?;
    on(doc.as_ref(), "visibilitychange", false, |_| {
        if document().hidden() {
            let t = PRESS.with_borrow(|s| s.as_ref().map(|v| v.owner.clone()));
            if let Some(t) = t {
                finish(&t, false);
            }
            settle();
        }
    })?;
    on(win.as_ref(), "resize", false, |_| settle())?;
    on(win.as_ref(), "scroll", true, |e| {
        if !target(&e).is_some_and(|t| closest(&t, "[data-kui-surface]").is_some()) {
            settle();
        }
    })?;
    on(doc.as_ref(), "load", true, |e| {
        if let Some(f) = target(&e)
            .filter(is_frame)
            .map(JsCast::unchecked_into::<HtmlIFrameElement>)
            && f.class_list().contains("rust-story")
        {
            preview(
                &f,
                "preferences",
                &Array::of3(&is_dark().into(), &reduced().into(), &false.into()),
            );
            accent::sync_frame(&f);
        }
    })?;
    Ok(())
}
