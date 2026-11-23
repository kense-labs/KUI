//! Browser interactions, WebGL studio 3D solids, liquid dock, spring belt, and companion physics.

mod belt;
mod dock;
mod dots;
mod fusion;
mod guide;
mod heat_flow;
mod intro;
mod nav;
mod solids;
mod tilt;
mod workbench;

use crate::cover::Result;
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
    intro::init(&window, &document, &root, &reduced)?;
    belt::init(&window, &document, &root, &reduced)?;
    heat_flow::init(&window, &document, &root, &reduced)?;
    dock::init(&window, &document, &root, &reduced)?;
    solids::init(&window, &document, &root, &reduced)?;
    workbench::init(&window, &document, &root, &reduced)?;
    tilt::init(&window, &document, &root, &reduced)?;
    dots::init(&window, &document, &root, &reduced)?;
    guide::init(&window, &document, &root, &reduced)?;
    root.set_attribute("data-initialized", "")
}
