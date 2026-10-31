//! Interactive examples for workspace navigation, selection and layout.
use gpui::{prelude::*, *};
use kui::{
    components::{
        combobox::Combobox,
        dialog::Dialog,
        empty_state::{EmptyAction, EmptyContent, EmptyState},
        popover::Popover,
        resizable_panels::{Pane, ResizablePanels},
        select::Select,
        switch::Switch,
        tabs::{TabPage, Tabs},
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage, disclosure::SurfaceEvent, family::Paint, glyph::Glyph,
        picker::PickerOption,
    },
};
pub const IDS: [&str; 8] = [
    "dialog",
    "popover",
    "tabs",
    "combobox",
    "select",
    "switch",
    "resizable-panels",
    "empty-state",
];
pub struct WorkspaceStory {
    id: String,
    control: AnyView,
    notice: SharedString,
    _subs: Vec<Subscription>,
    created: bool,
}
fn empty(saved: bool, l: UiLanguage) -> EmptyContent {
    if saved {
        EmptyContent::new(
            l.text("View created", "视图已创建"),
            l.text(
                "Launch review is saved with your current filters.",
                "发布评审已保存当前筛选条件。",
            ),
        )
        .with_icon(Glyph::FolderCheck)
        .with_secondary_action(l.text("Undo", "撤销"))
    } else {
        EmptyContent::new(
            l.text("No saved views", "还没有保存的视图"),
            l.text(
                "Create a view to keep your favorite filters close at hand.",
                "创建视图，随时使用常用的筛选条件。",
            ),
        )
        .with_action(l.text("Create view", "创建视图"))
    }
}
impl WorkspaceStory {
    pub fn new(id: &str, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = UiLanguage::current(cx);
        let t = move |en, zh| l.text(en, zh);
        let mut subs = vec![];
        let mut notice = "".into();
        let control: AnyView = match id {
            "dialog" => {
                notice = t("Opens a focused decision surface", "在对话框中确认发布").into();
                let v = cx.new(|cx| {
                    Dialog::new(
                        "review",
                        t("Review changes", "审阅更改"),
                        t("Publish workspace", "发布工作区"),
                        cx,
                    )
                    .with_description(t(
                        "Your team will see these changes immediately.",
                        "团队成员会立即看到这些更改。",
                    ))
                    .with_body(move |_, _| {
                        div().text_size(px(13.)).child(t(
                            "Three files are ready to publish. You can still revise them later.",
                            "有三个文件待发布，之后仍可继续修改。",
                        ))
                    })
                    .with_cancel(t("Keep editing", "继续编辑"))
                    .with_confirm(t("Publish now", "立即发布"))
                });
                subs.push(
                    cx.subscribe(&v, move |v: &mut Self, _, event: &SurfaceEvent, cx| {
                        if *event == SurfaceEvent::Confirmed {
                            v.notice = t("Workspace published", "工作区已发布").into();
                            cx.notify()
                        }
                    }),
                );
                v.into()
            }
            "popover" => {
                notice = t(
                    "Anchored context stays close to its trigger",
                    "在按钮旁查看状态说明",
                )
                .into();
                cx.new(|cx|Popover::new("why",t("Why this status?","为什么是这个状态？"),t("Ready for review","可供评审"),cx).with_icon(Glyph::HelpCircle).with_description(t("All required checks passed. Invite a teammate when you want a second pair of eyes.","所有必需检查均已通过。可以邀请队友一起审阅。")).with_cancel(t("Got it","知道了"))).into()
            }
            "tabs" => {
                let pages = [
                    (
                        "overview",
                        t("Overview", "概览"),
                        t(
                            "A calm summary of this workspace and its current health.",
                            "查看工作区概况与当前运行状况。",
                        ),
                    ),
                    (
                        "activity",
                        t("Activity", "动态"),
                        t(
                            "Emma reviewed the latest changes a few minutes ago.",
                            "艾玛几分钟前审阅了最新更改。",
                        ),
                    ),
                    (
                        "settings",
                        t("Settings", "设置"),
                        t(
                            "Choose who can publish and receive notifications.",
                            "选择谁可以发布或接收通知。",
                        ),
                    ),
                ]
                .into_iter()
                .map(|(id, label, body)| {
                    TabPage::new(id, label, move |_, cx| {
                        div()
                            .text_size(px(14.))
                            .text_center()
                            .text_color(Paint::get(cx).secondary)
                            .child(body)
                    })
                })
                .collect();
                cx.new(|cx| Tabs::new("views", t("Workspace views", "工作区视图"), pages, cx))
                    .into()
            }
            "combobox" => {
                let options = [
                    ("emma", t("Emma Collins", "艾玛·柯林斯"), "emma"),
                    ("marcus", t("Marcus Johnson", "马库斯·约翰逊"), "marcus"),
                    ("ava", t("Ava Mitchell", "艾娃·米切尔"), "ava"),
                ]
                .into_iter()
                .map(|(id, label, alias)| PickerOption::new(id, label).with_keywords([alias]))
                .collect();
                cx.new(|cx| {
                    Combobox::new("owner", t("Owner", "负责人"), options, w, cx).with_value("emma")
                })
                .into()
            }
            "select" => {
                let options = [
                    ("design", t("Design", "设计")),
                    ("engineering", t("Engineering", "工程")),
                    ("research", t("Research", "研究")),
                ]
                .into_iter()
                .map(|(key, label)| PickerOption::new(key, label))
                .collect();
                cx.new(|cx| {
                    Select::new("team", t("Team", "团队"), options, w, cx)
                        .with_description(t(
                            "Choose the team that owns this work.",
                            "选择负责这项工作的团队。",
                        ))
                        .with_value("design")
                })
                .into()
            }
            "switch" => cx
                .new(|cx| {
                    Switch::new("digest", t("Weekly digest", "每周摘要"), w, cx).with_checked(true)
                })
                .into(),
            "resizable-panels" => {
                let panes = [
                    ("sidebar", t("Sidebar", "侧栏"), 24., vec![0.70, 0.52, 0.64]),
                    (
                        "content",
                        t("Content", "内容"),
                        46.,
                        vec![0.88, 0.76, 0.82, 0.54],
                    ),
                    ("details", t("Details", "详情"), 30., vec![0.62, 0.80, 0.46]),
                ]
                .into_iter()
                .enumerate()
                .map(|(i, (id, label, share, lines))| {
                    let pane = Pane::new(id, label, share, move |width, _, cx| {
                        let p = Paint::get(cx);
                        let mut body = div()
                            .flex()
                            .flex_col()
                            .p_4()
                            .size_full()
                            .when(i != 1, |e| e.bg(p.stage))
                            .text_size(px(14.))
                            .child(div().font_weight(FontWeight::MEDIUM).child(label))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(format!("{:.0} px", f32::from(width))),
                            );
                        let mut skeleton = div().mt_4().flex().flex_col().gap_2();
                        for &fraction in &lines {
                            skeleton = skeleton.child(
                                div()
                                    .w(relative(fraction))
                                    .h(px(8.))
                                    .rounded_full()
                                    .bg(p.ink.opacity(0.10)),
                            )
                        }
                        body = body.child(skeleton);
                        body
                    })
                    .with_limits(if i == 1 { 120. } else { 80. }, None);
                    if i != 1 { pane.collapsible() } else { pane }
                })
                .collect();
                cx.new(|cx| {
                    ResizablePanels::new(
                        "workspace-layout",
                        t("Three panes", "三个面板"),
                        panes,
                        cx,
                    )
                })
                .into()
            }
            "empty-state" => {
                let v = cx.new(|_| EmptyState::new("saved-views", empty(false, l)));
                subs.push(
                    cx.subscribe(&v, move |v: &mut Self, e, _: &EmptyAction, cx| {
                        v.created = !v.created;
                        e.update(cx, |s, cx| s.set_content(empty(v.created, l), cx));
                        cx.notify()
                    }),
                );
                v.into()
            }
            _ => unreachable!("unknown workspace story"),
        };
        Self {
            id: id.into(),
            control,
            notice,
            _subs: subs,
            created: false,
        }
    }
}
impl Render for WorkspaceStory {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let vw = f32::from(w.viewport_size().width);
        let p = Paint::get(cx);
        let width = |v: f32| px(v.min((vw - 32.).max(1.)));
        let root = div().flex().flex_col().items_center().min_w_0();
        let v = self.control.clone();
        match self.id.as_str() {
            "dialog" | "popover" => root.gap_3().child(v).child(
                div()
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(self.notice.clone()),
            ),
            "tabs" => root.w(width(420.)).child(v),
            "select" | "combobox" => root
                .w(width(538_f32.min(vw * 0.86)))
                .pb(px(32.))
                .items_stretch()
                .child(v),
            "switch" => root.pb(px(34.)).child(v),
            "resizable-panels" => root
                .w(width(522.))
                .h(px(320.))
                .relative()
                .bg(p.panel)
                .p(px(1.))
                .overflow_hidden()
                .rounded(Theme::global(cx).tokens.radius.panel)
                .child(v)
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(Theme::global(cx).tokens.radius.panel)
                        .border_1()
                        .border_color(p.line),
                ),
            _ => root.w(width(266.)).child(v),
        }
    }
}
