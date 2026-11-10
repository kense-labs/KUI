//! Gesture and menu demos; all sample data and application effects live here.
use gpui::{prelude::*, *};
use kui::{
    components::{
        context_menu::ContextMenu,
        dropdown_menu::DropdownMenu,
        hold_to_confirm::{HoldEvent, HoldToConfirm},
        reactions::{Reaction, ReactionChanged, ReactionOption, Reactions},
        swipe_actions::{SwipeAction, SwipeActions, SwipeRow, SwipeSelection},
        user_menu::{Account, AccountEvent, Appearance, UserMenu},
    },
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
        glyph::{Glyph, icon},
        menu::{MenuItem, MenuSelection},
    },
};
use std::time::Duration;
pub const IDS: [&str; 6] = [
    "hold-to-confirm",
    "swipe-actions",
    "reactions",
    "dropdown-menu",
    "context-menu",
    "user-menu",
];
pub struct AccountHost {
    pub compact: fn() -> bool,
    pub current: fn() -> Appearance,
    pub preference: fn(Appearance),
}
impl Global for AccountHost {}
enum Demo {
    Hold(Entity<HoldToConfirm>),
    Inbox(Entity<SwipeActions>),
    React(Entity<Reactions>),
    Dropdown(Entity<DropdownMenu>),
    Context(Entity<ContextMenu>),
    Account(Entity<UserMenu>),
}
pub struct CollectionStory {
    demo: Demo,
    _sub: Subscription,
    _appearance: Option<Subscription>,
    host_appearance: Option<Appearance>,
    note: SharedString,
    holding: bool,
    done: bool,
    undo: Option<SharedString>,
}
pub(crate) fn portrait_image(seed: usize) -> std::sync::Arc<Image> {
    static IMAGES: std::sync::OnceLock<[std::sync::Arc<Image>; 4]> = std::sync::OnceLock::new();
    let data: [&[u8]; 4] = [
        include_bytes!("../../assets/portraits/emma.svg"),
        include_bytes!("../../assets/portraits/marcus.svg"),
        include_bytes!("../../assets/portraits/ava.svg"),
        include_bytes!("../../assets/portraits/sofia.svg"),
    ];
    IMAGES.get_or_init(|| {
        data.map(|s| std::sync::Arc::new(Image::from_bytes(ImageFormat::Svg, s.to_vec())))
    })[seed % 4]
        .clone()
}
fn portrait(seed: usize, side: f32) -> impl IntoElement {
    img(portrait_image(seed))
        .size(px(side))
        .rounded_full()
        .flex_none()
}
fn text(text: impl Into<SharedString>, size: f32, ink: Hsla) -> Div {
    div().text_size(px(size)).text_color(ink).child(text.into())
}
fn theme_mode(preference: Appearance, w: &Window) -> ThemeMode {
    match preference {
        Appearance::Dark => ThemeMode::Dark,
        Appearance::Light => ThemeMode::Light,
        Appearance::System
            if matches!(
                w.appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ) =>
        {
            ThemeMode::Dark
        }
        Appearance::System => ThemeMode::Light,
    }
}
impl CollectionStory {
    pub fn new(id: &str, w: &mut Window, cx: &mut Context<Self>) -> Self {
        use Glyph::*;
        let lang = UiLanguage::current(cx);
        let t = move |en, zh| lang.text(en, zh);
        let (demo, sub) = match id {
            "hold-to-confirm" => {
                let c = cx.new(|cx| {
                    HoldToConfirm::new("delete", t("Hold to delete", "按住删除"), w, cx)
                        .with_done_label(t("Deleted", "已删除"))
                });
                let s = cx.subscribe(&c, |v, _, e: &HoldEvent, cx| {
                    match e {
                        HoldEvent::Holding(h) => v.holding = *h,
                        HoldEvent::Confirmed => v.done = true,
                    }
                    cx.notify()
                });
                (Demo::Hold(c), s)
            }
            "swipe-actions" => {
                let data = [
                    (
                        "emma",
                        "EC",
                        "Emma Collins",
                        "9:41",
                        t("Atlas launch checklist", "Atlas 发布清单"),
                        t(
                            "QA signed off. Two billing blockers left before Thursday.",
                            "测试已通过。周四之前还有两个计费问题需要处理。",
                        ),
                    ),
                    (
                        "marcus",
                        "MJ",
                        "Marcus Johnson",
                        "8:15",
                        t("Invoice INV-2048 from Northwind", "Northwind 发票 INV-2048"),
                        t(
                            "$4,280.00 is due on October 3. The PDF is attached.",
                            "4,280 美元于 10 月 3 日到期。请查看附件。",
                        ),
                    ),
                    (
                        "ava",
                        "AM",
                        "Ava Mitchell",
                        t("Yesterday", "昨天"),
                        t("Research readout moved to Friday", "调研分享改到周五"),
                        t(
                            "Room 4B, 2:00 to 3:30. Interview clips are in the deck.",
                            "下午两点至三点半，4B 会议室。访谈片段已放入演示稿。",
                        ),
                    ),
                    (
                        "sofia",
                        "SR",
                        "Sofia Ramirez",
                        t("Mon", "周一"),
                        t("Design crit notes", "设计评审笔记"),
                        t(
                            "Navigation feedback, with an owner for each follow-up.",
                            "导航反馈已整理，每个后续事项都有负责人。",
                        ),
                    ),
                ];
                let rows = data
                    .into_iter()
                    .enumerate()
                    .map(|(i, (id, _, name, time, title, blurb))| {
                        SwipeRow::new(id, title, move |_, cx| {
                            let p = Paint::get(cx);
                            div()
                                .size_full()
                                .px(px(14.))
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(portrait(i, 38.))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .flex()
                                                .justify_between()
                                                .items_center()
                                                .child(
                                                    text(name, 14., p.ink)
                                                        .font_weight(FontWeight::MEDIUM),
                                                )
                                                .child(text(time, 11., p.faint)),
                                        )
                                        .child(text(title, 13., p.ink).truncate())
                                        .child(text(blurb, 12., p.secondary).truncate()),
                                )
                                .into_any_element()
                        })
                        .leading(vec![SwipeAction::new(
                            "archive",
                            t("Archive", "归档"),
                            Archive,
                        )])
                        .trailing(vec![
                            SwipeAction::new("snooze", t("Snooze", "稍后处理"), Clock),
                            SwipeAction::new("delete", t("Delete", "删除"), Trash).destructive(),
                        ])
                    })
                    .collect();
                let c = cx.new(|cx| SwipeActions::new("inbox", t("Inbox", "收件箱"), rows, cx));
                let s = cx.subscribe(&c, move |v, _, e: &SwipeSelection, cx| {
                    v.undo = Some(e.row.clone());
                    let verb = match e.action.as_ref() {
                        "archive" => t("Archived", "已归档"),
                        "snooze" => t("Snoozed", "已延后"),
                        _ => t("Deleted", "已删除"),
                    };
                    let title = data
                        .iter()
                        .find(|x| x.0 == e.row.as_ref())
                        .map_or("", |x| x.4);
                    v.note = format!("{verb} · {title}").into();
                    cx.notify()
                });
                (Demo::Inbox(c), s)
            }
            "reactions" => {
                let opts = [
                    ("like", "Like", "赞", Like),
                    ("love", "Love", "喜欢", Heart),
                    ("celebrate", "Celebrate", "庆祝", Celebrate),
                    ("insight", "Insightful", "有启发", Bulb),
                    ("curious", "Curious", "好奇", Curious),
                    ("ship", "Ship it", "发布吧", Rocket),
                ]
                .map(|(id, en, zh, g)| ReactionOption::new(id, t(en, zh), g))
                .to_vec();
                let c = cx.new(|cx| {
                    Reactions::new(
                        "release-reactions",
                        opts,
                        vec![
                            Reaction::new("celebrate", 12, false),
                            Reaction::new("like", 9, false),
                            Reaction::new("insight", 4, true),
                            Reaction::new("curious", 1, false),
                        ],
                        cx,
                    )
                });
                let s = cx.subscribe(&c, |_, _, _: &ReactionChanged, cx| cx.notify());
                (Demo::React(c), s)
            }
            "dropdown-menu" => {
                let c = cx.new(|cx| {
                    DropdownMenu::new(
                        "project-menu",
                        t("Project actions", "项目操作"),
                        vec![
                            MenuItem::new("edit", t("Edit project", "编辑项目")).with_icon(Edit),
                            MenuItem::new("download", t("Download brief", "下载简报"))
                                .with_icon(Download),
                            MenuItem::new("archive", t("Archive", "归档"))
                                .with_icon(Archive)
                                .with_separator(),
                        ],
                        cx,
                    )
                    .with_icon(More)
                });
                let s = cx.subscribe(&c, move |v, _, e: &MenuSelection, cx| {
                    v.note = match e.0.as_ref() {
                        "edit" => t("Edit opened", "已打开编辑"),
                        "download" => t("Brief selected", "已选择简报"),
                        _ => t("Project archived", "项目已归档"),
                    }
                    .into();
                    cx.notify()
                });
                (Demo::Dropdown(c), s)
            }
            "context-menu" => {
                let c = cx.new(|cx| {
                    ContextMenu::new(
                        "project-target",
                        t("Open actions", "打开操作"),
                        vec![
                            MenuItem::new("edit", t("Edit project", "编辑项目")).with_icon(Edit),
                            MenuItem::new("copy", t("Copy link", "复制链接")).with_icon(Copy),
                            MenuItem::new("delete", t("Delete project", "删除项目"))
                                .with_icon(Trash)
                                .destructive(),
                        ],
                        move |_, cx| {
                            let p = Paint::get(cx);
                            div()
                                .w(px(190.))
                                .p(px(15.))
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .rounded(Theme::global(cx).tokens.radius.md)
                                .bg(p.panel)
                                .border_1()
                                .border_color(p.line)
                                .child(
                                    text(t("Quarterly launch", "季度发布"), 14., p.ink)
                                        .font_weight(FontWeight::MEDIUM),
                                )
                                .child(text(
                                    t("Product · Updated today", "产品 · 今天更新"),
                                    12.,
                                    p.secondary,
                                ))
                                .child(
                                    div()
                                        .mt(px(7.))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(icon(More, 14., p.secondary))
                                        .child(text(
                                            t("Open actions", "打开操作"),
                                            12.,
                                            p.secondary,
                                        )),
                                )
                                .into_any_element()
                        },
                        cx,
                    )
                });
                let s = cx.subscribe(&c, move |v, _, e: &MenuSelection, cx| {
                    if e.0.as_ref() == "copy" {
                        let task = kui::primitives::clipboard::write(
                            "https://kense-labs.github.io/kui/components/context-menu/",
                            cx,
                        );
                        cx.spawn(async move |v, cx| {
                            let ok = task.await;
                            let _ = v.update(cx, |v, cx| {
                                v.note = if ok {
                                    t("Link copied", "已复制链接")
                                } else {
                                    t("Could not copy. Try again.", "复制失败，请重试。")
                                }
                                .into();
                                cx.notify()
                            });
                        })
                        .detach();
                        return;
                    }
                    v.note = match e.0.as_ref() {
                        "edit" => t("Edit opened", "已打开编辑"),
                        _ => t("Project deleted", "项目已删除"),
                    }
                    .into();
                    cx.notify()
                });
                (Demo::Context(c), s)
            }
            "user-menu" => {
                let mut account = Account::new("Emma Collins", "emma@example.com");
                account.plan = Some("Pro".into());
                let compact = cx
                    .try_global::<AccountHost>()
                    .map(|h| (h.compact)())
                    .unwrap_or(w.viewport_size().width < px(640.));
                let c = cx.new(|cx| {
                    let mut c = UserMenu::new(
                        "account-menu",
                        account,
                        vec![
                            MenuItem::new("profile", t("Profile", "个人资料"))
                                .with_icon(User)
                                .with_shortcut("⌘ P"),
                            MenuItem::new("settings", t("Settings", "设置"))
                                .with_icon(Settings)
                                .with_shortcut("⌘ ,"),
                            MenuItem::new("billing", t("Billing", "账单")).with_icon(Card),
                        ],
                        cx,
                    )
                    .show_name()
                    .with_align(kui::primitives::menu::MenuAlign::Center)
                    .with_panel_width(248.)
                    .with_sign_out_shortcut("⇧ ⌘ Q")
                    .with_avatar(portrait_image(0));
                    if let Some(host) = cx.try_global::<AccountHost>() {
                        c = c.with_appearance((host.current)())
                    }
                    if !compact {
                        c = c.initially_open()
                    }
                    c.set_compact(cx.try_global::<AccountHost>().map(|h| (h.compact)()));
                    c
                });
                let s = cx.subscribe_in(&c, w, move |v, ent, e: &AccountEvent, w, cx| {
                    match e {
                        AccountEvent::Navigate(id) => {
                            v.note = match id.as_ref() {
                                "profile" => t("Opened profile", "已打开个人资料"),
                                "billing" => t("Opened billing", "已打开账单"),
                                _ => t("Opened settings", "已打开设置"),
                            }
                            .into()
                        }
                        AccountEvent::Appearance(a) => {
                            if let Some(h) = cx.try_global::<AccountHost>() {
                                (h.preference)(*a)
                            } else {
                                let m = theme_mode(*a, w);
                                Theme::global_mut(cx).set_mode(m);
                                cx.refresh_windows()
                            }
                        }
                        AccountEvent::SignOut => {
                            let ent = ent.clone();
                            cx.spawn_in(w, async move |v, cx| {
                                cx.background_executor()
                                    .timer(Duration::from_millis(650))
                                    .await;
                                let _ = cx.update(|w, cx| {
                                    ent.update(cx, |s, cx| s.finish_sign_out(true, w, cx));
                                    let _ = v.update(cx, |v, cx| {
                                        v.note = t("Signed out", "已退出登录").into();
                                        cx.notify()
                                    });
                                });
                            })
                            .detach()
                        }
                        _ => {}
                    }
                    cx.notify()
                });
                (Demo::Account(c), s)
            }
            _ => unreachable!("only collection ids are mounted here"),
        };
        let host_appearance = cx.try_global::<AccountHost>().map(|h| (h.current)());
        let mut appearance = None;
        if matches!(&demo, Demo::Account(_)) && host_appearance.is_none() {
            Theme::global_mut(cx).set_mode(theme_mode(Appearance::System, w));
            appearance = Some(cx.observe_window_appearance(w, |v, w, cx| {
                if let Demo::Account(c) = &v.demo
                    && c.read(cx).appearance() == Appearance::System
                {
                    Theme::global_mut(cx).set_mode(theme_mode(Appearance::System, w));
                    cx.refresh_windows()
                }
            }));
        }
        Self {
            demo,
            _sub: sub,
            _appearance: appearance,
            host_appearance,
            note: "".into(),
            holding: false,
            done: false,
            undo: None,
        }
    }
}
impl Render for CollectionStory {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let t = move |en, zh| lang.text(en, zh);
        let avail = (f32::from(w.viewport_size().width) - 32.).max(1.);
        let stack = || div().flex().flex_col();
        let center = || stack().items_center();
        let hint = |s: SharedString| text(s, 13., p.secondary).id("status").role(Role::Status);
        match &self.demo {
            Demo::Hold(c) => center()
                .gap(px(14.))
                .child(c.clone())
                .child(
                    div()
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .when(!self.done, |e| {
                            e.child(hint(
                                if self.holding {
                                    t("Keep holding to delete", "继续按住以删除")
                                } else {
                                    t(
                                        "Hold for 1.2 seconds, or hold Space",
                                        "按住 1.2 秒，或按住空格键",
                                    )
                                }
                                .into(),
                            ))
                        })
                        .when(self.done, |e| {
                            e.child(
                                face(
                                    "restore",
                                    t("Restore", "还原"),
                                    24.,
                                    kui::foundation::surface_radius(cx).into(),
                                    Tone::Clear,
                                    cx,
                                )
                                .gap(px(5.))
                                .text_color(p.secondary)
                                .text_size(px(13.))
                                .child(icon(Glyph::Undo, 14., p.secondary))
                                .child(t("Restore", "还原"))
                                .on_click(cx.listener(
                                    |v, _, _, cx| {
                                        if let Demo::Hold(c) = &v.demo {
                                            c.update(cx, |s, cx| s.reset(cx))
                                        }
                                        v.done = false;
                                        cx.notify()
                                    },
                                )),
                            )
                        }),
                )
                .into_any_element(),
            Demo::Inbox(c) => {
                let count = c.read(cx).remaining();
                stack()
                    .w(px(avail.min(438.)))
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                text(t("Inbox", "收件箱"), 18., p.ink)
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .child(text(
                                format!("{} {}", count, t("left to triage", "封待处理")),
                                13.,
                                p.secondary,
                            )),
                    )
                    .child(c.clone())
                    .when(count == 0, |e| {
                        e.child(text(t("Inbox cleared", "收件箱已清空"), 14., p.secondary))
                    })
                    .child(
                        div()
                            .h(px(30.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(hint(if self.note.is_empty() {
                                t(
                                    "Swipe a message, or use its menu.",
                                    "滑动消息，或使用更多菜单。",
                                )
                                .into()
                            } else {
                                self.note.clone()
                            }))
                            .when(self.undo.is_some(), |e| {
                                e.child(
                                    face(
                                        "undo",
                                        t("Undo", "撤销"),
                                        28.,
                                        kui::foundation::surface_radius(cx).into(),
                                        Tone::Clear,
                                        cx,
                                    )
                                    .text_size(px(13.))
                                    .child(t("Undo", "撤销"))
                                    .on_click(cx.listener(
                                        |v, _, _, cx| {
                                            if let (Some(id), Demo::Inbox(c)) =
                                                (v.undo.take(), &v.demo)
                                            {
                                                c.update(cx, |s, cx| s.restore(&id, cx));
                                                v.note = "".into();
                                                cx.notify()
                                            }
                                        },
                                    )),
                                )
                            }),
                    )
                    .into_any_element()
            }
            Demo::React(c) => {
                let values = c.read(cx).values();
                let total: u32 = values.iter().map(|r| r.count).sum();
                let names = values
                    .iter()
                    .filter(|r| r.reacted)
                    .map(|r| match r.id.as_ref() {
                        "like" => t("Like", "赞"),
                        "love" => t("Love", "喜欢"),
                        "insight" => t("Insightful", "有启发"),
                        "curious" => t("Curious", "好奇"),
                        "celebrate" => t("Celebrate", "庆祝"),
                        _ => t("Ship it", "发布吧"),
                    })
                    .collect::<Vec<_>>()
                    .join("、");
                let mut summary = format!("{} {}", total, t("reactions", "个回应"));
                if !names.is_empty() {
                    summary += t(" · You reacted with ", " · 你的回应：");
                    summary += &names
                }
                stack().relative().top(px(-14.)).w(px(avail.min(438.))).p(px(22.)).gap(px(12.)).rounded(Theme::global(cx).tokens.radius.md).bg(p.panel).border_1().border_color(p.line)
 .child(div().flex().items_center().gap(px(10.)).child(portrait(3,36.)).child(stack().child(text("Sofia Ramirez",14.,p.ink).font_weight(FontWeight::MEDIUM)).child(text(t("Released Atlas 2.4 · Tue 16 Sep","发布了 Atlas 2.4 · 9 月 16 日"),12.,p.secondary))))
 .child(text(t("Offline drafts are here","现在可以离线保存草稿了"),17.,p.ink).font_weight(FontWeight::MEDIUM))
 .child(text(t("Keep editing during a flight or train ride. Your draft syncs when you reconnect. If a teammate changes the same field, you can compare both versions before choosing what to keep.","在飞机或列车上编辑的草稿，会在恢复网络时自动同步。如果两个人修改了同一个字段，两个版本会并排显示，方便对照。"),14.,p.secondary).line_height(px(20.))).child(c.clone()).child(hint(summary.into())).into_any_element()
            }
            Demo::Dropdown(c) => center()
                .gap(px(10.))
                .child(c.clone())
                .child(hint(self.note.clone()).h(px(18.)))
                .into_any_element(),
            Demo::Context(c) => center()
                .gap(px(10.))
                .child(c.clone())
                .child(hint(self.note.clone()).h(px(18.)))
                .into_any_element(),
            Demo::Account(c) => {
                let compact = cx.try_global::<AccountHost>().map(|h| (h.compact)());
                let host = cx.try_global::<AccountHost>().map(|h| (h.current)());
                if host != self.host_appearance {
                    self.host_appearance = host;
                    if let Some(preference) = host {
                        c.update(cx, |s, cx| s.set_appearance(preference, cx))
                    }
                }
                c.update(cx, |s, _| s.set_compact(compact));
                center()
                    .h(px(
                        if compact.unwrap_or(w.viewport_size().width < px(640.)) {
                            40.
                        } else {
                            402.
                        },
                    ))
                    .gap(px(10.))
                    .child(c.clone())
                    .child(hint(self.note.clone()))
                    .into_any_element()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CollectionStory, Demo, Duration, Theme, ThemeMode};
    use gpui::{Entity, MouseButton, Pixels, Point, TouchEvent, TouchId, TouchPhase, point, px};
    use gpui::{Modifiers, TestAppContext, VisualTestContext};
    fn mount(id: &'static str) -> (TestAppContext, Entity<CollectionStory>, VisualTestContext) {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (e, w) = app.add_window_view(|w, cx| CollectionStory::new(id, w, cx));
        let visual = w.clone();
        (app, e, visual)
    }
    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh()
        });
        w.run_until_parked()
    }
    fn pos(w: &mut VisualTestContext, name: &'static str) -> Point<Pixels> {
        w.debug_bounds(name)
            .unwrap_or_else(|| panic!("missing control: {name}"))
            .center()
    }
    fn click(w: &mut VisualTestContext, label: &'static str) {
        let p = pos(w, label);
        w.simulate_click(p, Modifiers::default());
        tick(w, 350)
    }
    fn touch(w: &mut VisualTestContext, phase: TouchPhase, p: Point<Pixels>) {
        w.simulate_event(TouchEvent {
            id: TouchId(1),
            phase,
            position: p,
            ..Default::default()
        });
    }
    #[test]
    fn premature_release_and_cancelled_contact_do_not_delete() {
        let (_a, e, mut w) = mount("hold-to-confirm");
        let p = pos(&mut w, "Hold to delete");
        w.simulate_mouse_down(p, MouseButton::Left, Modifiers::default());
        tick(&mut w, 1199);
        w.simulate_mouse_up(p, MouseButton::Left, Modifiers::default());
        tick(&mut w, 1500);
        assert!(!w.update(|_, cx| e.read(cx).done));
        touch(&mut w, TouchPhase::Started, p);
        tick(&mut w, 600);
        touch(&mut w, TouchPhase::Cancelled, p);
        tick(&mut w, 1500);
        assert!(!w.update(|_, cx| e.read(cx).done));
        touch(&mut w, TouchPhase::Started, p);
        tick(&mut w, 1200);
        assert!(w.update(|_, cx| e.read(cx).done));
        touch(&mut w, TouchPhase::Ended, p);
        click(&mut w, "Restore");
        assert!(!w.update(|_, cx| e.read(cx).done));
    }
    #[test]
    fn command_menus_anchor_and_restore_keyboard_access() {
        for (id, target) in [
            ("context-menu", "Open actions"),
            ("dropdown-menu", "Project actions"),
        ] {
            let (_a, e, mut w) = mount(id);
            let p = pos(&mut w, target);
            if id == "context-menu" {
                w.simulate_mouse_down(p, MouseButton::Right, Modifiers::default());
                w.simulate_mouse_up(p, MouseButton::Right, Modifiers::default());
                tick(&mut w, 450)
            } else {
                click(&mut w, target)
            }
            assert!(w.debug_bounds("Edit project").is_some());
            w.simulate_keystrokes("end enter");
            tick(&mut w, 400);
            assert!(!w.update(|_, cx| e.read(cx).note.is_empty()));
            assert!(w.debug_bounds("Edit project").is_none());
            w.simulate_keystrokes(if id == "context-menu" {
                "shift-f10"
            } else {
                "down"
            });
            tick(&mut w, 250);
            assert!(w.debug_bounds("Edit project").is_some());
            w.simulate_keystrokes("escape");
            tick(&mut w, 400);
            assert!(w.debug_bounds("Edit project").is_none());
        }
    }
    #[test]
    fn reaction_picker_adds_and_removes_a_new_vote() {
        let (_a, e, mut w) = mount("reactions");
        click(&mut w, "Add reaction");
        w.simulate_keystrokes("end enter");
        tick(&mut w, 400);
        let has = |w: &mut VisualTestContext| {
            w.update(|_,cx|matches!(&e.read(cx).demo,Demo::React(c) if c.read(cx).values().iter().any(|v|v.id.as_ref()=="ship"&&v.count==1&&v.reacted)))
        };
        assert!(has(&mut w));
        click(&mut w, "Ship it · 1, including yours");
        assert!(!has(&mut w))
    }
    #[test]
    fn swipe_menu_and_drag_share_the_removal_and_undo_path() {
        let (_a, e, mut w) = mount("swipe-actions");
        let label = "More actions: Atlas launch checklist";
        let count = |w: &mut VisualTestContext| {
            w.update(|_, cx| match &e.read(cx).demo {
                Demo::Inbox(c) => c.read(cx).remaining(),
                _ => 0,
            })
        };
        click(&mut w, label);
        click(&mut w, "Archive");
        assert_eq!(count(&mut w), 3);
        click(&mut w, "Undo");
        assert_eq!(count(&mut w), 4);
        let b = w.debug_bounds(label).unwrap();
        let start = point(px(32.), b.center().y);
        let end = point(b.left() - px(12.), start.y);
        touch(&mut w, TouchPhase::Started, start);
        tick(&mut w, 60);
        touch(&mut w, TouchPhase::Moved, end);
        tick(&mut w, 40);
        touch(&mut w, TouchPhase::Ended, end);
        tick(&mut w, 600);
        assert_eq!(count(&mut w), 3);
        click(&mut w, "Undo");
        assert_eq!(count(&mut w), 4)
    }
    #[test]
    fn account_preference_stays_open_and_sign_out_waits_for_host() {
        let (_a, e, mut w) = mount("user-menu");
        tick(&mut w, 500);
        if w.debug_bounds("Profile").is_none() {
            click(&mut w, "Account menu, Emma Collins")
        }
        click(&mut w, "Dark");
        assert_eq!(w.update(|_, cx| Theme::global(cx).mode), ThemeMode::Dark);
        assert!(w.debug_bounds("Profile").is_some());
        click(&mut w, "Sign out");
        assert!(w.debug_bounds("Profile").is_some());
        tick(&mut w, 1000);
        tick(&mut w, 400);
        assert_eq!(w.update(|_, cx| e.read(cx).note.clone()), "Signed out");
    }
}
