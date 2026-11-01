//! Stream public progress notes and disclose their completed transcript.
use crate::primitives::{
    clipboard,
    family::{Paint, PhaseClock, Slide, disabled_motion},
    glyph::{Glyph, icon, transformed},
    ops_surface::{Fold, column, row, swap_label, tr},
    presentation::RollingNumber,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteState {
    Live,
    Finished,
    Stopped,
    Interrupted,
}
pub struct StopRequested;
pub struct RetryRequested;

pub struct ExplanationNote {
    id: SharedString,
    pub notes: Vec<(String, String)>,
    pub elapsed: u32,
    pub state: NoteState,
    pub error: Option<SharedString>,
    expanded: bool,
    plain: bool,
    surrounding_color: Option<Hsla>,
    preview_height: Rc<Cell<Pixels>>,
    preview_y: Slide,
    chevron: Slide,
    seconds: RollingNumber,
    shimmer: PhaseClock,
    ink: Vec<ParagraphInk>,
    visible: bool,
    copying: bool,
    copied: Option<bool>,
    generation: u64,
    retry_pending: bool,
}
impl EventEmitter<StopRequested> for ExplanationNote {}
impl EventEmitter<RetryRequested> for ExplanationNote {}

impl ExplanationNote {
    pub fn new(id: impl Into<SharedString>, notes: Vec<(String, String)>) -> Self {
        Self {
            id: id.into(),
            notes,
            elapsed: 0,
            state: NoteState::Live,
            error: None,
            expanded: false,
            plain: false,
            surrounding_color: None,
            preview_height: Rc::new(Cell::new(px(0.))),
            preview_y: Slide::new(0.),
            chevron: Slide::new(0.),
            seconds: RollingNumber::new(0.),
            shimmer: Default::default(),
            ink: vec![],
            visible: true,
            copying: false,
            copied: None,
            generation: 0,
            retry_pending: false,
        }
    }
    /// Replace a cumulative streaming snapshot, preserving complete headings and paragraphs.
    pub fn set_transcript(&mut self, text: &str, cx: &mut Context<Self>) {
        self.notes = parse_transcript(text);
        cx.notify();
    }

    /// Set inside an existing message surface.
    pub fn plain(mut self) -> Self {
        self.plain = true;
        self
    }
    /// Embed on a custom message background, keeping the live text fade color-matched.
    pub fn plain_on(mut self, background: Hsla) -> Self {
        self.plain = true;
        self.surrounding_color = Some(background);
        self
    }
    /// Suspend decorative frames when the host hides this conversation.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }
    pub fn set_expanded(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.expanded != open {
            for paragraph in &mut self.ink {
                paragraph.chunks.clear();
            }
        }
        self.expanded = open;
        self.chevron.duration(f32::from(open), 0.26, cx);
        cx.notify()
    }
    /// Reveal the full transcript on completion. Repeated completion updates preserve the reader's choice.
    pub fn set_state(&mut self, state: NoteState, cx: &mut Context<Self>) {
        if state == NoteState::Finished && self.state == state {
            cx.notify();
            return;
        }
        if self.state == NoteState::Live && state != NoteState::Live && self.elapsed == 0 {
            self.elapsed = self.live_seconds(cx);
        }
        self.state = state;
        self.generation = self.generation.wrapping_add(1);
        self.copying = false;
        self.copied = None;
        self.retry_pending = false;
        if state != NoteState::Interrupted {
            self.error = None;
        }
        if state == NoteState::Live {
            self.ink.clear();
            self.preview_height.set(px(0.));
            self.preview_y = Slide::new(0.);
            self.shimmer.restart(cx);
            self.seconds = RollingNumber::new(self.elapsed as f32);
        }
        self.set_expanded(state == NoteState::Finished, cx);
    }
    fn live_seconds(&self, cx: &App) -> u32 {
        let age = self.shimmer.age(cx);
        if self.elapsed == 0 && age.is_finite() {
            age as u32
        } else {
            self.elapsed
        }
    }
    fn label(&self, cx: &App) -> String {
        let duration = if self.elapsed >= 60 {
            format!("{}m {}s", self.elapsed / 60, self.elapsed % 60)
        } else {
            format!("{}s", self.elapsed)
        };
        match self.state {
            NoteState::Live => tr(cx, "Thinking", "正在思考").into(),
            NoteState::Finished if self.elapsed == 0 => {
                tr(cx, "Thought process", "思考过程").into()
            }
            NoteState::Finished => format!("{} {duration}", tr(cx, "Thought for", "思考用时")),
            NoteState::Stopped => format!("{} {duration}", tr(cx, "Stopped after", "已停止，用时")),
            NoteState::Interrupted => tr(cx, "Thinking interrupted", "思考中断").into(),
        }
    }
    fn copy(&mut self, cx: &mut Context<Self>) {
        if self.copying {
            return;
        }
        self.copying = true;
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        let text = self
            .notes
            .iter()
            .flat_map(|(h, b)| [h.as_str(), b.as_str()])
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        let write = clipboard::write(&text, cx);
        cx.spawn(async move |weak, cx| {
            let ok = write.await;
            let _ = weak.update(cx, |s, cx| {
                if s.generation == generation {
                    s.copying = false;
                    s.copied = Some(ok);
                    cx.notify();
                }
            });
            cx.background_executor()
                .timer(Duration::from_millis(1600))
                .await;
            let _ = weak.update(cx, |s, cx| {
                if s.generation == generation {
                    s.copied = None;
                    cx.notify()
                }
            });
        })
        .detach();
    }
    fn live_preview(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = Paint::get(cx);
        let mut content = String::new();
        let mut highlights = Vec::new();
        for (i, (_, body)) in self.notes.iter().enumerate() {
            if body.is_empty() {
                continue;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            if self.state == NoteState::Live
                && let Some(ink) = self.ink.get(i)
            {
                highlights.extend(ink.highlights(content.len(), p.secondary, w, cx));
            }
            content.push_str(body);
        }
        let height = self.preview_height.clone();
        self.preview_y
            .duration(f32::from(height.get() - px(40.)).max(0.), 0.24, cx);
        let y = self.preview_y.get(w, cx);
        let overflow = height.get() > px(40.);
        let surface = self.surrounding_color.unwrap_or(p.panel);
        let styled = StyledText::new(content).with_highlights(highlights);
        div()
            .id("live-preview")
            .debug_selector(|| "reasoning-live-preview".into())
            .relative()
            .h(px(40.))
            .w_full()
            .overflow_hidden()
            .cursor_pointer()
            .on_click(cx.listener(|s, _, _, cx| s.set_expanded(true, cx)))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(-y))
                    .on_children_prepainted(move |b, w, _| {
                        if let Some(b) = b.first()
                            && b.size.height != height.get()
                        {
                            height.set(b.size.height);
                            w.request_animation_frame();
                        }
                    })
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(styled),
                    ),
            )
            .when(overflow, |view| {
                view.child(
                    div()
                        .id("live-preview-fade")
                        .debug_selector(|| "reasoning-live-fade".into())
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(16.))
                        .bg(linear_gradient(
                            180.,
                            linear_color_stop(surface, 0.),
                            linear_color_stop(surface.opacity(0.), 1.),
                        )),
                )
            })
    }
    fn transcript(&self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = Paint::get(cx);
        let live = self.state == NoteState::Live;
        let mut steps = column().gap(px(14.)).w_full().relative();
        for (i, (heading, body)) in self.notes.iter().enumerate() {
            let mut text = StyledText::new(body.clone());
            if live && let Some(ink) = self.ink.get(i) {
                text = text.with_highlights(ink.highlights(0, p.secondary, w, cx));
            }
            steps = steps.child(
                div()
                    .relative()
                    .w_full()
                    .pl(px(20.))
                    .child(
                        div()
                            .absolute()
                            .left(px(-3.))
                            .top(px(8.))
                            .size(px(7.))
                            .rounded_full()
                            .border_1()
                            .border_color(p.faint)
                            .bg(p.panel),
                    )
                    .when(!heading.is_empty(), |el| {
                        el.child(div().font_weight(FontWeight::MEDIUM).child(heading.clone()))
                    })
                    .child(
                        div()
                            .line_height(px(22.))
                            .text_color(p.secondary)
                            .child(text),
                    ),
            );
        }
        let (g, foot, ink) = match self.state {
            NoteState::Finished => (Glyph::Check, tr(cx, "Done", "已完成").into(), p.success),
            NoteState::Stopped => (
                Glyph::Pause,
                tr(cx, "Stopped", "已停止").into(),
                p.secondary,
            ),
            NoteState::Interrupted => (
                Glyph::Alert,
                self.error.clone().unwrap_or_else(|| {
                    tr(cx, "The stream was interrupted.", "连接中断，请重试。").into()
                }),
                p.danger,
            ),
            NoteState::Live => (
                Glyph::Clock,
                tr(cx, "Thinking", "正在思考").into(),
                p.secondary,
            ),
        };
        let (copy_mark, copy_label) = match self.copied {
            Some(true) => (Glyph::Check, tr(cx, "Copied", "已复制")),
            Some(false) => (Glyph::Alert, tr(cx, "Copy failed, retry", "复制失败，重试")),
            None => (Glyph::Copy, tr(cx, "Copy reasoning", "复制思考过程")),
        };
        column()
            .gap(px(11.))
            .pt(px(1.))
            .pb(px(12.))
            .child(
                div()
                    .ml(px(10.))
                    .border_l_1()
                    .border_color(p.line)
                    .child(steps),
            )
            .child(
                row()
                    .min_h(px(28.))
                    .gap(px(10.))
                    .child(icon(g, 14., ink))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .child(foot),
                    )
                    .child(
                        div()
                            .id("copy-reasoning")
                            .debug_selector(|| "Copy reasoning".into())
                            .aria_label(copy_label)
                            .role(Role::Button)
                            .tab_index(0)
                            .size(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(crate::foundation::surface_radius(cx))
                            .hover(move |s| s.bg(p.hover))
                            .focus_visible(move |s| s.bg(p.hover))
                            .cursor_pointer()
                            .on_click(cx.listener(|s, _, _, cx| s.copy(cx)))
                            .child(icon(copy_mark, 14., p.secondary)),
                    ),
            )
    }
}

/// Parse only the simple heading and paragraph syntax used in progress notes.
pub fn parse_transcript(text: &str) -> Vec<(String, String)> {
    let mut steps = Vec::new();
    let mut heading = String::new();
    let mut body = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let title = line
            .strip_prefix("## ")
            .or_else(|| line.strip_prefix("**").and_then(|s| s.strip_suffix("**")));
        if let Some(title) = title {
            if !heading.is_empty() || !body.is_empty() {
                steps.push((
                    std::mem::take(&mut heading),
                    body.join("\n").trim().to_owned(),
                ));
                body.clear();
            }
            heading = title.trim().to_owned();
        } else if line.is_empty() && !body.is_empty() {
            steps.push((
                std::mem::take(&mut heading),
                body.join("\n").trim().to_owned(),
            ));
            body.clear();
        } else if !line.is_empty()
            && !line.starts_with("**")
            && line != "*"
            && line != "#"
            && line != "##"
        {
            body.push(line);
        }
    }
    if !heading.is_empty() || body.iter().any(|line| !line.is_empty()) {
        steps.push((heading, body.join("\n").trim().to_owned()));
    }
    steps
}

#[derive(Default)]
struct ParagraphInk {
    previous: String,
    chunks: Vec<(std::ops::Range<usize>, PhaseClock)>,
}
impl ParagraphInk {
    fn update(&mut self, text: &str, cx: &App) {
        self.chunks.retain(|(_, clock)| clock.age(cx) < 0.14);
        if text == self.previous {
            return;
        }
        let start = if text.starts_with(&self.previous) {
            self.previous.len()
        } else {
            self.chunks.clear();
            0
        };
        if start < text.len() && !disabled_motion(cx) {
            let mut time = PhaseClock::default();
            time.restart(cx);
            self.chunks.push((start..text.len(), time));
        }
        self.previous.clear();
        self.previous.push_str(text);
    }
    fn highlights(
        &self,
        offset: usize,
        ink: Hsla,
        w: &Window,
        cx: &App,
    ) -> Vec<(std::ops::Range<usize>, HighlightStyle)> {
        self.chunks
            .iter()
            .map(|(r, clock)| {
                let opacity = clock.time(0.14, w, cx) / 0.14;
                (
                    offset + r.start..offset + r.end,
                    HighlightStyle {
                        color: Some(ink.opacity(opacity)),
                        ..Default::default()
                    },
                )
            })
            .collect()
    }
}

/// Use the same transparent artwork as the site header, cached across streaming frames.
fn kense_mark(cx: &App) -> impl IntoElement {
    use crate::{
        foundation::{Theme, ThemeMode},
        primitives::kense_mark,
    };
    use std::sync::{Arc, OnceLock};
    static LIGHT: OnceLock<Arc<Image>> = OnceLock::new();
    static DARK: OnceLock<Arc<Image>> = OnceLock::new();
    let (cache, bytes) = match Theme::global(cx).mode {
        ThemeMode::Light => (&LIGHT, kense_mark::LIGHT),
        ThemeMode::Dark => (&DARK, kense_mark::DARK),
    };
    let mark = cache.get_or_init(|| Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec())));
    div()
        .id("reasoning-kense-mark")
        .role(Role::Image)
        .aria_label("Kense")
        .size(px(18.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(img(mark.clone()).size(px(22.)).flex_none())
}
impl Render for ExplanationNote {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let live = self.state == NoteState::Live;
        if live {
            self.ink
                .resize_with(self.notes.len(), ParagraphInk::default);
            for (ink, (_, body)) in self.ink.iter_mut().zip(&self.notes) {
                ink.update(body, cx);
            }
        }
        let label = self.label(cx);
        let has_notes = self
            .notes
            .iter()
            .any(|(h, b)| !h.is_empty() || !b.is_empty());
        let radius = crate::foundation::surface_radius(cx);
        let angle = self.chevron.get(w, cx) * std::f32::consts::PI;
        if live && !self.shimmer.age(cx).is_finite() {
            self.shimmer.restart(cx);
        }
        let seconds = self.live_seconds(cx);
        self.seconds.update(
            (if seconds < 60 { seconds } else { seconds % 60 }) as f32,
            cx,
        );
        let mut heading = row()
            .min_w_0()
            .flex_1()
            .gap(px(10.))
            .child(kense_mark(cx))
            .when(self.state == NoteState::Interrupted, |el| {
                el.child(icon(Glyph::Alert, 14., p.danger))
            });
        let caption = swap_label("thinking-heading", label.clone(), w, cx);
        let caption = if live && self.visible && !disabled_motion(cx) && w.is_visible() {
            let phase = (self.shimmer.age(cx) % 2.0) / 2.;
            w.request_animation_frame();
            let n = label.chars().count().max(1) as f32;
            let ink = label.char_indices().enumerate().map(|(i, (start, ch))| {
                let x = i as f32 / n;
                let strength = (1. - ((x - (phase * 1.8 - 0.4)) / 0.34).abs()).clamp(0., 1.);
                (
                    start..start + ch.len_utf8(),
                    HighlightStyle {
                        color: Some(p.secondary.blend(p.ink.opacity(strength))),
                        ..Default::default()
                    },
                )
            });
            div().child(StyledText::new(label.clone()).with_highlights(ink))
        } else {
            caption
        };
        heading = heading
            .child(div().min_w_0().overflow_hidden().child(caption))
            .when(live, |el| {
                el.child(
                    row()
                        .gap_0()
                        .min_w(px(28.))
                        .text_color(p.faint)
                        .when(seconds >= 60, |el| el.child(format!("{}m ", seconds / 60)))
                        .child(self.seconds.render(12., w, cx))
                        .child(div().text_size(px(12.)).child("s")),
                )
            });
        let action = live || self.state == NoteState::Interrupted;
        let toggle = div()
            .id("reasoning-toggle")
            .debug_selector({
                let l = label.clone();
                move || l.clone()
            })
            .role(Role::Button)
            .aria_label(label.clone())
            .aria_expanded(self.expanded)
            .tab_index(0)
            .h_full()
            .min_w_0()
            .flex_1()
            .flex()
            .items_center()
            .rounded(radius)
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.faint))
            .on_click(cx.listener(|s, _, _, cx| s.set_expanded(!s.expanded, cx)))
            .child(heading);
        let mut header = row()
            .relative()
            .h(px(44.))
            .flex_none()
            .gap(px(8.))
            .child(toggle);
        if action {
            let text = if live {
                tr(cx, "Stop", "停止")
            } else {
                tr(cx, "Retry", "重试")
            };
            header = header.child(
                row()
                    .id("reasoning-action")
                    .debug_selector({
                        let s = text.to_owned();
                        move || s.clone()
                    })
                    .h(px(28.))
                    .px(px(10.))
                    .gap(px(5.))
                    .role(Role::Button)
                    .aria_label(text)
                    .tab_index(0)
                    .rounded(radius)
                    .cursor_pointer()
                    .border_1()
                    .border_color(p.line)
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_color(p.ink))
                    .text_size(px(12.))
                    .when(live, |el| el.child(div().size(px(6.)).bg(p.secondary)))
                    .on_click(cx.listener(|s, _, _, cx| match s.state {
                        NoteState::Live => {
                            s.set_state(NoteState::Stopped, cx);
                            cx.emit(StopRequested)
                        }
                        NoteState::Interrupted if !s.retry_pending => {
                            s.retry_pending = true;
                            cx.emit(RetryRequested);
                            cx.notify()
                        }
                        _ => {}
                    }))
                    .child(text),
            );
        }
        header = header.child(
            div()
                .id("reasoning-chevron")
                .h_full()
                .px(px(2.))
                .flex()
                .items_center()
                .cursor_pointer()
                .on_click(cx.listener(|s, _, _, cx| s.set_expanded(!s.expanded, cx)))
                .child(transformed(
                    Glyph::Down,
                    14.,
                    p.faint,
                    angle,
                    1.,
                    (0., 0.),
                    1.,
                )),
        );
        let mut card = column()
            .id(self.id.clone())
            .role(Role::Group)
            .w_full()
            .max_w(px(522.))
            .gap_0()
            .text_size(px(14.))
            .line_height(px(20.))
            .text_color(p.ink)
            .when(!self.plain, |el| {
                el.rounded(radius)
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .px(px(16.))
            })
            .child(header)
            .child(
                div()
                    .id("reasoning-status")
                    .role(Role::Status)
                    .aria_label(label)
                    .absolute()
                    .size(px(0.))
                    .overflow_hidden(),
            );
        let preview = Fold::get(
            "summary-fold",
            live && !self.expanded && has_notes,
            48.,
            0.26,
            w,
            cx,
        );
        if preview.mounted() {
            // Move the live preview up as it opens, keeping four pixels below the header label.
            let summary = div().pl(px(30.)).pb(px(8.)).child(self.live_preview(w, cx));
            card = card.child(
                div()
                    .w_full()
                    .flex_none()
                    .mt(px(-7.))
                    .child(preview.wrap(-7., summary)),
            );
        }
        let detail = Fold::get("notes-fold", self.expanded, 64., 0.26, w, cx);
        if detail.mounted() {
            card = card.child(detail.wrap(0., self.transcript(w, cx)))
        }
        card
    }
}
