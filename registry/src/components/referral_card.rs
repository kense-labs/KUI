//! A referral program with an invite composer, conversion funnel, and milestone.
use crate::{
    foundation::surface_radius,
    primitives::{
        clipboard,
        currency::format_money,
        editor::{EditEvent, Editor, InputMode},
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, ease, face, measure},
        glyph::{Glyph, icon, spinner},
        menu::MenuScrollRegion,
        nav::HorizontalScrollRegion,
        presentation::{column, row, tr},
    },
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc, time::Duration};

const MAX_WIDTH: f32 = 560.;
const NARROW: Pixels = px(470.);
const LIST_HEIGHT: f32 = 182.;
const ROW_HEIGHT: f32 = 52.;
const COUNT_TIME: f32 = 0.6;
const LABEL_IN: f32 = 0.16;
const LABEL_OUT: f32 = 0.10;

/// A friend's position in the referral funnel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferralStage {
    #[default]
    Invited,
    SignedUp,
    Earned,
}

/// One referral with a stable key and an ISO date or timestamp.
#[derive(Clone)]
pub struct Referral {
    pub id: SharedString,
    pub name: Option<SharedString>,
    pub email: SharedString,
    pub avatar: Option<ImageSource>,
    pub stage: ReferralStage,
    pub invited_at: SharedString,
    pub accent: Option<[u32; 2]>,
}

impl Referral {
    pub fn new(
        id: impl Into<SharedString>,
        email: impl Into<SharedString>,
        stage: ReferralStage,
        invited_at: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: None,
            email: email.into(),
            avatar: None,
            stage,
            invited_at: invited_at.into(),
            accent: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_avatar(mut self, image: impl Into<ImageSource>) -> Self {
        self.avatar = Some(image.into());
        self
    }

    pub fn with_accent(mut self, colors: [u32; 2]) -> Self {
        self.accent = Some(colors);
        self
    }
}

/// A bonus unlocked by paid referrals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReferralMilestone {
    pub goal: u32,
    pub reward: SharedString,
    pub detail: Option<SharedString>,
}

impl ReferralMilestone {
    pub fn new(goal: u32, reward: impl Into<SharedString>) -> Self {
        Self {
            goal,
            reward: reward.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Content and behavior of a referral program. Monetary values are whole currency units.
#[derive(Clone)]
pub struct ReferralCardOptions {
    pub link: SharedString,
    pub reward: u32,
    pub currency: SharedString,
    pub title: Option<SharedString>,
    pub description: Option<SharedString>,
    pub referrals: Option<Vec<Referral>>,
    pub milestone: ReferralMilestone,
    pub share_message: Option<SharedString>,
    pub max_invites: usize,
    pub simulate: Option<bool>,
}

impl Default for ReferralCardOptions {
    fn default() -> Self {
        Self {
            link: "https://northwind.app/r/emma20".into(),
            reward: 20,
            currency: "USD".into(),
            title: None,
            description: None,
            referrals: None,
            milestone: ReferralMilestone::new(5, "3 months free")
                .with_detail("Applied to your next invoice"),
            share_message: None,
            max_invites: 10,
            simulate: None,
        }
    }
}

/// An external sharing destination. Opening the destination is the host's responsibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferralShareChannel {
    X,
    LinkedIn,
    Email,
}

impl ReferralShareChannel {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::X => tr(cx, "Post on X", "发布到 X"),
            Self::LinkedIn => tr(cx, "Share on LinkedIn", "分享到 LinkedIn"),
            Self::Email => tr(cx, "Share by email", "通过邮件分享"),
        }
    }
}

/// Encodes a share destination without opening it.
pub fn referral_share_url(channel: ReferralShareChannel, link: &str, message: &str) -> String {
    let encode = |s: &str| {
        use std::fmt::Write;
        let mut out = String::new();
        for b in s.bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                out.push(char::from(b));
            } else {
                let _ = write!(out, "%{b:02X}");
            }
        }
        out
    };
    let body = if message.is_empty() {
        link.to_owned()
    } else {
        format!("{message}\n\n{link}")
    };
    match channel {
        ReferralShareChannel::X => format!(
            "https://twitter.com/intent/tweet?text={}&url={}",
            encode(message),
            encode(link)
        ),
        ReferralShareChannel::LinkedIn => format!(
            "https://www.linkedin.com/sharing/share-offsite/?url={}",
            encode(link)
        ),
        ReferralShareChannel::Email => format!(
            "mailto:?subject={}&body={}",
            encode(message.lines().next().unwrap_or("")),
            encode(&body)
        ),
    }
}

/// Indicates that a host invite operation has been cancelled or its view has closed.
#[derive(Clone, Default)]
pub struct ReferralInviteSignal(Rc<Cell<bool>>);

impl ReferralInviteSignal {
    pub fn is_cancelled(&self) -> bool {
        self.0.get()
    }
}

/// The addresses sent together in one request.
#[derive(Clone)]
pub struct ReferralInvite {
    pub emails: Vec<SharedString>,
    pub signal: ReferralInviteSignal,
}

struct PendingInvite {
    signal: Option<ReferralInviteSignal>,
    _task: Task<()>,
}

impl Drop for PendingInvite {
    fn drop(&mut self) {
        if let Some(signal) = &self.signal {
            signal.0.set(true);
        }
    }
}

type InviteHandler =
    Box<dyn Fn(ReferralInvite, &mut Window, &mut App) -> Task<Result<(), SharedString>>>;
type CopyHandler = Box<dyn Fn(SharedString, &mut Window, &mut App)>;
type ShareHandler = Box<dyn Fn(ReferralShareChannel, SharedString, &mut Window, &mut App)>;
type MilestoneHandler = Box<dyn Fn(&ReferralMilestone, &mut App)>;
type Visibility = Box<dyn Fn(&App) -> bool>;
type Style = Box<dyn Fn(Div) -> Div>;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum CopyState {
    #[default]
    Ready,
    Writing,
    Copied,
    Failed,
}

#[derive(Clone, Copy)]
struct Counter {
    from: f64,
    target: u64,
    clock: PhaseClock,
}

impl Counter {
    fn new(value: u64) -> Self {
        Self {
            from: value as f64,
            target: value,
            clock: PhaseClock::default(),
        }
    }

    fn sample(&self, age: f32) -> f64 {
        self.from + (self.target as f64 - self.from) * f64::from(ease(age / COUNT_TIME))
    }

    fn set(&mut self, value: u64, cx: &App) {
        if self.target != value {
            self.from = if disabled_motion(cx) {
                self.target as f64
            } else {
                self.sample(self.clock.age(cx))
            };
            self.target = value;
            self.clock.restart(cx);
        }
    }
}

struct StageTransition {
    previous: ReferralStage,
    clock: PhaseClock,
}

struct LabelTransition {
    current: SharedString,
    previous: SharedString,
    clock: PhaseClock,
}

impl LabelTransition {
    fn new() -> Self {
        Self {
            current: "".into(),
            previous: "".into(),
            clock: PhaseClock::default(),
        }
    }

    fn set(&mut self, text: SharedString, cx: &App) {
        if self.current != text {
            self.previous = std::mem::replace(&mut self.current, text);
            self.clock.restart(cx);
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
struct Measurement {
    width: Pixels,
    visible: bool,
}

/// A controlled referral card with an optional, self-contained demonstration.
pub struct ReferralCard {
    id: ElementId,
    options: ReferralCardOptions,
    data: Vec<Referral>,
    simulate: bool,
    editor: Entity<Editor>,
    _subscription: Subscription,
    chips: Vec<SharedString>,
    draft_len: usize,
    draft_error: SharedString,
    hint: LabelTransition,
    hint_reset: Option<Task<()>>,
    copy: CopyState,
    previous_copy: CopyState,
    copied_at: PhaseClock,
    copy_task: Option<Task<()>>,
    pending: Option<PendingInvite>,
    pending_clock: PhaseClock,
    next_id: u64,
    list_scroll: ScrollHandle,
    chip_scroll: ScrollHandle,
    counters: [Counter; 3],
    progress: Slide,
    success: Slide,
    milestone_label: LabelTransition,
    milestone_fired: bool,
    stages: HashMap<SharedString, StageTransition>,
    entrances: HashMap<SharedString, Slide>,
    started: bool,
    scenario: Option<Task<()>>,
    measurement: Rc<Cell<Measurement>>,
    visibility: Option<Visibility>,
    style: Option<Style>,
    on_invite: Option<InviteHandler>,
    on_copy: Option<CopyHandler>,
    on_share: Option<ShareHandler>,
    on_milestone: Option<MilestoneHandler>,
}

impl ReferralCard {
    pub fn new(options: ReferralCardOptions, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            let label = tr(cx, "Invite by email", "通过邮件邀请");
            let mut editor = Editor::new(label, InputMode::Single, w, cx);
            editor.set_placeholder(label);
            editor.description =
                tr(cx, "Press Enter after each address", "每个地址输入后按回车").into();
            editor
        });
        let subscription =
            cx.subscribe_in(&editor, w, |s, _, event: &EditEvent, w, cx| match event {
                EditEvent::Submitted => {
                    if s.editor.read(cx).value().trim().is_empty() {
                        s.send(w, cx);
                    } else {
                        s.commit_draft(w, cx);
                    }
                }
                EditEvent::Changed => s.draft_changed(w, cx),
                _ => {}
            });
        let simulate = options.simulate.unwrap_or(options.referrals.is_none());
        let data = options.referrals.clone().unwrap_or_else(|| sample(cx));
        let amounts = counts(&data, options.reward);
        let earned = paid(&data);
        let unlocked = earned >= u64::from(options.milestone.goal.max(1));
        Self {
            id: "referral-card".into(),
            options,
            data,
            simulate,
            editor,
            _subscription: subscription,
            chips: vec![],
            draft_len: 0,
            draft_error: "".into(),
            hint: LabelTransition::new(),
            hint_reset: None,
            copy: CopyState::Ready,
            previous_copy: CopyState::Ready,
            copied_at: PhaseClock::default(),
            copy_task: None,
            pending: None,
            pending_clock: PhaseClock::default(),
            next_id: 0,
            list_scroll: ScrollHandle::new(),
            chip_scroll: ScrollHandle::new(),
            counters: amounts.map(Counter::new),
            progress: Slide::new(earned as f32),
            success: Slide::new(f32::from(u8::from(unlocked))),
            milestone_label: LabelTransition::new(),
            milestone_fired: false,
            stages: HashMap::new(),
            entrances: HashMap::new(),
            started: false,
            scenario: None,
            measurement: Rc::default(),
            visibility: None,
            style: None,
            on_invite: None,
            on_copy: None,
            on_share: None,
            on_milestone: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    /// Lets an embedding host report whether the region is visible.
    pub fn with_visibility(mut self, visible: impl Fn(&App) -> bool + 'static) -> Self {
        self.visibility = Some(Box::new(visible));
        self
    }

    pub fn on_invite(
        mut self,
        handler: impl Fn(ReferralInvite, &mut Window, &mut App) -> Task<Result<(), SharedString>>
        + 'static,
    ) -> Self {
        self.on_invite = Some(Box::new(handler));
        self
    }

    pub fn on_copy(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_copy = Some(Box::new(handler));
        self
    }

    pub fn on_share(
        mut self,
        handler: impl Fn(ReferralShareChannel, SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_share = Some(Box::new(handler));
        self
    }

    pub fn on_milestone(
        mut self,
        handler: impl Fn(&ReferralMilestone, &mut App) + 'static,
    ) -> Self {
        self.on_milestone = Some(Box::new(handler));
        self
    }

    pub fn referrals(&self) -> &[Referral] {
        &self.data
    }

    /// Applies the host's most recent referral snapshot in newest-first order.
    pub fn set_referrals(&mut self, referrals: Vec<Referral>, cx: &mut Context<Self>) {
        for next in &referrals {
            match self.data.iter().find(|old| old.id == next.id) {
                Some(old) if old.stage != next.stage => {
                    let mut clock = PhaseClock::default();
                    clock.restart(cx);
                    self.stages.insert(
                        next.id.clone(),
                        StageTransition {
                            previous: old.stage,
                            clock,
                        },
                    );
                }
                None => {
                    let mut slide = Slide::new(0.);
                    slide.duration(ROW_HEIGHT, 0.24, cx);
                    self.entrances.insert(next.id.clone(), slide);
                }
                _ => {}
            }
        }
        self.stages
            .retain(|id, _| referrals.iter().any(|r| &r.id == id));
        self.entrances
            .retain(|id, _| referrals.iter().any(|r| &r.id == id));
        self.data = referrals;
        self.update_counts(cx);
        cx.notify();
    }

    /// Cancels the active request and leaves the addresses available for retry.
    pub fn cancel_invite(&mut self, w: &Window, cx: &mut Context<Self>) {
        if self.pending.take().is_some() {
            self.editor
                .update(cx, |e, cx| e.set_read_only(false, w, cx));
            cx.notify();
        }
    }

    /// Restarts the built-in sample when using the simulated program.
    pub fn replay(&mut self, w: &Window, cx: &mut Context<Self>) {
        if !self.simulate {
            return;
        }
        self.cancel_invite(w, cx);
        self.scenario = None;
        self.started = false;
        self.chips.clear();
        self.editor.update(cx, |e, cx| e.set_value("", w, cx));
        self.draft_error = "".into();
        self.hint_reset = None;
        self.hint.set("".into(), cx);
        self.stages.clear();
        self.entrances.clear();
        self.data = self.options.referrals.clone().unwrap_or_else(|| sample(cx));
        self.counters = counts(&self.data, self.options.reward).map(Counter::new);
        self.progress.reset(paid(&self.data) as f32);
        self.success.reset(if self.unlocked() { 1. } else { 0. });
        self.list_scroll.set_offset(Point::default());
        cx.notify();
    }

    fn unlocked(&self) -> bool {
        paid(&self.data) >= u64::from(self.options.milestone.goal.max(1))
    }

    fn update_counts(&mut self, cx: &mut Context<Self>) {
        for (counter, target) in self
            .counters
            .iter_mut()
            .zip(counts(&self.data, self.options.reward))
        {
            counter.set(target, cx);
        }
        self.progress.duration(paid(&self.data) as f32, 0.24, cx);
        let unlocked = self.unlocked();
        self.success
            .duration(f32::from(u8::from(unlocked)), 0.24, cx);
        if unlocked && !self.milestone_fired {
            self.milestone_fired = true;
            if let Some(handler) = &self.on_milestone {
                handler(&self.options.milestone, cx);
            }
        }
    }

    fn set_stage(&mut self, id: &str, stage: ReferralStage, cx: &mut Context<Self>) {
        let Some(referral) = self.data.iter_mut().find(|r| r.id == id) else {
            return;
        };
        if referral.stage >= stage {
            return;
        }
        let previous = std::mem::replace(&mut referral.stage, stage);
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        self.stages
            .insert(referral.id.clone(), StageTransition { previous, clock });
        self.update_counts(cx);
        cx.notify();
    }

    fn play_sample(&mut self, cx: &mut Context<Self>) {
        if self.started || !self.simulate {
            return;
        }
        self.started = true;
        let first = self
            .data
            .iter()
            .find(|r| r.stage == ReferralStage::Invited)
            .map(|r| r.id.clone());
        let signed_up = self
            .data
            .iter()
            .find(|r| r.stage == ReferralStage::SignedUp)
            .map(|r| r.id.clone());
        let reduced = disabled_motion(cx);
        self.scenario = Some(cx.spawn(async move |weak, cx| {
            for (delay, id, stage) in [
                (1000, signed_up, ReferralStage::Earned),
                (1560, first.clone(), ReferralStage::SignedUp),
                (1450, first, ReferralStage::Earned),
            ] {
                cx.background_executor()
                    .timer(Duration::from_millis(if reduced {
                        delay / 2
                    } else {
                        delay
                    }))
                    .await;
                if let Some(id) = id {
                    let _ = weak.update(cx, |s, cx| s.set_stage(&id, stage, cx));
                }
            }
        }));
    }

    fn draft_changed(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);
        if self.pending.is_some() || editor.is_composing() {
            return;
        }
        let value = editor.value();
        let pasted_address = value.len() > self.draft_len + 1 && valid_email(value);
        self.draft_len = value.len();
        if !value.is_empty() && (pasted_address || value.chars().any(separator)) {
            self.commit_draft(w, cx);
        }
        cx.notify();
    }

    fn commit_draft(&mut self, w: &Window, cx: &mut Context<Self>) -> bool {
        if self.pending.is_some() || self.editor.read(cx).is_composing() {
            return false;
        }
        let raw = self.editor.read(cx).value().to_owned();
        if raw.is_empty() {
            return true;
        }
        let mut invalid = Vec::new();
        let mut problem: SharedString = "".into();
        for address in raw.split(separator).filter(|s| !s.is_empty()) {
            if !valid_email(address) {
                invalid.push(address);
                problem = if tr(cx, "en", "zh") == "zh" {
                    format!("{address} 不是完整的邮箱地址").into()
                } else {
                    format!("{address} is not a full email address").into()
                };
            } else if self.chips.iter().any(|s| s.eq_ignore_ascii_case(address)) {
                continue;
            } else if self.chips.len() >= self.options.max_invites {
                invalid.push(address);
                problem = if tr(cx, "en", "zh") == "zh" {
                    format!("每次最多邀请 {} 人", self.options.max_invites).into()
                } else {
                    format!(
                        "You can send up to {} invites at a time",
                        self.options.max_invites
                    )
                    .into()
                };
            } else {
                self.chips.push(address.to_owned().into());
            }
        }
        self.draft_error = problem;
        let remainder = invalid.join(" ");
        self.draft_len = remainder.len();
        self.editor
            .update(cx, |e, cx| e.set_value(remainder, w, cx));
        self.chip_scroll.scroll_to_item(self.chips.len());
        cx.notify();
        invalid.is_empty()
    }

    fn remove(&mut self, email: &str, w: &mut Window, cx: &mut Context<Self>) {
        if self.pending.is_none() {
            self.chips.retain(|s| s != email);
            self.draft_error = "".into();
            self.editor.read(cx).focus.clone().focus(w, cx);
            cx.notify();
        }
    }

    fn send(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.pending.is_some() || !self.commit_draft(w, cx) || self.chips.is_empty() {
            return;
        }
        let signal = ReferralInviteSignal::default();
        let request = ReferralInvite {
            emails: self.chips.clone(),
            signal: signal.clone(),
        };
        let emails = request.emails.clone();
        let simulated = self.on_invite.is_none();
        let task = if let Some(handler) = &self.on_invite {
            handler(request, w, cx)
        } else {
            let delay = cx.background_executor().timer(Duration::from_millis(900));
            cx.spawn(async move |_, _| {
                delay.await;
                Ok(())
            })
        };
        self.pending_clock.restart(cx);
        self.hint_reset = None;
        self.hint.set("".into(), cx);
        self.editor.update(cx, |e, cx| e.set_read_only(true, w, cx));
        let check = signal.clone();
        let complete = cx.spawn_in(w, async move |weak, cx| {
            let result = task.await;
            if !check.is_cancelled() {
                let _ = weak.update_in(cx, |s, w, cx| {
                    s.finish_invite(emails, simulated, result, w, cx)
                });
            }
        });
        self.pending = Some(PendingInvite {
            signal: Some(signal),
            _task: complete,
        });
        cx.notify();
    }

    fn finish_invite(
        &mut self,
        emails: Vec<SharedString>,
        simulated: bool,
        result: Result<(), SharedString>,
        w: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mut pending) = self.pending.take() else {
            return;
        };
        pending.signal = None;
        self.editor
            .update(cx, |e, cx| e.set_read_only(false, w, cx));
        if let Err(error) = result {
            self.draft_error = if error.is_empty() {
                tr(
                    cx,
                    "Could not send the invites. Try again.",
                    "邀请发送失败，请重试。",
                )
                .into()
            } else {
                error
            };
            cx.notify();
            return;
        }
        self.draft_error = "".into();
        self.chips.clear();
        let count = emails.len();
        let now = Utc::now().to_rfc3339();
        let mut referrals = Vec::new();
        for email in emails {
            if self
                .data
                .iter()
                .any(|r| r.email.eq_ignore_ascii_case(&email))
            {
                continue;
            }
            let id = loop {
                self.next_id += 1;
                let id: SharedString = format!("invite-{}", self.next_id).into();
                if !self.data.iter().any(|r| r.id == id) {
                    break id;
                }
            };
            referrals.push(Referral::new(
                id,
                email,
                ReferralStage::Invited,
                now.clone(),
            ));
        }
        let first = referrals.first().map(|r| r.id.clone());
        referrals.extend(self.data.iter().cloned());
        self.set_referrals(referrals, cx);
        self.list_scroll.set_offset(Point::default());
        let message = if tr(cx, "en", "zh") == "zh" {
            if simulated {
                format!("已模拟发送 {count} 封邀请。")
            } else {
                format!("已发送 {count} 封邀请。")
            }
        } else {
            let word = if count == 1 { "invite" } else { "invites" };
            if simulated {
                format!("Sent {count} {word}. Simulated, nothing was sent.")
            } else {
                format!("Sent {count} {word}.")
            }
        };
        self.notice(message.into(), 4600, cx);
        if simulated
            && self.simulate
            && let Some(id) = first
        {
            let reduced = disabled_motion(cx);
            cx.spawn(async move |weak, cx| {
                for (delay, stage) in [
                    (2450, ReferralStage::SignedUp),
                    (1700, ReferralStage::Earned),
                ] {
                    cx.background_executor()
                        .timer(Duration::from_millis(if reduced {
                            delay / 2
                        } else {
                            delay
                        }))
                        .await;
                    let _ = weak.update(cx, |s, cx| s.set_stage(&id, stage, cx));
                }
            })
            .detach();
        }
    }

    fn notice(&mut self, text: SharedString, duration: u64, cx: &mut Context<Self>) {
        self.hint.set(text, cx);
        self.hint_reset = Some(cx.spawn(async move |weak, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(duration))
                .await;
            let _ = weak.update(cx, |s, cx| {
                s.hint.set("".into(), cx);
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn set_copy(&mut self, state: CopyState, cx: &App) {
        self.previous_copy = std::mem::replace(&mut self.copy, state);
        self.copied_at.restart(cx);
    }

    fn copy_link(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.copy == CopyState::Writing {
            return;
        }
        self.set_copy(CopyState::Writing, cx);
        let link = self.options.link.clone();
        let task = clipboard::write(&link, cx);
        self.copy_task = Some(cx.spawn_in(w, async move |weak, cx| {
            let ok = task.await;
            let _ = weak.update_in(cx, |s, w, cx| {
                s.set_copy(
                    if ok {
                        CopyState::Copied
                    } else {
                        CopyState::Failed
                    },
                    cx,
                );
                if ok && let Some(handler) = &s.on_copy {
                    handler(link, w, cx);
                }
                cx.notify();
            });
            cx.background_executor()
                .timer(Duration::from_millis(1700))
                .await;
            let _ = weak.update(cx, |s, cx| {
                s.set_copy(CopyState::Ready, cx);
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn share(&mut self, channel: ReferralShareChannel, w: &mut Window, cx: &mut Context<Self>) {
        let title = self.title(cx);
        let message = self.options.share_message.as_deref().unwrap_or(&title);
        let url = referral_share_url(channel, &self.options.link, message);
        if let Some(handler) = &self.on_share {
            handler(channel, url.into(), w, cx);
        } else {
            self.notice(
                tr(
                    cx,
                    "Simulated, nothing was posted.",
                    "仅模拟分享，未发布内容。",
                )
                .into(),
                2600,
                cx,
            );
        }
    }

    fn title(&self, cx: &App) -> SharedString {
        self.options.title.clone().unwrap_or_else(|| {
            let reward = credit(u64::from(self.options.reward), &self.options.currency);
            if tr(cx, "en", "zh") == "zh" {
                format!("送 {reward}，得 {reward}")
            } else {
                format!("Give {reward}, get {reward}")
            }
            .into()
        })
    }

    fn description(&self, cx: &App) -> SharedString {
        self.options.description.clone().unwrap_or_else(|| {
            let reward = credit(u64::from(self.options.reward), &self.options.currency);
            if tr(cx, "en", "zh") == "zh" {
                format!("好友首月减免 {reward}。每位好友订阅后，你也会获得 {reward} 抵扣额度。")
            } else {
                format!("Friends save {reward} on their first month. Get {reward} in credit for each friend who subscribes.")
            }.into()
        })
    }
}

fn separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, ',' | '，' | ';' | '；')
}

fn valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    let valid_part =
        |s: &str| !s.is_empty() && !s.starts_with('.') && !s.ends_with('.') && !s.contains("..");
    valid_part(local)
        && valid_part(domain)
        && local.len() <= 64
        && email.len() <= 254
        && local
            .chars()
            .all(|c| c.is_alphanumeric() || "!#$%&'*+-/=?^_`{|}~.".contains(c))
        && domain.contains('.')
        && domain.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_alphanumeric() || c == '-')
        })
}

fn paid(data: &[Referral]) -> u64 {
    data.iter()
        .filter(|r| r.stage == ReferralStage::Earned)
        .count() as u64
}

fn counts(data: &[Referral], reward: u32) -> [u64; 3] {
    [
        data.len() as u64,
        data.iter()
            .filter(|r| r.stage >= ReferralStage::SignedUp)
            .count() as u64,
        paid(data).saturating_mul(u64::from(reward)),
    ]
}

fn credit(amount: u64, currency: &str) -> String {
    let formatted = format_money(amount as f64, currency, "en-US");
    if let Some((whole, zeros)) = formatted.rsplit_once('.')
        && zeros.bytes().all(|b| b == b'0')
    {
        whole.to_owned()
    } else {
        formatted
    }
}

fn sample(cx: &App) -> Vec<Referral> {
    use ReferralStage::{Earned, Invited, SignedUp};
    [
        (
            "riley",
            "Riley Morgan",
            "莱利·摩根",
            "2026-10-26",
            Invited,
            Some([0x58e7a4, 0xf666a5]),
        ),
        (
            "avery",
            "Avery Tran",
            "艾弗里·陈",
            "2026-10-23",
            SignedUp,
            Some([0xadf45e, 0x7569d6]),
        ),
        (
            "morgan",
            "Morgan Lee",
            "摩根·李",
            "2026-10-21",
            Invited,
            None,
        ),
        (
            "noah",
            "Noah Kim",
            "诺亚·金",
            "2026-10-17",
            Earned,
            Some([0x98dad0, 0x48d8da]),
        ),
        (
            "jules",
            "Jules Chen",
            "朱尔斯·陈",
            "2026-10-12",
            Earned,
            Some([0x98e4ab, 0xe781db]),
        ),
        (
            "remy",
            "Remy Park",
            "雷米·朴",
            "2026-10-06",
            Earned,
            Some([0x83a8e5, 0xf1a374]),
        ),
    ]
    .into_iter()
    .map(|(id, en, zh, day, stage, colors)| {
        let mut referral =
            Referral::new(id, format!("{id}@example.com"), stage, day).with_name(tr(cx, en, zh));
        referral.accent = colors;
        referral
    })
    .collect()
}

fn line(text: impl Into<SharedString>, size: f32, ink: Hsla) -> Div {
    div()
        .min_w_0()
        .truncate()
        .text_size(px(size))
        .text_color(ink)
        .child(text.into())
}

fn date_label(value: &str, cx: &App) -> SharedString {
    let instant = DateTime::parse_from_rfc3339(value).ok();
    if instant.as_ref().is_some_and(|date| {
        let age = Utc::now()
            .signed_duration_since(date.with_timezone(&Utc))
            .num_seconds();
        (0..60).contains(&age)
    }) {
        return tr(cx, "just now", "刚刚").into();
    }
    let date = instant
        .map(|date| date.date_naive())
        .or_else(|| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
    date.map_or_else(
        || value.to_owned().into(),
        |date| {
            if tr(cx, "en", "zh") == "zh" {
                format!("{}月{}日", date.month(), date.day()).into()
            } else {
                date.format("%b %-d").to_string().into()
            }
        },
    )
}

fn profile_picture(referral: &Referral, p: Paint) -> Div {
    let face = div().size(px(28.)).flex_none().rounded_full().bg(p.stage);
    if let Some(source) = &referral.avatar {
        return face.child(
            img(source.clone())
                .size_full()
                .rounded_full()
                .object_fit(ObjectFit::Cover),
        );
    }
    if let Some([a, b]) = referral.accent {
        return face.bg(linear_gradient(
            135.,
            linear_color_stop(rgb(a), 0.),
            linear_color_stop(rgb(b), 1.),
        ));
    }
    let initials: String = match &referral.name {
        Some(name) => name
            .split_whitespace()
            .filter_map(|s| s.chars().next())
            .take(2)
            .collect(),
        None => referral
            .email
            .chars()
            .take(1)
            .flat_map(char::to_uppercase)
            .collect(),
    };
    face.flex()
        .items_center()
        .justify_center()
        .text_size(px(10.))
        .text_color(p.ink)
        .child(initials)
}

fn roll(counter: Counter, money: Option<&str>, digit_width: f32, w: &Window, cx: &App) -> Div {
    let age = counter.clock.time(COUNT_TIME, w, cx);
    let value = counter.sample(age);
    let up = counter.target as f64 >= counter.from;
    let whole = if up { value.floor() } else { value.ceil() };
    let first = whole as u64;
    let next = if up {
        first.saturating_add(1)
    } else {
        first.saturating_sub(1)
    };
    let fraction = if age >= COUNT_TIME {
        0.
    } else {
        (value - whole).abs() as f32
    };
    let format =
        |number| money.map_or_else(|| u64::to_string(&number), |code| credit(number, code));
    let base = format(first);
    let next = format(next);
    let mut digits = row()
        .h(px(22.))
        .overflow_hidden()
        .text_size(px(18.))
        .line_height(px(22.));
    let direction = if up { 1. } else { -1. };
    for (c, other) in base.chars().zip(next.chars().chain(std::iter::repeat(' '))) {
        let value: SharedString = c.to_string().into();
        if c.is_ascii_digit() {
            digits = digits.child(
                div()
                    .relative()
                    .w(px(digit_width))
                    .h(px(22.))
                    .flex_none()
                    .text_center()
                    .child(
                        div()
                            .absolute()
                            .w_full()
                            .top(px(if c == other {
                                0.
                            } else {
                                -direction * fraction * 22.
                            }))
                            .child(value),
                    )
                    .when(fraction > 0. && c != other && other.is_ascii_digit(), |e| {
                        e.child(
                            div()
                                .absolute()
                                .w_full()
                                .top(px(direction * (1. - fraction) * 22.))
                                .child(other.to_string()),
                        )
                    }),
            );
        } else {
            digits = digits.child(div().flex_none().child(value));
        }
    }
    digits
}

fn crossfade(old: Option<Div>, new: Div, age: f32) -> Div {
    let incoming = ease(age / LABEL_IN);
    let outgoing = ease(age / LABEL_OUT);
    div()
        .relative()
        .when_some(old.filter(|_| age < LABEL_OUT), |e, old| {
            e.child(old.absolute().inset_0().opacity(1. - outgoing))
        })
        .child(
            new.relative()
                .top(px(2. * (1. - incoming)))
                .opacity(incoming),
        )
}

struct ShareHint(&'static str);
impl Render for ShareHint {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .px(px(8.))
            .py(px(4.))
            .rounded(surface_radius(cx))
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_size(px(12.))
            .text_color(p.ink)
            .child(self.0)
    }
}

fn channel_symbol(channel: ReferralShareChannel, p: Paint) -> AnyElement {
    match channel {
        ReferralShareChannel::X => canvas(
            |_, _, _| (),
            move |b, _, w, _| {
                let q = |x, y| b.origin + point(px(x), px(y));
                let mut path = PathBuilder::stroke(px(1.25));
                path.move_to(q(3., 2.));
                path.line_to(q(12., 14.));
                path.line_to(q(15., 14.));
                path.line_to(q(6., 2.));
                path.close();
                path.move_to(q(14., 2.));
                path.line_to(q(9.5, 7.));
                path.move_to(q(7.1, 9.5));
                path.line_to(q(2.5, 14.));
                if let Ok(path) = path.build() {
                    w.paint_path(path, p.ink);
                }
            },
        )
        .size(px(16.))
        .into_any_element(),
        ReferralShareChannel::LinkedIn => div()
            .size(px(14.))
            .rounded(px(1.))
            .bg(p.ink)
            .flex()
            .items_center()
            .justify_center()
            .text_color(p.panel)
            .text_size(px(10.))
            .font_weight(FontWeight::BOLD)
            .child("in")
            .into_any_element(),
        ReferralShareChannel::Email => envelope(p.ink).into_any_element(),
    }
}

fn envelope(ink: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |b, _, w, _| {
            let q = |x, y| b.origin + point(px(x), px(y));
            let mut path = PathBuilder::stroke(px(1.2));
            path.move_to(q(1.5, 3.));
            path.line_to(q(14.5, 3.));
            path.line_to(q(14.5, 13.));
            path.line_to(q(1.5, 13.));
            path.close();
            path.move_to(q(2., 4.));
            path.line_to(q(8., 8.7));
            path.line_to(q(14., 4.));
            if let Ok(path) = path.build() {
                w.paint_path(path, ink);
            }
        },
    )
    .size(px(16.))
    .flex_none()
}

impl ReferralCard {
    fn link_field(&self, radius: Pixels, p: Paint, w: &Window, cx: &Context<Self>) -> Div {
        let displayed = self
            .options
            .link
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let (domain, code) = displayed
            .rsplit_once('/')
            .map_or((displayed, ""), |(a, b)| (a, b));
        let domain = if code.is_empty() {
            domain.to_owned()
        } else {
            format!("{domain}/")
        };
        let idle = tr(cx, "Copy", "复制");
        let label = |state| {
            let (word, glyph) = match state {
                CopyState::Copied => (tr(cx, "Copied", "已复制"), Glyph::Check),
                CopyState::Failed => (tr(cx, "Failed", "失败"), Glyph::Alert),
                CopyState::Writing => (tr(cx, "Copying", "复制中"), Glyph::Copy),
                CopyState::Ready => (idle, Glyph::Copy),
            };
            row()
                .gap(px(6.))
                .justify_center()
                .child(icon(glyph, 15., p.secondary))
                .child(line(word, 14., p.ink))
        };
        let age = self.copied_at.time(LABEL_IN, w, cx);
        let copy = face(
            "referral-copy",
            idle,
            36.,
            f32::from(radius),
            Tone::Clear,
            cx,
        )
        .debug_selector(|| "referral-copy".into())
        .w(px(85.))
        .hover(|s| s.bg(p.hover))
        .child(crossfade(
            Some(label(self.previous_copy)),
            label(self.copy),
            age,
        ))
        .on_click(cx.listener(|s, _, w, cx| s.copy_link(w, cx)));
        row()
            .h(px(44.))
            .flex_1()
            .gap(px(4.))
            .pl(px(16.))
            .pr(px(4.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel.blend(p.ink.opacity(0.035)))
            .child(
                row()
                    .flex_1()
                    .overflow_hidden()
                    .child(line(domain, 14., p.faint).flex_none())
                    .when(!code.is_empty(), |e| {
                        e.child(line(code.to_owned(), 14., p.ink))
                    }),
            )
            .child(copy)
    }

    fn share_actions(&self, narrow: bool, radius: Pixels, p: Paint, cx: &Context<Self>) -> Div {
        let mut buttons = row().gap(px(8.)).when(narrow, |e| e.w_full());
        for (i, channel) in [
            ReferralShareChannel::X,
            ReferralShareChannel::LinkedIn,
            ReferralShareChannel::Email,
        ]
        .into_iter()
        .enumerate()
        {
            let name = channel.label(cx);
            buttons = buttons.child(
                face(
                    ("referral-share", i),
                    name,
                    44.,
                    f32::from(radius),
                    Tone::Surface,
                    cx,
                )
                .debug_selector(move || format!("referral-share-{i}"))
                .when_else(narrow, |e| e.flex_1(), |e| e.w(px(44.)))
                .border_1()
                .border_color(p.line)
                .hover(|s| s.bg(p.hover))
                .active(|s| s.opacity(0.78))
                .tooltip(move |_, cx| cx.new(|_| ShareHint(name)).into())
                .child(channel_symbol(channel, p))
                .on_click(cx.listener(move |s, _, w, cx| s.share(channel, w, cx))),
            );
        }
        buttons
    }

    fn invite_field(&self, radius: Pixels, p: Paint, w: &Window, cx: &Context<Self>) -> Div {
        let editor = self.editor.read(cx);
        let focused = editor.focus.is_focused(w);
        let pending = self.pending.is_some();
        let has_error = !self.draft_error.is_empty();
        let mut track = row()
            .id("referral-chip-track")
            .h_full()
            .flex_1()
            .min_w_0()
            .gap(px(4.))
            .overflow_x_scroll()
            .track_scroll(&self.chip_scroll)
            .on_scroll(cx.listener(|_, _, _, cx| cx.notify()));
        for email in &self.chips {
            let copy = email.clone();
            let name = format!("{} {email}", tr(cx, "Remove", "移除"));
            track = track.child(
                row()
                    .h(px(30.))
                    .max_w(px(220.))
                    .flex_none()
                    .pl(px(8.))
                    .rounded(radius)
                    .bg(p.hover)
                    .child(line(email.clone(), 12., p.ink))
                    .child(
                        face(
                            SharedString::from(format!("referral-remove-{email}")),
                            name,
                            28.,
                            f32::from(radius),
                            Tone::Clear,
                            cx,
                        )
                        .w(px(28.))
                        .hover(|s| s.bg(p.stage))
                        .child(icon(Glyph::Close, 12., p.secondary))
                        .on_click(cx.listener(move |s, _, w, cx| s.remove(&copy, w, cx))),
                    ),
            );
        }
        track = track.child(
            div()
                .min_w(px(if self.chips.is_empty() { 46. } else { 108. }))
                .flex_1()
                .h(px(38.))
                .child(self.editor.clone()),
        );
        let scroll = self.chip_scroll.clone();
        let region = canvas(
            |_, _, _| (),
            move |b, _, _, cx| {
                if let Some(host) = cx.try_global::<HorizontalScrollRegion>() {
                    let offset = -scroll.offset().x;
                    let end = scroll.max_offset().x;
                    if end > px(0.1) {
                        (host.0)(b, offset > px(0.1), offset < end - px(0.1));
                    }
                }
            },
        )
        .absolute()
        .inset_0();
        let field = row()
            .relative()
            .flex_1()
            .h(px(44.))
            .pl(px(8.))
            .pr(px(4.))
            .rounded(radius)
            .border_1()
            .border_color(if has_error {
                p.danger
            } else if focused {
                p.ink
            } else {
                p.line
            })
            .child(track)
            .child(region);
        let label = if pending {
            tr(cx, "Sending", "发送中")
        } else {
            tr(cx, "Send", "发送")
        };
        let period = if disabled_motion(cx) { 2.2 } else { 0.84 };
        let send = face(
            "referral-send",
            label,
            44.,
            f32::from(radius),
            Tone::Primary,
            cx,
        )
        .debug_selector(|| "referral-send".into())
        .w(px(84.))
        .gap(px(4.))
        .when(pending, |e| {
            if w.is_visible() {
                w.request_animation_frame();
            }
            e.child(spinner(
                (self.pending_clock.age(cx) / period).fract(),
                13.,
                p.inverse,
            ))
        })
        .child(label)
        .on_click(cx.listener(|s, _, w, cx| s.send(w, cx)));
        row().mt(px(12.)).gap(px(8.)).child(field).child(send)
    }

    fn hint_lines(&self, p: Paint, w: &Window, cx: &Context<Self>) -> Div {
        let default = tr(cx, "Press Enter after each address", "每个地址输入后按回车");
        let display = |text: &SharedString| {
            line(
                if text.is_empty() {
                    default.to_owned().into()
                } else {
                    text.clone()
                },
                12.,
                p.faint,
            )
            .line_height(px(17.))
        };
        let age = self.hint.clock.time(LABEL_IN, w, cx);
        div()
            .mt(px(8.))
            .h(px(36.))
            .px(px(4.))
            .child(crossfade(
                Some(display(&self.hint.previous)),
                display(&self.hint.current),
                age,
            ))
            .when(!self.draft_error.is_empty(), |e| {
                e.child(
                    row()
                        .gap(px(5.))
                        .h(px(17.))
                        .child(icon(Glyph::Alert, 12., p.danger))
                        .child(line(self.draft_error.clone(), 12., p.danger)),
                )
            })
    }

    fn metric_row(&self, p: Paint, w: &Window, cx: &Context<Self>) -> Div {
        let digit_width = ('0'..='9')
            .map(|digit| measure(&digit.to_string().into(), 18., w))
            .fold(0_f32, f32::max);
        let labels = [
            tr(cx, "Invited", "已邀请"),
            tr(cx, "Signed up", "已注册"),
            tr(cx, "Earned", "已赚取"),
        ];
        let mut cells = row().mt(px(24.)).gap(px(16.)).h(px(42.)).text_color(p.ink);
        for (i, label) in labels.into_iter().enumerate() {
            cells = cells.child(
                column()
                    .flex_1()
                    .child(line(label, 12., p.faint).line_height(px(17.)))
                    .child(roll(
                        self.counters[i],
                        (i == 2).then_some(self.options.currency.as_ref()),
                        digit_width,
                        w,
                        cx,
                    )),
            );
        }
        cells
    }

    fn milestone_text(&self, cx: &App) -> SharedString {
        let reward = &self.options.milestone.reward;
        let goal = u64::from(self.options.milestone.goal.max(1));
        let remaining = goal.saturating_sub(paid(&self.data));
        if remaining == 0 {
            if tr(cx, "en", "zh") == "zh" {
                format!("已解锁{reward}")
            } else {
                format!("{reward} unlocked")
            }
        } else if tr(cx, "en", "zh") == "zh" {
            format!("再有 {remaining} 位付费推荐即可解锁{reward}")
        } else {
            let noun = if remaining == 1 {
                "referral"
            } else {
                "referrals"
            };
            format!("{remaining} more paid {noun} to unlock {reward}")
        }
        .into()
    }

    fn milestone_view(&mut self, narrow: bool, p: Paint, w: &Window, cx: &Context<Self>) -> Div {
        let actual = paid(&self.data);
        let goal = self.options.milestone.goal.max(1);
        let shown = actual.min(u64::from(goal));
        let unlocked = self.unlocked();
        let text = self.milestone_text(cx);
        if self.milestone_label.current.is_empty() {
            self.milestone_label.current = text;
        } else {
            self.milestone_label.set(text, cx);
        }
        let count = if tr(cx, "en", "zh") == "zh" {
            format!("{shown} / {goal}")
        } else {
            format!("{shown} of {goal}")
        };
        let value = if unlocked {
            self.milestone_label.current.clone()
        } else {
            count.clone().into()
        };
        let age = self.milestone_label.clock.time(LABEL_IN, w, cx);
        let label = |text: SharedString, is_new: bool| {
            let detail = if is_new && unlocked {
                self.options.milestone.detail.clone()
            } else {
                None
            };
            let detail = detail.filter(|d| !d.is_empty());
            let ink = if is_new && unlocked {
                p.ink
            } else {
                p.secondary
            };
            if narrow {
                column()
                    .max_w_full()
                    .child(
                        div()
                            .min_w_0()
                            .text_size(px(14.))
                            .text_color(ink)
                            .line_height(px(19.))
                            .child(text),
                    )
                    .when_some(detail, |e, text| e.child(line(text, 12., p.faint)))
            } else {
                row()
                    .gap(px(8.))
                    .child(line(text, 14., ink).line_height(px(19.)))
                    .when_some(detail, |e, text| e.child(line(text, 14., p.faint)))
            }
        };
        let color = p
            .ink
            .blend(p.success.opacity(self.success.get(w, cx).clamp(0., 1.)));
        let fill = self.progress.get(w, cx).clamp(0., goal as f32);
        let segment_count = goal.min(12);
        let filled = fill * segment_count as f32 / goal as f32;
        let mut track = row()
            .id("referral-progress")
            .role(Role::ProgressIndicator)
            .aria_label(tr(cx, "Milestone progress", "里程碑进度"))
            .aria_value(value)
            .h(px(6.))
            .gap(px(4.));
        for index in 0..segment_count {
            let fraction = (filled - index as f32).clamp(0., 1.);
            track = track.child(
                div()
                    .relative()
                    .flex_1()
                    .h_full()
                    .rounded_full()
                    .bg(p.ink.opacity(0.09))
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top_0()
                            .w(relative(fraction))
                            .h_full()
                            .rounded_full()
                            .bg(color),
                    ),
            );
        }
        div()
            .mt(px(14.))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .h(px(if narrow { 42. } else { 22. }))
                    .overflow_hidden()
                    .child(div().mt(px(2.)).child(icon(
                        if unlocked { Glyph::Check } else { Glyph::Bag },
                        16.,
                        if unlocked { p.success } else { p.secondary },
                    )))
                    .child(div().flex_1().min_w_0().child(crossfade(
                        Some(label(self.milestone_label.previous.clone(), false)),
                        label(self.milestone_label.current.clone(), true),
                        age,
                    )))
                    .child(line(count, 14., p.faint).flex_none().line_height(px(19.))),
            )
            .child(div().mt(px(9.)).child(track))
    }

    fn stage_label(&self, stage: ReferralStage, p: Paint, cx: &App) -> Div {
        let (label, color, symbol): (SharedString, Hsla, Glyph) = match stage {
            ReferralStage::Invited => (tr(cx, "Invited", "已邀请").into(), p.faint, Glyph::Comment),
            ReferralStage::SignedUp => (
                tr(cx, "Signed up", "已注册").into(),
                p.secondary,
                Glyph::User,
            ),
            ReferralStage::Earned => {
                let reward = credit(u64::from(self.options.reward), &self.options.currency);
                let name = if tr(cx, "en", "zh") == "zh" {
                    format!("已赚取 {reward}")
                } else {
                    format!("Earned {reward}")
                };
                (name.into(), p.ink, Glyph::Check)
            }
        };
        let symbol_ink = if stage == ReferralStage::Earned {
            p.success
        } else {
            color
        };
        row()
            .gap(px(4.))
            .justify_end()
            .h(px(20.))
            .child(if stage == ReferralStage::Invited {
                envelope(symbol_ink).into_any_element()
            } else {
                icon(symbol, 14., symbol_ink).into_any_element()
            })
            .child(line(label, 13., color).line_height(px(20.)))
    }

    fn referral_list(&mut self, p: Paint, w: &Window, cx: &Context<Self>) -> Div {
        self.stages
            .retain(|_, transition| transition.clock.age(cx) < LABEL_IN);
        self.entrances
            .retain(|_, slide| slide.value(cx) < ROW_HEIGHT - 0.001);
        let mut list = column()
            .id("referral-list")
            .role(Role::List)
            .aria_label(tr(cx, "Referrals", "推荐记录"))
            .debug_selector(|| "referral-list".into())
            .h(px(LIST_HEIGHT))
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll)
            .on_scroll(cx.listener(|_, _, _, cx| cx.notify()));
        if self.data.is_empty() {
            list = list.child(div().py(px(12.)).child(line(
                tr(cx, "No referrals yet", "暂无推荐记录"),
                13.,
                p.secondary,
            )));
        }
        for item in &self.data {
            let name = item.name.as_ref().unwrap_or(&item.email);
            let date = date_label(&item.invited_at, cx);
            let email: SharedString = if item.name.is_some() {
                item.email.clone()
            } else {
                tr(cx, "Invited", "已邀请").into()
            };
            let transition = self.stages.get(&item.id);
            let age = transition.map_or(LABEL_IN, |v| v.clock.time(LABEL_IN, w, cx));
            let badge = crossfade(
                transition.map(|v| self.stage_label(v.previous, p, cx)),
                self.stage_label(item.stage, p, cx),
                age,
            );
            let height = self
                .entrances
                .get(&item.id)
                .map_or(ROW_HEIGHT, |slide| slide.get(w, cx));
            list = list.child(
                div()
                    .id(SharedString::from(format!("referral-row-{}", item.id)))
                    .role(Role::ListItem)
                    .h(px(height.max(0.)))
                    .flex_none()
                    .overflow_hidden()
                    .child(
                        row()
                            .h(px(ROW_HEIGHT))
                            .gap(px(12.))
                            .border_t_1()
                            .border_color(p.line)
                            .child(profile_picture(item, p))
                            .child(
                                column()
                                    .flex_1()
                                    .child(line(name.clone(), 14., p.ink).line_height(px(19.)))
                                    .child(
                                        row()
                                            .gap(px(4.))
                                            .h(px(17.))
                                            .child(line(email, 12., p.faint))
                                            .child(
                                                line(format!("· {date}"), 12., p.faint).flex_none(),
                                            ),
                                    ),
                            )
                            .child(badge.flex_none()),
                    ),
            );
        }
        let scroll = self.list_scroll.clone();
        let reporter = canvas(
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
        .inset_0();
        div().relative().mt(px(16.)).child(list).child(reporter)
    }
}

impl Render for ReferralCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let observed = self.measurement.get();
        let visible = observed.visible && self.visibility.as_ref().is_none_or(|check| check(cx));
        if visible {
            self.play_sample(cx);
        }
        let width = if observed.width > px(0.) {
            observed.width
        } else {
            w.viewport_size().width.min(px(MAX_WIDTH))
        };
        let narrow = width < NARROW;
        let p = Paint::get(cx);
        let radius = surface_radius(cx);
        let title = self.title(cx);
        let description = self.description(cx);
        let link = self.link_field(radius, p, w, cx);
        let shares = self.share_actions(narrow, radius, p, cx);
        let actions = if narrow {
            column().gap(px(8.)).child(link.flex_none()).child(shares)
        } else {
            row().gap(px(8.)).child(link).child(shares)
        };
        let measured = self.measurement.clone();
        let weak = cx.weak_entity();
        let report = canvas(
            move |bounds, w, _| {
                let window = Bounds::new(Point::default(), w.viewport_size());
                let overlap = bounds.intersect(&window).size;
                let area = f32::from(bounds.size.width) * f32::from(bounds.size.height);
                let visible = w.is_visible()
                    && area > 0.
                    && f32::from(overlap.width) * f32::from(overlap.height) >= area * 0.4;
                let next = Measurement {
                    width: bounds.size.width,
                    visible,
                };
                if measured.replace(next) != next {
                    let weak = weak.clone();
                    w.on_next_frame(move |_, cx| {
                        let _ = weak.update(cx, |_, cx| cx.notify());
                    });
                }
            },
            |_, _, _, _| (),
        )
        .absolute()
        .inset_0();
        let body = column()
            .relative()
            .w(px(MAX_WIDTH))
            .max_w_full()
            .p(px(if narrow { 20. } else { 24. }))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel);
        let body = if let Some(style) = &self.style {
            style(body)
        } else {
            body
        };
        let status = match self.copy {
            CopyState::Copied => tr(cx, "Link copied", "已复制链接").into(),
            CopyState::Failed => tr(cx, "Copy failed", "复制失败").into(),
            _ if !self.draft_error.is_empty() => self.draft_error.clone(),
            _ => self.hint.current.clone(),
        };
        body.id(self.id.clone())
            .role(Role::Region)
            .aria_label(title.clone())
            .capture_key_down(cx.listener(|s, event: &KeyDownEvent, w, cx| {
                let editor = s.editor.read(cx);
                if !editor.focus.is_focused(w) || editor.is_composing() {
                    return;
                }
                let key = event.keystroke.key.as_str();
                if key == "tab"
                    && !event.keystroke.modifiers.shift
                    && !editor.value().trim().is_empty()
                {
                    s.commit_draft(w, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                } else if key == "backspace"
                    && editor.value().is_empty()
                    && let Some(email) = s.chips.last().cloned()
                {
                    s.remove(&email, w, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
            .child(report)
            .child(
                line(title, 18., p.ink)
                    .font_weight(FontWeight::MEDIUM)
                    .line_height(px(20.)),
            )
            .child(
                div()
                    .mt(px(4.))
                    .max_w(px(430.))
                    .min_h(px(42.))
                    .text_size(px(14.))
                    .line_height(px(21.))
                    .text_color(p.secondary)
                    .child(description),
            )
            .child(div().mt(px(22.)).child(actions))
            .child(self.invite_field(radius, p, w, cx))
            .child(self.hint_lines(p, w, cx))
            .child(div().mt(px(7.)).h(px(1.)).bg(p.line))
            .child(self.metric_row(p, w, cx))
            .child(self.milestone_view(narrow, p, w, cx))
            .child(self.referral_list(p, w, cx))
            .when(!status.is_empty(), |e| {
                e.child(
                    div()
                        .id("referral-status")
                        .absolute()
                        .size(px(1.))
                        .opacity(0.)
                        .role(Role::Status)
                        .aria_label(status),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Referral, ReferralCard, ReferralCardOptions, ReferralInviteSignal, ReferralMilestone,
        ReferralShareChannel, ReferralStage, counts, referral_share_url,
    };
    use crate::foundation::Theme;
    use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        time::Duration,
    };

    fn tick(w: &mut VisualTestContext, millis: u64) {
        w.executor().advance_clock(Duration::from_millis(millis));
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.run_until_parked();
    }

    #[test]
    fn batch_survives_failure_and_cancellation_with_focus_intact() {
        let mut app = TestAppContext::single();
        app.update(|cx| {
            crate::init(cx);
            Theme::global_mut(cx).reduced_motion = true;
        });
        let seen = Rc::new(RefCell::new(Vec::<ReferralInviteSignal>::new()));
        let records = seen.clone();
        let (view, w) = app.add_window_view(|w, cx| {
            ReferralCard::new(
                ReferralCardOptions {
                    referrals: Some(vec![]),
                    max_invites: 2,
                    ..Default::default()
                },
                w,
                cx,
            )
            .on_invite(move |request, _, cx| {
                let attempt = records.borrow().len();
                assert_eq!(request.emails.len(), 2);
                records.borrow_mut().push(request.signal);
                let timer = cx.background_executor().timer(Duration::from_millis(90));
                cx.spawn(async move |_| {
                    timer.await;
                    if attempt == 0 {
                        Err("Delivery failed".into())
                    } else {
                        Ok(())
                    }
                })
            })
        });
        let input = w.debug_bounds("Invite by email").unwrap().center();
        w.simulate_click(input, Modifiers::default());
        w.simulate_input("alpha@example.com, beta@example.com\nALPHA@example.com");
        tick(w, 0);
        w.update(|_, cx| assert_eq!(view.read(cx).chips.len(), 2));
        w.simulate_keystrokes("enter enter");
        tick(w, 10);
        assert_eq!(seen.borrow().len(), 1);
        w.simulate_input("must not be inserted");
        w.update(|w, cx| {
            let draft = view.read(cx).editor.read(cx);
            assert!(draft.value().is_empty());
            assert!(draft.focus.is_focused(w));
        });
        tick(w, 100);
        w.update(|_, cx| {
            assert_eq!(view.read(cx).chips.len(), 2);
            assert_eq!(view.read(cx).draft_error.as_ref(), "Delivery failed");
        });
        w.simulate_keystrokes("enter");
        tick(w, 5);
        w.update(|w, cx| view.update(cx, |card, cx| card.cancel_invite(w, cx)));
        assert!(seen.borrow()[1].is_cancelled());
        tick(w, 100);
        w.update(|_, cx| assert!(view.read(cx).referrals().is_empty()));
        w.simulate_keystrokes("enter");
        tick(w, 100);
        w.update(|w, cx| {
            let card = view.read(cx);
            assert!(card.chips.is_empty());
            assert_eq!(card.referrals().len(), 2);
            assert_eq!(card.referrals()[0].email.as_ref(), "alpha@example.com");
            assert!(card.editor.read(cx).focus.is_focused(w));
        });
    }

    #[test]
    fn controlled_progress_uses_paid_count_and_fires_once() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let fired = Rc::new(Cell::new(0));
        let observed = fired.clone();
        let (view, w) = app.add_window_view(|w, cx| {
            ReferralCard::new(
                ReferralCardOptions {
                    referrals: Some(vec![]),
                    milestone: ReferralMilestone::new(2, "Bonus"),
                    ..Default::default()
                },
                w,
                cx,
            )
            .on_milestone(move |_, _| observed.set(observed.get() + 1))
        });
        let row = |id: &'static str, stage| {
            Referral::new(id, format!("{id}@example.com"), stage, "2026-10-26")
        };
        w.update(|_, cx| {
            view.update(cx, |card, cx| {
                card.set_referrals(
                    vec![
                        row("a", ReferralStage::Earned),
                        row("b", ReferralStage::SignedUp),
                    ],
                    cx,
                );
                assert_eq!(counts(card.referrals(), 20), [2, 2, 20]);
                assert!(!card.unlocked());
                card.set_referrals(
                    vec![
                        row("a", ReferralStage::Earned),
                        row("b", ReferralStage::Earned),
                    ],
                    cx,
                );
                assert_eq!(counts(card.referrals(), 20), [2, 2, 40]);
                assert!(card.unlocked());
                card.set_referrals(vec![], cx);
                card.set_referrals(
                    vec![
                        row("a", ReferralStage::Earned),
                        row("b", ReferralStage::Earned),
                    ],
                    cx,
                );
            })
        });
        assert_eq!(fired.get(), 1);
        w.simulate_resize(size(px(320.), px(730.)));
        tick(w, 800);
        assert_eq!(
            w.debug_bounds("referral-list").unwrap().size.height,
            px(182.)
        );
    }

    #[test]
    fn share_urls_preserve_the_encoded_link_and_message() {
        let link = "https://example.com/r/test?x=1&y=2";
        let encoded = "https%3A%2F%2Fexample.com%2Fr%2Ftest%3Fx%3D1%26y%3D2";
        let message = "Hello & 朋友";
        let x = referral_share_url(ReferralShareChannel::X, link, message);
        assert!(x.contains("text=Hello%20%26%20%E6%9C%8B%E5%8F%8B"));
        assert!(x.ends_with(encoded));
        let linkedin = referral_share_url(ReferralShareChannel::LinkedIn, link, message);
        assert!(linkedin.ends_with(encoded));
        let email = referral_share_url(ReferralShareChannel::Email, link, message);
        assert!(email.starts_with("mailto:?subject=Hello"));
        assert!(email.ends_with(encoded));
    }
}
