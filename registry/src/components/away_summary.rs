//! Inline "While you were away" thread catch-up card that folds into a one-line divider.
//!
//! Summarizes what an agent accomplished, failed on, or needs human input for while the user was
//! away from a chat thread, offering one direct action per row (`Approve`, `Retry`, `View`) and
//! folding smoothly into a quiet one-line thread divider when dismissed with `"Got it"`.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_btn, pro_check_draw, pro_ease, pro_eid,
        pro_empty_state, pro_fold, pro_mix_color, pro_rolling_number, pro_skeleton_bar,
        pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const FAILED_MARK_SVG: &str = r##"<circle cx="7" cy="7" r="6" fill="currentColor"></circle><path d="M7 4v3.4" stroke="#ffffff" stroke-width="1.5" stroke-linecap="round"></path><circle cx="7" cy="9.8" r="0.85" fill="#ffffff"></circle>"##;
const NEEDS_MARK_SVG: &str = r#"<circle cx="7" cy="7" r="5.25" fill="none" stroke="currentColor" stroke-width="1.5"></circle><circle cx="7" cy="7" r="2.25" fill="currentColor"></circle>"#;

/// Status category of an [`AwayItem`] in [`AwaySummary`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AwayItemKind {
    /// Waiting on a human approval or decision (`Approve` / `Decide`).
    NeedsYou,
    /// Encountered an error that can be retried (`Retry`).
    Failed,
    /// Completed while the user was away (`View`).
    #[default]
    Done,
}

impl AwayItemKind {
    fn sort_rank(self) -> u8 {
        match self {
            Self::NeedsYou => 0,
            Self::Failed => 1,
            Self::Done => 2,
        }
    }

    fn default_action_label(self, cx: &App) -> &'static str {
        match self {
            Self::NeedsYou => tr(cx, "Approve", "批准"),
            Self::Failed => tr(cx, "Retry", "重试"),
            Self::Done => tr(cx, "View", "查看"),
        }
    }
}

/// Display format for the elapsed absence label in [`AwaySummary`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AwayTimeStyle {
    /// Elapsed duration (for example `"2h 14m"`).
    #[default]
    Duration,
    /// Wall-clock timestamp (for example `"since 9:40"`).
    Clock,
}

/// Single update row inside [`AwaySummary`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AwayItem {
    /// Stable item identifier.
    pub id: SharedString,
    /// Task or workstream name (for example `"Weekly report"`).
    pub title: SharedString,
    /// Status category (`NeedsYou`, `Failed`, `Done`).
    pub kind: AwayItemKind,
    /// Optional single-line outcome or blocker description.
    pub detail: Option<SharedString>,
    /// Optional custom action button label (`None` uses the category default; `Some("")` hides the button).
    pub action: Option<SharedString>,
}

impl AwayItem {
    /// Construct an [`AwayItem`] with `id`, `title`, and `kind`.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        kind: AwayItemKind,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            kind,
            detail: None,
            action: None,
        }
    }

    /// Set the single-line detail subtitle.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Override the row's action button label (pass `""` to omit the button).
    pub fn with_action(mut self, action: impl Into<SharedString>) -> Self {
        self.action = Some(action.into());
        self
    }
}

/// Events emitted by [`AwaySummary`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AwaySummaryEvent {
    /// User clicked the direct action button on a row (`Approve`, `Retry`, `View`).
    ActionTriggered {
        /// Row identifier.
        id: SharedString,
        /// Row category at the moment the action was triggered.
        kind: AwayItemKind,
    },
    /// User pressed `"Got it"` to dismiss and fold the card into a thread divider.
    Dismissed,
    /// Card collapsed state changed (`true` = folded into one-line divider, `false` = expanded).
    CollapsedChanged(bool),
    /// User expanded or collapsed overflow rows beyond `limit`.
    ShowMoreToggled(bool),
    /// User sent a follow-up message from the surrounding thread composer.
    ThreadReplySent(SharedString),
}

impl EventEmitter<AwaySummaryEvent> for AwaySummary {}

fn default_away_items() -> Vec<AwayItem> {
    vec![
        AwayItem::new("weekly-report", "Weekly report", AwayItemKind::NeedsYou)
            .with_detail("Waiting on: Approve send to 14 stakeholders?"),
        AwayItem::new("billing-sync", "Billing sync", AwayItemKind::Failed)
            .with_detail("Failed: rate limit on Stripe live API (429)"),
        AwayItem::new("ingest-pipeline", "Ingest pipeline", AwayItemKind::Done)
            .with_detail("Deployed to staging · 14/14 checks green"),
        AwayItem::new("auth-refactor", "Auth refactor", AwayItemKind::Done)
            .with_detail("Opened PR #482 · session token rotation"),
        AwayItem::new("usage-page", "Usage page", AwayItemKind::Done)
            .with_detail("Added per-seat CSV export"),
        AwayItem::new("webhook-retries", "Webhook retries", AwayItemKind::Done)
            .with_detail("Configured exponential backoff & dead-letter queue"),
        AwayItem::new("schema-docs", "Schema docs", AwayItemKind::Done)
            .with_detail("Published OpenAPI 3.1 billing spec"),
        AwayItem::new("staging-smoke", "Staging smoke", AwayItemKind::Done)
            .with_detail("Passed 42 end-to-end verification checks"),
    ]
}

fn sort_items_by_priority(mut items: Vec<AwayItem>) -> Vec<AwayItem> {
    items.sort_by_key(|item| item.kind.sort_rank());
    items
}

/// Inline thread catch-up card showing updates since the user left, folding into a one-line divider.
pub struct AwaySummary {
    id: ElementId,
    focus: FocusHandle,
    away_duration: SharedString,
    away_since_clock: SharedString,
    time_style: AwayTimeStyle,
    items: Vec<AwayItem>,
    limit: usize,
    show_all: bool,
    collapsed: bool,
    thread_chrome: bool,
    replies: Vec<SharedString>,
    reply_entrances: Vec<(Slide, PhaseClock)>,
    done_clocks: HashMap<SharedString, PhaseClock>,
    send_hover: Slide,
    send_press: Slide,
    reply_editor: Option<Entity<Editor>>,
    editor_focused: bool,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl AwaySummary {
    /// Create an [`AwaySummary`] preloaded with `"2h 14m"` away and 8 grouped thread updates.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            away_duration: "2h 14m".into(),
            away_since_clock: "9:40".into(),
            time_style: AwayTimeStyle::Duration,
            items: sort_items_by_priority(default_away_items()),
            limit: 5,
            show_all: false,
            collapsed: false,
            thread_chrome: true,
            replies: Vec::new(),
            reply_entrances: Vec::new(),
            done_clocks: HashMap::new(),
            send_hover: Slide::snappy(0.0),
            send_press: Slide::snappy(0.0),
            reply_editor: None,
            editor_focused: false,
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(600.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Set the elapsed absence duration label (for example `"2h 14m"`).
    pub fn with_since_duration(mut self, duration: impl Into<SharedString>) -> Self {
        self.away_duration = duration.into();
        self
    }

    /// Set the wall-clock absence timestamp used when [`AwayTimeStyle::Clock`] is active.
    pub fn with_since_clock(mut self, clock_label: impl Into<SharedString>) -> Self {
        self.away_since_clock = clock_label.into();
        self
    }

    /// Choose between `"2h 14m"` (`Duration`) and `"since 9:40"` (`Clock`).
    pub fn with_time_style(mut self, style: AwayTimeStyle) -> Self {
        self.time_style = style;
        self
    }

    /// Supply the list of [`AwayItem`] updates (sorted automatically so `NeedsYou` and `Failed` appear first).
    pub fn with_items(mut self, items: Vec<AwayItem>) -> Self {
        self.items = sort_items_by_priority(items);
        self
    }

    /// Set the maximum visible rows before `"Show N more"` (defaults to `5`).
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    /// Set whether the card starts folded into the one-line thread divider.
    pub fn with_collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Show or hide the surrounding chat thread context messages and composer.
    pub fn with_thread_chrome(mut self, thread_chrome: bool) -> Self {
        self.thread_chrome = thread_chrome;
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Whether the summary is currently folded into the one-line thread divider.
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    /// Return `(needs_you_count, failed_count, done_count)`.
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut needs = 0usize;
        let mut failed = 0usize;
        let mut done = 0usize;
        for item in &self.items {
            match item.kind {
                AwayItemKind::NeedsYou => needs += 1,
                AwayItemKind::Failed => failed += 1,
                AwayItemKind::Done => done += 1,
            }
        }
        (needs, failed, done)
    }

    /// Fold or expand the summary card.
    pub fn set_collapsed(&mut self, collapsed: bool, dismiss_event: bool, cx: &mut Context<Self>) {
        if self.collapsed == collapsed {
            return;
        }
        self.collapsed = collapsed;
        if collapsed && dismiss_event {
            cx.emit(AwaySummaryEvent::Dismissed);
        }
        cx.emit(AwaySummaryEvent::CollapsedChanged(collapsed));
        cx.notify();
    }

    /// Trigger the direct action on item `id`, resolving `NeedsYou` or `Failed` rows in place so nothing jumps.
    pub fn trigger_action(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(item) = self.items.iter_mut().find(|it| it.id.as_ref() == id) else {
            return;
        };
        let prev_kind = item.kind;
        let resolved_id = item.id.clone();
        match prev_kind {
            AwayItemKind::NeedsYou => {
                item.kind = AwayItemKind::Done;
                item.detail = Some(
                    tr(
                        cx,
                        "Approved · sent to 14 stakeholders",
                        "已批准 · 已发送给 14 位相关方",
                    )
                    .into(),
                );
                item.action = None;
                self.done_clocks.entry(resolved_id).or_default().restart(cx);
                self.toast = Some((
                    tr(cx, "Approved weekly report", "已批准发送周报").into(),
                    Instant::now(),
                ));
            }
            AwayItemKind::Failed => {
                item.kind = AwayItemKind::Done;
                item.detail = Some(
                    tr(
                        cx,
                        "Retried · synced 128 Stripe invoices",
                        "重试成功 · 已同步 128 笔账单",
                    )
                    .into(),
                );
                item.action = None;
                self.done_clocks.entry(resolved_id).or_default().restart(cx);
                self.toast = Some((
                    tr(cx, "Retried billing sync · succeeded", "账单同步已重试成功").into(),
                    Instant::now(),
                ));
            }
            AwayItemKind::Done => {
                let msg = format!("{}: {}", tr(cx, "Opened", "已打开"), item.title);
                self.toast = Some((msg.into(), Instant::now()));
            }
        }
        cx.emit(AwaySummaryEvent::ActionTriggered {
            id: id.to_owned().into(),
            kind: prev_kind,
        });
        cx.notify();
    }

    fn push_reply(&mut self, msg: SharedString, cx: &mut Context<Self>) {
        self.replies.push(msg.clone());
        let mut sl = Slide::new(10.0);
        sl.duration(0.0, 0.24, cx);
        let mut clk = PhaseClock::default();
        clk.restart(cx);
        self.reply_entrances.push((sl, clk));
        cx.emit(AwaySummaryEvent::ThreadReplySent(msg));
        cx.notify();
    }

    fn ensure_reply_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.reply_editor.is_none() {
            let ph: SharedString = tr(cx, "Reply to the agent…", "回复智能体…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.0;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().trim().to_owned();
                    if !val.is_empty() {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                        this.push_reply(val.into(), cx);
                    }
                }
                _ => {}
            })
            .detach();
            self.reply_editor = Some(ed);
        }
    }

    fn render_row(
        &self,
        item: &AwayItem,
        halo_scale_alpha: f32,
        done_draw: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let row_hover: Hsla = if p.dark {
            hsla(0.0, 0.0, 1.0, 0.035)
        } else {
            hsla(0.0, 0.0, 0.0, 0.025)
        };
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };

        let mark_el = match item.kind {
            AwayItemKind::NeedsYou => div()
                .relative()
                .size(px(14.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .when(halo_scale_alpha > 0.01, |d| {
                    d.child(
                        div()
                            .absolute()
                            .inset(px(-2.0))
                            .rounded_full()
                            .bg(p.amber.opacity(halo_scale_alpha * 0.28)),
                    )
                })
                .child(pro_svg("0 0 14 14", NEEDS_MARK_SVG, 14.0, p.amber))
                .into_any_element(),
            AwayItemKind::Failed => pro_svg("0 0 14 14", FAILED_MARK_SVG, 14.0, p.red),
            AwayItemKind::Done => pro_check_draw(
                pro_eid(&self.id, format!("done-chk-{}", item.id)),
                done_draw,
                14.0,
                true,
                p.blue,
            ),
        };

        let title_str: SharedString = match item.title.as_ref() {
            "Weekly report" => tr(cx, "Weekly report", "每周运营简报").into(),
            "Billing sync" => tr(cx, "Billing sync", "Stripe 账单同步").into(),
            "Ingest pipeline" => tr(cx, "Ingest pipeline", "数据采集流水线").into(),
            "Auth refactor" => tr(cx, "Auth refactor", "鉴权令牌重构").into(),
            "Usage page" => tr(cx, "Usage page", "用量统计页").into(),
            "Webhook retries" => tr(cx, "Webhook retries", "Webhook 自动重试").into(),
            "Schema docs" => tr(cx, "Schema docs", "接口 Schema 文档").into(),
            "Staging smoke" => tr(cx, "Staging smoke", "预发环境冒烟测试").into(),
            _ => item.title.clone(),
        };

        let detail_str: Option<SharedString> = item.detail.as_ref().map(|d| match d.as_ref() {
            "Waiting on: Approve send to 14 stakeholders?" => tr(
                cx,
                "Waiting on: Approve send to 14 stakeholders?",
                "等待确认：是否向 14 位干系人发送周报？",
            )
            .into(),
            "Failed: rate limit on Stripe live API (429)" => tr(
                cx,
                "Failed: rate limit on Stripe live API (429)",
                "失败：触发 Stripe 接口限流 (429)",
            )
            .into(),
            "Deployed to staging · 14/14 checks green" => tr(
                cx,
                "Deployed to staging · 14/14 checks green",
                "已部署至预发环境 · 14/14 项检查通过",
            )
            .into(),
            "Opened PR #482 · session token rotation" => tr(
                cx,
                "Opened PR #482 · session token rotation",
                "已提交 PR #482 · 会话令牌自动轮换",
            )
            .into(),
            "Added per-seat CSV export" => {
                tr(cx, "Added per-seat CSV export", "新增按席位 CSV 导出能力").into()
            }
            _ => d.clone(),
        });

        let act_label: Option<SharedString> = match &item.action {
            Some(s) if s.is_empty() => None,
            Some(s) => Some(s.clone()),
            None => Some(item.kind.default_action_label(cx).into()),
        };

        let item_id = item.id.clone();
        let is_needs = item.kind == AwayItemKind::NeedsYou;
        let has_detail = detail_str.is_some();

        div()
            .w_full()
            .h(px(48.0))
            .pl(px(10.0))
            .pr(px(8.0))
            .rounded(px(8.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .hover(move |s| s.bg(row_hover))
            .child(
                div()
                    .when(has_detail, |d| d.self_start().mt(px(9.0)))
                    .child(mark_el),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::MEDIUM)
                            .line_height(px(20.0))
                            .text_color(p.ink)
                            .truncate()
                            .child(title_str.clone()),
                    )
                    .when_some(detail_str, |d, det| {
                        d.child(div().min_w_0().line_height(px(16.0)).truncate().child(
                            pro_swap_label(
                                pro_eid(&self.id, format!("det-swap-{}", item.id)),
                                det,
                                12.0,
                                FontWeight::NORMAL,
                                p.muted,
                            ),
                        ))
                    }),
            )
            .when_some(act_label, |d, act_lbl| {
                let aria = SharedString::from(format!("{act_lbl}: {title_str}"));
                let act_inner = div()
                    .min_w(px(58.0))
                    .h(px(26.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_swap_label(
                        pro_eid(&self.id, format!("act-lbl-{}", item.id)),
                        act_lbl,
                        12.0,
                        FontWeight::MEDIUM,
                        if is_needs { p.ink } else { p.text },
                    ));
                d.child(
                    pro_tactile(
                        pro_eid(&self.id, format!("act-{}", item.id)),
                        aria,
                        6.0,
                        if is_needs {
                            card_bg
                        } else {
                            transparent_black()
                        },
                        p.chip,
                        if is_needs {
                            p.line_strong
                        } else {
                            transparent_black()
                        },
                        if is_needs {
                            p.line_strong
                        } else {
                            transparent_black()
                        },
                        false,
                        act_inner,
                        cx,
                        move |this, _, cx| {
                            this.trigger_action(item_id.as_ref(), cx);
                        },
                    )
                    .flex_none(),
                )
            })
            .into_any_element()
    }
}

impl Render for AwaySummary {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        if self.thread_chrome {
            self.ensure_reply_editor(w, cx);
        }

        if self.loading {
            let phase = (self.boot_at.elapsed().as_secs_f32() * 0.5).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .py(px(24.0))
                .px(px(16.0))
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(600.0))
                        .p(px(14.0))
                        .rounded(px(12.0))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .child(pro_skeleton_bar(220.0, 16.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(540.0, 44.0, 7.0, phase, cx))
                        .child(pro_skeleton_bar(540.0, 44.0, 7.0, phase, cx)),
                )
                .into_any_element();
        }

        if self.empty || self.items.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(pro_empty_state(
                    ProIcon::Clock,
                    tr(cx, "Nothing happened while you were away", "离线期间暂无新动态"),
                    tr(
                        cx,
                        "Completed tasks, retries, and questions waiting on you will appear inline here.",
                        "当你离开会话时，智能体完成的任务、失败重试与待确认事项将汇总于此。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let (needs_n, failed_n, done_n) = self.counts();
        let total_n = self.items.len();
        let reduced = disabled_motion(cx);
        if !reduced && needs_n > 0 && !self.collapsed {
            w.request_animation_frame();
        }
        let halo_alpha = if !reduced && needs_n > 0 {
            let cyc = (self.boot_at.elapsed().as_secs_f32() / 2.4).fract();
            if cyc < 0.7 {
                (1.0 - cyc / 0.7).powi(2)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let cw = self.measured_width.get();
        let narrow = cw < 460.0;

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };

        let when_txt: SharedString = match self.time_style {
            AwayTimeStyle::Duration => self.away_duration.clone(),
            AwayTimeStyle::Clock => {
                if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("自 {} 起", self.away_since_clock).into()
                } else {
                    format!("since {}", self.away_since_clock).into()
                }
            }
        };

        // 1. Folded one-line divider
        let divider_dot_c = if needs_n > 0 {
            p.amber
        } else if failed_n > 0 {
            p.red
        } else {
            p.blue
        };
        let divider_text: SharedString = if UiLanguage::current(cx) == UiLanguage::Chinese {
            if needs_n > 0 {
                format!(
                    "离开 {} · {} 项动态，{} 项待你处理",
                    when_txt, total_n, needs_n
                )
                .into()
            } else {
                format!("离开 {} · {} 项动态", when_txt, total_n).into()
            }
        } else if needs_n > 0 {
            format!(
                "You were away {} · {} updates, {} needs you",
                when_txt, total_n, needs_n
            )
            .into()
        } else {
            format!("You were away {} · {} updates", when_txt, total_n).into()
        };

        let divider_inner = div()
            .h(px(28.0))
            .pl(px(10.0))
            .pr(px(8.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .text_size(px(12.0))
            .text_color(p.muted)
            .child(div().size(px(6.0)).rounded_full().bg(divider_dot_c))
            .child(div().truncate().child(divider_text.clone()))
            .child(pro_icon(ProIcon::ChevronDown, 12.0, p.faint));

        let divider_el = div()
            .w_full()
            .py(px(2.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(div().flex_1().h(px(1.0)).bg(p.line_strong))
            .child(
                pro_tactile(
                    pro_eid(&self.id, "divider-open-btn"),
                    divider_text,
                    999.0,
                    transparent_black(),
                    p.chip,
                    transparent_black(),
                    transparent_black(),
                    false,
                    divider_inner,
                    cx,
                    |this, _, cx| this.set_collapsed(false, false, cx),
                )
                .flex_none(),
            )
            .child(div().flex_1().h(px(1.0)).bg(p.line_strong));

        // 2. Expanded summary card — pre-sample done_clocks for completed rows
        let mut done_draws_map: HashMap<SharedString, bool> = HashMap::new();
        for item in &self.items {
            if item.kind == AwayItemKind::Done {
                let drawn = if reduced {
                    true
                } else if let Some(clk) = self.done_clocks.get(&item.id) {
                    clk.time(0.28, w, cx) > 0.012
                } else {
                    true
                };
                done_draws_map.insert(item.id.clone(), drawn);
            }
        }

        let visible_slice: Vec<AwayItem> = self.items.iter().take(self.limit).cloned().collect();
        let extra_slice: Vec<AwayItem> = self.items.iter().skip(self.limit).cloned().collect();
        let extra_len = extra_slice.len();

        let mut primary_rows = div().w_full().flex().flex_col();
        for item in &visible_slice {
            let draw_ok = done_draws_map.get(&item.id).copied().unwrap_or(true);
            primary_rows = primary_rows.child(self.render_row(item, halo_alpha, draw_ok, cx));
        }

        let mut extra_rows = div().w_full().flex().flex_col();
        for item in &extra_slice {
            let draw_ok = done_draws_map.get(&item.id).copied().unwrap_or(true);
            extra_rows = extra_rows.child(self.render_row(item, halo_alpha, draw_ok, cx));
        }

        let show_all = self.show_all;
        let more_btn = (extra_len > 0).then(|| {
            let lbl: SharedString = if show_all {
                tr(cx, "Show less", "收起").into()
            } else if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("展开其余 {extra_len} 项").into()
            } else {
                format!("Show {extra_len} more").into()
            };
            let more_inner = div()
                .h(px(28.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(pro_swap_label(
                    pro_eid(&self.id, "show-more-swap"),
                    lbl.clone(),
                    12.0,
                    FontWeight::MEDIUM,
                    p.text,
                ))
                .child(pro_icon(
                    if show_all {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    12.0,
                    p.faint,
                ));
            pro_tactile(
                pro_eid(&self.id, "show-more-btn"),
                lbl,
                6.0,
                transparent_black(),
                p.chip,
                transparent_black(),
                transparent_black(),
                false,
                more_inner,
                cx,
                |this, _, cx| {
                    this.show_all = !this.show_all;
                    cx.emit(AwaySummaryEvent::ShowMoreToggled(this.show_all));
                    cx.notify();
                },
            )
            .flex_none()
        });

        let got_it_btn = pro_btn(
            pro_eid(&self.id, "got-it-btn"),
            tr(cx, "Got it", "知道了"),
            Some(ProIcon::Check),
            ProBtnStyle::Ghost,
            true,
            cx,
            |this, _, cx| this.set_collapsed(true, true, cx),
        );

        // Header counts strip with rolling numbers
        let mut counts_row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.0))
            .text_size(px(12.0))
            .text_color(p.muted);
        let mut pushed_any = false;
        if needs_n > 0 {
            pushed_any = true;
            counts_row = counts_row.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(div().size(px(6.0)).rounded_full().bg(p.amber))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "cnt-needs"),
                        needs_n as f64,
                        0,
                        "",
                        "",
                        12.0,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))
                    .child(tr(cx, "needs you", "待确认")),
            );
        }
        if failed_n > 0 {
            if pushed_any {
                counts_row = counts_row.child(div().text_color(p.faint).child("·"));
            }
            pushed_any = true;
            counts_row = counts_row.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(div().size(px(6.0)).rounded_full().bg(p.red))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "cnt-failed"),
                        failed_n as f64,
                        0,
                        "",
                        "",
                        12.0,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))
                    .child(tr(cx, "failed", "失败")),
            );
        }
        if done_n > 0 {
            if pushed_any {
                counts_row = counts_row.child(div().text_color(p.faint).child("·"));
            }
            counts_row = counts_row.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(div().size(px(6.0)).rounded_full().bg(p.blue))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "cnt-done"),
                        done_n as f64,
                        0,
                        "",
                        "",
                        12.0,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))
                    .child(tr(cx, "done", "已完成")),
            );
        }

        let expanded_card = div()
            .w_full()
            .rounded(px(12.0))
            .bg(card_bg)
            .border_1()
            .border_color(p.line)
            .when(!p.dark, |d| d.shadow_xs())
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .px(px(14.0))
                    .py(px(10.0))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .when(narrow, |d| d.flex_col().items_start().gap(px(4.0)))
                    .when(!narrow, |d| {
                        d.items_center().justify_between().gap(px(12.0))
                    })
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(7.0))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "While you were away", "你离开期间的进展")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(p.muted)
                                    .child(when_txt),
                            ),
                    )
                    .child(counts_row),
            )
            .child(
                div()
                    .w_full()
                    .p(px(4.0))
                    .flex()
                    .flex_col()
                    .child(primary_rows)
                    .when(extra_len > 0, |d| {
                        d.child(pro_fold(
                            pro_eid(&self.id, "extra-rows-fold"),
                            show_all,
                            (extra_len as f32) * 48.0,
                            0.0,
                            move || extra_rows.into_any_element(),
                        ))
                    }),
            )
            .child(
                div()
                    .w_full()
                    .px(px(6.0))
                    .py(px(5.0))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(match more_btn {
                        Some(b) => b.into_any_element(),
                        None => div().into_any_element(),
                    })
                    .child(got_it_btn),
            );

        let is_collapsed = self.collapsed;
        let summary_region = div()
            .w_full()
            .flex()
            .flex_col()
            .child(pro_fold(
                pro_eid(&self.id, "div-fold"),
                is_collapsed,
                32.0,
                0.0,
                move || divider_el.into_any_element(),
            ))
            .child(pro_fold(
                pro_eid(&self.id, "card-fold"),
                !is_collapsed,
                326.0,
                0.0,
                move || expanded_card.into_any_element(),
            ));

        // Optional surrounding thread messages & composer matching the reference stage
        let mut thread_col = div()
            .w_full()
            .max_w(px(600.0))
            .flex()
            .flex_col()
            .gap(px(14.0));

        if self.thread_chrome {
            let user_bubble_bg: Hsla = if p.dark {
                hsla(0.0, 0.0, 1.0, 0.07)
            } else {
                rgb(0xf1f1f2).into()
            };
            thread_col = thread_col
                .child(
                    div().w_full().flex().justify_end().child(
                        div()
                            .min_w_0()
                            .max_w(relative(0.88))
                            .px(px(14.0))
                            .py(px(8.0))
                            .rounded(px(16.0))
                            .bg(user_bubble_bg)
                            .text_size(px(13.0))
                            .line_height(px(20.0))
                            .text_color(p.ink)
                            .child(tr(
                                cx,
                                "Get ingest onto staging today – ping me if you need anything.",
                                "今天把数据采集流水线推上预发环境——有需要确认的随时喊我。",
                            )),
                    ),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(pro_agent_avatar("orchestrator", p.purple, 20.0, false, 0.0, cx))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(13.0))
                                .text_color(p.ink)
                                .child(tr(
                                    cx,
                                    "On it. I’ll split it into tasks and keep going while you’re out.",
                                    "收到，我会拆分为子任务并在你离开期间持续推进。",
                                )),
                        ),
                );
        }

        thread_col = thread_col.child(summary_region);

        if self.thread_chrome {
            for (r_idx, r_msg) in self.replies.iter().enumerate() {
                let user_bubble_bg: Hsla = if p.dark {
                    hsla(0.0, 0.0, 1.0, 0.07)
                } else {
                    rgb(0xf1f1f2).into()
                };
                let (dy, alpha) = if reduced {
                    (0.0, 1.0)
                } else if let Some((sl, clk)) = self.reply_entrances.get(r_idx) {
                    let y = sl.get(w, cx);
                    let age = clk.time(0.22, w, cx);
                    let a = 0.25 + 0.75 * pro_ease((age / 0.22).clamp(0.0, 1.0));
                    (y, a)
                } else {
                    (0.0, 1.0)
                };
                thread_col = thread_col.child(
                    div()
                        .w_full()
                        .relative()
                        .top(px(dy))
                        .opacity(alpha)
                        .flex()
                        .justify_end()
                        .child(
                            div()
                                .min_w_0()
                                .max_w(relative(0.88))
                                .px(px(14.0))
                                .py(px(8.0))
                                .rounded(px(16.0))
                                .bg(user_bubble_bg)
                                .text_size(px(13.0))
                                .line_height(px(20.0))
                                .text_color(p.ink)
                                .child(r_msg.clone()),
                        ),
                );
            }

            let composer_bg: Hsla = if p.dark {
                rgb(0x19191b).into()
            } else {
                rgb(0xffffff).into()
            };
            let ed_view = self.reply_editor.clone();
            let send_lbl = tr(cx, "Send reply", "发送回复");
            let send_hov = self.send_hover.get(w, cx).clamp(0.0, 1.0);
            let send_down = self.send_press.get(w, cx).clamp(0.0, 1.0);
            let send_bg = pro_mix_color(
                p.primary,
                pro_mix_color(p.primary, p.on_primary, 0.14),
                send_hov,
            );
            let send_inset = 1.4 * send_down;
            thread_col = thread_col.child(
                div()
                    .w_full()
                    .h(px(46.0))
                    .pl(px(14.0))
                    .pr(px(9.0))
                    .rounded(px(14.0))
                    .bg(composer_bg)
                    .border_1()
                    .border_color(if self.editor_focused {
                        p.focus
                    } else {
                        p.line_strong
                    })
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h(px(22.0))
                            .when_some(ed_view, |d, ed| d.child(ed)),
                    )
                    .child(
                        div()
                            .id(pro_eid(&self.id, "thread-send-btn"))
                            .role(Role::Button)
                            .aria_label(send_lbl)
                            .tab_index(0)
                            .size(px(28.0))
                            .flex_none()
                            .relative()
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .on_hover(cx.listener(|this, inside: &bool, w, cx| {
                                this.send_hover.set(if *inside { 1.0 } else { 0.0 }, cx);
                                if !*inside {
                                    this.send_press.set(0.0, cx);
                                }
                                cx.notify();
                                w.refresh();
                            }))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, w, cx| {
                                    this.send_press.set(1.0, cx);
                                    cx.notify();
                                    w.refresh();
                                }),
                            )
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, w, cx| {
                                    this.send_press.set(0.0, cx);
                                    cx.notify();
                                    w.refresh();
                                }),
                            )
                            .on_mouse_up_out(
                                MouseButton::Left,
                                cx.listener(|this, _, w, cx| {
                                    this.send_press.set(0.0, cx);
                                    cx.notify();
                                    w.refresh();
                                }),
                            )
                            .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                                let val = this
                                    .reply_editor
                                    .as_ref()
                                    .map(|ed| ed.read(cx).value().trim().to_owned())
                                    .unwrap_or_default();
                                if !val.is_empty() {
                                    if let Some(ed) = &this.reply_editor {
                                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                                    }
                                    this.push_reply(val.into(), cx);
                                }
                            }))
                            .child(
                                canvas(
                                    |_, _, _| (),
                                    move |bounds, _, window, _| {
                                        let bw = f32::from(bounds.size.width);
                                        let bh = f32::from(bounds.size.height);
                                        let rect = Bounds::new(
                                            point(
                                                bounds.origin.x + px(send_inset),
                                                bounds.origin.y + px(send_inset),
                                            ),
                                            size(
                                                px((bw - 2.0 * send_inset).max(2.0)),
                                                px((bh - 2.0 * send_inset).max(2.0)),
                                            ),
                                        );
                                        window.paint_quad(
                                            fill(rect, send_bg).corner_radii(rect.size.height / 2.),
                                        );
                                    },
                                )
                                .absolute()
                                .inset_0(),
                            )
                            .child(div().relative().child(pro_icon(
                                ProIcon::ArrowUp,
                                14.0 * (1.0 - 0.08 * send_down),
                                p.on_primary,
                            ))),
                    ),
            );
        }

        let width_sensor = self.measured_width.clone();
        let a11y_label = tr(cx, "While you were away summary", "离开期间进展汇总");
        let focus_ring = p.focus;

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(a11y_label)
            .track_focus(&self.focus)
            .tab_index(0)
            .relative()
            .w_full()
            .py(px(22.0))
            .px(px(16.0))
            .flex()
            .flex_col()
            .items_center()
            .border_1()
            .border_color(transparent_black())
            .focus_visible(move |s| s.rounded(px(14.0)).border_color(focus_ring))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if !this.editor_focused && ev.keystroke.key.as_str() == "escape" && !this.collapsed
                {
                    this.set_collapsed(true, true, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .on_children_prepainted(move |bounds, _, _| {
                        if let Some(b) = bounds.first() {
                            let w_px = f32::from(b.size.width);
                            if w_px > 20.0 {
                                width_sensor.set(w_px);
                            }
                        }
                    })
                    .child(thread_col),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{AwayItem, AwayItemKind, AwaySummary, AwayTimeStyle};
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn away_summary_sorts_by_priority_resolves_in_place_and_folds(cx: &mut TestAppContext) {
        let view = cx.new(|cx| {
            AwaySummary::new("test-away-summary", cx)
                .with_time_style(AwayTimeStyle::Duration)
                .with_since_duration("1h 20m")
        });

        view.update(cx, |s, cx| {
            assert_eq!(s.counts(), (1, 1, 6));
            assert!(!s.is_collapsed());

            s.trigger_action("weekly-report", cx);
            assert_eq!(s.counts(), (0, 1, 7));

            s.trigger_action("billing-sync", cx);
            assert_eq!(s.counts(), (0, 0, 8));

            s.set_collapsed(true, true, cx);
            assert!(s.is_collapsed());
            s.set_collapsed(false, false, cx);
            assert!(!s.is_collapsed());
        });

        let custom = AwayItem::new("task-1", "Sync docs", AwayItemKind::Done).with_detail("Done");
        assert_eq!(custom.kind, AwayItemKind::Done);
    }
}
