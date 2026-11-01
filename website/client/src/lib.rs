//! Small browser host for the Rust-rendered Kense site. GPUI loads independently.
mod cover;
mod docs;
mod navigation;
mod shell;
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

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    copy_actions()?;
    navigation::init()?;
    shell::init()?;
    docs::init()?;
    cover::init()?;
    if let Some(el) = document()
        .document_element()
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        el.set_attribute("data-client", "ready")?;
    }
    Ok(())
}
