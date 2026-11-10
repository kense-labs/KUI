//! Provider health display. The host owns polling, visibility and networking.
use crate::primitives::brand_mark::brand_mark;
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderHealth {
    Operational,
    Degraded,
    Outage,
    Unknown,
}

#[derive(Clone)]
pub struct ProviderDay {
    pub date: SharedString,
    pub health: ProviderHealth,
    pub uptime: Option<f32>,
    pub note: SharedString,
}
impl ProviderDay {
    pub fn new(date: impl Into<SharedString>, health: ProviderHealth, uptime: Option<f32>) -> Self {
        Self {
            date: date.into(),
            health,
            uptime: uptime.filter(|n| n.is_finite()).map(|n| n.clamp(0., 100.)),
            note: "".into(),
        }
    }
    pub fn note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = note.into();
        self
    }
}
#[derive(Clone)]
pub struct ProviderIncidentUpdate {
    pub stage: SharedString,
    pub time: SharedString,
    pub message: SharedString,
}
impl ProviderIncidentUpdate {
    pub fn new(
        stage: impl Into<SharedString>,
        time: impl Into<SharedString>,
        message: impl Into<SharedString>,
    ) -> Self {
        Self {
            stage: stage.into(),
            time: time.into(),
            message: message.into(),
        }
    }
}
#[derive(Clone)]
pub struct ProviderIncident {
    pub title: SharedString,
    pub updates: Vec<ProviderIncidentUpdate>,
    pub link: Option<SharedString>,
}
impl ProviderIncident {
    pub fn new(title: impl Into<SharedString>, updates: Vec<ProviderIncidentUpdate>) -> Self {
        Self {
            title: title.into(),
            updates,
            link: None,
        }
    }
    pub fn link(mut self, link: impl Into<SharedString>) -> Self {
        self.link = Some(link.into());
        self
    }
}
#[derive(Clone)]
pub struct ProviderStatusItem {
    pub id: SharedString,
    pub name: SharedString,
    pub detail: SharedString,
    pub health: ProviderHealth,
    pub latency_ms: Option<f32>,
    pub uptime: Option<f32>,
    pub days: Vec<ProviderDay>,
    pub incident: Option<ProviderIncident>,
}
impl ProviderStatusItem {
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        health: ProviderHealth,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            detail: "".into(),
            health,
            latency_ms: None,
            uptime: None,
            days: Vec::new(),
            incident: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderStatusEvent {
    Refresh,
    Expanded(Option<SharedString>),
    OpenIncident(SharedString),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HealthCounts {
    total: usize,
    outage: usize,
    degraded: usize,
    unknown: usize,
}
fn health_counts(providers: &[ProviderStatusItem]) -> HealthCounts {
    let mut counts = HealthCounts {
        total: providers.len(),
        outage: 0,
        degraded: 0,
        unknown: 0,
    };
    for provider in providers {
        match provider.health {
            ProviderHealth::Outage => counts.outage += 1,
            ProviderHealth::Degraded => counts.degraded += 1,
            ProviderHealth::Unknown => counts.unknown += 1,
            ProviderHealth::Operational => {}
        }
    }
    counts
}
fn is_current_request(active: Option<u64>, request: u64) -> bool {
    active == Some(request)
}

pub struct ProviderStatus {
    title: SharedString,
    footnote: SharedString,
    providers: Vec<ProviderStatusItem>,
    expanded: Option<SharedString>,
    selected_day: Option<(SharedString, usize)>,
    day_focus: HashMap<SharedString, FocusHandle>,
    checking: bool,
    request_serial: u64,
    active_request: Option<u64>,
    refresh_enabled: bool,
    remaining_seconds: Option<u32>,
    error: Option<SharedString>,
    width: Rc<Cell<Pixels>>,
    panel_height: Rc<Cell<Pixels>>,
    animated_id: Option<SharedString>,
    expansion: Slide,
}
impl EventEmitter<ProviderStatusEvent> for ProviderStatus {}
impl ProviderStatus {
    pub fn new(title: impl Into<SharedString>, providers: Vec<ProviderStatusItem>) -> Self {
        Self {
            title: title.into(),
            footnote: "".into(),
            providers,
            expanded: None,
            selected_day: None,
            day_focus: HashMap::new(),
            checking: false,
            request_serial: 0,
            active_request: None,
            refresh_enabled: true,
            remaining_seconds: None,
            error: None,
            width: Rc::new(Cell::new(px(0.))),
            panel_height: Rc::new(Cell::new(px(80.))),
            animated_id: None,
            expansion: Slide::new(0.),
        }
    }
    pub fn footnote(mut self, footnote: impl Into<SharedString>) -> Self {
        self.footnote = footnote.into();
        self
    }
    /// Start with a provider's incident expanded when it exists in the snapshot.
    pub fn with_open_incident(mut self, id: impl Into<SharedString>) -> Self {
        let id = id.into();
        if self
            .providers
            .iter()
            .any(|p| p.id == id && p.incident.is_some())
        {
            self.expanded = Some(id.clone());
            self.animated_id = Some(id);
            self.expansion.reset(1.);
        }
        self
    }
    pub fn refresh_enabled(mut self, enabled: bool) -> Self {
        self.refresh_enabled = enabled;
        self
    }
    pub fn set_providers(&mut self, providers: Vec<ProviderStatusItem>, cx: &mut Context<Self>) {
        self.day_focus
            .retain(|id, _| providers.iter().any(|provider| &provider.id == id));
        self.providers = providers;
        self.checking = false;
        self.active_request = None;
        self.error = None;
        cx.notify();
    }
    /// Capture the ID when handling Refresh; use the guarded result methods for async work.
    pub fn current_request(&self) -> Option<u64> {
        self.active_request
    }
    /// Ignore results from earlier or already settled checks.
    pub fn set_providers_for(
        &mut self,
        request: u64,
        providers: Vec<ProviderStatusItem>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !is_current_request(self.active_request, request) {
            return false;
        }
        self.set_providers(providers, cx);
        true
    }
    /// Ignore errors from earlier or already settled checks.
    pub fn set_error_for(
        &mut self,
        request: u64,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !is_current_request(self.active_request, request) {
            return false;
        }
        self.set_error(error, cx);
        true
    }
    /// The host drives the countdown and stops it while hidden.
    pub fn set_remaining_seconds(&mut self, seconds: Option<u32>, cx: &mut Context<Self>) {
        self.remaining_seconds = seconds;
        cx.notify();
    }
    /// On failure retain the last provider data and allow another refresh.
    pub fn set_error(&mut self, error: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.error = Some(error.into());
        self.checking = false;
        self.active_request = None;
        cx.notify();
    }
    fn refresh(&mut self, cx: &mut Context<Self>) {
        if !self.checking {
            self.checking = true;
            self.request_serial = self.request_serial.wrapping_add(1);
            self.active_request = Some(self.request_serial);
            self.error = None;
            cx.emit(ProviderStatusEvent::Refresh);
            cx.notify();
        }
    }
    fn toggle(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if self.expanded.as_ref() == Some(&id) {
            self.expanded = None;
            self.expansion.duration(0., 0.24, cx);
        } else {
            if self.animated_id.as_ref() != Some(&id) {
                self.expansion.reset(0.);
                self.panel_height.set(px(80.));
            }
            self.animated_id = Some(id.clone());
            self.expanded = Some(id);
            self.expansion.duration(1., 0.24, cx);
        }
        cx.emit(ProviderStatusEvent::Expanded(self.expanded.clone()));
        cx.notify();
    }
}
fn status_label(health: ProviderHealth, cx: &App) -> &'static str {
    match health {
        ProviderHealth::Operational => tr(cx, "Operational", "正常"),
        ProviderHealth::Degraded => tr(cx, "Degraded", "降级"),
        ProviderHealth::Outage => tr(cx, "Outage", "故障"),
        ProviderHealth::Unknown => tr(cx, "No data", "暂无数据"),
    }
}
fn status_color(health: ProviderHealth, p: Paint) -> Hsla {
    match health {
        ProviderHealth::Operational => p.success,
        ProviderHealth::Degraded => rgb(0xe2a93b).into(),
        ProviderHealth::Outage => p.danger,
        ProviderHealth::Unknown => p.faint,
    }
}
fn latency(value: Option<f32>) -> String {
    match value.filter(|n| n.is_finite() && *n >= 0.) {
        Some(n) if n >= 1000. => format!("{:.2} s", n / 1000.),
        Some(n) => format!("{n:.0} ms"),
        None => "—".into(),
    }
}
impl Render for ProviderStatus {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let width = self.width.get();
        let compact = if width > px(0.) {
            width < px(560.)
        } else {
            w.viewport_size().width < px(560.)
        };
        let expansion = self.expansion.get(w, cx).clamp(0., 1.);
        let measured_width = self.width.clone();
        let counts = health_counts(&self.providers);
        let summary = if counts.total == 0 || counts.unknown == counts.total {
            tr(cx, "No provider data", "暂无服务状态数据").to_string()
        } else if counts.outage == 0 && counts.degraded == 0 && counts.unknown == 0 {
            tr(cx, "All systems operational", "所有服务正常").to_string()
        } else {
            let mut parts = Vec::new();
            if counts.outage > 0 {
                parts.push(format!("{} {}", counts.outage, tr(cx, "outage", "故障")));
            }
            if counts.degraded > 0 {
                parts.push(format!(
                    "{} {}",
                    counts.degraded,
                    tr(cx, "degraded", "降级")
                ));
            }
            if counts.unknown > 0 {
                parts.push(format!(
                    "{} {}",
                    counts.unknown,
                    tr(cx, "unknown", "状态未知")
                ));
            }
            let operational = counts.total - counts.outage - counts.degraded - counts.unknown;
            if operational > 0 {
                parts.push(format!("{} {}", operational, tr(cx, "operational", "正常")));
            }
            parts.join(" · ")
        };
        let mut header = div().flex().items_center().justify_between().gap_3().child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(self.title.clone()),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.secondary)
                        .child(summary),
                ),
        );
        if self.refresh_enabled {
            let text = if self.checking {
                tr(cx, "Checking…", "检查中…").to_string()
            } else if self.error.is_some() {
                tr(cx, "Retry", "重试").to_string()
            } else if let Some(s) = self.remaining_seconds {
                format!("{} {s}s", tr(cx, "Refresh in", "刷新倒计时"))
            } else {
                tr(cx, "Refresh", "刷新").to_string()
            };
            let mut tools = div().flex().items_center().gap(px(8.));
            if let Some(seconds) = self.remaining_seconds.filter(|_| !self.checking) {
                tools = tools.child(
                    div()
                        .h(px(32.))
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.faint)
                        .child(format!("{} {seconds}s", tr(cx, "Refresh in", "刷新倒计时"))),
                );
            }
            tools = tools.child(
                face("provider-refresh", text.clone(), 32., 16., Tone::Clear, cx)
                    .size(px(32.))
                    .rounded_full()
                    .p_0()
                    .text_color(p.secondary)
                    .hover(|s| s.bg(p.tray).text_color(p.ink))
                    .child(icon(Glyph::Redo, 16., p.secondary))
                    .on_click(cx.listener(|s, _, _, cx| s.refresh(cx))),
            );
            header = header.child(tools);
        }
        let mut root = div()
            .id("provider-status")
            .role(Role::Group)
            .aria_label(self.title.clone())
            .w_full()
            .relative()
            .child(
                canvas(
                    move |bounds, w, _| {
                        if (measured_width.get() - bounds.size.width).abs() > px(0.5) {
                            measured_width.set(bounds.size.width);
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .max_w(px(760.))
            .p(px(8.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_size(px(14.))
            .flex()
            .flex_col()
            .gap_3()
            .child(header);
        if let Some(error) = &self.error {
            root = root.child(
                div()
                    .id("provider-status-error")
                    .role(Role::Alert)
                    .text_size(px(12.))
                    .text_color(p.danger)
                    .child(error.clone()),
            );
        }
        for (i, provider) in self.providers.iter().enumerate() {
            let id = provider.id.clone();
            let selected = self
                .selected_day
                .as_ref()
                .and_then(|(key, day)| (key == &id).then_some(*day));
            let visible_days: Vec<_> = provider.days.iter().rev().take(30).rev().cloned().collect();
            let spoken_value = selected
                .and_then(|index| visible_days.get(index))
                .map(|day| {
                    let uptime = day
                        .uptime
                        .map_or_else(|| "—".to_string(), |value| format!("{value:.2}%"));
                    format!(
                        "{} · {} · {} {}",
                        day.date,
                        status_label(day.health, cx),
                        uptime,
                        day.note
                    )
                })
                .unwrap_or_else(|| tr(cx, "No day selected", "尚未选择日期").to_string());
            let focus = self
                .day_focus
                .entry(id.clone())
                .or_insert_with(|| cx.focus_handle().tab_stop(true))
                .clone();
            let touch_bounds = TriggerBounds::default();
            let mut bar = div()
                .id(("provider-days", i))
                .role(Role::Slider)
                .aria_label(format!(
                    "{} · {}",
                    provider.name,
                    tr(
                        cx,
                        "30 day uptime, use arrow keys",
                        "30 日可用性，使用方向键"
                    )
                ))
                .aria_value(spoken_value)
                .track_focus(&focus)
                .tab_index(0)
                .relative()
                .flex()
                .items_center()
                .gap(px(2.))
                .h(px(if compact { 20. } else { 24. }))
                .child(touch_bounds.measure());
            let count = visible_days.len();
            let bar_id = id.clone();
            bar = bar.on_key_down(cx.listener(move |s, e: &KeyDownEvent, _, cx| {
                if count == 0 {
                    return;
                }
                let index = s
                    .selected_day
                    .as_ref()
                    .and_then(|(key, n)| (key == &bar_id).then_some(*n))
                    .unwrap_or(count - 1)
                    .min(count - 1);
                let next = match e.keystroke.key.as_str() {
                    "left" => Some(index.saturating_sub(1)),
                    "right" => Some((index + 1).min(count - 1)),
                    "home" => Some(0),
                    "end" => Some(count - 1),
                    "escape" => None,
                    _ => return,
                };
                s.selected_day = next.map(|n| (bar_id.clone(), n));
                cx.stop_propagation();
                cx.notify();
            }));
            for (day_index, day) in visible_days.iter().enumerate() {
                let key = id.clone();
                let col = status_color(day.health, p);
                bar = bar.child(
                    div()
                        .id(("provider-day", i * 30 + day_index))
                        .flex_1()
                        .min_w(px(2.))
                        .h(px(if selected == Some(day_index) {
                            18.
                        } else {
                            14.
                        }))
                        .rounded(px(2.))
                        .bg(col)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |s, _, w, cx| {
                                s.selected_day = Some((key.clone(), day_index));
                                if let Some(handle) = s.day_focus.get(&key) {
                                    handle.focus(w, cx);
                                }
                                cx.notify();
                            }),
                        ),
                );
            }
            let touch_is_horizontal = Rc::new(Cell::new(false));
            let touch_origin = Rc::new(Cell::new(point(px(0.), px(0.))));
            let touch_active = touch_is_horizontal.clone();
            let start_point = touch_origin.clone();
            let bar_bounds = touch_bounds.clone();
            let touch_id = id.clone();
            let update_touch = cx.listener(move |s, e: &TouchDragEvent, _, cx| {
                if e.phase == TouchPhase::Started {
                    start_point.set(e.position);
                    return;
                }
                let delta = e.position - start_point.get();
                if e.phase == TouchPhase::Moved
                    && !touch_active.get()
                    && delta.x.abs() > px(4.)
                    && delta.x.abs() > delta.y.abs()
                {
                    touch_active.set(true);
                }
                if touch_active.get() && count > 0 {
                    let bounds = bar_bounds.get();
                    let fraction = (f32::from(e.position.x - bounds.left())
                        / f32::from(bounds.size.width).max(1.))
                    .clamp(0., 0.9999);
                    s.selected_day = Some((touch_id.clone(), (fraction * count as f32) as usize));
                    cx.notify();
                }
                if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                    touch_active.set(false);
                }
            });
            let active_touch = touch_is_horizontal.clone();
            let started_here = Rc::new(Cell::new(false));
            bar = bar.child(
                canvas(
                    |bounds, w, _| w.insert_hitbox(bounds, HitboxBehavior::Normal),
                    move |_, hit, w, _| {
                        w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                            if !phase.bubble() {
                                return;
                            }
                            if e.phase == TouchPhase::Started {
                                started_here.set(hit.is_hovered_at(e.position, w));
                            }
                            if !started_here.get() {
                                return;
                            }
                            let engaged = active_touch.get();
                            update_touch(e, w, cx);
                            if engaged || active_touch.get() {
                                w.prevent_default();
                                cx.stop_propagation();
                            }
                            if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                                started_here.set(false);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            );
            let mut bar = Some(bar);
            let percentage = provider
                .uptime
                .filter(|n| n.is_finite())
                .map_or_else(|| "—".to_string(), |n| format!("{:.2}%", n.clamp(0., 100.)));
            let info = div()
                .flex()
                .flex_col()
                .min_w(px(100.))
                .flex_1()
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .line_height(px(20.))
                        .text_color(p.ink)
                        .child(provider.name.clone()),
                )
                .when(!provider.detail.is_empty(), |info| {
                    info.child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.faint)
                            .child(provider.detail.clone()),
                    )
                });
            let status = || {
                div()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(status_color(provider.health, p))
                    .child(format!("● {}", status_label(provider.health, cx)))
            };
            let latency_readout = || {
                div()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.secondary)
                    .child(latency(provider.latency_ms))
            };
            let has_incident = provider.incident.is_some();
            let is_open = self.expanded.as_ref() == Some(&id);
            let mut main = div()
                .id(("provider-incident", i))
                .flex()
                .items_center()
                .gap_3()
                .flex_1()
                .when_some(brand_mark(&provider.name, 20., cx), |row, mark| {
                    row.child(mark)
                })
                .child(info)
                .when(!compact, |row| row.child(status()));
            if has_incident {
                let toggle_id = id.clone();
                main = main
                    .role(Role::Button)
                    .aria_label(format!(
                        "{} · {} · {}",
                        provider.name,
                        status_label(provider.health, cx),
                        if is_open {
                            tr(cx, "Collapse incident", "收起事件")
                        } else {
                            tr(cx, "Expand incident", "展开事件")
                        }
                    ))
                    .aria_expanded(is_open)
                    .tab_index(0)
                    .cursor_pointer()
                    .child(icon(
                        if is_open { Glyph::Up } else { Glyph::Down },
                        14.,
                        p.faint,
                    ))
                    .on_click(cx.listener(move |s, _, _, cx| s.toggle(toggle_id.clone(), cx)));
            }
            let mut row = div()
                .flex()
                .items_center()
                .gap_3()
                .min_h(px(if compact { 0. } else { 56. }))
                .px(px(12.))
                .when(has_incident, |row| {
                    row.hover(|s| s.bg(p.panel.blend(p.tray.opacity(0.55))))
                })
                .child(main);
            if !compact {
                row = row
                    .child(latency_readout().w(px(64.)))
                    .child(div().w(px(150.)).child(bar.take().unwrap()))
                    .child(
                        div()
                            .w(px(52.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(percentage.clone()),
                    );
            }
            let mut group = div()
                .border_t_1()
                .border_color(p.line)
                .pt_3()
                .flex()
                .flex_col()
                .gap_2()
                .child(row);
            if compact {
                group = group
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .pl(px(44.))
                            .pr(px(12.))
                            .child(status())
                            .child(latency_readout()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .pl(px(44.))
                            .pr(px(12.))
                            .child(div().flex_1().child(bar.take().unwrap()))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(percentage),
                            ),
                    );
            }
            if let Some(day) = selected.and_then(|n| visible_days.get(n)) {
                let uptime = day
                    .uptime
                    .map_or_else(|| "—".to_string(), |n| format!("{n:.2}%"));
                group = group.child(div().text_size(px(12.)).text_color(p.secondary).child(
                    format!(
                        "{} · {} · {}  {}",
                        day.date,
                        status_label(day.health, cx),
                        uptime,
                        day.note
                    ),
                ));
            }
            if self.animated_id.as_ref() == Some(&id)
                && (is_open || expansion > 0.001)
                && let Some(incident) = &provider.incident
            {
                let mut panel = div()
                    .pl(px(40.))
                    .pr(px(12.))
                    .pb(px(16.))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .text_size(px(12.))
                    .child(div().text_color(p.ink).child(incident.title.clone()));
                for update in &incident.updates {
                    panel = panel.child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(8.))
                            .pl(px(20.))
                            .relative()
                            .child(
                                div()
                                    .absolute()
                                    .top(px(7.))
                                    .left(px(0.))
                                    .size(px(7.))
                                    .rounded_full()
                                    .bg(p.line),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(3.))
                                    .child(
                                        div()
                                            .text_color(p.ink)
                                            .child(format!("{} · {}", update.stage, update.time)),
                                    )
                                    .child(
                                        div().text_color(p.secondary).child(update.message.clone()),
                                    ),
                            ),
                    );
                }
                if incident.link.is_some() {
                    let incident_id = id.clone();
                    panel = panel.child(
                        face(
                            ("provider-link", i),
                            tr(cx, "Open status page", "打开状态页"),
                            28.,
                            8.,
                            Tone::Clear,
                            cx,
                        )
                        .child(tr(cx, "Open status page", "打开状态页"))
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.emit(ProviderStatusEvent::OpenIncident(incident_id.clone()))
                        })),
                    );
                }
                let measure_height = self.panel_height.clone();
                panel = panel.relative().child(
                    canvas(
                        move |bounds, w, _| {
                            if (measure_height.get() - bounds.size.height).abs() > px(0.5) {
                                measure_height.set(bounds.size.height);
                                w.request_animation_frame();
                            }
                        },
                        |_, _, _, _| (),
                    )
                    .absolute()
                    .inset_0(),
                );
                group = group.child(
                    div()
                        .h(self.panel_height.get() * expansion)
                        .overflow_hidden()
                        .child(panel),
                );
            }
            root = root.child(group);
        }
        if !self.footnote.is_empty() {
            root = root.child(
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.secondary)
                    .child(self.footnote.clone()),
            );
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use super::{ProviderHealth, ProviderStatusItem, health_counts, is_current_request};
    #[test]
    fn unknown_and_empty_are_not_all_operational() {
        let empty = health_counts(&[]);
        assert_eq!(empty.total, 0);
        let items = vec![
            ProviderStatusItem::new("a", "A", ProviderHealth::Unknown),
            ProviderStatusItem::new("b", "B", ProviderHealth::Operational),
        ];
        let counts = health_counts(&items);
        assert_eq!((counts.total, counts.unknown), (2, 1));
    }
    #[test]
    fn stale_and_settled_refreshes_are_rejected() {
        assert!(is_current_request(Some(2), 2));
        assert!(!is_current_request(Some(2), 1));
        assert!(!is_current_request(None, 2));
    }
}
