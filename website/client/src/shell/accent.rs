//! Accent preferences, animated preview colors, and the accessible HSV editor.

use super::*;
use std::cell::RefCell;
use web_sys::{HtmlInputElement, PointerEvent};

#[path = "../../../site/shell/palette.rs"]
mod palette;
use palette::PRESETS;

type Pair = [u32; 2];

#[derive(Default, Clone, Copy)]
struct Hsv {
    h: f64,
    s: f64,
    v: f64,
}
impl Hsv {
    fn from_rgb(rgb: u32, fallback: f64) -> Self {
        let [r, g, b] = [16, 8, 0].map(|shift| ((rgb >> shift) & 255u32) as f64 / 255.);
        let v = r.max(g).max(b);
        let d = v - r.min(g).min(b);
        let h = if d == 0. {
            fallback
        } else {
            let sector = if r == v {
                (g - b) / d
            } else if g == v {
                2. + (b - r) / d
            } else {
                4. + (r - g) / d
            };
            (sector * 60.).rem_euclid(360.)
        };
        Self {
            h,
            s: if v == 0. { 0. } else { d / v },
            v,
        }
    }
    fn rgb(self) -> u32 {
        let c = self.s * self.v;
        let sector = (self.h % 360.) / 60.;
        let x = c * (1. - (sector % 2. - 1.).abs());
        let channels = match sector as u8 {
            0 => [c, x, 0.],
            1 => [x, c, 0.],
            2 => [0., c, x],
            3 => [0., x, c],
            4 => [x, 0., c],
            _ => [c, 0., x],
        };
        channels.into_iter().fold(0, |rgb, v| {
            (rgb << 8) | ((v + self.v - c) * 255.).round() as u32
        })
    }
}
fn hex(rgb: u32) -> String {
    format!("#{rgb:06x}")
}
fn parse(value: &str) -> Option<u32> {
    let digits = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if !digits.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let n = u32::from_str_radix(digits, 16).ok()?;
    match digits.len() {
        3 => Some(((n & 0xf00) * 0x1100) | ((n & 0xf0) * 0x110) | ((n & 0xf) * 0x11)),
        6 => Some(n),
        _ => None,
    }
}

fn endpoint(value: &str) -> Option<Pair> {
    if let Some(&(_, _, _, l, d)) = PRESETS.iter().find(|p| p.0 == value) {
        return Some(if value == "neutral" {
            [0; 2]
        } else {
            [(l << 8) | 255, (d << 8) | 255]
        });
    }
    Some([(parse(value)? << 8) | 255; 2])
}
fn ink(rgb: u32) -> &'static str {
    let [r, g, b] = [16, 8, 0].map(|shift| {
        let c = ((rgb >> shift) & 255u32) as f64 / 255.;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    if r * 0.2126 + g * 0.7152 + b * 0.0722 > 0.179129 {
        "#000000"
    } else {
        "#ffffff"
    }
}
fn mix(a: u32, b: u32, t: f64) -> u32 {
    let first = (a & 255) as f64 * (1. - t);
    let second = (b & 255) as f64 * t;
    let opacity = first + second;
    if opacity == 0. {
        return 0;
    }
    let mut color = opacity.round() as u32;
    for shift in [24, 16, 8] {
        let v = (((a >> shift) & 255u32) as f64 * first + ((b >> shift) & 255u32) as f64 * second)
            / opacity;
        color |= (v.round() as u32) << shift;
    }
    color
}
fn opaque(color: u32, base: u32) -> u32 {
    mix((base << 8) | 255, color | 255, (color & 255) as f64 / 255.) >> 8
}

#[derive(Default)]
struct Accent {
    committed: String,
    custom: u32,
    draft: Option<Hsv>,
    pointer: Option<i32>,
    frame: u32,
    colors: Pair,
}
thread_local! {static STATE:RefCell<Accent>=RefCell::default();}

fn present(colors: Pair, settled: bool) {
    let root = root();
    let Some(el) = root.dyn_ref::<HtmlElement>() else {
        return;
    };
    if settled {
        for key in [
            "--accent-light",
            "--accent-dark",
            "--ink-light",
            "--ink-dark",
            "--accent-wash",
        ] {
            let _ = el.style().remove_property(key);
        }
    } else {
        for (i, suffix) in ["light", "dark"].into_iter().enumerate() {
            let rgb = opaque(colors[i], if i == 0 { 0 } else { 0xffffff });
            css(el, &format!("--accent-{suffix}"), &hex(rgb));
            css(el, &format!("--ink-{suffix}"), ink(rgb));
        }
        css(
            el,
            "--accent-wash",
            &format!("{}%", (colors[0] & 255) as f64 * 16. / 255.),
        );
    }
    STATE.with_borrow_mut(|s| s.colors = colors);
    for f in frames() {
        sync_frame(&f);
    }
}
pub(super) fn sync_frame(frame: &HtmlIFrameElement) {
    let [l, d] = STATE.with_borrow(|s| s.colors);
    let _ = preview(frame, "accent", &Array::of2(&l.into(), &d.into()));
}
fn step(frame: u32, previous: f64, elapsed: f64, from: Pair, to: Pair) {
    let cb = Closure::once_into_js(move |now: f64| {
        if STATE.with_borrow(|s| s.frame != frame) {
            return;
        }
        // A busy GPU can delay a single animation frame past the entire
        // transition. Resume visibly from the last presented color.
        let elapsed = elapsed + (now - previous).clamp(0., 50.);
        let t = if document().hidden() || reduced() {
            1.
        } else {
            (elapsed / 180.).min(1.)
        };
        let ease = 1. - (1. - t).powi(3);
        present(
            [mix(from[0], to[0], ease), mix(from[1], to[1], ease)],
            t == 1.,
        );
        if t < 1. {
            step(frame, now, elapsed, from, to);
        }
    });
    let _ = window().request_animation_frame(cb.unchecked_ref());
}
fn show(value: &str, animate: bool) {
    let Some(to) = endpoint(value) else { return };
    let (from, frame) = STATE.with_borrow_mut(|s| {
        s.frame = s.frame.wrapping_add(1);
        (s.colors, s.frame)
    });
    let r = root();
    attr(&r, "data-accent", value);
    if let Some(rgb) = parse(value) {
        css(&r, "--custom-accent", &hex(rgb));
        css(&r, "--custom-ink", ink(rgb));
    } else if let Some(el) = r.dyn_ref::<HtmlElement>() {
        let _ = el.style().remove_property("--custom-ink");
        let _ = el.style().remove_property("--custom-accent");
    }
    if !animate || reduced() || document().hidden() || from == to {
        present(to, true)
    } else if let Some(clock) = window().performance() {
        // The new data-accent selector already exposes its final fill. Hold
        // the currently displayed color until the first interpolation frame.
        present(from, false);
        step(frame, clock.now(), 0., from, to)
    } else {
        present(to, true)
    }
}
fn mark_selected() {
    let (value, custom) = STATE.with_borrow(|s| (s.committed.clone(), s.custom));
    let which = if value.starts_with('#') {
        "custom"
    } else {
        &value
    };
    if let Some(s) = q("[data-custom-swatch]") {
        css(&s, "--saved-custom", &hex(custom));
    }
    for el in all("[data-accent-value]") {
        let selected = el.get_attribute("data-accent-value").as_deref() == Some(which);
        attr(&el, "aria-checked", if selected { "true" } else { "false" });
        if selected
            && let (Some(trigger), Some(menu)) =
                (q("[data-accent-trigger]"), q("[data-accent-menu]"))
        {
            let mut label = format!(
                "{} · {}",
                menu.get_attribute("aria-label").unwrap_or_default(),
                el.text_content().unwrap_or_default().trim()
            );
            if which == "custom" {
                label.push_str(&format!(" · {}", value.to_uppercase()));
            }
            attr(&trigger, "aria-label", &label);
            attr(&trigger, "title", &label);
        }
    }
}
fn commit(value: &str) {
    if endpoint(value).is_none() {
        return;
    }
    STATE.with_borrow_mut(|s| {
        s.committed = value.to_owned();
        if let Some(rgb) = parse(value) {
            s.custom = rgb;
            store("kui-custom-accent", value);
        }
    });
    store("kui-accent", value);
    show(value, true);
    mark_selected();
    let _ = window().dispatch_event(&Event::new("kui:accent-change").expect("valid event type"));
}

pub(super) fn refresh() {
    let value = STATE.with_borrow_mut(|s| {
        s.draft = None;
        s.pointer = None;
        s.committed.clone()
    });
    show(&value, false);
    mark_selected();
}
fn input(selector: &str) -> Option<HtmlInputElement> {
    q(selector)?.dyn_into().ok()
}
fn valid(ok: bool) {
    if let Some(e) = q("[data-hex-input]") {
        attr(&e, "aria-invalid", if ok { "false" } else { "true" });
    }
    if let Some(e) = q("[data-hex-error]") {
        hide(&e, ok);
    }
    if let Some(e) = q("[data-custom-apply]") {
        if ok {
            remove(&e, "disabled")
        } else {
            attr(&e, "disabled", "")
        }
    }
}
fn render_editor(hsv: Hsv, keep_input: bool) {
    let value = hex(hsv.rgb());
    let Some(panel) = q("[data-custom-accent-editor]") else {
        return;
    };
    STATE.with_borrow_mut(|s| s.draft = Some(hsv));
    css(&panel, "--hue", &hsv.h.to_string());
    css(&panel, "--saturation", &format!("{}%", 100. * hsv.s));
    css(&panel, "--top", &format!("{}%", 100. * (1. - hsv.v)));
    css(&panel, "--draft", &value);
    if !keep_input && let Some(i) = input("[data-hex-input]") {
        i.set_value(&value.to_uppercase());
    }
    for (key, val, unit) in [
        ("hue", hsv.h, "°"),
        ("saturation", 100. * hsv.s, "%"),
        ("brightness", 100. * hsv.v, "%"),
    ] {
        let n = val.round().to_string();
        let label = format!("{n}{unit}");
        if let Some(i) = input(&format!("[data-{key}]")) {
            i.set_value(&n);
            attr(&i, "aria-valuetext", &label);
        }
        if let Some(el) = q(&format!("[data-{key}-text]")) {
            el.set_text_content(Some(&label));
        }
    }
    valid(true);
    show(&value, false);
}
fn open_editor() {
    if let Some(p) = q("[data-accent-picker]") {
        attr(&p, "data-editing", "");
    }
    if let Some(m) = q("[data-accent-menu]") {
        attr(&m, "inert", "");
    }
    if let Some(t) = q("[data-accent-trigger]") {
        attr(&t, "aria-controls", "site-custom-accent");
    }
    if let Some(e) = q("[data-custom-accent-editor]") {
        hide(&e, false);
    }
    render_editor(STATE.with_borrow(|s| Hsv::from_rgb(s.custom, 0.)), false);
    if let Some(s) = q("[data-saturation]") {
        focus(&s);
    }
}
pub(super) fn close_editor() {
    let restore = STATE.with_borrow_mut(|s| {
        s.pointer = None;
        s.draft.take().map(|_| s.committed.clone())
    });
    if let Some(value) = restore {
        show(&value, true);
    }
    if let Some(p) = q("[data-accent-picker]") {
        remove(&p, "data-editing");
    }
    if let Some(m) = q("[data-custom-accent-editor]") {
        hide(&m, true);
    }
    if let Some(t) = q("[data-accent-trigger]") {
        attr(&t, "aria-controls", "site-accent-menu");
    }
}
fn save() {
    if let Some(rgb) = input("[data-hex-input]").and_then(|i| parse(&i.value())) {
        STATE.with_borrow_mut(|s| s.draft = None);
        commit(&hex(rgb));
        dismiss(true);
    }
}
fn move_pointer(p: &PointerEvent, plane: &Element) {
    let rect = plane.get_bounding_client_rect();
    if rect.width() <= 0. || rect.height() <= 0. {
        return;
    }
    if let Some(mut hsv) = STATE.with_borrow(|s| s.draft) {
        hsv.s = ((f64::from(p.client_x()) - rect.left()) / rect.width()).clamp(0., 1.);
        hsv.v = (1. - (f64::from(p.client_y()) - rect.top()) / rect.height()).clamp(0., 1.);
        render_editor(hsv, false);
    }
}
fn read_saved() {
    let saved = stored("kui-accent")
        .or_else(|| root().get_attribute("data-accent"))
        .filter(|v| endpoint(v).is_some())
        .unwrap_or_else(|| "neutral".into());
    let custom = stored("kui-custom-accent")
        .and_then(|v| parse(&v))
        .or_else(|| parse(&saved))
        .unwrap_or(PRESETS[1].3);
    STATE.with_borrow_mut(|s| {
        s.committed = saved;
        s.custom = custom;
    });
    refresh();
}
pub(super) fn init() -> Result<(), JsValue> {
    read_saved();
    let doc = document();
    on(doc.as_ref(), "click", false, |e| {
        let Some(el) = target(&e) else { return };
        if let Some(opt) = closest(&el, "[data-accent-value]") {
            let value = opt.get_attribute("data-accent-value").unwrap_or_default();
            if value == "custom" {
                open_editor()
            } else {
                commit(&value);
                dismiss(true)
            }
        } else if closest(&el, "[data-custom-back]").is_some() {
            close_editor();
            if let Some(m) = q("[data-accent-menu]") {
                remove(&m, "inert");
            }
            if let Some(t) = q(r#"[data-accent-value="custom"]"#) {
                focus(&t);
            }
        } else if closest(&el, "[data-custom-cancel]").is_some() {
            dismiss(true)
        } else if closest(&el, "[data-custom-apply]").is_some() {
            save()
        }
    })?;
    on(doc.as_ref(), "input", false, |e| {
        let (Some(i), Some(mut hsv)) = (
            target(&e).and_then(|t| t.dyn_into::<HtmlInputElement>().ok()),
            STATE.with_borrow(|s| s.draft),
        ) else {
            return;
        };
        if i.has_attribute("data-hex-input") {
            if let Some(rgb) = parse(&i.value()) {
                render_editor(Hsv::from_rgb(rgb, hsv.h), true);
            } else {
                valid(false)
            }
        } else {
            let val = i.value_as_number();
            if i.has_attribute("data-hue") {
                hsv.h = val;
            } else if i.has_attribute("data-saturation") {
                hsv.s = val / 100.;
            } else if i.has_attribute("data-brightness") {
                hsv.v = val / 100.;
            } else {
                return;
            }
            render_editor(hsv, false);
        }
    })?;
    on(doc.as_ref(), "keydown", false, |e| {
        let (Some(k), Some(el), Some(mut hsv)) = (
            e.dyn_ref::<KeyboardEvent>(),
            target(&e),
            STATE.with_borrow(|s| s.draft),
        ) else {
            return;
        };
        if k.key() == "Enter" && el.has_attribute("data-hex-input") && !k.is_composing() {
            k.prevent_default();
            save();
            return;
        }
        if closest(&el, "[data-sv-surface]").is_none() {
            return;
        }
        let delta = if k.shift_key() { 0.10 } else { 0.01 };
        let selector = match k.key().as_str() {
            "ArrowRight" => {
                hsv.s = (hsv.s + delta).min(1.);
                "[data-saturation]"
            }
            "ArrowLeft" => {
                hsv.s = (hsv.s - delta).max(0.);
                "[data-saturation]"
            }
            "ArrowUp" => {
                hsv.v = (hsv.v + delta).min(1.);
                "[data-brightness]"
            }
            "ArrowDown" => {
                hsv.v = (hsv.v - delta).max(0.);
                "[data-brightness]"
            }
            _ => return,
        };
        k.prevent_default();
        render_editor(hsv, false);
        if let Some(i) = q(selector) {
            focus(&i);
        }
    })?;
    for name in [
        "pointerdown",
        "pointermove",
        "pointerup",
        "lostpointercapture",
    ] {
        on(doc.as_ref(), name, false, move |e| {
            let Some(p) = e.dyn_ref::<PointerEvent>() else {
                return;
            };
            let Some(plane) = target(&e).and_then(|t| closest(&t, "[data-sv-surface]")) else {
                return;
            };
            if name == "pointerdown" {
                if !p.is_primary() || p.button() != 0 {
                    return;
                }
                p.prevent_default();
                let _ = plane.set_pointer_capture(p.pointer_id());
                STATE.with_borrow_mut(|s| s.pointer = Some(p.pointer_id()));
                if let Some(i) = q("[data-saturation]") {
                    focus(&i);
                }
            }
            if STATE.with_borrow(|s| s.pointer != Some(p.pointer_id())) {
                return;
            }
            if name != "lostpointercapture" {
                move_pointer(p, &plane);
            }
            if matches!(name, "pointerup" | "lostpointercapture") {
                STATE.with_borrow_mut(|s| s.pointer = None);
            }
        })?;
    }
    on(window().as_ref(), "storage", false, |e| {
        let key = prop(e.as_ref(), "key");
        if key.is_null()
            || matches!(
                key.as_string().as_deref(),
                Some("kui-accent" | "kui-custom-accent")
            )
        {
            dismiss(false);
            if key.is_null() || stored("kui-accent").is_none() {
                attr(&root(), "data-accent", "neutral");
            }
            read_saved();
        }
    })?;
    Ok(())
}
