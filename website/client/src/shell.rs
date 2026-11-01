//! Browser interactions for the bilingual site shell.

use crate::document;
use js_sys::{Array, Function, Reflect};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{Element, Event, EventTarget, HtmlElement, HtmlIFrameElement, KeyboardEvent, Window};

mod accent;
mod surface;
mod theme;

fn window() -> Window {
    web_sys::window().expect("browser window")
}
fn root() -> Element {
    document().document_element().expect("document element")
}
fn q(selector: &str) -> Option<Element> {
    document().query_selector(selector).ok().flatten()
}
fn all(selector: &str) -> Vec<Element> {
    let Ok(nodes) = document().query_selector_all(selector) else {
        return Vec::new();
    };
    (0..nodes.length())
        .filter_map(|i| nodes.get(i).map(JsCast::unchecked_into))
        .collect()
}
fn closest(el: &Element, selector: &str) -> Option<Element> {
    el.closest(selector).ok().flatten()
}
fn target(event: &Event) -> Option<Element> {
    let target = event.target()?;
    (prop(target.as_ref(), "nodeType").as_f64() == Some(1.)).then(|| target.unchecked_into())
}
fn attr(el: &Element, key: &str, value: &str) {
    let _ = el.set_attribute(key, value);
}
fn remove(el: &Element, key: &str) {
    let _ = el.remove_attribute(key);
}
fn hide(el: &Element, hidden: bool) {
    if hidden {
        attr(el, "hidden", "")
    } else {
        remove(el, "hidden")
    }
}
fn focus(el: &Element) {
    if let Some(e) = el.dyn_ref::<HtmlElement>() {
        let _ = e.focus();
    }
}
fn css(el: &Element, key: &str, value: &str) {
    if let Some(e) = el.dyn_ref::<HtmlElement>() {
        let _ = e.style().set_property(key, value);
    }
}
fn stored(key: &str) -> Option<String> {
    window().local_storage().ok()??.get_item(key).ok().flatten()
}
fn store(key: &str, val: &str) {
    if let Ok(Some(s)) = window().local_storage() {
        let _ = s.set_item(key, val);
    }
}
fn media(query: &str) -> bool {
    window()
        .match_media(query)
        .ok()
        .flatten()
        .is_some_and(|m| m.matches())
}
fn reduced() -> bool {
    media("(prefers-reduced-motion: reduce)")
}
fn on(
    target: &EventTarget,
    name: &str,
    capture: bool,
    handler: impl FnMut(Event) + 'static,
) -> Result<(), JsValue> {
    let closure = Closure::wrap(Box::new(handler) as Box<dyn FnMut(Event)>);
    target.add_event_listener_with_callback_and_bool(
        name,
        closure.as_ref().unchecked_ref(),
        capture,
    )?;
    closure.forget();
    Ok(())
}
fn is_frame(el: &Element) -> bool {
    el.local_name() == "iframe"
        && el.namespace_uri().as_deref() == Some("http://www.w3.org/1999/xhtml")
}
fn frames() -> Vec<HtmlIFrameElement> {
    // WebKit can wrap the live frame with a different realm's prototype.
    all("iframe.rust-story[src]")
        .into_iter()
        .filter(is_frame)
        .map(JsCast::unchecked_into)
        .collect()
}
fn prop(object: &JsValue, key: &str) -> JsValue {
    Reflect::get(object, &key.into()).unwrap_or(JsValue::UNDEFINED)
}
fn preview(frame: &HtmlIFrameElement, method: &str, args: &Array) -> Option<JsValue> {
    let runtime = prop(frame.content_window()?.as_ref(), "kuiPreview");
    prop(&runtime, method)
        .dyn_into::<Function>()
        .ok()?
        .apply(&runtime, args)
        .ok()
}

fn update_links() {
    let Some(brand) = q(".topbar .brand") else {
        return;
    };
    let home = brand.get_attribute("href").unwrap_or_default();
    let en = root().get_attribute("lang").as_deref() == Some("en");
    let suffix = match (en, q(".topbar.product").is_some()) {
        (true, true) => "/en/kui/",
        (false, true) => "/kui/",
        (true, false) => "/en/",
        (false, false) => "/",
    };
    let base = home.strip_suffix(suffix).unwrap_or("");
    let loc = window().location();
    let pathname = loc.pathname().unwrap_or_default();
    let local = pathname.strip_prefix(base).unwrap_or(&pathname);
    let route = if en {
        local.strip_prefix("/en").unwrap_or(local)
    } else {
        local
    };
    let route = if route.is_empty() { "/" } else { route };
    for link in all("[data-locale]") {
        let lang = if link.get_attribute("data-locale").as_deref() == Some("en") {
            "/en"
        } else {
            ""
        };
        attr(
            &link,
            "href",
            &format!(
                "{base}{lang}{route}{}{}",
                loc.search().unwrap_or_default(),
                loc.hash().unwrap_or_default()
            ),
        );
    }
    for a in all("[data-nav]") {
        let current = a
            .get_attribute("data-nav")
            .is_some_and(|n| route.trim_end_matches('/').split('/').nth(2) == Some(&n));
        if current {
            attr(&a, "aria-current", "page")
        } else {
            remove(&a, "aria-current")
        }
    }
}

fn back_top() {
    if let Some(b) = q("[data-back-top]") {
        let show = window().scroll_y().unwrap_or_default() > 420.;
        let _ = b.class_list().toggle_with_force("is-visible", show);
        attr(&b, "aria-hidden", if show { "false" } else { "true" });
        attr(&b, "tabindex", if show { "0" } else { "-1" });
    }
}

fn menu_open(selector: &str) -> bool {
    q(selector).is_some_and(|t| t.get_attribute("aria-expanded").as_deref() == Some("true"))
}
fn dismiss(restore: bool) {
    for (trigger, menu) in [
        ("[data-accent-trigger]", "[data-accent-menu]"),
        ("[data-lang-trigger]", "[data-lang-menu]"),
    ] {
        if !menu_open(trigger) {
            continue;
        }
        if let Some(t) = q(trigger) {
            attr(&t, "aria-expanded", "false");
            if restore {
                focus(&t);
            }
        }
        if menu == "[data-accent-menu]" {
            if let Some(m) = q(menu) {
                attr(&m, "inert", "");
            }
            accent::close_editor();
        } else if let Some(m) = q(menu) {
            hide(&m, true);
        }
    }
}
fn open_menu(accent: bool) {
    dismiss(false);
    let (trigger, menu, selected) = if accent {
        (
            "[data-accent-trigger]",
            "[data-accent-menu]",
            "[data-accent-value]",
        )
    } else {
        ("[data-lang-trigger]", "[data-lang-menu]", "[data-locale]")
    };
    if let (Some(t), Some(m)) = (q(trigger), q(menu)) {
        attr(&t, "aria-expanded", "true");
        remove(&m, "inert");
        hide(&m, false);
        if let Some(item) =
            q(&format!(r#"{selected}[aria-checked="true"]"#)).or_else(|| q(selected))
        {
            focus(&item);
        }
    }
}

fn menu_key(e: &KeyboardEvent, el: &Element) {
    let key = e.key();
    if let Some(t) = closest(el, "[data-lang-trigger],[data-accent-trigger]")
        && matches!(key.as_str(), "ArrowDown" | "ArrowUp")
    {
        e.prevent_default();
        open_menu(t.has_attribute("data-accent-trigger"));
        return;
    }
    if !menu_open("[data-accent-trigger]") && !menu_open("[data-lang-trigger]") {
        return;
    }
    if key == "Escape" {
        e.prevent_default();
        dismiss(true);
        return;
    }
    let Some(menu) = closest(el, "[data-lang-menu],[data-accent-menu]") else {
        return;
    };
    if key == "Tab" {
        dismiss(true);
        return;
    }
    let items = all(if menu.has_attribute("data-lang-menu") {
        "[data-locale]"
    } else {
        "[data-accent-value]"
    });
    if items.is_empty() {
        return;
    }
    let i = items.iter().position(|item| item == el).unwrap_or(0);
    let next = match key.as_str() {
        "Home" => 0,
        "End" => items.len() - 1,
        "ArrowDown" => (i + 1) % items.len(),
        "ArrowUp" => (i + items.len() - 1) % items.len(),
        " " if el.has_attribute("data-locale") => {
            e.prevent_default();
            if let Some(a) = el.dyn_ref::<HtmlElement>() {
                a.click();
            }
            return;
        }
        _ => return,
    };
    e.prevent_default();
    focus(&items[next]);
}

pub fn init() -> Result<(), JsValue> {
    let doc = document();
    let win = window();
    accent::init()?;
    theme::init()?;
    on(doc.as_ref(), "click", false, |event| {
        let Some(el) = target(&event) else { return };
        if let Some(t) = closest(&el, "[data-accent-trigger],[data-lang-trigger]") {
            if t.get_attribute("aria-expanded").as_deref() == Some("true") {
                dismiss(true);
            } else {
                open_menu(t.has_attribute("data-accent-trigger"));
            }
        } else if closest(&el, "[data-back-top]").is_some() {
            let options = web_sys::ScrollToOptions::new();
            options.set_top(0.);
            options.set_behavior(if reduced() {
                web_sys::ScrollBehavior::Auto
            } else {
                web_sys::ScrollBehavior::Smooth
            });
            window().scroll_to_with_scroll_to_options(&options);
        }
    })?;
    on(doc.as_ref(), "pointerdown", true, |e| {
        if let Some(t) = target(&e)
            && closest(&t, "[data-accent-picker],[data-lang]").is_none()
        {
            dismiss(false)
        }
    })?;
    on(doc.as_ref(), "focusin", false, |e| {
        let Some(t) = target(&e) else { return };
        if (menu_open("[data-accent-trigger]") && closest(&t, "[data-accent-picker]").is_none())
            || (menu_open("[data-lang-trigger]") && closest(&t, "[data-lang]").is_none())
        {
            dismiss(false);
        }
    })?;
    on(doc.as_ref(), "keydown", false, |e| {
        if let (Some(k), Some(t)) = (e.dyn_ref::<KeyboardEvent>(), target(&e))
            && !e.default_prevented()
        {
            menu_key(k, &t)
        }
    })?;
    on(win.as_ref(), "scroll", false, |_| back_top())?;
    on(win.as_ref(), "blur", false, |_| dismiss(false))?;
    on(win.as_ref(), "hashchange", false, |_| update_links())?;
    on(doc.as_ref(), "kui:navigated", false, |_| {
        dismiss(false);
        accent::refresh();
        theme::refresh();
        update_links();
        back_top();
    })?;
    update_links();
    back_top();
    Ok(())
}
