//! A fixed, inert copy of the outgoing viewport for reversible theme reveals.

use super::*;
use web_sys::{Animation, CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement, Node};

#[derive(Clone, Copy)]
pub(super) struct Opening {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    radius: [f64; 4],
}
impl Opening {
    pub fn from_element(el: &Element) -> Self {
        let r = el.get_bounding_client_rect();
        let style = window().get_computed_style(el).ok().flatten();
        let radius = [
            "border-top-left-radius",
            "border-top-right-radius",
            "border-bottom-right-radius",
            "border-bottom-left-radius",
        ]
        .map(|key| {
            style
                .as_ref()
                .and_then(|s| s.get_property_value(key).ok())
                .and_then(|v| v.trim_end_matches("px").parse::<f64>().ok())
                .unwrap_or_default()
        });
        Self {
            x: r.x(),
            y: r.y(),
            w: r.width(),
            h: r.height(),
            radius,
        }
    }
    pub fn in_frame(frame: &HtmlIFrameElement, x: f64, y: f64, w: f64, h: f64) -> Self {
        let r = frame.get_bounding_client_rect();
        let radius = preview(frame, "controlRadius", &Array::new())
            .and_then(|v| v.as_f64())
            .unwrap_or(8.);
        let width = f64::from(frame.client_width());
        let scale = if width > 0. { r.width() / width } else { 1. };
        Self {
            x: r.x() + x * scale,
            y: r.y() + y * scale,
            w: w * scale,
            h: h * scale,
            radius: [radius * scale; 4],
        }
    }
    fn mask(self) -> String {
        let [tl, tr, br, bl] = self.radius.map(|n| n.max(0.).min(self.w.min(self.h) / 2.));
        let (l, t, r, b) = (self.x, self.y, self.x + self.w, self.y + self.h);
        format!(
            r#"<path fill="black" d="M{} {t}H{}Q{r} {t} {r} {}V{}Q{r} {b} {} {b}H{}Q{l} {b} {l} {}V{}Q{l} {t} {} {t}Z"/>"#,
            l + tl,
            r - tr,
            t + tr,
            b - br,
            r - br,
            l + bl,
            b - bl,
            t + tl,
            l + tl
        )
    }
}
pub(super) fn visible(el: &Element) -> bool {
    let r = el.get_bounding_client_rect();
    !document().hidden()
        && r.width() > 0.
        && r.height() > 0.
        && r.bottom() > 0.
        && r.right() > 0.
        && r.top()
            < window()
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or_default()
        && r.left() < f64::from(root().client_width())
        && closest(el, "details:not([open]),[hidden]").is_none()
}

#[derive(Default)]
struct Snapshot {
    scroll: Vec<(Element, i32, i32)>,
    holes: Vec<Opening>,
}
impl Snapshot {
    fn copy(&mut self, src: &Node) -> Result<Option<Node>, JsValue> {
        if src.node_type() == Node::TEXT_NODE {
            return src.clone_node().map(Some);
        }
        if src.node_type() != Node::ELEMENT_NODE {
            return Ok(None);
        }
        let el: &Element = src.unchecked_ref();
        if matches!(
            el.tag_name().as_str(),
            "SCRIPT" | "STYLE" | "LINK" | "TEMPLATE"
        ) || el.has_attribute("data-kui-surface")
        {
            return Ok(None);
        }
        if is_frame(el) {
            let f: &HtmlIFrameElement = el.unchecked_ref();
            let c = document().create_element("canvas")?;
            for key in ["class", "style"] {
                if let Some(v) = f.get_attribute(key) {
                    attr(&c, key, &v)
                }
            }
            if visible(f) {
                let r = f.get_bounding_client_rect();
                css(&c, "width", &format!("{}px", r.width()));
                css(&c, "height", &format!("{}px", r.height()));
                if !preview(f, "copyPicture", &Array::of1(c.as_ref()))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    self.holes.push(Opening::from_element(f));
                }
            }
            return Ok(Some(c.into()));
        }
        let copy: Element = src.clone_node()?.unchecked_into();
        remove(&copy, "id");
        remove(&copy, "autofocus");
        if let (Some(a), Some(b)) = (
            el.dyn_ref::<HtmlInputElement>(),
            copy.dyn_ref::<HtmlInputElement>(),
        ) {
            b.set_value(&a.value());
            b.set_checked(a.checked());
        }
        if let (Some(a), Some(b)) = (
            el.dyn_ref::<HtmlCanvasElement>(),
            copy.dyn_ref::<HtmlCanvasElement>(),
        ) && let Ok(Some(ctx)) = b.get_context("2d")
            && let Ok(ctx) = ctx.dyn_into::<CanvasRenderingContext2d>()
        {
            let _ = ctx.draw_image_with_html_canvas_element(a, 0., 0.);
        }
        if el.scroll_top() != 0 || el.scroll_left() != 0 {
            self.scroll
                .push((copy.clone(), el.scroll_left(), el.scroll_top()));
        }
        if !el.has_attribute("hidden") {
            let closed = el.tag_name() == "DETAILS" && !el.has_attribute("open");
            let mut child = src.first_child();
            while let Some(n) = child {
                child = n.next_sibling();
                if closed && n.node_name() != "SUMMARY" {
                    continue;
                }
                if let Some(node) = self.copy(&n)? {
                    copy.append_child(&node)?;
                }
            }
        }
        Ok(Some(copy.into()))
    }
}
pub(super) struct Surface {
    pub layer: Element,
    pub viewport: Element,
}
impl Surface {
    pub fn freeze(from: &str, opening: Option<Opening>) -> Result<Self, JsValue> {
        let doc = document();
        let body = doc.body().ok_or(JsValue::NULL)?;
        let layer = doc.create_element("div")?;
        attr(&layer, "data-kui-surface", from);
        attr(&layer, "data-kui-frozen", from);
        attr(&layer, "inert", "");
        attr(&layer, "aria-hidden", "true");
        let viewport = doc.create_element("div")?;
        viewport.set_class_name("kui-frozen-viewport");
        let mut snapshot = Snapshot::default();
        let children = body.children();
        for i in 0..children.length() {
            if let Some(el) = children.item(i)
                && let Some(node) = snapshot.copy(&el)?
            {
                viewport.append_child(&node)?;
            }
        }
        snapshot.holes.extend(opening);
        if !snapshot.holes.is_empty() {
            let paths = snapshot
                .holes
                .into_iter()
                .map(Opening::mask)
                .collect::<String>();
            let h = window().inner_height()?.as_f64().unwrap_or(1.);
            let svg = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {h}"><mask id="cuts"><rect width="100%" height="100%" fill="white"/>{paths}</mask><rect width="100%" height="100%" fill="white" mask="url(#cuts)"/></svg>"##,
                root().client_width()
            );
            css(
                &viewport,
                "mask-image",
                &format!(
                    r#"url("data:image/svg+xml,{}")"#,
                    js_sys::encode_uri_component(&svg)
                ),
            );
        }
        layer.append_child(&viewport)?;
        body.append_child(&layer)?;
        viewport.scroll_to_with_x_and_y(
            window().scroll_x().unwrap_or(0.),
            window().scroll_y().unwrap_or(0.),
        );
        for (el, x, y) in snapshot.scroll {
            el.set_scroll_left(x);
            el.set_scroll_top(y);
        }
        Ok(Self { layer, viewport })
    }
    fn animate(
        el: &Element,
        key: &str,
        from: &str,
        to: &str,
        duration: f64,
    ) -> Result<Animation, JsValue> {
        let keyframes = js_sys::Object::new();
        Reflect::set(
            &keyframes,
            &key.into(),
            &Array::of2(&from.into(), &to.into()),
        )?;
        let options = js_sys::Object::new();
        Reflect::set(&options, &"duration".into(), &duration.into())?;
        Reflect::set(
            &options,
            &"easing".into(),
            &"cubic-bezier(.22,1,.36,1)".into(),
        )?;
        Reflect::set(&options, &"fill".into(), &"both".into())?;
        prop(el.as_ref(), "animate")
            .dyn_into::<Function>()?
            .call2(el, &keyframes, &options)?
            .dyn_into()
    }
    pub fn reveal(&self, mode: &str) -> Result<Vec<Animation>, JsValue> {
        if matches!(mode, "reveal" | "rise") {
            // Counter-translate the frozen pixels so only the clipping edge moves.
            let (axis, n, duration) = if mode == "rise" {
                ("Y", -100, 360.)
            } else {
                ("X", 100, 380.)
            };
            let shift = |v| format!("translate{axis}({v}%)");
            return Ok(vec![
                Self::animate(&self.layer, "transform", &shift(0), &shift(n), duration)?,
                Self::animate(&self.viewport, "transform", &shift(0), &shift(-n), duration)?,
            ]);
        }
        let cutout = |points: String| {
            format!("polygon(evenodd,0% 0%,100% 0%,100% 100%,0% 100%,0% 0%,{points})")
        };
        let (from, to, ms) = if mode == "split" {
            let split = |a, b| cutout(format!("0% {a}%,0% {b}%,100% {b}%,100% {a}%,0% {a}%"));
            (split(50, 50), split(0, 100), 390.)
        } else {
            let ellipse = |rx: f64| {
                cutout(
                    (0..=96)
                        .map(|i| {
                            let a = i as f64 * std::f64::consts::TAU / 96.;
                            format!("{:.3}% {:.3}%", 50. + rx * a.cos(), 50. + 110. * a.sin())
                        })
                        .collect::<Vec<_>>()
                        .join(","),
                )
            };
            (ellipse(0.), ellipse(80.), 430.)
        };
        Ok(vec![Self::animate(
            &self.layer,
            "clipPath",
            &from,
            &to,
            ms,
        )?])
    }
}
