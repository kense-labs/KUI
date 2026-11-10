//! A share button that opens into an access and recipient picker.
use crate::{
    foundation::surface_radius,
    primitives::{
        clipboard,
        direct_touch::Contact,
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, ease, face, measure},
        flyout::TriggerBounds,
        glyph::{Glyph, icon, spinner},
        menu::MenuScrollRegion,
        presentation::tr,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const PANEL_WIDTH: f32 = 384.;
const SENT_WIDTH: f32 = 304.;
const TRIGGER_HEIGHT: f32 = 36.;
const FACE_SECONDS: f32 = 0.18;
const FLIGHT_SECONDS: f32 = 0.32;

/// A recent recipient. Supply a unique, stable ID and a small avatar image.
#[derive(Clone)]
pub struct SharePerson {
    pub id: SharedString,
    pub name: SharedString,
    pub avatar: Option<ImageSource>,
    pub accent: Option<[u32; 2]>,
}

impl SharePerson {
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            avatar: None,
            accent: None,
        }
    }

    pub fn with_avatar(mut self, source: impl Into<ImageSource>) -> Self {
        self.avatar = Some(source.into());
        self
    }

    pub fn with_accent(mut self, colors: [u32; 2]) -> Self {
        self.accent = Some(colors);
        self
    }

    fn given_name(&self) -> SharedString {
        self.name
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_owned()
            .into()
    }
}

/// A decorative KUI glyph or an application-provided GPUI icon.
#[derive(Clone)]
pub enum ShareSheetIcon {
    Glyph(Glyph),
    View(AnyView),
}

impl From<Glyph> for ShareSheetIcon {
    fn from(glyph: Glyph) -> Self {
        Self::Glyph(glyph)
    }
}

impl From<AnyView> for ShareSheetIcon {
    fn from(view: AnyView) -> Self {
        Self::View(view)
    }
}

impl ShareSheetIcon {
    fn element(&self, side: f32, ink: Hsla) -> AnyElement {
        match self {
            Self::Glyph(glyph) => icon(*glyph, side, ink).into_any_element(),
            Self::View(view) => div()
                .size(px(side))
                .flex_none()
                .child(view.clone())
                .into_any_element(),
        }
    }
}

/// One selectable link access level.
#[derive(Clone)]
pub struct ShareSheetAccess {
    pub value: SharedString,
    pub label: SharedString,
    pub description: SharedString,
    pub icon: Option<ShareSheetIcon>,
}

impl ShareSheetAccess {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: "".into(),
            icon: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_icon(mut self, glyph: impl Into<ShareSheetIcon>) -> Self {
        self.icon = Some(glyph.into());
        self
    }
}

/// A host-provided destination with its own progress and result.
#[derive(Clone)]
pub struct ShareChannel {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Option<ShareSheetIcon>,
    pub done_label: Option<SharedString>,
}

impl ShareChannel {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            done_label: None,
        }
    }

    pub fn with_icon(mut self, glyph: impl Into<ShareSheetIcon>) -> Self {
        self.icon = Some(glyph.into());
        self
    }

    pub fn with_done_label(mut self, label: impl Into<SharedString>) -> Self {
        self.done_label = Some(label.into());
        self
    }
}

/// The trigger edge preserved as the desktop surface changes width.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShareSheetAlign {
    Start,
    #[default]
    End,
    Center,
}

/// A snapshot of the recipients and access at submission time.
#[derive(Clone)]
pub struct ShareSubmission {
    pub people: Vec<SharePerson>,
    pub access: SharedString,
}

type ShareResult = Task<Result<(), SharedString>>;
type Send = Box<dyn Fn(ShareSubmission, &mut Window, &mut App) -> ShareResult>;
type Channel = Box<dyn Fn(SharedString, &mut Window, &mut App) -> ShareResult>;
type Copy = Box<dyn Fn(SharedString, &mut Window, &mut App)>;
type Style = Box<dyn Fn(Div) -> Div>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Surface {
    Trigger,
    Panel,
    Sent,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Feedback {
    #[default]
    Idle,
    Pending,
    Done,
    Failed,
}

struct Flight {
    person: usize,
    from: Bounds<Pixels>,
    to_chip: bool,
    started: Instant,
}

impl Flight {
    fn sample(&self, to: Bounds<Pixels>, now: Instant) -> (Bounds<Pixels>, bool) {
        let elapsed = now.saturating_duration_since(self.started).as_secs_f32();
        let t = ease(elapsed / FLIGHT_SECONDS);
        let x = self.from.left() + (to.left() - self.from.left()) * t;
        let y = self.from.top() + (to.top() - self.from.top()) * t;
        let width = self.from.size.width + (to.size.width - self.from.size.width) * t;
        let height = self.from.size.height + (to.size.height - self.from.size.height) * t;
        (
            Bounds::new(
                point(x, y - px(12. * 4. * t * (1. - t))),
                size(width, height),
            ),
            elapsed < FLIGHT_SECONDS,
        )
    }
}

#[derive(Default)]
struct PersonBounds {
    recent: Rc<Cell<Bounds<Pixels>>>,
    chip: Rc<Cell<Bounds<Pixels>>>,
}

/// Sharing UI backed by host operations and the platform clipboard.
pub struct ShareSheet {
    id: ElementId,
    title: SharedString,
    link: SharedString,
    people: Vec<SharePerson>,
    access: Vec<ShareSheetAccess>,
    channels: Vec<ShareChannel>,
    selected: Vec<usize>,
    access_index: usize,
    access_open: bool,
    access_active: usize,
    channel_feedback: Vec<Feedback>,
    copy_feedback: Feedback,
    send_error: Option<SharedString>,
    status: SharedString,
    sent: Option<ShareSubmission>,
    sending: bool,
    on_send: Option<Send>,
    on_channel: Option<Channel>,
    on_copy: Option<Copy>,
    label: Option<SharedString>,
    align: ShareSheetAlign,
    sheet_on_phones: bool,
    width: f32,
    style: Option<Style>,
    surface: Surface,
    leaving: Surface,
    face_clock: PhaseClock,
    shake: PhaseClock,
    count_clock: PhaseClock,
    pending_clock: PhaseClock,
    previous_count: usize,
    geometry: [Slide; 4],
    phone_open: Slide,
    initialized: bool,
    drag_start: Option<Pixels>,
    contact: Contact,
    panel_scroll: ScrollHandle,
    trigger_bounds: TriggerBounds,
    people_bounds: Vec<PersonBounds>,
    flights: Vec<Flight>,
    trigger_focus: FocusHandle,
    scope: FocusHandle,
    copy_focus: FocusHandle,
    access_focus: FocusHandle,
    options_focus: Vec<FocusHandle>,
    done_focus: FocusHandle,
    send_focus: FocusHandle,
}

impl ShareSheet {
    /// Starts closed. The first access level is selected by default.
    pub fn new(
        title: impl Into<SharedString>,
        link: impl Into<SharedString>,
        people: Vec<SharePerson>,
        access: Vec<ShareSheetAccess>,
        cx: &App,
    ) -> Self {
        let people_bounds = (0..people.len()).map(|_| PersonBounds::default()).collect();
        let options_focus = access.iter().map(|_| cx.focus_handle()).collect();
        Self {
            id: "share-sheet".into(),
            title: title.into(),
            link: link.into(),
            people,
            access,
            channels: vec![],
            selected: vec![],
            access_index: 0,
            access_open: false,
            access_active: 0,
            channel_feedback: vec![],
            copy_feedback: Feedback::Idle,
            send_error: None,
            status: "".into(),
            sent: None,
            sending: false,
            on_send: None,
            on_channel: None,
            on_copy: None,
            label: None,
            align: ShareSheetAlign::End,
            sheet_on_phones: true,
            width: PANEL_WIDTH,
            style: None,
            surface: Surface::Trigger,
            leaving: Surface::Trigger,
            face_clock: PhaseClock::default(),
            shake: PhaseClock::default(),
            count_clock: PhaseClock::default(),
            pending_clock: PhaseClock::default(),
            previous_count: 0,
            geometry: [Slide::new(0.); 4],
            phone_open: Slide::new(0.),
            initialized: false,
            drag_start: None,
            contact: Contact::default(),
            panel_scroll: ScrollHandle::new(),
            trigger_bounds: TriggerBounds::default(),
            people_bounds,
            flights: vec![],
            trigger_focus: cx.focus_handle(),
            scope: cx.focus_handle(),
            copy_focus: cx.focus_handle(),
            access_focus: cx.focus_handle(),
            options_focus,
            done_focus: cx.focus_handle(),
            send_focus: cx.focus_handle(),
        }
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_channels(mut self, channels: Vec<ShareChannel>) -> Self {
        self.channel_feedback = vec![Feedback::Idle; channels.len()];
        self.channels = channels;
        self
    }

    /// Selects a known access value. Unknown values keep the first option.
    pub fn with_default_access(mut self, value: &str) -> Self {
        if let Some(index) = self.access.iter().position(|level| level.value == value) {
            self.access_index = index;
        }
        self
    }

    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_align(mut self, align: ShareSheetAlign) -> Self {
        self.align = align;
        self
    }

    /// Enables a draggable bottom sheet at window widths of 520px or less.
    pub fn with_sheet_on_phones(mut self, enabled: bool) -> Self {
        self.sheet_on_phones = enabled;
        self
    }

    /// Sets the preferred desktop width in logical pixels.
    pub fn with_width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0. {
            self.width = width;
        }
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn on_send(
        mut self,
        send: impl Fn(ShareSubmission, &mut Window, &mut App) -> ShareResult + 'static,
    ) -> Self {
        self.on_send = Some(Box::new(send));
        self
    }

    pub fn on_channel(
        mut self,
        send: impl Fn(SharedString, &mut Window, &mut App) -> ShareResult + 'static,
    ) -> Self {
        self.on_channel = Some(Box::new(send));
        self
    }

    pub fn on_copy(
        mut self,
        copied: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_copy = Some(Box::new(copied));
        self
    }

    fn access_value(&self) -> SharedString {
        self.access
            .get(self.access_index)
            .map(|a| a.value.clone())
            .unwrap_or_default()
    }

    fn change_surface(&mut self, surface: Surface, cx: &mut Context<Self>) {
        if self.surface == surface {
            return;
        }
        self.leaving = self.surface;
        self.surface = surface;
        self.face_clock.restart(cx);
        self.drag_start = None;
        self.access_open = false;
        if surface != Surface::Trigger {
            self.panel_scroll.set_offset(point(px(0.), px(0.)));
        }
        self.flights.clear();
        cx.notify();
    }

    fn open(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.surface == Surface::Trigger {
            if self.copy_feedback != Feedback::Pending {
                self.copy_feedback = Feedback::Idle;
            }
            self.status = "".into();
            self.change_surface(Surface::Panel, cx);
            self.copy_focus.focus(w, cx);
        }
    }

    fn close(&mut self, restore: bool, w: &mut Window, cx: &mut Context<Self>) {
        self.change_surface(Surface::Trigger, cx);
        if restore {
            self.trigger_focus.focus(w, cx);
        }
    }

    fn copy(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.surface != Surface::Panel || self.copy_feedback == Feedback::Pending {
            return;
        }
        self.copy_feedback = Feedback::Pending;
        self.pending_clock.restart(cx);
        cx.notify();
        let access = self.access_value();
        let task = clipboard::write(&self.link, cx);
        cx.spawn_in(w, async move |weak, cx| {
            let ok = task.await;
            let _ = weak.update_in(cx, |s, w, cx| {
                s.copy_feedback = if ok { Feedback::Done } else { Feedback::Failed };
                s.status = if ok {
                    tr(cx, "Link copied", "已复制链接")
                } else {
                    tr(cx, "Could not copy the link", "无法复制链接")
                }
                .into();
                if ok && let Some(callback) = &s.on_copy {
                    callback(access, w, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn choose_channel(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.surface != Surface::Panel || self.channel_feedback[index] == Feedback::Pending {
            return;
        }
        let id = self.channels[index].id.clone();
        let task = self
            .on_channel
            .as_ref()
            .map_or_else(|| Task::ready(Ok(())), |callback| callback(id, w, cx));
        self.channel_feedback[index] = Feedback::Pending;
        self.pending_clock.restart(cx);
        cx.notify();
        cx.spawn_in(w, async move |weak, cx| {
            let result = task.await;
            let _ = weak.update_in(cx, |s, _, cx| {
                s.channel_feedback[index] = if result.is_ok() {
                    Feedback::Done
                } else {
                    Feedback::Failed
                };
                let channel = &s.channels[index];
                let status = status_label(
                    s.channel_feedback[index],
                    channel.label.clone(),
                    channel
                        .done_label
                        .clone()
                        .unwrap_or_else(|| tr(cx, "Done", "完成").into()),
                    cx,
                );
                s.status = format!("{} · {status}", channel.label).into();
                cx.notify();
            });
        })
        .detach();
    }

    fn send(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.surface != Surface::Panel || self.sending {
            return;
        }
        if self.selected.is_empty() {
            self.shake.restart(cx);
            self.status = tr(cx, "Pick a recipient first", "请先选择收件人").into();
            cx.notify();
            return;
        }
        let request = ShareSubmission {
            people: self
                .selected
                .iter()
                .map(|&i| self.people[i].clone())
                .collect(),
            access: self.access_value(),
        };
        let task = self.on_send.as_ref().map_or_else(
            || Task::ready(Ok(())),
            |callback| callback(request.clone(), w, cx),
        );
        self.sending = true;
        self.pending_clock.restart(cx);
        self.send_error = None;
        cx.notify();
        cx.spawn_in(w, async move |weak, cx| {
            let result = task.await;
            let _ = weak.update_in(cx, |s, w, cx| {
                s.sending = false;
                match result {
                    Ok(()) => {
                        s.sent = Some(request);
                        if s.surface == Surface::Panel {
                            s.change_surface(Surface::Sent, cx);
                            s.done_focus.focus(w, cx);
                        }
                    }
                    Err(error) => {
                        s.send_error = Some(if error.is_empty() {
                            tr(cx, "Couldn’t send. Try again.", "发送失败，请重试。").into()
                        } else {
                            error
                        });
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_person(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if self.surface != Surface::Panel || self.sending {
            return;
        }
        let to_chip = !self.selected.contains(&i);
        let bounds = &self.people_bounds[i];
        let now = cx.background_executor().now();
        let from = self
            .flights
            .iter()
            .find(|flight| flight.person == i)
            .map_or_else(
                || {
                    if to_chip {
                        bounds.recent.get()
                    } else {
                        bounds.chip.get()
                    }
                },
                |flight| {
                    flight
                        .sample(
                            if to_chip {
                                bounds.recent.get()
                            } else {
                                bounds.chip.get()
                            },
                            now,
                        )
                        .0
                },
            );
        self.flights.retain(|flight| flight.person != i);
        if !disabled_motion(cx) && from.size.width > px(0.) {
            self.flights.push(Flight {
                person: i,
                from,
                to_chip,
                started: now,
            });
        }
        self.previous_count = self.selected.len();
        self.count_clock.restart(cx);
        if to_chip {
            self.selected.push(i);
        } else {
            self.selected.retain(|&selected| selected != i);
            if !self.scope.contains_focused(w, cx) {
                self.send_focus.focus(w, cx);
            }
        }
        let name = &self.people[i].name;
        self.status = match (tr(cx, "en", "zh"), to_chip) {
            ("zh", true) => format!("已添加 {name}"),
            ("zh", false) => format!("已移除 {name}"),
            (_, true) => format!("{name} added"),
            (_, false) => format!("{name} removed"),
        }
        .into();
        self.send_error = None;
        cx.notify();
    }

    fn set_access_open(&mut self, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        self.access_open = open && !self.access.is_empty();
        if self.access_open {
            self.access_active = self.access_index;
            self.options_focus[self.access_active].focus(w, cx);
        } else {
            self.access_focus.focus(w, cx);
        }
        cx.notify();
    }

    fn select_access(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if !self.sending {
            self.access_index = i;
            if self.copy_feedback != Feedback::Pending {
                self.copy_feedback = Feedback::Idle;
            }
            self.set_access_open(false, w, cx);
        }
    }

    fn keyboard(&mut self, event: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        match key {
            "escape" if self.access_open => self.set_access_open(false, w, cx),
            "escape" => self.close(true, w, cx),
            "enter" | "space"
                if self.access_open && self.options_focus[self.access_active].is_focused(w) =>
            {
                self.select_access(self.access_active, w, cx);
            }
            "up" | "left" | "down" | "right" | "home" | "end" if self.access_open => {
                let count = self.access.len();
                self.access_active = match key {
                    "home" => 0,
                    "end" => count - 1,
                    "up" | "left" => (self.access_active + count - 1) % count,
                    _ => (self.access_active + 1) % count,
                };
                self.options_focus[self.access_active].focus(w, cx);
                cx.notify();
            }
            "tab" if self.sheet_on_phones && w.viewport_size().width <= px(520.) => {
                if event.keystroke.modifiers.shift {
                    w.focus_prev(cx);
                } else {
                    w.focus_next(cx);
                }
                if !self.scope.contains_focused(w, cx) {
                    if self.surface == Surface::Sent {
                        self.done_focus.focus(w, cx);
                    } else {
                        self.copy_focus.focus(w, cx);
                    }
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    fn move_drag(&mut self, y: Pixels, cx: &mut Context<Self>) {
        if let Some(start) = self.drag_start {
            self.geometry[2].reset(f32::from((y - start).max(px(0.))));
            cx.notify();
        }
    }

    fn end_drag(&mut self, cancelled: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.drag_start.take().is_none() {
            return;
        }
        if !cancelled && self.geometry[2].value(cx) >= 72. {
            self.close(true, w, cx);
        }
        self.geometry[2].duration(0., 0.27, cx);
        cx.notify();
    }

    fn drag_capture(&self, cx: &Context<Self>) -> impl IntoElement {
        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
            if e.pressed_button == Some(MouseButton::Left) {
                s.move_drag(e.position.y, cx);
            }
        });
        let released = cx.listener(|s, e: &MouseUpEvent, w, cx| {
            if e.button == MouseButton::Left && s.drag_start.is_some() {
                s.move_drag(e.position.y, cx);
                s.end_drag(false, w, cx);
                cx.stop_propagation();
            }
        });
        canvas(
            |_, _, _| (),
            move |_, _, w, _| {
                w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                    if phase.capture() {
                        moved(e, w, cx);
                    }
                });
                w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                    if phase.capture() {
                        released(e, w, cx);
                    }
                });
            },
        )
        .absolute()
        .size(px(0.))
    }

    fn announce(&self, cx: &App) -> SharedString {
        if let Some(sent) = &self.sent
            && self.surface == Surface::Sent
        {
            return sent_title(sent, cx);
        }
        if let Some(error) = &self.send_error {
            return error.clone();
        }
        self.status.clone()
    }
}

fn status_label(state: Feedback, idle: SharedString, done: SharedString, cx: &App) -> SharedString {
    match state {
        Feedback::Idle => idle,
        Feedback::Pending => tr(cx, "Working…", "处理中…").into(),
        Feedback::Done => done,
        Feedback::Failed => tr(cx, "Failed", "失败").into(),
    }
}

fn sent_title(sent: &ShareSubmission, cx: &App) -> SharedString {
    let first = &sent.people[0].name;
    let remaining = sent.people.len().saturating_sub(1);
    match (tr(cx, "en", "zh"), remaining) {
        ("zh", 0) => format!("已发送给 {first}").into(),
        ("zh", n) => format!("已发送给 {first} 等 {} 人", n + 1).into(),
        (_, 0) => format!("Sent to {first}").into(),
        (_, n) => format!("Sent to {first} + {n}").into(),
    }
}

fn record_bounds(saved: Rc<Cell<Bounds<Pixels>>>) -> impl IntoElement {
    canvas(move |bounds, _, _| saved.set(bounds), |_, _, _, _| ())
        .absolute()
        .inset_0()
}

fn avatar(person: &SharePerson, size: Pixels, paint: Paint) -> Div {
    let avatar = div()
        .relative()
        .size(size)
        .flex_none()
        .rounded_full()
        .bg(paint.tray);
    if let Some(source) = &person.avatar {
        avatar.child(
            img(source.clone())
                .size_full()
                .rounded_full()
                .object_fit(ObjectFit::Cover),
        )
    } else if let Some([from, to]) = person.accent {
        avatar.bg(linear_gradient(
            135.,
            linear_color_stop(rgb(from), 0.),
            linear_color_stop(rgb(to), 1.),
        ))
    } else {
        avatar
            .flex()
            .items_center()
            .justify_center()
            .text_size(size * 0.38)
            .child(
                person
                    .name
                    .split_whitespace()
                    .filter_map(|part| part.chars().next())
                    .take(2)
                    .collect::<String>(),
            )
    }
}

fn share_icon(ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, w, _| {
            let at = |x, y| bounds.origin + point(px(x), px(y));
            let mut path = PathBuilder::stroke(px(1.3));
            path.move_to(at(4., 6.));
            path.line_to(at(2.5, 6.));
            path.line_to(at(2.5, 14.));
            path.line_to(at(13.5, 14.));
            path.line_to(at(13.5, 6.));
            path.line_to(at(12., 6.));
            path.move_to(at(8., 10.));
            path.line_to(at(8., 2.));
            path.move_to(at(4.9, 5.1));
            path.line_to(at(8., 2.));
            path.line_to(at(11.1, 5.1));
            if let Ok(path) = path.build() {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(16.))
    .flex_none()
}

fn text_line(text: impl Into<SharedString>, size: f32, color: Hsla) -> Div {
    div()
        .min_w_0()
        .truncate()
        .text_size(px(size))
        .text_color(color)
        .child(text.into())
}

impl ShareSheet {
    fn access_row(&self, i: usize, selected: bool, paint: Paint) -> Div {
        let level = &self.access[i];
        div()
            .w_full()
            .h(px(52.))
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(12.))
            .when_some(level.icon.as_ref(), |e, glyph| {
                e.child(glyph.element(16., paint.secondary))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(text_line(level.label.clone(), 14., paint.ink).line_height(px(20.)))
                    .child(
                        text_line(level.description.clone(), 12., paint.secondary)
                            .line_height(px(16.)),
                    ),
            )
            .when(selected, |e| e.child(icon(Glyph::Check, 14., paint.ink)))
    }

    fn access_section(&self, r: Pixels, paint: Paint, cx: &Context<Self>) -> Div {
        let Some(current) = self.access.get(self.access_index) else {
            return div();
        };
        let toggle = div()
            .id("share-access-trigger")
            .debug_selector(|| "share-access-trigger".into())
            .role(Role::Button)
            .aria_label(format!("{} {}", current.label, current.description))
            .aria_expanded(self.access_open)
            .track_focus(&self.access_focus)
            .tab_stop(true)
            .relative()
            .rounded(r)
            .cursor_pointer()
            .hover(|s| s.bg(paint.hover))
            .focus_visible(|s| s.bg(paint.hover))
            .child(self.access_row(self.access_index, false, paint).pr(px(34.)))
            .child(div().absolute().right(px(12.)).top(px(19.)).child(icon(
                if self.access_open {
                    Glyph::Up
                } else {
                    Glyph::Down
                },
                14.,
                paint.secondary,
            )))
            .on_click(cx.listener(|s, _, w, cx| {
                if !s.sending {
                    s.set_access_open(!s.access_open, w, cx);
                }
            }));
        let mut list = div().child(toggle);
        if self.access_open {
            let mut options = div()
                .id("share-access-list")
                .role(Role::ListBox)
                .aria_label(tr(cx, "Who can open the link", "谁可以打开链接"))
                .px(px(6.))
                .pb(px(4.))
                .flex()
                .flex_col()
                .gap(px(3.));
            for (i, level) in self.access.iter().enumerate() {
                let chosen = i == self.access_index;
                options = options.child(
                    self.access_row(i, chosen, paint)
                        .id(("share-access-option", i))
                        .debug_selector(move || format!("share-access-option-{i}"))
                        .role(Role::ListBoxOption)
                        .aria_label(format!("{} {}", level.label, level.description))
                        .aria_selected(chosen)
                        .track_focus(&self.options_focus[i])
                        .tab_stop(self.access_active == i)
                        .rounded(r)
                        .when(chosen, |e| e.bg(paint.tray))
                        .cursor_pointer()
                        .hover(|s| s.bg(paint.hover))
                        .focus_visible(|s| s.bg(paint.hover))
                        .on_click(cx.listener(move |s, _, w, cx| s.select_access(i, w, cx))),
                );
            }
            list = list.child(options);
        }
        list
    }

    fn pending(&self, w: &Window, cx: &App, ink: Hsla) -> impl IntoElement {
        if w.is_visible() {
            w.request_animation_frame();
        }
        let period = if disabled_motion(cx) { 2.2 } else { 0.84 };
        spinner((self.pending_clock.age(cx) / period).fract(), 14., ink)
    }

    fn link_row(&self, paint: Paint, w: &Window, cx: &Context<Self>) -> Div {
        let label = tr(cx, "Copy", "复制");
        let word = status_label(
            self.copy_feedback,
            label.into(),
            tr(cx, "Copied", "已复制").into(),
            cx,
        );
        let link = self
            .link
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        div()
            .h(px(48.))
            .flex()
            .items_center()
            .gap(px(9.))
            .pl(px(12.))
            .pr(px(6.))
            .child(icon(Glyph::Link, 16., paint.secondary))
            .child(text_line(link.to_owned(), 14., paint.secondary).flex_1())
            .child(
                face(
                    "share-copy",
                    tr(cx, "Copy link", "复制链接"),
                    34.,
                    17.,
                    Tone::Surface,
                    cx,
                )
                .debug_selector(|| "share-copy".into())
                .track_focus(&self.copy_focus)
                .px(px(10.))
                .gap(px(6.))
                .border_1()
                .border_color(paint.line)
                .hover(|s| s.bg(paint.hover))
                .when_else(
                    self.copy_feedback == Feedback::Pending,
                    |e| e.child(self.pending(w, cx, paint.ink)),
                    |e| {
                        e.child(icon(
                            if self.copy_feedback == Feedback::Done {
                                Glyph::Check
                            } else {
                                Glyph::Copy
                            },
                            14.,
                            paint.ink,
                        ))
                    },
                )
                .child(word)
                .on_click(cx.listener(Self::click_copy)),
            )
    }

    fn click_copy(&mut self, _: &ClickEvent, w: &mut Window, cx: &mut Context<Self>) {
        self.copy(w, cx);
    }

    fn channel_row(&self, paint: Paint, w: &Window, cx: &Context<Self>) -> Div {
        let mut row = div().mt(px(12.)).flex().gap(px(6.));
        for (i, channel) in self.channels.iter().enumerate() {
            let state = self.channel_feedback[i];
            let label = status_label(
                state,
                channel.label.clone(),
                channel
                    .done_label
                    .clone()
                    .unwrap_or_else(|| tr(cx, "Done", "完成").into()),
                cx,
            );
            row = row.child(
                face(
                    ("share-channel", i),
                    label.clone(),
                    40.,
                    20.,
                    Tone::Surface,
                    cx,
                )
                .debug_selector(move || format!("share-channel-{i}"))
                .flex_1()
                .min_w_0()
                .gap(px(6.))
                .border_1()
                .border_color(paint.line)
                .hover(|s| s.bg(paint.hover))
                .when_some(
                    match state {
                        Feedback::Done => Some(ShareSheetIcon::from(Glyph::Check)),
                        Feedback::Pending => None,
                        _ => channel.icon.clone(),
                    },
                    |e, glyph| e.child(glyph.element(15., paint.secondary)),
                )
                .when(state == Feedback::Pending, |e| {
                    e.child(self.pending(w, cx, paint.secondary))
                })
                .child(text_line(label, 14., paint.ink))
                .on_click(cx.listener(move |s, _, w, cx| s.choose_channel(i, w, cx))),
            );
        }
        row
    }

    fn chips(&self, paint: Paint, cx: &Context<Self>) -> Stateful<Div> {
        let mut chips = div()
            .id("share-chips")
            .role(Role::Group)
            .aria_label(tr(cx, "Send to", "发送给"))
            .h(px(32.))
            .flex()
            .items_center()
            .gap(px(6.))
            .overflow_x_scroll()
            .when(self.selected.is_empty(), |e| {
                e.child(text_line(
                    tr(cx, "Pick people below", "从下方选择收件人"),
                    14.,
                    paint.faint,
                ))
            });
        for &i in &self.selected {
            let person = &self.people[i];
            let flying = self.flights.iter().any(|f| f.person == i && f.to_chip);
            let remove = if tr(cx, "en", "zh") == "zh" {
                format!("移除 {}", person.name)
            } else {
                format!("Remove {}", person.name)
            };
            chips = chips.child(
                div()
                    .h(px(32.))
                    .flex_none()
                    .px(px(4.))
                    .rounded_full()
                    .bg(paint.panel.blend(paint.ink.opacity(0.08)))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(
                        avatar(person, px(24.), paint)
                            .child(record_bounds(self.people_bounds[i].chip.clone()))
                            .opacity(if flying { 0. } else { 1. }),
                    )
                    .child(text_line(person.given_name(), 13., paint.ink))
                    .child(
                        face(("share-remove", i), remove, 24., 12., Tone::Clear, cx)
                            .w(px(24.))
                            .hover(|s| s.bg(paint.hover))
                            .child(icon(Glyph::Close, 12., paint.secondary))
                            .on_click(cx.listener(move |s, _, w, cx| {
                                s.toggle_person(i, w, cx);
                                s.send_focus.focus(w, cx);
                            })),
                    ),
            );
        }
        chips
    }

    fn people_row(&self, r: Pixels, paint: Paint, cx: &Context<Self>) -> Stateful<Div> {
        let mut row = div()
            .id("share-recent")
            .role(Role::Group)
            .aria_label(tr(cx, "Recent people", "最近联系人"))
            .h(px(77.))
            .flex()
            .gap(px(4.))
            .overflow_x_scroll();
        for (i, person) in self.people.iter().enumerate() {
            let selected = self.selected.contains(&i);
            row = row.child(
                div()
                    .id(("share-person", i))
                    .debug_selector(move || format!("share-person-{i}"))
                    .role(Role::Button)
                    .aria_label(person.name.clone())
                    .aria_toggled(selected.into())
                    .tab_stop(true)
                    .h_full()
                    .flex_1()
                    .min_w(px(47.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(5.))
                    .rounded(r)
                    .hover(|s| s.bg(paint.hover))
                    .focus_visible(|s| s.bg(paint.hover))
                    .cursor_pointer()
                    .child(
                        div()
                            .relative()
                            .size(px(46.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .border_1()
                            .border_color(if selected {
                                paint.secondary
                            } else {
                                transparent_black()
                            })
                            .child(
                                avatar(person, px(40.), paint)
                                    .child(record_bounds(self.people_bounds[i].recent.clone())),
                            )
                            .when(selected, |e| {
                                e.child(
                                    div()
                                        .absolute()
                                        .right(px(-1.))
                                        .bottom(px(-2.))
                                        .size(px(18.))
                                        .rounded_full()
                                        .border_2()
                                        .border_color(paint.panel)
                                        .bg(paint.ink)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(icon(Glyph::Check, 11., paint.panel)),
                                )
                            }),
                    )
                    .child(text_line(person.given_name(), 12., paint.secondary).max_w_full())
                    .on_click(cx.listener(move |s, _, w, cx| s.toggle_person(i, w, cx))),
            );
        }
        row
    }

    fn send_word(&self, w: &Window, cx: &App) -> (SharedString, Div) {
        let n = self.selected.len();
        let paint = Paint::get(cx);
        let contrast = if n == 0 { paint.faint } else { paint.inverse };
        if self.sending || n == 0 {
            let word = if self.sending {
                tr(cx, "Sending", "发送中")
            } else {
                tr(cx, "Send", "发送")
            };
            return (word.into(), text_line(word, 14., contrast));
        }
        let chinese = tr(cx, "en", "zh") == "zh";
        let prefix = if chinese { "发送给 " } else { "Send to " };
        let suffix = if chinese {
            " 人"
        } else if n == 1 {
            " person"
        } else {
            " people"
        };
        let current: SharedString = n.to_string().into();
        let previous: SharedString = self.previous_count.to_string().into();
        let progress = ease(self.count_clock.time(0.22, w, cx) / 0.22);
        let direction = if n > self.previous_count { 1. } else { -1. };
        let digit_width = measure(&current, 14., w).max(measure(&previous, 14., w));
        let digit = |value: SharedString, offset: f32, opacity: f32| {
            text_line(value, 14., contrast)
                .absolute()
                .w_full()
                .top(px(offset))
                .line_height(px(20.))
                .text_center()
                .opacity(opacity)
        };
        (
            format!("{prefix}{n}{suffix}").into(),
            div()
                .flex()
                .items_center()
                .child(text_line(prefix, 14., contrast).whitespace_nowrap())
                .child(
                    div()
                        .relative()
                        .w(px(digit_width))
                        .h(px(20.))
                        .overflow_hidden()
                        .when(progress < 1. && self.previous_count > 0, |e| {
                            e.child(digit(previous, -direction * progress * 20., 1. - progress))
                        })
                        .child(digit(current, direction * (1. - progress) * 20., progress)),
                )
                .child(text_line(suffix, 14., contrast).whitespace_nowrap()),
        )
    }

    fn panel(&self, phone: bool, w: &Window, cx: &Context<Self>) -> Div {
        let paint = Paint::get(cx);
        let radius = surface_radius(cx);
        let title: SharedString = format!("{} “{}”", tr(cx, "Share", "分享"), self.title).into();
        let [close, send] = [tr(cx, "Close", "关闭"), tr(cx, "Send to", "发送给")];
        let mut panel = div()
            .w_full()
            .p(px(8.))
            .child(
                div()
                    .h(px(36.))
                    .pl(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(text_line(title, 16., paint.ink).font_weight(FontWeight::MEDIUM))
                    .child(
                        face("share-close", close, 32., 16., Tone::Clear, cx)
                            .w(px(32.))
                            .hover(|s| s.bg(paint.hover))
                            .child(icon(Glyph::Close, 14., paint.secondary))
                            .on_click(cx.listener(|s, _, w, cx| s.close(true, w, cx))),
                    ),
            )
            .child(
                div()
                    .mt(px(12.))
                    .rounded(radius)
                    .bg(paint.panel.blend(paint.ink.opacity(0.025)))
                    .child(self.link_row(paint, w, cx))
                    .child(self.access_section(radius, paint, cx)),
            );
        if !self.channels.is_empty() {
            panel = panel.child(self.channel_row(paint, w, cx));
        }
        let shake = self.shake.time(0.34, w, cx) / 0.34;
        let offset = (shake * 4. * std::f32::consts::TAU).sin() * (1. - shake).powi(2) * 8.;
        let (label, word) = self.send_word(w, cx);
        let mut recipients = div()
            .mt(px(16.))
            .border_t_1()
            .border_color(paint.line)
            .pt(px(15.))
            .child(
                div()
                    .relative()
                    .left(px(offset))
                    .px(px(8.))
                    .child(text_line(send, 12., paint.faint).line_height(px(16.)))
                    .child(div().mt(px(4.)).child(self.chips(paint, cx))),
            )
            .when(!self.people.is_empty(), |e| {
                e.child(div().mt(px(16.)).child(self.people_row(radius, paint, cx)))
            });
        if let Some(error) = &self.send_error {
            recipients = recipients.child(
                div()
                    .px(px(8.))
                    .mt(px(8.))
                    .h(px(16.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .child(icon(Glyph::Alert, 13., paint.danger))
                    .child(text_line(error.clone(), 12., paint.danger)),
            );
        }
        panel.child(
            recipients.child(
                face(
                    "share-send",
                    label,
                    if phone { 48. } else { 44. },
                    if phone { 24. } else { 22. },
                    if self.selected.is_empty() {
                        Tone::Clear
                    } else {
                        Tone::Primary
                    },
                    cx,
                )
                .debug_selector(|| "share-send".into())
                .track_focus(&self.send_focus)
                .w_full()
                .mt(px(12.))
                .when(self.selected.is_empty(), |e| {
                    e.bg(paint.panel.blend(paint.ink.opacity(0.08)))
                })
                .when(self.sending, |e| {
                    e.child(self.pending(w, cx, paint.inverse))
                })
                .child(word)
                .on_click(cx.listener(|s, _, w, cx| s.send(w, cx))),
            ),
        )
    }

    fn confirmation(&self, paint: Paint, cx: &Context<Self>) -> Div {
        let Some(result) = &self.sent else {
            return div();
        };
        let details = self
            .access
            .iter()
            .find(|level| level.value == result.access)
            .map(|level| {
                if level.description.is_empty() {
                    level.label.clone()
                } else {
                    format!("{} · {}", level.label, level.description).into()
                }
            });
        div()
            .w_full()
            .p(px(16.))
            .flex()
            .flex_col()
            .items_center()
            .child(
                avatar(&result.people[0], px(44.), paint).child(
                    div()
                        .absolute()
                        .right(px(-7.))
                        .bottom(px(-6.))
                        .size(px(25.))
                        .rounded_full()
                        .border_2()
                        .border_color(paint.panel)
                        .bg(paint.success)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(Glyph::Check, 14., paint.panel)),
                ),
            )
            .child(
                text_line(sent_title(result, cx), 16., paint.ink)
                    .mt(px(14.))
                    .line_height(px(22.)),
            )
            .child(
                text_line(details.unwrap_or_default(), 12., paint.secondary)
                    .mt(px(4.))
                    .line_height(px(16.)),
            )
            .child(
                div()
                    .w_full()
                    .mt(px(18.))
                    .flex()
                    .gap(px(8.))
                    .child(
                        face(
                            "share-undo",
                            tr(cx, "Undo", "撤销"),
                            40.,
                            20.,
                            Tone::Surface,
                            cx,
                        )
                        .debug_selector(|| "share-undo".into())
                        .flex_1()
                        .border_1()
                        .border_color(paint.line)
                        .hover(|s| s.bg(paint.hover))
                        .child(tr(cx, "Undo", "撤销"))
                        .on_click(cx.listener(|s, _, w, cx| {
                            s.change_surface(Surface::Panel, cx);
                            s.copy_focus.focus(w, cx);
                        })),
                    )
                    .child(
                        face(
                            "share-done",
                            tr(cx, "Done", "完成"),
                            40.,
                            20.,
                            Tone::Primary,
                            cx,
                        )
                        .debug_selector(|| "share-done".into())
                        .track_focus(&self.done_focus)
                        .flex_1()
                        .child(tr(cx, "Done", "完成"))
                        .on_click(cx.listener(|s, _, w, cx| s.close(true, w, cx))),
                    ),
            )
    }

    fn panel_height(&self, phone: bool) -> f32 {
        let mut height = 449.;
        if phone {
            height += 18.;
        }
        if self.access.is_empty() {
            height -= 52.;
        }
        if self.channels.is_empty() {
            height -= 52.;
        }
        if self.people.is_empty() {
            height -= 93.;
        }
        if self.access_open {
            height += self.access.len() as f32 * 55. + 1.;
        }
        if self.send_error.is_some() {
            height += 24.;
        }
        height
    }
}

impl Render for ShareSheet {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = w.viewport_size();
        let phone = self.sheet_on_phones && viewport.width <= px(520.);
        let paint = Paint::get(cx);
        let radius = surface_radius(cx);
        let title = self
            .label
            .clone()
            .unwrap_or_else(|| tr(cx, "Share", "分享").into());
        let trigger_width = (measure(&title, 14., w) + 55.)
            .ceil()
            .min((f32::from(viewport.width) - 16.).max(1.));
        let available = (f32::from(viewport.width) - if phone { 16. } else { 32. }).max(1.);
        let panel_width = self.width.min(available);
        let sent_width = SENT_WIDTH.min(available);
        let available_height = (f32::from(viewport.height) - 16.).max(1.);
        let panel_height = self.panel_height(phone).min(available_height);
        let sent_height = (if phone { 206. } else { 192_f32 }).min(available_height);
        let dimensions = |surface| match surface {
            Surface::Trigger => (trigger_width, TRIGGER_HEIGHT),
            Surface::Panel => (panel_width, panel_height),
            Surface::Sent => (sent_width, sent_height),
        };
        let (target_w, target_h) = dimensions(self.surface);
        let open = self.surface != Surface::Trigger;
        let target_corner = if open {
            f32::from(radius)
        } else {
            TRIGGER_HEIGHT / 2.
        };
        for (index, target) in [(0, target_w), (1, target_h), (3, target_corner)] {
            let spring = &mut self.geometry[index];
            if self.initialized {
                spring.duration(target, 0.36, cx);
            } else {
                spring.reset(target);
            }
        }
        self.initialized = true;
        let mut width = self.geometry[0].get(w, cx);
        let mut height = self.geometry[1].get(w, cx);
        let drag = self.geometry[2].get(w, cx);
        let corner = self.geometry[3].get(w, cx);
        let age = self.face_clock.age(cx);
        if age < FACE_SECONDS {
            w.request_animation_frame();
        }
        let mix = ease(age / FACE_SECONDS);
        let reduced = disabled_motion(cx);
        let phone_reveal = if phone {
            self.phone_open
                .duration(if open { 1. } else { 0. }, 0.36, cx);
            if !open || self.leaving == Surface::Trigger {
                (width, height) = dimensions(if open { self.surface } else { self.leaving });
            }
            self.phone_open.get(w, cx).clamp(0., 1.)
        } else {
            self.phone_open.reset(if open { 1. } else { 0. });
            0.
        };
        let animating_closed = !open
            && if phone {
                phone_reveal > 0.001
            } else {
                (width - trigger_width).abs() > 0.05 || age < FACE_SECONDS
            };
        let now = cx.background_executor().now();
        self.flights.retain(|flight| {
            now.saturating_duration_since(flight.started).as_secs_f32() < FLIGHT_SECONDS
        });
        let trigger = face(
            "share-trigger",
            title.clone(),
            TRIGGER_HEIGHT,
            TRIGGER_HEIGHT / 2.,
            Tone::Primary,
            cx,
        )
        .debug_selector(|| "share-trigger".into())
        .track_focus(&self.trigger_focus)
        .tab_stop(!open)
        .aria_expanded(open)
        .w(px(trigger_width))
        .gap(px(8.))
        .opacity(if open && !phone {
            0.
        } else if animating_closed && !phone {
            mix
        } else {
            1.
        })
        .child(share_icon(paint.inverse))
        .child(text_line(title, 14., paint.inverse))
        .on_click(cx.listener(|s, _, w, cx| s.open(w, cx)));
        let root = div()
            .relative()
            .w(px(trigger_width))
            .h(px(TRIGGER_HEIGHT))
            .flex_none();
        let root = match &self.style {
            Some(style) => style(root),
            None => root,
        };
        let mut root = root
            .id(self.id.clone())
            .child(self.trigger_bounds.measure())
            .child(trigger);
        if !open && !animating_closed {
            return root;
        }

        let (left, top) = place(
            self.trigger_bounds.get(),
            viewport,
            size(px(width), px(height)),
            self.align,
            phone,
            px(drag
                + if phone {
                    (1. - phone_reveal) * (height + 8.)
                } else {
                    0.
                }),
        );
        let face_for = |surface, opacity: f32, offset: f32| {
            let (content_w, content_h) = dimensions(surface);
            let face = match surface {
                Surface::Trigger => div(),
                Surface::Panel => self.panel(phone, w, cx),
                Surface::Sent => self.confirmation(paint, cx),
            };
            let grabber = div()
                .relative()
                .id("share-grabber")
                .debug_selector(|| "share-grabber".into())
                .w_full()
                .h(px(14.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_move()
                .child(div().w(px(36.)).h(px(4.)).rounded_full().bg(paint.faint))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseDownEvent, _, _| s.drag_start = Some(e.position.y)),
                )
                .child(
                    self.contact
                        .listen(cx, |s, event, w, cx| match event.phase {
                            TouchPhase::Started => s.drag_start = Some(event.position.y),
                            TouchPhase::Moved => s.move_drag(event.position.y, cx),
                            TouchPhase::Ended => s.end_drag(false, w, cx),
                            TouchPhase::Cancelled => s.end_drag(true, w, cx),
                        }),
                );
            let grabber_height = if phone { 14. } else { 0. };
            let scroll = self.panel_scroll.clone();
            let current = self.surface == surface;
            let region = canvas(
                |_, _, _| (),
                move |bounds, _, _, cx| {
                    if let Some(host) = cx.try_global::<MenuScrollRegion>() {
                        let offset = -scroll.offset().y;
                        let end = scroll.max_offset().y;
                        if end > px(0.1) {
                            (host.0)(bounds, offset > px(0.1), offset < end - px(0.1));
                        }
                    }
                },
            )
            .absolute()
            .top(px(grabber_height))
            .bottom_0()
            .left_0()
            .right_0();
            div()
                .id(("share-face", surface as usize))
                .absolute()
                .top(px(offset))
                .left(px(if phone {
                    (width - content_w) * 0.5
                } else {
                    match self.align {
                        ShareSheetAlign::Start => 0.,
                        ShareSheetAlign::Center => (width - content_w) * 0.5,
                        ShareSheetAlign::End => width - content_w,
                    }
                }))
                .w(px(content_w))
                .h(px(content_h))
                .opacity(opacity)
                .when(phone, |e| e.child(grabber))
                .child(
                    div()
                        .id("share-body")
                        .w_full()
                        .h(px((content_h - grabber_height - 2.).max(1.)))
                        .overflow_y_scroll()
                        .when(current, |e| {
                            e.track_scroll(&self.panel_scroll)
                                .on_scroll(cx.listener(|_, _, _, cx| cx.notify()))
                        })
                        .child(face),
                )
                .when(current, |e| e.child(region))
        };
        let coverage = (width / target_w).min(height / target_h);
        let incoming = if self.leaving == self.surface {
            1.
        } else if !phone && self.leaving == Surface::Trigger && !reduced && age < 0.35 {
            mix.min(ease(((coverage - 0.68) / 0.32).clamp(0., 1.)))
        } else {
            mix
        };
        let shell_color = if phone {
            paint.panel
        } else if !open {
            paint.panel.blend(paint.bright.opacity(mix))
        } else if self.leaving == Surface::Trigger {
            paint.bright.blend(paint.panel.opacity(mix))
        } else {
            paint.panel
        };
        let mut shell = div()
            .id("share-shell")
            .debug_selector(|| "share-shell".into())
            .absolute()
            .left(left)
            .top(top)
            .w(px(width))
            .h(px(height))
            .rounded(if phone { radius } else { px(corner) })
            .bg(shell_color)
            .border_1()
            .border_color(paint.line)
            .shadow(vec![
                BoxShadow::new(
                    px(0.),
                    px(14.),
                    black().opacity(if paint.panel.l < 0.5 { 0.28 } else { 0.14 }),
                )
                .blur_radius(px(30.)),
            ])
            .overflow_hidden()
            .when(open, |e| {
                e.track_focus(&self.scope)
                    .tab_stop(false)
                    .tab_group()
                    .role(Role::Dialog)
                    .aria_label(format!("{} {}", tr(cx, "Share", "分享"), self.title))
                    .block_mouse_except_scroll()
                    .on_key_down(cx.listener(Self::keyboard))
                    .on_mouse_down_out(cx.listener(move |s, _, w, cx| s.close(phone, w, cx)))
            })
            .when_some(
                (self.leaving != Surface::Trigger
                    && (age < FACE_SECONDS || phone && animating_closed))
                    .then_some(self.leaving),
                |e, prior| {
                    e.child(face_for(
                        prior,
                        if phone && !open { 1. } else { 1. - mix },
                        if reduced { 0. } else { -5. * mix },
                    ))
                },
            )
            .when(open, |e| {
                e.child(face_for(
                    self.surface,
                    incoming,
                    if reduced { 0. } else { 5. * (1. - incoming) },
                ))
            });
        if !open && !phone {
            shell = shell.opacity(1. - mix);
        }

        let mut layers = div()
            .w(viewport.width)
            .h(viewport.height)
            .relative()
            .when(phone, |e| {
                e.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(black().opacity(0.54 * phone_reveal))
                        .when(open, |e| e.block_mouse_except_scroll()),
                )
            })
            .child(shell)
            .when(phone && open, |e| e.child(self.drag_capture(cx)));
        for flight in &self.flights {
            let saved = &self.people_bounds[flight.person];
            let to = if flight.to_chip {
                saved.chip.get()
            } else {
                saved.recent.get()
            };
            w.request_animation_frame();
            if to.size.width > px(0.) {
                let (at, _) = flight.sample(to, now);
                layers = layers.child(
                    avatar(&self.people[flight.person], at.size.width, paint)
                        .absolute()
                        .left(at.left())
                        .top(at.top()),
                );
            }
        }
        let announcement = self.announce(cx);
        if !announcement.is_empty() {
            root = root.child(
                div()
                    .id("share-status")
                    .role(Role::Status)
                    .aria_label(announcement)
                    .absolute()
                    .size(px(1.))
                    .opacity(0.),
            );
        }
        root.child(
            deferred(anchored().position(point(px(0.), px(0.))).child(layers)).with_priority(4),
        )
    }
}

fn place(
    trigger: Bounds<Pixels>,
    viewport: Size<Pixels>,
    size: Size<Pixels>,
    align: ShareSheetAlign,
    phone: bool,
    drag: Pixels,
) -> (Pixels, Pixels) {
    let center = trigger.center();
    if phone {
        return (
            (viewport.width - size.width) / 2.,
            viewport.height - size.height - px(8.) + drag,
        );
    }
    let x = match align {
        ShareSheetAlign::Start => trigger.left(),
        ShareSheetAlign::End => trigger.right() - size.width,
        ShareSheetAlign::Center => center.x - size.width / 2.,
    };
    (
        x.clamp(px(8.), (viewport.width - size.width - px(8.)).max(px(8.))),
        (center.y - size.height / 2.)
            .clamp(px(8.), (viewport.height - size.height - px(8.)).max(px(8.))),
    )
}

#[cfg(test)]
mod tests {
    use super::{SharePerson, ShareSheet, ShareSheetAccess, ShareSheetAlign, Surface};
    use crate::foundation::Theme;
    use gpui::{
        App, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, prelude::*, px, size,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};

    fn tick(w: &mut VisualTestContext, millis: u64) {
        w.executor().advance_clock(Duration::from_millis(millis));
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }

    fn click(w: &mut VisualTestContext, selector: &'static str) {
        let bounds = w
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("missing {selector}"));
        w.simulate_click(bounds.center(), Modifiers::default());
        tick(w, 0);
    }

    fn sheet(cx: &App) -> ShareSheet {
        ShareSheet::new(
            "Roadmap",
            "https://example.com/docs/roadmap",
            vec![
                SharePerson::new("ana", "Ana Ruiz"),
                SharePerson::new("sam", "Sam Lee"),
            ],
            vec![
                ShareSheetAccess::new("private", "Invited only"),
                ShareSheetAccess::new("team", "Workspace"),
                ShareSheetAccess::new("public", "Anyone"),
            ],
            cx,
        )
        .with_default_access("team")
        .with_align(ShareSheetAlign::Center)
        .with_style(|e| e.mt(px(200.)).ml(px(240.)))
    }

    #[test]
    fn access_keys_wrap_and_close_in_order() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let (view, w) = app.add_window_view(|_, cx| sheet(cx));
        click(w, "share-trigger");
        w.update(|w, cx| assert!(view.read(cx).copy_focus.is_focused(w)));
        click(w, "share-access-trigger");
        w.simulate_keystrokes("down down");
        tick(w, 0);
        w.update(|w, cx| assert!(view.read(cx).options_focus[0].is_focused(w)));
        w.simulate_keystrokes("enter");
        tick(w, 0);
        w.update(|_, cx| assert_eq!(view.read(cx).access_value().as_ref(), "private"));
        click(w, "share-access-trigger");
        w.simulate_keystrokes("escape");
        tick(w, 0);
        w.update(|w, cx| {
            let share = view.read(cx);
            assert_eq!(share.surface, Surface::Panel);
            assert!(share.access_focus.is_focused(w));
        });
        w.simulate_keystrokes("escape");
        tick(w, 0);
        w.update(|w, cx| {
            let share = view.read(cx);
            assert_eq!(share.surface, Surface::Trigger);
            assert!(share.trigger_focus.is_focused(w));
        });
    }

    #[test]
    fn send_failure_preserves_the_snapshot_and_retry_fires_once() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let requests = Rc::new(RefCell::new(Vec::new()));
        let record = requests.clone();
        let (view, w) = app.add_window_view(|_, cx| {
            sheet(cx).on_send(move |request, _, cx| {
                let failed = record.borrow().is_empty();
                record.borrow_mut().push(request);
                let time = cx.background_executor().timer(Duration::from_millis(80));
                cx.spawn(async move |_| {
                    time.await;
                    if failed {
                        Err("Delivery failed".into())
                    } else {
                        Ok(())
                    }
                })
            })
        });
        click(w, "share-trigger");
        click(w, "share-send");
        assert!(requests.borrow().is_empty());
        click(w, "share-person-0");
        click(w, "share-send");
        click(w, "share-send");
        assert_eq!(requests.borrow().len(), 1);
        tick(w, 90);
        w.update(|_, cx| {
            let share = view.read(cx);
            assert_eq!(share.send_error.as_deref(), Some("Delivery failed"));
            assert_eq!(share.selected, vec![0]);
            assert_eq!(share.surface, Surface::Panel);
        });
        click(w, "share-send");
        tick(w, 90);
        let submitted = requests.borrow();
        assert_eq!(submitted.len(), 2);
        assert_eq!(submitted[1].people[0].id.as_ref(), "ana");
        assert_eq!(submitted[1].access.as_ref(), "team");
        w.update(|w, cx| assert!(view.read(cx).done_focus.is_focused(w)));
        click(w, "share-undo");
        w.update(|_, cx| assert_eq!(view.read(cx).selected, vec![0]));
    }

    #[test]
    fn copy_reports_only_the_access_that_was_written() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let copied = Rc::new(RefCell::new(Vec::new()));
        let observed = copied.clone();
        let (_, w) = app.add_window_view(|_, cx| {
            sheet(cx).on_copy(move |access, _, _| observed.borrow_mut().push(access))
        });
        click(w, "share-trigger");
        click(w, "share-copy");
        w.update(|_, cx| {
            let clipboard = cx
                .read_from_clipboard()
                .and_then(|contents| contents.text());
            assert_eq!(
                clipboard.as_deref(),
                Some("https://example.com/docs/roadmap")
            );
        });
        assert_eq!(copied.borrow().as_slice(), &["team"]);
    }

    #[test]
    fn mobile_drag_uses_current_offset_and_restores_focus() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| sheet(cx));
        w.simulate_resize(size(px(320.), px(700.)));
        tick(w, 0);
        click(w, "share-trigger");
        tick(w, 800);
        let bounds = w.debug_bounds("share-shell").unwrap();
        assert_eq!(bounds.size.width, px(304.));
        assert!((bounds.bottom() - px(692.)).abs() < px(0.1));
        for (distance, expected) in [(60., Surface::Panel), (103., Surface::Trigger)] {
            let start = w.debug_bounds("share-grabber").unwrap().center();
            let end = start + point(px(0.), px(distance));
            w.simulate_mouse_move(start, None, Modifiers::default());
            w.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
            w.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
            w.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
            tick(w, 800);
            w.update(|w, cx| {
                let share = view.read(cx);
                assert_eq!(share.surface, expected);
                if expected == Surface::Trigger {
                    assert!(share.trigger_focus.is_focused(w));
                }
            });
        }
    }
}
