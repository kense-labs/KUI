//! Colour math shared by chart components.
//!
//! The reference charts mix their muted fills with `color-mix(in oklab, …)`.
//! These helpers convert through Ok Lab so the washed fills land on the
//! reference colour instead of an sRGB near miss.
use gpui::Hsla;
pub(crate) fn srgb_channel_to_linear(c: f32) -> f32 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
pub(crate) fn linear_channel_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1. / 2.4) - 0.055
    }
}
pub(crate) fn to_oklab(color: Hsla) -> (f32, f32, f32) {
    let rgb = color.to_rgb();
    let (r, g, b) = (
        srgb_channel_to_linear(rgb.r),
        srgb_channel_to_linear(rgb.g),
        srgb_channel_to_linear(rgb.b),
    );
    let l = (0.412_221_5 * r + 0.536_332_5 * g + 0.051_445_99 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_397 * b).cbrt();
    let s = (0.088_302_5 * r + 0.281_718_8 * g + 0.629_978_7 * b).cbrt();
    (
        0.210_454_3 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592 * m + 0.450_593_7 * s,
        0.025_904_04 * l + 0.782_771_8 * m - 0.808_675_8 * s,
    )
}
pub(crate) fn from_oklab(lightness: f32, a: f32, b: f32) -> Hsla {
    let l_ = lightness + 0.396_337_8 * a + 0.215_803_8 * b;
    let m_ = lightness - 0.105_561_3 * a - 0.063_854_17 * b;
    let s_ = lightness - 0.089_483_52 * a - 1.291_486 * b;
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;
    let r = linear_channel_to_srgb((4.076_742 * l - 3.307_712 * m + 0.230_969_9 * s).clamp(0., 1.));
    let g =
        linear_channel_to_srgb((-1.268_438 * l + 2.609_756 * m - 0.341_399_4 * s).clamp(0., 1.));
    let b = linear_channel_to_srgb(
        (-0.004_196_086 * l - 0.703_418_6 * m + 1.707_615 * s).clamp(0., 1.),
    );
    gpui::Rgba { r, g, b, a: 1.0 }.into()
}
/// Mix `fg` over `bg` in Ok Lab space by `t` (0..1), like the reference
/// `color-mix(in oklab, fg t%, bg)`.
pub(crate) fn oklab_mix(bg: Hsla, fg: Hsla, t: f32) -> Hsla {
    let (bl, ba, bb) = to_oklab(bg);
    let (fl, fa, fb) = to_oklab(fg);
    from_oklab(bl + (fl - bl) * t, ba + (fa - ba) * t, bb + (fb - bb) * t)
}
