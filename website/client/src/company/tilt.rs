//! Smooth 3D perspective tilt for borderless `#capabilities` porcelain plinths.

use crate::cover::{Frame, Result, css, element, listen};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, MediaQueryList, PointerEvent, Window};

struct CardTilt {
    surface: HtmlElement,
    cur_x: f64,
    cur_y: f64,
    cur_hover: f64,
    goal_x: f64,
    goal_y: f64,
    goal_hover: f64,
}

impl CardTilt {
    fn reset(&mut self) {
        self.cur_x = 0.0;
        self.cur_y = 0.0;
        self.cur_hover = 0.0;
        self.goal_x = 0.0;
        self.goal_y = 0.0;
        self.goal_hover = 0.0;
        for prop in ["--tilt-rx", "--tilt-ry", "--tilt-lift"] {
            let _ = self.surface.style().remove_property(prop);
        }
    }

    fn step(&mut self, dt: f64) -> bool {
        let alpha = 1.0 - (-16.0 * dt).exp();
        self.cur_x += (self.goal_x - self.cur_x) * alpha;
        self.cur_y += (self.goal_y - self.cur_y) * alpha;
        self.cur_hover += (self.goal_hover - self.cur_hover) * alpha;

        let settled = (self.goal_x - self.cur_x).abs() < 0.002
            && (self.goal_y - self.cur_y).abs() < 0.002
            && (self.goal_hover - self.cur_hover).abs() < 0.003;

        if settled && self.goal_hover == 0.0 {
            self.reset();
            return false;
        }

        let rx = -self.cur_y * 4.4 * self.cur_hover;
        let ry = self.cur_x * 4.4 * self.cur_hover;
        let lift = -8.0 * self.cur_hover;

        css(&self.surface, "--tilt-rx", &format!("{rx:.3}deg"));
        css(&self.surface, "--tilt-ry", &format!("{ry:.3}deg"));
        css(&self.surface, "--tilt-lift", &format!("{lift:.2}px"));
        !settled
    }
}

pub(super) fn init(
    window: &Window,
    _document: &Document,
    root: &Element,
    reduced: &MediaQueryList,
) -> Result {
    let nodes = root.query_selector_all("[data-kense-tilt]")?;
    for i in 0..nodes.length() {
        let Some(card) = nodes.item(i).and_then(|n| n.dyn_into::<Element>().ok()) else {
            continue;
        };
        let Ok(surface) = element::<HtmlElement>(&card, "[data-kense-tilt-surface]") else {
            continue;
        };
        let state = Rc::new(RefCell::new((
            CardTilt {
                surface,
                cur_x: 0.0,
                cur_y: 0.0,
                cur_hover: 0.0,
                goal_x: 0.0,
                goal_y: 0.0,
                goal_hover: 0.0,
            },
            0.0_f64,
        )));

        let frame = Frame::new(window, {
            let state = state.clone();
            let media = reduced.clone();
            move |now| {
                let mut guard = state.borrow_mut();
                if media.matches() {
                    guard.0.reset();
                    guard.1 = 0.0;
                    return false;
                }
                let dt = if guard.1 == 0.0 {
                    0.016
                } else {
                    ((now - guard.1) / 1000.0).clamp(0.001, 0.05)
                };
                guard.1 = now;
                let keep = guard.0.step(dt);
                if !keep {
                    guard.1 = 0.0;
                }
                keep
            }
        });

        listen(&card, "pointermove", {
            let card = card.clone();
            let state = state.clone();
            let frame = frame.clone();
            let media = reduced.clone();
            move |ev| {
                if media.matches() {
                    return;
                }
                let Some(p) = ev.dyn_ref::<PointerEvent>() else {
                    return;
                };
                if p.pointer_type() == "touch" {
                    return;
                }
                let r = card.get_bounding_client_rect();
                if r.width() <= 1.0 || r.height() <= 1.0 {
                    return;
                }
                let px = ((f64::from(p.client_x()) - r.left()) / r.width()).clamp(0.0, 1.0);
                let py = ((f64::from(p.client_y()) - r.top()) / r.height()).clamp(0.0, 1.0);
                let mut guard = state.borrow_mut();
                guard.0.goal_x = (px - 0.5) * 2.0;
                guard.0.goal_y = (py - 0.5) * 2.0;
                guard.0.goal_hover = 1.0;
                frame.request();
            }
        })?;

        for name in ["pointerleave", "pointercancel"] {
            listen(&card, name, {
                let state = state.clone();
                let frame = frame.clone();
                move |_| {
                    let mut guard = state.borrow_mut();
                    guard.0.goal_x = 0.0;
                    guard.0.goal_y = 0.0;
                    guard.0.goal_hover = 0.0;
                    frame.request();
                }
            })?;
        }
    }
    Ok(())
}
