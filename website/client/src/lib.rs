//! Small browser host for the Rust-rendered Kense site. GPUI loads independently.
mod company;
mod cover;
mod docs;
mod navigation;
mod shell;
#[path = "../../syntax.rs"]
mod syntax;
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Element, HtmlDialogElement, HtmlElement, HtmlTextAreaElement};

pub(crate) fn document() -> web_sys::Document {
    web_sys::window().unwrap().document().unwrap()
}

fn copy_actions() -> Result<(), JsValue> {
    let doc = document();
    let seq = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let click = Closure::<dyn Fn(web_sys::Event)>::new(move |event: web_sys::Event| {
        let Some(target) = event.target().and_then(|v| v.dyn_into::<Element>().ok()) else {
            return;
        };
        let Ok(Some(source)) =
            target.closest("button[data-copy],button[aria-busy=true],[data-close-copy]")
        else {
            return;
        };
        if source.has_attribute("data-close-copy") {
            if let Ok(Some(dialog)) = source.closest("dialog") {
                let _ = dialog.dyn_into::<HtmlDialogElement>().map(|d| d.close());
            }
            return;
        }
        if source.get_attribute("aria-busy").as_deref() == Some("true") {
            event.prevent_default();
            event.stop_immediate_propagation();
            return;
        }
        let Some(text) = source.get_attribute("data-copy") else {
            return;
        };
        let number = seq.get().wrapping_add(1);
        seq.set(number);
        let seq = seq.clone();
        let command = source.get_attribute("data-kind").as_deref() == Some("command");
        // Safari checks transient user activation when writeText is *called*.
        let navigator = web_sys::window().unwrap().navigator();
        let promise = js_sys::Reflect::get(&navigator, &"clipboard".into())
            .ok()
            .and_then(|v| v.dyn_into::<web_sys::Clipboard>().ok())
            .map(|cb| cb.write_text(&text));
        spawn_local(async move {
            let written = match promise {
                Some(p) => JsFuture::from(p).await.is_ok(),
                None => false,
            };
            if number != seq.get() || !source.is_connected() {
                return;
            }
            let doc = document();
            if written {
                let _ = source.set_attribute("data-copied", &number.to_string());
                let ev_name = match source.get_attribute("data-kind").as_deref() {
                    Some("prompt") => "kense:copied-prompt",
                    Some("command") => "kense:copied-cmd",
                    _ => "kense:copied-text",
                };
                if let Ok(ev) = web_sys::Event::new(ev_name) {
                    let _ = doc.dispatch_event(&ev);
                }
                let zh = doc.document_element().is_none_or(|e| !e.lang_is_en());
                let status = doc.get_element_by_id("site-copy-status");
                if let Some(st) = &status {
                    st.set_text_content(Some(match (zh, command) {
                        (true, true) => "命令已复制。",
                        (true, false) => "已复制到剪贴板。",
                        (false, true) => "Command copied.",
                        _ => "Copied to clipboard.",
                    }));
                }
                let timer = Closure::once_into_js(move || {
                    if source.get_attribute("data-copied").as_deref() == Some(&number.to_string()) {
                        let _ = source.remove_attribute("data-copied");
                    }
                    if number == seq.get()
                        && let Some(st) = status
                    {
                        st.set_text_content(None);
                    }
                });
                let _ = web_sys::window()
                    .unwrap()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        timer.unchecked_ref(),
                        2000,
                    );
            } else if let Some(dialog) = doc
                .get_element_by_id("site-manual-copy")
                .and_then(|e| e.dyn_into::<HtmlDialogElement>().ok())
                && let Some(field) = dialog
                    .query_selector("textarea")
                    .ok()
                    .flatten()
                    .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
            {
                field.set_value(&text);
                if !dialog.open() {
                    let _ = dialog.show_modal();
                }
                let _ = field.focus();
                field.select();
            }
        });
    });
    doc.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    click.forget();
    Ok(())
}
trait Language {
    fn lang_is_en(&self) -> bool;
}
impl Language for Element {
    fn lang_is_en(&self) -> bool {
        self.get_attribute("lang")
            .is_some_and(|v| v.starts_with("en"))
    }
}

/// Arms KUI's `ProductIntro` entrance animation whenever the user clicks into `/kui/` or `/en/kui/`.
fn arm_kui_intro() -> Result<(), JsValue> {
    if web_sys::window().is_none() {
        return Ok(());
    }
    let doc = document();
    let click = Closure::<dyn Fn(web_sys::Event)>::new(move |event: web_sys::Event| {
        let Some(me) = event.dyn_ref::<web_sys::MouseEvent>() else {
            return;
        };
        if me.button() != 0 || me.meta_key() || me.ctrl_key() || me.shift_key() || me.alt_key() {
            return;
        }
        let Some(anchor) = event
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .and_then(|el| el.closest("a[href]").ok().flatten())
            .and_then(|el| el.dyn_into::<web_sys::HtmlAnchorElement>().ok())
        else {
            return;
        };
        let target = anchor.target();
        if !target.is_empty() && target != "_self" {
            return;
        }
        let Some(win) = web_sys::window() else {
            return;
        };
        let Ok(base) = win.location().href() else {
            return;
        };
        let Ok(url) = web_sys::Url::new_with_base(&anchor.href(), &base) else {
            return;
        };
        if !url.hash().is_empty() {
            return;
        }
        let dest = url.pathname();
        if dest == "/klang" || dest.starts_with("/klang/") {
            if let Ok(Some(storage)) = win.session_storage() {
                let _ = storage.set_item("klang:product-intro:armed", "1");
            }
            if let Ok(Some(local)) = win.local_storage()
                && let Ok(Some(theme)) = local.get_item("theme")
            {
                let _ = local.set_item("k-docs-theme", &theme);
            }
            return;
        }
        if win.location().origin().ok().as_deref() != Some(url.origin().as_str()) {
            return;
        }
        if !matches!(dest.as_str(), "/kui" | "/kui/" | "/en/kui" | "/en/kui/") {
            return;
        }
        let cur = win.location().pathname().unwrap_or_default();
        if cur.trim_end_matches('/') == dest.trim_end_matches('/')
            || cur.starts_with("/kui/")
            || cur.starts_with("/en/kui/")
        {
            return;
        }
        if let Ok(Some(storage)) = win.session_storage() {
            let _ = storage.set_item("kui:product-intro:armed", "1");
        }
    });
    doc.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    click.forget();
    Ok(())
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    copy_actions()?;
    arm_kui_intro()?;
    navigation::init()?;
    shell::init()?;
    docs::init()?;
    cover::init()?;
    company::init()?;
    if let Some(el) = document()
        .document_element()
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        el.set_attribute("data-client", "ready")?;
    }
    Ok(())
}
