//! Interactive HiDPI architectural dot-matrix cursor field for `#principles`.

use crate::cover::{Frame, Result, listen};
use js_sys::{Array, Reflect};
use std::{cell::RefCell, f64::consts::TAU, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{
    CanvasRenderingContext2d, Document, Element, HtmlCanvasElement, IntersectionObserver,
    IntersectionObserverEntry, MediaQueryList, PointerEvent, ResizeObserver, Window,
};

struct DotField {
    window: Window,
    document: Document,
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    ptr: Option<(f64, f64)>,
    smooth_ptr: (f64, f64),
    strength: f64,
    in_view: bool,
    force_paint: bool,
    last_ms: f64,
}

impl DotField {
    fn sync_size(&mut self) -> (f64, f64, f64) {
        let rect = self.canvas.get_bounding_client_rect();
        let w = rect.width().max(320.0);
        let h = rect.height().max(240.0);
        let dpr = self.window.device_pixel_ratio().clamp(1.0, 2.0);
        let pw = (w * dpr).round() as u32;
        let ph = (h * dpr).round() as u32;
        if self.canvas.width() != pw || self.canvas.height() != ph {
            self.canvas.set_width(pw);
            self.canvas.set_height(ph);
        }
        (w, h, dpr)
    }

    fn paint(&mut self) {
        let (w, h, dpr) = self.sync_size();
        let _ = self.ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
        self.ctx.clear_rect(0.0, 0.0, w, h);

        let dark = self
            .document
            .document_element()
            .and_then(|e| e.get_attribute("data-theme"))
            .as_deref()
            == Some("dark");

        let base_fill = if dark {
            "rgba(255,255,255,0.085)"
        } else {
            "rgba(18,19,22,0.075)"
        };
        let active_fill = if dark {
            "rgba(255,255,255,0.34)"
        } else {
            "rgba(18,19,22,0.28)"
        };

        let spacing = 20.0;
        let cols = (w / spacing).ceil() as i32;
        let rows = (h / spacing).ceil() as i32;
        let cx = w * 0.5;
        let cy = h * 0.42;

        // Pass 1: Crisp ambient dot lattice with subtle central emblem modulation
        let _ = Reflect::set(
            self.ctx.as_ref(),
            &JsValue::from_str("fillStyle"),
            &JsValue::from_str(base_fill),
        );
        self.ctx.begin_path();
        for r in 0..=rows {
            let y = f64::from(r) * spacing + 10.0;
            for c in 0..=cols {
                let x = f64::from(c) * spacing + 10.0;
                let dx_c = (x - cx) / 150.0;
                let dy_c = (y - cy) / 110.0;
                let ring = (dx_c.hypot(dy_c) - 1.0).abs();
                let rad = if ring < 0.14 { 1.45 } else { 1.0 };

                let mut px = x;
                let mut py = y;
                if self.strength > 0.01 {
                    let dx = x - self.smooth_ptr.0;
                    let dy = y - self.smooth_ptr.1;
                    let d2 = dx * dx + dy * dy;
                    if d2 < 25600.0 && d2 > 1.0 {
                        let d = d2.sqrt();
                        let push = (1.0 - d / 160.0).powi(2) * 4.5 * self.strength;
                        px += dx / d * push;
                        py += dy / d * push;
                    }
                }
                self.ctx.move_to(px + rad, py);
                let _ = self.ctx.arc(px, py, rad, 0.0, TAU);
            }
        }
        self.ctx.fill();

        // Pass 2: Cursor-illuminated dots around pointer
        if self.strength > 0.02 {
            let _ = Reflect::set(
                self.ctx.as_ref(),
                &JsValue::from_str("fillStyle"),
                &JsValue::from_str(active_fill),
            );
            self.ctx.begin_path();
            let c_min = ((self.smooth_ptr.0 - 165.0) / spacing).floor().max(0.0) as i32;
            let c_max = ((self.smooth_ptr.0 + 165.0) / spacing)
                .ceil()
                .min(f64::from(cols)) as i32;
            let r_min = ((self.smooth_ptr.1 - 165.0) / spacing).floor().max(0.0) as i32;
            let r_max = ((self.smooth_ptr.1 + 165.0) / spacing)
                .ceil()
                .min(f64::from(rows)) as i32;

            for r in r_min..=r_max {
                let y = f64::from(r) * spacing + 10.0;
                for c in c_min..=c_max {
                    let x = f64::from(c) * spacing + 10.0;
                    let dx = x - self.smooth_ptr.0;
                    let dy = y - self.smooth_ptr.1;
                    let d = dx.hypot(dy);
                    if d < 160.0 {
                        let w_f = (1.0 - d / 160.0).powi(2) * self.strength;
                        let push = w_f * 4.5;
                        let (px, py) = if d > 1.0 {
                            (x + dx / d * push, y + dy / d * push)
                        } else {
                            (x, y)
                        };
                        let rad = 1.1 + w_f * 1.35;
                        self.ctx.move_to(px + rad, py);
                        let _ = self.ctx.arc(px, py, rad, 0.0, TAU);
                    }
                }
            }
            self.ctx.fill();
        }
    }

    fn tick(&mut self, now: f64, reduced: bool) -> bool {
        if self.document.hidden() || !self.canvas.is_connected() {
            self.last_ms = 0.0;
            return false;
        }
        if reduced {
            self.strength = 0.0;
            if self.force_paint {
                self.force_paint = false;
                self.paint();
            }
            return false;
        }
        let dt = if self.last_ms == 0.0 {
            0.016
        } else {
            ((now - self.last_ms) / 1000.0).clamp(0.001, 0.05)
        };
        self.last_ms = now;

        let target_str = if self.in_view && self.ptr.is_some() {
            1.0
        } else {
            0.0
        };
        let blend = 1.0 - (-12.0 * dt).exp();
        self.strength += (target_str - self.strength) * blend;
        if let Some((tx, ty)) = self.ptr {
            self.smooth_ptr.0 += (tx - self.smooth_ptr.0) * blend;
            self.smooth_ptr.1 += (ty - self.smooth_ptr.1) * blend;
        }

        let need_paint = self.force_paint
            || (self.in_view && (self.strength - target_str).abs() > 0.005)
            || (self.in_view
                && self.ptr.is_some_and(|(tx, ty)| {
                    (tx - self.smooth_ptr.0).hypot(ty - self.smooth_ptr.1) > 0.4
                }));
        self.force_paint = false;
        if need_paint {
            self.paint();
        }
        if !need_paint {
            self.last_ms = 0.0;
        }
        need_paint
    }
}

pub(super) fn init(
    window: &Window,
    document: &Document,
    root: &Element,
    reduced: &MediaQueryList,
) -> Result {
    let Some(el) = root.query_selector("[data-kense-dots]")? else {
        return Ok(());
    };
    let Ok(canvas) = el.dyn_into::<HtmlCanvasElement>() else {
        return Ok(());
    };
    let Some(section) = canvas.parent_element() else {
        return Ok(());
    };
    let Some(ctx) = canvas
        .get_context("2d")?
        .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
    else {
        return Ok(());
    };

    let state = Rc::new(RefCell::new(DotField {
        window: window.clone(),
        document: document.clone(),
        canvas: canvas.clone(),
        ctx,
        ptr: None,
        smooth_ptr: (0.0, 0.0),
        strength: 0.0,
        in_view: false,
        force_paint: true,
        last_ms: 0.0,
    }));

    let frame = Frame::new(window, {
        let state = state.clone();
        let media = reduced.clone();
        move |now| state.borrow_mut().tick(now, media.matches())
    });

    state.borrow_mut().paint();

    let io_cb = Closure::<dyn FnMut(Array)>::new({
        let state = state.clone();
        let frame = frame.clone();
        move |entries: Array| {
            let mut s = state.borrow_mut();
            for item in entries.iter() {
                if let Ok(entry) = item.dyn_into::<IntersectionObserverEntry>() {
                    s.in_view = entry.is_intersecting();
                    if s.in_view {
                        s.force_paint = true;
                    }
                }
            }
            frame.request();
        }
    });
    let io = IntersectionObserver::new(io_cb.as_ref().unchecked_ref())?;
    io.observe(&section);
    io_cb.forget();

    let ro_cb = Closure::<dyn FnMut()>::new({
        let state = state.clone();
        let frame = frame.clone();
        move || {
            state.borrow_mut().force_paint = true;
            frame.request();
        }
    });
    let ro = ResizeObserver::new(ro_cb.as_ref().unchecked_ref())?;
    ro.observe(&canvas);
    ro_cb.forget();

    listen(&section, "pointermove", {
        let state = state.clone();
        let frame = frame.clone();
        move |ev| {
            let Some(pe) = ev.dyn_ref::<PointerEvent>() else {
                return;
            };
            if pe.pointer_type() == "touch" {
                return;
            }
            let mut s = state.borrow_mut();
            let r = s.canvas.get_bounding_client_rect();
            let x = f64::from(pe.client_x()) - r.left();
            let y = f64::from(pe.client_y()) - r.top();
            if y >= -20.0 && y <= r.height() + 40.0 {
                if s.ptr.is_none() {
                    s.smooth_ptr = (x, y);
                }
                s.ptr = Some((x, y));
            } else {
                s.ptr = None;
            }
            frame.request();
        }
    })?;

    listen(&section, "pointerleave", {
        let state = state.clone();
        let frame = frame.clone();
        move |_| {
            state.borrow_mut().ptr = None;
            frame.request();
        }
    })?;

    for ev_name in ["kui:theme-change", "kui:accent-change"] {
        let state = state.clone();
        let frame = frame.clone();
        listen(window, ev_name, move |_| {
            state.borrow_mut().force_paint = true;
            frame.request();
        })?;
    }

    Ok(())
}
