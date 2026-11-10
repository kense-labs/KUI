//! Trial status with a progress ring, billing actions and a recoverable collapsed view.
use crate::{
    foundation::{Theme, surface_radius},
    primitives::{
        family::{Paint, PhaseClock, Slide, Tone, ease, face, measure},
        flyout::TriggerBounds,
        glyph::{Glyph, icon},
        nav::overlay,
        presentation::{column, row, tr},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, f32::consts::TAU, rc::Rc, time::Duration};

pub use crate::primitives::form_dates::CivilDate;

const BREAKPOINT: f32 = 600.;
const CARD_WIDTH: f32 = 340.;
const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Placement of the trial information.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrialVariant {
    #[default]
    Banner,
    Card,
    Pill,
}

/// What remains after a snooze or dismissal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrialCollapse {
    #[default]
    Pill,
    Hidden,
}

/// The reminder date selected by the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialSnooze {
    Tomorrow,
    ThreeDays,
    AtWarning,
}

/// A feature available during the trial.
#[derive(Clone, Debug)]
pub struct TrialFeature {
    pub id: SharedString,
    pub label: SharedString,
    pub detail: Option<SharedString>,
    pub icon: Option<Glyph>,
}

impl TrialFeature {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            detail: None,
            icon: None,
        }
    }
}

/// Intent and completion events for the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrialEvent {
    ComparePlans,
    ManageBilling,
    Dismissed,
    Snoozed {
        choice: TrialSnooze,
        until: CivilDate,
    },
    Upgraded {
        simulated: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Active,
    Warning,
    Expired,
    Upgraded,
}

#[derive(Clone, Copy)]
struct Timing {
    reference: CivilDate,
    end: Option<CivilDate>,
    left: i32,
    total: u32,
    warn_at: u32,
}

impl Timing {
    fn status(self) -> Status {
        match self.left {
            ..=0 => Status::Expired,
            days if days as u32 <= self.warn_at => Status::Warning,
            _ => Status::Active,
        }
    }

    fn ratio(self) -> f32 {
        (self.left as f32 / self.total.max(1) as f32).clamp(0., 1.)
    }

    fn end(self) -> CivilDate {
        self.end
            .unwrap_or_else(|| self.reference.add_days(self.left))
    }

    fn options(self) -> Vec<(TrialSnooze, CivilDate)> {
        let mut options = vec![(TrialSnooze::Tomorrow, self.reference.add_days(1))];
        let end = self.end();
        let in_three_days = self.reference.add_days(3);
        if self.left > 3 && in_three_days < end {
            options.push((TrialSnooze::ThreeDays, in_three_days));
        }
        let warning = end.add_days(-(self.warn_at.min(i32::MAX as u32) as i32));
        if self.warn_at != 0 && self.status() == Status::Active && warning > self.reference {
            options.push((TrialSnooze::AtWarning, warning));
        }
        options
    }
}

type Payment = Box<dyn Fn(&mut App) -> Task<Result<(), SharedString>>>;

/// A shared GPUI trial banner, sidebar card or expandable pill.
pub struct TrialBanner {
    id: ElementId,
    variant: TrialVariant,
    collapse: TrialCollapse,
    plan: SharedString,
    workspace: SharedString,
    features: Option<Vec<TrialFeature>>,
    summary: Option<SharedString>,
    timing: Timing,
    dismissible: bool,
    snoozable: bool,
    collapsed: bool,
    open: bool,
    menu: bool,
    upgraded: bool,
    error: Option<SharedString>,
    payment: Option<Payment>,
    pending: Option<Task<()>>,
    notice: Option<SharedString>,
    angle: Slide,
    urgency: Slide,
    payment_width: Option<Slide>,
    pill_width: Option<Slide>,
    text_change: PhaseClock,
    last_text: Option<[SharedString; 2]>,
    text: Option<[SharedString; 2]>,
    width: Rc<Cell<Pixels>>,
    pill_bounds: TriggerBounds,
    snooze_bounds: TriggerBounds,
    menu_bounds: TriggerBounds,
    pill: FocusHandle,
    primary: FocusHandle,
    snooze: FocusHandle,
    options: [FocusHandle; 3],
}

impl EventEmitter<TrialEvent> for TrialBanner {}

impl TrialBanner {
    /// Uses a deterministic sample trial until the host supplies its billing dates.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        Self {
            id: id.into(),
            variant: TrialVariant::Banner,
            collapse: TrialCollapse::Pill,
            plan: "Pro".into(),
            workspace: "Northwind".into(),
            features: None,
            summary: None,
            timing: Timing {
                reference: CivilDate {
                    year: 2026,
                    month: 10,
                    day: 27,
                },
                end: None,
                left: 9,
                total: 14,
                warn_at: 3,
            },
            dismissible: true,
            snoozable: true,
            collapsed: false,
            open: false,
            menu: false,
            upgraded: false,
            error: None,
            payment: None,
            pending: None,
            notice: None,
            angle: Slide::new(9. / 14.),
            urgency: Slide::new(0.),
            payment_width: None,
            pill_width: None,
            text_change: PhaseClock::default(),
            last_text: None,
            text: None,
            width: Rc::default(),
            pill_bounds: TriggerBounds::default(),
            snooze_bounds: TriggerBounds::default(),
            menu_bounds: TriggerBounds::default(),
            pill: cx.focus_handle().tab_stop(true),
            primary: cx.focus_handle().tab_stop(true),
            snooze: cx.focus_handle().tab_stop(true),
            options: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
        }
    }

    pub fn with_variant(mut self, variant: TrialVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn with_plan(mut self, plan: impl Into<SharedString>) -> Self {
        self.plan = plan.into();
        self
    }

    pub fn with_workspace(mut self, workspace: impl Into<SharedString>) -> Self {
        self.workspace = workspace.into();
        self
    }

    pub fn with_features(mut self, features: Vec<TrialFeature>) -> Self {
        self.features = Some(features);
        self
    }

    pub fn with_loss_summary(mut self, summary: impl Into<SharedString>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Uses the host's local civil day for snooze dates and its end date for copy.
    pub fn with_schedule(
        mut self,
        today: CivilDate,
        ends_at: CivilDate,
        days_left: i32,
        trial_days: u32,
    ) -> Self {
        self.timing = Timing {
            reference: today,
            end: Some(ends_at),
            left: days_left,
            total: trial_days,
            ..self.timing
        };
        self.angle.reset(self.timing.ratio());
        self.urgency
            .reset(f32::from(self.timing.status() == Status::Warning));
        self
    }

    pub fn with_warning_days(mut self, days: u32) -> Self {
        self.timing.warn_at = days;
        self.urgency
            .reset(f32::from(self.timing.status() == Status::Warning));
        self
    }

    pub fn with_collapse(mut self, collapse: TrialCollapse) -> Self {
        self.collapse = collapse;
        self
    }

    pub fn with_dismissible(mut self, enabled: bool) -> Self {
        self.dismissible = enabled;
        self
    }

    pub fn with_snoozable(mut self, enabled: bool) -> Self {
        self.snoozable = enabled;
        self
    }

    /// Supplies the host's payment task. Dropping the control cancels the task.
    pub fn with_payment(
        mut self,
        payment: impl Fn(&mut App) -> Task<Result<(), SharedString>> + 'static,
    ) -> Self {
        self.payment = Some(Box::new(payment));
        self
    }

    /// Updates the same trial without resetting an in-flight payment or a snooze.
    pub fn set_schedule(
        &mut self,
        today: CivilDate,
        ends_at: CivilDate,
        days_left: i32,
        trial_days: u32,
        cx: &mut Context<Self>,
    ) {
        self.timing = Timing {
            reference: today,
            end: Some(ends_at),
            left: days_left,
            total: trial_days,
            ..self.timing
        };
        self.angle.duration(self.timing.ratio(), 0.24, cx);
        self.urgency
            .duration(f32::from(self.timing.status() == Status::Warning), 0.16, cx);
        cx.notify();
    }

    /// Changes placement while retaining billing and snooze state.
    pub fn set_variant(&mut self, variant: TrialVariant, cx: &mut Context<Self>) {
        self.variant = variant;
        self.open = false;
        self.menu = false;
        cx.notify();
    }

    /// Whether the pill currently has its card open.
    pub fn is_expanded(&self) -> bool {
        self.open
    }

    fn status(&self) -> Status {
        if self.upgraded {
            Status::Upgraded
        } else {
            self.timing.status()
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.pending.is_some() || self.upgraded {
            return;
        }
        self.error = None;
        self.notice = Some(tr(cx, "Opening billing…", "正在打开账单…").into());
        let payment = self.payment.as_ref().map(|payment| payment(cx));
        self.pending = Some(cx.spawn(async move |view, cx| {
            let simulated = payment.is_none();
            let result = match payment {
                Some(task) => task.await,
                None => {
                    cx.background_executor()
                        .timer(Duration::from_millis(900))
                        .await;
                    Ok(())
                }
            };
            let _ = view.update(cx, |view, cx| {
                view.pending = None;
                match result {
                    Ok(()) => {
                        view.upgraded = true;
                        view.notice = Some(
                            if simulated {
                                tr(
                                    cx,
                                    "Upgrade simulated. Nothing was charged.",
                                    "已模拟升级，未产生费用。",
                                )
                            } else {
                                tr(cx, "Payment method added.", "已添加付款方式。")
                            }
                            .into(),
                        );
                        cx.emit(TrialEvent::Upgraded { simulated });
                    }
                    Err(message) => {
                        view.notice = Some(message.clone());
                        view.error = Some(message);
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn collapse(&mut self, event: TrialEvent, w: &mut Window, cx: &mut Context<Self>) {
        self.pending = None;
        self.error = None;
        self.open = false;
        self.menu = false;
        self.collapsed = true;
        self.notice = Some(
            match event {
                TrialEvent::Snoozed { until, .. } => format!(
                    "{} {}",
                    tr(cx, "Reminder set for", "提醒日期"),
                    date_label(until, false, cx)
                ),
                _ => tr(cx, "Trial banner dismissed.", "已关闭试用横幅。").to_owned(),
            }
            .into(),
        );
        if self.collapse == TrialCollapse::Pill {
            self.pill.focus(w, cx);
        }
        cx.emit(event);
        cx.notify();
    }

    fn toggle_menu(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.menu = !self.menu;
        if self.menu {
            self.options[0].focus(w, cx);
        } else {
            self.snooze.focus(w, cx);
        }
        cx.notify();
    }

    fn close_card(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.menu = false;
        self.pill.focus(w, cx);
        cx.notify();
    }

    fn strings(&self, card: bool, cx: &App) -> [SharedString; 2] {
        let plan = &self.plan;
        let days = self.timing.left;
        let day = if days == 1 { "day" } else { "days" };
        let date = self.timing.end();
        let end = date_label(date, false, cx);
        let dated = date_label(date, true, cx);
        let summary = self.summary.as_deref().unwrap_or(tr(
            cx,
            "Okta SSO and 12 seats",
            "Okta SSO 和 12 个席位",
        ));
        let (title, detail) = match self.status() {
            Status::Active => (
                format!("{days} {day} left in your {plan} trial"),
                if card {
                    format!("Trial ends {dated}")
                } else {
                    format!("Keep {summary} after {end}.")
                },
            ),
            Status::Warning => (
                format!("Your {plan} trial ends in {days} {day}"),
                if card {
                    format!("Ends {dated}")
                } else {
                    format!("On {} you lose {summary}.", WEEKDAYS[date.weekday()])
                },
            ),
            Status::Expired => (
                format!("Your {plan} trial has ended"),
                if card {
                    format!("Ended {dated}")
                } else {
                    format!("{summary} are paused on Free.")
                },
            ),
            Status::Upgraded => (
                format!("You're on {plan}"),
                tr(cx, "Your features are ready to use.", "你的功能现已可用。").to_owned(),
            ),
        };
        if tr(cx, "en", "zh") == "en" {
            return [title.into(), detail.into()];
        }
        let title = match self.status() {
            Status::Active => format!("{plan} 试用还剩 {days} 天"),
            Status::Warning => format!("{plan} 试用将在 {days} 天后结束"),
            Status::Expired => format!("{plan} 试用已结束"),
            Status::Upgraded => format!("已升级至 {plan}"),
        };
        let detail = match self.status() {
            Status::Active | Status::Warning if card => format!("结束于 {dated}"),
            Status::Active => format!("{end}后保留{summary}。"),
            Status::Warning => format!("你将在{end}失去{summary}。"),
            Status::Expired if card => format!("已于{dated}结束"),
            Status::Expired => format!("免费版已暂停{summary}。"),
            Status::Upgraded => detail,
        };
        [title.into(), detail.into()]
    }

    fn heading(&mut self, card: bool, p: Paint, w: &Window, cx: &App) -> Div {
        let next = self.strings(card, cx);
        if self.text.as_ref() != Some(&next) {
            self.last_text = self.text.replace(next.clone());
            if self.last_text.is_some() {
                self.text_change.restart(cx);
            }
        }
        let elapsed = self.text_change.time(0.16, w, cx);
        let entering = ease(elapsed / 0.16);
        let leaving = 1. - ease(elapsed / 0.10);
        let content = |lines: [SharedString; 2]| {
            column()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .line_height(px(20.))
                        .child(lines[0].clone()),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(18.))
                        .text_color(p.secondary)
                        .child(lines[1].clone()),
                )
        };
        column()
            .relative()
            .child(
                content(next)
                    .opacity(entering)
                    .relative()
                    .top(px(2. * (1. - entering))),
            )
            .when(leaving > 0., |div| {
                div.children(self.last_text.clone().map(|old| {
                    content(old)
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .opacity(leaving)
                }))
            })
    }

    fn indicator(&self, size: f32, w: &Window, cx: &App) -> Div {
        let p = Paint::get(cx);
        let status = self.status();
        let color = p.ink.blend(
            Theme::global(cx)
                .tokens
                .colors
                .warning
                .opacity(self.urgency.get(w, cx).clamp(0., 1.)),
        );
        let mut mark = row().relative().size(px(size)).flex_none().justify_center();
        mark = match status {
            Status::Expired | Status::Upgraded => mark.bg(p.hover).rounded_full().child(icon(
                if status == Status::Expired {
                    Glyph::Lock
                } else {
                    Glyph::Check
                },
                size * 0.52,
                if status == Status::Expired {
                    p.secondary
                } else {
                    p.success
                },
            )),
            _ => {
                let ratio = self.angle.get(w, cx).clamp(0., 1.);
                mark.child(
                    canvas(
                        |_, _, _| (),
                        move |b, _, w, _| {
                            let center = b.center();
                            let radius = size / 2. - 2.;
                            paint_arc(w, center, radius, 1., p.line);
                            paint_arc(w, center, radius, ratio, color);
                        },
                    )
                    .absolute()
                    .inset_0(),
                )
                .when(size >= 28., |mark| {
                    mark.child(
                        div()
                            .text_size(px(if size > 30. { 16. } else { 11. }))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(color)
                            .child(self.timing.left.to_string()),
                    )
                })
            }
        };
        mark
    }

    fn controls(
        &mut self,
        primary_first: bool,
        height: f32,
        w: &Window,
        cx: &Context<Self>,
    ) -> [Stateful<Div>; 2] {
        let p = Paint::get(cx);
        let upgraded = self.upgraded;
        let busy = self.pending.is_some();
        let label: SharedString = if upgraded {
            tr(cx, "Manage billing", "管理账单").into()
        } else if busy {
            tr(cx, "Opening billing…", "正在打开账单…").into()
        } else if self.error.is_some() {
            tr(cx, "Try again", "重试").into()
        } else if self.status() == Status::Expired {
            if tr(cx, "en", "zh") == "en" {
                format!("Upgrade to {}", self.plan).into()
            } else {
                format!("升级至 {}", self.plan).into()
            }
        } else {
            tr(cx, "Add payment method", "添加付款方式").into()
        };
        let mut primary = action(
            "trial-primary",
            &label,
            Tone::Primary,
            height,
            cx,
            |s, _, cx| {
                if s.upgraded {
                    cx.emit(TrialEvent::ManageBilling);
                } else {
                    s.submit(cx);
                }
            },
        )
        .track_focus(&self.primary)
        .when(busy, |e| e.cursor_default());
        if !primary_first {
            let target = measure(&label, 14., w).ceil() + 26.;
            let width = self.payment_width.get_or_insert_with(|| Slide::new(target));
            width.duration(target, 0.24, cx);
            primary = primary.w(px(width.get(w, cx))).px_0().overflow_hidden();
        }
        let compare = action(
            "trial-compare",
            tr(cx, "Compare plans", "比较方案"),
            Tone::Clear,
            height,
            cx,
            |_, _, cx| cx.emit(TrialEvent::ComparePlans),
        )
        .text_color(p.secondary);
        if primary_first {
            [primary, compare]
        } else {
            [compare, primary]
        }
    }

    fn utilities(&self, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        let can_snooze = self.snoozable && !self.upgraded;
        let close = if self.open {
            tr(cx, "Close", "关闭")
        } else {
            tr(cx, "Dismiss", "收起")
        };
        row()
            .flex_none()
            .when(can_snooze, |r| {
                r.child(
                    action_face(
                        "trial-snooze",
                        tr(cx, "Snooze", "稍后提醒"),
                        Tone::Clear,
                        36.,
                        cx,
                        |s, w, cx| s.toggle_menu(w, cx),
                    )
                    .px_0()
                    .w(px(36.))
                    .track_focus(&self.snooze)
                    .aria_expanded(self.menu)
                    .child(self.snooze_bounds.measure())
                    .child(icon(Glyph::Clock, 16., p.faint)),
                )
            })
            .when(self.open || self.dismissible, |r| {
                r.child(
                    action_face("trial-dismiss", close, Tone::Clear, 36., cx, |s, w, cx| {
                        if s.open {
                            s.close_card(w, cx);
                        } else {
                            s.collapse(TrialEvent::Dismissed, w, cx);
                        }
                    })
                    .px_0()
                    .w(px(36.))
                    .child(icon(Glyph::Close, 16., p.faint)),
                )
            })
    }

    fn banner(&mut self, narrow: bool, w: &Window, cx: &Context<Self>) -> Stateful<Div> {
        let p = Paint::get(cx);
        let heading = self.heading(false, p, w, cx);
        let [compare, pay] = self.controls(false, 36., w, cx);
        let actions = row()
            .flex_wrap()
            .gap(px(4.))
            .when(!self.upgraded, |r| r.child(compare))
            .child(pay);
        let utilities = self.utilities(cx);
        let body = if narrow {
            column()
                .w_full()
                .gap(px(12.))
                .child(
                    row()
                        .items_start()
                        .gap(px(12.))
                        .child(self.indicator(28., w, cx).mt(px(6.)))
                        .child(heading.flex_1().min_h(px(61.)))
                        .child(utilities),
                )
                .child(actions.ml(px(28.)))
        } else {
            column().w_full().child(
                row()
                    .gap(px(10.))
                    .child(self.indicator(28., w, cx))
                    .child(heading.flex_1())
                    .child(actions)
                    .child(utilities),
            )
        };
        self.surface("trial-banner", Role::Region, cx)
            .w_full()
            .pl(px(14.))
            .pr(px(8.))
            .pt(px(12.))
            .pb(px(14.))
            .child(body)
            .children(self.failure(cx))
    }

    fn card(&mut self, width: Pixels, w: &Window, cx: &Context<Self>) -> Stateful<Div> {
        let p = Paint::get(cx);
        let heading = self.heading(true, p, w, cx);
        let utilities = self.utilities(cx);
        let [pay, compare] = self.controls(true, 44., w, cx);
        let role = if self.open {
            Role::Dialog
        } else {
            Role::Region
        };
        let mut card = self
            .surface("trial-card", role, cx)
            .w(width)
            .max_w_full()
            .p(px(16.))
            .child(self.indicator(40., w, cx))
            .child(utilities.absolute().top(px(10.)).right(px(10.)))
            .child(heading.mt(px(12.)));
        if !self.upgraded {
            let en = tr(cx, "en", "zh") == "en";
            let title = match self.status() {
                Status::Warning if en => format!(
                    "You lose these on {}",
                    WEEKDAYS[self.timing.end().weekday()]
                ),
                Status::Warning => format!(
                    "你将在 {} 失去以下功能",
                    date_label(self.timing.end(), false, cx)
                ),
                Status::Expired if en => format!("Paused while {} is on Free", self.workspace),
                Status::Expired => format!("{} 使用免费版期间暂停", self.workspace),
                _ => tr(cx, "Add a card to keep", "添加付款方式以保留").to_owned(),
            };
            let default_features = [
                (
                    tr(cx, "Single sign-on with Okta", "Okta 单点登录"),
                    "",
                    Glyph::Lock,
                ),
                (
                    tr(cx, "Slack alerts", "Slack 通知"),
                    tr(cx, "4 channels", "4 个频道"),
                    Glyph::Bell,
                ),
                (
                    tr(cx, "Linear issue sync", "Linear 问题同步"),
                    tr(cx, "148 issues", "148 个问题"),
                    Glyph::Link,
                ),
                (
                    tr(cx, "Team seats", "团队席位"),
                    tr(cx, "12 in use", "已使用 12 个"),
                    Glyph::User,
                ),
            ];
            let features = column().id("trial-feature-list").role(Role::List);
            let feature_row =
                |label: SharedString, detail: Option<SharedString>, mark: Option<Glyph>| {
                    row()
                        .gap(px(8.))
                        .min_h(px(32.))
                        .text_size(px(13.))
                        .children(mark.map(|mark| icon(mark, 14., p.secondary)))
                        .child(div().flex_1().child(label))
                        .children(detail.map(|detail| div().text_color(p.secondary).child(detail)))
                };
            let features = if let Some(items) = &self.features {
                features.children(items.iter().map(|f| {
                    feature_row(f.label.clone(), f.detail.clone(), f.icon)
                        .id(f.id.clone())
                        .role(Role::ListItem)
                }))
            } else {
                features.children(default_features.into_iter().enumerate().map(
                    |(i, (label, detail, glyph))| {
                        feature_row(
                            label.into(),
                            (!detail.is_empty()).then(|| detail.into()),
                            Some(glyph),
                        )
                        .id(("trial-feature", i))
                        .role(Role::ListItem)
                    },
                ))
            };
            card = card.child(
                column()
                    .gap(px(2.))
                    .mt(px(16.))
                    .border_t_1()
                    .border_color(p.line)
                    .pt(px(12.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.secondary)
                            .child(title),
                    )
                    .child(features),
            );
        }
        card.child(
            column()
                .mt(px(16.))
                .gap(px(4.))
                .child(pay.w_full())
                .when(!self.upgraded, |e| e.child(compare.w_full())),
        )
        .children(self.failure(cx))
        .when(self.open, |card| card.children(self.status_note(cx)))
    }

    fn surface(&self, id: &'static str, role: Role, cx: &App) -> Stateful<Div> {
        let p = Paint::get(cx);
        column()
            .id(id)
            .debug_selector(move || id.to_owned())
            .relative()
            .role(role)
            .aria_label(format!("{} {}", self.plan, tr(cx, "trial", "试用")))
            .rounded(surface_radius(cx))
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
    }

    fn failure(&self, cx: &App) -> Option<Stateful<Div>> {
        self.error.as_ref().map(|message| {
            div()
                .id("trial-error")
                .role(Role::Status)
                .mt(px(4.))
                .text_size(px(12.))
                .text_color(Paint::get(cx).danger)
                .child(message.clone())
        })
    }

    fn status_note(&self, cx: &App) -> Option<Stateful<Div>> {
        self.notice
            .as_ref()
            .filter(|_| self.error.is_none())
            .map(|notice| {
                div()
                    .id("trial-status")
                    .role(Role::Status)
                    .text_size(px(12.))
                    .text_color(Paint::get(cx).secondary)
                    .child(notice.clone())
            })
    }
}

impl Render for TrialBanner {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let width = self.width.get();
        let compact = (if width == px(0.) {
            w.viewport_size().width
        } else {
            width
        }) < px(BREAKPOINT);
        let measured = self.width.clone();
        let mut root = column()
            .id(self.id.clone())
            .relative()
            .w_full()
            .items_center()
            .min_w_0()
            .gap(px(6.))
            .child(
                canvas(
                    move |b, w, _| {
                        let previous = measured.replace(b.size.width);
                        if (previous < px(BREAKPOINT)) != (b.size.width < px(BREAKPOINT))
                            || previous == px(0.) && b.size.width != px(0.)
                        {
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .on_key_down(cx.listener(|s, event: &KeyDownEvent, w, cx| {
                if event.keystroke.key == "escape" {
                    if s.menu {
                        s.toggle_menu(w, cx);
                    } else if s.open {
                        s.close_card(w, cx);
                    } else {
                        return;
                    }
                    cx.stop_propagation();
                }
            }));

        if self.collapsed && self.collapse == TrialCollapse::Hidden {
            return root;
        }
        if self.collapsed || self.variant == TrialVariant::Pill {
            let caption = match self.status() {
                Status::Upgraded => self.plan.clone(),
                Status::Expired => tr(cx, "Trial ended", "试用已结束").into(),
                _ => {
                    if tr(cx, "en", "zh") == "en" {
                        format!(
                            "{} {} left",
                            self.timing.left,
                            if self.timing.left == 1 { "day" } else { "days" }
                        )
                        .into()
                    } else {
                        format!("剩 {} 天", self.timing.left).into()
                    }
                }
            };
            let accessible = format!("{} {}: {caption}", self.plan, tr(cx, "trial", "试用"));
            let target = measure(&caption, 14., w).ceil() + 52.;
            let width = self.pill_width.get_or_insert_with(|| Slide::new(target));
            width.duration(target, 0.24, cx);
            let animated_width = width.get(w, cx);
            root = root.child(
                action_face(
                    "trial-pill",
                    &accessible,
                    Tone::Surface,
                    36.,
                    cx,
                    |s, w, cx| {
                        if s.open {
                            s.close_card(w, cx);
                        } else {
                            s.open = true;
                            s.primary.focus(w, cx);
                            cx.notify();
                        }
                    },
                )
                .rounded_full()
                .w(px(animated_width))
                .px_0()
                .overflow_hidden()
                .border_1()
                .border_color(p.line)
                .track_focus(&self.pill)
                .aria_expanded(self.open)
                .child(self.pill_bounds.measure())
                .child(self.indicator(18., w, cx))
                .child(caption),
            );
            if self.open {
                let width = px(CARD_WIDTH)
                    .min(w.viewport_size().width - px(24.))
                    .max(px(1.));
                let height = px(414.);
                let (pos, anchor) = self.pill_bounds.place(w.viewport_size(), height, px(0.));
                let panel = self
                    .card(width, w, cx)
                    .shadow_lg()
                    .occlude()
                    .on_mouse_down_out(cx.listener(|s, event: &MouseDownEvent, w, cx| {
                        if !s.pill_bounds.get().contains(&event.position)
                            && !(s.menu && s.menu_bounds.get().contains(&event.position))
                        {
                            s.close_card(w, cx);
                        }
                    }));
                root = root.child(
                    deferred(
                        anchored()
                            .position(pos)
                            .anchor(anchor)
                            .snap_to_window_with_margin(px(12.))
                            .child(panel),
                    )
                    .with_priority(2),
                );
            }
        } else if self.variant == TrialVariant::Card {
            root = root.child(self.card(px(CARD_WIDTH), w, cx));
        } else {
            root = root.child(self.banner(compact, w, cx));
        }
        if self.menu {
            let options = self.timing.options();
            let count = options.len();
            let list = self
                .surface("trial-snooze-menu", Role::Menu, cx)
                .aria_label(tr(cx, "Snooze", "稍后提醒"))
                .w(px(220.).min(w.viewport_size().width - px(24.)))
                .p(px(4.))
                .occlude()
                .shadow_lg()
                .child(self.menu_bounds.measure())
                .on_mouse_down_out(cx.listener(|s, event: &MouseDownEvent, _, cx| {
                    if !s.snooze_bounds.get().contains(&event.position) {
                        s.menu = false;
                        cx.notify();
                    }
                }))
                .children(options.into_iter().enumerate().map(|(i, (choice, until))| {
                    let label = match choice {
                        TrialSnooze::Tomorrow => {
                            tr(cx, "Remind me tomorrow", "明天提醒我").to_owned()
                        }
                        TrialSnooze::ThreeDays => {
                            tr(cx, "Remind me in 3 days", "3 天后提醒我").to_owned()
                        }
                        TrialSnooze::AtWarning => {
                            if tr(cx, "en", "zh") == "en" {
                                format!("Remind me when {} days are left", self.timing.warn_at)
                            } else {
                                format!("剩 {} 天时提醒我", self.timing.warn_at)
                            }
                        }
                    };
                    action(
                        ("trial-reminder", i),
                        &label,
                        Tone::Clear,
                        36.,
                        cx,
                        move |s, w, cx| s.collapse(TrialEvent::Snoozed { choice, until }, w, cx),
                    )
                    .role(Role::MenuItem)
                    .track_focus(&self.options[i])
                    .justify_start()
                    .on_key_down(cx.listener(
                        move |s, event: &KeyDownEvent, w, cx| {
                            let next = match event.keystroke.key.as_str() {
                                "up" => (i + count - 1) % count,
                                "down" => (i + 1) % count,
                                "home" => 0,
                                "end" => count - 1,
                                "escape" => {
                                    s.toggle_menu(w, cx);
                                    cx.stop_propagation();
                                    return;
                                }
                                _ => return,
                            };
                            s.options[next].focus(w, cx);
                            cx.stop_propagation();
                        },
                    ))
                }));
            let (pos, anchor) =
                self.snooze_bounds
                    .place(w.viewport_size(), px(10. + 36. * count as f32), px(4.));
            root = root.child(overlay(list, pos, anchor));
        }
        root.when(!self.open, |root| root.children(self.status_note(cx)))
    }
}

fn action(
    id: impl Into<ElementId>,
    label: &str,
    tone: Tone,
    height: f32,
    cx: &Context<TrialBanner>,
    f: impl Fn(&mut TrialBanner, &mut Window, &mut Context<TrialBanner>) + Copy + 'static,
) -> Stateful<Div> {
    action_face(id, label, tone, height, cx, f).child(label.to_owned())
}

fn action_face(
    id: impl Into<ElementId>,
    label: &str,
    tone: Tone,
    height: f32,
    cx: &Context<TrialBanner>,
    f: impl Fn(&mut TrialBanner, &mut Window, &mut Context<TrialBanner>) + Copy + 'static,
) -> Stateful<Div> {
    let paint = Paint::get(cx);
    face(
        id,
        SharedString::from(label.to_owned()),
        height,
        surface_radius(cx).into(),
        tone,
        cx,
    )
    .relative()
    .px(px(if height == 44. { 16. } else { 12. }))
    .border_1()
    .border_color(transparent_black())
    .hover(move |s| {
        s.bg(if tone == Tone::Primary {
            paint.primary_hover(0.12)
        } else {
            paint.hover
        })
    })
    .active(|s| s.opacity(0.76))
    .on_click(cx.listener(move |s, _, w, cx| f(s, w, cx)))
    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
        if !e.is_held && matches!(e.keystroke.key.as_str(), "enter" | "space") {
            f(s, w, cx);
            cx.stop_propagation();
        }
    }))
}

fn date_label(date: CivilDate, weekday: bool, cx: &App) -> String {
    if tr(cx, "en", "zh") == "zh" {
        return format!("{}月{}日", date.month, date.day);
    }
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let month = months
        .get(usize::from(date.month.saturating_sub(1)))
        .unwrap_or(&"");
    let prefix = if weekday {
        format!("{}, ", weekdays[date.weekday()])
    } else {
        String::new()
    };
    format!("{prefix}{month} {}", date.day)
}

fn paint_arc(w: &mut Window, center: Point<Pixels>, radius: f32, ratio: f32, color: Hsla) {
    if ratio <= 0. {
        return;
    }
    let steps = (ratio * 52.).ceil() as usize;
    let mut path = PathBuilder::stroke(px(2.));
    for i in 0..=steps {
        let theta = TAU * (ratio * i as f32 / steps as f32 - 0.25);
        let position = center + point(px(theta.cos() * radius), px(theta.sin() * radius));
        if i == 0 {
            path.move_to(position);
        } else {
            path.line_to(position);
        }
    }
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::{CivilDate, Status, Timing, TrialBanner, TrialSnooze, TrialVariant};
    use gpui::{Task, TestAppContext, px, size};
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn remaining_days_determine_warning_expiry_and_valid_reminders() {
        let today = CivilDate::new(2026, 10, 27).unwrap();
        let normal = Timing {
            reference: today,
            end: None,
            left: 9,
            total: 14,
            warn_at: 3,
        };
        assert_eq!(normal.status(), Status::Active);
        assert_eq!(
            normal.options(),
            vec![
                (TrialSnooze::Tomorrow, today.add_days(1)),
                (TrialSnooze::ThreeDays, today.add_days(3)),
                (TrialSnooze::AtWarning, today.add_days(6)),
            ]
        );
        let warning = Timing { left: 2, ..normal };
        assert_eq!(warning.status(), Status::Warning);
        assert_eq!(
            warning.options(),
            vec![(TrialSnooze::Tomorrow, today.add_days(1))]
        );
        let expired = Timing { left: -2, ..normal };
        assert_eq!(expired.status(), Status::Expired);
        assert_eq!(
            expired.options(),
            vec![(TrialSnooze::Tomorrow, today.add_days(1))]
        );
        assert_eq!(expired.ratio(), 0.);
    }

    #[test]
    fn duplicate_payment_is_ignored_and_host_errors_can_be_retried() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let calls = Rc::new(Cell::new(0));
        let (view, w) = app.add_window_view(|_, cx| {
            let calls = calls.clone();
            TrialBanner::new("trial", cx).with_payment(move |_| {
                calls.set(calls.get() + 1);
                Task::ready(if calls.get() == 1 {
                    Err("Unavailable".into())
                } else {
                    Ok(())
                })
            })
        });
        w.update(|_, cx| {
            view.update(cx, |trial, cx| {
                trial.submit(cx);
                trial.submit(cx);
            })
        });
        w.run_until_parked();
        w.update(|_, cx| {
            assert_eq!(calls.get(), 1);
            assert_eq!(view.read(cx).error.as_deref(), Some("Unavailable"));
            view.update(cx, |trial, cx| trial.submit(cx));
        });
        w.run_until_parked();
        w.update(|_, cx| {
            let trial = view.read(cx);
            assert_eq!(calls.get(), 2);
            assert_eq!(trial.status(), Status::Upgraded);
            assert!(trial.error.is_none() && trial.pending.is_none());
        });
    }

    #[test]
    fn keyboard_snooze_returns_focus_to_the_pill_then_primary_action() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| TrialBanner::new("trial", cx));
        w.update(|w, cx| view.read(cx).snooze.clone().focus(w, cx));
        w.simulate_keystrokes("enter");
        w.update(|w, cx| assert!(view.read(cx).options[0].is_focused(w)));
        w.simulate_keystrokes("down");
        w.update(|w, cx| assert!(view.read(cx).options[1].is_focused(w)));
        w.simulate_keystrokes("enter");
        w.update(|w, cx| {
            assert!(view.read(cx).collapsed);
            assert!(view.read(cx).pill.is_focused(w));
        });
        w.simulate_keystrokes("space");
        w.update(|w, cx| assert!(view.read(cx).primary.is_focused(w)));
        w.simulate_keystrokes("escape");
        w.update(|w, cx| {
            assert!(!view.read(cx).open);
            assert!(view.read(cx).pill.is_focused(w));
        });
    }

    #[test]
    fn narrow_banner_moves_actions_below_copy_and_card_keeps_its_width() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| TrialBanner::new("trial", cx));
        for (width, expected) in [(710., false), (522., true), (320., true)] {
            w.simulate_resize(size(px(width), px(650.)));
            w.update(|w, _| w.refresh());
            w.run_until_parked();
            let action = w.debug_bounds("Add payment method").unwrap();
            let banner = w.debug_bounds("trial-banner").unwrap();
            assert_eq!(action.top() - banner.top() > px(55.), expected);
            assert!(action.right() <= banner.right());
        }
        w.simulate_resize(size(px(522.), px(650.)));
        w.update(|_, cx| view.update(cx, |v, cx| v.set_variant(TrialVariant::Card, cx)));
        w.run_until_parked();
        let card = w.debug_bounds("trial-card").unwrap();
        assert_eq!(card.size.width, px(340.));
        assert_eq!(card.size.height, px(411.));
    }
}
