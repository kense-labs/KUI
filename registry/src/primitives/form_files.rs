//! Host boundary for local files. File data never leaves the application here.
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, sync::Arc};
#[derive(Clone, Debug)]
pub struct PickedFile {
    pub name: String,
    pub bytes: Arc<[u8]>,
    pub size: u64,
}
impl PickedFile {
    pub fn image_format(&self) -> Option<ImageFormat> {
        if self.bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(ImageFormat::Png)
        } else if self.bytes.starts_with(&[255, 216, 255]) {
            Some(ImageFormat::Jpeg)
        } else if self.bytes.starts_with(b"RIFF") && self.bytes.get(8..12) == Some(b"WEBP") {
            Some(ImageFormat::Webp)
        } else {
            None
        }
    }
}
pub type FileResult = Result<Vec<PickedFile>, String>;
type HoverListener<T> = fn(&mut T, bool, &mut Context<T>);
#[cfg(not(target_family = "wasm"))]
fn read_files(paths: Vec<std::path::PathBuf>, max: u64) -> FileResult {
    use std::io::Read;
    paths
        .into_iter()
        .map(|path| {
            let input = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            let size = input.metadata().map_err(|e| e.to_string())?.len();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let mut bytes = Vec::new();
            if size <= max {
                input
                    .take(max.saturating_add(1))
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
            }
            let size = size.max(bytes.len() as u64);
            Ok(PickedFile {
                name,
                size,
                bytes: bytes.into(),
            })
        })
        .collect()
}
#[cfg(not(target_family = "wasm"))]
pub fn choose(multiple: bool, _accept: &str, max: u64, cx: &mut App) -> Task<FileResult> {
    let prompt = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple,
        prompt: None,
    });
    let exec = cx.background_executor().clone();
    cx.spawn(async move |_| {
        let paths = prompt
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        exec.spawn(async move { read_files(paths, max) }).await
    })
}
#[cfg(target_family = "wasm")]
mod browser {
    use super::*;
    use wasm_bindgen::{JsCast, prelude::*};
    use wasm_bindgen_futures::JsFuture;
    #[wasm_bindgen(inline_js = r#"
 async function kuiReadFiles(files,limit) {return Promise.all(Array.from(files).map(async f=>[f.name,f.size,new Uint8Array(f.size<=limit?await f.arrayBuffer():0)]))}
 export function kuiChooseFiles(multiple,accept,max) {return new Promise((resolve,reject)=>{const x=document.createElement('input');x.type='file';x.multiple=multiple;x.accept=accept;
 x.oncancel=()=>resolve([]);x.onchange=()=>kuiReadFiles(x.files,max).then(resolve,reject);x.click()})}
 export function kuiWatchFiles(hit,deliver,over,max) {const c=new AbortController(),opts={capture:true,signal:c.signal};
 function has(t){return t&&Array.from(t.types).includes('Files')}
 let armed=false;document.addEventListener('pointerdown',e=>{armed=hit(e.clientX,e.clientY)},opts);
 function consume(files){kuiReadFiles(files,max).then(result=>{if(!c.signal.aborted)deliver(result)},()=>{if(!c.signal.aborted)deliver(null)})}
 let inside=false;function hover(next){if(next!==inside){inside=next;over(next)}}
 document.addEventListener('dragover',e=>{const on=has(e.dataTransfer)&&hit(e.clientX,e.clientY);hover(on);if(on)e.preventDefault()},opts);
 document.addEventListener('dragleave',e=>{if(!e.relatedTarget)hover(false)},opts);
 document.addEventListener('dragend',()=>hover(false),opts);
 document.addEventListener('drop',e=>{hover(false);if(!has(e.dataTransfer)||!hit(e.clientX,e.clientY))return;e.preventDefault();e.stopImmediatePropagation();consume(e.dataTransfer.files)},opts);
 document.addEventListener('paste',e=>{if(!e.clipboardData?.files?.length||!armed)return;e.preventDefault();consume(e.clipboardData.files)},opts);return c}
 export function kuiUnwatchFiles(c){c.abort()}
 "#)]
    extern "C" {
        #[wasm_bindgen(js_name=kuiChooseFiles)]
        fn choose_js(multiple: bool, accept: &str, max: f64) -> js_sys::Promise;
        #[wasm_bindgen(js_name=kuiWatchFiles)]
        fn watch(
            hit: &js_sys::Function,
            deliver: &js_sys::Function,
            over: &js_sys::Function,
            max: f64,
        ) -> JsValue;
        #[wasm_bindgen(js_name=kuiUnwatchFiles)]
        fn unwatch(c: JsValue);
    }
    fn decode(v: JsValue) -> FileResult {
        if !js_sys::Array::is_array(&v) {
            return Err("Could not read file".into());
        }
        Ok(js_sys::Array::from(&v)
            .iter()
            .map(|r| {
                let a = js_sys::Array::from(&r);
                PickedFile {
                    name: a.get(0).as_string().unwrap_or_default(),
                    size: a.get(1).as_f64().unwrap_or(0.) as u64,
                    bytes: js_sys::Uint8Array::new(&a.get(2)).to_vec().into(),
                }
            })
            .collect())
    }
    pub(super) fn select(multiple: bool, accept: &str, max: u64, cx: &mut App) -> Task<FileResult> {
        let future = JsFuture::from(choose_js(multiple, accept, max as f64));
        cx.spawn(async move |_| decode(future.await.map_err(|_| "File chooser failed".to_owned())?))
    }
    pub(super) fn drops<T: 'static>(
        bounds: Rc<Cell<Bounds<Pixels>>>,
        max: u64,
        cx: &Context<T>,
        recv: fn(&mut T, FileResult, &mut Context<T>),
        hover: Option<HoverListener<T>>,
    ) -> Subscription {
        let hit = Closure::<dyn Fn(f32, f32) -> bool>::new(move |x, y| {
            let b = bounds.get();
            b.size.width > px(0.) && (x < 0. || b.contains(&point(px(x), px(y))))
        });
        let this = cx.weak_entity();
        let mut app = cx.to_async();
        let cb = Closure::<dyn FnMut(JsValue)>::new(move |v| {
            let result = decode(v);
            let _ = this.update(&mut app, |s, cx| recv(s, result, cx));
        });
        let this = cx.weak_entity();
        let mut app = cx.to_async();
        let over = Closure::<dyn FnMut(bool)>::new(move |inside| {
            if let Some(hover) = hover {
                let _ = this.update(&mut app, |s, cx| hover(s, inside, cx));
            }
        });
        let c = watch(
            hit.as_ref().unchecked_ref(),
            cb.as_ref().unchecked_ref(),
            over.as_ref().unchecked_ref(),
            max as f64,
        );
        Subscription::new(move || {
            unwatch(c);
            drop(hit);
            drop(cb);
            drop(over)
        })
    }
}
#[cfg(target_family = "wasm")]
pub fn choose(multiple: bool, accept: &str, max: u64, cx: &mut App) -> Task<FileResult> {
    browser::select(multiple, accept, max, cx)
}
pub fn drop_subscription<T: 'static>(
    bounds: Rc<Cell<Bounds<Pixels>>>,
    max: u64,
    cx: &Context<T>,
    recv: fn(&mut T, FileResult, &mut Context<T>),
    hover: Option<HoverListener<T>>,
) -> Option<Subscription> {
    #[cfg(target_family = "wasm")]
    {
        Some(browser::drops(bounds, max, cx, recv, hover))
    }
    #[cfg(not(target_family = "wasm"))]
    {
        let _ = (bounds, max, cx, recv, hover);
        None
    }
}
pub fn drop_target<T: 'static>(
    el: Stateful<Div>,
    cx: &Context<T>,
    max: u64,
    recv: fn(&mut T, FileResult, &mut Context<T>),
) -> Stateful<Div> {
    #[cfg(not(target_family = "wasm"))]
    {
        el.on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
            if e.keystroke.key != "v"
                || !(e.keystroke.modifiers.platform || e.keystroke.modifiers.control)
            {
                return;
            }
            let Some(clip) = cx.read_from_clipboard() else {
                return;
            };
            let mut files = vec![];
            let mut paths = vec![];
            for entry in clip.into_entries() {
                match entry {
                    ClipboardEntry::ExternalPaths(v) => paths.extend(v.0),
                    ClipboardEntry::Image(i) => {
                        let size = i.bytes.len() as u64;
                        files.push(PickedFile {
                            name: format!("clipboard.{}", i.format.extension()),
                            size,
                            bytes: if size <= max {
                                i.bytes.into()
                            } else {
                                Arc::from([])
                            },
                        })
                    }
                    ClipboardEntry::String(_) => {}
                }
            }
            if files.is_empty() && paths.is_empty() {
                return;
            }
            w.prevent_default();
            cx.stop_propagation();
            if !files.is_empty() {
                recv(s, Ok(files), cx)
            }
            if !paths.is_empty() {
                let task = cx
                    .background_executor()
                    .spawn(async move { read_files(paths, max) });
                cx.spawn(async move |this, cx| {
                    let r = task.await;
                    let _ = this.update(cx, |s, cx| recv(s, r, cx));
                })
                .detach()
            }
        }))
        .drag_over::<ExternalPaths>(|s, _, _, _| s.opacity(0.75))
        .on_drop(cx.listener(move |_, paths: &ExternalPaths, _, cx| {
            let ps = paths.0.to_vec();
            let task = cx
                .background_executor()
                .spawn(async move { read_files(ps, max) });
            cx.spawn(async move |this, cx| {
                let result = task.await;
                let _ = this.update(cx, |s, cx| recv(s, result, cx));
            })
            .detach()
        }))
    }
    #[cfg(target_family = "wasm")]
    {
        let _ = (cx, max, recv);
        el
    }
}
