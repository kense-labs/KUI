//! A bilingual quick-capture board with simulated save and retry.
use gpui::{prelude::*, *};
use kui::{
    components::action_morph::{
        ActionMorph, ActionMorphAction, ActionMorphChoice, ActionMorphComposer, ActionMorphFailure,
        ActionMorphSubmit,
    },
    foundation::surface_radius,
    primitives::{
        control::UiLanguage,
        family::Paint,
        glyph::{Glyph, icon},
        presentation::{column, row},
    },
};
use std::time::Duration;

pub const IDS: &[&str] = &["action-morph"];

pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Example::new(w, cx)).into())
}

fn actions(lang: UiLanguage) -> Vec<ActionMorphAction> {
    let t = |en, zh| lang.text(en, zh);
    let task = ActionMorphComposer::new(
        t("New task", "新任务"),
        t("What needs doing?", "需要做什么？"),
        t("Add task", "添加任务"),
        t("Task added", "任务已添加"),
    )
    .with_empty_hint(t("Give the task a title", "请填写任务标题"))
    .with_choices(
        t("Due", "到期时间"),
        vec![
            ActionMorphChoice::new("today", t("Today", "今天")),
            ActionMorphChoice::new("tomorrow", t("Tomorrow", "明天")),
            ActionMorphChoice::new("next-week", t("Next week", "下周")),
        ],
    );
    let note = ActionMorphComposer::new(
        t("New note", "新便笺"),
        t("Write it down", "写下你的想法"),
        t("Save note", "保存便笺"),
        t("Note saved", "便笺已保存"),
    )
    .with_empty_hint(t("Write a note first", "请先写下便笺内容"))
    .with_multiline(true);
    let reminder = ActionMorphComposer::new(
        t("New reminder", "新提醒"),
        t("Remind me to…", "提醒我…"),
        t("Set reminder", "设置提醒"),
        t("Reminder set", "提醒已设置"),
    )
    .with_empty_hint(t("Give the reminder a title", "请填写提醒标题"))
    .with_choices(
        t("When", "提醒时间"),
        vec![
            ActionMorphChoice::new("in-one-hour", t("In 1 hour", "1 小时后")),
            ActionMorphChoice::new("five", "5:00 PM"),
            ActionMorphChoice::new("tomorrow", t("Tomorrow", "明天")),
        ],
    )
    .with_default_choice("five");
    vec![
        ActionMorphAction::new("task", t("Task", "任务"), Glyph::Check, task)
            .with_description(t("Add to your list", "添加到待办清单"))
            .with_shortcut('t'),
        ActionMorphAction::new("note", t("Note", "便笺"), Glyph::FileText, note)
            .with_description(t("Private to you", "仅自己可见"))
            .with_shortcut('n'),
        ActionMorphAction::new("reminder", t("Reminder", "提醒"), Glyph::Bell, reminder)
            .with_description(t("Nudge me at a set time", "在指定时间提醒我"))
            .with_shortcut('r'),
    ]
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Kind {
    Task,
    Note,
    Reminder,
}

struct Entry {
    id: u64,
    kind: Kind,
    text: SharedString,
    time: SharedString,
    done: bool,
    focus: FocusHandle,
}

struct Example {
    morph: Entity<ActionMorph>,
    lang: UiLanguage,
    fail: bool,
    entries: Vec<Entry>,
    mode: [FocusHandle; 2],
    _submit: Subscription,
}

impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let morph = cx.new(|cx| {
            ActionMorph::new(actions(lang), w, cx)
                .with_label(t("Create", "创建"))
                .with_deferred_submit()
                .with_style(|d| d.absolute().right(px(16.)).bottom(px(16.)))
        });
        let submit = cx.subscribe_in(
            &morph,
            w,
            |s: &mut Self, morph, event: &ActionMorphSubmit, w, cx| {
                let request = event.clone();
                let failed = s.fail;
                let lang = s.lang;
                let morph = morph.downgrade();
                cx.spawn_in(w, async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(580))
                        .await;
                    let result = if failed {
                        Err(ActionMorphFailure::new(
                            lang.text("Couldn’t save. Try again.", "保存失败，请重试。"),
                        ))
                    } else {
                        Ok(())
                    };
                    let _ = cx.update(|w, cx| {
                        let accepted = morph
                            .update(cx, |m, cx| m.complete(request.ticket, result, w, cx))
                            .unwrap_or(false);
                        if accepted && !failed {
                            let _ = this.update(cx, |s, cx| s.add(request, cx));
                        }
                    });
                })
                .detach();
            },
        );
        let entries = [
            (
                Kind::Task,
                t("Review Emma’s onboarding flow", "审阅 Emma 的引导流程"),
                t("Today", "今天"),
                false,
            ),
            (
                Kind::Reminder,
                t(
                    "Call Andre about the Q3 budget",
                    "联系 Andre 讨论第三季度预算",
                ),
                "4:30 PM",
                false,
            ),
            (
                Kind::Task,
                t("Send launch notes to Diane", "给 Diane 发送发布说明"),
                t("Tomorrow", "明天"),
                true,
            ),
            (
                Kind::Note,
                t(
                    "Pricing page: lead with the free tier",
                    "定价页优先展示免费方案",
                ),
                "",
                false,
            ),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (kind, text, time, done))| Entry {
            id: u64::MAX - i as u64,
            kind,
            text: text.into(),
            time: time.into(),
            done,
            focus: cx.focus_handle().tab_stop(true),
        })
        .collect();
        Self {
            morph,
            lang,
            fail: false,
            entries,
            mode: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            _submit: submit,
        }
    }

    fn add(&mut self, request: ActionMorphSubmit, cx: &mut Context<Self>) {
        let kind = match request.action.as_ref() {
            "task" => Kind::Task,
            "reminder" => Kind::Reminder,
            _ => Kind::Note,
        };
        let t = |en, zh| self.lang.text(en, zh);
        let time = match request.choice.as_deref() {
            Some("today") => t("Today", "今天"),
            Some("tomorrow") => t("Tomorrow", "明天"),
            Some("next-week") => t("Next week", "下周"),
            Some("in-one-hour") => t("In 1 hour", "1 小时后"),
            Some("five") => "5:00 PM",
            _ => "",
        };
        self.entries.insert(
            0,
            Entry {
                id: request.ticket,
                kind,
                text: request.text,
                time: time.into(),
                done: false,
                focus: cx.focus_handle().tab_stop(true),
            },
        );
        cx.notify();
    }
}

impl Render for Example {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = surface_radius(cx);
        let lang = self.lang;
        let t = |en, zh| lang.text(en, zh);
        let open = self
            .entries
            .iter()
            .filter(|e| e.kind == Kind::Task && !e.done)
            .count();
        let room = w.viewport_size().width.min(px(522.)) - px(32.);
        self.morph.update(cx, |m, cx| m.set_max_width(room, cx));
        let list = self.entries.iter().map(|entry| {
            let name: SharedString = if lang == UiLanguage::Chinese {
                format!(
                    "{} {}",
                    if entry.done { "重新打开" } else { "完成" },
                    entry.text
                )
                .into()
            } else {
                format!(
                    "Mark {} {}",
                    entry.text,
                    if entry.done { "open" } else { "done" }
                )
                .into()
            };
            let id = entry.id;
            let mark = match entry.kind {
                Kind::Task => div()
                    .id(("action-morph-task-toggle", id))
                    .size(px(20.))
                    .flex_none()
                    .rounded_full()
                    .border_1()
                    .border_color(if entry.done {
                        p.bright
                    } else {
                        p.faint.opacity(0.55)
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(entry.done, |d| {
                        d.bg(p.bright).child(icon(Glyph::Check, 13., p.inverse))
                    })
                    .role(Role::CheckBox)
                    .aria_toggled(entry.done.into())
                    .aria_label(name)
                    .track_focus(&entry.focus)
                    .cursor_pointer()
                    .focus_visible(move |d| d.border_color(p.bright))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        if let Some(entry) = s.entries.iter_mut().find(|e| e.id == id) {
                            entry.done = !entry.done;
                            cx.notify();
                        }
                    }))
                    .into_any_element(),
                Kind::Note => icon(Glyph::FileText, 20., p.faint).into_any_element(),
                Kind::Reminder => icon(Glyph::Bell, 20., p.faint).into_any_element(),
            };
            row()
                .id(("action-morph-entry", id))
                .h(px(48.))
                .flex_none()
                .px(px(12.))
                .gap_3()
                .border_b_1()
                .border_color(p.line)
                .child(mark)
                .child(
                    div()
                        .flex_1()
                        .truncate()
                        .text_size(px(14.))
                        .text_color(if entry.done { p.faint } else { p.ink })
                        .child(entry.text.clone()),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(entry.time.clone()),
                )
        });
        let controls = row()
            .id("action-morph-save-mode")
            .role(Role::Group)
            .aria_label(t("Simulated save result", "模拟保存结果"))
            .h(px(42.))
            .p(px(3.))
            .gap(px(2.))
            .rounded_full()
            .border_1()
            .border_color(p.line)
            .children([false, true].into_iter().map(|failure| {
                let label = if failure {
                    t("Saving fails", "保存失败")
                } else {
                    t("Saving works", "保存成功")
                };
                let selected = failure == self.fail;
                let focus = &self.mode[usize::from(failure)];
                row()
                    .id(("action-morph-mode", usize::from(failure)))
                    .role(Role::Button)
                    .aria_label(label)
                    .aria_toggled(selected.into())
                    .track_focus(focus)
                    .cursor_pointer()
                    .h(px(34.))
                    .px(px(12.))
                    .rounded_full()
                    .text_size(px(14.))
                    .text_color(if selected { p.ink } else { p.faint })
                    .when(selected, |d| d.bg(p.panel))
                    .focus_visible(move |s| s.border_1().border_color(p.bright))
                    .child(label)
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.fail = failure;
                        cx.notify();
                    }))
            }));
        column()
            .w_full()
            .items_center()
            .gap(px(16.))
            .py(px(12.))
            .child(
                div()
                    .id("action-morph-board")
                    .relative()
                    .w_full()
                    .max_w(px(522.))
                    .h(px(460.))
                    .rounded(radius)
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .child(
                        column()
                            .w_full()
                            .h_full()
                            .p(px(11.))
                            .child(
                                row()
                                    .h(px(56.))
                                    .px(px(12.))
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(26.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(t("Today", "今天")),
                                    )
                                    .child(div().text_size(px(14.)).text_color(p.faint).child(
                                        if lang == UiLanguage::Chinese {
                                            format!("{open} 项待办")
                                        } else {
                                            format!(
                                                "{open} open {}",
                                                if open == 1 { "task" } else { "tasks" }
                                            )
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .id("action-morph-list")
                                    .h(px(305.))
                                    .overflow_y_scroll()
                                    .children(list),
                            ),
                    )
                    .child(self.morph.clone()),
            )
            .child(controls)
    }
}
