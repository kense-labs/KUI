//! Swap document fragments without disconnecting the GPUI frame or rebuilding the sidebar.
use js_sys::{Function, Promise, Reflect};
use serde::Deserialize;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, future_to_promise, spawn_local};
use web_sys::{
    Element, Event, HtmlAnchorElement, HtmlIFrameElement, HtmlTemplateElement, MouseEvent, Node,
    Response, Url,
};

use crate::document;
#[derive(Deserialize)]
struct Fragment {
    id: String,
    lang: String,
    title: String,
    description: String,
    html: String,
}
struct Router {
    stamp: String,
    lang: String,
    cache: RefCell<VecDeque<(String, Promise)>>,
    bundle: RefCell<Option<JsValue>>,
    request: Cell<u32>,
}
fn win() -> web_sys::Window {
    web_sys::window().unwrap()
}
fn route(path: &str) -> Option<(&str, &str)> {
    let path = path.trim_end_matches('/');
    let (lang, id) = path
        .strip_prefix("/en/kui/components/")
        .map(|s| ("en", s))
        .or_else(|| path.strip_prefix("/kui/components/").map(|s| ("zh", s)))?;
    (!id.is_empty()
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'))
    .then_some((lang, id))
}
fn link(event: &Event) -> Option<HtmlAnchorElement> {
    event
        .target()?
        .dyn_into::<Element>()
        .ok()?
        .closest("a[data-nav-item]")
        .ok()??
        .dyn_into()
        .ok()
}
fn frame_in(article: &Element) -> Option<HtmlIFrameElement> {
    let frame = article
        .query_selector("[data-preview-slot] iframe")
        .ok()??;
    // A child can access its own frameElement first. In WebKit the parent
    // then receives a wrapper with the right interface but another prototype.
    (frame.namespace_uri().as_deref() == Some("http://www.w3.org/1999/xhtml"))
        .then(|| frame.unchecked_into())
}
fn attrs(to: &Element, from: &Element) {
    for name in to
        .get_attribute_names()
        .iter()
        .filter_map(|s| s.as_string())
    {
        if !from.has_attribute(&name) {
            let _ = to.remove_attribute(&name);
        }
    }
    for name in from
        .get_attribute_names()
        .iter()
        .filter_map(|s| s.as_string())
    {
        if let Some(v) = from.get_attribute(&name)
            && to.get_attribute(&name).as_ref() != Some(&v)
        {
            let _ = to.set_attribute(&name, &v);
        }
    }
}
// WebKit reloads an iframe if ANY ancestor is removed, even if reinserted in the same task.
fn around(current: &Node, incoming: &Node) -> Result<(), JsValue> {
    let a = current.parent_node().ok_or("missing host")?;
    let b = incoming.parent_node().ok_or("missing fragment")?;
    while let Some(n) = current.previous_sibling() {
        a.remove_child(&n)?;
    }
    while let Some(n) = current.next_sibling() {
        a.remove_child(&n)?;
    }
    while incoming.previous_sibling().is_some() {
        a.insert_before(&b.first_child().unwrap(), Some(current))?;
    }
    while incoming.next_sibling().is_some() {
        a.insert_before(&b.last_child().unwrap(), current.next_sibling().as_ref())?;
    }
    Ok(())
}
fn mount(current: &Element, incoming: &Element, id: &str, lang: &str) -> Result<(), JsValue> {
    let frame = frame_in(current).ok_or("missing live preview")?;
    let next = frame_in(incoming).ok_or("missing incoming preview")?;
    let base = win().location().href()?;
    let first = Url::new_with_base(&frame.get_attribute("src").ok_or("no preview src")?, &base)?;
    let second = Url::new_with_base(&next.get_attribute("src").ok_or("no preview src")?, &base)?;
    if first.pathname() != second.pathname() {
        return Err("preview build changed".into());
    }
    let mut a: Element = frame.clone().into();
    let mut b: Element = next.clone().into();
    let mut path = Vec::new();
    while !a.is_same_node(Some(current)) {
        if a.tag_name() != b.tag_name() {
            return Err("preview layout changed".into());
        }
        path.push((a.clone(), b.clone()));
        a = a.parent_element().ok_or("bad preview ancestor")?;
        b = b.parent_element().ok_or("bad fragment ancestor")?;
    }
    if !b.is_same_node(Some(incoming)) {
        return Err("preview tree changed".into());
    }
    for (i, (a, b)) in path.iter().enumerate() {
        if i != 0 {
            attrs(a, b);
        }
        around(a, b)?;
    }
    attrs(current, incoming);
    for key in [
        "aria-label",
        "class",
        "data-story",
        "data-story-key",
        "data-lang",
    ] {
        if let Some(val) = next.get_attribute(key) {
            frame.set_attribute(key, &val)?;
        }
    }
    frame.style().remove_property("height")?;
    if let Some(w) = frame.content_window() {
        let runtime = Reflect::get(&w, &"kuiPreview".into())?;
        if !runtime.is_undefined() && !runtime.is_null() {
            Reflect::get(&runtime, &"show".into())?
                .dyn_into::<Function>()?
                .call2(&runtime, &id.into(), &lang.into())?;
        }
    }
    Ok(())
}
impl Router {
    fn preload_locale(self: &Rc<Self>) {
        let router = self.clone();
        let request =
            win().fetch_with_str(&format!("/assets/pages/{}/{}.json", self.stamp, self.lang));
        spawn_local(async move {
            // A failed/stale preload never blocks a click: per-page requests remain available.
            let Ok(response) = JsFuture::from(request)
                .await
                .and_then(|v| v.dyn_into::<Response>())
            else {
                return;
            };
            if response.ok()
                && let Ok(payload) = response.json()
                && let Ok(index) = JsFuture::from(payload).await
                && index.is_object()
            {
                router.bundle.replace(Some(index));
            }
        });
    }
    fn load(&self, id: &str) -> Promise {
        if let Some(index) = self.bundle.borrow().as_ref()
            && let Ok(value) = Reflect::get(index, &id.into())
            && value.is_string()
        {
            return Promise::resolve(&value);
        }
        if let Some((_, p)) = self.cache.borrow().iter().find(|(key, _)| key == id) {
            return p.clone();
        }
        let req = win().fetch_with_str(&format!(
            "/assets/pages/{}/{}/{}.json",
            self.stamp, self.lang, id
        ));
        let future = future_to_promise(async move {
            let response = JsFuture::from(req).await?.dyn_into::<Response>()?;
            if !response.ok() {
                return Err("page not available".into());
            }
            JsFuture::from(response.text()?).await
        });
        let mut cache = self.cache.borrow_mut();
        if cache.len() >= 10 {
            cache.pop_front();
        }
        cache.push_back((id.into(), future.clone()));
        future
    }
    fn warm(&self, id: &str) {
        let p = self.load(id);
        spawn_local(async move {
            let _ = JsFuture::from(p).await;
        });
    }
    fn eligible(&self, url: &Url) -> Option<String> {
        if url.origin() != win().location().origin().ok()?
            || !url.search().is_empty()
            || !url.hash().is_empty()
        {
            return None;
        }
        let path = url.pathname();
        let (lang, id) = route(&path)?;
        (lang == self.lang).then(|| id.into())
    }
    fn visit(self: &Rc<Self>, url: Url, id: String, push: bool) {
        let n = self.request.get().wrapping_add(1);
        self.request.set(n);
        let this = self.clone();
        let p = self.load(&id);
        spawn_local(async move {
            let update = async {
                let value = JsFuture::from(p).await?;
                if n != this.request.get() {
                    return Ok::<(), JsValue>(());
                }
                let data: Fragment =
                    serde_json::from_str(&value.as_string().ok_or("invalid response")?)
                        .map_err(|e| e.to_string())?;
                if data.id != id || data.lang != this.lang {
                    return Err("invalid destination".into());
                }
                let doc = document();
                let template = doc
                    .create_element("template")?
                    .dyn_into::<HtmlTemplateElement>()?;
                template.set_inner_html(&data.html);
                let incoming = template
                    .content()
                    .query_selector("article.doc")?
                    .ok_or("no incoming article")?;
                let current = doc.query_selector("article.doc")?.ok_or("no article")?;
                mount(&current, &incoming, &id, &this.lang)?;
                if push {
                    win()
                        .history()?
                        .push_state_with_url(&JsValue::NULL, "", Some(&url.href()))?;
                    win().scroll_to_with_x_and_y(0., 0.);
                }
                doc.set_title(&format!("{} · KUI", data.title));
                if let Some(meta) = doc.query_selector("meta[name=description]")? {
                    meta.set_attribute("content", &data.description)?;
                }
                if let Some(canon) = doc.query_selector("link[rel=canonical]")? {
                    canon.set_attribute(
                        "href",
                        &format!("https://kense-labs.github.io{}", url.pathname()),
                    )?;
                }
                let rows = doc.query_selector_all("[data-nav-item]")?;
                for i in 0..rows.length() {
                    if let Some(row) = rows
                        .get(i)
                        .and_then(|n| n.dyn_into::<HtmlAnchorElement>().ok())
                    {
                        if row.pathname().trim_end_matches('/')
                            == url.pathname().trim_end_matches('/')
                        {
                            row.set_attribute("aria-current", "page")?;
                        } else if row.has_attribute("aria-current") {
                            row.remove_attribute("aria-current")?;
                        }
                    }
                }
                doc.dispatch_event(&Event::new("kui:navigated")?)?;
                Ok(())
            }
            .await;
            if let Err(reason) = update
                && this.request.get() == n
            {
                web_sys::console::warn_2(&"KUI: using full navigation".into(), &reason);
                if push {
                    let _ = win().location().assign(&url.href());
                } else {
                    let _ = win().location().reload();
                }
            }
        });
    }
}
pub fn init() -> Result<(), JsValue> {
    let doc = document();
    let current = win().location().pathname()?;
    let Some((lang, _)) = route(&current) else {
        return Ok(());
    };
    if doc
        .query_selector("article.doc [data-preview-slot] iframe")?
        .is_none()
    {
        return Ok(());
    }
    let Some(stamp) = doc
        .query_selector("meta[name=kui-pages]")?
        .and_then(|e| e.get_attribute("content"))
    else {
        return Ok(());
    };
    let router = Rc::new(Router {
        stamp,
        lang: lang.into(),
        cache: Default::default(),
        bundle: Default::default(),
        request: Cell::new(0),
    });
    router.preload_locale();
    let r = router.clone();
    let click = Closure::<dyn Fn(MouseEvent)>::new(move |e: MouseEvent| {
        if e.default_prevented()
            || e.button() != 0
            || e.meta_key()
            || e.ctrl_key()
            || e.shift_key()
            || e.alt_key()
        {
            return;
        }
        let Some(a) = link(&e) else { return };
        if a.has_attribute("download") || !a.target().is_empty() {
            return;
        }
        let Ok(url) = Url::new(&a.href()) else { return };
        let Some(id) = r.eligible(&url) else { return };
        e.prevent_default();
        r.visit(url, id, true);
    });
    doc.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    click.forget();
    let r = router.clone();
    let warm = Closure::<dyn Fn(Event)>::new(move |e: Event| {
        if let Some(id) = link(&e)
            .and_then(|a| Url::new(&a.href()).ok())
            .and_then(|u| r.eligible(&u))
        {
            r.warm(&id);
        }
    });
    for name in ["pointerover", "focusin", "touchstart"] {
        doc.add_event_listener_with_callback(name, warm.as_ref().unchecked_ref())?;
    }
    warm.forget();
    let r = router.clone();
    let pop = Closure::<dyn Fn(Event)>::new(move |_: Event| {
        let Some(url) = win().location().href().ok().and_then(|s| Url::new(&s).ok()) else {
            return;
        };
        if let Some(id) = r.eligible(&url) {
            r.visit(url, id, false);
        }
    });
    win().add_event_listener_with_callback("popstate", pop.as_ref().unchecked_ref())?;
    pop.forget();
    // Just the adjacent item, not 108 requests competing with the initial canvas.
    if let Some(row) = doc
        .query_selector("[data-nav-item][aria-current=page]")?
        .and_then(|e| e.next_element_sibling())
        .and_then(|e| e.dyn_into::<HtmlAnchorElement>().ok())
        && let Some(id) = Url::new(&row.href()).ok().and_then(|u| router.eligible(&u))
    {
        router.warm(&id);
    }
    Ok(())
}
