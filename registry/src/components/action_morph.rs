//! A floating create button that becomes a menu or an inline composer.
use crate::{
    foundation::{Theme, ThemeMode, surface_radius},
    primitives::{
        control::UiDirection,
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        family::{Paint, PhaseClock, Slide, disabled_motion, ease, measure, press},
        glyph::{Glyph, icon, spinner, transformed},
        presentation::{column, row, tr},
    },
};
use gpui::{prelude::*, *};
use std::{f32::consts::FRAC_PI_2, time::Duration};

const REST: f32 = 56.;
const PAD: f32 = 6.;
const FACE_SECONDS: f32 = 0.22;

/// The anchor side in the current text direction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ActionMorphCorner {
    Start,
    #[default]
    End,
}

/// A choice in a composer's optional single-selection group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionMorphChoice {
    pub value: SharedString,
    pub label: SharedString,
}

impl ActionMorphChoice {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// Labels, input mode and choices for one composer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionMorphComposer {
    pub title: SharedString,
    pub placeholder: SharedString,
    pub submit_label: SharedString,
    pub success_label: SharedString,
    pub empty_hint: Option<SharedString>,
    pub multiline: bool,
    pub choices: Vec<ActionMorphChoice>,
    pub choices_label: SharedString,
    pub default_choice: Option<SharedString>,
}

impl ActionMorphComposer {
    pub fn new(
        title: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
        submit_label: impl Into<SharedString>,
        success_label: impl Into<SharedString>,
    ) -> Self {
        Self {
            title: title.into(),
            placeholder: placeholder.into(),
            submit_label: submit_label.into(),
            success_label: success_label.into(),
            empty_hint: None,
            multiline: false,
            choices: vec![],
            choices_label: "".into(),
            default_choice: None,
        }
    }

    pub fn with_multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    pub fn with_choices(
        mut self,
        label: impl Into<SharedString>,
        choices: Vec<ActionMorphChoice>,
    ) -> Self {
        self.choices_label = label.into();
        self.choices = choices;
        self
    }

    pub fn with_default_choice(mut self, value: impl Into<SharedString>) -> Self {
        self.default_choice = Some(value.into());
        self
    }

    pub fn with_empty_hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.empty_hint = Some(hint.into());
        self
    }

    fn initial_choice(&self) -> usize {
        self.choices
            .iter()
            .position(|choice| self.default_choice.as_ref() == Some(&choice.value))
            .unwrap_or(0)
    }
}

/// An action with a stable identifier and an optional menu shortcut.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionMorphAction {
    pub id: SharedString,
    pub label: SharedString,
    pub description: Option<SharedString>,
    pub icon: Glyph,
    pub shortcut: Option<char>,
    pub composer: ActionMorphComposer,
}

impl ActionMorphAction {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: Glyph,
        composer: ActionMorphComposer,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            composer,
            description: None,
            shortcut: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_shortcut(mut self, key: char) -> Self {
        self.shortcut = Some(key.to_ascii_lowercase());
        self
    }
}

/// A validated submission. Echo its ticket to `ActionMorph::complete` for deferred saves.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionMorphSubmit {
    pub ticket: u64,
    pub action: SharedString,
    pub text: SharedString,
    pub choice: Option<SharedString>,
}

/// A recoverable save failure displayed beside the preserved draft.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionMorphFailure {
    pub message: SharedString,
}

impl ActionMorphFailure {
    pub fn new(message: impl Into<SharedString>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Face {
    Rest,
    Menu,
    Compose(usize),
}

impl Face {
    fn depth(self) -> u8 {
        match self {
            Self::Rest => 0,
            Self::Menu => 1,
            Self::Compose(_) => 2,
        }
    }
}

#[derive(Clone)]
struct Departure {
    face: Face,
    draft: Option<SharedString>,
    problem: SharedString,
}

struct ActionSlot {
    config: ActionMorphAction,
    editor: Entity<Editor>,
    menu_focus: FocusHandle,
    choice_focus: Vec<FocusHandle>,
    choice: usize,
    _listeners: [Subscription; 2],
}

#[derive(Clone, Copy)]
struct Spring {
    start: f32,
    target: f32,
    velocity: f32,
    opens: bool,
    clock: PhaseClock,
}

impl Spring {
    fn rest(value: f32) -> Self {
        Self {
            start: value,
            target: value,
            velocity: 0.,
            opens: false,
            clock: PhaseClock::default(),
        }
    }

    fn sample(self, cx: &App) -> (f32, f32, bool) {
        let t = self.clock.age(cx);
        if disabled_motion(cx) || t >= 0.68 {
            return (self.target, 0., false);
        }
        let delta = self.start - self.target;
        let (x, speed) = if self.opens {
            // A damped spring with a slight overshoot on growth.
            let decay = 16.;
            let frequency = 10.4;
            let coefficient = (self.velocity + decay * delta) / frequency;
            let (sin, cos) = (frequency * t).sin_cos();
            let envelope = (-decay * t).exp();
            let wave = delta * cos + coefficient * sin;
            (
                wave * envelope,
                (frequency * (coefficient * cos - delta * sin) - decay * wave) * envelope,
            )
        } else {
            let decay = 17.6;
            let coefficient = self.velocity + decay * delta;
            let wave = delta + coefficient * t;
            let envelope = (-decay * t).exp();
            (wave * envelope, (coefficient - decay * wave) * envelope)
        };
        if x.abs() < 0.045 && speed.abs() < 0.8 || !self.opens && x * delta < 0. {
            (self.target, 0., false)
        } else {
            (self.target + x, speed, true)
        }
    }

    fn set(&mut self, target: f32, growing: bool, cx: &App) {
        if self.target != target {
            let (start, velocity, _) = self.sample(cx);
            *self = Self {
                start,
                target,
                velocity,
                opens: growing,
                clock: PhaseClock::default(),
            };
            self.clock.restart(cx);
        }
    }
}

/// One bottom-corner surface with separately preserved drafts and a ticketed save lifecycle.
pub struct ActionMorph {
    id: ElementId,
    actions: Vec<ActionSlot>,
    label: Option<SharedString>,
    corner: ActionMorphCorner,
    style: Option<Box<dyn Fn(Div) -> Div>>,
    max_width: Option<Pixels>,
    reset_after: Duration,
    deferred: bool,
    face: Face,
    departure: Option<Departure>,
    face_clock: PhaseClock,
    direction: f32,
    springs: [Spring; 3],
    highlight: Slide,
    pressed: Slide,
    active: usize,
    hovered: Option<usize>,
    trigger: FocusHandle,
    menu_close: FocusHandle,
    back: FocusHandle,
    close: FocusHandle,
    submit: FocusHandle,
    problem: SharedString,
    pending: Option<(u64, usize)>,
    sequence: u64,
    success: Option<SharedString>,
    announcement: SharedString,
    status_clock: PhaseClock,
    save_clock: PhaseClock,
    error_clock: PhaseClock,
    reset: Option<Task<()>>,
}

impl EventEmitter<ActionMorphSubmit> for ActionMorph {}

impl ActionMorph {
    pub fn new(actions: Vec<ActionMorphAction>, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let actions = actions
            .into_iter()
            .enumerate()
            .map(|(i, config)| {
                let composer = &config.composer;
                let editor = cx.new(|cx| {
                    let mode = if composer.multiline {
                        InputMode::Multi
                    } else {
                        InputMode::Single
                    };
                    let mut field = Editor::new(composer.title.clone(), mode, w, cx);
                    field.font_size = 16.;
                    field.set_placeholder(composer.placeholder.clone());
                    field.set_command_submit(composer.multiline);
                    field
                });
                let edit =
                    cx.subscribe_in(&editor, w, move |s: &mut Self, _, e: &EditEvent, w, cx| {
                        if s.face != Face::Compose(i) || s.pending.is_some() {
                            return;
                        }
                        match e {
                            EditEvent::Changed if !s.problem.is_empty() => {
                                s.set_problem("".into(), cx)
                            }
                            EditEvent::Submitted => s.submit(w, cx),
                            EditEvent::Escape => s.open_menu(i, w, cx),
                            _ => {}
                        }
                    });
                let shortcut = cx.subscribe_in(
                    &editor,
                    w,
                    move |s: &mut Self, _, _: &EditorShortcutEvent, w, cx| {
                        if s.face == Face::Compose(i) {
                            s.submit(w, cx);
                        }
                    },
                );
                let choice_focus = composer
                    .choices
                    .iter()
                    .map(|_| cx.focus_handle().tab_stop(true))
                    .collect();
                ActionSlot {
                    choice: composer.initial_choice(),
                    config,
                    editor,
                    menu_focus: cx.focus_handle().tab_stop(true),
                    choice_focus,
                    _listeners: [edit, shortcut],
                }
            })
            .collect();
        Self {
            id: format!("action-morph-{}", cx.entity_id()).into(),
            actions,
            label: None,
            corner: ActionMorphCorner::End,
            style: None,
            max_width: None,
            reset_after: Duration::from_millis(1600),
            deferred: false,
            face: Face::Rest,
            departure: None,
            face_clock: PhaseClock::default(),
            direction: 1.,
            springs: [
                Spring::rest(REST),
                Spring::rest(REST),
                Spring::rest(REST / 2.),
            ],
            highlight: Slide::snappy(0.),
            pressed: Slide::snappy(0.),
            active: 0,
            hovered: None,
            trigger: cx.focus_handle().tab_stop(true),
            menu_close: cx.focus_handle().tab_stop(true),
            back: cx.focus_handle().tab_stop(true),
            close: cx.focus_handle().tab_stop(true),
            submit: cx.focus_handle().tab_stop(true),
            problem: "".into(),
            pending: None,
            sequence: 0,
            success: None,
            announcement: "".into(),
            status_clock: PhaseClock::default(),
            save_clock: PhaseClock::default(),
            error_clock: PhaseClock::default(),
            reset: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_corner(mut self, corner: ActionMorphCorner) -> Self {
        self.corner = corner;
        self
    }

    pub fn with_reset_after(mut self, delay: Duration) -> Self {
        self.reset_after = delay;
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    /// Require the host to resolve requests with `complete`.
    pub fn with_deferred_submit(mut self) -> Self {
        self.deferred = true;
        self
    }

    /// Constrain an embedded surface independently of its window.
    pub fn set_max_width(&mut self, width: Pixels, cx: &mut Context<Self>) {
        let width = width.max(px(REST));
        if self.max_width != Some(width) {
            self.max_width = Some(width);
            cx.notify();
        }
    }

    pub fn is_saving(&self) -> bool {
        self.pending.is_some()
    }

    fn move_face(&mut self, next: Face, cx: &mut Context<Self>) {
        if self.face != next {
            let draft = if let Face::Compose(i) = self.face {
                Some(self.actions[i].editor.read(cx).value().to_owned().into())
            } else {
                None
            };
            self.departure = Some(Departure {
                face: self.face,
                draft,
                problem: self.problem.clone(),
            });
            self.direction = if next.depth() > self.face.depth() {
                1.
            } else {
                -1.
            };
            self.face_clock.restart(cx);
            self.face = next;
            self.pressed.reset(0.);
            cx.notify();
        }
    }

    fn focus_menu(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(slot) = self.actions.get(i) {
            self.active = i;
            self.hovered = None;
            let offset = self.menu_top(i);
            self.highlight.duration(offset, 0.18, cx);
            slot.menu_focus.focus(w, cx);
            cx.notify();
        }
    }

    fn open_menu(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.pending.is_some() || self.actions.is_empty() {
            return;
        }
        self.reset.take();
        self.success = None;
        self.announcement = "".into();
        let was_menu = self.face == Face::Menu;
        self.move_face(Face::Menu, cx);
        if !was_menu {
            self.highlight.reset(self.menu_top(i));
        }
        self.focus_menu(i, w, cx);
        self.problem = "".into();
    }

    fn open_composer(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.pending.is_some() || self.actions.get(i).is_none() {
            return;
        }
        self.move_face(Face::Compose(i), cx);
        self.set_problem("".into(), cx);
        self.actions[i].editor.read(cx).focus.clone().focus(w, cx);
        w.request_virtual_keyboard();
    }

    fn close(&mut self, restore_focus: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.pending.is_some() || self.face == Face::Rest {
            return;
        }
        self.move_face(Face::Rest, cx);
        self.problem = "".into();
        if restore_focus {
            self.trigger.focus(w, cx);
        }
    }

    fn set_problem(&mut self, message: SharedString, cx: &mut Context<Self>) {
        if !message.is_empty() {
            self.error_clock.restart(cx);
        }
        self.problem = message;
        if let Face::Compose(i) = self.face {
            self.actions[i].editor.update(cx, |ed, cx| {
                ed.description = self.problem.clone();
                cx.notify();
            });
        }
        cx.notify();
    }

    fn submit(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Face::Compose(i) = self.face else { return };
        let slot = &self.actions[i];
        let editor = slot.editor.read(cx);
        if self.pending.is_some() || editor.is_composing() {
            return;
        }
        let text = editor.value().trim();
        if text.is_empty() {
            let hint = slot.config.composer.empty_hint.clone().unwrap_or_else(|| {
                tr(cx, "Enter something to save.", "请输入要保存的内容。").into()
            });
            let editor = slot.editor.clone();
            self.set_problem(hint, cx);
            editor.read(cx).focus.clone().focus(w, cx);
            return;
        }
        self.sequence = self.sequence.wrapping_add(1);
        let ticket = self.sequence;
        let event = ActionMorphSubmit {
            ticket,
            action: slot.config.id.clone(),
            text: text.to_owned().into(),
            choice: slot
                .config
                .composer
                .choices
                .get(slot.choice)
                .map(|choice| choice.value.clone()),
        };
        self.pending = Some((ticket, i));
        self.save_clock.restart(cx);
        self.set_problem("".into(), cx);
        self.actions[i].editor.update(cx, |ed, cx| {
            ed.disabled = true;
            cx.notify();
        });
        cx.emit(event);
        if !self.deferred {
            self.complete(ticket, Ok(()), w, cx);
        }
    }

    /// Resolve the outstanding ticket. Stale completions are ignored.
    pub fn complete(
        &mut self,
        ticket: u64,
        result: Result<(), ActionMorphFailure>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((pending, i)) = self.pending else {
            return false;
        };
        if ticket != pending {
            return false;
        }
        self.pending = None;
        let editor = self.actions[i].editor.clone();
        editor.update(cx, |ed, cx| {
            ed.disabled = false;
            cx.notify();
        });
        if let Err(failure) = result {
            self.set_problem(failure.message, cx);
            editor.read(cx).focus.clone().focus(w, cx);
            return true;
        }
        let composer = &self.actions[i].config.composer;
        self.success = Some(composer.success_label.clone());
        self.announcement = composer.success_label.clone();
        self.actions[i].choice = composer.initial_choice();
        self.status_clock.restart(cx);
        self.move_face(Face::Rest, cx);
        editor.update(cx, |ed, cx| ed.set_value("", w, cx));
        self.trigger.focus(w, cx);
        let delay = self.reset_after;
        self.reset = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |s, cx| {
                if s.sequence == ticket && s.success.take().is_some() {
                    s.status_clock.restart(cx);
                    cx.notify();
                }
            });
        }));
        cx.notify();
        true
    }

    fn menu_top(&self, i: usize) -> f32 {
        self.actions
            .iter()
            .take(i)
            .map(|s| menu_row_height(&s.config) + 2.)
            .sum()
    }

    fn choose(&mut self, i: usize, choice: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.pending.is_some() || self.face != Face::Compose(i) {
            return;
        }
        let slot = &mut self.actions[i];
        if let Some(focus) = slot.choice_focus.get(choice) {
            slot.choice = choice;
            focus.focus(w, cx);
            cx.notify();
        }
    }

    fn keyboard(&mut self, event: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let stroke = &event.keystroke;
        if self.pending.is_some() {
            return;
        }
        match (self.face, stroke.key.as_str()) {
            (Face::Menu, "escape") => self.close(true, w, cx),
            (Face::Compose(i), "escape") => self.open_menu(i, w, cx),
            (Face::Menu, "up" | "down") => {
                let len = self.actions.len();
                if len == 0 {
                    return;
                }
                let next = if stroke.key == "up" {
                    (self.active + len - 1) % len
                } else {
                    (self.active + 1) % len
                };
                self.focus_menu(next, w, cx);
            }
            (Face::Menu, "home") => self.focus_menu(0, w, cx),
            (Face::Menu, "end") => self.focus_menu(self.actions.len().saturating_sub(1), w, cx),
            (Face::Menu, key) if !stroke.modifiers.modified() && !event.is_held => {
                let mut chars = key.chars();
                let Some(key) = chars.next().filter(|_| chars.next().is_none()) else {
                    return;
                };
                let Some(i) = self
                    .actions
                    .iter()
                    .position(|s| s.config.shortcut == Some(key.to_ascii_lowercase()))
                else {
                    return;
                };
                self.open_composer(i, w, cx);
            }
            _ => return,
        }
        w.prevent_default();
        cx.stop_propagation();
    }
}

fn menu_row_height(action: &ActionMorphAction) -> f32 {
    if action.description.is_some() {
        55.4
    } else {
        40.
    }
}

fn choice_height(composer: &ActionMorphComposer) -> f32 {
    if composer.choices_label.is_empty() {
        36.
    } else {
        58.8
    }
}

fn field_height(composer: &ActionMorphComposer) -> f32 {
    if composer.multiline { 88. } else { 42.4 }
}

#[derive(Clone, Copy)]
struct Skin {
    paint: Paint,
    surface: Hsla,
    field: Hsla,
    primary: Hsla,
    inverse: Hsla,
    hover: Hsla,
}

impl Skin {
    fn new(cx: &App) -> Self {
        let theme = Theme::global(cx);
        let p = Paint::get(cx);
        let dark = theme.mode == ThemeMode::Dark;
        let neutral: Hsla = rgb(if dark { 0xbebebe } else { 0x383838 }).into();
        let accent = theme.accent.for_mode(theme.mode);
        let primary = neutral.blend(accent);
        Self {
            paint: p,
            surface: rgb(if dark { 0x1f1f1f } else { 0xffffff }).into(),
            field: rgb(if dark { 0x242424 } else { 0xf4f4f4 }).into(),
            primary,
            inverse: theme.accent.on_fill(primary, p.inverse),
            hover: p.ink.opacity(if dark { 0.065 } else { 0.06 }),
        }
    }
}

fn check(progress: f32, color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let point_at = |x, y| b.origin + point(px(x), px(y));
            let a = point_at(3., 10.5);
            let bend = point_at(8., 15.5);
            let end = point_at(18., 5.5);
            let mut mark = PathBuilder::stroke(px(2.));
            mark.move_to(a);
            if progress < 1. / 3. {
                mark.line_to(a + (bend - a) * (progress * 3.));
            } else {
                mark.line_to(bend);
                mark.line_to(bend + (end - bend) * ((progress - 1. / 3.) * 1.5));
            }
            if let Ok(path) = mark.build() {
                w.paint_path(path, color);
            }
        },
    )
    .size(px(21.))
}

fn return_key(color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let at = |x, y| b.origin + point(px(x), px(y));
            let mut mark = PathBuilder::stroke(px(1.1));
            mark.move_to(at(10., 2.));
            mark.line_to(at(10., 7.));
            mark.line_to(at(2., 7.));
            mark.move_to(at(5., 4.));
            mark.line_to(at(2., 7.));
            mark.line_to(at(5., 10.));
            if let Ok(path) = mark.build() {
                w.paint_path(path, color);
            }
        },
    )
    .size(px(12.))
    .flex_none()
}

fn error_mark(color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let at = |x, y| b.origin + point(px(x), px(y));
            w.paint_quad(outline(b.inset(px(1.)), color, BorderStyle::Solid).corner_radii(px(6.)));
            let mut mark = PathBuilder::stroke(px(1.));
            mark.move_to(at(6.5, 3.8));
            mark.line_to(at(6.5, 6.9));
            mark.move_to(at(6.5, 8.5));
            mark.line_to(at(6.5, 9.4));
            if let Ok(path) = mark.build() {
                w.paint_path(path, color);
            }
        },
    )
    .size(px(13.))
    .flex_none()
}

impl ActionMorph {
    fn dimensions(&self, face: Face, available: f32, radius: f32) -> [f32; 3] {
        match face {
            Face::Rest => [REST, REST, REST / 2.],
            Face::Menu => [
                256_f32.min(available),
                38. + (self.menu_top(self.actions.len()) - 2.).max(0.) + 2. * PAD,
                radius,
            ],
            Face::Compose(i) => {
                let c = &self.actions[i].config.composer;
                let options = if c.choices.is_empty() {
                    0.
                } else {
                    choice_height(c) + 12.
                };
                [
                    320_f32.min(available),
                    96. + field_height(c) + options + 2. * PAD,
                    radius,
                ]
            }
        }
    }

    fn icon_button(
        id: &'static str,
        name: &'static str,
        glyph: Glyph,
        focus: &FocusHandle,
        interactive: bool,
        p: Paint,
        radius: Pixels,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .debug_selector(move || id.into())
            .size(px(32.))
            .flex_none()
            .flex()
            .justify_center()
            .items_center()
            .rounded(radius)
            .child(if glyph == Glyph::Arrow {
                transformed(
                    glyph,
                    15.,
                    p.secondary,
                    std::f32::consts::PI,
                    1.,
                    (0., 0.),
                    1.,
                )
                .into_any_element()
            } else {
                icon(glyph, 15., p.secondary).into_any_element()
            })
            .when(interactive, |d| {
                d.role(Role::Button)
                    .aria_label(name)
                    .track_focus(focus)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
            })
    }

    fn rest(&self, active: bool, skin: Skin, w: &Window, cx: &Context<Self>) -> Div {
        let p = skin.paint;
        let current = self.success.as_ref().filter(|_| active);
        let name = self
            .label
            .clone()
            .unwrap_or_else(|| tr(cx, "Create", "创建").into());
        let age = if active {
            self.status_clock.time(0.24, w, cx)
        } else {
            0.24
        };
        let stage = ease(age / 0.24);
        let plus_scale = if active && !disabled_motion(cx) {
            1. - 0.08 * self.pressed.get(w, cx)
        } else {
            1.
        };
        let mark = if current.is_some() {
            check(stage, p.success).into_any_element()
        } else {
            transformed(
                Glyph::Plus,
                22.,
                skin.inverse,
                (1. - stage) * -FRAC_PI_2,
                plus_scale,
                (0., 0.),
                1.,
            )
            .into_any_element()
        };
        let label: SharedString = match current {
            Some(success) => format!("{name}, {success}").into(),
            None => name,
        };
        let child = div()
            .id("action-morph-trigger")
            .debug_selector(|| "action-morph-trigger".into())
            .size_full()
            .flex()
            .justify_center()
            .items_center()
            .rounded_full()
            .child(mark);
        div().size_full().child(if active {
            press(
                child
                    .role(Role::Button)
                    .aria_label(label)
                    .track_focus(&self.trigger)
                    .cursor_pointer()
                    .focus_visible(move |s| s.border_2().border_color(p.focus))
                    .on_click(cx.listener(|s, _, w, cx| s.open_menu(0, w, cx))),
                cx,
                |s| &mut s.pressed,
            )
        } else {
            child
        })
    }

    fn menu(&self, active: bool, age: f32, skin: Skin, w: &Window, cx: &Context<Self>) -> Div {
        let p = skin.paint;
        let radius = surface_radius(cx);
        let title = self
            .label
            .clone()
            .unwrap_or_else(|| tr(cx, "Create", "创建").into());
        let close = Self::icon_button(
            "action-morph-menu-close",
            tr(cx, "Close actions", "关闭菜单"),
            Glyph::Close,
            &self.menu_close,
            active,
            p,
            radius,
        )
        .when(active, |d| {
            d.on_click(cx.listener(|s, _, w, cx| s.close(true, w, cx)))
        });
        let menu_height = (self.menu_top(self.actions.len()) - 2.).max(0.);
        let highlight = self.highlight.get(w, cx);
        let highlighted = self
            .actions
            .get(self.hovered.unwrap_or(self.active))
            .map_or(0., |s| menu_row_height(&s.config));
        let mut items = div()
            .id("action-morph-menu")
            .debug_selector(|| "action-morph-menu".into())
            .relative()
            .flex()
            .flex_col()
            .gap(px(2.))
            .h(px(menu_height))
            .when(active, |d| d.role(Role::Menu).aria_label(title.clone()))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(highlight))
                    .h(px(highlighted))
                    .rounded(radius)
                    .bg(skin.hover),
            );
        for (i, slot) in self.actions.iter().enumerate() {
            let action = &slot.config;
            let motion = if active && !disabled_motion(cx) {
                ease((age - (i.min(5) as f32 * 0.028)) / 0.14)
            } else {
                1.
            };
            let name: SharedString = action.description.as_ref().map_or_else(
                || action.label.clone(),
                |d| format!("{} {d}", action.label).into(),
            );
            let debug = action.id.clone();
            let item = row()
                .id(("action-morph-menu-item", i))
                .debug_selector(move || format!("action-morph-menu-{debug}"))
                .relative()
                .top(px((1. - motion) * 5.))
                .opacity(motion)
                .h(px(menu_row_height(action)))
                .flex_none()
                .px(px(10.))
                .gap_3()
                .rounded(radius)
                .child(
                    div()
                        .size(px(20.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(action.icon, 18., p.secondary)),
                )
                .child(
                    column()
                        .flex_1()
                        .gap(px(1.))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(14.))
                                .line_height(px(19.6))
                                .child(action.label.clone()),
                        )
                        .when_some(action.description.clone(), |c, description| {
                            c.child(
                                div()
                                    .truncate()
                                    .text_size(px(12.))
                                    .line_height(px(16.8))
                                    .text_color(p.secondary)
                                    .child(description),
                            )
                        }),
                )
                .when_some(action.shortcut, |d, key| {
                    d.child(
                        div()
                            .size(px(22.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(radius)
                            .border_1()
                            .border_color(p.line)
                            .text_size(px(11.))
                            .text_color(p.faint)
                            .child(key.to_uppercase().to_string()),
                    )
                })
                .when(active, |b| {
                    b.role(Role::MenuItem)
                        .aria_label(name)
                        .when_some(action.shortcut, |b, key| {
                            b.aria_keyshortcuts(key.to_string())
                        })
                        .track_focus(&slot.menu_focus)
                        .tab_stop(self.active == i)
                        .cursor_pointer()
                        .on_mouse_move(cx.listener(move |s, _: &MouseMoveEvent, _, cx| {
                            if s.hovered != Some(i) {
                                s.hovered = Some(i);
                                s.highlight.duration(s.menu_top(i), 0.18, cx);
                                cx.notify();
                            }
                        }))
                        .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                            if !over && s.hovered == Some(i) {
                                s.hovered = None;
                                s.highlight.duration(s.menu_top(s.active), 0.18, cx);
                                cx.notify();
                            }
                        }))
                        .on_click(cx.listener(move |s, _, w, cx| s.open_composer(i, w, cx)))
                });
            items = items.child(item);
        }
        column()
            .w_full()
            .p(px(PAD))
            .child(
                row()
                    .h(px(38.))
                    .pl(px(10.))
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(title),
                    )
                    .child(close),
            )
            .child(items)
    }

    fn choices(&self, index: usize, active: bool, skin: Skin, cx: &Context<Self>) -> Div {
        let slot = &self.actions[index];
        let c = &slot.config.composer;
        let p = skin.paint;
        let busy = active && self.pending.is_some();
        let name = if c.choices_label.is_empty() {
            tr(cx, "Choices", "选项").into()
        } else {
            c.choices_label.clone()
        };
        let mut rail = row()
            .id("action-morph-choices")
            .w_full()
            .h(px(36.))
            .p(px(3.))
            .rounded_full()
            .bg(skin.hover)
            .when(active, |d| d.role(Role::RadioGroup).aria_label(name));
        for (i, item) in c.choices.iter().enumerate() {
            let chosen = slot.choice == i;
            rail = rail.child(
                div()
                    .id(("action-morph-choice", i))
                    .h(px(30.))
                    .flex_1()
                    .min_w_0()
                    .px(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .text_size(px(12.))
                    .text_color(if chosen { p.ink } else { p.secondary })
                    .when(chosen, |d| d.bg(p.panel))
                    .child(div().truncate().child(item.label.clone()))
                    .when(active, |d| {
                        d.role(Role::RadioButton)
                            .aria_label(item.label.clone())
                            .aria_toggled(chosen.into())
                            .track_focus(&slot.choice_focus[i])
                            .tab_stop(chosen && !busy)
                            .when(!busy, |d| {
                                d.cursor_pointer()
                                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                                    .on_click(
                                        cx.listener(move |s, _, w, cx| s.choose(index, i, w, cx)),
                                    )
                                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                        let count = s.actions[index].config.composer.choices.len();
                                        let target = match e.keystroke.key.as_str() {
                                            "left" | "right" => {
                                                let step = UiDirection::current(cx).inline_step(
                                                    if e.keystroke.key == "left" { -1 } else { 1 },
                                                );
                                                (i as isize + step).rem_euclid(count as isize)
                                                    as usize
                                            }
                                            "up" => (i + count - 1) % count,
                                            "down" => (i + 1) % count,
                                            "home" => 0,
                                            "end" => count - 1,
                                            _ => return,
                                        };
                                        s.choose(index, target, w, cx);
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    }))
                            })
                    }),
            );
        }
        column()
            .w_full()
            .px(px(2.))
            .h(px(choice_height(c)))
            .gap(px(6.))
            .when(!c.choices_label.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.8))
                        .text_color(p.secondary)
                        .child(c.choices_label.clone()),
                )
            })
            .child(rail)
    }

    fn composer(
        &self,
        index: usize,
        previous: Option<&Departure>,
        skin: Skin,
        w: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let slot = &self.actions[index];
        let c = &slot.config.composer;
        let p = skin.paint;
        let radius = surface_radius(cx);
        let active = previous.is_none();
        let busy = active && self.pending.is_some();
        let interactive = active && !busy;
        let problem = previous.map_or(&self.problem, |old| &old.problem);
        let back = Self::icon_button(
            "action-morph-back",
            tr(cx, "Back to actions", "返回动作"),
            Glyph::Arrow,
            &self.back,
            interactive,
            p,
            radius,
        )
        .when(interactive, |d| {
            d.on_click(cx.listener(move |s, _, w, cx| s.open_menu(index, w, cx)))
        });
        let close = Self::icon_button(
            "action-morph-close",
            tr(cx, "Close composer", "关闭表单"),
            Glyph::Close,
            &self.close,
            interactive,
            p,
            radius,
        )
        .when(interactive, |d| {
            d.on_click(cx.listener(|s, _, w, cx| s.close(true, w, cx)))
        });
        let editor_focused = active && slot.editor.read(cx).focus.is_focused(w);
        let shake = if active && !problem.is_empty() && !disabled_motion(cx) {
            let age = self.error_clock.time(0.26, w, cx);
            (age * 62.).sin() * 4. * (1. - age / 0.26)
        } else {
            0.
        };
        let field = div()
            .id("action-morph-field")
            .relative()
            .left(px(shake))
            .debug_selector(|| "action-morph-field".into())
            .w_full()
            .h(px(field_height(c)))
            .rounded(radius)
            .bg(skin.field)
            .border_1()
            .border_color(if !problem.is_empty() {
                p.danger.opacity(0.65)
            } else if editor_focused {
                p.focus.opacity(0.48)
            } else {
                p.line
            })
            .px(px(11.))
            .when(c.multiline, |d| d.py(px(6.)))
            .child(if let Some(old) = previous {
                let value = old.draft.as_ref().filter(|s| !s.is_empty()).cloned();
                div()
                    .pt(px(8.))
                    .text_size(px(16.))
                    .line_height(px(22.4))
                    .text_color(if value.is_some() { p.ink } else { p.faint })
                    .child(value.unwrap_or_else(|| c.placeholder.clone()))
                    .into_any_element()
            } else {
                slot.editor.clone().into_any_element()
            });
        let button_text = if busy {
            tr(cx, "Saving", "保存中").into()
        } else {
            c.submit_label.clone()
        };
        let mac = cx.platform_info().is_mac();
        let keys = if c.multiline && !mac {
            36.
        } else if c.multiline {
            22.
        } else {
            14.
        };
        let size = (measure(&c.submit_label, 14., w) + 36. + keys).max(90.);
        let submit = row()
            .id("action-morph-submit")
            .debug_selector(|| "action-morph-submit".into())
            .w(px(size))
            .h(px(36.))
            .flex_none()
            .justify_center()
            .gap(px(5.))
            .rounded(radius)
            .bg(skin.primary)
            .text_color(skin.inverse)
            .text_size(px(14.))
            .font_weight(FontWeight::MEDIUM)
            .child(div().truncate().child(button_text))
            .when_else(
                busy,
                |d| {
                    let turn = if disabled_motion(cx) {
                        0.
                    } else {
                        w.request_animation_frame();
                        self.save_clock.age(cx) * 5.8
                    };
                    d.opacity(0.74).child(spinner(turn, 14., skin.inverse))
                },
                |d| {
                    d.child(
                        row()
                            .gap(px(2.))
                            .when(c.multiline, |d| {
                                d.child(div().text_size(px(12.)).child(if mac {
                                    "⌘"
                                } else {
                                    "Ctrl"
                                }))
                            })
                            .child(return_key(skin.inverse)),
                    )
                },
            )
            .when(active && busy, |d| {
                d.role(Role::Status).aria_label(tr(cx, "Saving", "保存中"))
            })
            .when(interactive, |d| {
                d.track_focus(&self.submit)
                    .role(Role::Button)
                    .aria_label(c.submit_label.clone())
                    .cursor_pointer()
                    .hover(move |s| s.bg(skin.primary.blend(skin.inverse.opacity(0.10))))
                    .focus_visible(move |s| s.border_2().border_color(p.focus))
                    .on_click(cx.listener(|s, _, w, cx| s.submit(w, cx)))
            });
        column()
            .id("action-morph-composer")
            .debug_selector(|| "action-morph-composer".into())
            .w_full()
            .p(px(PAD))
            .gap(px(12.))
            .when(active, |d| d.role(Role::Form).aria_label(c.title.clone()))
            .child(
                row()
                    .h(px(36.))
                    .gap_1()
                    .child(back)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .pl(px(2.))
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(c.title.clone()),
                    )
                    .child(close),
            )
            .child(field)
            .when(!c.choices.is_empty(), |d| {
                d.child(self.choices(index, active, skin, cx))
            })
            .child(
                row()
                    .h(px(36.))
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        row()
                            .id("action-morph-problem")
                            .flex_1()
                            .gap(px(5.))
                            .text_color(p.danger)
                            .when(!problem.is_empty(), |d| {
                                d.child(error_mark(p.danger)).child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(12.))
                                        .child(problem.clone()),
                                )
                            })
                            .when(active && !problem.is_empty(), |d| {
                                d.role(Role::Alert).aria_label(problem.clone())
                            }),
                    )
                    .child(submit),
            )
    }

    fn content(
        &self,
        face: Face,
        previous: Option<&Departure>,
        age: f32,
        skin: Skin,
        w: &Window,
        cx: &Context<Self>,
    ) -> AnyElement {
        match face {
            Face::Rest => self
                .rest(previous.is_none(), skin, w, cx)
                .into_any_element(),
            Face::Menu => self
                .menu(previous.is_none(), age, skin, w, cx)
                .into_any_element(),
            Face::Compose(index) => self
                .composer(index, previous, skin, w, cx)
                .into_any_element(),
        }
    }
}

impl Render for ActionMorph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let skin = Skin::new(cx);
        let p = skin.paint;
        let reduced = disabled_motion(cx);
        let available = (w.viewport_size().width - px(32.))
            .min(self.max_width.unwrap_or(px(320.)))
            .max(px(REST))
            .into();
        let radius = surface_radius(cx).into();
        let target = self.dimensions(self.face, available, radius);
        let expanding = self.direction > 0.;
        let mut running = false;
        let geometry = std::array::from_fn::<_, 3, _>(|i| {
            self.springs[i].set(target[i], expanding, cx);
            let (value, _, active) = self.springs[i].sample(cx);
            running |= active;
            value
        });
        if running {
            w.request_animation_frame();
        }
        let [width, height, corner] = geometry;
        let r = px(corner.min(width.min(height) / 2.).max(0.));
        let age = self.face_clock.age(cx);
        if age >= FACE_SECONDS {
            self.departure = None;
        }
        if age < FACE_SECONDS {
            w.request_animation_frame();
        }
        let rtl = UiDirection::current(cx) == UiDirection::Rtl;
        let anchor_left = (self.corner == ActionMorphCorner::Start) != rtl;
        let offset = if reduced {
            0.
        } else {
            22. * self.direction * if rtl { -1. } else { 1. }
        };
        let blend = ease(age / FACE_SECONDS);
        let incoming = ease((age - 0.05) / 0.17);
        let outgoing = ease(age / 0.13);
        let old = self.departure.as_ref().map_or(self.face, |old| old.face);
        let success_color = skin.surface.blend(p.success.opacity(0.14));
        let restore = if reduced {
            1.
        } else {
            ease(self.status_clock.age(cx) / 0.24)
        };
        let base = |face| {
            if face == Face::Rest {
                if self.success.is_some() && self.face == Face::Rest {
                    success_color
                } else {
                    success_color.blend(skin.primary.opacity(restore))
                }
            } else {
                skin.surface
            }
        };
        let bg = base(old).blend(base(self.face).opacity(blend));
        let content = |face, departure: Option<&Departure>, opacity: f32, dx: f32| {
            let [w_target, h_target, _] = self.dimensions(face, available, radius);
            div()
                .absolute()
                .bottom_0()
                .when_else(anchor_left, |d| d.left(px(dx)), |d| d.right(px(-dx)))
                .w(px(w_target))
                .h(px(h_target))
                .opacity(opacity)
                .child(self.content(face, departure, age, skin, w, cx))
        };
        let body = div()
            .id("action-morph-surface")
            .debug_selector(|| "action-morph-surface".into())
            .absolute()
            .bottom_0()
            .when_else(anchor_left, |d| d.left_0(), |d| d.right_0())
            .w(px(width))
            .h(px(height))
            .rounded(r)
            .bg(bg)
            .shadow(vec![
                BoxShadow::new(px(0.), px(12.), black().opacity(0.25))
                    .blur_radius(px(30.))
                    .spread_radius(px(-5.)),
                BoxShadow::new(
                    px(0.),
                    px(0.),
                    p.ink
                        .opacity(if self.face == Face::Rest { 0.04 } else { 0.07 }),
                )
                .spread_radius(px(1.))
                .inset(),
            ])
            .overflow_hidden()
            .block_mouse_except_scroll()
            .text_color(p.ink)
            .on_key_down(cx.listener(Self::keyboard))
            .on_mouse_down_out(cx.listener(|s, _, w, cx| s.close(false, w, cx)))
            .when_some(self.departure.as_ref(), |d, old| {
                d.child(content(old.face, Some(old), 1. - outgoing, -offset * blend))
            })
            .child(content(self.face, None, incoming, offset * (1. - blend)));
        let root = div()
            .id(self.id.clone())
            .relative()
            .size(px(REST))
            .flex_none()
            .child(body)
            .when(!self.announcement.is_empty(), |d| {
                d.child(
                    div()
                        .id("action-morph-status")
                        .role(Role::Status)
                        .aria_label(self.announcement.clone())
                        .absolute()
                        .size(px(1.))
                        .opacity(0.),
                )
            });
        let anchor = div().relative().size(px(REST)).flex_none();
        match &self.style {
            Some(style) => style(anchor),
            None => anchor,
        }
        .child(root)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActionMorph, ActionMorphAction, ActionMorphChoice, ActionMorphComposer, ActionMorphFailure,
        ActionMorphSubmit, Face,
    };
    use crate::{foundation::Theme, primitives::glyph::Glyph};
    use gpui::{Context, Modifiers, TestAppContext, VisualTestContext, Window, px};
    use std::{cell::RefCell, rc::Rc, time::Duration};

    fn control(w: &mut Window, cx: &mut Context<ActionMorph>) -> ActionMorph {
        ActionMorph::new(
            vec![
                ActionMorphAction::new(
                    "task",
                    "Task",
                    Glyph::Check,
                    ActionMorphComposer::new("New task", "Task title", "Add", "Task added")
                        .with_choices(
                            "Due",
                            vec![
                                ActionMorphChoice::new("today", "Today"),
                                ActionMorphChoice::new("later", "Later"),
                            ],
                        )
                        .with_empty_hint("A title is required"),
                )
                .with_shortcut('t'),
                ActionMorphAction::new(
                    "note",
                    "Note",
                    Glyph::FileText,
                    ActionMorphComposer::new("New note", "Write", "Save", "Note saved")
                        .with_multiline(true),
                )
                .with_shortcut('n'),
            ],
            w,
            cx,
        )
        .with_deferred_submit()
    }

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
    fn failed_request_preserves_the_draft_and_only_the_active_ticket_can_commit() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(control);
        let requests = Rc::new(RefCell::new(Vec::new()));
        let _sub = w.update(|_, cx| {
            let requests = requests.clone();
            cx.subscribe(&view, move |_, event: &ActionMorphSubmit, _| {
                requests.borrow_mut().push(event.clone())
            })
        });
        let trigger = w.debug_bounds("action-morph-trigger").unwrap().center();
        w.simulate_click(trigger, Modifiers::default());
        tick(w, 800);
        w.simulate_keystrokes("t");
        tick(w, 800);
        w.update(|w, cx| {
            view.update(cx, |s, cx| {
                assert_eq!(s.face, Face::Compose(0));
                s.submit(w, cx);
                assert_eq!(s.problem.as_ref(), "A title is required");
                s.actions[0]
                    .editor
                    .update(cx, |ed, cx| ed.set_value("  Keep this draft  ", w, cx));
                s.choose(0, 1, w, cx);
                s.submit(w, cx);
                s.submit(w, cx);
            })
        });
        assert_eq!(requests.borrow().len(), 1);
        let first = requests.borrow()[0].clone();
        assert_eq!(first.text.as_ref(), "Keep this draft");
        assert_eq!(first.choice.as_deref(), Some("later"));
        w.update(|w, cx| {
            view.update(cx, |s, cx| {
                assert!(s.complete(
                    first.ticket,
                    Err(ActionMorphFailure::new("Try again")),
                    w,
                    cx
                ));
                assert_eq!(s.actions[0].editor.read(cx).value(), "  Keep this draft  ");
                assert_eq!(s.actions[0].choice, 1);
                s.open_menu(0, w, cx);
                s.open_composer(1, w, cx);
                assert!(s.actions[1].editor.read(cx).value().is_empty());
                s.open_menu(1, w, cx);
                s.open_composer(0, w, cx);
                s.submit(w, cx);
            })
        });
        assert_eq!(requests.borrow().len(), 2);
        let next = requests.borrow()[1].ticket;
        w.update(|w, cx| {
            view.update(cx, |s, cx| {
                assert!(!s.complete(first.ticket, Ok(()), w, cx));
                assert!(s.complete(next, Ok(()), w, cx));
                assert!(!s.complete(next, Ok(()), w, cx));
                assert!(s.actions[0].editor.read(cx).value().is_empty());
                assert_eq!(s.face, Face::Rest);
                assert!(s.trigger.is_focused(w));
                assert_eq!(s.success.as_deref(), Some("Task added"));
            })
        });
        tick(w, 1599);
        assert!(w.update(|_, cx| view.read(cx).success.is_some()));
        tick(w, 1);
        assert!(w.update(|_, cx| view.read(cx).success.is_none()));
    }

    #[test]
    fn multiline_enter_inserts_a_line_and_modified_enter_submits() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let (view, w) = app.add_window_view(control);
        let trigger = w.debug_bounds("action-morph-trigger").unwrap().center();
        w.simulate_click(trigger, Modifiers::default());
        tick(w, 0);
        w.simulate_keystrokes("up");
        w.update(|w, cx| assert!(view.read(cx).actions[1].menu_focus.is_focused(w)));
        w.simulate_keystrokes("n");
        tick(w, 0);
        w.update(|w, cx| {
            view.update(cx, |s, cx| {
                assert_eq!(s.face, Face::Compose(1));
                s.set_max_width(px(256.), cx);
                s.actions[1]
                    .editor
                    .update(cx, |ed, cx| ed.set_value("First line", w, cx));
            })
        });
        tick(w, 0);
        assert_eq!(
            w.debug_bounds("action-morph-surface").unwrap().size.width,
            px(256.)
        );
        w.simulate_keystrokes("enter");
        assert!(w.update(|_, cx| {
            view.read(cx).actions[1]
                .editor
                .read(cx)
                .value()
                .contains('\n')
        }));
        assert!(!w.update(|_, cx| view.read(cx).is_saving()));
        w.simulate_keystrokes("ctrl-enter");
        assert!(w.update(|_, cx| view.read(cx).is_saving()));
    }

    #[test]
    fn interrupted_surface_retargets_from_the_displayed_bounds() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(control);
        let rest = w.debug_bounds("action-morph-surface").unwrap();
        w.simulate_click(rest.center(), Modifiers::default());
        tick(w, 65);
        let opening = w.debug_bounds("action-morph-surface").unwrap();
        assert!(opening.size.width > rest.size.width && opening.size.width < px(256.));
        assert_eq!(opening.right(), rest.right());
        assert_eq!(opening.bottom(), rest.bottom());
        w.simulate_keystrokes("escape");
        tick(w, 0);
        let returning = w.debug_bounds("action-morph-surface").unwrap();
        assert!((opening.size.width - returning.size.width).abs() <= px(1.));
        tick(w, 800);
        assert_eq!(w.debug_bounds("action-morph-surface").unwrap(), rest);
        assert!(w.update(|w, cx| view.read(cx).trigger.is_focused(w)));
    }
}
