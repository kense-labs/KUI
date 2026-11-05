//! A currency input whose committed value is expressed in minor units, with currency selection and quick-add chips.
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, PhaseClock, Slide, Tone, face},
    form_chrome::{RollingValue, hint, row, tr},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};

/// A committed minor-unit amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoneyChanged(pub i64);

/// Currency metadata for multi-currency fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrencySpec {
    pub code: SharedString,
    pub symbol: SharedString,
    pub name: SharedString,
    pub precision: u8,
}
impl CurrencySpec {
    pub fn new(
        code: impl Into<SharedString>,
        symbol: impl Into<SharedString>,
        name: impl Into<SharedString>,
        precision: u8,
    ) -> Self {
        Self {
            code: code.into(),
            symbol: symbol.into(),
            name: name.into(),
            precision,
        }
    }
}

/// A locale-neutral currency field. Values are integer minor units (100 = 1.00).
pub struct MoneyInput {
    label: SharedString,
    description: SharedString,
    symbol: SharedString,
    code: SharedString,
    currencies: Vec<CurrencySpec>,
    menu_open: bool,
    precision: u8,
    precision_dirty: bool,
    value: i64,
    min: i64,
    max: i64,
    editor: Entity<Editor>,
    rolling: RollingValue,
    direction: f32,
    focus: FocusHandle,
    error: bool,
    disabled: bool,
    chips: Vec<i64>,
    /// Sideways strain toward a refused limit, and the age of the last refusal.
    kick: Slide,
    refused: PhaseClock,
    _sub: Subscription,
}
impl EventEmitter<MoneyChanged> for MoneyInput {}
impl MoneyInput {
    pub fn new(
        label: impl Into<SharedString>,
        value: i64,
        range: std::ops::RangeInclusive<i64>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let first = *range.start();
        let last = *range.end();
        let (min, max) = (first.min(last), first.max(last));
        let value = value.clamp(min, max);
        let editor = cx.new(|cx| {
            let mut e = Editor::new(label.clone(), InputMode::Single, w, cx);
            e.font_size = 30.;
            e.set_value(decimal_grouped(value, 2), w, cx);
            e
        });
        let sub = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, e, ev: &EditEvent, w, cx| match ev {
                EditEvent::Submitted | EditEvent::Blurred => {
                    if s.disabled || e.read(cx).is_composing() {
                        return;
                    }
                    let v = parse_minor(e.read(cx).value(), s.precision);
                    match v {
                        Some(v) if v >= s.min && v <= s.max => s.set(v, w, cx),
                        _ => {
                            s.error = true;
                            s.editor.update(cx, |e, cx| {
                                e.set_value(decimal_grouped(s.value, s.precision), w, cx)
                            });
                            cx.notify()
                        }
                    }
                }
                EditEvent::Navigate(down) => {
                    if s.disabled || e.read(cx).is_composing() {
                        return;
                    }
                    let step = 10_i64.pow(u32::from(s.precision)).max(1);
                    let delta = if *down { -step } else { step };
                    s.nudge(delta, w, cx);
                }
                _ => {}
            },
        );
        let currencies = vec![
            CurrencySpec::new("USD", "$", "US Dollar", 2),
            CurrencySpec::new("EUR", "€", "Euro", 2),
            CurrencySpec::new("GBP", "£", "British Pound", 2),
            CurrencySpec::new("JPY", "¥", "Japanese Yen", 0),
        ];
        Self {
            label,
            description: "".into(),
            symbol: "$".into(),
            code: "USD".into(),
            currencies,
            menu_open: false,
            precision: 2,
            precision_dirty: false,
            value,
            min,
            max,
            editor,
            rolling: RollingValue::default(),
            direction: 1.,
            focus: cx.focus_handle(),
            error: false,
            disabled: false,
            chips: vec![],
            kick: Slide::new(0.),
            refused: PhaseClock::default(),
            _sub: sub,
        }
    }
    pub fn with_symbol(mut self, symbol: impl Into<SharedString>) -> Self {
        self.symbol = symbol.into();
        self
    }
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }
    pub fn with_currencies(mut self, currencies: Vec<CurrencySpec>) -> Self {
        if let Some(first) = currencies.first() {
            self.code = first.code.clone();
            self.symbol = first.symbol.clone();
            self.precision = first.precision.min(18);
            self.precision_dirty = true;
        }
        self.currencies = currencies;
        self
    }
    /// Configure the number of fractional digits (0–18); the stored value stays in minor units.
    pub fn with_precision(mut self, precision: u8) -> Self {
        self.precision = precision.min(18);
        self.precision_dirty = true;
        self
    }
    /// Change precision without interrupting an input method. Returns false while composing.
    pub fn set_precision(&mut self, precision: u8, w: &Window, cx: &mut Context<Self>) -> bool {
        if self.editor.read(cx).is_composing() {
            return false;
        }
        self.precision = precision.min(18);
        self.precision_dirty = false;
        let text = decimal_grouped(self.value, self.precision);
        self.editor
            .update(cx, |editor, cx| editor.set_value(text, w, cx));
        cx.notify();
        true
    }
    pub fn with_quick_add(mut self, values: Vec<i64>) -> Self {
        self.chips = values;
        self
    }
    pub fn minor_units(&self) -> i64 {
        self.value
    }
    pub fn currency_code(&self) -> &str {
        self.code.as_ref()
    }
    pub fn symbol(&self) -> &str {
        self.symbol.as_ref()
    }
    pub fn precision(&self) -> u8 {
        self.precision
    }
    pub fn select_currency(&mut self, index: usize, w: &Window, cx: &mut Context<Self>) {
        if self.editor.read(cx).is_composing() {
            return;
        }
        let Some(spec) = self.currencies.get(index).cloned() else {
            return;
        };
        let converted = rescale_minor(self.value, self.precision, spec.precision);
        self.code = spec.code;
        self.symbol = spec.symbol;
        self.precision = spec.precision.min(18);
        self.menu_open = false;
        self.set(converted.clamp(self.min, self.max), w, cx);
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        self.menu_open = false;
        self.editor.update(cx, |editor, cx| {
            editor.disabled = disabled;
            cx.notify();
        });
        cx.notify();
    }
    pub fn set_value(&mut self, value: i64, w: &Window, cx: &mut Context<Self>) {
        let _ = self.try_set_value(value, w, cx);
    }
    /// Returns false rather than replacing unfinished IME composition.
    pub fn try_set_value(&mut self, value: i64, w: &Window, cx: &mut Context<Self>) -> bool {
        if self.editor.read(cx).is_composing() {
            return false;
        }
        self.set(value, w, cx);
        true
    }
    fn set(&mut self, value: i64, w: &Window, cx: &mut Context<Self>) {
        if self.editor.read(cx).is_composing() {
            return;
        }
        let v = value.clamp(self.min, self.max);
        self.error = value != v;
        if v != self.value {
            self.direction = if v >= self.value { 1. } else { -1. };
            self.value = v;
            cx.emit(MoneyChanged(v));
        }
        self.editor.update(cx, |e, cx| {
            e.set_value(decimal_grouped(v, self.precision), w, cx)
        });
        cx.notify()
    }
    /// Strain the amount toward the pressed direction and light the range as a
    /// warning, the same refusal signal the official field gives at a limit: a
    /// 160 px/s kick into the critically damped 0.42s spring that settles home.
    fn refuse(&mut self, toward: f32, cx: &mut Context<Self>) {
        self.kick.release(0., toward * 160., 0.42, cx);
        self.refused.restart(cx);
        if crate::primitives::family::disabled_motion(cx) {
            // Reduced motion keeps the instant warning color but has no frame
            // loop to age the clock, so one timer clears it after the same hold.
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(1400))
                    .await;
                let _ = this.update(cx, |s, cx| {
                    s.refused = PhaseClock::default();
                    cx.notify()
                });
            })
            .detach();
        }
        cx.notify()
    }
    /// Step the amount against the official limit rule: a step that crosses the
    /// ceiling settles on it and flags the refusal, a step already at the ceiling
    /// holds the value, and a step below the floor rises to it without a flag.
    fn nudge(&mut self, delta: i64, w: &Window, cx: &mut Context<Self>) {
        if self.editor.read(cx).is_composing() {
            return;
        }
        let from = self.value;
        let next = from.saturating_add(delta);
        if next > self.max {
            if from < self.max {
                self.set(self.max, w, cx);
            }
            self.refuse(1., cx);
            return;
        }
        if next < self.min {
            if from <= self.min {
                self.refuse(-1., cx);
                return;
            }
            self.set(self.min, w, cx);
            return;
        }
        self.set(next, w, cx);
    }
    /// The allowed range as money, shown beside the quick-add chips. Empty when unbounded.
    fn range_text(&self) -> SharedString {
        let fmt = |v: i64| format!("{}{}", self.symbol, decimal_grouped(v, self.precision));
        if self.min > i64::MIN / 2 && self.max < i64::MAX / 2 {
            format!("{} to {}", fmt(self.min), fmt(self.max)).into()
        } else if self.max < i64::MAX / 2 {
            format!("Up to {}", fmt(self.max)).into()
        } else if self.min > i64::MIN / 2 {
            format!("At least {}", fmt(self.min)).into()
        } else {
            SharedString::default()
        }
    }
}
fn decimal(value: i64, precision: u8) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let n = u128::from(value.unsigned_abs());
    let scale = 10_u128.pow(u32::from(precision));
    if precision == 0 {
        format!("{sign}{n}")
    } else {
        format!(
            "{sign}{}.{:0width$}",
            n / scale,
            n % scale,
            width = usize::from(precision)
        )
    }
}
fn group_whole(whole: &str) -> String {
    let bytes = whole.as_bytes();
    let mut out = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(b as char);
    }
    out
}
pub fn decimal_grouped(value: i64, precision: u8) -> String {
    let raw = decimal(value, precision);
    let (sign, rest) = if let Some(s) = raw.strip_prefix('-') {
        ("-", s)
    } else {
        ("", raw.as_str())
    };
    if let Some((w, f)) = rest.split_once('.') {
        format!("{sign}{}.{f}", group_whole(w))
    } else {
        format!("{sign}{}", group_whole(rest))
    }
}
fn parse_minor(value: &str, precision: u8) -> Option<i64> {
    let s = value.trim().replace([',', ' '], "");
    let (neg, body) = if let Some(v) = s.strip_prefix('-') {
        (true, v)
    } else {
        (false, s.as_str())
    };
    let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
    if frac.len() > usize::from(precision)
        || whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || !frac.bytes().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let scale = 10_i128.pow(u32::from(precision));
    let whole = whole.parse::<i128>().ok()?;
    let fraction = if frac.is_empty() {
        0
    } else {
        frac.parse::<i128>().ok()?
    }
    .checked_mul(10_i128.pow(u32::from(precision) - frac.len() as u32))?;
    let value = whole.checked_mul(scale)?.checked_add(fraction)?;
    let signed = if neg { value.checked_neg()? } else { value };
    i64::try_from(signed).ok()
}
impl Render for MoneyInput {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.precision_dirty && !self.editor.read(cx).is_composing() {
            self.precision_dirty = false;
            let text = decimal_grouped(self.value, self.precision);
            self.editor
                .update(cx, |editor, cx| editor.set_value(text, w, cx));
        }
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let focused = self.editor.read(cx).focus.is_focused(w);
        let scale_factor = 10_i64.pow(u32::from(self.precision.min(18)));
        let at_max = self.value >= self.max;
        let mut chips = row().flex_wrap().gap(px(6.));
        for (i, amount) in self.chips.iter().copied().enumerate() {
            let scaled = amount;
            let major = scaled / scale_factor.max(1);
            let text = if scaled % scale_factor == 0 && major > 0 {
                format!("+{}{}", self.symbol, group_whole(&major.to_string()))
            } else {
                format!(
                    "+{}{}",
                    self.symbol,
                    decimal_grouped(scaled, self.precision)
                )
            };
            let add = face(
                ("add", i),
                text.clone(),
                30.,
                radius.into(),
                Tone::Clear,
                cx,
            )
            .px(px(12.))
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .when(!self.disabled && !at_max, |b| {
                b.hover(move |s| s.bg(p.tray).text_color(p.ink))
            })
            .active(move |s| s.bg(p.tray).text_color(p.ink))
            .child(text);
            // A chip at the ceiling dims like a disabled one but stays clickable so
            // the press still answers with the refusal strain.
            let add = if self.disabled {
                add.opacity(0.45).tab_index(-1)
            } else if at_max {
                add.opacity(0.45)
                    .on_click(cx.listener(move |s, _, _, cx| s.refuse(1., cx)))
            } else {
                add.on_click(cx.listener(move |s, _, w, cx| s.nudge(scaled, w, cx)))
            };
            chips = chips.child(add)
        }
        let range_text = self.range_text();
        let _ = self.rolling.label(
            decimal_grouped(self.value, self.precision),
            self.direction,
            w,
            cx,
        );
        let mut field = row()
            .relative()
            .h(px(76.))
            .pl(px(18.))
            .pr(px(8.))
            .gap(px(8.))
            .rounded(radius)
            .border_1()
            .border_color(if self.error {
                p.danger
            } else if focused {
                p.ink
            } else {
                p.faint
            })
            .when(!self.error, |f| f.hover(|s| s.border_color(p.ink)))
            .bg(if self.disabled { p.tray } else { p.panel })
            .when(self.disabled, |f| f.opacity(0.5))
            .child(
                div()
                    .relative()
                    .left(px(self.kick.get(w, cx)))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_baseline()
                    .child(
                        div()
                            .mr(px(1.8))
                            .text_size(px(30.))
                            .line_height(px(36.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.secondary)
                            .child(self.symbol.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h(px(36.))
                            .text_size(px(30.))
                            .line_height(px(36.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.editor.clone()),
                    ),
            );
        if self.currencies.len() > 1 {
            let code = self.code.clone();
            let trigger = face(
                "money-currency-trigger",
                format!("{}: {}", tr(cx, "Currency", "币种"), code),
                36.,
                radius.into(),
                Tone::Clear,
                cx,
            )
            .pl(px(12.))
            .pr(px(8.))
            .gap(px(4.))
            .hover(|s| s.bg(p.tray))
            .when(self.menu_open, |b| b.bg(p.tray))
            .child(code)
            .child(icon(
                if self.menu_open {
                    Glyph::Up
                } else {
                    Glyph::Down
                },
                16.,
                p.faint,
            ))
            .when(!self.disabled, |b| {
                b.on_click(cx.listener(|s, _, _, cx| {
                    s.menu_open = !s.menu_open;
                    cx.notify();
                }))
            });
            field = field.child(div().relative().flex_none().child(trigger));
        }
        let mut root = div()
            .id("money-input")
            .relative()
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .w_full()
            .max_w(px(360.))
            .text_color(p.ink)
            .on_mouse_down_out(cx.listener(|s, _, _, cx| {
                if s.menu_open {
                    s.menu_open = false;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .mb(px(8.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(self.label.clone()),
            )
            .child(field);
        if self.menu_open && !self.disabled {
            let mut menu = div()
                .id("money-currency-menu")
                .role(Role::ListBox)
                .aria_label(tr(cx, "Currency", "币种"))
                .w(px(256.))
                .max_h(px(288.))
                .p(px(6.))
                .rounded(radius)
                .border_1()
                .border_color(p.line)
                .bg(p.panel)
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(2.));
            for (i, spec) in self.currencies.iter().enumerate() {
                let selected = spec.code == self.code;
                let row_label = format!("{} · {}", spec.code, spec.name);
                menu = menu.child(
                    face(
                        ("money-currency-opt", i),
                        row_label.clone(),
                        36.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .w_full()
                    .min_h(px(36.))
                    .px(px(10.))
                    .gap(px(10.))
                    .justify_between()
                    .text_color(if selected { p.ink } else { p.secondary })
                    .role(Role::ListBoxOption)
                    .aria_selected(selected)
                    .when(selected, |d| d.bg(p.ink.opacity(0.065)))
                    .hover(move |s| s.bg(p.ink.opacity(0.065)).text_color(p.ink))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .w(px(22.))
                                    .text_color(p.faint)
                                    .child(spec.symbol.clone()),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(spec.code.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(spec.name.clone()),
                            ),
                    )
                    .when(selected, |d| d.child(icon(Glyph::Check, 16., p.ink)))
                    .on_click(cx.listener(move |s, _, w, cx| s.select_currency(i, w, cx))),
                );
            }
            root = root.child(div().w_full().flex().justify_end().mt(px(8.)).child(menu));
        }
        let has_range = !range_text.is_empty();
        if !self.chips.is_empty() || has_range {
            // `time` keeps requesting frames while the refusal is fresh under
            // normal motion; `age` is the honest elapsed time, so the warning
            // still shows when reduced motion stops the frame loop.
            let _ = self.refused.time(1.4, w, cx);
            let flash = self.refused.age(cx) < 1.4;
            let mut bar = row()
                .flex_wrap()
                .items_center()
                .justify_between()
                .mt(px(12.));
            if !self.chips.is_empty() {
                bar = bar.child(chips);
            }
            if has_range {
                bar = bar.child(
                    div()
                        .text_size(px(12.))
                        .text_color(if flash {
                            crate::foundation::Theme::global(cx).tokens.colors.warning
                        } else {
                            p.faint
                        })
                        .child(range_text),
                );
            }
            root = root.child(bar);
        }
        root.when(!self.description.is_empty() && !self.error, |v| {
            v.child(hint(self.description.clone(), cx).pt(px(8.)))
        })
        .when(self.error, |v| {
            v.child(
                hint(
                    tr(
                        cx,
                        "Enter an amount within the allowed range.",
                        "请输入允许范围内的金额。",
                    ),
                    cx,
                )
                .pt(px(8.))
                .text_color(p.danger),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{MoneyInput, decimal, parse_minor};
    use gpui::{EntityInputHandler, TestAppContext};

    #[test]
    fn precision_and_full_signed_integer_range_round_trip() {
        for precision in [0, 2, 3, 18] {
            for value in [i64::MIN, -101, -1, 0, 1, 101, i64::MAX] {
                assert_eq!(
                    parse_minor(&decimal(value, precision), precision),
                    Some(value)
                );
            }
        }
        assert_eq!(parse_minor("12.345", 3), Some(12345));
        assert_eq!(parse_minor("12.3", 3), Some(12300));
        assert_eq!(parse_minor("12.3", 0), None);
    }

    #[test]
    fn reversed_range_is_normalized_and_composition_is_preserved() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| MoneyInput::new("Amount", 5, 10..=0, w, cx));
        assert_eq!(w.update(|_, cx| control.read(cx).minor_units()), 5);
        w.update(|w, cx| {
            let editor = control.read(cx).editor.clone();
            editor.update(cx, |editor, cx| {
                editor.focus(w, cx);
                editor.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
            })
        });
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                assert!(!s.try_set_value(8, w, cx));
                assert!(!s.set_precision(0, w, cx));
                assert_eq!(s.minor_units(), 5);
                assert!(s.editor.read(cx).is_composing());
            })
        });
    }

    #[test]
    fn stepping_past_a_limit_refuses_instead_of_clamping() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) =
            app.add_window_view(|w, cx| MoneyInput::new("Amount", 99_00, 0..=100_00, w, cx));
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                // A step that crosses the ceiling settles on the limit and flags
                // the refusal, rather than overshooting.
                s.nudge(5_00, w, cx);
                assert_eq!(s.minor_units(), 100_00);
                assert!(s.refused.age(cx).is_finite());
                // A step already pressed against the ceiling holds the value.
                s.nudge(1_00, w, cx);
                assert_eq!(s.minor_units(), 100_00);
                // A step that fits still commits.
                s.nudge(-5_00, w, cx);
                assert_eq!(s.minor_units(), 95_00);
            })
        });
    }
}

fn rescale_minor(value: i64, from: u8, to: u8) -> i64 {
    let old_scale = 10_i128.pow(u32::from(from.min(18)));
    let new_scale = 10_i128.pow(u32::from(to.min(18)));
    (i128::from(value) * new_scale / old_scale).clamp(i128::from(i64::MIN), i128::from(i64::MAX))
        as i64
}

#[cfg(test)]
mod currency_conversion_tests {
    use super::rescale_minor;
    #[test]
    fn changing_currency_preserves_fractional_amounts() {
        assert_eq!(rescale_minor(12345, 2, 2), 12345);
        assert_eq!(rescale_minor(12345, 2, 3), 123450);
        assert_eq!(rescale_minor(-12345, 2, 3), -123450);
        assert_eq!(rescale_minor(12345, 3, 2), 1234);
    }
    #[test]
    fn high_precision_conversion_saturates_without_overflowing() {
        assert_eq!(rescale_minor(i64::MAX, 18, 18), i64::MAX);
        assert_eq!(rescale_minor(i64::MAX, 0, 18), i64::MAX);
        assert_eq!(rescale_minor(i64::MIN, 0, 18), i64::MIN);
    }
}
