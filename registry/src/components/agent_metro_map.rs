//! Multi-agent transit metro map with 45-degree rails, interchange hubs, and train chips.
//!
//! Draws a multi-agent workflow as a color-coded transit map: straight rails with crisp 45° bends,
//! twinkling agent train chips, solid trails behind, faint rails ahead, and amber approval stations.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_card_shadow,
        pro_check_draw, pro_ease, pro_eid, pro_empty_state, pro_icon_btn, pro_mix_color,
        pro_popover_presence, pro_presence, pro_skeleton_bar, pro_spring_f32, pro_swap_label,
        pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        family::Slide,
        pro_icons::{ProIcon, pro_app_artwork},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

/// Placement of a station's text label relative to its ring dot on [`AgentMetroMap`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetroLabelSide {
    /// Place station label underneath the station ring.
    #[default]
    Below,
    /// Place station label above the station ring.
    Above,
}

/// Optional gate or failure override on a [`MetroStation`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetroStationStatus {
    /// Normal station whose passed/active/ahead state follows train positions.
    #[default]
    Normal,
    /// Station waiting on human sign-off; glows amber and offers `Approve` / `Reject`.
    Waiting,
    /// Station blocked by an error; turns red.
    Failed,
}

/// One stop on the [`AgentMetroMap`] grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetroStation {
    /// Stable station identifier referenced by [`MetroLine::stations`].
    pub id: SharedString,
    /// Short one- or two-word station title.
    pub label: SharedString,
    /// Zero-based horizontal column index on the transit grid.
    pub col: usize,
    /// Zero-based vertical row index on the transit grid.
    pub row: usize,
    /// Label side (`Below` or `Above`) chosen so rails never cut across text.
    pub label_side: MetroLabelSide,
    /// Optional human-approval or error status.
    pub status: MetroStationStatus,
    /// Longer description shown inside the station inspector dialog.
    pub detail: SharedString,
    /// Connected tool names shown inside the station inspector dialog.
    pub tools: Vec<SharedString>,
    /// Optional step duration string (for example `"1m 40s"`).
    pub duration: Option<SharedString>,
}

impl MetroStation {
    /// Construct a [`MetroStation`] at `(col, row)` on the transit grid.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        col: usize,
        row: usize,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            col,
            row,
            label_side: MetroLabelSide::Below,
            status: MetroStationStatus::Normal,
            detail: SharedString::default(),
            tools: Vec::new(),
            duration: None,
        }
    }

    /// Position the station name above the station ring.
    pub fn label_above(mut self) -> Self {
        self.label_side = MetroLabelSide::Above;
        self
    }

    /// Set [`MetroStationStatus`] (`Waiting` or `Failed`).
    pub fn with_status(mut self, status: MetroStationStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the detailed description shown when inspecting this station.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Attach tool names used at this station.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.tools = tools.into_iter().map(Into::into).collect();
        self
    }

    /// Set elapsed or recorded duration at this station.
    pub fn with_duration(mut self, duration: impl Into<SharedString>) -> Self {
        self.duration = Some(duration.into());
        self
    }
}

/// Lifecycle state of a [`MetroLine`] train.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MetroLineStatus {
    /// Not started yet; entire line is faint with no active train chip.
    Idle,
    /// Actively riding its line; solid behind the train, faint ahead with a leading light pulse.
    #[default]
    Running,
    /// Arrived at its terminus; entire line is solid full color.
    Done,
}

impl MetroLineStatus {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Idle => tr(cx, "Idle", "待发车"),
            Self::Running => tr(cx, "Running", "运行中"),
            Self::Done => tr(cx, "Done", "已抵达"),
        }
    }
}

/// Colored transit line ridden by one agent across [`AgentMetroMap`].
#[derive(Clone, Debug, PartialEq)]
pub struct MetroLine {
    /// Stable line identifier.
    pub id: SharedString,
    /// Agent name shown in the legend and route card.
    pub name: SharedString,
    /// Model name shown alongside its provider mark.
    pub model: SharedString,
    /// Ordered station ids visited by this line.
    pub stations: Vec<SharedString>,
    /// Current station id where this agent's train sits (`None` defaults to the first stop).
    pub at: Option<SharedString>,
    /// Current run status of this line.
    pub status: MetroLineStatus,
    /// Optional custom rail color (`None` uses the transit line palette).
    pub color: Option<Hsla>,
}

impl MetroLine {
    /// Construct a [`MetroLine`] visiting `stations` in order.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        model: impl Into<SharedString>,
        stations: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: model.into(),
            stations: stations.into_iter().map(Into::into).collect(),
            at: None,
            status: MetroLineStatus::Running,
            color: None,
        }
    }

    /// Set the station id where this line's train currently sits.
    pub fn with_at(mut self, station_id: impl Into<SharedString>) -> Self {
        self.at = Some(station_id.into());
        self
    }

    /// Set [`MetroLineStatus`] (`Idle`, `Running`, or `Done`).
    pub fn with_status(mut self, status: MetroLineStatus) -> Self {
        self.status = status;
        self
    }

    /// Override the line's rail color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// Typed events emitted by [`AgentMetroMap`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentMetroMapEvent {
    /// User clicked a station to open its inspector card.
    StationSelected {
        /// Station identifier.
        station_id: SharedString,
    },
    /// User clicked a transit line in the top legend to open its route card.
    LineSelected {
        /// Line identifier.
        line_id: SharedString,
    },
    /// Operator approved or rejected a waiting station.
    StationApproved {
        /// Station identifier.
        station_id: SharedString,
        /// `true` when approved, `false` when rejected.
        approved: bool,
    },
}

impl EventEmitter<AgentMetroMapEvent> for AgentMetroMap {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MetroModalTarget {
    Station(SharedString),
    Line(SharedString),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MetroHoverFocus {
    Station(SharedString),
    Line(SharedString),
}

/// Multi-agent workflow rendered as a transit metro map with 45-degree rails and train chips.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use kui::components::agent_metro_map::AgentMetroMap;
///
/// let metro = cx.new(|cx| AgentMetroMap::new("agent-metro-map", cx));
/// # }
/// ```
pub struct AgentMetroMap {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    stations: Vec<MetroStation>,
    lines: Vec<MetroLine>,
    custom_data: bool,
    train_slides: Vec<Slide>,
    hover_focus: Option<MetroHoverFocus>,
    modal_target: Option<MetroModalTarget>,
    last_modal_target: Option<MetroModalTarget>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentMetroMap {
    /// Create a live [`AgentMetroMap`] initialized with the 6-line, 21-station `"Ship v2.4"` release route.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let (stations, lines) = default_metro_network();
        let train_slides = lines
            .iter()
            .map(|ln| Slide::new(stop_index_for_line(ln) as f32))
            .collect();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: None,
            stations,
            lines,
            custom_data: false,
            train_slides,
            hover_focus: None,
            modal_target: None,
            last_modal_target: None,
            toast: None,
            loading: false,
            empty: false,
            boot: Instant::now(),
            measured_width: Rc::new(Cell::new(780.0)),
        }
    }

    /// Override the map heading (defaults to `"Ship v2.4"` / `"发版 v2.4 地铁图"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replace the grid stations.
    pub fn with_stations(mut self, stations: Vec<MetroStation>) -> Self {
        self.custom_data = true;
        if stations.is_empty() {
            self.stations.clear();
            self.empty = true;
        } else {
            self.stations = stations;
        }
        self
    }

    /// Replace the transit lines.
    pub fn with_lines(mut self, lines: Vec<MetroLine>) -> Self {
        self.custom_data = true;
        self.train_slides = lines
            .iter()
            .map(|ln| Slide::new(stop_index_for_line(ln) as f32))
            .collect();
        if lines.is_empty() {
            self.lines.clear();
            self.empty = true;
        } else {
            self.lines = lines;
        }
        self
    }

    /// Render skeleton placeholders when `loading` is `true`.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state when `empty` is `true`.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Approve or reject an amber `Waiting` station and advance waiting trains.
    pub fn resolve_station(
        &mut self,
        station_id: impl Into<SharedString>,
        approved: bool,
        cx: &mut Context<Self>,
    ) {
        let target = station_id.into();
        let mut st_label = target.clone();
        if let Some(st) = self.stations.iter_mut().find(|s| s.id == target) {
            st_label = st.label.clone();
            st.status = if approved {
                MetroStationStatus::Normal
            } else {
                MetroStationStatus::Failed
            };
        }
        if approved {
            for (li, ln) in self.lines.iter_mut().enumerate() {
                if let Some(pos) = ln.stations.iter().position(|id| id == &target) {
                    let cur = stop_index_for_line(ln);
                    if cur + 1 >= pos {
                        let next_idx = (pos + 1).min(ln.stations.len().saturating_sub(1));
                        ln.at = Some(ln.stations[next_idx].clone());
                        if next_idx + 1 >= ln.stations.len() {
                            ln.status = MetroLineStatus::Done;
                        }
                        if let Some(sl) = self.train_slides.get_mut(li) {
                            sl.duration(next_idx as f32, 0.55, cx);
                        }
                    }
                }
            }
        }
        self.modal_target = None;
        let msg = if approved {
            format!("{} {st_label}", tr(cx, "Approved", "已批准站点"))
        } else {
            format!("{} {st_label}", tr(cx, "Rejected", "已拒绝站点"))
        };
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(AgentMetroMapEvent::StationApproved {
            station_id: target,
            approved,
        });
        cx.notify();
    }

    /// Advance active trains by one station stop (stopping respectfully at `Waiting` stations).
    pub fn advance_trains(&mut self, cx: &mut Context<Self>) {
        if self.train_slides.len() != self.lines.len() {
            self.train_slides = self
                .lines
                .iter()
                .map(|ln| Slide::new(stop_index_for_line(ln) as f32))
                .collect();
        }
        let mut any_advanced = false;
        for (li, ln) in self.lines.iter_mut().enumerate() {
            if ln.status == MetroLineStatus::Idle && !ln.stations.is_empty() {
                ln.status = MetroLineStatus::Running;
                ln.at = Some(ln.stations[0].clone());
                self.train_slides[li].duration(0.0, 0.45, cx);
                any_advanced = true;
                continue;
            }
            if ln.status == MetroLineStatus::Running && !ln.stations.is_empty() {
                let cur = stop_index_for_line(ln);
                let cur_id = &ln.stations[cur];
                let blocked_here = self
                    .stations
                    .iter()
                    .find(|s| &s.id == cur_id)
                    .map(|s| s.status == MetroStationStatus::Waiting)
                    .unwrap_or(false);
                if blocked_here {
                    continue;
                }
                if cur + 1 < ln.stations.len() {
                    let next = cur + 1;
                    ln.at = Some(ln.stations[next].clone());
                    let next_waiting = self
                        .stations
                        .iter()
                        .find(|s| s.id == ln.stations[next])
                        .map(|s| s.status == MetroStationStatus::Waiting)
                        .unwrap_or(false);
                    if next + 1 >= ln.stations.len() && !next_waiting {
                        ln.status = MetroLineStatus::Done;
                    }
                    self.train_slides[li].duration(next as f32, 0.55, cx);
                    any_advanced = true;
                }
            }
        }
        if !any_advanced {
            // Reset route for replay
            let (stations, lines) = default_metro_network();
            if !self.custom_data {
                self.stations = stations;
                self.lines = lines;
                for (li, ln) in self.lines.iter().enumerate() {
                    if let Some(sl) = self.train_slides.get_mut(li) {
                        sl.duration(stop_index_for_line(ln) as f32, 0.45, cx);
                    }
                }
            }
        }
        cx.notify();
    }

    /// Toggle skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether skeleton loading is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the metro map is empty.
    pub fn is_empty(&self) -> bool {
        self.empty || self.stations.is_empty() || self.lines.is_empty()
    }

    fn line_color(&self, idx: usize, line: &MetroLine, p: &ProPalette) -> Hsla {
        if let Some(c) = line.color {
            return c;
        }
        let defaults: [Hsla; 6] = [
            p.blue,
            p.purple,
            p.teal,
            p.orange,
            p.pink,
            if p.dark {
                rgb(0x818cf8).into()
            } else {
                rgb(0x5e6ad2).into()
            },
        ];
        defaults[idx % defaults.len()]
    }

    fn render_station_dialog(
        &self,
        station: &MetroStation,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sid = station.id.clone();
        let disp_label = if self.custom_data {
            station.label.clone()
        } else {
            tr_station_label(&station.id, &station.label, cx).into()
        };
        let serving: Vec<(usize, &MetroLine)> = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.stations.contains(&sid))
            .collect();
        let is_interchange = serving.len() >= 2;

        let close_btn = pro_icon_btn(
            pro_eid(&self.id, "m-close-st"),
            tr(cx, "Close station card", "关闭站点详情"),
            ProIcon::Close,
            24.,
            false,
            cx,
            |this, w, cx| {
                this.modal_target = None;
                this.focus.focus(w, cx);
                cx.notify();
            },
        );

        let (st_text, st_bg, st_fg, st_passed_chk) = match station.status {
            MetroStationStatus::Waiting => (
                tr(cx, "Needs approval", "等待批准"),
                p.amber_soft,
                p.amber_ink,
                false,
            ),
            MetroStationStatus::Failed => (tr(cx, "Failed", "失败"), p.red_soft, p.red_ink, false),
            MetroStationStatus::Normal => {
                let any_here = serving.iter().any(|(_, l)| {
                    l.status == MetroLineStatus::Running && l.at.as_ref() == Some(&sid)
                });
                let any_passed = serving.iter().any(|(_, l)| station_passed_by_line(l, &sid));
                if any_here {
                    (
                        tr(cx, "Train at station", "智能体在此站"),
                        p.accent_soft,
                        p.blue,
                        false,
                    )
                } else if any_passed {
                    (tr(cx, "Passed", "已通过"), p.green_soft, p.green_ink, true)
                } else {
                    (tr(cx, "Upcoming", "前方站点"), p.chip, p.muted, false)
                }
            }
        };

        let mut lines_list = div()
            .w_full()
            .pt(px(8.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(5.));
        for (li, ln) in &serving {
            let lcol = self.line_color(*li, ln, p);
            let pkey = infer_provider_key(&ln.model);
            lines_list = lines_list.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .text_size(px(12.))
                    .child(div().w(px(10.)).h(px(3.)).rounded(px(1.5)).bg(lcol))
                    .children(pro_app_artwork(pkey, 12., cx))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(ln.name.clone()),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(ln.model.clone()),
                    ),
            );
        }

        let mut dialog = div()
            .id(pro_eid(&self.id, "m-station-dlg"))
            .role(Role::Dialog)
            .aria_label(disp_label.clone())
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(320.))
            .max_w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(9.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(disp_label),
                            )
                            .when(is_interchange, |d| {
                                d.child(
                                    div()
                                        .h(px(18.))
                                        .px(px(6.))
                                        .rounded(px(4.))
                                        .bg(p.chip)
                                        .text_size(px(10.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.muted)
                                        .flex()
                                        .items_center()
                                        .child(tr(cx, "Interchange", "换乘交汇站")),
                                )
                            }),
                    )
                    .child(
                        div()
                            .h(px(19.))
                            .px(px(6.))
                            .rounded(px(4.))
                            .bg(st_bg)
                            .text_color(st_fg)
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .gap(px(3.5))
                            .when(st_passed_chk, |d| {
                                d.child(pro_check_draw(
                                    pro_eid(&self.id, format!("m-stdlg-chk-{sid}")),
                                    true,
                                    12.0,
                                    false,
                                    st_fg,
                                ))
                            })
                            .child(st_text),
                    )
                    .child(close_btn),
            );

        if !station.detail.is_empty() {
            dialog = dialog.child(
                div()
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .text_color(p.text)
                    .child(station.detail.clone()),
            );
        }

        if !station.tools.is_empty() {
            let mut trow = div().flex().flex_wrap().gap(px(5.));
            for t in &station.tools {
                trow = trow.child(
                    div()
                        .h(px(22.))
                        .pl(px(4.))
                        .pr(px(7.))
                        .rounded(px(6.))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(11.5))
                        .text_color(p.text)
                        .child(pro_app_tile(t, 14., cx))
                        .child(t.clone()),
                );
            }
            dialog = dialog.child(trow);
        }

        dialog = dialog.child(lines_list);

        if station.status == MetroStationStatus::Waiting {
            let s_rej = sid.clone();
            let s_app = sid.clone();
            dialog = dialog.child(
                div()
                    .w_full()
                    .pt(px(8.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .justify_end()
                    .gap(px(6.))
                    .child(pro_btn(
                        pro_eid(&self.id, "m-st-reject"),
                        tr(cx, "Reject", "拒绝"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.resolve_station(s_rej.clone(), false, cx);
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "m-st-approve"),
                        tr(cx, "Approve", "批准放行"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.resolve_station(s_app.clone(), true, cx);
                        },
                    )),
            );
        }

        dialog.into_any_element()
    }

    fn render_line_dialog(
        &self,
        line_idx: usize,
        line: &MetroLine,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let lid = line.id.clone();
        let lcol = self.line_color(line_idx, line, p);
        let pkey = infer_provider_key(&line.model);
        let cur_idx = stop_index_for_line(line);

        let close_btn = pro_icon_btn(
            pro_eid(&self.id, "m-close-ln"),
            tr(cx, "Close line card", "关闭线路详情"),
            ProIcon::Close,
            24.,
            false,
            cx,
            |this, w, cx| {
                this.modal_target = None;
                this.focus.focus(w, cx);
                cx.notify();
            },
        );

        let mut stops = div()
            .w_full()
            .pt(px(8.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(6.));
        for (si, st_id) in line.stations.iter().enumerate() {
            let st_label = self
                .stations
                .iter()
                .find(|s| &s.id == st_id)
                .map(|s| {
                    if self.custom_data {
                        s.label.clone()
                    } else {
                        tr_station_label(&s.id, &s.label, cx).into()
                    }
                })
                .unwrap_or_else(|| st_id.clone());
            let passed = match line.status {
                MetroLineStatus::Done => true,
                MetroLineStatus::Idle => false,
                MetroLineStatus::Running => si <= cur_idx,
            };
            let is_here = line.status == MetroLineStatus::Running && si == cur_idx;
            stops = stops.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.))
                    .child(if passed && !is_here {
                        pro_check_draw(
                            pro_eid(&self.id, format!("m-ln-chk-{lid}-{si}")),
                            true,
                            12.0,
                            false,
                            lcol,
                        )
                    } else {
                        div()
                            .size(px(11.))
                            .rounded_full()
                            .bg(p.card)
                            .border_2()
                            .border_color(if passed { lcol } else { lcol.opacity(0.32) })
                            .into_any_element()
                    })
                    .child(
                        div()
                            .flex_1()
                            .font_weight(if is_here {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if passed { p.ink } else { p.muted })
                            .child(st_label),
                    )
                    .when(is_here, |d| {
                        d.child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(lcol)
                                .child(tr(cx, "Here", "当前站")),
                        )
                    }),
            );
        }

        let dialog = div()
            .id(pro_eid(&self.id, "m-line-dlg"))
            .role(Role::Dialog)
            .aria_label(line.name.clone())
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(300.))
            .max_w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(9.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_agent_avatar(
                        metro_avatar_seed(&line.model, &line.name, &line.id),
                        lcol,
                        24.,
                        line.status == MetroLineStatus::Running,
                        0.,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(line.name.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .children(pro_app_artwork(pkey, 11.5, cx))
                                    .child(line.model.clone()),
                            ),
                    )
                    .child(
                        div()
                            .h(px(19.))
                            .px(px(6.))
                            .rounded(px(4.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .child(pro_swap_label(
                                pro_eid(&self.id, format!("m-ln-st-{lid}")),
                                line.status.label(cx),
                                11.,
                                FontWeight::MEDIUM,
                                p.ink,
                            )),
                    )
                    .child(close_btn),
            )
            .child(stops);

        dialog.into_any_element()
    }
}

impl Render for AgentMetroMap {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_ms = self.boot.elapsed().as_secs_f32() * 1000.0;

        let any_running = self
            .lines
            .iter()
            .any(|l| l.status == MetroLineStatus::Running)
            || self
                .stations
                .iter()
                .any(|s| s.status == MetroStationStatus::Waiting);
        if any_running && !reduced {
            w.request_animation_frame();
        }

        let width_cell = self.measured_width.clone();
        let width_sensor = canvas(
            move |bounds, _, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 20.0 {
                    width_cell.set(bw);
                }
            },
            |_, _, _, _| (),
        )
        .absolute()
        .left_0()
        .right_0()
        .top_0()
        .h(px(1.));

        if self.loading {
            let phase = (elapsed_ms / 1200.0).fract();
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(20.))
                .rounded(px(14.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .shadow(pro_card_shadow(p.dark))
                .flex()
                .flex_col()
                .gap(px(18.))
                .child(width_sensor)
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_between()
                        .child(pro_skeleton_bar(120., 20., 6., phase, cx))
                        .child(pro_skeleton_bar(320., 26., 7., phase, cx)),
                )
                .child(pro_skeleton_bar(720., 380., 10., phase, cx).w_full())
                .into_any_element();
        }

        if self.empty || self.stations.is_empty() || self.lines.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(width_sensor)
                .child(pro_empty_state(
                    ProIcon::GitBranch,
                    tr(cx, "No metro routes configured", "暂无地铁运行线路"),
                    tr(
                        cx,
                        "Define transit lines and interchange stations to visualize multi-agent progress.",
                        "配置智能体运行线路与换乘站点后，即可在此直观追踪列车进度与审批节点。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        if self.train_slides.len() != self.lines.len() {
            self.train_slides = self
                .lines
                .iter()
                .map(|ln| Slide::new(stop_index_for_line(ln) as f32))
                .collect();
        }

        let container_w = self.measured_width.get();
        let inner_w = (container_w - 44.0).max(280.0);
        let max_col = self
            .stations
            .iter()
            .map(|s| s.col)
            .max()
            .unwrap_or(1)
            .max(1);
        let max_row = self
            .stations
            .iter()
            .map(|s| s.row)
            .max()
            .unwrap_or(1)
            .max(1);

        let pad_x = if inner_w < 520.0 { 24.0 } else { 38.0 };
        let pad_y = 44.0;
        let col_step = ((inner_w - pad_x * 2.0) / (max_col as f32)).clamp(28.0, 86.0);
        let row_step = (col_step * 1.12).clamp(52.0, 86.0);
        let stage_h = pad_y * 2.0 + row_step * (max_row as f32);

        let mut st_px: HashMap<SharedString, (f32, f32)> =
            HashMap::with_capacity(self.stations.len());
        let mut st_line_counts: HashMap<SharedString, usize> = HashMap::new();
        for st in &self.stations {
            let x = pad_x + (st.col as f32) * col_step;
            let y = pad_y + (st.row as f32) * row_step;
            st_px.insert(st.id.clone(), (x, y));
        }
        for ln in &self.lines {
            for sid in &ln.stations {
                *st_line_counts.entry(sid.clone()).or_insert(0) += 1;
            }
        }

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Ship v2.4", "发版 v2.4 路线图").into());

        if let Some(mt) = &self.modal_target {
            self.last_modal_target = Some(mt.clone());
        }

        // Determine lit line ids based on hover or open modal
        let active_focus = self
            .hover_focus
            .clone()
            .or_else(|| match &self.modal_target {
                Some(MetroModalTarget::Line(id)) => Some(MetroHoverFocus::Line(id.clone())),
                Some(MetroModalTarget::Station(id)) => Some(MetroHoverFocus::Station(id.clone())),
                None => None,
            });
        let lit_line_ids: Option<Vec<SharedString>> = active_focus.as_ref().map(|foc| match foc {
            MetroHoverFocus::Line(lid) => vec![lid.clone()],
            MetroHoverFocus::Station(sid) => self
                .lines
                .iter()
                .filter(|l| l.stations.contains(sid))
                .map(|l| l.id.clone())
                .collect(),
        });

        // Pre-sample smooth spring opacities for lines, legend pills, and stations!
        let line_opacities: Vec<f32> = self
            .lines
            .iter()
            .enumerate()
            .map(|(li, ln)| {
                let dimmed = lit_line_ids
                    .as_ref()
                    .map(|lit| !lit.contains(&ln.id))
                    .unwrap_or(false);
                pro_spring_f32(
                    pro_eid(&self.id, format!("mln-op-{li}")),
                    if dimmed { 0.25 } else { 1.0 },
                    0.22,
                    w,
                    cx,
                )
            })
            .collect();

        let leg_opacities: Vec<f32> = self
            .lines
            .iter()
            .enumerate()
            .map(|(li, ln)| {
                let dimmed = lit_line_ids
                    .as_ref()
                    .map(|lit| !lit.contains(&ln.id))
                    .unwrap_or(false);
                pro_spring_f32(
                    pro_eid(&self.id, format!("mleg-op-{li}")),
                    if dimmed { 0.42 } else { 1.0 },
                    0.20,
                    w,
                    cx,
                )
            })
            .collect();

        let mut st_opacities: HashMap<SharedString, f32> =
            HashMap::with_capacity(self.stations.len());
        for st in &self.stations {
            let st_dimmed = match &active_focus {
                Some(MetroHoverFocus::Line(lid)) => !self
                    .lines
                    .iter()
                    .any(|l| &l.id == lid && l.stations.contains(&st.id)),
                Some(MetroHoverFocus::Station(sid)) => sid != &st.id,
                None => false,
            };
            let op = pro_spring_f32(
                pro_eid(&self.id, format!("mst-op-{}", st.id)),
                if st_dimmed { 0.28 } else { 1.0 },
                0.22,
                w,
                cx,
            );
            st_opacities.insert(st.id.clone(), op);
        }

        // Legend bar
        let mut legend = div().flex().flex_wrap().items_center().gap(px(6.));
        let chip_surface: Hsla = if p.dark {
            rgb(0x27272b).into()
        } else {
            rgb(0xffffff).into()
        };

        for (li, ln) in self.lines.iter().enumerate() {
            let lcol = self.line_color(li, ln, &p);
            let pkey = infer_provider_key(&ln.model);
            let lid = ln.id.clone();
            let lid_hov = ln.id.clone();
            let leg_op = leg_opacities.get(li).copied().unwrap_or(1.0);
            let is_line_sel = self.modal_target == Some(MetroModalTarget::Line(lid.clone()));

            legend = legend.child(
                pro_tactile(
                    pro_eid(&self.id, format!("mleg-{li}")),
                    format!("{}, {}", ln.name, ln.model),
                    7.0,
                    chip_surface,
                    pro_mix_color(chip_surface, lcol, if p.dark { 0.12 } else { 0.06 }),
                    if is_line_sel {
                        lcol.opacity(0.65)
                    } else {
                        p.line
                    },
                    p.line_strong,
                    is_line_sel,
                    div()
                        .h(px(28.))
                        .pl(px(8.))
                        .pr(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(div().w(px(12.)).h(px(3.)).rounded(px(1.5)).bg(lcol))
                        .child(
                            div()
                                .size(px(18.))
                                .rounded(px(5.))
                                .bg(p.logo_tile)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .justify_center()
                                .children(pro_app_artwork(pkey, 11., cx)),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(ln.name.clone()),
                        ),
                    cx,
                    move |this, _, cx| {
                        let target = lid.clone();
                        this.modal_target = Some(MetroModalTarget::Line(target.clone()));
                        cx.emit(AgentMetroMapEvent::LineSelected { line_id: target });
                        cx.notify();
                    },
                )
                .opacity(leg_op)
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    let next = if *hovered {
                        Some(MetroHoverFocus::Line(lid_hov.clone()))
                    } else if this.hover_focus == Some(MetroHoverFocus::Line(lid_hov.clone())) {
                        None
                    } else {
                        this.hover_focus.clone()
                    };
                    if this.hover_focus != next {
                        this.hover_focus = next;
                        cx.notify();
                    }
                })),
            );
        }

        let header = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(title_str.clone()),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "m-step-btn"),
                        tr(cx, "Step trains", "推进列车"),
                        Some(ProIcon::Play),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.advance_trains(cx);
                        },
                    )),
            )
            .child(legend);

        // Precompute per-line 45° polylines and train positions
        struct LineRenderData {
            segments: Vec<Vec<(f32, f32)>>,
            train_pos: f32,
            status: MetroLineStatus,
            color: Hsla,
            vis: f32,
        }

        type MetroTrainMarker = (SharedString, &'static str, Hsla, (f32, f32), f32);

        let mut line_renders: Vec<LineRenderData> = Vec::with_capacity(self.lines.len());
        let mut train_markers: Vec<MetroTrainMarker> = Vec::new();

        for (li, ln) in self.lines.iter().enumerate() {
            let lcol = self.line_color(li, ln, &p);
            let slid = self.train_slides[li].get(w, cx);
            let vis = line_opacities.get(li).copied().unwrap_or(1.0);

            let mut segs: Vec<Vec<(f32, f32)>> = Vec::new();
            for pair in ln.stations.windows(2) {
                if let (Some(&p0), Some(&p1)) = (st_px.get(&pair[0]), st_px.get(&pair[1])) {
                    segs.push(build_metro_45_segment(p0, p1));
                }
            }

            if ln.status != MetroLineStatus::Idle && !ln.stations.is_empty() {
                let max_idx = (ln.stations.len() - 1) as f32;
                let clamped = if ln.status == MetroLineStatus::Done {
                    max_idx
                } else {
                    slid.clamp(0.0, max_idx)
                };
                if ln.status == MetroLineStatus::Running {
                    let pt = sample_train_point(&ln.stations, &segs, &st_px, clamped);
                    train_markers.push((
                        ln.id.clone(),
                        metro_avatar_seed(&ln.model, &ln.name, &ln.id),
                        lcol,
                        pt,
                        vis,
                    ));
                }
                line_renders.push(LineRenderData {
                    segments: segs,
                    train_pos: clamped,
                    status: ln.status,
                    color: lcol,
                    vis,
                });
            } else {
                line_renders.push(LineRenderData {
                    segments: segs,
                    train_pos: 0.0,
                    status: MetroLineStatus::Idle,
                    color: lcol,
                    vis,
                });
            }
        }

        let light_phase = if !reduced {
            Some((elapsed_ms / 1350.0).fract())
        } else {
            None
        };

        let rails_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let ox = f32::from(bounds.origin.x);
                let oy = f32::from(bounds.origin.y);

                for lr in &line_renders {
                    let dim_mul = lr.vis;
                    for (si, pts) in lr.segments.iter().enumerate() {
                        if pts.len() < 2 {
                            continue;
                        }
                        // Base faint rail ahead
                        let ahead_alpha = 0.24 * dim_mul;
                        paint_offset_polyline(
                            pts,
                            ox,
                            oy,
                            2.35,
                            lr.color.opacity(ahead_alpha),
                            window,
                        );

                        // Solid completed stretch behind train
                        let fill_frac = match lr.status {
                            MetroLineStatus::Idle => 0.0,
                            MetroLineStatus::Done => 1.0,
                            MetroLineStatus::Running => (lr.train_pos - si as f32).clamp(0.0, 1.0),
                        };
                        if fill_frac > 0.01 {
                            paint_partial_polyline(
                                pts,
                                ox,
                                oy,
                                fill_frac,
                                2.45,
                                lr.color.opacity(0.95 * dim_mul),
                                window,
                            );
                        }

                        // Traveling light ahead of active train on the immediate upcoming segment
                        if lr.status == MetroLineStatus::Running
                            && dim_mul > 0.5
                            && let Some(ph) = light_phase
                        {
                            let cur_seg = lr.train_pos.floor() as usize;
                            if si == cur_seg {
                                paint_ahead_pulse(
                                    pts,
                                    ox,
                                    oy,
                                    ph,
                                    lr.color.opacity(dim_mul),
                                    window,
                                );
                            }
                        }
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        // Station buttons & labels
        let mut station_elements: Vec<AnyElement> = Vec::with_capacity(self.stations.len());
        let breathe = 0.5 + 0.5 * ((elapsed_ms / 1000.0) * 3.0).sin();

        for st in &self.stations {
            let Some(&(sx, sy)) = st_px.get(&st.id) else {
                continue;
            };
            let count = st_line_counts.get(&st.id).copied().unwrap_or(1);
            let is_hub = count >= 2;
            let dot_side: f32 = if is_hub { 13.0 } else { 9.5 };

            let serving_colors: Vec<Hsla> = self
                .lines
                .iter()
                .enumerate()
                .filter(|(_, l)| l.stations.contains(&st.id))
                .map(|(i, l)| self.line_color(i, l, &p))
                .collect();
            let primary_line_col = serving_colors.first().copied().unwrap_or(p.blue);

            let any_passed_or_here = self.lines.iter().any(|l| station_passed_by_line(l, &st.id));
            let any_here = self
                .lines
                .iter()
                .any(|l| l.status == MetroLineStatus::Running && l.at.as_ref() == Some(&st.id));
            let st_completed =
                st.status == MetroStationStatus::Normal && any_passed_or_here && !any_here;
            let st_op = st_opacities.get(&st.id).copied().unwrap_or(1.0);

            let (ring_bg, ring_border) = match st.status {
                MetroStationStatus::Waiting => (p.amber_soft, p.amber),
                MetroStationStatus::Failed => (p.red_soft, p.red),
                MetroStationStatus::Normal => {
                    if is_hub {
                        (
                            p.card,
                            if any_passed_or_here {
                                p.ink.opacity(0.75)
                            } else {
                                p.muted.opacity(0.45)
                            },
                        )
                    } else {
                        (
                            p.card,
                            if any_passed_or_here {
                                primary_line_col
                            } else {
                                primary_line_col.opacity(0.36)
                            },
                        )
                    }
                }
            };

            let disp_lbl: SharedString = if self.custom_data {
                st.label.clone()
            } else {
                tr_station_label(&st.id, &st.label, cx).into()
            };

            let sid_click = st.id.clone();
            let sid_hov = st.id.clone();
            let is_st_sel = self.modal_target == Some(MetroModalTarget::Station(st.id.clone()));

            let hit_btn = pro_tactile(
                pro_eid(&self.id, format!("mst-{}", st.id)),
                disp_lbl.clone(),
                11.0,
                transparent_black(),
                p.hover.opacity(0.65),
                transparent_black(),
                transparent_black(),
                is_st_sel,
                div()
                    .size(px(22.))
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(st.status == MetroStationStatus::Waiting && !reduced, |d| {
                        let halo = 15.0 + 5.0 * breathe;
                        d.child(
                            div()
                                .absolute()
                                .size(px(halo))
                                .rounded_full()
                                .bg(p.amber.opacity(0.24 * (1.0 - breathe * 0.4))),
                        )
                    })
                    .child(if st_completed {
                        div()
                            .size(px(dot_side.max(12.5)))
                            .rounded_full()
                            .bg(p.card)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_check_draw(
                                pro_eid(&self.id, format!("mst-chk-{}", st.id)),
                                true,
                                12.0,
                                false,
                                primary_line_col,
                            ))
                            .into_any_element()
                    } else {
                        div()
                            .size(px(dot_side))
                            .rounded_full()
                            .bg(ring_bg)
                            .border_2()
                            .border_color(ring_border)
                            .into_any_element()
                    }),
                cx,
                move |this, _, cx| {
                    let target = sid_click.clone();
                    this.modal_target = Some(MetroModalTarget::Station(target.clone()));
                    cx.emit(AgentMetroMapEvent::StationSelected { station_id: target });
                    cx.notify();
                },
            )
            .size(px(22.))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                let next = if *hovered {
                    Some(MetroHoverFocus::Station(sid_hov.clone()))
                } else if this.hover_focus == Some(MetroHoverFocus::Station(sid_hov.clone())) {
                    None
                } else {
                    this.hover_focus.clone()
                };
                if this.hover_focus != next {
                    this.hover_focus = next;
                    cx.notify();
                }
            }));

            let label_patch = div()
                .px(px(3.5))
                .rounded(px(4.))
                .bg(p.card.opacity(0.92))
                .text_size(px(if inner_w < 540.0 { 10.5 } else { 11.5 }))
                .line_height(px(15.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if st.status == MetroStationStatus::Waiting {
                    p.amber_ink
                } else if any_passed_or_here {
                    p.ink
                } else {
                    p.muted
                })
                .truncate()
                .child(disp_lbl);

            let box_w = 96.0_f32;
            let lx = (sx - box_w * 0.5).clamp(0.0, (inner_w - box_w).max(0.0));
            let above = st.label_side == MetroLabelSide::Above;
            let ly = if above { sy - 27.0 } else { sy - 11.0 };

            station_elements.push(
                div()
                    .absolute()
                    .left(px(lx))
                    .top(px(ly.max(2.0)))
                    .w(px(box_w))
                    .opacity(st_op)
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(1.))
                    .map(|d| {
                        if above {
                            d.child(label_patch).child(hit_btn)
                        } else {
                            d.child(hit_btn).child(label_patch)
                        }
                    })
                    .into_any_element(),
            );
        }

        // White agent train chips riding on top of active stations/rails
        let mut train_elements: Vec<AnyElement> = Vec::with_capacity(train_markers.len());
        for (ti, (lid, seed, lcol, (tx, ty), vis)) in train_markers.into_iter().enumerate() {
            let offset_y = (ti % 2) as f32 * 0.0;
            let lid_click = lid.clone();
            train_elements.push(
                pro_tactile(
                    pro_eid(&self.id, format!("mtrain-{lid}")),
                    format!("{lid} train"),
                    7.0,
                    p.card,
                    p.hover,
                    transparent_black(),
                    lcol.opacity(0.55),
                    false,
                    div()
                        .size(px(22.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_agent_avatar(seed, lcol, 22., true, elapsed_ms, cx)),
                    cx,
                    move |this, _, cx| {
                        let target = lid_click.clone();
                        this.modal_target = Some(MetroModalTarget::Line(target.clone()));
                        cx.emit(AgentMetroMapEvent::LineSelected { line_id: target });
                        cx.notify();
                    },
                )
                .size(px(22.))
                .absolute()
                .left(px(tx - 11.0))
                .top(px(ty - 11.0 + offset_y))
                .opacity(vis)
                .shadow(pro_card_shadow(p.dark))
                .into_any_element(),
            );
        }

        let modal_open = self.modal_target.is_some();
        let shown_target = self
            .modal_target
            .clone()
            .or_else(|| self.last_modal_target.clone());
        let open_modal = shown_target.and_then(|target| {
            let (key, card): (SharedString, AnyElement) = match target {
                MetroModalTarget::Station(sid) => {
                    let st = self.stations.iter().find(|s| s.id == sid)?;
                    (
                        format!("st:{sid}").into(),
                        self.render_station_dialog(st, &p, cx),
                    )
                }
                MetroModalTarget::Line(lid) => {
                    let (idx, ln) = self.lines.iter().enumerate().find(|(_, l)| l.id == lid)?;
                    (
                        format!("ln:{lid}").into(),
                        self.render_line_dialog(idx, ln, &p, cx),
                    )
                }
            };
            let scrim_alpha = if p.dark { 0.46 } else { 0.20 };
            let on_scrim_click = cx.listener(|this, _, w, cx| {
                this.modal_target = None;
                this.focus.focus(w, cx);
                cx.notify();
            });
            let card_vtx = pro_view_transition(pro_eid(&self.id, "m-modal-vtx"), key, card);
            let card_pop =
                pro_popover_presence(pro_eid(&self.id, "m-modal-pop"), modal_open, move || {
                    card_vtx
                });
            Some(pro_presence(
                pro_eid(&self.id, "m-modal-pres"),
                modal_open,
                0.18,
                0.14,
                move |amt| {
                    let mut scrim = div()
                        .id(pro_eid(&ElementId::Name("m-scrim".into()), "ov"))
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., scrim_alpha * amt))
                        .flex()
                        .items_center()
                        .justify_center()
                        .p(px(16.));
                    if modal_open {
                        scrim = scrim.on_click(on_scrim_click);
                    }
                    scrim.child(card_pop).into_any_element()
                },
            ))
        });

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(title_str)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "escape" && this.modal_target.is_some() {
                    this.modal_target = None;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
            }))
            .w_full()
            .p(px(20.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow(pro_card_shadow(p.dark))
            .relative()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(width_sensor)
            .child(header)
            .child(
                div()
                    .w_full()
                    .h(px(stage_h))
                    .relative()
                    .child(rails_canvas)
                    .children(station_elements)
                    .children(train_elements),
            )
            .children(open_modal)
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

/// Build a straight or true 45-degree diagonal segment with subtle 6px corner fillets.
fn build_metro_45_segment(p0: (f32, f32), p1: (f32, f32)) -> Vec<(f32, f32)> {
    let dx = p1.0 - p0.0;
    let dy = p1.1 - p0.1;
    let adx = dx.abs();
    let ady = dy.abs();

    if adx < 1.5 || ady < 1.5 {
        let mut pts = Vec::with_capacity(17);
        for i in 0..=16 {
            let t = i as f32 / 16.0;
            pts.push((p0.0 + dx * t, p0.1 + dy * t));
        }
        return pts;
    }

    let diag = adx.min(ady);
    let sx = dx.signum();
    let sy = dy.signum();

    // Split excess straight run symmetrically at both ends so the diagonal is strictly 45°!
    let (k1, k2) = if ady >= adx {
        let stub = (ady - diag) * 0.5;
        ((p0.0, p0.1 + sy * stub), (p1.0, p1.1 - sy * stub))
    } else {
        let stub = (adx - diag) * 0.5;
        ((p0.0 + sx * stub, p0.1), (p1.0 - sx * stub, p1.1))
    };

    let mut pts = Vec::with_capacity(24);
    pts.push(p0);
    if (k1.0 - p0.0).hypot(k1.1 - p0.1) > 1.0 {
        pts.push(k1);
    }
    for i in 1..=14 {
        let t = i as f32 / 14.0;
        pts.push((k1.0 + (k2.0 - k1.0) * t, k1.1 + (k2.1 - k1.1) * t));
    }
    if (p1.0 - k2.0).hypot(p1.1 - k2.1) > 1.0 {
        pts.push(p1);
    }
    pts
}

fn sample_train_point(
    stations: &[SharedString],
    segments: &[Vec<(f32, f32)>],
    st_px: &HashMap<SharedString, (f32, f32)>,
    pos: f32,
) -> (f32, f32) {
    if segments.is_empty() {
        return stations
            .first()
            .and_then(|s| st_px.get(s).copied())
            .unwrap_or((40.0, 40.0));
    }
    let si = (pos.floor() as usize).min(segments.len() - 1);
    let frac = (pos - si as f32).clamp(0.0, 1.0);
    let pts = &segments[si];
    if pts.len() < 2 {
        return pts.first().copied().unwrap_or((40.0, 40.0));
    }
    let total = (pts.len() - 1) as f32;
    let f_idx = frac * total;
    let idx = (f_idx.floor() as usize).min(pts.len() - 2);
    let u = (f_idx - idx as f32).clamp(0.0, 1.0);
    (
        pts[idx].0 + (pts[idx + 1].0 - pts[idx].0) * u,
        pts[idx].1 + (pts[idx + 1].1 - pts[idx].1) * u,
    )
}

fn paint_offset_polyline(
    pts: &[(f32, f32)],
    ox: f32,
    oy: f32,
    stroke_w: f32,
    color: Hsla,
    window: &mut Window,
) {
    let mut pb = PathBuilder::stroke(px(stroke_w));
    for (i, &(x, y)) in pts.iter().enumerate() {
        let pt = point(px(ox + x), px(oy + y));
        if i == 0 {
            pb.move_to(pt);
        } else {
            pb.line_to(pt);
        }
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

fn paint_partial_polyline(
    pts: &[(f32, f32)],
    ox: f32,
    oy: f32,
    frac: f32,
    stroke_w: f32,
    color: Hsla,
    window: &mut Window,
) {
    let total = (pts.len() - 1) as f32;
    let limit = (frac.clamp(0.0, 1.0) * total).max(0.0);
    let full_end = (limit.floor() as usize).min(pts.len() - 1);
    let rem = limit - full_end as f32;

    let mut pb = PathBuilder::stroke(px(stroke_w));
    pb.move_to(point(px(ox + pts[0].0), px(oy + pts[0].1)));
    for &(x, y) in pts.iter().take(full_end + 1).skip(1) {
        pb.line_to(point(px(ox + x), px(oy + y)));
    }
    if full_end + 1 < pts.len() && rem > 0.01 {
        let nx = pts[full_end].0 + (pts[full_end + 1].0 - pts[full_end].0) * rem;
        let ny = pts[full_end].1 + (pts[full_end + 1].1 - pts[full_end].1) * rem;
        pb.line_to(point(px(ox + nx), px(oy + ny)));
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

fn paint_ahead_pulse(
    pts: &[(f32, f32)],
    ox: f32,
    oy: f32,
    phase: f32,
    color: Hsla,
    window: &mut Window,
) {
    let total = (pts.len() - 1) as f32;
    let head = pro_ease(phase) * total;
    let span = (total * 0.34).max(2.5);
    for i in 0..(pts.len() - 1) {
        let mid = i as f32 + 0.5;
        let dist = (mid - head).abs();
        if dist <= span {
            let falloff = (1.0 - dist / span).clamp(0.0, 1.0);
            let alpha = falloff * falloff * 0.85;
            let mut seg = PathBuilder::stroke(px(2.8));
            seg.move_to(point(px(ox + pts[i].0), px(oy + pts[i].1)));
            seg.line_to(point(px(ox + pts[i + 1].0), px(oy + pts[i + 1].1)));
            if let Ok(path) = seg.build() {
                window.paint_path(path, color.opacity(alpha));
            }
        }
    }
}

fn stop_index_for_line(line: &MetroLine) -> usize {
    if line.stations.is_empty() {
        return 0;
    }
    if line.status == MetroLineStatus::Done {
        return line.stations.len() - 1;
    }
    line.at
        .as_ref()
        .and_then(|id| line.stations.iter().position(|s| s == id))
        .unwrap_or(0)
}

fn station_passed_by_line(line: &MetroLine, station_id: &SharedString) -> bool {
    let Some(pos) = line.stations.iter().position(|s| s == station_id) else {
        return false;
    };
    match line.status {
        MetroLineStatus::Idle => false,
        MetroLineStatus::Done => true,
        MetroLineStatus::Running => pos <= stop_index_for_line(line),
    }
}

fn infer_provider_key(model: &str) -> &'static str {
    let lower = model.to_ascii_lowercase();
    if lower.contains("claude")
        || lower.contains("opus")
        || lower.contains("sonnet")
        || lower.contains("haiku")
        || lower.contains("anthropic")
    {
        "claude"
    } else if lower.contains("gemini") {
        "gemini"
    } else if lower.contains("deepseek") {
        "deepseek"
    } else if lower.contains("mistral") || lower.contains("codestral") {
        "mistral"
    } else if lower.contains("qwen") {
        "qwen"
    } else if lower.contains("llama") {
        "llama"
    } else if lower.contains("grok") {
        "grok"
    } else {
        "openai"
    }
}

fn metro_avatar_seed(model: &str, name: &str, id: &str) -> &'static str {
    let m = model.to_ascii_lowercase();
    if m.contains("claude")
        || m.contains("opus")
        || m.contains("sonnet")
        || m.contains("haiku")
        || m.contains("anthropic")
    {
        "claude"
    } else if m.contains("gemini") {
        "gemini"
    } else if m.contains("deepseek") {
        "deepseek"
    } else if m.contains("mistral") || m.contains("codestral") {
        "mistral"
    } else if m.contains("qwen") {
        "qwen"
    } else if m.contains("llama") {
        "llama"
    } else if m.contains("grok") {
        "grok"
    } else if m.contains("gpt") || m.contains("openai") || m.contains("codex") {
        "gpt"
    } else {
        let s = format!("{name} {id}").to_ascii_lowercase();
        if s.contains("research") || s.contains("note") || s.contains("write") || s.contains("lead")
        {
            "claude"
        } else if s.contains("doc") || s.contains("help") || s.contains("review") {
            "gemini"
        } else {
            "gpt"
        }
    }
}

fn tr_station_label<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    match id {
        "question" => tr(cx, "Question", "问题定位"),
        "search" => tr(cx, "Search", "代码检索"),
        "docs" => tr(cx, "Read docs", "查阅文档"),
        "brief" => tr(cx, "Brief", "发版简报"),
        "split" => tr(cx, "Split work", "任务拆解"),
        "branch" => tr(cx, "Branch", "创建分支"),
        "fix" => tr(cx, "Write fix", "编写修复"),
        "tests" => tr(cx, "Tests", "单元测试"),
        "track" => tr(cx, "Track", "进度同步"),
        "notes" => tr(cx, "Draft notes", "起草日志"),
        "shots" => tr(cx, "Screenshots", "更新截图"),
        "proof" => tr(cx, "Proofread", "文案校对"),
        "help" => tr(cx, "Help article", "帮助文档"),
        "translate" => tr(cx, "Translate", "多语翻译"),
        "publish" => tr(cx, "Publish docs", "文档上线"),
        "review" => tr(cx, "Review", "汇合评审"),
        "e2e" => tr(cx, "E2E tests", "端到端测试"),
        "load" => tr(cx, "Load test", "压力测试"),
        "staging" => tr(cx, "Staging", "预发环境"),
        "signoff" => tr(cx, "Sign-off", "人工签批"),
        "prod" => tr(cx, "Production", "生产发布"),
        _ => fallback,
    }
}

fn default_metro_network() -> (Vec<MetroStation>, Vec<MetroLine>) {
    let stations = vec![
        MetroStation::new("question", "Question", 0, 0)
            .label_above()
            .with_detail("Clarify scope and breaking API constraints for v2.4."),
        MetroStation::new("search", "Search", 1, 0)
            .label_above()
            .with_detail("Index recent checkout and webhook commits.")
            .with_tools(["GitHub"]),
        MetroStation::new("docs", "Read docs", 2, 0)
            .label_above()
            .with_detail("Cross-check SDK migration guide in Notion.")
            .with_tools(["Notion"]),
        MetroStation::new("brief", "Brief", 0, 2)
            .with_detail("Initialize v2.4 release manifest and target Friday window.")
            .with_tools(["Linear"]),
        MetroStation::new("split", "Split work", 1, 2)
            .with_detail("Interchange where Planner dispatches Coder and Writer.")
            .with_tools(["Linear", "Slack"]),
        MetroStation::new("branch", "Branch", 2, 1)
            .label_above()
            .with_detail("Create release/v2.4 branch and lock dependency pins.")
            .with_tools(["GitHub"]),
        MetroStation::new("fix", "Write fix", 3, 1)
            .label_above()
            .with_detail("Interchange merging Researcher findings into Coder patch.")
            .with_tools(["GitHub"]),
        MetroStation::new("tests", "Tests", 4, 1)
            .label_above()
            .with_detail("Run 412 unit and integration assertions."),
        MetroStation::new("track", "Track", 3, 2)
            .with_detail("Monitor parallel milestones across engineering and docs."),
        MetroStation::new("notes", "Draft notes", 2, 3)
            .with_detail("Draft customer-facing v2.4 changelog highlights.")
            .with_tools(["Notion"]),
        MetroStation::new("shots", "Screenshots", 3, 3)
            .with_detail("Capture clean light and dark UI release visuals.")
            .with_tools(["Figma"]),
        MetroStation::new("proof", "Proofread", 4, 3)
            .with_detail("Interchange handing verified release copy to Support.")
            .with_tools(["Notion", "Slack"]),
        MetroStation::new("help", "Help article", 5, 4)
            .with_detail("Update help-center billing & team permissions articles."),
        MetroStation::new("translate", "Translate", 6, 4)
            .with_detail("Localize release documentation into Chinese and Japanese."),
        MetroStation::new("publish", "Publish docs", 7, 4)
            .with_detail("Stage and publish documentation portal updates."),
        MetroStation::new("review", "Review", 5, 2)
            .with_detail("Central 4-line interchange joining Planner, Coder, Writer, and QA.")
            .with_tools(["GitHub", "Linear"]),
        MetroStation::new("e2e", "E2E tests", 7, 3)
            .with_detail("Run full browser checkout and SSO smoke suite."),
        MetroStation::new("load", "Load test", 8, 3)
            .with_detail("Verify p99 latency under 5,000 req/s synthetic traffic."),
        MetroStation::new("staging", "Staging", 6, 2)
            .with_detail("Deploy release candidate to staging cluster."),
        MetroStation::new("signoff", "Sign-off", 8, 2)
            .with_status(MetroStationStatus::Waiting)
            .with_detail("Release candidate v2.4 passed staging & E2E checks. Approve final production rollout?")
            .with_tools(["GitHub", "Slack"]),
        MetroStation::new("prod", "Production", 9, 1)
            .label_above()
            .with_detail("Promote v2.4 across global production regions."),
    ];

    let lines = vec![
        MetroLine::new(
            "planner",
            "Planner",
            "Claude Opus 5.5",
            [
                "brief", "split", "track", "review", "staging", "signoff", "prod",
            ],
        )
        .with_at("staging")
        .with_status(MetroLineStatus::Running),
        MetroLine::new(
            "coder",
            "Coder",
            "GPT-5.5 Codex",
            ["split", "branch", "fix", "tests", "review"],
        )
        .with_status(MetroLineStatus::Done),
        MetroLine::new(
            "writer",
            "Writer",
            "Gemini 3 Pro",
            ["split", "notes", "shots", "proof", "review"],
        )
        .with_at("proof")
        .with_status(MetroLineStatus::Running),
        MetroLine::new(
            "researcher",
            "Researcher",
            "Claude Sonnet 5.5",
            ["question", "search", "docs", "fix"],
        )
        .with_status(MetroLineStatus::Done),
        MetroLine::new("qa", "QA", "GPT-5.5", ["review", "e2e", "load", "signoff"])
            .with_at("load")
            .with_status(MetroLineStatus::Running),
        MetroLine::new(
            "support",
            "Support",
            "Gemini 3 Flash",
            ["proof", "help", "translate", "publish"],
        )
        .with_at("help")
        .with_status(MetroLineStatus::Running),
    ];

    (stations, lines)
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{AgentMetroMap, MetroStationStatus, build_metro_45_segment, default_metro_network};
    use gpui::TestAppContext;

    #[test]
    fn metro_diagonal_segments_preserve_exact_45_degree_slope() {
        let pts = build_metro_45_segment((111.0, 228.0), (188.0, 140.0));
        assert!(pts.len() >= 16);
        // Middle diagonal segment must have unit slope |dy / dx| == 1.0
        let a = pts[4];
        let b = pts[10];
        let slope = ((b.1 - a.1) / (b.0 - a.0)).abs();
        assert!(
            (slope - 1.0).abs() < 1e-4,
            "expected 45° slope, got {slope}"
        );
    }

    #[test]
    fn approving_waiting_signoff_station_advances_trains_cleanly() {
        let (stations, lines) = default_metro_network();
        assert_eq!(stations.len(), 21);
        assert_eq!(lines.len(), 6);

        let mut app = TestAppContext::single();
        app.update(crate::init);
        let map = app.new(|cx| AgentMetroMap::new("metro-test", cx));
        map.update(&mut app, |m, cx| {
            m.advance_trains(cx);
            // Sign-off is Waiting before approval
            let signoff = m
                .stations
                .iter()
                .find(|s| s.id.as_ref() == "signoff")
                .expect("signoff exists");
            assert_eq!(signoff.status, MetroStationStatus::Waiting);

            m.resolve_station("signoff", true, cx);
            let signoff_after = m
                .stations
                .iter()
                .find(|s| s.id.as_ref() == "signoff")
                .expect("signoff exists");
            assert_eq!(signoff_after.status, MetroStationStatus::Normal);
        });
    }
}
