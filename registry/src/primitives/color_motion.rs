//! Continuous premultiplied Oklab transitions without platform-specific drawing.
use super::family::standard_ease;
use gpui::{Hsla, Rgba, transparent_black};
use web_time::Instant;

// Premultiplied Oklab mixing matches semantic tinting and CSS color transitions.
fn lab(color: Hsla) -> [f64; 4] {
    let c = Rgba::from(color);
    let linear = |v: f32| {
        let v = v as f64;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (linear(c.r), linear(c.g), linear(c.b));
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
        c.a as f64,
    ]
}
pub(crate) fn mix(a: Hsla, b: Hsla, t: f64) -> Hsla {
    if t <= 0. {
        return a;
    }
    if t >= 1. {
        return b;
    }
    let a = lab(a);
    let b = lab(b);
    let alpha = a[3] + (b[3] - a[3]) * t;
    if alpha <= 0. {
        return transparent_black();
    }
    let v = std::array::from_fn::<_, 3, _>(|i| (a[i] * a[3] * (1. - t) + b[i] * b[3] * t) / alpha);
    let l = (v[0] + 0.3963377774 * v[1] + 0.2158037573 * v[2]).powi(3);
    let m = (v[0] - 0.1055613458 * v[1] - 0.0638541728 * v[2]).powi(3);
    let s = (v[0] - 0.0894841775 * v[1] - 1.2914855480 * v[2]).powi(3);
    let encode = |v: f64| {
        if v <= 0.0031308 {
            (v * 12.92) as f32
        } else {
            (1.055 * v.powf(1. / 2.4) - 0.055) as f32
        }
    };
    Rgba {
        r: encode(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s).clamp(0., 1.),
        g: encode(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s).clamp(0., 1.),
        b: encode(-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s).clamp(0., 1.),
        a: alpha as f32,
    }
    .into()
}
#[derive(Clone, Copy)]
pub(crate) struct ColorFade {
    pub(crate) from: Hsla,
    pub(crate) to: Hsla,
    pub(crate) at: Instant,
}
impl ColorFade {
    pub(crate) fn value(self, now: Instant, seconds: f32) -> (Hsla, bool) {
        if self.from == self.to || seconds <= 0. {
            return (self.to, false);
        }
        let t = (now.saturating_duration_since(self.at).as_secs_f32() / seconds).clamp(0., 1.);
        (mix(self.from, self.to, standard_ease(t) as f64), t < 1.)
    }
    pub(crate) fn retarget(&mut self, to: Hsla, now: Instant, seconds: f32) -> (Hsla, bool) {
        if self.to != to {
            self.from = self.value(now, seconds).0;
            self.to = to;
            self.at = now;
        }
        self.value(now, seconds)
    }
}
