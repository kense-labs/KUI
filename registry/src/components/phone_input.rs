//! International phone entry with searchable country/calling-code pairs.
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, Slide},
    form_chrome::*,
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhoneValue {
    pub country: &'static str,
    pub national: String,
    pub international: String,
    pub possible: bool,
}
pub struct PhoneInput {
    label: SharedString,
    country: usize,
    edit: Entity<Editor>,
    query: Entity<Editor>,
    open: bool,
    reveal: FormReveal,
    indicator: Slide,
    dial: RollingValue,
    direction: f32,
    active: usize,
    focus: FocusHandle,
    _subs: Vec<Subscription>,
}
impl EventEmitter<PhoneValue> for PhoneInput {}
impl PhoneInput {
    pub fn new(
        label: impl Into<SharedString>,
        iso: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let country = COUNTRIES.iter().position(|c| c.0 == iso).unwrap_or(0);
        let edit = cx.new(|cx| {
            let mut e = Editor::new(label.clone(), InputMode::Single, w, cx);
            e.set_placeholder(if iso == "US" {
                "(201) 555-0123"
            } else {
                "123 456 7890"
            });
            e
        });
        let query = cx.new(|cx| {
            let mut e = Editor::new(
                tr(cx, "Find country", "搜索国家或地区"),
                InputMode::Combobox,
                w,
                cx,
            );
            e.set_placeholder(tr(cx, "Country or calling code…", "国家、地区或区号…"));
            e
        });
        let edit_sub = cx.subscribe_in(&edit, w, |s: &mut Self, e, ev: &EditEvent, w, cx| {
            if *ev == EditEvent::Changed {
                cx.emit(s.value(cx));
                cx.notify()
            }
            if *ev == EditEvent::Blurred {
                let val = s.value(cx);
                if ["US", "CA"].contains(&val.country) && val.national.len() == 10 {
                    let n = val.national;
                    e.update(cx, |e, cx| {
                        e.set_value(format!("({}) {}-{}", &n[..3], &n[3..6], &n[6..]), w, cx)
                    })
                }
            }
        });
        let q_sub = cx.subscribe_in(&query, w, |s: &mut Self, _, ev: &EditEvent, w, cx| {
            let ids = s.matches(cx);
            match ev {
                EditEvent::Changed => s.active = 0,
                EditEvent::Escape => s.toggle(w, cx),
                EditEvent::Navigate(next) if !ids.is_empty() => {
                    s.active = if *next {
                        (s.active + 1) % ids.len()
                    } else {
                        (s.active + ids.len() - 1) % ids.len()
                    }
                }
                EditEvent::Submitted => {
                    if let Some(&i) = ids.get(s.active) {
                        s.choose(i, w, cx)
                    }
                }
                _ => {}
            }
            cx.notify()
        });
        Self {
            label,
            country,
            edit,
            query,
            open: false,
            reveal: FormReveal::new(false),
            indicator: Slide::snappy(0.),
            dial: Default::default(),
            direction: 1.,
            active: 0,
            focus: cx.focus_handle().tab_stop(true),
            _subs: vec![edit_sub, q_sub],
        }
    }
    pub fn value(&self, cx: &App) -> PhoneValue {
        let c = COUNTRIES[self.country];
        let raw = self.edit.read(cx).value().trim();
        let digits = raw.chars().filter(char::is_ascii_digit).collect::<String>();
        let national = if raw.starts_with('+') {
            digits
                .strip_prefix(c.3.trim_start_matches('+'))
                .unwrap_or(&digits)
                .to_owned()
        } else {
            digits
        };
        let possible = match c.0 {
            "US" | "CA" => national.len() == 10,
            "CN" => national.len() == 11,
            _ => (4..=15 - c.3.len() + 1).contains(&national.len()),
        };
        PhoneValue {
            country: c.0,
            international: format!("{}{}", c.3, national),
            national,
            possible,
        }
    }
    fn matches(&self, cx: &App) -> Vec<usize> {
        let q = self.query.read(cx).value().to_lowercase();
        COUNTRIES
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                [c.0, c.1, c.2, c.3]
                    .into_iter()
                    .any(|v| v.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect()
    }
    fn choose(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        self.direction = if i >= self.country { 1. } else { -1. };
        self.country = i;
        self.open = false;
        self.edit.read(cx).focus.clone().focus(w, cx);
        cx.emit(self.value(cx));
        cx.notify()
    }
    fn toggle(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        if self.open {
            self.active = 0;
            self.query.update(cx, |e, cx| {
                e.set_value("", w, cx);
                e.focus(w, cx)
            })
        } else {
            self.focus.focus(w, cx)
        }
        cx.notify()
    }
}
impl Render for PhoneInput {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let c = COUNTRIES[self.country];
        let zh = tr(cx, "en", "zh") == "zh";
        let progress = self.reveal.sample(self.open, 0.42, 0.40, w, cx);
        self.indicator.duration(self.active as f32, 0.42, cx);
        let code = self
            .dial
            .label(format!("{}  {}", c.0, c.3), self.direction, w, cx);
        let mut root = section(380., cx)
            .id("phone")
            .on_mouse_down_out(cx.listener(|s, _, _, cx| {
                if s.open {
                    s.open = false;
                    cx.notify()
                }
            }))
            .child(self.label.clone())
            .child(
                row()
                    .h(px(44.))
                    .p(px(4.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .child(
                        button("country", "", cx)
                            .child(code)
                            .aria_label(if zh { c.2 } else { c.1 })
                            .track_focus(&self.focus)
                            .child(icon(Glyph::Down, 13., p.faint))
                            .on_click(cx.listener(|s, _, w, cx| s.toggle(w, cx))),
                    )
                    .child(div().flex_1().min_w_0().h(px(30.)).child(self.edit.clone())),
            );
        if self.open || progress > 0. {
            let ids = self.matches(cx);
            let mut rows = col().id("countries").h(px(170.)).overflow_y_scroll();
            rows = rows.relative().child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(40. * self.indicator.get(w, cx)))
                    .h(px(32.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.hover),
            );
            for &i in &ids {
                let c = COUNTRIES[i];
                rows = rows.child(
                    button(
                        ("country", i),
                        format!("{}  {}", if zh { c.2 } else { c.1 }, c.3),
                        cx,
                    )
                    .justify_start()
                    .on_click(cx.listener(move |s, _, w, cx| s.choose(i, w, cx))),
                )
            }
            root = root.child(
                self.reveal
                    .wrap(
                        panel(cx)
                            .child(div().h(px(32.)).child(self.query.clone()))
                            .child(rows),
                        progress,
                        self.open,
                        228.,
                        0.,
                    )
                    .w(px(105. + 275. * progress))
                    .max_w_full(),
            )
        }
        root.child(hint(
            tr(
                cx,
                "Type + to search by calling code",
                "输入 + 可按国际区号搜索",
            ),
            cx,
        ))
    }
}
const COUNTRIES: &[(&str, &str, &str, &str)] = &[
    ("AR", "Argentina", "阿根廷", "+54"),
    ("AU", "Australia", "澳大利亚", "+61"),
    ("AT", "Austria", "奥地利", "+43"),
    ("BE", "Belgium", "比利时", "+32"),
    ("BR", "Brazil", "巴西", "+55"),
    ("CA", "Canada", "加拿大", "+1"),
    ("CL", "Chile", "智利", "+56"),
    ("CN", "China", "中国", "+86"),
    ("CO", "Colombia", "哥伦比亚", "+57"),
    ("CZ", "Czechia", "捷克", "+420"),
    ("DK", "Denmark", "丹麦", "+45"),
    ("EG", "Egypt", "埃及", "+20"),
    ("FI", "Finland", "芬兰", "+358"),
    ("FR", "France", "法国", "+33"),
    ("DE", "Germany", "德国", "+49"),
    ("GR", "Greece", "希腊", "+30"),
    ("HK", "Hong Kong", "中国香港", "+852"),
    ("IN", "India", "印度", "+91"),
    ("ID", "Indonesia", "印度尼西亚", "+62"),
    ("IE", "Ireland", "爱尔兰", "+353"),
    ("IL", "Israel", "以色列", "+972"),
    ("IT", "Italy", "意大利", "+39"),
    ("JP", "Japan", "日本", "+81"),
    ("KE", "Kenya", "肯尼亚", "+254"),
    ("MX", "Mexico", "墨西哥", "+52"),
    ("NL", "Netherlands", "荷兰", "+31"),
    ("NZ", "New Zealand", "新西兰", "+64"),
    ("NG", "Nigeria", "尼日利亚", "+234"),
    ("NO", "Norway", "挪威", "+47"),
    ("PK", "Pakistan", "巴基斯坦", "+92"),
    ("PE", "Peru", "秘鲁", "+51"),
    ("PH", "Philippines", "菲律宾", "+63"),
    ("PL", "Poland", "波兰", "+48"),
    ("PT", "Portugal", "葡萄牙", "+351"),
    ("SA", "Saudi Arabia", "沙特阿拉伯", "+966"),
    ("SG", "Singapore", "新加坡", "+65"),
    ("ZA", "South Africa", "南非", "+27"),
    ("KR", "South Korea", "韩国", "+82"),
    ("ES", "Spain", "西班牙", "+34"),
    ("SE", "Sweden", "瑞典", "+46"),
    ("CH", "Switzerland", "瑞士", "+41"),
    ("TW", "Taiwan", "中国台湾", "+886"),
    ("TH", "Thailand", "泰国", "+66"),
    ("TR", "Turkey", "土耳其", "+90"),
    ("UA", "Ukraine", "乌克兰", "+380"),
    ("AE", "United Arab Emirates", "阿联酋", "+971"),
    ("GB", "United Kingdom", "英国", "+44"),
    ("US", "United States", "美国", "+1"),
    ("VN", "Vietnam", "越南", "+84"),
];
