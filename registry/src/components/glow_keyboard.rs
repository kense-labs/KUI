//! Self-typing AI chat input above a frosted glass keyboard and reactive gradient floor.
//!
//! Renders an `11 : 6` hero scene where a demo (or the user's own keystrokes and clicks)
//! types prompts with human rhythm, depressing matching glass keycaps while sending glowing
//! color waves up a two-layer Gaussian-blurred gradient floor and flashing the input halo.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_eid, pro_empty_state, pro_mix_color, pro_skeleton_bar, tr,
    },
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiLanguage,
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_raw_svg, pro_svg},
    },
};
use gpui::{prelude::*, *};
use image::{Frame, RgbaImage};
use std::{
    cell::{Cell, RefCell},
    f32::consts::{PI, TAU},
    rc::Rc,
    sync::Arc,
    time::Duration,
};
use web_time::Instant;

const STAGE_U_W: f32 = 2200.0;
const STAGE_U_H: f32 = 1200.0;

const TEX_W: usize = 160;
const TEX_H: usize = 92;
const PAD_X: usize = 18;
const PAD_Y: usize = 14;
const GW: usize = TEX_W + 2 * PAD_X;
const GH: usize = TEX_H + 2 * PAD_Y;

const ORB_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 96 96"><defs><radialGradient id="gk-orb-g" cx="50%" cy="14%" r="86%"><stop offset="0%" stop-color="#FFB36B"/><stop offset="28%" stop-color="#FF7700"/><stop offset="100%" stop-color="#FF2A91"/></radialGradient></defs><circle cx="48" cy="48" r="46" fill="url(#gk-orb-g)"/><ellipse cx="48" cy="24" rx="28" ry="14" fill="#ffffff" fill-opacity="0.34"/></svg>"##;

/// Events emitted by [`GlowKeyboard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlowKeyboardEvent {
    /// Emitted when a prompt is submitted with `Enter` (either by the self-typing demo or by the user).
    Submit(SharedString),
    /// Emitted when the user edits the input text directly.
    Change(SharedString),
    /// Emitted when self-typing demo playback pauses or resumes.
    DemoPaused(bool),
}

impl EventEmitter<GlowKeyboardEvent> for GlowKeyboard {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyStyle {
    Alpha,
    Mod,
    ModSmall,
    FKey,
    TouchId,
    Arrow,
    HalfArrow,
}

#[derive(Clone, Copy)]
struct KeySpec {
    id: &'static str,
    legend: &'static str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    style: KeyStyle,
}

/// Complete 6-row Apple-style glass keyboard geometry in `1575 x 634` board units (`25u` outer margin, `10u` key gap).
fn build_keyboard_layout() -> Vec<KeySpec> {
    let mut keys = Vec::with_capacity(78);

    // Row 0: Function row (`y = 25, h = 56`, total inner width = `1525u`)
    keys.push(KeySpec {
        id: "escape",
        legend: "esc",
        x: 25.,
        y: 25.,
        w: 149.,
        h: 56.,
        style: KeyStyle::ModSmall,
    });
    let mut cx_u = 25. + 149. + 10.;
    for f_num in 1..=12 {
        let label: &'static str = match f_num {
            1 => "F1",
            2 => "F2",
            3 => "F3",
            4 => "F4",
            5 => "F5",
            6 => "F6",
            7 => "F7",
            8 => "F8",
            9 => "F9",
            10 => "F10",
            11 => "F11",
            _ => "F12",
        };
        keys.push(KeySpec {
            id: label,
            legend: label,
            x: cx_u,
            y: 25.,
            w: 96.,
            h: 56.,
            style: KeyStyle::FKey,
        });
        cx_u += 106.;
    }
    keys.push(KeySpec {
        id: "touchid",
        legend: "",
        x: cx_u,
        y: 25.,
        w: 94.,
        h: 56.,
        style: KeyStyle::TouchId,
    });

    // Row 1: Number row (`y = 91, h = 96`)
    let row1: [(&str, &str); 13] = [
        ("`", "`"),
        ("1", "1"),
        ("2", "2"),
        ("3", "3"),
        ("4", "4"),
        ("5", "5"),
        ("6", "6"),
        ("7", "7"),
        ("8", "8"),
        ("9", "9"),
        ("0", "0"),
        ("-", "-"),
        ("=", "="),
    ];
    cx_u = 25.;
    for (id, leg) in row1 {
        keys.push(KeySpec {
            id,
            legend: leg,
            x: cx_u,
            y: 91.,
            w: 96.,
            h: 96.,
            style: KeyStyle::Alpha,
        });
        cx_u += 106.;
    }
    keys.push(KeySpec {
        id: "backspace",
        legend: "⌫",
        x: cx_u,
        y: 91.,
        w: 147.,
        h: 96.,
        style: KeyStyle::Mod,
    });

    // Row 2: Top alpha row (`y = 197, h = 96`)
    keys.push(KeySpec {
        id: "tab",
        legend: "⇥",
        x: 25.,
        y: 197.,
        w: 147.,
        h: 96.,
        style: KeyStyle::Mod,
    });
    cx_u = 25. + 147. + 10.;
    for ch in [
        "q", "w", "e", "r", "t", "y", "u", "i", "o", "p", "[", "]", "\\",
    ] {
        keys.push(KeySpec {
            id: ch,
            legend: ch,
            x: cx_u,
            y: 197.,
            w: 96.,
            h: 96.,
            style: KeyStyle::Alpha,
        });
        cx_u += 106.;
    }

    // Row 3: Home alpha row (`y = 303, h = 96`)
    keys.push(KeySpec {
        id: "capslock",
        legend: "⇪",
        x: 25.,
        y: 303.,
        w: 172.,
        h: 96.,
        style: KeyStyle::Mod,
    });
    cx_u = 25. + 172. + 10.;
    for ch in ["a", "s", "d", "f", "g", "h", "j", "k", "l", ";", "'"] {
        keys.push(KeySpec {
            id: ch,
            legend: ch,
            x: cx_u,
            y: 303.,
            w: 96.,
            h: 96.,
            style: KeyStyle::Alpha,
        });
        cx_u += 106.;
    }
    keys.push(KeySpec {
        id: "enter",
        legend: "↩",
        x: cx_u,
        y: 303.,
        w: 177.,
        h: 96.,
        style: KeyStyle::Mod,
    });

    // Row 4: Bottom alpha row (`y = 409, h = 96`)
    keys.push(KeySpec {
        id: "shift-l",
        legend: "⇧",
        x: 25.,
        y: 409.,
        w: 225.,
        h: 96.,
        style: KeyStyle::Mod,
    });
    cx_u = 25. + 225. + 10.;
    for ch in ["z", "x", "c", "v", "b", "n", "m", ",", ".", "/"] {
        keys.push(KeySpec {
            id: ch,
            legend: ch,
            x: cx_u,
            y: 409.,
            w: 96.,
            h: 96.,
            style: KeyStyle::Alpha,
        });
        cx_u += 106.;
    }
    keys.push(KeySpec {
        id: "shift-r",
        legend: "⇧",
        x: cx_u,
        y: 409.,
        w: 230.,
        h: 96.,
        style: KeyStyle::Mod,
    });

    // Row 5: Modifiers, spacebar, and inverted-T arrow cluster (`y = 515, h = 96`)
    let mods_left: [(&str, &str, f32); 4] = [
        ("fn", "fn", 96.),
        ("ctrl", "⌃", 96.),
        ("alt-l", "⌥", 96.),
        ("cmd-l", "⌘", 122.),
    ];
    cx_u = 25.;
    for (id, leg, kw) in mods_left {
        keys.push(KeySpec {
            id,
            legend: leg,
            x: cx_u,
            y: 515.,
            w: kw,
            h: 96.,
            style: KeyStyle::ModSmall,
        });
        cx_u += kw + 10.;
    }
    keys.push(KeySpec {
        id: "space",
        legend: "",
        x: cx_u,
        y: 515.,
        w: 529.,
        h: 96.,
        style: KeyStyle::Alpha,
    });
    cx_u += 529. + 10.;
    for (id, leg, kw) in [("cmd-r", "⌘", 122.), ("alt-r", "⌥", 96.)] {
        keys.push(KeySpec {
            id,
            legend: leg,
            x: cx_u,
            y: 515.,
            w: kw,
            h: 96.,
            style: KeyStyle::ModSmall,
        });
        cx_u += kw + 10.;
    }
    keys.push(KeySpec {
        id: "left",
        legend: "◀",
        x: cx_u,
        y: 565.,
        w: 96.,
        h: 46.,
        style: KeyStyle::Arrow,
    });
    cx_u += 106.;
    keys.push(KeySpec {
        id: "up",
        legend: "▲",
        x: cx_u,
        y: 515.,
        w: 96.,
        h: 45.,
        style: KeyStyle::HalfArrow,
    });
    keys.push(KeySpec {
        id: "down",
        legend: "▼",
        x: cx_u,
        y: 566.,
        w: 96.,
        h: 45.,
        style: KeyStyle::HalfArrow,
    });
    cx_u += 106.;
    keys.push(KeySpec {
        id: "right",
        legend: "▶",
        x: cx_u,
        y: 565.,
        w: 96.,
        h: 46.,
        style: KeyStyle::Arrow,
    });

    keys
}

/// Map a CJK character to its lowercase ASCII Pinyin syllable for realistic IME typing.
fn cjk_pinyin(ch: char) -> &'static str {
    match ch {
        '规' => "gui",
        '划' => "hua",
        '六' => "liu",
        '月' => "yue",
        '多' => "duo",
        '洛' => "luo",
        '米' => "mi",
        '蒂' => "di",
        '三' => "san",
        '天' => "tian",
        '两' => "liang",
        '夜' => "ye",
        '山' => "shan",
        '屋' => "wu",
        '徒' => "tu",
        '步' => "bu",
        '路' => "lu",
        '线' => "xian",
        '整' => "zheng",
        '理' => "li",
        '公' => "gong",
        '斤' => "jin",
        '以' => "yi",
        '内' => "nei",
        '的' => "de",
        '云' => "yun",
        '雾' => "wu",
        '脊' => "ji",
        '轻' => "qing",
        '量' => "liang",
        '化' => "hua",
        '装' => "zhuang",
        '备' => "bei",
        '清' => "qing",
        '单' => "dan",
        '查' => "cha",
        '找' => "zhao",
        '刀' => "dao",
        '锋' => "feng",
        '与' => "yu",
        '休' => "xiu",
        '斯' => "si",
        '高' => "gao",
        '原' => "yuan",
        '附' => "fu",
        '近' => "jin",
        '日' => "ri",
        '出' => "chu",
        '观' => "guan",
        '景' => "jing",
        '机' => "ji",
        '位' => "wei",
        '帮' => "bang",
        '我' => "wo",
        '写' => "xie",
        '一' => "yi",
        '份' => "fen",
        '分' => "fen",
        '析' => "xi",
        '报' => "bao",
        '告' => "gao",
        '设' => "she",
        '计' => "ji",
        '生' => "sheng",
        '成' => "cheng",
        '总' => "zong",
        '结' => "jie",
        '代' => "dai",
        '码' => "ma",
        '优' => "you",
        '方' => "fang",
        '案' => "an",
        '搜' => "sou",
        '索' => "suo",
        '翻' => "fan",
        '译' => "yi",
        '数' => "shu",
        '据' => "ju",
        _ => {
            const FALLBACK_SYLLABLES: [&str; 24] = [
                "shi", "zhe", "wei", "lai", "kan", "hao", "xin", "ren", "jia", "dao", "cheng",
                "gong", "jian", "ming", "tian", "wen", "dong", "shui", "guang", "xing", "hai",
                "yang", "chun", "lin",
            ];
            FALLBACK_SYLLABLES[(ch as usize).wrapping_mul(17) % FALLBACK_SYLLABLES.len()]
        }
    }
}

fn is_cjk_ideograph(ch: char) -> bool {
    matches!(ch as u32, 0x3400..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2CEAF)
}

/// Map a typed character or key token to `(primary_key_id, needs_shift)`.
fn map_char_to_key(ch: char) -> (&'static str, bool) {
    if is_cjk_ideograph(ch) {
        let py = cjk_pinyin(ch);
        if let Some(first) = py.chars().next() {
            return map_char_to_key(first);
        }
    }
    match ch {
        'a'..='z' => {
            let idx = (ch as u8 - b'a') as usize;
            const LOWER: [&str; 26] = [
                "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p",
                "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
            ];
            (LOWER[idx], false)
        }
        'A'..='Z' => {
            let lower = ch.to_ascii_lowercase();
            let (id, _) = map_char_to_key(lower);
            (id, true)
        }
        '0' | '）' | ')' => ("0", ch != '0'),
        '1' | '！' | '!' => ("1", ch != '1'),
        '2' | '@' => ("2", ch == '@'),
        '3' | '#' => ("3", ch == '#'),
        '4' | '$' => ("4", ch == '$'),
        '5' | '%' => ("5", ch == '%'),
        '6' | '^' => ("6", ch == '^'),
        '7' | '&' => ("7", ch == '&'),
        '8' | '*' => ("8", ch == '*'),
        '9' | '（' | '(' => ("9", ch != '9'),
        ' ' => ("space", false),
        '-' | '—' => ("-", false),
        '_' => ("-", true),
        '=' => ("=", false),
        '+' => ("=", true),
        ',' | '，' => (",", false),
        '.' | '。' => (".", false),
        '/' => ("/", false),
        '?' | '？' => ("/", true),
        ':' | '：' => (";", true),
        ';' | '；' => (";", false),
        '[' | '【' => ("[", false),
        ']' | '】' => ("]", false),
        '\\' | '、' => ("\\", false),
        '\'' | '"' | '“' | '”' => ("'", matches!(ch, '"' | '“' | '”')),
        _ => ("space", false),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DemoStepKind {
    TypeChar,
    Backspace,
    EnterSubmit(SharedString),
    ClearAfterSubmit,
}

#[derive(Clone, Debug)]
struct DemoEvent {
    at_ms: u64,
    hold_ms: u64,
    text_after: String,
    key_id: Option<&'static str>,
    shift_held: bool,
    kind: DemoStepKind,
}

/// Build a human-rhythm self-typing timeline across `prompts`, supporting Shift holds, typo correction, and Pinyin IME composition for CJK text.
fn compile_demo_timeline(prompts: &[SharedString]) -> (Vec<DemoEvent>, u64) {
    let mut events = Vec::new();
    let mut t_ms = 480u64;

    for (p_idx, prompt) in prompts.iter().enumerate() {
        let mut buf = String::new();
        let chars: Vec<char> = prompt.chars().collect();
        let has_cjk = chars.iter().copied().any(is_cjk_ideograph);
        let typo_pos = if p_idx == 0 && !has_cjk && chars.len() > 14 {
            chars
                .iter()
                .enumerate()
                .skip(8)
                .find(|&(_, &c)| c.is_ascii_lowercase() && c != 'o')
                .map(|(i, _)| i)
        } else {
            None
        };

        let mut c_idx = 0usize;
        while c_idx < chars.len() {
            if is_cjk_ideograph(chars[c_idx]) {
                // Compose 2 CJK characters at a time via real QWERTY Pinyin keystrokes + Space commit
                let end = (c_idx + 2).min(chars.len()).min({
                    let mut k = c_idx + 1;
                    while k < chars.len() && k < c_idx + 2 && is_cjk_ideograph(chars[k]) {
                        k += 1;
                    }
                    k
                });
                let phrase: String = chars[c_idx..end].iter().collect();
                let mut py_raw = String::new();
                for (sub_i, &han) in chars[c_idx..end].iter().enumerate() {
                    for py_ch in cjk_pinyin(han).chars() {
                        py_raw.push(py_ch);
                        let (kid, sh) = map_char_to_key(py_ch);
                        let step_dur = 62
                            + ((c_idx as u64 + sub_i as u64 * 7 + py_raw.len() as u64 * 13) % 28);
                        let mut composing = buf.clone();
                        composing.push_str(&py_raw);
                        events.push(DemoEvent {
                            at_ms: t_ms,
                            hold_ms: (step_dur * 3 / 4).clamp(44, 72),
                            text_after: composing,
                            key_id: Some(kid),
                            shift_held: sh,
                            kind: DemoStepKind::TypeChar,
                        });
                        t_ms += step_dur;
                    }
                }
                // Commit composed Pinyin phrase into Hanzi with Space
                buf.push_str(&phrase);
                events.push(DemoEvent {
                    at_ms: t_ms,
                    hold_ms: 72,
                    text_after: buf.clone(),
                    key_id: Some("space"),
                    shift_held: false,
                    kind: DemoStepKind::TypeChar,
                });
                t_ms += 105;
                c_idx = end;
                continue;
            }

            let ch = chars[c_idx];
            if Some(c_idx) == typo_pos {
                let wrong = 'o';
                buf.push(wrong);
                events.push(DemoEvent {
                    at_ms: t_ms,
                    hold_ms: 82,
                    text_after: buf.clone(),
                    key_id: Some("o"),
                    shift_held: false,
                    kind: DemoStepKind::TypeChar,
                });
                t_ms += 190;
                buf.pop();
                events.push(DemoEvent {
                    at_ms: t_ms,
                    hold_ms: 82,
                    text_after: buf.clone(),
                    key_id: Some("backspace"),
                    shift_held: false,
                    kind: DemoStepKind::Backspace,
                });
                t_ms += 140;
            }

            let (key_id, shift) = map_char_to_key(ch);
            buf.push(ch);
            let jitter = ((c_idx as u64 * 37 + p_idx as u64 * 19) % 54) + 68;
            let extra = if ch == ' ' || ch == '-' { 42 } else { 0 };
            let step_dur = jitter + extra;
            events.push(DemoEvent {
                at_ms: t_ms,
                hold_ms: (step_dur * 3 / 4).clamp(48, 84),
                text_after: buf.clone(),
                key_id: Some(key_id),
                shift_held: shift,
                kind: DemoStepKind::TypeChar,
            });
            t_ms += step_dur;
            c_idx += 1;
        }

        t_ms += 560;
        let submitted: SharedString = buf.clone().into();
        events.push(DemoEvent {
            at_ms: t_ms,
            hold_ms: 95,
            text_after: buf,
            key_id: Some("enter"),
            shift_held: false,
            kind: DemoStepKind::EnterSubmit(submitted),
        });
        t_ms += 220;
        events.push(DemoEvent {
            at_ms: t_ms,
            hold_ms: 0,
            text_after: String::new(),
            key_id: None,
            shift_held: false,
            kind: DemoStepKind::ClearAfterSubmit,
        });
        t_ms += 640;
    }

    (events, t_ms.max(1000))
}

/// Default three hiking prompts matching the reference specification.
pub fn default_glow_prompts(cx: &App) -> Vec<SharedString> {
    vec![
        tr(
            cx,
            "Plan a 3-day Dolomites hut-to-hut hike in June",
            "规划六月多洛米蒂三天两夜山屋徒步路线",
        )
        .into(),
        tr(
            cx,
            "Pack list for misty ridge trails under 8 kg",
            "整理 8 公斤以内的云雾山脊轻量化徒步装备清单",
        )
        .into(),
        tr(
            cx,
            "Find sunrise viewpoints near Seceda and Alpe di Siusi",
            "查找刀锋山与休斯高原附近的日出观景机位",
        )
        .into(),
    ]
}

#[derive(Clone, Copy)]
struct WavePulse {
    u_x: f32,
    born: Instant,
}

struct LiveFloorSurface {
    last_hash: u64,
    image: Option<Arc<RenderImage>>,
    color_buf: Vec<[f32; 4]>,
    color_tmp: Vec<[f32; 4]>,
    light_buf: Vec<[f32; 1]>,
    light_tmp: Vec<[f32; 1]>,
}

impl LiveFloorSurface {
    fn new() -> Self {
        let n = GW * GH;
        Self {
            last_hash: u64::MAX,
            image: None,
            color_buf: vec![[0.0; 4]; n],
            color_tmp: vec![[0.0; 4]; n],
            light_buf: vec![[0.0; 1]; n],
            light_tmp: vec![[0.0; 1]; n],
        }
    }

    fn render_frame(
        &mut self,
        dark: bool,
        bg_rgb: [u8; 3],
        phase: f32,
        static_pose: bool,
        pulses: &[(f32, f32)],
        window: &mut Window,
    ) -> Arc<RenderImage> {
        let phase_q = if static_pose {
            9999u64
        } else {
            (phase * 360.0).round() as u64
        };
        let mut h = phase_q
            ^ (u64::from(dark) << 16)
            ^ (u64::from(bg_rgb[0]) << 24)
            ^ (u64::from(bg_rgb[1]) << 32)
            ^ (u64::from(bg_rgb[2]) << 40);
        for &(ux, age) in pulses {
            let uq = (ux * 0.5).round() as u64;
            let aq = (age * 60.0).round() as u64;
            h = h.wrapping_mul(1099511628211).wrapping_add((uq << 8) ^ aq);
        }
        if self.last_hash == h
            && let Some(existing) = &self.image
        {
            return existing.clone();
        }
        let img = Arc::new(rasterize_glow_floor(
            dark,
            bg_rgb,
            if static_pose { -1.0 } else { phase },
            pulses,
            &mut self.color_buf,
            &mut self.color_tmp,
            &mut self.light_buf,
            &mut self.light_tmp,
        ));
        if let Some(prev) = self.image.replace(img.clone()) {
            let _ = window.drop_image(prev);
        }
        self.last_hash = h;
        img
    }
}

fn hsla_to_rgb8(c: Hsla) -> [u8; 3] {
    let rgba: Rgba = c.into();
    [
        (rgba.r.clamp(0., 1.) * 255.).round() as u8,
        (rgba.g.clamp(0., 1.) * 255.).round() as u8,
        (rgba.b.clamp(0., 1.) * 255.).round() as u8,
    ]
}

fn smooth_three_phase(
    t: f32,
    p33: (f32, f32, f32, f32),
    p66: (f32, f32, f32, f32),
) -> (f32, f32, f32, f32) {
    const INV_SQRT3: f32 = 0.577_350_26;
    let (s, c) = (t * TAU).sin_cos();
    let interp = |y0: f32, y1: f32, y2: f32| -> f32 {
        let a0 = (y0 + y1 + y2) * (1. / 3.);
        let a1 = y0 - a0;
        let b1 = (y1 - y2) * INV_SQRT3;
        a0 + a1 * c + b1 * s
    };
    (
        interp(0., p33.0, p66.0),
        interp(0., p33.1, p66.1),
        interp(1., p33.2, p66.2),
        interp(1., p33.3, p66.3),
    )
}

fn crest_y_svg(svg_x: f32, crests: [f32; 5]) -> Option<f32> {
    let [y_a, y_b, y_c, y_d, y_e] = crests;
    if !(291.3..=3155.8).contains(&svg_x) {
        return None;
    }
    let y = if svg_x <= 926.8 {
        let u = ((svg_x - 291.3) / (926.8 - 291.3)).clamp(0., 1.);
        let inv = 1. - u;
        inv * inv * inv * y_b
            + 3. * inv * inv * u * (y_b + 6.)
            + 3. * inv * u * u * y_a
            + u * u * u * y_a
    } else if svg_x <= 1230.8 {
        y_a
    } else if svg_x <= 1510.8 {
        let u = (svg_x - 1230.8) / (1510.8 - 1230.8);
        y_a + (y_e - y_a) * u
    } else if svg_x <= 1898.8 {
        y_e
    } else if svg_x <= 2189.3 {
        let u = (svg_x - 1898.8) / (2189.3 - 1898.8);
        y_e + (y_d - y_e) * u
    } else if svg_x <= 2493.8 {
        y_d
    } else {
        let u = (svg_x - 2493.8) / (3155.8 - 2493.8);
        y_d + (y_c - y_d) * u
    };
    Some(y)
}

fn blur_planar<const CH: usize>(
    buf: &mut [[f32; CH]],
    scratch: &mut [[f32; CH]],
    w: usize,
    h: usize,
    sigma_x: f32,
    sigma_y: f32,
) {
    let rx = ((sigma_x * 0.90).round() as usize).clamp(1, w / 2);
    let ry = ((sigma_y * 0.90).round() as usize).clamp(1, h / 2);
    for _ in 0..3 {
        let inv_x = 1. / (2 * rx + 1) as f32;
        for y in 0..h {
            let row = y * w;
            let mut acc = [0.0f32; CH];
            for _ in 0..=rx {
                for c in 0..CH {
                    acc[c] += buf[row][c];
                }
            }
            for x in 1..=rx {
                let sx = x.min(w - 1);
                for c in 0..CH {
                    acc[c] += buf[row + sx][c];
                }
            }
            for x in 0..w {
                for c in 0..CH {
                    scratch[row + x][c] = acc[c] * inv_x;
                }
                let rem = x.saturating_sub(rx);
                let add = (x + rx + 1).min(w - 1);
                for c in 0..CH {
                    acc[c] += buf[row + add][c] - buf[row + rem][c];
                }
            }
        }
        let inv_y = 1. / (2 * ry + 1) as f32;
        for x in 0..w {
            let mut acc = [0.0f32; CH];
            for _ in 0..=ry {
                for c in 0..CH {
                    acc[c] += scratch[x][c];
                }
            }
            for y in 1..=ry {
                let sy = y.min(h - 1);
                for c in 0..CH {
                    acc[c] += scratch[sy * w + x][c];
                }
            }
            for y in 0..h {
                for c in 0..CH {
                    buf[y * w + x][c] = acc[c] * inv_y;
                }
                let rem = y.saturating_sub(ry);
                let add = (y + ry + 1).min(h - 1);
                for c in 0..CH {
                    acc[c] += scratch[add * w + x][c] - scratch[rem * w + x][c];
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_glow_floor(
    dark: bool,
    bg_rgb: [u8; 3],
    phase: f32,
    pulses: &[(f32, f32)],
    color_buf: &mut [[f32; 4]],
    color_tmp: &mut [[f32; 4]],
    light_buf: &mut [[f32; 1]],
    light_tmp: &mut [[f32; 1]],
) -> RenderImage {
    let (crests0, drift_c, drift_l) = if phase < 0.0 {
        (
            [583.4, 389.3, 456.3, 587.8, 318.8],
            (0., 0., 1., 1.),
            (0., 0., 1., 1.),
        )
    } else {
        let shifted = (phase + 0.55).fract();
        let a = shifted * TAU;
        let crests = [
            565.65 + 79.55 * (a - 0.52).sin(),
            291.70 + 109.60 * (a + 2.25).sin(),
            361.45 + 109.85 * (a - 1.38).sin(),
            565.40 + 79.60 * (a + 3.10).sin(),
            429.30 + 109.50 * (a + 1.18).sin(),
        ];
        let dc = smooth_three_phase(shifted, (-120., -20., 1.05, 1.08), (100., 12., 0.98, 0.96));
        let dl = smooth_three_phase(shifted, (-100., -10., 1.03, 1.00), (85., 10., 0.98, 0.94));
        (crests, dc, dl)
    };

    let c0 = [1.0f32, 0x77 as f32 / 255., 0.0];
    let c1 = [1.0f32, 0.0, 0x55 as f32 / 255.];
    let c2 = [0.0f32, 0x73 as f32 / 255., 1.0];
    let sample_grad = |svg_y: f32| -> [f32; 3] {
        let g = ((svg_y - 423.3) / (894.3 - 423.3)).clamp(0., 1.);
        if g <= 0.302_885 {
            let u = g / 0.302_885;
            [
                c0[0] + (c1[0] - c0[0]) * u,
                c0[1] + (c1[1] - c0[1]) * u,
                c0[2] + (c1[2] - c0[2]) * u,
            ]
        } else if g <= 0.673_077 {
            let u = (g - 0.302_885) / (0.673_077 - 0.302_885);
            [
                c1[0] + (c2[0] - c1[0]) * u,
                c1[1] + (c2[1] - c1[1]) * u,
                c1[2] + (c2[2] - c1[2]) * u,
            ]
        } else {
            c2
        }
    };

    color_buf.fill([0.0; 4]);
    light_buf.fill([0.0; 1]);

    let c_left = -581.3f32;
    let c_top = STAGE_U_H - 738.3f32;
    let c_ox = c_left + 3447.1 * 0.5;
    let c_oy = c_top + 1514.1 * 0.7;

    let l_left = -310.5f32;
    let l_top = STAGE_U_H - 523.5f32;
    let l_ox = l_left + 2904.96 * 0.5;
    let l_oy = l_top + 1275.97 * 0.7;
    let l_scale_to_l0 = 3447.1 / 2904.96;

    let mut top_c = [None; GW];
    let mut top_l = [None; GW];
    let mut col_ux = [0.0f32; GW];
    for gx in 0..GW {
        let u = (gx as f32 - PAD_X as f32 + 0.5) / TEX_W as f32;
        let cx = u * STAGE_U_W;
        col_ux[gx] = cx;

        // Upward wave lift from active keystrokes (`age` in `0.0..1.0`)
        let mut wave_lift = 0.0f32;
        for &(ux, age) in pulses {
            let p = age.clamp(0., 1.);
            let env = (p * PI).sin() * (1.0 - p * 0.45);
            let dx_norm = (cx - ux) / 215.0;
            wave_lift += 265.0 * env * (-dx_norm * dx_norm).exp();
        }

        let lx0 = c_ox + (cx - drift_c.0 - c_ox) / drift_c.2 - c_left;
        top_c[gx] = crest_y_svg(lx0, crests0).map(|y| y - wave_lift);
        let lx1 = (l_ox + (cx - drift_l.0 - l_ox) / drift_l.2 - l_left) * l_scale_to_l0;
        top_l[gx] = crest_y_svg(lx1, crests0).map(|y| y - wave_lift * 0.85);
    }

    let dy0 = (STAGE_U_H / TEX_H as f32 / drift_c.3).abs().max(1.0);
    let dy1 = ((STAGE_U_H / TEX_H as f32 / drift_l.3) * l_scale_to_l0)
        .abs()
        .max(1.0);

    // Match `--ui-gk-floor-light: 1` in light mode and `0.3` in dark mode!
    let light_gain = if dark { 0.28 } else { 0.82 };

    for gy in 0..GH {
        let v = (gy as f32 - PAD_Y as f32 + 0.5) / TEX_H as f32;
        let cy = v * STAGE_U_H;
        let ly0 = c_oy + (cy - drift_c.1 - c_oy) / drift_c.3 - c_top;
        let ly1 = (l_oy + (cy - drift_l.1 - l_oy) / drift_l.3 - l_top) * l_scale_to_l0;
        let bot_cov0 = ((1222.8 - ly0) / dy0 + 0.5).clamp(0., 1.);
        let bot_cov1 = ((1222.8 - ly1) / dy1 + 0.5).clamp(0., 1.);
        let rgb = sample_grad(ly0);
        let row = gy * GW;

        for gx in 0..GW {
            if bot_cov0 > 0.0
                && let Some(ty) = top_c[gx]
            {
                let cov = ((ly0 - ty) / dy0 + 0.5).clamp(0., 1.) * bot_cov0;
                if cov > 0.0 {
                    color_buf[row + gx] = [rgb[0] * cov, rgb[1] * cov, rgb[2] * cov, cov];
                }
            }
            if bot_cov1 > 0.0
                && let Some(ty) = top_l[gx]
            {
                let cov = ((ly1 - ty) / dy1 + 0.5).clamp(0., 1.) * bot_cov1;
                if cov > 0.0 {
                    light_buf[row + gx] = [light_gain * cov];
                }
            }

            // Rising luminous wave plume (`<ellipse rx="200" ry="260">` in reference SVG[1])
            let cx = col_ux[gx];
            for &(ux, age) in pulses {
                let p = age.clamp(0., 1.);
                let plume_cy = 1040.0 - 520.0 * p;
                let ex = (cx - ux) / 185.0;
                let ey = (cy - plume_cy) / 225.0;
                let d2 = ex * ex + ey * ey;
                if d2 < 1.0 {
                    let strength = (1.0 - d2) * (1.0 - p).powf(1.25);
                    let p_rgb = [1.0, 0.32 + 0.25 * (1. - p), 0.42];
                    let prev = color_buf[row + gx];
                    let add_a = (strength * 0.72).clamp(0., 1.);
                    color_buf[row + gx] = [
                        (prev[0] + p_rgb[0] * add_a).min(1.0),
                        (prev[1] + p_rgb[1] * add_a).min(1.0),
                        (prev[2] + p_rgb[2] * add_a).min(1.0),
                        (prev[3] + add_a).min(1.0),
                    ];
                    light_buf[row + gx][0] =
                        (light_buf[row + gx][0] + strength * light_gain * 0.85).min(1.0);
                }
            }
        }
    }

    let px_per_ux = TEX_W as f32 / STAGE_U_W;
    let px_per_uy = TEX_H as f32 / STAGE_U_H;
    blur_planar(
        color_buf,
        color_tmp,
        GW,
        GH,
        145.65 * px_per_ux,
        145.65 * px_per_uy,
    );
    blur_planar(
        light_buf,
        light_tmp,
        GW,
        GH,
        122.74 * px_per_ux,
        122.74 * px_per_uy,
    );

    let bg = bg_rgb.map(|v| f32::from(v) / 255.);
    let mut raw = vec![0u8; TEX_W * TEX_H * 4];
    for y in 0..TEX_H {
        for x in 0..TEX_W {
            let src = (y + PAD_Y) * GW + (x + PAD_X);
            let [pr, pg, pb, ca] = color_buf[src];
            let ca = ca.clamp(0., 1.);
            let la = light_buf[src][0].clamp(0., 1.);
            let r = (bg[0] * (1. - ca) + pr + la).clamp(0., 1.);
            let g = (bg[1] * (1. - ca) + pg + la).clamp(0., 1.);
            let b = (bg[2] * (1. - ca) + pb + la).clamp(0., 1.);
            let dst = (y * TEX_W + x) * 4;
            raw[dst] = (b * 255.).round() as u8;
            raw[dst + 1] = (g * 255.).round() as u8;
            raw[dst + 2] = (r * 255.).round() as u8;
            raw[dst + 3] = 255;
        }
    }

    let buf = RgbaImage::from_raw(TEX_W as u32, TEX_H as u32, raw)
        .expect("TEX_W * TEX_H * 4 matches raw buffer length");
    RenderImage::new(vec![Frame::new(buf)])
}

const IDLE_RESUME_MS: u64 = 4500;

fn is_grapheme_extend(ch: char) -> bool {
    matches!(
        ch as u32,
        0x0300..=0x036F
            | 0x1AB0..=0x1AFF
            | 0x1DC0..=0x1DFF
            | 0x20D0..=0x20FF
            | 0xFE00..=0xFE0F
            | 0xFE20..=0xFE2F
            | 0x1F3FB..=0x1F3FF
            | 0xE0100..=0xE01EF
    )
}

fn is_regional_indicator(ch: char) -> bool {
    matches!(ch as u32, 0x1F1E6..=0x1F1FF)
}

/// Remove the last user-perceived grapheme cluster from `buf`.
fn pop_last_grapheme(buf: &mut String) {
    let Some(first_popped) = buf.pop() else {
        return;
    };
    if first_popped == '\n' && buf.ends_with('\r') {
        buf.pop();
        return;
    }
    if is_regional_indicator(first_popped) {
        let trailing_ri = buf
            .chars()
            .rev()
            .take_while(|&c| is_regional_indicator(c))
            .count();
        if trailing_ri % 2 == 1 {
            buf.pop();
        }
        return;
    }
    let mut cur = first_popped;
    loop {
        while is_grapheme_extend(cur) {
            let Some(prev) = buf.pop() else {
                return;
            };
            cur = prev;
        }
        if buf.ends_with('\u{200D}') {
            buf.pop();
            let Some(prev) = buf.pop() else {
                return;
            };
            cur = prev;
        } else {
            break;
        }
    }
}

fn shifted_ascii_symbol(id: &str) -> Option<char> {
    match id {
        "`" => Some('~'),
        "1" => Some('!'),
        "2" => Some('@'),
        "3" => Some('#'),
        "4" => Some('$'),
        "5" => Some('%'),
        "6" => Some('^'),
        "7" => Some('&'),
        "8" => Some('*'),
        "9" => Some('('),
        "0" => Some(')'),
        "-" => Some('_'),
        "=" => Some('+'),
        "[" => Some('{'),
        "]" => Some('}'),
        "\\" => Some('|'),
        ";" => Some(':'),
        "'" => Some('"'),
        "," => Some('<'),
        "." => Some('>'),
        "/" => Some('?'),
        _ => None,
    }
}

struct KeyCapAnim {
    hover: Slide,
    press: Slide,
    hold_until: Option<Instant>,
}

impl KeyCapAnim {
    fn new() -> Self {
        Self {
            hover: Slide::snappy(0.),
            press: Slide::snappy(0.),
            hold_until: None,
        }
    }
}

/// Interactive AI chat composer and frosted glass keyboard over a reactive gradient floor.
///
/// # Examples
///
/// ```ignore
/// use kui::components::glow_keyboard::GlowKeyboard;
///
/// let hero = cx.new(|cx| GlowKeyboard::new("hero-glow-kb", cx));
/// ```
pub struct GlowKeyboard {
    id: ElementId,
    focus: FocusHandle,
    custom_prompts: Option<Vec<SharedString>>,
    placeholder: Option<SharedString>,
    label: Option<SharedString>,
    background: Option<Hsla>,
    paused: bool,
    demo_epoch: Instant,
    last_demo_idx: Option<usize>,
    last_lang_zh: bool,
    manual_text: Option<String>,
    idle_epoch: u64,
    idle_timer: Option<Task<()>>,
    sticky_shift: bool,
    caps_lock: bool,
    hovered_key: Option<usize>,
    held_mouse_key: Option<usize>,
    held_phys_keys: Vec<usize>,
    key_anims: Vec<KeyCapAnim>,
    glow_x: Slide,
    glow_y: Slide,
    glow_energy: Slide,
    composer_hover: Slide,
    composer_press: Slide,
    submit_clock: PhaseClock,
    pulses: Vec<WavePulse>,
    keys: Vec<KeySpec>,
    height: Option<Pixels>,
    loading: bool,
    empty: bool,
    measured_w: Rc<Cell<f32>>,
    floor: Rc<RefCell<LiveFloorSurface>>,
}

impl GlowKeyboard {
    /// Construct a [`GlowKeyboard`] with three self-typing hiking prompts and full interactive keyboard support.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let keys = build_keyboard_layout();
        let key_anims = (0..keys.len()).map(|_| KeyCapAnim::new()).collect();
        let now = cx.background_executor().now();
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            custom_prompts: None,
            placeholder: None,
            label: None,
            background: None,
            paused: false,
            demo_epoch: now,
            last_demo_idx: None,
            last_lang_zh: UiLanguage::current(cx) == UiLanguage::Chinese,
            manual_text: None,
            idle_epoch: 0,
            idle_timer: None,
            sticky_shift: false,
            caps_lock: false,
            hovered_key: None,
            held_mouse_key: None,
            held_phys_keys: Vec::new(),
            key_anims,
            glow_x: Slide::new(1100.0),
            glow_y: Slide::new(757.0),
            glow_energy: Slide::snappy(0.58),
            composer_hover: Slide::snappy(0.),
            composer_press: Slide::snappy(0.),
            submit_clock: PhaseClock::default(),
            pulses: Vec::new(),
            keys,
            height: None,
            loading: false,
            empty: false,
            measured_w: Rc::new(Cell::new(860.)),
            floor: Rc::new(RefCell::new(LiveFloorSurface::new())),
        }
    }

    /// Override the demo prompts typed one after another and submitted with `Enter`.
    pub fn with_prompts<I, S>(mut self, prompts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.custom_prompts = Some(prompts.into_iter().map(Into::into).collect());
        self
    }

    /// Override the placeholder shown when the prompt input is empty (`"Ask AI anything..."` by default).
    pub fn with_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Override the accessible description of the scene.
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Override the stage background color (`#fafafa` in light mode, `#111113` in dark mode by default).
    pub fn with_background(mut self, bg: Hsla) -> Self {
        self.background = Some(bg);
        self
    }

    /// Start with self-typing demo and floor drift paused.
    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    /// Override the scene height explicitly (defaults to `11 : 6` container aspect ratio).
    pub fn with_height(mut self, height: Pixels) -> Self {
        self.height = Some(height);
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the current user-typed text (when in manual input mode).
    pub fn value(&self) -> &str {
        self.manual_text.as_deref().unwrap_or("")
    }

    /// Set the input value directly and switch into manual input mode.
    pub fn set_value(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.idle_epoch = self.idle_epoch.wrapping_add(1);
        self.idle_timer = None;
        self.manual_text = Some(value.into());
        cx.notify();
    }

    /// Return whether scene animations are paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Pause or resume the scene at runtime.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.paused != paused {
            self.paused = paused;
            cx.emit(GlowKeyboardEvent::DemoPaused(paused));
            cx.notify();
        }
    }

    /// Resume the self-typing demo loop from the beginning.
    pub fn resume_demo(&mut self, cx: &mut Context<Self>) {
        self.idle_epoch = self.idle_epoch.wrapping_add(1);
        self.idle_timer = None;
        self.manual_text = None;
        self.sticky_shift = false;
        self.paused = false;
        self.demo_epoch = cx.background_executor().now();
        self.last_demo_idx = None;
        cx.emit(GlowKeyboardEvent::DemoPaused(false));
        cx.notify();
    }

    /// Return whether skeleton loading is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle skeleton loading at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    fn key_index(&self, key_id: &str) -> Option<usize> {
        self.keys
            .iter()
            .position(|k| k.id.eq_ignore_ascii_case(key_id))
    }

    fn key_stage_center(&self, key_id: &str) -> (f32, f32) {
        self.keys
            .iter()
            .find(|k| k.id.eq_ignore_ascii_case(key_id))
            .map(|k| (312.5 + k.x + k.w * 0.5, 440.0 + k.y + k.h * 0.5))
            .unwrap_or((1100.0, 757.0))
    }

    fn arm_idle_resume(&mut self, cx: &mut Context<Self>) {
        self.idle_epoch = self.idle_epoch.wrapping_add(1);
        let epoch = self.idle_epoch;
        self.idle_timer = Some(cx.spawn(async move |weak, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(IDLE_RESUME_MS))
                .await;
            let _ = weak.update(cx, |this, cx| {
                if this.idle_epoch == epoch && this.manual_text.is_some() && !this.paused {
                    this.manual_text = None;
                    this.sticky_shift = false;
                    this.demo_epoch = cx.background_executor().now();
                    this.last_demo_idx = None;
                    this.glow_x.duration(1100.0, 0.45, cx);
                    this.glow_y.duration(757.0, 0.45, cx);
                    this.glow_energy.duration(0.58, 0.45, cx);
                    cx.notify();
                }
            });
        }));
    }

    fn trigger_key(
        &mut self,
        key_id: &'static str,
        shift: bool,
        hold_phys: bool,
        cx: &mut Context<Self>,
    ) {
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let tap_until = now + Duration::from_millis(105);
        if let Some(idx) = self.key_index(key_id) {
            self.key_anims[idx].hold_until = Some(tap_until);
            self.key_anims[idx].press.set(1.0, cx);
            if hold_phys && !self.held_phys_keys.contains(&idx) {
                self.held_phys_keys.push(idx);
            }
        }
        if shift && let Some(s_idx) = self.key_index("shift-l") {
            self.key_anims[s_idx].hold_until = Some(tap_until);
            self.key_anims[s_idx].press.set(1.0, cx);
            if hold_phys && !self.held_phys_keys.contains(&s_idx) {
                self.held_phys_keys.push(s_idx);
            }
        }
        let (ux, uy) = self.key_stage_center(key_id);
        self.glow_x.duration(ux, 0.22, cx);
        self.glow_y.duration(uy, 0.22, cx);
        self.glow_energy.set(1.0, cx);
        if !reduced {
            self.pulses
                .retain(|p| now.saturating_duration_since(p.born).as_secs_f32() < 0.90);
            if self.pulses.len() >= 8 {
                self.pulses.remove(0);
            }
            self.pulses.push(WavePulse { u_x: ux, born: now });
        }
    }

    fn release_physical_key(&mut self, key: &str, key_char: Option<&str>, cx: &mut Context<Self>) {
        if matches!(key, "shift" | "shift-l" | "shift-r") {
            for sid in ["shift-l", "shift-r"] {
                if let Some(idx) = self.key_index(sid) {
                    self.held_phys_keys.retain(|&i| i != idx);
                }
            }
            cx.notify();
            return;
        }
        let mapped = match key {
            " " | "space" => Some("space"),
            "enter" => Some("enter"),
            "backspace" => Some("backspace"),
            "escape" => Some("escape"),
            "tab" => Some("tab"),
            "capslock" => Some("capslock"),
            "left" => Some("left"),
            "right" => Some("right"),
            "up" => Some("up"),
            "down" => Some("down"),
            _ => self.key_index(key).map(|i| self.keys[i].id).or_else(|| {
                key_char
                    .and_then(|s| s.chars().next())
                    .map(|ch| map_char_to_key(ch).0)
            }),
        };
        if let Some(kid) = mapped
            && let Some(idx) = self.key_index(kid)
        {
            self.held_phys_keys.retain(|&i| i != idx);
        }
        if self.manual_text.is_some() {
            self.arm_idle_resume(cx);
        }
        cx.notify();
    }

    fn handle_manual_key(
        &mut self,
        key: &str,
        key_char: Option<&str>,
        shift: bool,
        from_physical: bool,
        cx: &mut Context<Self>,
    ) {
        if matches!(key, "shift-l" | "shift-r" | "shift") {
            let sid = if key == "shift-r" {
                "shift-r"
            } else {
                "shift-l"
            };
            if !from_physical {
                self.sticky_shift = !self.sticky_shift;
            }
            if self.manual_text.is_none() {
                self.manual_text = Some(String::new());
            }
            self.trigger_key(sid, false, from_physical, cx);
            self.arm_idle_resume(cx);
            cx.notify();
            return;
        }
        if key == "capslock" {
            self.caps_lock = !self.caps_lock;
            if self.manual_text.is_none() {
                self.manual_text = Some(String::new());
            }
            self.trigger_key("capslock", false, from_physical, cx);
            self.arm_idle_resume(cx);
            cx.notify();
            return;
        }
        if key == "escape" {
            self.trigger_key("escape", false, from_physical, cx);
            self.resume_demo(cx);
            return;
        }

        if self.manual_text.is_none() {
            self.manual_text = Some(String::new());
        }
        let eff_shift = shift || self.sticky_shift;
        self.sticky_shift = false;

        match key {
            "enter" => {
                self.trigger_key("enter", false, from_physical, cx);
                self.submit_clock.restart(cx);
                let submitted = self.manual_text.as_deref().unwrap_or("").trim().to_owned();
                if !submitted.is_empty() {
                    cx.emit(GlowKeyboardEvent::Submit(submitted.into()));
                }
                if let Some(buf) = self.manual_text.as_mut() {
                    buf.clear();
                }
                cx.emit(GlowKeyboardEvent::Change(SharedString::default()));
            }
            "backspace" => {
                self.trigger_key("backspace", false, from_physical, cx);
                if let Some(buf) = self.manual_text.as_mut() {
                    pop_last_grapheme(buf);
                    cx.emit(GlowKeyboardEvent::Change(buf.clone().into()));
                }
            }
            "tab" => {
                self.trigger_key("tab", false, from_physical, cx);
            }
            "left" => self.trigger_key("left", false, from_physical, cx),
            "right" => self.trigger_key("right", false, from_physical, cx),
            "up" => self.trigger_key("up", false, from_physical, cx),
            "down" => self.trigger_key("down", false, from_physical, cx),
            "fn" | "ctrl" | "alt-l" | "alt-r" | "cmd-l" | "cmd-r" | "touchid" => {
                if let Some(idx) = self.key_index(key) {
                    let kid = self.keys[idx].id;
                    self.trigger_key(kid, false, from_physical, cx);
                }
            }
            " " | "space" => {
                self.trigger_key("space", false, from_physical, cx);
                if let Some(buf) = self.manual_text.as_mut() {
                    buf.push(' ');
                    cx.emit(GlowKeyboardEvent::Change(buf.clone().into()));
                }
            }
            _ => {
                if let Some(idx) = self.key_index(key)
                    && self.keys[idx].style == KeyStyle::FKey
                {
                    let kid = self.keys[idx].id;
                    self.trigger_key(kid, false, from_physical, cx);
                    self.arm_idle_resume(cx);
                    cx.notify();
                    return;
                }
                let candidate = key_char
                    .and_then(|s| {
                        let mut chars = s.chars();
                        let c = chars.next()?;
                        (!c.is_control()).then_some(c)
                    })
                    .or_else(|| {
                        if eff_shift && let Some(sym) = shifted_ascii_symbol(key) {
                            return Some(sym);
                        }
                        let mut it = key.chars();
                        let first = it.next()?;
                        it.next().is_none().then_some(first)
                    });
                if let Some(mut ch) = candidate
                    && !ch.is_control()
                {
                    if ch.is_ascii_alphabetic() {
                        if eff_shift ^ self.caps_lock {
                            ch = ch.to_ascii_uppercase();
                        } else {
                            ch = ch.to_ascii_lowercase();
                        }
                    } else if eff_shift
                        && key_char.is_none()
                        && let Some(sym) = shifted_ascii_symbol(&ch.to_string())
                    {
                        ch = sym;
                    }
                    let (kid, sh) = map_char_to_key(ch);
                    self.trigger_key(kid, sh || eff_shift, from_physical, cx);
                    if let Some(buf) = self.manual_text.as_mut() {
                        buf.push(ch);
                        cx.emit(GlowKeyboardEvent::Change(buf.clone().into()));
                    }
                }
            }
        }
        self.arm_idle_resume(cx);
        cx.notify();
    }
}

impl Render for GlowKeyboard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let reduced = disabled_motion(cx);
        let now = cx.background_executor().now();

        if self.loading {
            let phase = if reduced {
                0.0
            } else {
                w.request_animation_frame();
                (now.saturating_duration_since(self.demo_epoch).as_secs_f32() / 1.4).fract()
            };
            return div()
                .id(pro_eid(&self.id, "loading"))
                .w_full()
                .h(self.height.unwrap_or(px(420.)))
                .rounded(px(20.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(22.))
                .child(pro_skeleton_bar(320., 46., 23., phase, cx))
                .child(pro_skeleton_bar(480., 196., 20., phase, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(pro_eid(&self.id, "empty"))
                .w_full()
                .child(pro_empty_state::<Self>(
                    ProIcon::Terminal,
                    tr(cx, "No prompts configured", "暂无提示词序列"),
                    tr(
                        cx,
                        "Provide prompts or type directly to light up the glass keyboard.",
                        "添加演示提示词或直接输入即可点亮磨砂玻璃键盘。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        if !self.focus.is_focused(w) && !self.held_phys_keys.is_empty() {
            self.held_phys_keys.clear();
        }

        let is_zh = UiLanguage::current(cx) == UiLanguage::Chinese;
        if self.last_lang_zh != is_zh {
            self.last_lang_zh = is_zh;
            self.demo_epoch = now;
            self.last_demo_idx = None;
        }
        let prompts = self
            .custom_prompts
            .clone()
            .unwrap_or_else(|| default_glow_prompts(cx));

        let demo_mode = self.manual_text.is_none();
        let mut displayed_text = self.manual_text.clone().unwrap_or_default();
        let mut demo_key_down: Option<&'static str> = None;
        let mut demo_shift_down = false;

        if demo_mode && !self.paused && !reduced && !prompts.is_empty() {
            let (timeline, total_ms) = compile_demo_timeline(&prompts);
            let cycle_ms =
                (now.saturating_duration_since(self.demo_epoch).as_millis() as u64) % total_ms;
            let active_idx = timeline.partition_point(|ev| ev.at_ms <= cycle_ms);
            if active_idx > 0 {
                let ev = &timeline[active_idx - 1];
                displayed_text.clone_from(&ev.text_after);
                let since_ev = cycle_ms.saturating_sub(ev.at_ms);
                if ev.hold_ms > 0 && since_ev < ev.hold_ms {
                    demo_key_down = ev.key_id;
                    demo_shift_down = ev.shift_held;
                }
                if self.last_demo_idx != Some(active_idx - 1) {
                    self.last_demo_idx = Some(active_idx - 1);
                    if let Some(kid) = ev.key_id {
                        let (ux, uy) = self.key_stage_center(kid);
                        self.glow_x.duration(ux, 0.26, cx);
                        self.glow_y.duration(uy, 0.26, cx);
                        self.glow_energy.set(0.92, cx);
                        self.pulses.retain(|pl| {
                            now.saturating_duration_since(pl.born).as_secs_f32() < 0.90
                        });
                        if self.pulses.len() >= 8 {
                            self.pulses.remove(0);
                        }
                        self.pulses.push(WavePulse { u_x: ux, born: now });
                    }
                    if let DemoStepKind::EnterSubmit(submitted) = &ev.kind {
                        self.submit_clock.restart(cx);
                        cx.emit(GlowKeyboardEvent::Submit(submitted.clone()));
                    }
                }
            } else {
                self.last_demo_idx = None;
            }
        }

        let mut any_key_down = false;
        for (k_idx, k) in self.keys.iter().enumerate() {
            let tap_active = self.key_anims[k_idx]
                .hold_until
                .is_some_and(|until| now < until);
            if !tap_active {
                self.key_anims[k_idx].hold_until = None;
            }
            let mouse_down = self.held_mouse_key == Some(k_idx);
            let phys_down = self.held_phys_keys.contains(&k_idx);
            let auto_down = demo_key_down == Some(k.id) || (demo_shift_down && k.id == "shift-l");
            let sticky_down = self.sticky_shift && k.id == "shift-l";
            let is_down = tap_active || mouse_down || phys_down || auto_down || sticky_down;
            if is_down {
                any_key_down = true;
            }
            self.key_anims[k_idx]
                .press
                .set(if is_down { 1.0 } else { 0.0 }, cx);
            self.key_anims[k_idx].hover.set(
                if self.hovered_key == Some(k_idx) {
                    1.0
                } else {
                    0.0
                },
                cx,
            );
        }

        if !any_key_down {
            if let Some(hov_idx) = self.hovered_key {
                let hk = self.keys[hov_idx];
                self.glow_x.duration(312.5 + hk.x + hk.w * 0.5, 0.34, cx);
                self.glow_y.duration(440.0 + hk.y + hk.h * 0.5, 0.34, cx);
                self.glow_energy.duration(0.76, 0.30, cx);
            } else {
                self.glow_energy.duration(0.54, 0.48, cx);
            }
        }

        self.pulses
            .retain(|pl| now.saturating_duration_since(pl.born).as_secs_f32() < 0.90);

        let measured_cell = self.measured_w.clone();
        let stage_w = measured_cell.get().max(340.);
        let stage_h = self
            .height
            .map(f32::from)
            .unwrap_or_else(|| (stage_w * (STAGE_U_H / STAGE_U_W)).round().max(190.));
        let u = stage_w / STAGE_U_W;

        let default_bg: Hsla = if dark {
            rgb(0x111113).into()
        } else {
            rgb(0xfafafa).into()
        };
        let bg_hsla = self.background.unwrap_or(default_bg);
        let bg_rgb = hsla_to_rgb8(bg_hsla);

        let animate_floor = !reduced && !self.paused;
        let has_live_pulses = !self.pulses.is_empty() || any_key_down;
        let elapsed_demo_s = now.saturating_duration_since(self.demo_epoch).as_secs_f32();
        let floor_phase = if animate_floor {
            (elapsed_demo_s / 6.0).fract()
        } else {
            0.0
        };
        let pulse_pairs: Vec<(f32, f32)> = self
            .pulses
            .iter()
            .map(|pl| {
                (
                    pl.u_x,
                    (now.saturating_duration_since(pl.born).as_secs_f32() / 0.85).clamp(0., 1.),
                )
            })
            .collect();

        let floor_surface = self.floor.clone();
        let request_next = animate_floor || has_live_pulses;

        // Sample 2D keyboard backlight spring + composer tactile state
        let cur_glow_ux = self.glow_x.get(w, cx).clamp(320.0, 1880.0);
        let cur_glow_uy = self.glow_y.get(w, cx).clamp(455.0, 1060.0);
        let cur_glow_e = self.glow_energy.get(w, cx).clamp(0.0, 1.2);
        let comp_hov = self.composer_hover.get(w, cx).clamp(0.0, 1.0);
        let comp_down = self.composer_press.get(w, cx).clamp(0.0, 1.0);
        let submit_age = self.submit_clock.time(0.52, w, cx);
        let submit_pulse = if submit_age < 0.52 && !reduced {
            (1.0 - submit_age / 0.52).powi(2)
        } else {
            0.0
        };

        // Layer 1: Two-layer Gaussian gradient floor + rising keystroke wave plumes
        let floor_canvas = canvas(
            move |bounds, window, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 40. && (bw - measured_cell.get()).abs() > 1.0 {
                    measured_cell.set(bw);
                    window.request_animation_frame();
                }
            },
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 4. || bh <= 4. {
                    return;
                }
                let img = floor_surface.borrow_mut().render_frame(
                    dark,
                    bg_rgb,
                    floor_phase,
                    !animate_floor && pulse_pairs.is_empty(),
                    &pulse_pairs,
                    window,
                );
                let _ = window.paint_image(bounds, bounds, Corners::all(px(20.)), img, 0, false);
                if request_next {
                    window.request_animation_frame();
                }
            },
        )
        .absolute()
        .inset_0();

        // Layer 2: Floating AI Chat Input Capsule + ambient orange-rose halo + key-hit flashes
        let comp_x = 577.0 * u;
        let comp_y = 224.0 * u;
        let comp_w = 1046.0 * u;
        let comp_h = (153.0 * u).max(34.);
        let comp_r = comp_h * 0.5;
        let orb_side = (95.6 * u).max(20.) * (1.0 + 0.06 * submit_pulse);

        let input_bg: Hsla = if dark {
            hsla(
                240. / 360.,
                0.06,
                0.10 + 0.02 * comp_hov,
                0.88 + 0.05 * comp_hov,
            )
        } else {
            hsla(0., 0., 1., 0.91 + 0.06 * comp_hov)
        };
        let kb_focused = self.focus.is_focused(w);
        let input_bdr: Hsla = if submit_pulse > 0.05 {
            pro_mix_color(
                if dark {
                    hsla(0., 0., 1., 0.18)
                } else {
                    rgb(0xffffff).into()
                },
                hsla(18. / 360., 1.0, 0.60, 0.85),
                submit_pulse,
            )
        } else if kb_focused && !demo_mode {
            hsla(24. / 360., 0.95, 0.60, if dark { 0.52 } else { 0.68 })
        } else if dark {
            hsla(0., 0., 1., 0.14 + 0.08 * comp_hov)
        } else {
            rgb(0xffffff).into()
        };
        let prompt_fg: Hsla = if dark {
            rgb(0xe4e4e7).into()
        } else {
            rgb(0x676767).into()
        };
        let placeholder_fg: Hsla = if dark {
            rgb(0x737378).into()
        } else {
            rgb(0xafafaf).into()
        };
        let caret_c: Hsla = if !demo_mode && kb_focused {
            hsla(18. / 360., 0.95, 0.56, 0.82)
        } else if dark {
            hsla(0., 0., 1., 0.34)
        } else {
            hsla(0., 0., 0., 0.28)
        };

        let placeholder_str: SharedString = self
            .placeholder
            .clone()
            .unwrap_or_else(|| tr(cx, "Ask AI anything...", "向 AI 提问任何事...").into());
        let is_empty_prompt = displayed_text.is_empty();
        let caret_on = if reduced {
            true
        } else {
            ((now.saturating_duration_since(self.demo_epoch).as_millis()) / 530).is_multiple_of(2)
        };

        let breathe_t = if animate_floor {
            0.78 + 0.22 * (elapsed_demo_s * TAU / 4.0).sin()
        } else {
            0.82
        } + 0.45 * submit_pulse;

        let mut composer_layer = div().absolute().inset_0();
        for pl in &self.pulses {
            let ux = pl.u_x;
            let age = now.saturating_duration_since(pl.born).as_secs_f32();
            if age < 0.44 {
                let flash_a = (1.0 - age / 0.44).powi(2) * if dark { 0.52 } else { 0.68 };
                let hit_w = 320.0 * u;
                let hit_x = (ux * u - hit_w * 0.5).clamp(comp_x, comp_x + comp_w - hit_w);
                composer_layer = composer_layer.child(
                    div()
                        .absolute()
                        .left(px(hit_x))
                        .top(px(comp_y))
                        .w(px(hit_w))
                        .h(px(comp_h))
                        .rounded(px(comp_r))
                        .shadow(vec![
                            BoxShadow::new(px(0.), px(0.), hsla(346. / 360., 1.0, 0.56, flash_a))
                                .blur_radius(px(26. * u)),
                        ]),
                );
            }
        }

        let halo_shift_x = ((cur_glow_ux - 1100.0) / 550.0).clamp(-1.0, 1.0) * 22.0 * u;
        let comp_inset = comp_down * 1.8 * u;
        let composer_pill = div()
            .id(pro_eid(&self.id, "composer"))
            .role(Role::Button)
            .aria_label(placeholder_str.clone())
            .absolute()
            .left(px(comp_x + comp_inset))
            .top(px(comp_y + comp_inset * 0.8))
            .w(px((comp_w - 2.0 * comp_inset).max(40.)))
            .h(px((comp_h - 1.6 * comp_inset).max(28.)))
            .rounded(px(comp_r))
            .px(px((27.0 * u).max(8.)))
            .bg(input_bg)
            .border_1()
            .border_color(input_bdr)
            .cursor_text()
            .shadow(vec![
                BoxShadow::new(
                    px(-18. * u + halo_shift_x),
                    px(2. * u),
                    hsla(28. / 360., 1.0, 0.50, (0.32 * breathe_t).clamp(0., 0.85)),
                )
                .blur_radius(px((32. + 14. * submit_pulse) * u)),
                BoxShadow::new(
                    px(18. * u + halo_shift_x),
                    px(2. * u),
                    hsla(340. / 360., 1.0, 0.50, (0.32 * breathe_t).clamp(0., 0.85)),
                )
                .blur_radius(px((32. + 14. * submit_pulse) * u)),
                BoxShadow::new(
                    px(halo_shift_x * 0.5),
                    px(4. * u),
                    hsla(8. / 360., 1.0, 0.58, (0.26 * breathe_t).clamp(0., 0.80)),
                )
                .blur_radius(px((16. + 12. * submit_pulse) * u)),
            ])
            .flex()
            .items_center()
            .gap(px((20.0 * u).max(8.)))
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                this.composer_hover.set(if *inside { 1.0 } else { 0.0 }, cx);
                if !*inside {
                    this.composer_press.set(0.0, cx);
                }
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, w, cx| {
                    this.focus.focus(w, cx);
                    this.composer_press.set(1.0, cx);
                    if this.manual_text.is_none() {
                        this.manual_text = Some(String::new());
                    }
                    this.arm_idle_resume(cx);
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.composer_press.set(0.0, cx);
                    cx.notify();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.composer_press.set(0.0, cx);
                    cx.notify();
                }),
            )
            .child(pro_raw_svg(ORB_SVG, orb_side, orb_side))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_size(px((46.0 * u).clamp(12., 24.)))
                    .line_height(px((58.0 * u).clamp(16., 30.)))
                    .text_color(if is_empty_prompt {
                        placeholder_fg
                    } else {
                        prompt_fg
                    })
                    .when(is_empty_prompt && caret_on, |d| {
                        d.child(
                            div()
                                .w(px((2.8 * u).max(1.5)))
                                .h(px((56.0 * u).clamp(14., 26.)))
                                .mr(px(4.))
                                .bg(caret_c),
                        )
                    })
                    .child(if is_empty_prompt {
                        placeholder_str
                    } else {
                        displayed_text.clone().into()
                    })
                    .when(!is_empty_prompt && caret_on, |d| {
                        d.child(
                            div()
                                .w(px((2.8 * u).max(1.5)))
                                .h(px((56.0 * u).clamp(14., 26.)))
                                .ml(px(2.))
                                .bg(caret_c),
                        )
                    }),
            );

        composer_layer = composer_layer.child(composer_pill);

        // Layer 3: Frosted Glass Mechanical Keyboard (`left = 312.5u, top = 440u, w = 1575u, h = 634u`)
        let board_x = 312.5 * u;
        let board_y = 440.0 * u;
        let board_w = 1575.0 * u;
        let board_h = 634.0 * u;
        let board_r = (52.0 * u).max(12.);

        let board_bg: Hsla = if dark {
            hsla(240. / 360., 0.07, 0.06, 0.54)
        } else {
            hsla(0., 0., 1., 0.22)
        };
        let board_bdr: Hsla = if dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 1., 0.58)
        };

        let key_rest_bg: Hsla = if dark {
            hsla(240. / 360., 0.05, 0.13, 0.86)
        } else {
            hsla(0., 0., 1., 0.74)
        };
        let key_hover_bg: Hsla = if dark {
            hsla(240. / 360., 0.06, 0.19, 0.92)
        } else {
            hsla(0., 0., 1., 0.88)
        };
        let key_down_bg: Hsla = if dark {
            hsla(240. / 360., 0.06, 0.26, 0.98)
        } else {
            hsla(240. / 360., 0.10, 0.84, 0.98)
        };
        let key_rest_fg: Hsla = if dark {
            hsla(240. / 360., 0.20, 0.94, 0.60)
        } else {
            hsla(240. / 360., 0.20, 0.10, 0.52)
        };
        let key_down_fg: Hsla = if dark {
            hsla(0., 0., 1., 0.96)
        } else {
            hsla(240. / 360., 0.20, 0.10, 0.92)
        };
        let key_bdr: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 1., 0.65)
        };

        // 2D spring-driven warm ambient backlight beneath the frosted keycaps
        let glow_local_x = (cur_glow_ux - 312.5) * u;
        let glow_local_y = (cur_glow_uy - 440.0) * u;
        let glow_outer_r = (260.0 * u).max(48.0);
        let glow_inner_r = (128.0 * u).max(24.0);
        let glow_alpha_scale = if reduced { 0.35 } else { cur_glow_e };

        let backlight_layer = div()
            .absolute()
            .inset_0()
            .rounded(px(board_r))
            .overflow_hidden()
            .child(
                div()
                    .absolute()
                    .left(px(glow_local_x - glow_outer_r))
                    .top(px(glow_local_y - glow_outer_r * 0.72))
                    .w(px(glow_outer_r * 2.0))
                    .h(px(glow_outer_r * 1.44))
                    .rounded_full()
                    .shadow(vec![
                        BoxShadow::new(
                            px(0.),
                            px(0.),
                            hsla(
                                24. / 360.,
                                1.0,
                                0.56,
                                ((if dark { 0.36 } else { 0.30 }) * glow_alpha_scale)
                                    .clamp(0., 0.62),
                            ),
                        )
                        .blur_radius(px((76.0 * u).max(18.0))),
                        BoxShadow::new(
                            px(0.),
                            px(0.),
                            hsla(
                                342. / 360.,
                                1.0,
                                0.58,
                                ((if dark { 0.28 } else { 0.24 }) * glow_alpha_scale)
                                    .clamp(0., 0.50),
                            ),
                        )
                        .blur_radius(px((112.0 * u).max(26.0))),
                    ]),
            )
            .child(
                div()
                    .absolute()
                    .left(px(glow_local_x - glow_inner_r))
                    .top(px(glow_local_y - glow_inner_r * 0.72))
                    .w(px(glow_inner_r * 2.0))
                    .h(px(glow_inner_r * 1.44))
                    .rounded_full()
                    .bg(hsla(
                        28. / 360.,
                        1.0,
                        0.62,
                        ((if dark { 0.16 } else { 0.14 }) * glow_alpha_scale).clamp(0., 0.32),
                    )),
            );

        let mut board_el = div()
            .absolute()
            .left(px(board_x))
            .top(px(board_y))
            .w(px(board_w))
            .h(px(board_h))
            .rounded(px(board_r))
            .bg(board_bg)
            .border_1()
            .border_color(board_bdr)
            .shadow(vec![
                BoxShadow::new(
                    px(0.),
                    px(18. * u),
                    black().opacity(if dark { 0.45 } else { 0.06 }),
                )
                .blur_radius(px(36. * u)),
            ])
            .child(backlight_layer);

        let keys_snapshot = self.keys.clone();
        let caps_on = self.caps_lock;
        for (k_idx, k) in keys_snapshot.iter().enumerate() {
            let hov_t = self.key_anims[k_idx].hover.get(w, cx).clamp(0.0, 1.0);
            let down_t = self.key_anims[k_idx].press.get(w, cx).clamp(0.0, 1.0);

            let press_shift_y = if reduced {
                0.0
            } else {
                3.6 * u * down_t - 0.8 * u * hov_t * (1.0 - down_t)
            };
            let scale_inset = if reduced { 0.0 } else { 2.8 * u * down_t };

            let kr = match k.style {
                KeyStyle::FKey | KeyStyle::TouchId => 14.0 * u,
                KeyStyle::HalfArrow => 11.0 * u,
                _ => 20.0 * u,
            }
            .max(3.);

            let fs = match k.style {
                KeyStyle::Alpha => 40.0 * u,
                KeyStyle::Mod => 33.0 * u,
                KeyStyle::ModSmall | KeyStyle::Arrow => 26.0 * u,
                KeyStyle::FKey | KeyStyle::HalfArrow | KeyStyle::TouchId => 21.0 * u,
            }
            .max(7.);

            // Proximity illumination from 2D backlight spring
            let key_ux = 312.5 + k.x + k.w * 0.5;
            let key_uy = 440.0 + k.y + k.h * 0.5;
            let dist_sq =
                ((key_ux - cur_glow_ux) / 230.0).powi(2) + ((key_uy - cur_glow_uy) / 170.0).powi(2);
            let proximity = ((-dist_sq).exp() * cur_glow_e * 0.42).clamp(0.0, 0.42);

            let base_face = pro_mix_color(key_rest_bg, key_hover_bg, hov_t);
            let face_bg = pro_mix_color(base_face, key_down_bg, down_t);
            let warm_rim = hsla(18. / 360., 0.95, 0.62, 0.76);
            let bdr_col = pro_mix_color(
                pro_mix_color(key_bdr, warm_rim.opacity(0.45), hov_t * 0.6 + proximity),
                warm_rim,
                down_t,
            );
            let fg_col = pro_mix_color(
                pro_mix_color(key_rest_fg, key_down_fg, hov_t * 0.55),
                key_down_fg,
                down_t,
            );

            let kid = k.id;
            let mut key_div = div()
                .id(pro_eid(&self.id, format!("key-{k_idx}")))
                .role(Role::Button)
                .aria_label(if k.legend.is_empty() { k.id } else { k.legend })
                .absolute()
                .left(px(k.x * u + scale_inset * 0.5))
                .top(px(k.y * u + press_shift_y + scale_inset * 0.5))
                .w(px((k.w * u - scale_inset).max(6.)))
                .h(px((k.h * u - scale_inset).max(6.)))
                .rounded(px(kr))
                .bg(face_bg)
                .border_1()
                .border_color(bdr_col)
                .when(down_t > 0.08 || hov_t > 0.15, |d| {
                    let sh_a = ((if dark { 0.42 } else { 0.34 }) * down_t
                        + (if dark { 0.16 } else { 0.12 }) * hov_t * (1.0 - down_t))
                        .clamp(0.0, 0.52);
                    d.shadow(vec![
                        BoxShadow::new(px(0.), px(2. * u), hsla(18. / 360., 0.96, 0.58, sh_a))
                            .blur_radius(px((18. * u).max(6.))),
                    ])
                })
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_size(px(fs))
                .line_height(px(fs))
                .text_color(fg_col)
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside {
                        this.hovered_key = Some(k_idx);
                    } else if this.hovered_key == Some(k_idx) {
                        this.hovered_key = None;
                    }
                    if this.manual_text.is_some() {
                        this.arm_idle_resume(cx);
                    }
                    cx.notify();
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, e: &MouseDownEvent, w, cx| {
                        this.focus.focus(w, cx);
                        this.held_mouse_key = Some(k_idx);
                        this.handle_manual_key(kid, None, e.modifiers.shift, false, cx);
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        if this.held_mouse_key == Some(k_idx) {
                            this.held_mouse_key = None;
                            cx.notify();
                        }
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        if this.held_mouse_key == Some(k_idx) {
                            this.held_mouse_key = None;
                            cx.notify();
                        }
                    }),
                );

            if k.id == "capslock" {
                let led_d = (9.0 * u).max(3.0);
                key_div = key_div.child(
                    div()
                        .absolute()
                        .left(px((14.0 * u).max(3.0)))
                        .top(px((12.0 * u).max(3.0)))
                        .size(px(led_d))
                        .rounded_full()
                        .bg(if caps_on {
                            rgb(0x4ade80)
                        } else {
                            rgb(0x737378)
                        }
                        .opacity(if caps_on { 0.95 } else { 0.25 })),
                );
            }

            if k.style == KeyStyle::TouchId {
                let ring_d = (28.0 * u).max(6.);
                key_div = key_div.child(
                    div()
                        .size(px(ring_d))
                        .rounded(px(999.))
                        .border_1()
                        .border_color(fg_col.opacity(0.50)),
                );
            } else if let Some(svg_inner) = match k.id {
                "backspace" => Some(
                    r#"<path d="M9.5 6.5H19.5C20.33 6.5 21 7.17 21 8V16C21 16.83 20.33 17.5 19.5 17.5H9.5L4 12L9.5 6.5Z" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/><path d="M12.5 9.8L16.7 14.2M16.7 9.8L12.5 14.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>"#,
                ),
                "tab" => Some(
                    r#"<path d="M4 12H16M16 12L11.5 7.5M16 12L11.5 16.5M19.5 6.5V17.5" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"/>"#,
                ),
                "capslock" => Some(
                    r#"<path d="M12 4.5L5.5 11.5H9.2V15.5H14.8V11.5H18.5L12 4.5Z" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"/><path d="M9.2 18.8H14.8" stroke="currentColor" stroke-width="1.9" stroke-linecap="round"/>"#,
                ),
                "enter" => Some(
                    r#"<path d="M19 6.5V12.5C19 13.6 18.1 14.5 17 14.5H6M6 14.5L9.8 10.7M6 14.5L9.8 18.3" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"/>"#,
                ),
                "ctrl" => Some(
                    r#"<path d="M6.5 14.5L12 8.5L17.5 14.5" stroke="currentColor" stroke-width="2.0" stroke-linecap="round" stroke-linejoin="round"/>"#,
                ),
                "alt-l" | "alt-r" => Some(
                    r#"<path d="M4.5 7.5H9.2L14.8 16.5H19.5M14.2 7.5H19.5" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"/>"#,
                ),
                _ => None,
            } {
                let icon_side = (fs * 1.05).max(8.);
                key_div = key_div.child(pro_svg("0 0 24 24", svg_inner, icon_side, fg_col));
            } else if !k.legend.is_empty() {
                key_div = key_div.child(k.legend);
            }

            board_el = board_el.child(key_div);
        }

        let a11y_label: SharedString = self.label.clone().unwrap_or_else(|| {
            tr(
                cx,
                "AI chat input above a glass keyboard and a moving color gradient",
                "悬浮在磨砂玻璃机械键盘与动态流光底幕上方的 AI 对话输入框",
            )
            .into()
        });

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(a11y_label)
            .track_focus(&self.focus)
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(stage_h))
            .rounded(px(20.))
            .overflow_hidden()
            .bg(bg_hsla)
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.border_color(p.focus))
            .on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
                if e.keystroke.modifiers.platform || e.keystroke.modifiers.control {
                    return;
                }
                this.handle_manual_key(
                    e.keystroke.key.as_str(),
                    e.keystroke.key_char.as_deref(),
                    e.keystroke.modifiers.shift,
                    true,
                    cx,
                );
                cx.stop_propagation();
            }))
            .on_key_up(cx.listener(move |this, e: &KeyUpEvent, _, cx| {
                this.release_physical_key(
                    e.keystroke.key.as_str(),
                    e.keystroke.key_char.as_deref(),
                    cx,
                );
            }))
            .child(floor_canvas)
            .child(composer_layer)
            .child(board_el)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DemoStepKind, GH, GW, GlowKeyboard, GlowKeyboardEvent, IDLE_RESUME_MS, TEX_H, TEX_W,
        build_keyboard_layout, compile_demo_timeline, map_char_to_key, pop_last_grapheme,
        rasterize_glow_floor,
    };
    use gpui::{SharedString, TestAppContext, px, size};
    use std::{cell::RefCell, rc::Rc, time::Duration};

    #[test]
    fn keyboard_rows_align_flush_and_demo_timeline_includes_shift_and_typo_fix() {
        let keys = build_keyboard_layout();
        assert!(keys.len() >= 76);
        assert_eq!(map_char_to_key('D'), ("d", true));
        assert_eq!(map_char_to_key('h'), ("h", false));

        let prompts: Vec<SharedString> =
            vec!["Plan a 3-day Dolomites hike".into(), "Sunrise trail".into()];
        let (events, total_ms) = compile_demo_timeline(&prompts);
        assert!(total_ms > 3000);
        // Verifies Shift is held for capital 'P' / 'D', one typo is corrected with Backspace, and both prompts submit.
        assert!(events.iter().any(|e| e.shift_held));
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, DemoStepKind::Backspace))
        );
        let submits = events
            .iter()
            .filter(|e| matches!(e.kind, DemoStepKind::EnterSubmit(_)))
            .count();
        assert_eq!(submits, 2);

        let zh_prompts: Vec<SharedString> = vec!["规划六月多洛米蒂徒步路线".into()];
        let (zh_events, _) = compile_demo_timeline(&zh_prompts);
        let alpha_presses = zh_events
            .iter()
            .filter(|e| {
                e.key_id
                    .is_some_and(|k| k.len() == 1 && k.as_bytes()[0].is_ascii_lowercase())
            })
            .count();
        assert!(
            alpha_presses >= 20,
            "expected CJK prompt to type Pinyin letter keys, got {alpha_presses}"
        );
    }

    #[test]
    fn keystroke_wave_pulse_lifts_luminance_above_resting_gradient_floor() {
        let n = GW * GH;
        let mut cb = vec![[0.0; 4]; n];
        let mut ct = vec![[0.0; 4]; n];
        let mut lb = vec![[0.0; 1]; n];
        let mut lt = vec![[0.0; 1]; n];

        let rest = rasterize_glow_floor(
            true,
            [0x11, 0x11, 0x13],
            0.20,
            &[],
            &mut cb,
            &mut ct,
            &mut lb,
            &mut lt,
        );
        let pulsed = rasterize_glow_floor(
            true,
            [0x11, 0x11, 0x13],
            0.20,
            &[(1100.0, 0.35)],
            &mut cb,
            &mut ct,
            &mut lb,
            &mut lt,
        );

        let b_rest = rest.as_bytes(0).expect("rest BGRA");
        let b_pulse = pulsed.as_bytes(0).expect("pulse BGRA");
        let mid_plume = ((TEX_H * 58 / 100) * TEX_W + TEX_W / 2) * 4;
        let lum_rest = u16::from(b_rest[mid_plume])
            + u16::from(b_rest[mid_plume + 1])
            + u16::from(b_rest[mid_plume + 2]);
        let lum_pulse = u16::from(b_pulse[mid_plume])
            + u16::from(b_pulse[mid_plume + 1])
            + u16::from(b_pulse[mid_plume + 2]);
        assert!(
            lum_pulse > lum_rest + 35,
            "expected keystroke wave pulse to brighten floor plume ({lum_pulse} vs {lum_rest})"
        );
    }

    #[test]
    fn grapheme_backspace_deletes_ascii_cjk_combining_and_zwj_clusters() {
        let mut s = String::from("Hi 山峰 e\u{0301} 👨‍👩‍👧");
        pop_last_grapheme(&mut s);
        assert_eq!(s, "Hi 山峰 e\u{0301} ");
        pop_last_grapheme(&mut s);
        pop_last_grapheme(&mut s);
        assert_eq!(s, "Hi 山峰 ");
        pop_last_grapheme(&mut s);
        pop_last_grapheme(&mut s);
        assert_eq!(s, "Hi 山");
    }

    #[gpui::test]
    fn interactive_typing_glides_2d_backlight_submits_and_auto_resumes_demo(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let exec = cx.executor();
        let events: Rc<RefCell<Vec<GlowKeyboardEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let ev_sink = events.clone();

        let (kb, window) = cx.add_window_view(|_, cx| {
            let view = GlowKeyboard::new("gk-interactive", cx);
            cx.subscribe_self(move |_, ev: &GlowKeyboardEvent, _| {
                ev_sink.borrow_mut().push(ev.clone());
            })
            .detach();
            view
        });
        window.simulate_resize(size(px(880.), px(480.)));
        window.run_until_parked();

        // Focus keyboard and type "Q!" -> 'q' is on upper-left of board, so 2D glow glides left & up.
        let rest_gx = window.update(|_, app| kb.read(app).glow_x.value(app));
        window.update(|w, app| {
            kb.update(app, |this, cx| {
                this.focus.focus(w, cx);
                this.handle_manual_key("q", Some("q"), true, true, cx);
                this.handle_manual_key("1", None, true, false, cx);
            });
        });
        exec.advance_clock(Duration::from_millis(80));
        window.update(|w, app| w.simulate_next_frame(app));
        window.run_until_parked();

        let (typed_val, moved_gx) = window.update(|_, app| {
            (
                kb.read(app).value().to_owned(),
                kb.read(app).glow_x.value(app),
            )
        });
        assert_eq!(typed_val, "Q!");
        assert!(
            moved_gx < rest_gx - 50.0,
            "expected 2D glow_x to glide toward left 'q'/'1' keys"
        );

        // Backspace deletes '!', Enter submits "Q" and clears buffer.
        window.update(|_, app| {
            kb.update(app, |this, cx| {
                this.handle_manual_key("backspace", None, false, true, cx);
                this.handle_manual_key("enter", None, false, true, cx);
            });
        });
        assert_eq!(window.update(|_, app| kb.read(app).value().to_owned()), "");
        assert!(
            events
                .borrow()
                .iter()
                .any(|e| *e == GlowKeyboardEvent::Submit("Q".into()))
        );

        // After 4.5s of idle, manual_text clears to None and auto-demo resumes!
        exec.advance_clock(Duration::from_millis(IDLE_RESUME_MS + 100));
        window.run_until_parked();
        assert!(window.update(|_, app| kb.read(app).manual_text.is_none()));
    }
}
