//! Article views with host-provided, cancellable streamed summaries.
use crate::primitives::{
    control::UiLanguage,
    direct_touch::Contact,
    editor::EditorScrollRegion,
    family::{Paint, PhaseClock, Slide, Tone, ease, face},
    glyph::{Glyph, icon},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TldrParagraph {
    pub id: SharedString,
    pub text: SharedString,
}
impl TldrParagraph {
    pub fn new(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TldrArticle {
    pub title: SharedString,
    pub source: SharedString,
    pub paragraphs: Vec<TldrParagraph>,
}
impl TldrArticle {
    pub fn new(title: impl Into<SharedString>, paragraphs: Vec<TldrParagraph>) -> Self {
        Self {
            title: title.into(),
            source: "".into(),
            paragraphs,
        }
    }
    pub fn with_source(mut self, source: impl Into<SharedString>) -> Self {
        self.source = source.into();
        self
    }
}
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub enum TldrMode {
    #[default]
    Full,
    Summary,
    KeyPoints,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TldrStatus {
    #[default]
    Idle,
    Thinking,
    Writing,
    Ready,
    Stopped,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TldrPoint {
    pub text: SharedString,
    pub source: SharedString,
}
impl TldrPoint {
    pub fn new(text: impl Into<SharedString>, source: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            source: source.into(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct TldrRequest {
    pub id: u64,
    pub mode: TldrMode,
    pub length: u8,
    pub target_words: usize,
    pub point_count: usize,
    pub paragraphs: Vec<TldrParagraph>,
}
#[derive(Clone, Debug)]
pub enum TldrEvent {
    Requested(TldrRequest),
    Cancelled(u64),
    Jumped(SharedString),
}
#[derive(Clone, Default)]
struct ResultData {
    text: String,
    points: Vec<TldrPoint>,
}
/// The component renders supplied article content; a host owns all summarization work.
pub struct TldrToggle {
    article: TldrArticle,
    mode: TldrMode,
    length: u8,
    status: TldrStatus,
    request_seq: u64,
    active: Option<u64>,
    result: ResultData,
    cache: HashMap<(TldrMode, u8), ResultData>,
    error: SharedString,
    focus_modes: [FocusHandle; 3],
    focus_length: FocusHandle,
    focus_paragraphs: Vec<FocusHandle>,
    length_bounds: Rc<Cell<Bounds<Pixels>>>,
    touch: Contact,
    scroll: ScrollHandle,
    height: Slide,
    measured: Rc<Cell<Pixels>>,
    crossfade: PhaseClock,
    highlighted: Option<SharedString>,
    highlight_clock: PhaseClock,
}
impl EventEmitter<TldrEvent> for TldrToggle {}
impl TldrToggle {
    pub fn new(article: TldrArticle, cx: &mut Context<Self>) -> Self {
        let focus_paragraphs = article
            .paragraphs
            .iter()
            .map(|_| cx.focus_handle())
            .collect();
        Self {
            article,
            mode: TldrMode::Full,
            length: 1,
            status: TldrStatus::Idle,
            request_seq: 0,
            active: None,
            result: ResultData::default(),
            cache: HashMap::new(),
            error: "".into(),
            focus_modes: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
            focus_length: cx.focus_handle().tab_stop(true),
            focus_paragraphs,
            length_bounds: Default::default(),
            touch: Default::default(),
            scroll: ScrollHandle::new(),
            height: Slide::snappy(1.),
            measured: Default::default(),
            crossfade: Default::default(),
            highlighted: None,
            highlight_clock: Default::default(),
        }
    }
    pub fn mode(&self) -> TldrMode {
        self.mode
    }
    pub fn status(&self) -> TldrStatus {
        self.status
    }
    pub fn length(&self) -> u8 {
        self.length
    }
    pub fn active_request(&self) -> Option<u64> {
        self.active
    }
    fn targets(length: u8) -> (usize, usize) {
        match length.min(2) {
            0 => (35, 3),
            1 => (60, 4),
            _ => (100, 6),
        }
    }
    fn cancel_active(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.active.take() {
            cx.emit(TldrEvent::Cancelled(id));
        }
    }
    fn start(&mut self, cx: &mut Context<Self>) {
        self.cancel_active(cx);
        self.result = ResultData::default();
        self.error = "".into();
        self.status = TldrStatus::Thinking;
        self.request_seq = self.request_seq.wrapping_add(1).max(1);
        let id = self.request_seq;
        self.active = Some(id);
        let (target_words, point_count) = Self::targets(self.length);
        cx.emit(TldrEvent::Requested(TldrRequest {
            id,
            mode: self.mode,
            length: self.length,
            target_words,
            point_count,
            paragraphs: self.article.paragraphs.clone(),
        }));
        cx.notify();
    }
    /// Select a view; a cached result is reused without requesting the host again.
    pub fn select_mode(&mut self, mode: TldrMode, cx: &mut Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.cancel_active(cx);
        self.mode = mode;
        self.crossfade.restart(cx);
        self.highlighted = None;
        if mode == TldrMode::Full {
            self.status = TldrStatus::Idle;
        } else if let Some(result) = self.cache.get(&(mode, self.length)).cloned() {
            self.result = result;
            self.status = TldrStatus::Ready;
        } else {
            self.start(cx);
        }
        cx.notify();
    }
    /// Change between short, medium and long targets; changing it cancels a pending request.
    pub fn set_length(&mut self, length: u8, cx: &mut Context<Self>) {
        let length = length.min(2);
        if self.length == length {
            return;
        }
        self.length = length;
        if self.mode != TldrMode::Full {
            self.cancel_active(cx);
            if let Some(result) = self.cache.get(&(self.mode, length)).cloned() {
                self.result = result;
                self.status = TldrStatus::Ready;
            } else {
                self.start(cx);
            }
        }
        cx.notify();
    }
    /// Apply a streamed text chunk only if it belongs to the active request.
    pub fn append_text(&mut self, request: u64, chunk: &str, cx: &mut Context<Self>) -> bool {
        if self.active != Some(request) || self.mode != TldrMode::Summary {
            return false;
        }
        self.result.text.push_str(chunk);
        self.status = TldrStatus::Writing;
        cx.notify();
        true
    }
    /// Append a point with the stable source paragraph ID returned by the host.
    pub fn append_point(&mut self, request: u64, point: TldrPoint, cx: &mut Context<Self>) -> bool {
        if self.active != Some(request) || self.mode != TldrMode::KeyPoints {
            return false;
        }
        self.result.points.push(point);
        self.status = TldrStatus::Writing;
        cx.notify();
        true
    }
    pub fn finish(&mut self, request: u64, cx: &mut Context<Self>) -> bool {
        if self.active != Some(request) {
            return false;
        }
        self.active = None;
        self.status = TldrStatus::Ready;
        self.cache
            .insert((self.mode, self.length), self.result.clone());
        cx.notify();
        true
    }
    pub fn fail(
        &mut self,
        request: u64,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active != Some(request) {
            return false;
        }
        self.active = None;
        self.error = message.into();
        self.status = TldrStatus::Failed;
        cx.notify();
        true
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if self.active.is_some() {
            self.cancel_active(cx);
            self.status = TldrStatus::Stopped;
            cx.notify();
        }
    }
    pub fn retry(&mut self, cx: &mut Context<Self>) {
        if self.mode != TldrMode::Full {
            self.start(cx);
        }
    }
    /// Source jumps select the original article and move focus to the named paragraph.
    pub fn jump_to(&mut self, source: &str, w: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(index) = self
            .article
            .paragraphs
            .iter()
            .position(|p| p.id.as_ref() == source)
        else {
            return false;
        };
        self.cancel_active(cx);
        self.mode = TldrMode::Full;
        self.status = TldrStatus::Idle;
        self.highlighted = Some(self.article.paragraphs[index].id.clone());
        let expected = self.article.paragraphs[index].id.clone();
        self.highlight_clock.restart(cx);
        cx.defer_in(w, move |s, w, cx| {
            if s.mode != TldrMode::Full || s.highlighted.as_ref() != Some(&expected) {
                return;
            }
            s.focus_paragraphs[index].focus(w, cx);
            s.scroll.scroll_to_item(index);
            cx.notify();
        });
        cx.emit(TldrEvent::Jumped(source.to_owned().into()));
        cx.notify();
        true
    }
    fn set_length_from_x(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if self.mode == TldrMode::Full {
            return;
        }
        let b = self.length_bounds.get();
        if b.size.width <= px(0.) {
            return;
        }
        let t = ((x - b.left()) / b.size.width).clamp(0., 1.);
        self.set_length((t * 2.).round() as u8, cx);
    }
}
impl Render for TldrToggle {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let radius = crate::foundation::surface_radius(cx);
        let t = |en: &'static str, zh: &'static str| l.text(en, zh);
        let modes = [
            (TldrMode::Full, t("Full", "全文")),
            (TldrMode::Summary, t("Summary", "摘要")),
            (TldrMode::KeyPoints, t("Key points", "要点")),
        ];
        let mut controls = div()
            .id("tldr-modes")
            .flex()
            .items_center()
            .gap(px(2.))
            .p(px(3.))
            .rounded(radius)
            .bg(p.tray)
            .role(Role::Group)
            .aria_label(t("Reading mode", "阅读模式"));
        for (i, (mode, title)) in modes.into_iter().enumerate() {
            let selected = self.mode == mode;
            controls = controls.child(
                face(("tldr-mode", i), title, 32., radius.into(), Tone::Clear, cx)
                    .px(px(12.))
                    .track_focus(&self.focus_modes[i])
                    .aria_toggled(selected.into())
                    .text_color(if selected { p.ink } else { p.secondary })
                    .when(selected, |v| v.bg(p.panel).border_1().border_color(p.line))
                    .child(title)
                    .on_click(cx.listener(move |s, _, _, cx| s.select_mode(mode, cx)))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                        let next = match e.keystroke.key.as_str() {
                            "left" => (i + 2) % 3,
                            "right" => (i + 1) % 3,
                            "home" => 0,
                            "end" => 2,
                            _ => return,
                        };
                        let target = [TldrMode::Full, TldrMode::Summary, TldrMode::KeyPoints][next];
                        s.select_mode(target, cx);
                        s.focus_modes[next].focus(w, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    })),
            );
        }
        let bounds = self.length_bounds.clone();
        let (words, points) = Self::targets(self.length);
        let spoken = match self.mode {
            TldrMode::KeyPoints => format!("{points} {}", t("points", "个要点")),
            _ => format!("~{words} {}", t("words", "字")),
        };
        let disabled = self.mode == TldrMode::Full;
        let mut length = div()
            .id("tldr-length")
            .relative()
            .w(px(120.))
            .h(px(32.))
            .flex()
            .items_center()
            .role(Role::Slider)
            .aria_label(t("Summary length", "摘要长度"))
            .aria_value(spoken.clone())
            .track_focus(&self.focus_length)
            .tab_index(if disabled { -1 } else { 0 })
            .opacity(if disabled { 0.45 } else { 1. })
            .child(
                canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                    .absolute()
                    .inset_0(),
            )
            .child(
                div()
                    .w_full()
                    .h(px(4.))
                    .rounded_full()
                    .bg(p.hover)
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w(relative(self.length as f32 / 2.))
                            .bg(p.bright),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(self.length as f32 / 2.))
                    .ml(px(-7.))
                    .size(px(14.))
                    .rounded_full()
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.faint)
                    .shadow_sm(),
            );
        if !disabled {
            length = length
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseDownEvent, w, cx| {
                        s.focus_length.focus(w, cx);
                        s.set_length_from_x(e.position.x, cx);
                    }),
                )
                .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                    if e.pressed_button == Some(MouseButton::Left) {
                        s.set_length_from_x(e.position.x, cx);
                    }
                }))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    let v = match e.keystroke.key.as_str() {
                        "left" | "down" => s.length.saturating_sub(1),
                        "right" | "up" => (s.length + 1).min(2),
                        "home" => 0,
                        "end" => 2,
                        _ => return,
                    };
                    s.set_length(v, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }))
                .child(self.touch.listen(cx, |s, e, _, cx| {
                    if matches!(e.phase, TouchPhase::Started | TouchPhase::Moved) {
                        s.set_length_from_x(e.position.x, cx);
                    }
                }));
        }
        let mut body = div()
            .flex()
            .flex_col()
            .gap(px(match self.mode {
                TldrMode::Full => 4.,
                TldrMode::Summary => 12.,
                TldrMode::KeyPoints => 0.,
            }))
            .w_full();
        match self.mode {
            TldrMode::Full => {
                for (i, paragraph) in self.article.paragraphs.iter().enumerate() {
                    let marked = self.highlighted.as_ref() == Some(&paragraph.id);
                    let age = self.highlight_clock.time(0.76, w, cx);
                    let alpha = if marked {
                        if age < 0.16 {
                            ease(age / 0.16)
                        } else {
                            1. - ease((age - 0.16) / 0.60)
                        }
                    } else {
                        0.
                    };
                    body = body.child(
                        div()
                            .id(("tldr-paragraph", i))
                            .flex_none()
                            .track_focus(&self.focus_paragraphs[i])
                            .tab_index(-1)
                            .mx(px(-10.))
                            .py(px(4.))
                            .px(px(10.))
                            .rounded(px(8.))
                            .bg(p.focus.opacity(alpha * 0.20))
                            .text_color(p.ink)
                            .text_size(px(14.))
                            .line_height(px(22.))
                            .child(paragraph.text.clone()),
                    );
                }
            }
            TldrMode::Summary => {
                if !self.result.text.is_empty() {
                    body = body.child(
                        div()
                            .flex_none()
                            .text_size(px(14.))
                            .line_height(px(22.))
                            .text_color(p.ink)
                            .child(self.result.text.clone()),
                    );
                }
            }
            TldrMode::KeyPoints => {
                for (i, point) in self.result.points.iter().enumerate() {
                    let source = point.source.clone();
                    let source_key = source.clone();
                    let valid = self.article.paragraphs.iter().any(|p| p.id == source);
                    let label = format!("{}: {}", t("Jump to source", "跳转到原文"), point.text);
                    let debug_label = label.clone();
                    body = body.child(
                        div()
                            .id(("tldr-point", i))
                            .debug_selector(move || debug_label.clone())
                            .flex_none()
                            .py(px(if i == 0 { 0. } else { 10. }))
                            .pb(px(if i + 1 == self.result.points.len() {
                                0.
                            } else {
                                10.
                            }))
                            .flex()
                            .items_start()
                            .gap(px(12.))
                            .text_size(px(14.))
                            .line_height(px(22.))
                            .text_color(p.ink)
                            .when(valid, |v| {
                                v.cursor_pointer()
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .aria_label(label)
                                    .on_click(cx.listener(move |s, _, w, cx| {
                                        s.jump_to(source.as_ref(), w, cx);
                                    }))
                                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                        if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                                            s.jump_to(source_key.as_ref(), w, cx);
                                            w.prevent_default();
                                            cx.stop_propagation();
                                        }
                                    }))
                            })
                            .child(
                                div()
                                    .mt(px(3.))
                                    .size(px(16.))
                                    .flex_none()
                                    .child(icon(Glyph::Check, 16., p.secondary)),
                            )
                            .child(div().flex_1().min_w_0().child(point.text.clone()))
                            .when(valid, |v| {
                                v.child(
                                    div()
                                        .flex_none()
                                        .h(px(28.))
                                        .min_w(px(36.))
                                        .mt(px(-3.))
                                        .mr(px(-8.))
                                        .mb(px(-3.))
                                        .px(px(8.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(crate::foundation::surface_radius(cx))
                                        .text_size(px(12.))
                                        .text_color(p.secondary)
                                        .hover(move |s| s.bg(p.tray).text_color(p.ink))
                                        .child(format!("§{}", i + 1)),
                                )
                            }),
                    );
                }
            }
        }
        if self.mode != TldrMode::Full && self.status == TldrStatus::Thinking {
            body = body.child(
                div()
                    .flex_none()
                    .text_size(px(14.))
                    .text_color(p.secondary)
                    .child(t("Thinking…", "正在思考…")),
            );
        }
        if self.status == TldrStatus::Failed {
            body = body.child(
                div()
                    .id("tldr-error")
                    .flex_none()
                    .role(Role::Alert)
                    .text_color(p.danger)
                    .text_size(px(13.))
                    .child(self.error.clone()),
            );
        }
        let body_count = match self.mode {
            TldrMode::Full => self.article.paragraphs.len(),
            TldrMode::Summary => usize::from(!self.result.text.is_empty()),
            TldrMode::KeyPoints => self.result.points.len(),
        } + usize::from(
            self.mode != TldrMode::Full && self.status == TldrStatus::Thinking,
        ) + usize::from(self.status == TldrStatus::Failed);
        let measured = self.measured.clone();
        let notify = cx.weak_entity();
        let desired = f32::from(measured.get()).clamp(1., 300.);
        self.height.duration(desired, 0.26, cx);
        let height = self.height.get(w, cx).max(1.);
        let enter = ease(self.crossfade.time(0.16, w, cx) / 0.16);
        let scroll = self.scroll.clone();
        let measure_scroll = self.scroll.clone();
        let monitor = canvas(
            move |_, _, cx| {
                // A tracked scroll element does not supply its child bounds to
                // on_children_prepainted. Read its current direct children from
                // ScrollHandle after that sibling has completed prepaint instead.
                let natural_height = body_count
                    .checked_sub(1)
                    .and_then(|last| {
                        Some((
                            measure_scroll.bounds_for_item(0)?,
                            measure_scroll.bounds_for_item(last)?,
                        ))
                    })
                    .map_or(px(1.), |(first, last)| {
                        (last.bottom() - first.top()).max(px(1.))
                    });
                if measured.replace(natural_height) != natural_height {
                    let _ = notify.update(cx, |_, cx| cx.notify());
                }
            },
            move |bounds, _, _, cx| {
                if let Some(host) = cx.try_global::<EditorScrollRegion>() {
                    let top = -scroll.offset().y;
                    (host.0)(bounds, top > px(0.1), top + px(0.1) < scroll.max_offset().y);
                }
            },
        )
        .absolute()
        .inset_0();
        let content = div()
            .relative()
            .w_full()
            .h(px(height))
            .child(
                body.opacity(enter)
                    .id("tldr-scroll")
                    .w_full()
                    .h_full()
                    .max_h(px(300.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll),
            )
            .child(monitor);
        let status = match self.status {
            TldrStatus::Idle => "",
            TldrStatus::Thinking => t("Thinking", "思考中"),
            TldrStatus::Writing => t("Writing", "生成中"),
            TldrStatus::Ready => t("Ready", "已完成"),
            TldrStatus::Stopped => t("Stopped", "已停止"),
            TldrStatus::Failed => t("Failed", "失败"),
        };
        let mut footer = div()
            .flex()
            .items_center()
            .justify_between()
            .min_h(px(32.))
            .mt(px(16.))
            .gap(px(12.))
            .text_size(px(12.))
            .line_height(px(16.))
            .text_color(p.secondary)
            .child(status);
        if self.active.is_some() {
            footer = footer.child(
                face(
                    "tldr-stop",
                    t("Stop", "停止"),
                    32.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(12.))
                .child(t("Stop", "停止"))
                .on_click(cx.listener(|s, _, _, cx| s.stop(cx))),
            );
        }
        if matches!(self.status, TldrStatus::Stopped | TldrStatus::Failed) {
            footer = footer.child(
                face(
                    "tldr-retry",
                    t("Try again", "重试"),
                    32.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(12.))
                .child(t("Try again", "重试"))
                .on_click(cx.listener(|s, _, _, cx| s.retry(cx))),
            );
        }
        div()
            .id("tldr-article")
            .role(Role::Article)
            .aria_label(self.article.title.clone())
            .w_full()
            .max_w(px(680.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .pt(px(24.))
            .px(px(24.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(18.))
                    .line_height(px(24.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(self.article.title.clone()),
            )
            .when(!self.article.source.is_empty(), |v| {
                v.child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.secondary)
                        .child(self.article.source.clone()),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(24.))
                    .mt(px(12.))
                    .child(controls)
                    .child(
                        div().flex().items_center().gap(px(8.)).child(length).child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.secondary)
                                .child(spoken),
                        ),
                    ),
            )
            .child(content)
            .child(footer)
    }
}

#[cfg(test)]
mod tests {
    use super::{TldrArticle, TldrMode, TldrParagraph, TldrPoint, TldrStatus, TldrToggle};
    use gpui::{Modifiers, TestAppContext, VisualTestContext, px};
    use std::time::Duration;

    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }

    #[test]
    fn source_jump_scrolls_and_focuses_the_actual_later_paragraph() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let paragraphs = (0..12).map(|i| TldrParagraph::new(
            format!("paragraph-{i}"),
            format!("Paragraph {i}. A longer source paragraph remains readable and takes space in the article."),
        )).collect();
        let (control, mut w) = app
            .add_window_view(|_, cx| TldrToggle::new(TldrArticle::new("Article", paragraphs), cx));
        tick(&mut w, 400);
        w.update(|_, cx| {
            control.update(cx, |s, cx| {
                s.select_mode(TldrMode::KeyPoints, cx);
                let request = s.active_request().unwrap();
                assert!(s.append_point(request, TldrPoint::new("Evidence", "paragraph-10"), cx));
                assert!(s.finish(request, cx));
            })
        });
        tick(&mut w, 400);
        let link = w
            .debug_bounds("Jump to source: Evidence")
            .expect("source link")
            .center();
        w.simulate_click(link, Modifiers::default());
        tick(&mut w, 16);
        tick(&mut w, 400);
        tick(&mut w, 400);
        w.update(|w, cx| {
            let s = control.read(cx);
            assert_eq!(s.mode(), TldrMode::Full);
            assert!(
                s.focus_paragraphs[10].is_focused(w),
                "focus should follow the source"
            );
            assert!(
                s.scroll.offset().y < -px(1.),
                "later paragraph should scroll into view"
            );
            let viewport = s.scroll.bounds();
            let target = s
                .scroll
                .bounds_for_item(10)
                .expect("each paragraph is a direct scroll child");
            let top = target.top() + s.scroll.offset().y;
            let bottom = target.bottom() + s.scroll.offset().y;
            assert!(
                top >= viewport.top() - px(1.) && bottom <= viewport.bottom() + px(1.),
                "source should be visible in the scroll viewport"
            );
        });
    }
    #[test]
    fn cancellation_stale_results_cache_and_source_jump() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let article = TldrArticle::new(
            "Article",
            vec![
                TldrParagraph::new("p1", "First paragraph."),
                TldrParagraph::new("p2", "Second paragraph."),
            ],
        );
        let (control, w) = app.add_window_view(|_, cx| TldrToggle::new(article, cx));
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                s.select_mode(TldrMode::Summary, cx);
                let first = s.active_request().unwrap();
                assert!(s.append_text(first, "First chunk", cx));
                s.stop(cx);
                assert_eq!(s.status(), TldrStatus::Stopped);
                assert!(!s.append_text(first, "late", cx));
                s.retry(cx);
                let second = s.active_request().unwrap();
                assert_ne!(first, second);
                assert!(s.append_text(second, "Ready", cx));
                assert!(s.finish(second, cx));
                s.select_mode(TldrMode::Full, cx);
                s.select_mode(TldrMode::Summary, cx);
                assert_eq!(s.status(), TldrStatus::Ready);
                assert_eq!(s.active_request(), None);
                s.set_length(2, cx);
                let third = s.active_request().unwrap();
                s.select_mode(TldrMode::KeyPoints, cx);
                let fourth = s.active_request().unwrap();
                assert!(!s.finish(third, cx));
                assert!(s.append_point(fourth, TldrPoint::new("Evidence", "p2"), cx));
                assert!(s.finish(fourth, cx));
                assert!(!s.jump_to("missing", w, cx));
                assert!(s.jump_to("p2", w, cx));
                assert_eq!(s.mode(), TldrMode::Full);
            })
        });
    }
}
