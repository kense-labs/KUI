//! The reversible ticket reel and the refracted spectrum painted behind it.
use super::{Frame, Result, css, element, flag, listen, viewport};
use std::{
    cell::RefCell,
    f64::consts::{PI, TAU},
    rc::Rc,
};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{
    CanvasRenderingContext2d, Document, Element, HtmlCanvasElement, HtmlElement, HtmlImageElement,
    HtmlInputElement, IntersectionObserver, IntersectionObserverEntry, MediaQueryList,
    PointerEvent, Window,
};

struct Ribbon {
    image: HtmlImageElement,
    canvas: HtmlCanvasElement,
    context: Option<CanvasRenderingContext2d>,
    seconds: f64,
    previous: Option<f64>,
}

impl Ribbon {
    fn paint(&mut self, now: f64) {
        let Some(ctx) = &self.context else { return };
        if !self.image.complete() || self.image.natural_width() == 0 {
            return;
        }
        let elapsed = self.previous.map(|last| now - last);
        if elapsed.is_some_and(|ms| ms < 1000.0 / 60.0 - 1.0) {
            return;
        }
        self.previous = Some(now);
        self.seconds += elapsed.unwrap_or_default().min(80.0) / 1000.0;
        let width = f64::from(self.canvas.width());
        let height = f64::from(self.canvas.height());
        let source_width = f64::from(self.image.natural_width());
        let source_height = f64::from(self.image.natural_height());
        let phase = self.seconds * TAU / 42.0;
        let entry = (self.seconds / 2.4).min(1.0);
        let strength = entry * entry * (3.0 - 2.0 * entry);
        let _ = ctx.reset_transform();
        ctx.clear_rect(0.0, 0.0, width, height);
        let mut source_left = 0.0;
        let mut lift_left = 0.0;
        const SLICES: u32 = 24;
        let step = width / f64::from(SLICES);
        // Share each slice endpoint with its neighbour so the highlight stays continuous.
        for i in 1..=SLICES {
            let u = f64::from(i) / f64::from(SLICES);
            let envelope = strength * (PI * u).sin();
            let swell = (TAU * u - 2.0 * phase).sin();
            let ripple = (1.65 * TAU * u + phase + 0.7).sin();
            let source_right = source_width * (u + 0.042 * envelope * ripple);
            let lift_right = height * envelope * (0.061 * swell + 0.027 * ripple);
            let thickness = 1.0 + 0.13 * envelope * (TAU * u - 2.0 * phase).cos();
            let x = f64::from(i - 1) * step;
            let slope = (lift_right - lift_left) / step;
            let centre = height * (0.28 + 0.47 * (PI * u).sin());
            let _ = ctx.set_transform(
                1.0,
                slope,
                0.0,
                thickness,
                0.0,
                lift_left - slope * x + centre * (1.0 - thickness),
            );
            let _ = ctx
                .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &self.image,
                    source_left,
                    0.0,
                    source_right - source_left,
                    source_height,
                    x,
                    0.0,
                    step,
                    height,
                );
            source_left = source_right;
            lift_left = lift_right;
        }
        self.canvas.set_hidden(false);
    }
}

struct Motion {
    page: Document,
    cover: HtmlElement,
    media: MediaQueryList,
    transport: HtmlElement,
    tickets: Vec<(HtmlElement, f64)>,
    radios: [HtmlInputElement; 2],
    plate: HtmlElement,
    thumb: HtmlElement,
    ribbon: Ribbon,
    playing: bool,
    hover: bool,
    visible: bool,
    destination: f64,
    position: f64,
    velocity: f64,
    previous: Option<f64>,
    next_change: Option<f64>,
}

impl Motion {
    fn draw(&self) {
        for (ticket, slot) in &self.tickets {
            let offset = (slot - self.position + 2.0).rem_euclid(4.0) - 2.0;
            let d = offset.abs();
            css(
                ticket,
                "transform",
                &format!(
                    "translate3d(0,{:.3}%,0) scale({:.4}) rotateX({:.2}deg)",
                    offset * 105.0,
                    1.0 - 0.11 * d,
                    -14.0 * offset
                ),
            );
            css(
                ticket,
                "opacity",
                &format!(
                    "{:.4}",
                    (-1.72 * d * d).exp() * ((1.65 - d) * 3.0).clamp(0.0, 1.0)
                ),
            );
        }
        let web = (1.0 - (self.position * PI).cos()) / 2.0;
        css(
            &self.thumb,
            "transform",
            &format!("translateX({:.3}%)", web * 100.0),
        );
        css(
            &self.plate,
            "transform",
            &format!(
                "translate3d({:.3}%,{:.3}%,0) rotate({:.3}deg)",
                -0.5 * web,
                0.55 * web,
                1.2 * web
            ),
        );
    }

    fn tick(&mut self, now: f64) -> bool {
        let reduced = self.media.matches();
        let active = !reduced && self.visible && !self.page.hidden();
        flag(&self.cover, "data-paused", !self.playing || reduced);
        let label = if self.playing {
            "data-pause"
        } else {
            "data-play"
        };
        if let Some(s) = self.transport.get_attribute(label)
            && self.transport.get_attribute("aria-label").as_deref() != Some(&s)
        {
            let _ = self.transport.set_attribute("aria-label", &s);
        }
        if active && self.playing && !self.hover {
            let next = self.next_change.get_or_insert(now + 3250.0);
            if now >= *next {
                self.destination += 1.0;
                self.radios[self.destination.rem_euclid(2.0) as usize].set_checked(true);
                *next = now + 3250.0;
            }
        } else {
            self.next_change = None;
        }
        if active && self.playing {
            self.ribbon.paint(now);
        } else {
            self.ribbon.previous = None;
            if reduced {
                self.ribbon.canvas.set_hidden(true);
                self.ribbon.seconds = 0.0;
            }
        }
        let dt = (now - self.previous.unwrap_or(now)).max(0.0) / 1000.0;
        let offset = self.position - self.destination;
        if offset != 0.0 || self.velocity != 0.0 {
            let slope = self.velocity + 13.8 * offset;
            let decay = (-13.8 * dt).exp();
            self.position = self.destination + (offset + slope * dt) * decay;
            self.velocity = (self.velocity - 13.8 * slope * dt) * decay;
            if !active
                || ((self.position - self.destination).abs() < 0.001 && self.velocity.abs() < 0.014)
            {
                self.destination = self.destination.rem_euclid(4.0);
                self.position = self.destination;
                self.velocity = 0.0;
            }
            self.draw();
        }
        let animating = active && (self.playing || self.position != self.destination);
        self.previous = animating.then_some(now);
        animating
    }
}

pub(super) fn init(
    window: &Window,
    page: &Document,
    root: &Element,
    reduced: &MediaQueryList,
) -> Result {
    let cover: HtmlElement = element(root, ".cover")?;
    let canvas: HtmlCanvasElement = element(&cover, "canvas")?;
    let context = canvas.get_context("2d")?.and_then(|c| c.dyn_into().ok());
    let ribbon = Ribbon {
        image: element(&cover, ".light-field img")?,
        canvas,
        context,
        seconds: 0.0,
        previous: None,
    };
    let slots = cover.query_selector_all("[data-slot]")?;
    let tickets = (0..slots.length())
        .filter_map(|i| {
            let el: HtmlElement = slots.item(i)?.dyn_into().ok()?;
            let slot = el.get_attribute("data-slot")?.parse().ok()?;
            Some((el, slot))
        })
        .collect();
    let radios: [HtmlInputElement; 2] = [
        element(&cover, "#cover-desktop")?,
        element(&cover, "#cover-web")?,
    ];
    let transport: HtmlElement = element(&cover, ".transport")?;
    let position = f64::from(u8::from(radios[1].checked()));
    let state = Rc::new(RefCell::new(Motion {
        page: page.clone(),
        cover: cover.clone(),
        media: reduced.clone(),
        transport: transport.clone(),
        tickets,
        radios: radios.clone(),
        plate: element(&cover, ".plate")?,
        thumb: element(&cover, ".mode-thumb")?,
        ribbon,
        playing: true,
        hover: false,
        visible: true,
        destination: position,
        position,
        velocity: 0.0,
        previous: None,
        next_change: None,
    }));
    state.borrow().draw();
    let frame = Frame::new(window, {
        let state = state.clone();
        move |now| state.borrow_mut().tick(now)
    });
    for (i, radio) in radios.iter().enumerate() {
        let (state, frame) = (state.clone(), frame.clone());
        listen(radio, "change", move |_| {
            let mut s = state.borrow_mut();
            let i = i as f64;
            let left = i + 2.0 * ((s.position - i) / 2.0).floor();
            let distance = s.position - left;
            s.destination = if distance < 1.0 || (distance == 1.0 && i == 0.0) {
                left
            } else {
                left + 2.0
            };
            s.playing = false;
            frame.request();
        })?;
    }
    listen(&transport, "click", {
        let (s, f) = (state.clone(), frame.clone());
        move |_| {
            let mut m = s.borrow_mut();
            m.playing = !m.playing;
            f.request();
        }
    })?;
    for selector in [".reel", ".modes"] {
        let el: Element = element(&cover, selector)?;
        for (event, hover) in [("pointerenter", true), ("pointerleave", false)] {
            let (s, f) = (state.clone(), frame.clone());
            listen(&el, event, move |_| {
                let mut m = s.borrow_mut();
                m.hover = hover;
                m.next_change = None;
                f.request();
            })?;
        }
    }
    listen(root, "focusin", {
        let (state, frame) = (state.clone(), frame.clone());
        move |event| {
            if let Some(el) = event.target().and_then(|t| t.dyn_into::<Element>().ok())
                && !el.class_list().contains("transport")
                && el.closest(".cover,.guide").ok().flatten().is_some()
            {
                state.borrow_mut().playing = false;
                frame.request();
            }
        }
    })?;
    if window
        .match_media("(hover: hover) and (pointer: fine)")?
        .is_some_and(|m| m.matches())
    {
        let optics: HtmlElement = element(&cover, ".optics")?;
        listen(&cover, "pointermove", {
            let (cover, optics, reduced) = (cover.clone(), optics.clone(), reduced.clone());
            move |event| {
                let Some(e) = event.dyn_ref::<PointerEvent>() else {
                    return;
                };
                if e.pointer_type() == "touch" || reduced.matches() {
                    return;
                }
                let r = cover.get_bounding_client_rect();
                let x = 24.0 * ((f64::from(e.client_x()) - r.left()) / r.width() - 0.5);
                let y = 16.0 * ((f64::from(e.client_y()) - r.top()) / r.height() - 0.5);
                css(
                    &optics,
                    "transform",
                    &format!("translate3d({x:.1}px,{y:.1}px,0)"),
                );
            }
        })?;
        listen(&cover, "pointerleave", move |_| {
            css(&optics, "transform", "")
        })?;
    }
    for (target, event) in [
        (page.as_ref(), "visibilitychange"),
        (reduced.as_ref(), "change"),
    ] {
        let f = frame.clone();
        listen(target, event, move |_| f.request())?;
    }
    let observer_callback = Closure::<dyn FnMut(js_sys::Array, IntersectionObserver)>::new({
        let state = state.clone();
        let frame = frame.clone();
        let reduced = reduced.clone();
        let hero = cover.clone();
        move |entries: js_sys::Array, observer: IntersectionObserver| {
            for entry in entries
                .iter()
                .filter_map(|v| v.dyn_into::<IntersectionObserverEntry>().ok())
            {
                let target = entry.target();
                if target == *hero {
                    state.borrow_mut().visible = entry.is_intersecting();
                    frame.request();
                } else if entry.is_intersecting() {
                    observer.unobserve(&target);
                    if !reduced.matches() && !target.matches(":focus-within").unwrap_or(false) {
                        let _ = target.set_attribute("data-arrive", "");
                    }
                }
            }
        }
    });
    let observer = IntersectionObserver::new(observer_callback.as_ref().unchecked_ref())?;
    observer.observe(&cover);
    let (_, vh) = viewport(window);
    let arrivals = root.query_selector_all(".section-head,.sample,.start-copy,.starter-prompt")?;
    for n in 0..arrivals.length() {
        if let Some(el) = arrivals.item(n).and_then(|v| v.dyn_into::<Element>().ok())
            && el.get_bounding_client_rect().top() >= vh
        {
            observer.observe(&el);
        }
    }
    observer_callback.forget();
    flag(&cover, "data-ready", true);
    frame.request();
    Ok(())
}
