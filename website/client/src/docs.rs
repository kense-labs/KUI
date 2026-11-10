//! Browser interactions for the component reference and its sidebar.

use crate::document;
use js_sys::{Array, Function, Object, Reflect};
use std::cell::RefCell;
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    Animation, DomRect, Element, Event, EventTarget, HtmlElement, HtmlIFrameElement,
    HtmlInputElement, HtmlSelectElement, KeyboardEvent, MessageEvent, NodeList, Response,
};

fn query(selector: &str) -> Option<Element> {
    document().query_selector(selector).ok().flatten()
}

fn child(root: &Element, selector: &str) -> Option<Element> {
    root.query_selector(selector).ok().flatten()
}

fn matches(root: &Element, selector: &str) -> bool {
    root.matches(selector).unwrap_or(false)
}

fn nodes(list: NodeList) -> impl Iterator<Item = Element> {
    (0..list.length()).filter_map(move |i| list.get(i)?.dyn_into().ok())
}

fn children(root: &Element, selector: &str) -> Vec<Element> {
    root.query_selector_all(selector)
        .map(|v| nodes(v).collect())
        .unwrap_or_default()
}

fn target(event: &Event) -> Option<Element> {
    event.target()?.dyn_into().ok()
}

fn attr(el: &Element, name: &str, value: &str) {
    let _ = el.set_attribute(name, value);
}

fn flag(el: &Element, name: &str, on: bool) {
    if on {
        attr(el, name, "");
    } else {
        let _ = el.remove_attribute(name);
    }
}

fn english() -> bool {
    document()
        .document_element()
        .and_then(|e| e.get_attribute("lang"))
        .is_some_and(|v| v.starts_with("en"))
}

fn position_pill() -> Option<()> {
    let root = query("[data-nav-root]")?;
    let pill = child(&root, "[data-nav-pill]")?;
    let active = child(&root, "a[aria-current=page]");
    let active = active.filter(|row| row.closest("[hidden],[inert]").ok().flatten().is_none());
    attr(
        &pill,
        "data-hidden",
        if active.is_some() { "0" } else { "1" },
    );
    let row = active?.get_bounding_client_rect();
    let container = root.get_bounding_client_rect();
    let style = pill.dyn_ref::<HtmlElement>()?.style();
    let _ = style.set_property("width", &format!("{}px", row.width()));
    let _ = style.set_property("height", &format!("{}px", row.height()));
    let _ = style.set_property(
        "transform",
        &format!(
            "translate({}px,{}px)",
            row.left() - container.left(),
            row.top() - container.top()
        ),
    );
    attr(&pill, "data-guide", "1");
    Some(())
}

fn set_open(shell: &Element, open: bool) {
    let value = if open { "true" } else { "false" };
    attr(shell, "data-open", value);
    if let Some(btn) = child(shell, ":scope > button") {
        attr(&btn, "aria-expanded", value);
    }
    if let Some(body) = child(
        shell,
        ":scope > [data-section-body],:scope > [data-group-body]",
    ) {
        flag(&body, "inert", !open);
    }
}

fn filter() -> Option<()> {
    let root = query("[data-docs-sidebar]")?;
    let input = child(&root, "[data-sidebar-search]")?
        .dyn_into::<HtmlInputElement>()
        .ok()?;
    let search = input.value().trim().to_lowercase();
    let mut total = 0;
    for row in children(&root, "[data-nav-item]") {
        let show = search.is_empty()
            || row
                .get_attribute("data-search-terms")
                .unwrap_or_default()
                .contains(&search);
        flag(&row, "hidden", !show);
        total += usize::from(show);
    }
    for shell in children(&root, "[data-section],[data-group]") {
        let count = shell
            .query_selector_all("[data-nav-item]:not([hidden])")
            .ok()?
            .length();
        flag(&shell, "hidden", count == 0);
        if let Some(badge) = child(
            &shell,
            ":scope > button [data-section-count],:scope > button [data-group-count]",
        ) {
            badge.set_text_content(Some(&count.to_string()));
        }
        set_open(
            &shell,
            (!search.is_empty() && count > 0)
                || shell.get_attribute("data-user-open").as_deref() != Some("false"),
        );
    }
    if let Some(el) = child(&root, "[data-empty]") {
        flag(&el, "hidden", total != 0);
    }
    if let Some(el) = child(&root, "[data-search-status]") {
        el.set_text_content(Some(&if search.is_empty() {
            String::new()
        } else if english() {
            format!("{total} matches")
        } else {
            format!("{total} 个匹配项")
        }));
    }
    position_pill()
}

fn scroll_state() -> Option<()> {
    let scroll = query("[data-nav-scroll]")?;
    let at_bottom = scroll.scroll_top() + scroll.client_height() >= scroll.scroll_height() - 4;
    flag(&scroll, "data-fade-bottom", !at_bottom);
    Some(())
}

fn show_current(after_navigation: bool) -> Option<()> {
    let root = query("[data-docs-sidebar]")?;
    if after_navigation
        && let Some(input) = child(&root, "[data-sidebar-search]")
            .and_then(|v| v.dyn_into::<HtmlInputElement>().ok())
        && !input.value().is_empty()
    {
        input.set_value("");
        filter();
    }
    let row = child(&root, "[aria-current=page]")?;
    for selector in ["[data-section]", "[data-group]"] {
        if let Ok(Some(shell)) = row.closest(selector) {
            // Navigation always reveals the newly selected item.
            if shell.get_attribute("data-open").as_deref() != Some("true") {
                attr(&shell, "data-user-open", "true");
                set_open(&shell, true);
            }
        }
    }
    reveal_row(&row);
    scroll_state();
    position_pill()
}

fn reveal_row(row: &Element) {
    let Some(scroll) = query("[data-nav-scroll]") else {
        return;
    };
    let clip = scroll.get_bounding_client_rect();
    let bounds = row.get_bounding_client_rect();
    let mut top = clip.top();
    for selector in ["[data-section]", "[data-group]"] {
        if let Some(head) = row
            .closest(selector)
            .ok()
            .flatten()
            .and_then(|s| child(&s, ":scope > button"))
        {
            let sticky = web_sys::window()
                .and_then(|w| w.get_computed_style(&head).ok().flatten())
                .and_then(|s| s.get_property_value("position").ok())
                .as_deref()
                == Some("sticky");
            if sticky {
                top += head.get_bounding_client_rect().height();
            }
        }
    }
    let dy = if bounds.top() < top {
        bounds.top() - top
    } else {
        (bounds.bottom() - clip.bottom() + 4.0).max(0.0)
    };
    if dy != 0.0 {
        scroll.set_scroll_top(scroll.scroll_top() + dy.round() as i32);
    }
}

fn indicator() -> Option<()> {
    let stage = query("[data-component-stage]")?;
    let tabs = child(&stage, "[role=tablist]")?;
    let tab = child(&tabs, "[aria-selected=true]")?.get_bounding_client_rect();
    let underline = child(&tabs, "[data-view-indicator]")?
        .dyn_into::<HtmlElement>()
        .ok()?;
    let style = underline.style();
    let _ = style.set_property("width", &format!("{}px", tab.width()));
    let _ = style.set_property(
        "transform",
        &format!(
            "translateX({}px)",
            tab.left() - tabs.get_bounding_client_rect().left()
        ),
    );
    let _ = style.set_property("opacity", "1");
    Some(())
}

fn set_view(tab: &Element, focus: bool) -> Option<()> {
    let stage = tab.closest("[data-component-stage]").ok()??;
    let view = tab.get_attribute("data-view")?;
    let collapsed = view != "preview" && stage.has_attribute("data-preview-expanded");
    if collapsed && let Some(button) = child(&stage, "[data-preview-expand]") {
        expand_preview(&button, false);
    }
    if let Some(options) = child(&stage, "[data-preview-options]") {
        flag(&options, "hidden", view != "preview");
    }
    for item in children(&stage, "[data-view]") {
        let selected = item == *tab;
        attr(
            &item,
            "aria-selected",
            if selected { "true" } else { "false" },
        );
        attr(&item, "tabindex", if selected { "0" } else { "-1" });
    }
    for panel in children(&stage, "[data-panel]") {
        let selected = panel.get_attribute("data-panel").as_deref() == Some(&view);
        flag(&panel, "hidden", !selected);
        let _ = panel.class_list().toggle_with_force("is-active", selected);
    }
    if focus || collapsed {
        let _ = tab.dyn_ref::<HtmlElement>()?.focus();
    }
    indicator()
}

thread_local! {
    static PREVIEW_ANIMATION: RefCell<Option<Animation>> = const { RefCell::new(None) };
}

fn cancel_preview_animation() {
    PREVIEW_ANIMATION.with_borrow_mut(|running| {
        if let Some(animation) = running.take() {
            animation.cancel();
        }
    });
}

fn preview_animation(
    target: &Element,
    values: &[(&str, String, String)],
    duration: f64,
) -> Result<Animation, JsValue> {
    let frames = Object::new();
    for (property, from, to) in values {
        Reflect::set(
            &frames,
            &(*property).into(),
            &Array::of2(&from.into(), &to.into()),
        )?;
    }
    let options = Object::new();
    Reflect::set(&options, &"duration".into(), &duration.into())?;
    Reflect::set(
        &options,
        &"easing".into(),
        &"cubic-bezier(.22,1,.36,1)".into(),
    )?;
    Reflect::get(target, &"animate".into())?
        .dyn_into::<Function>()?
        .call2(target, &frames, &options)?
        .dyn_into()
}

fn preview_motion() -> bool {
    web_sys::window()
        .and_then(|win| {
            win.match_media("(prefers-reduced-motion: no-preference)")
                .ok()
                .flatten()
        })
        .is_some_and(|query| query.matches())
}

fn device_geometry(device: &Element) -> Option<(DomRect, String)> {
    let bounds = device.get_bounding_client_rect();
    let style = web_sys::window()?.get_computed_style(device).ok()??;
    Some((bounds, style.get_property_value("border-radius").ok()?))
}

fn fit_height(slot: &Element, restore: bool) -> Option<()> {
    let frame: HtmlIFrameElement = child(slot, "iframe")?.unchecked_into();
    let style = frame.style();
    if restore {
        let height = slot.get_attribute("data-fit-height").unwrap_or_default();
        style.set_property("height", &height).ok()?;
    } else {
        attr(
            slot,
            "data-fit-height",
            &style.get_property_value("height").ok()?,
        );
    }
    Some(())
}

fn set_preview_size(button: &Element) -> Option<()> {
    let stage = button.closest("[data-component-stage]").ok()??;
    let mode = button.get_attribute("data-preview-mode")?;
    let slot = child(&stage, "[data-preview-slot]")?;
    if slot.get_attribute("data-preview-size").as_deref() == Some(&mode) {
        return Some(());
    }
    let device = child(&slot, "[data-preview-device]")?;
    let before = preview_motion().then(|| device_geometry(&device)).flatten();
    cancel_preview_animation();
    // Use the previous fit height until GPUI paints at the new width.
    fit_height(&slot, mode == "fit");
    attr(&slot, "data-preview-size", &mode);
    for item in children(&stage, "[data-preview-mode]") {
        let selected = item == *button;
        attr(
            &item,
            "aria-checked",
            if selected { "true" } else { "false" },
        );
        attr(&item, "tabindex", if selected { "0" } else { "-1" });
    }
    if let Some((old, old_radius)) = before
        && let Some((new, new_radius)) = device_geometry(&device)
        && let Ok(animation) = preview_animation(
            &device,
            &[
                (
                    "width",
                    format!("{}px", old.width()),
                    format!("{}px", new.width()),
                ),
                (
                    "height",
                    format!("{}px", old.height()),
                    format!("{}px", new.height()),
                ),
                ("borderRadius", old_radius, new_radius),
            ],
            320.,
        )
    {
        PREVIEW_ANIMATION.with_borrow_mut(|running| *running = Some(animation));
    }
    Some(())
}

fn expand_preview(trigger: &Element, open: bool) -> Option<()> {
    let stage = trigger.closest("[data-component-stage]").ok()??;
    let bar = child(&stage, ".preview-fullscreen-bar")?;
    let slot = child(&stage, "[data-preview-slot]")?;
    let origin = slot.get_bounding_client_rect();
    cancel_preview_animation();
    if slot.get_attribute("data-preview-size").as_deref() == Some("fit") {
        fit_height(&slot, !open);
    }
    flag(&stage, "data-preview-expanded", open);
    flag(&bar, "hidden", !open);
    if open {
        attr(&stage, "role", "dialog");
        attr(&stage, "aria-modal", "true");
        attr(&stage, "aria-labelledby", "stage-fullscreen-title");
        if preview_motion() {
            let bounds = stage.get_bounding_client_rect();
            let (w, h) = (bounds.width(), bounds.height());
            let start = format!(
                "inset({}px {}px {}px {}px round 8px)",
                origin.top().clamp(0., h),
                (w - origin.right()).clamp(0., w),
                (h - origin.bottom()).clamp(0., h),
                origin.left().clamp(0., w)
            );
            if let Ok(animation) =
                preview_animation(&stage, &[("clipPath", start, "inset(0px)".into())], 260.)
            {
                PREVIEW_ANIMATION.with_borrow_mut(|running| *running = Some(animation));
            }
        }
    } else {
        for name in ["role", "aria-modal", "aria-labelledby"] {
            flag(&stage, name, false);
        }
    }
    let target = child(
        &stage,
        if open {
            "[data-preview-close]"
        } else {
            "[data-preview-expand]"
        },
    )?;
    target.dyn_ref::<HtmlElement>()?.focus().ok();
    indicator()
}

fn tab_stops(scope: &Element) -> Vec<HtmlElement> {
    children(scope, "a[href],button:not(:disabled),input:not(:disabled),select:not(:disabled),summary,iframe,[tabindex='0']")
        .into_iter()
        .filter_map(|element| element.dyn_into::<HtmlElement>().ok())
        .filter(|element| {
            element.tab_index() >= 0
                && element.closest("[inert],[hidden]").ok().flatten().is_none()
                && element.get_bounding_client_rect().height() > 0.
        })
        .collect()
}

fn click(event: &Event) -> Option<()> {
    let target = target(event)?;
    if let Some(tab) = target.closest("[data-component-stage] [data-view]").ok()? {
        set_view(&tab, false);
    }
    if let Some(button) = target.closest("[data-preview-mode]").ok()? {
        return set_preview_size(&button);
    }
    if let Some(button) = target
        .closest("[data-preview-expand],[data-preview-close]")
        .ok()?
    {
        return expand_preview(&button, button.has_attribute("data-preview-expand"));
    }
    if let Some(toggle) = target
        .closest("[data-section-toggle],[data-group-toggle]")
        .ok()?
    {
        let shell = toggle.parent_element()?;
        let open = toggle.get_attribute("aria-expanded").as_deref() != Some("true");
        attr(
            &shell,
            "data-user-open",
            if open { "true" } else { "false" },
        );
        set_open(&shell, open);
        if !open
            && shell.get_attribute("data-section-id").as_deref() == Some("components")
            && let Some(scroll) = query("[data-nav-scroll]")
        {
            scroll.set_scroll_top(0);
        }
        position_pill();
        scroll_state();
    }
    Some(())
}

fn keydown(event: &Event) -> Option<()> {
    let key = event.dyn_ref::<KeyboardEvent>()?;
    if key.is_composing() || key.default_prevented() {
        return None;
    }
    let target = target(event)?;
    let input = query("[data-sidebar-search]")?
        .dyn_into::<HtmlInputElement>()
        .ok()?;
    let name = key.key();
    if let Some(stage) = query("[data-preview-expanded]") {
        if name == "Escape" {
            event.prevent_default();
            return expand_preview(&child(&stage, "[data-preview-close]")?, false);
        }
        if name == "Tab" {
            let stops = tab_stops(&stage);
            let (edge, next) = if key.shift_key() {
                (stops.first(), stops.last())
            } else {
                (stops.last(), stops.first())
            };
            if edge.is_some_and(|element| element.unchecked_ref::<Element>() == &target) {
                event.prevent_default();
                return next?.focus().ok();
            }
        }
        return None;
    }
    if matches(&target, "[data-preview-mode]") {
        let next = match name.as_str() {
            "ArrowRight" | "ArrowLeft" | "ArrowUp" | "ArrowDown" => {
                if target.get_attribute("data-preview-mode").as_deref() == Some("fit") {
                    "mobile"
                } else {
                    "fit"
                }
            }
            "Home" => "fit",
            "End" => "mobile",
            _ => return None,
        };
        let group = target.closest("[role=radiogroup]").ok()??;
        let next = child(&group, &format!(r#"[data-preview-mode="{next}"]"#))?;
        event.prevent_default();
        set_preview_size(&next);
        return next.dyn_ref::<HtmlElement>()?.focus().ok();
    }
    if ((key.meta_key() || key.ctrl_key()) && name.eq_ignore_ascii_case("k"))
        || (name == "/"
            && !key.alt_key()
            && !key.ctrl_key()
            && !key.meta_key()
            && target
                .closest("input,textarea,select,[contenteditable]:not([contenteditable=false])")
                .ok()?
                .is_none())
    {
        event.prevent_default();
        let _ = input.focus();
        input.select();
        return Some(());
    }
    if matches(&target, "[data-sidebar-search]") {
        match name.as_str() {
            "Escape" => {
                input.set_value("");
                filter();
                event.prevent_default();
            }
            "Enter" if !input.value().trim().is_empty() => {
                if let Some(row) = query("a[data-nav-item]:not([hidden])")
                    .and_then(|e| e.dyn_into::<HtmlElement>().ok())
                {
                    event.prevent_default();
                    row.click();
                }
            }
            _ => {}
        }
    } else if matches(&target, "[data-view]") {
        let next = match name.as_str() {
            "ArrowRight" | "ArrowLeft" => {
                if target.get_attribute("data-view").as_deref() == Some("preview") {
                    "code"
                } else {
                    "preview"
                }
            }
            "Home" => "preview",
            "End" => "code",
            _ => return None,
        };
        event.prevent_default();
        set_view(&query(&format!("[data-view={next}]"))?, true);
    }
    Some(())
}

fn paint_file(select: &HtmlSelectElement) -> Option<()> {
    let files = Reflect::get(select, &"kuiSourceFiles".into())
        .ok()?
        .dyn_into::<Array>()
        .ok()?;
    let file = files.get(select.selected_index().try_into().ok()?);
    let content = Reflect::get(&file, &"content".into()).ok()?.as_string()?;
    let source = select.closest("[data-item-source]").ok()??;
    let pre = child(&source, "pre")?;
    child(&pre, "code")?.set_inner_html(&crate::syntax::highlight(&content, false));
    pre.set_scroll_top(0);
    Some(())
}

async fn load_source(source: &Element) -> Result<(), JsValue> {
    let link = child(source, "a[href]")
        .and_then(|a| a.get_attribute("href"))
        .ok_or("registry link missing")?;
    let response = JsFuture::from(web_sys::window().ok_or("window")?.fetch_with_str(&link))
        .await?
        .dyn_into::<Response>()?;
    if !response.ok() {
        return Err("registry fetch failed".into());
    }
    let data = JsFuture::from(response.json()?).await?;
    let files = Reflect::get(&data, &"files".into())?.dyn_into::<Array>()?;
    if files.length() == 0 {
        return Err("empty registry item".into());
    }
    let select = child(source, "select")
        .ok_or("file list")?
        .dyn_into::<HtmlSelectElement>()?;
    let options = document().create_document_fragment();
    for file in files.iter() {
        let path = Reflect::get(&file, &"path".into())?
            .as_string()
            .ok_or("file path")?;
        let option = document().create_element("option")?;
        option.set_attribute("value", &path)?;
        option.set_text_content(Some(&path));
        options.append_child(&option)?;
    }
    select.set_text_content(None);
    select.append_child(&options)?;
    Reflect::set(&select, &"kuiSourceFiles".into(), &files)?;
    paint_file(&select).ok_or("source content")?;
    select.set_hidden(false);
    if let Some(pre) = child(source, "pre") {
        flag(&pre, "hidden", false);
    }
    Ok(())
}

fn toggle(event: &Event) -> Option<()> {
    let details = target(event)?;
    if !matches(&details, "details[open]") {
        return None;
    }
    if let Some(frame) = child(&details, "iframe[data-src]:not([src])") {
        attr(&frame, "src", &frame.get_attribute("data-src")?);
    }
    if !details.has_attribute("data-item-source") || details.has_attribute("data-loaded") {
        return None;
    }
    attr(&details, "data-loaded", "pending");
    let en = details.get_attribute("data-locale").as_deref() == Some("en");
    let status = child(&details, "[role=status]")?;
    flag(&status, "hidden", false);
    status.set_text_content(Some(if en {
        "Loading source…"
    } else {
        "正在读取源码…"
    }));
    spawn_local(async move {
        if load_source(&details).await.is_ok() {
            attr(&details, "data-loaded", "yes");
            flag(&status, "hidden", true);
        } else {
            let _ = details.remove_attribute("data-loaded");
            status.set_text_content(Some(if en {
                "Could not load source. Reopen to retry."
            } else {
                "未能读取。重新展开可重试。"
            }));
        }
    });
    Some(())
}

fn leave_story(event: &Event) -> Option<()> {
    let message = event.dyn_ref::<MessageEvent>()?;
    let origin = web_sys::window()?.location().origin().ok()?;
    if message.origin() != origin {
        return None;
    }
    let data = message.data();
    let kind = Reflect::get(&data, &"type".into()).ok()?.as_string()?;
    let source = message.source()?;
    if kind == "kui:preview-exit" {
        let frame = query("[data-preview-expanded] [data-preview-slot] iframe")?;
        let frame: HtmlIFrameElement = frame.unchecked_into();
        if js_sys::Object::is(frame.content_window()?.as_ref(), source.as_ref()) {
            return expand_preview(
                &query("[data-preview-expanded] [data-preview-close]")?,
                false,
            );
        }
        return None;
    }
    if kind != "kui:story-leave" {
        return None;
    }
    let back = Reflect::get(&data, &"back".into()).ok()?.as_bool()?;
    let expanded = query("[data-preview-expanded]");
    let scope = expanded.clone().or_else(|| document().document_element())?;
    let stops = tab_stops(&scope);
    let index = stops.iter().position(|el| {
        el.dyn_ref::<HtmlIFrameElement>()
            .filter(|f| f.has_attribute("data-story"))
            .and_then(|f| f.content_window())
            .is_some_and(|win| js_sys::Object::is(win.as_ref(), source.as_ref()))
    })?;
    let next = if back {
        index
            .checked_sub(1)
            .or_else(|| expanded.as_ref().map(|_| stops.len() - 1))?
    } else if expanded.is_some() {
        (index + 1) % stops.len()
    } else {
        index + 1
    };
    stops.get(next)?.focus().ok()
}

fn on(
    target: &EventTarget,
    name: &str,
    capture: bool,
    handle: impl Fn(&Event) + 'static,
) -> Result<(), JsValue> {
    let listener = Closure::<dyn Fn(Event)>::new(move |event: Event| handle(&event));
    target.add_event_listener_with_callback_and_bool(
        name,
        listener.as_ref().unchecked_ref(),
        capture,
    )?;
    listener.forget();
    Ok(())
}

/// Bind delegated reference interactions once, including future route fragments.
pub fn init() -> Result<(), JsValue> {
    if query("[data-docs-sidebar]").is_none() {
        return Ok(());
    }
    let doc = document();
    let win = web_sys::window().ok_or("window")?;
    on(&doc, "click", false, |e| {
        click(e);
    })?;
    on(&doc, "input", false, |e| {
        if target(e).is_some_and(|t| matches(&t, "[data-sidebar-search]")) {
            filter();
        }
    })?;
    on(&doc, "keydown", false, |e| {
        keydown(e);
    })?;
    on(&doc, "change", false, |e| {
        if let Some(el) = target(e)
            .filter(|t| matches(t, "[data-item-source] select"))
            .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
        {
            paint_file(&el);
        }
    })?;
    on(&doc, "toggle", true, |e| {
        toggle(e);
    })?;
    on(&doc, "transitionend", false, |e| {
        if target(e).is_some_and(|t| matches(&t, "[data-group-body]")) {
            position_pill();
            scroll_state();
        }
    })?;
    on(&doc, "scroll", true, |e| {
        if target(e).is_some_and(|t| matches(&t, "[data-nav-scroll]")) {
            scroll_state();
        }
    })?;
    on(&doc, "kui:navigated", false, |_| {
        cancel_preview_animation();
        show_current(true);
        indicator();
    })?;
    on(&win, "resize", false, |_| {
        position_pill();
        indicator();
        scroll_state();
    })?;
    on(&win, "message", false, |e| {
        leave_story(e);
    })?;
    if let Some(kbd) = query("[data-search-kbd]") {
        let mac = win.navigator().platform().ok().is_some_and(|v| {
            ["Mac", "iPhone", "iPad"]
                .iter()
                .any(|name| v.contains(name))
        });
        kbd.set_text_content(Some(if mac { "⌘K" } else { "/" }));
    }
    show_current(false);
    indicator();
    spawn_local(async move {
        if let Ok(ready) = doc.fonts().ready() {
            let _ = JsFuture::from(ready).await;
        }
        position_pill();
        indicator();
    });
    Ok(())
}
