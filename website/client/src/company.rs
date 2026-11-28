//! Browser interactions, WebGL studio 3D solids, liquid dock, spring belt, and companion physics.

mod belt;
mod dock;
mod dots;
mod floor;
mod fusion;
mod guide;
mod heat_flow;
mod intro;
mod nav;
mod solids;
mod tilt;
mod workbench;

use crate::cover::{Result, listen};
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::Document;

/// Registers interactive behaviors for Kense company pages (`/` and `/docs/`).
pub fn init() -> Result {
    let window = web_sys::window().ok_or("window")?;
    let document: Document = window.document().ok_or("document")?;
    let reduced = window
        .match_media("(prefers-reduced-motion: reduce)")?
        .ok_or("matchMedia")?;

    // Header mega-popovers & product badge preview cards run on both `/` and `/docs/`.
    nav::init(&window, &document, &reduced)?;

    let Some(root) = document.query_selector(".kense-home")? else {
        return Ok(());
    };
    if root.has_attribute("data-initialized") {
        return Ok(());
    }
    root.set_attribute("data-initialized", "")?;

    let intro_playing = intro::init(&window, &document, &root, &reduced)?;
    workbench::init(&window, &document, &root, &reduced)?;
    tilt::init(&window, &document, &root, &reduced)?;
    guide::init(&window, &document, &root, &reduced)?;

    let boot_ambient = {
        let booted = Rc::new(Cell::new(false));
        let win = window.clone();
        let doc = document.clone();
        let root = root.clone();
        let reduced = reduced.clone();
        move || {
            if booted.replace(true) {
                return;
            }
            let _ = floor::init(&win, &doc, &root, &reduced);
            let _ = belt::init(&win, &doc, &root, &reduced);
            let _ = heat_flow::init(&win, &doc, &root, &reduced);
            let _ = dock::init(&win, &doc, &root, &reduced);

            // Stagger below-the-fold WebGL 3D solids & dot field onto the next idle turn
            let win2 = win.clone();
            let doc2 = doc.clone();
            let root2 = root.clone();
            let reduced2 = reduced.clone();
            let cb = Closure::once_into_js(move || {
                let _ = solids::init(&win2, &doc2, &root2, &reduced2);
                let _ = dots::init(&win2, &doc2, &root2, &reduced2);
            });
            let _ =
                win.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 48);
        }
    };

    if intro_playing {
        let boot = boot_ambient.clone();
        listen(&document, "kense:intro-done", move |_| boot())?;
    } else {
        boot_ambient();
    }

    Ok(())
}
