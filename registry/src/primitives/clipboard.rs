//! Confirm a clipboard write on the active platform.
use gpui::{App, Task};
#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(
    inline_js = "export function kuiCopy(s){return navigator.clipboard ? navigator.clipboard.writeText(s) : Promise.reject(new Error('Clipboard unavailable'))}"
)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name=kuiCopy,catch)]
    fn write_web(s: &str) -> Result<js_sys::Promise, wasm_bindgen::JsValue>;
}
pub fn write(text: &str, cx: &mut App) -> Task<bool> {
    #[cfg(not(target_family = "wasm"))]
    {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.to_owned()));
        Task::ready(
            cx.read_from_clipboard()
                .and_then(|v| v.text())
                .is_some_and(|v| v == text),
        )
    }
    #[cfg(target_family = "wasm")]
    {
        let request = write_web(text);
        cx.spawn(async move |_| match request {
            Ok(p) => wasm_bindgen_futures::JsFuture::from(p).await.is_ok(),
            Err(_) => false,
        })
    }
}
