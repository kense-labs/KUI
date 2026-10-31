//! Reference examples rendered with the same installed components on both platforms.
pub mod collections;
mod connection_switch;
pub mod fields;
use connection_switch::ConnectionSwitch;
use gpui::{prelude::*, *};
use kui::{
    components::{
        action_button::ActionButton,
        action_swap::ActionSwap,
        button_group::ButtonGroup,
        confirm_morph::{ConfirmMode, ConfirmMorph, ConfirmRequest, Intent},
        copy_button::CopyButton,
        expanding_button_group::ExpandingButtonGroup,
        floating_button_group::FloatingButtonGroup,
        icon_morph::{IconMorph, MorphChanged},
        split_button::SplitButton,
        theme_switch::{ThemeChange, ThemeSwitch, Transition},
    },
    foundation::Theme,
    primitives::{
        clipboard,
        commit::{Commit, CommitControl, finish},
        control::{UiLanguage, activate},
        family::{Paint, PhaseClock, Tone, face},
        glyph::{Glyph, icon},
        strip::{Activated, Choice, Segment, StripControl, acknowledge},
    },
};
use std::time::Duration;
pub const FAMILY_IDS: [&str; 11] = [
    "button",
    "action-button",
    "split-button",
    "button-group",
    "floating-button-group",
    "expanding-button-group",
    "copy-button",
    "action-swap",
    "icon-morph",
    "confirm-morph",
    "theme-switch",
];
pub struct ThemeBridge(pub fn(ThemeChange));
impl Global for ThemeBridge {}
pub struct FamilyStory {
    id: String,
    views: Vec<AnyView>,
    _subscriptions: Vec<Subscription>,
    failure: bool,
    project_exists: bool,
    mode: ConfirmMode,
    confirm: Option<Entity<ConfirmMorph>>,
    zoom: i32,
    flags: std::collections::BTreeSet<String>,
    message: SharedString,
    stamp: u64,
    focus: FocusHandle,
    edges: [FocusHandle; 2],
    transition: Option<(ThemeChange, Hsla, PhaseClock)>,
}
fn async_demo<T: CommitControl>(
    ent: Entity<T>,
    example: Option<usize>,
    cx: &mut Context<FamilyStory>,
) -> (AnyView, Subscription) {
    let sub = cx.subscribe(&ent, move |v, e, r: &Commit, cx| {
        let ticket = r.ticket;
        let can_fail = example == Some(2);
        let ok = !(can_fail && v.failure);
        let next = r.next;
        let lang = UiLanguage::current(cx);
        // Follow is a synchronous local toggle; only the library and invite demos await a host.
        if example == Some(0) {
            e.update(cx, |c, cx| {
                kui::primitives::commit::finish_inline(c, ticket, ok, cx)
            });
            v.notice(
                if next {
                    lang.text("You’re following Emma Collins", "已关注艾玛·柯林斯")
                } else {
                    lang.text("Unfollowed Emma Collins", "已取消关注")
                }
                .to_owned(),
                cx,
            );
            return;
        }
        cx.spawn(async move |story, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(match example {
                    Some(1) => 380,
                    Some(2) => 1100,
                    _ => 420,
                }))
                .await;
            e.update(cx, |c, cx| finish(c, ticket, ok, cx));
            if let Some(i) = example {
                let msg = match (i, next, ok) {
                    (2, _, false) => lang.text(
                        "Couldn’t send the invite. Try again",
                        "邀请发送失败，请重试",
                    ),
                    (2, _, true) => {
                        lang.text("Invite sent to Ava Mitchell", "已向 Ava Mitchell 发送邀请")
                    }
                    (1, true, _) => lang.text("Added to your library", "已加入资料库"),
                    _ => lang.text("Removed from your library", "已移出资料库"),
                };
                let _ = story.update(cx, |v, cx| v.notice(msg.to_owned(), cx));
            }
        })
        .detach()
    });
    (ent.into(), sub)
}
fn update_item<T: StripControl>(entity: &Entity<T>, id: &str, label: &str, cx: &mut App) {
    entity.update(cx, |v, cx| {
        if let Some(it) = v.strip().items.iter_mut().find(|i| &*i.id == id) {
            it.label = label.to_owned().into()
        }
        cx.notify()
    });
}
fn grouped<T: StripControl>(
    ent: Entity<T>,
    cx: &mut Context<FamilyStory>,
) -> (AnyView, Subscription) {
    let sub = cx.subscribe(&ent, |v, e, a: &Activated, cx| {
        let lang = UiLanguage::current(cx);
        let t = move |en, zh| lang.text(en, zh);
        let id = &*a.0;

        match id {
            "zoom-in" | "zoom-out" | "zoom" => {
                v.zoom = match id {
                    "zoom-in" => (v.zoom + 10).min(200),
                    "zoom-out" => (v.zoom - 10).max(10),
                    _ => 100,
                };
                let text = format!("{}%", v.zoom);
                update_item(&e, "zoom", &text, cx);
                if v.id == "floating-button-group" {
                    v.notice(text, cx)
                }
            }
            "share" | "copy-text" | "copy-link" => {
                let task = clipboard::write("https://example.com/launch", cx);
                let id = a.0.clone();
                let is_group = matches!(v.id.as_str(), "button-group" | "expanding-button-group");
                cx.spawn(async move |v, cx| {
                    let ok = task.await;
                    let label = if ok {
                        t("Copied", "已复制")
                    } else {
                        t("Failed", "失败")
                    };
                    let _ = v.update(cx, |v, cx| {
                        if is_group {
                            acknowledge(&e, &id, label, ok, cx)
                        } else {
                            v.notice(label.to_owned(), cx)
                        }
                    });
                })
                .detach();
            }
            "archive" | "preview" if v.id == "button-group" => {
                let on = if v.flags.remove(id) {
                    false
                } else {
                    v.flags.insert(id.to_owned());
                    true
                };
                update_item(
                    &e,
                    id,
                    match (id, on) {
                        ("archive", true) => t("Restore", "还原"),
                        ("archive", false) => t("Archive", "归档"),
                        (_, true) => t("Edit", "编辑"),
                        _ => t("Preview", "预览"),
                    },
                    cx,
                )
            }
            "duplicate" | "save" if v.id == "button-group" => {
                acknowledge(
                    &e,
                    id,
                    if id == "duplicate" {
                        t("Duplicated", "已创建副本")
                    } else {
                        t("Saved", "已保存")
                    },
                    true,
                    cx,
                );
            }
            _ => {
                let label = match id {
                    "publish" => t("Published", "已发布"),
                    "notify" => t("Published and notified", "已发布并通知"),
                    "draft" => t("Draft saved", "草稿已保存"),
                    "delete" => t("Deleted", "已删除"),
                    "reply" => t("Replied", "已回复"),
                    "forward" => t("Forwarded", "已转发"),
                    "archive" => t("Archived", "已归档"),
                    "snooze" => t("Snoozed", "已延后"),
                    "download" => t("Downloaded", "已下载"),
                    "rename" => t("Renamed", "已重命名"),
                    "duplicate" => t("Duplicated", "已创建副本"),
                    "pin" => t("Pinned", "已固定"),
                    "undo" => t("Undone", "已撤销"),
                    "redo" => t("Redone", "已重做"),
                    "comment" => t("Comment added", "已评论"),
                    "present" => t("Presenting", "演示中"),
                    "lock" => t("Layer lock toggled", "已切换锁定"),
                    "hide" => t("Layer visibility changed", "已切换可见性"),
                    "final" => t("Marked as final", "已标为最终版"),
                    _ => t("Done", "已完成"),
                };
                if id == "undo" || id == "redo" {
                    e.update(cx, |v, cx| {
                        for item in &mut v.strip().items {
                            if matches!(&*item.id, "undo" | "redo") {
                                item.disabled = item.id == a.0
                            }
                        }
                        cx.notify()
                    })
                }
                if matches!(v.id.as_str(), "expanding-button-group" | "split-button") {
                    acknowledge(
                        &e,
                        if v.id == "split-button" {
                            "publish"
                        } else {
                            id
                        },
                        label,
                        true,
                        cx,
                    );
                }
                v.notice(label.to_owned(), cx)
            }
        }
        cx.notify()
    });
    (ent.into(), sub)
}
impl FamilyStory {
    fn notice(&mut self, text: String, cx: &mut Context<Self>) {
        self.stamp += 1;
        let stamp = self.stamp;
        self.message = text.into();
        cx.notify();
        cx.spawn(async move |v, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1800))
                .await;
            v.update(cx, |v, cx| {
                if v.stamp == stamp {
                    v.message = "".into();
                    cx.notify()
                }
            })
            .ok();
        })
        .detach();
    }
    pub fn new(key: &str, w: &mut Window, cx: &mut Context<Self>) -> Self {
        struct StoryFonts;
        impl gpui::Global for StoryFonts {}
        if !cx.has_global::<StoryFonts>() {
            let _ = cx.text_system().add_fonts(vec![
                std::borrow::Cow::Borrowed(include_bytes!("../assets/LatinRegular.ttf")),
                std::borrow::Cow::Borrowed(include_bytes!("../assets/LatinMedium.ttf")),
            ]);
            cx.set_global(StoryFonts);
        }
        let (id, variant) = key.split_once(':').unwrap_or((key, "reveal"));
        let l = UiLanguage::current(cx);
        let t = |en, zh| l.text(en, zh);
        let focus = cx.focus_handle().tab_stop(false);
        // A documentation navigation must not move browser focus into the iframe.
        if !cx.has_global::<crate::StoryBridge>() {
            focus.focus(w, cx);
        }
        let mut s = Self {
            id: id.into(),
            views: vec![],
            _subscriptions: vec![],
            failure: false,
            project_exists: true,
            mode: ConfirmMode::Click,
            confirm: None,
            zoom: 100,
            flags: Default::default(),
            message: "".into(),
            stamp: 0,
            focus,
            edges: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
            transition: None,
        };
        use Glyph::*;
        let demo = s.id.clone();
        let seg = |id, en, zh, g, width| {
            let label = t(en, zh);
            let reserved = match id {
                "share" if demo == "floating-button-group" => t("Copied", "已复制"),
                "share" if demo == "button-group" => t("Copied", "已复制"),
                "archive" if demo == "button-group" => t("Restore", "还原"),
                "share" => t("Shared", "已分享"),
                "duplicate" => t("Duplicated", "已复制"),
                "archive" => t("Archived", "已归档"),
                "reply" => t("Replied", "已回复"),
                "forward" => t("Forwarded", "已转发"),
                "snooze" => t("Snoozed", "已延后"),
                "delete" => t("Deleted", "已删除"),
                "download" => t("Downloaded", "已下载"),
                "copy-link" => t("Copied", "已复制"),
                "rename" => t("Renamed", "已重命名"),
                "publish" if demo == "expanding-button-group" => t("Published", "已发布"),
                "publish" => label,
                "save" => t("Saved", "已保存"),
                _ => label,
            };
            Segment::new(id, label, g, width).fit_label(reserved)
        };
        macro_rules! add {
            ($pair:expr) => {{
                let (view, sub) = $pair;
                s.views.push(view);
                s._subscriptions.push(sub);
            }};
        }
        match id {
            "button" => s.views.push(
                cx.new(|_| crate::base_buttons::BaseButtons::default())
                    .into(),
            ),
            "action-button" => {
                let e = cx.new(|_| {
                    ActionButton::new(
                        "save",
                        [
                            t("Save changes", "保存更改"),
                            t("Saving", "正在保存"),
                            t("Saved", "已保存"),
                            t("Try again", "重试"),
                        ],
                    )
                });
                add!(async_demo(e, None, cx));
            }
            "split-button" => {
                let menu = vec![
                    Choice::new("notify", t("Publish and notify", "发布并通知")).icon(Send),
                    Choice::new("draft", t("Save as draft", "保存草稿")).icon(Save),
                    Choice::new("delete", t("Delete draft", "删除草稿"))
                        .icon(Trash)
                        .danger(),
                ];
                let e = cx.new(|cx| {
                    SplitButton::new(
                        "publishing",
                        vec![
                            seg("publish", "Publish", "发布", Some(Send), 103.),
                            seg(
                                "options",
                                "Publish more actions",
                                "更多发布操作",
                                Some(Down),
                                38.,
                            )
                            .icon_only()
                            .with_menu(menu),
                        ],
                        cx,
                    )
                });
                add!(grouped(e, cx));
            }
            "button-group" => {
                let menu = vec![
                    Choice::new("pin", t("Pin to sidebar", "固定到侧栏")).icon(Pin),
                    Choice::new("final", t("Mark as final", "标为最终版")).icon(Check),
                    Choice::new("delete", t("Delete copy", "删除副本"))
                        .icon(Trash)
                        .disabled(),
                ];
                let rows = [
                    vec![
                        seg("share", "Share", "分享", Some(Link), 99.),
                        seg("duplicate", "Duplicate", "复制副本", Some(Copy), 123.),
                        seg("archive", "Archive", "归档", Some(Archive), 102.),
                        seg("more", "More actions", "更多操作", Some(Down), 38.)
                            .icon_only()
                            .with_menu(menu),
                    ],
                    vec![
                        seg("zoom-out", "Zoom out", "缩小", Some(Minus), 34.).icon_only(),
                        Segment::new("zoom", "100%", None, 71.),
                        seg("zoom-in", "Zoom in", "放大", Some(Plus), 34.).icon_only(),
                    ],
                    vec![
                        seg("save", "Save", "保存", Some(Save), 93.),
                        seg("preview", "Preview", "预览", Some(Eye), 104.),
                    ],
                ];
                for (i, row) in rows.into_iter().enumerate() {
                    let e = cx.new(|cx| {
                        let v = ButtonGroup::new(("group", i), row, cx);
                        match i {
                            1 => v.with_height(34.),
                            2 => v.primary(),
                            _ => v,
                        }
                    });
                    add!(grouped(e, cx));
                }
            }
            "floating-button-group" => {
                let rows = [
                    vec![
                        seg("undo", "Undo", "撤销", Some(Undo), 36.).icon_only(),
                        seg("redo", "Redo", "重做", Some(Redo), 36.)
                            .icon_only()
                            .disabled(),
                        seg("comment", "Comment", "评论", Some(Comment), 36.)
                            .icon_only()
                            .divider(),
                        seg("share", "Share", "分享", Some(Link), 91.),
                        seg("present", "Present", "演示", Some(Play), 94.).divider(),
                    ],
                    vec![
                        seg("zoom-in", "Zoom in", "放大", Some(ZoomIn), 36.).icon_only(),
                        seg("zoom-out", "Zoom out", "缩小", Some(ZoomOut), 36.).icon_only(),
                        seg("fit", "Zoom to fit", "适应窗口", Some(Fit), 36.)
                            .icon_only()
                            .disabled(),
                        seg("lock", "Lock layer", "锁定图层", Some(Lock), 36.)
                            .toggle(false)
                            .icon_only()
                            .divider(),
                        seg("hide", "Hide layer", "隐藏图层", Some(Eye), 36.)
                            .toggle(false)
                            .icon_only(),
                    ],
                    vec![
                        seg("copy-text", "Copy text", "复制文字", Some(Copy), 30.).icon_only(),
                        seg("pin", "Pin message", "固定消息", Some(Pin), 30.)
                            .toggle(false)
                            .icon_only(),
                        seg("archive", "Archive", "归档", Some(Archive), 30.).icon_only(),
                        seg("delete", "Delete", "删除", Some(Trash), 30.)
                            .icon_only()
                            .disabled()
                            .divider(),
                    ],
                ];
                for (i, row) in rows.into_iter().enumerate() {
                    let e = cx.new(|cx| {
                        let v = FloatingButtonGroup::new(("tray", i), row, cx);
                        match i {
                            1 => v.vertical(),
                            2 => v.with_height(30.),
                            _ => v,
                        }
                    });
                    add!(grouped(e, cx));
                }
            }
            "expanding-button-group" => {
                let rows = [
                    vec![
                        seg("reply", "Reply", "回复", Some(Undo), 96.),
                        seg("forward", "Forward", "转发", Some(Redo), 106.),
                        seg("archive", "Archive", "归档", Some(Archive), 103.),
                        seg("snooze", "Snooze", "延后", Some(Clock), 101.),
                        seg("delete", "Delete", "删除", Some(Trash), 96.),
                    ],
                    vec![
                        seg("download", "Download", "下载", Some(Download), 108.),
                        seg("copy-link", "Copy link", "复制链接", Some(Link), 102.),
                        seg("rename", "Rename", "重命名", Some(Edit), 94.),
                    ],
                    vec![
                        seg("publish", "Publish", "发布", Some(Globe), 112.),
                        seg("duplicate", "Duplicate", "复制副本", Some(Copy), 118.),
                        seg("pin", "Pin", "固定", Some(Pin), 82.),
                        seg("delete", "Delete", "删除", Some(Trash), 96.),
                    ],
                ];
                for (i, row) in rows.into_iter().enumerate() {
                    let e =
                        cx.new(|cx| {
                            ExpandingButtonGroup::new(("expand", i), row, cx)
                                .with_height(if i == 1 { 28. } else { 36. })
                        });
                    add!(grouped(e, cx));
                }
            }
            "copy-button" => {
                s.views.push(
                    cx.new(|_| {
                        CopyButton::new(
                            "link",
                            "https://example.com/launch",
                            t("Copy link", "复制链接"),
                        )
                    })
                    .into(),
                );
                s.views.push(
                    cx.new(|_| {
                        CopyButton::new("invite", "KUI-4821", t("Copy invite code", "复制邀请码"))
                            .icon_only()
                    })
                    .into(),
                );
            }
            "action-swap" => {
                for (i, (labels, g, tone, _width)) in [
                    (
                        [
                            t("Follow", "关注"),
                            t("Following…", "关注中"),
                            t("Following", "已关注"),
                            t("Retry", "重试"),
                        ],
                        User,
                        Tone::Primary,
                        101.,
                    ),
                    (
                        [
                            t("Add to library", "加入资料库"),
                            t("", ""),
                            t("Added", "已加入"),
                            t("Retry", "重试"),
                        ],
                        Plus,
                        Tone::Surface,
                        146.,
                    ),
                    (
                        [
                            t("Send invite", "发送邀请"),
                            t("Sending", "正在发送"),
                            t("Sent", "已发送"),
                            t("Try again", "重试"),
                        ],
                        Send,
                        Tone::Muted,
                        131.,
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    let e = cx.new(|_| {
                        let button = ActionSwap::new(("swap", i), labels)
                            .with_tone(tone)
                            .with_done_tone(if i == 0 { Tone::Surface } else { Tone::Success })
                            .with_icons([g, Spinner, Check, Alert]);
                        if i == 2 {
                            button.with_reset(Duration::from_millis(1900))
                        } else {
                            button
                        }
                    });
                    add!(async_demo(e, Some(i), cx));
                }
                let connection = cx.new(|cx| ConnectionSwitch::new(w, cx));
                let sub = cx.subscribe(&connection, |story, _, checked: &bool, _| {
                    story.failure = *checked;
                });
                add!((connection.into(), sub));
            }
            "icon-morph" => {
                let pairs = [
                    (
                        [Menu, Close],
                        ["Open menu", "Close menu", "Menu", "Menu open"],
                        ["打开菜单", "关闭菜单", "菜单", "关闭"],
                    ),
                    (
                        [Play, Pause],
                        ["Play", "Pause", "Paused", "Playing"],
                        ["播放", "暂停", "已暂停", "播放中"],
                    ),
                    (
                        [Plus, Check],
                        ["Add to list", "Remove from list", "Add to list", "Added"],
                        ["加入列表", "移出列表", "加入列表", "已加入"],
                    ),
                    (
                        [Copy, Check],
                        [
                            "Copy install command",
                            "Copy again",
                            "Copy command",
                            "Copied",
                        ],
                        ["复制安装命令", "再次复制", "复制命令", "已复制"],
                    ),
                    (
                        [Sun, Moon],
                        [
                            "Switch tile to dark",
                            "Switch tile to light",
                            "Light",
                            "Dark",
                        ],
                        ["切换至深色", "切换至浅色", "浅色", "深色"],
                    ),
                    (
                        [Down, Up],
                        [
                            "Show details",
                            "Hide details",
                            "Show details",
                            "Hide details",
                        ],
                        ["显示详情", "隐藏详情", "显示详情", "隐藏详情"],
                    ),
                    (
                        [Bell, BellOff],
                        [
                            "Turn alerts off",
                            "Turn alerts on",
                            "Alerts on",
                            "Alerts off",
                        ],
                        ["关闭通知", "打开通知", "通知开启", "通知关闭"],
                    ),
                    (
                        [Heart, Heart],
                        ["Like", "Unlike", "128 likes", "129 likes"],
                        ["喜欢", "取消喜欢", "128 人喜欢", "129 人喜欢"],
                    ),
                ];
                for (i, (pair, en, zh)) in pairs.into_iter().enumerate() {
                    let e = cx.new(|_| {
                        IconMorph::new(("morph", i), pair, [t(en[0], zh[0]), t(en[1], zh[1])])
                            .with_captions([t(en[2], zh[2]), t(en[3], zh[3])])
                            .controlled(i == 3)
                    });
                    if i == 3 {
                        s._subscriptions
                            .push(cx.subscribe(&e, |v, e, r: &MorphChanged, cx| {
                                if !r.0 {
                                    return;
                                }
                                let task = clipboard::write("kui add icon-morph", cx);
                                cx.spawn(async move |v, cx| {
                                    if !task.await {
                                        let _ = v.update(cx, |v, cx| {
                                            v.notice(
                                                UiLanguage::current(cx)
                                                    .text("Copy failed", "复制失败")
                                                    .into(),
                                                cx,
                                            )
                                        });
                                    } else {
                                        e.update(cx, |e, cx| e.set(true, cx));
                                        cx.background_executor()
                                            .timer(Duration::from_millis(1600))
                                            .await;
                                    }
                                    e.update(cx, |e, cx| e.set(false, cx));
                                })
                                .detach();
                                let _ = v;
                            }));
                    }
                    s.views.push(e.into());
                }
            }
            "confirm-morph" => {
                let e = cx.new(|cx| {
                    ConfirmMorph::new(
                        "remove",
                        t("Delete project", "删除项目"),
                        t("Delete project?", "确定删除项目？"),
                        w,
                        cx,
                    )
                });
                let sub = cx.subscribe(&e, |v, e, r: &ConfirmRequest, cx| {
                    let ticket = r.ticket;
                    let ok = !v.failure;
                    let restored = r.intent == Intent::Undo;
                    cx.spawn(async move |story, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(800))
                            .await;
                        e.update(cx, |e, cx| {
                            e.complete(ticket, ok, cx);
                            if ok {
                                e.set_available(restored, cx)
                            }
                        });
                        if ok {
                            let _ = story.update(cx, |v, cx| {
                                v.project_exists = restored;
                                cx.notify()
                            });
                        }
                    })
                    .detach()
                });
                s.confirm = Some(e.clone());
                add!((e.into(), sub));
            }
            "theme-switch" => {
                let mode = match variant {
                    "eclipse" => Transition::Eclipse,
                    "split" => Transition::Split,
                    "rise" => Transition::Rise,
                    _ => Transition::Reveal,
                };
                let e = cx.new(|_| ThemeSwitch::new("theme").with_mode(mode));
                let sub = cx.subscribe(&e, |v, _, r: &ThemeChange, cx| {
                    if let Some(bridge) = cx.try_global::<ThemeBridge>() {
                        (bridge.0)(*r)
                    } else {
                        let old = Paint::get(cx).stage;
                        let mut anim = PhaseClock::default();
                        anim.restart(cx);
                        v.transition = Some((*r, old, anim));
                        Theme::global_mut(cx).set_mode(r.next);
                        cx.refresh_windows()
                    }
                });
                add!((e.into(), sub));
            }
            id if fields::IDS.contains(&id) => s
                .views
                .push(cx.new(|cx| fields::FieldStory::new(id, w, cx)).into()),
            id if collections::IDS.contains(&id) => s.views.push(
                cx.new(|cx| collections::CollectionStory::new(id, w, cx))
                    .into(),
            ),
            _ => {}
        }
        s
    }
    fn scene(&self, narrow: bool, cx: &Context<Self>) -> AnyElement {
        let p = Paint::get(cx);
        let radii = &Theme::global(cx).tokens.radius;
        let l = UiLanguage::current(cx);
        let t = |en, zh| l.text(en, zh);
        let view = |i: usize| self.views[i].clone();
        let col = || div().flex().flex_col();
        let row = || div().flex().items_center();
        let tiny = |s: &'static str| div().text_size(px(13.)).text_color(p.secondary).child(s);
        match self.id.as_str() {
            "action-button" | "split-button" => col()
                .items_center()
                .pb(px(34.))
                .child(view(0))
                .into_any_element(),
            "button-group" => col()
                .gap(px(24.))
                .items_center()
                .children(self.views.clone())
                .into_any_element(),
            "expanding-button-group" => col()
                .w(px(336.))
                .max_w_full()
                .items_start()
                .gap(px(20.))
                .children(self.views.clone())
                .into_any_element(),
            "floating-button-group" => col()
                .items_center()
                .gap(px(32.))
                .child(view(0))
                .child(row().gap(px(32.)).child(view(1)).child(view(2)))
                .into_any_element(),
            "copy-button" => col()
                .pb(px(38.))
                .w(px(400.))
                .max_w_full()
                .gap(px(20.))
                .child(
                    col()
                        .gap(px(8.))
                        .child(tiny(t("Share link", "分享链接")))
                        .child(
                            row()
                                .w_full()
                                .h(px(44.))
                                .rounded(radii.md)
                                .bg(p.panel)
                                .px(px(5.))
                                .pl(px(15.))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .text_size(px(13.))
                                        .font_family("monospace")
                                        .child("example.com/launch"),
                                )
                                .child(view(0)),
                        ),
                )
                .child(
                    row()
                        .gap(px(12.))
                        .child(tiny(t("Invite code", "邀请码")))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(13.))
                                .font_family("monospace")
                                .child("KUI-4821"),
                        )
                        .child(view(1)),
                )
                .into_any_element(),
            "action-swap" => {
                let mut stack = col().pb(px(46.)).w(px(440.)).max_w_full();
                for (i, (name, desc)) in [
                    (
                        t("Emma Collins", "艾玛·柯林斯"),
                        t("Design lead at KUI", "KUI 设计负责人"),
                    ),
                    (
                        t("Designing calm interfaces", "设计宁静的界面"),
                        t("Article · 8 min read", "文章 · 阅读约 8 分钟"),
                    ),
                    ("Ava Mitchell", "ava@example.com"),
                ]
                .into_iter()
                .enumerate()
                {
                    stack = stack.child(
                        row()
                            .min_h(px(68.))
                            .gap(px(8.))
                            .when(i < 2, |e| e.border_b_1().border_color(p.line.opacity(0.6)))
                            .child(col().min_w_0().flex_1().child(name).child(tiny(desc)))
                            .child(view(i)),
                    )
                }
                stack.child(view(3)).into_any_element()
            }
            "icon-morph" => {
                let n = if narrow { 2 } else { 4 };
                let mut grid = col().items_center().gap(px(8.));
                for r in self.views.chunks(n) {
                    grid = grid.child(row().gap(px(8.)).children(r.iter().cloned()))
                }
                grid.into_any_element()
            }
            "confirm-morph" => {
                let opts = |modes: bool| {
                    let mut line = row()
                        .p(px(3.))
                        .rounded(radii.md)
                        .bg(p.panel.opacity(0.32))
                        .gap(px(2.));
                    let labels = if modes {
                        vec![
                            t("Click", "点击"),
                            t("Hold", "长按"),
                            t("Countdown", "倒计时"),
                        ]
                    } else {
                        vec![t("Succeeds", "成功"), t("Fails", "失败")]
                    };
                    for (i, label) in labels.into_iter().enumerate() {
                        let selected = if modes {
                            self.mode as usize == i
                        } else {
                            self.failure == (i == 1)
                        };
                        line = line.child(activate(
                            face(
                                (if modes { "mode" } else { "result" }, i),
                                label,
                                36.,
                                radii.sm.into(),
                                if selected { Tone::Surface } else { Tone::Clear },
                                cx,
                            )
                            .px(px(if narrow { 8. } else { 12. }))
                            .text_color(if selected { p.ink } else { p.faint })
                            .aria_selected(selected)
                            .child(label),
                            cx,
                            move |v, _, cx| {
                                if modes {
                                    v.mode = [
                                        ConfirmMode::Click,
                                        ConfirmMode::Hold,
                                        ConfirmMode::Countdown,
                                    ][i];
                                    if let Some(e) = &v.confirm {
                                        e.update(cx, |e, cx| e.set_mode(v.mode, cx));
                                    }
                                } else {
                                    v.failure = i == 1
                                }
                                cx.notify()
                            },
                        ));
                    }
                    line
                };
                col()
                    .relative()
                    .w(px(420.))
                    .max_w_full()
                    .h(px(320.))
                    .child(
                        row()
                            .w_full()
                            .h(px(260.))
                            .when(narrow, |e| e.flex_col().justify_center())
                            .gap(px(12.))
                            .child(
                                row()
                                    .gap(px(7.))
                                    .when(!narrow, |e| e.w(px(158.)).justify_end())
                                    .when(self.project_exists, |e| {
                                        e.child(icon(Glyph::Folder, 16., p.secondary))
                                    })
                                    .child(if self.project_exists {
                                        t("Northwind redesign", "Northwind 改版")
                                    } else {
                                        t("No projects", "暂无项目")
                                    }),
                            )
                            .child(view(0)),
                    )
                    .child(
                        row()
                            .w_full()
                            .justify_center()
                            .when(narrow, |e| e.flex_wrap())
                            .gap(px(8.))
                            .mt(px(16.))
                            .child(opts(true))
                            .child(opts(false))
                            .when(!self.project_exists, |row| {
                                row.child(activate(
                                    face(
                                        "restore",
                                        t("Restore", "还原示例"),
                                        36.,
                                        14.,
                                        Tone::Surface,
                                        cx,
                                    )
                                    .px_2()
                                    .child(t("Restore", "还原示例")),
                                    cx,
                                    |v, _, cx| {
                                        v.project_exists = true;
                                        if let Some(e) = &v.confirm {
                                            e.update(cx, |c, cx| c.restore(cx))
                                        }
                                        cx.notify()
                                    },
                                ))
                            }),
                    )
                    .into_any_element()
            }
            "theme-switch" => div()
                .flex()
                .flex_none()
                .child(view(0))
                .on_children_prepainted(|bounds, _, cx| {
                    if let (Some(&bounds), Some(bridge)) =
                        (bounds.first(), cx.try_global::<crate::StoryBridge>())
                    {
                        (bridge.theme_target)(bounds, Theme::global(cx).tokens.radius.md)
                    }
                })
                .into_any_element(),
            _ => view(0).into_any_element(),
        }
    }
}
impl Render for FamilyStory {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let vw = f32::from(w.viewport_size().width);
        let narrow = vw < 532.;
        let height = if self.id == "theme-switch" {
            220.
        } else if self.id == "user-menu" {
            626.
        } else if self.id == "icon-morph" && narrow {
            560.
        } else {
            520.
        };
        let embedded = cx.try_global::<crate::StoryBridge>().is_some();
        let edge = |i: usize| {
            div()
                .id(("edge", i))
                .track_focus(&self.edges[i])
                .tab_stop(true)
                .size(px(0.))
        };
        let mut scene = div()
            .relative()
            .w_full()
            .h(px(height))
            .px(px(8.))
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .bg(p.stage);
        if let Some((ev, old, slide)) = &self.transition {
            let duration = match ev.mode {
                Transition::Reveal => 0.38,
                Transition::Eclipse => 0.43,
                Transition::Split => 0.39,
                Transition::Rise => 0.36,
            };
            let t = kui::primitives::family::standard_ease(slide.time(duration, w, cx) / duration);
            if t < 1. {
                scene = scene.bg(*old).child(match ev.mode {
                    Transition::Reveal => {
                        div().absolute().left_0().h_full().w(px(vw * t)).bg(p.stage)
                    }
                    Transition::Split => div()
                        .absolute()
                        .top(px(height * (1. - t) * 0.5))
                        .w_full()
                        .h(px(height * t))
                        .bg(p.stage),
                    Transition::Rise => div()
                        .absolute()
                        .bottom_0()
                        .w_full()
                        .h(px(height * t))
                        .bg(p.stage),
                    Transition::Eclipse => gpui::div().absolute().inset_0().child(
                        gpui::canvas(
                            |_, _, _| (),
                            move |b, _, w, _| {
                                let mut path = PathBuilder::fill();
                                let center = b.center();
                                for i in 0..=96 {
                                    let a = i as f32 * std::f32::consts::TAU / 96.;
                                    let pt = center
                                        + point(
                                            px(vw * 0.8 * t * a.cos()),
                                            px(height * 1.1 * a.sin()),
                                        );
                                    if i == 0 {
                                        path.move_to(pt)
                                    } else {
                                        path.line_to(pt)
                                    }
                                }
                                path.close();
                                if let Ok(shape) = path.build() {
                                    w.paint_path(shape, p.stage)
                                }
                            },
                        )
                        .size_full(),
                    ),
                });
            }
        }
        if embedded {
            scene = scene.child(edge(0))
        }
        scene = scene.child(self.scene(narrow, cx));
        if embedded {
            scene = scene.child(edge(1))
        }
        if !self.message.is_empty()
            && matches!(
                self.id.as_str(),
                "split-button" | "action-swap" | "icon-morph"
            )
        {
            scene = scene.child(
                div()
                    .absolute()
                    .bottom(px(match self.id.as_str() {
                        "split-button" | "action-button" => 215.,
                        "action-swap" => 99.,
                        "expanding-button-group" => 120.,
                        _ => 34.,
                    }))
                    .text_color(p.secondary)
                    .text_size(px(12.))
                    .child(self.message.clone()),
            );
        }
        div()
            .on_children_prepainted(|b, _, cx| {
                if let (Some(b), Some(h)) = (b.first(), cx.try_global::<crate::StoryBridge>()) {
                    (h.frame)(b.size.height.into(), Theme::global(cx).mode)
                }
            })
            .id("family-story")
            .track_focus(&self.focus)
            .tab_stop(false)
            .tab_group()
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .font_family("KUI Preview Latin")
            .text_color(p.ink)
            .text_size(px(14.))
            .when(
                cfg!(target_family = "wasm") && UiLanguage::current(cx) == UiLanguage::Chinese,
                |e| e.font_family("KUI Story Sans"),
            )
            .when(
                fields::IDS.contains(&self.id.as_str())
                    && cx.try_global::<crate::InputFontLoaded>().is_some(),
                |mut s| {
                    if UiLanguage::current(cx) == UiLanguage::Chinese {
                        s = s.font_family("KUI Input Sans");
                    }
                    s.text_style().font_fallbacks =
                        Some(FontFallbacks::from_fonts(vec!["KUI Input Sans".into()]));
                    s
                },
            )
            .on_key_down(cx.listener(|v, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "tab" {
                    let back = e.keystroke.modifiers.shift;
                    let entering = v.focus.is_focused(w);
                    let step = |w: &mut Window, cx: &mut Context<Self>| {
                        if back {
                            w.focus_prev(cx)
                        } else {
                            w.focus_next(cx)
                        }
                    };
                    step(w, cx);
                    if entering && v.edges[usize::from(back)].is_focused(w) {
                        step(w, cx)
                    }
                    if v.edges[usize::from(!back)].is_focused(w) {
                        v.focus.focus(w, cx);
                        if let Some(b) = cx.try_global::<crate::StoryBridge>() {
                            (b.leave)(back)
                        }
                    }
                    cx.stop_propagation()
                }
            }))
            .child(scene)
    }
}
#[cfg(test)]
mod tests;
