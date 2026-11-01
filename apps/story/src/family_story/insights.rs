//! Localized data and host actions for feedback, chart and identity examples.
mod chart_data;
use super::FamilyStory;
use gpui::{prelude::*, *};
use kui::{
    components::{
        activity_heatmap::DailyActivity, alert::*, avatar::*, avatar_group::*, badge::*,
        bar_chart::BarChart, card::*, donut_chart::DonutChart, line_chart::LineChart,
        metric_card::*, onboarding_checklist::*, progress::*, skeleton::*, sparkline::Sparkline,
        stepper::*, timeline::*, toast::*, usage_meter::*,
    },
    primitives::{
        control::UiLanguage,
        family::{Paint, Slide, Tone},
        glyph::Glyph,
        plot::Sample,
        presentation::*,
    },
};
use std::{rc::Rc, sync::Arc, time::Duration};
type Action = Rc<dyn Fn(&mut Demo, usize, &mut Context<Demo>)>;
/// A host-owned selector lives inside the chart header; its selection drives actual data.
struct RangeBar {
    labels: Vec<SharedString>,
    selected: usize,
    owner: WeakEntity<Demo>,
    focus: Vec<FocusHandle>,
    slide: Slide,
    width: Slide,
    initialized: bool,
}
impl RangeBar {
    fn new(labels: &[&str], selected: usize, owner: WeakEntity<Demo>, cx: &App) -> Self {
        Self {
            labels: labels.iter().map(|t| t.to_string().into()).collect(),
            owner,
            focus: (0..labels.len()).map(|_| cx.focus_handle()).collect(),
            selected,
            slide: Slide::new(0.),
            width: Slide::new(0.),
            initialized: false,
        }
    }
    fn choose(&mut self, i: usize, cx: &mut Context<Self>) {
        if self.selected == i {
            return;
        }
        self.selected = i;
        let _ = self.owner.update(cx, |d, cx| {
            if let Some(f) = d.action.clone() {
                f(d, i, cx);
                cx.notify();
            }
        });
        cx.notify();
    }
}
impl Render for RangeBar {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let widths: Vec<f32> = self
            .labels
            .iter()
            .map(|s| {
                (kui::primitives::family::measure(s, 12., w) + 20.)
                    .ceil()
                    .max(34.)
            })
            .collect();
        let x = widths.iter().take(self.selected).sum::<f32>();
        if !self.initialized {
            self.initialized = true;
            self.slide.reset(x);
            self.width.reset(widths[self.selected]);
        }
        self.slide.duration(x, 0.26, cx);
        self.width.duration(widths[self.selected], 0.26, cx);
        row()
            .id("range-filter")
            .role(Role::RadioGroup)
            .flex_none()
            .relative()
            .rounded(kui::foundation::surface_radius(cx))
            .p(px(3.))
            .bg(p.tray)
            .child(
                div()
                    .absolute()
                    .top(px(3.))
                    .left(px(3. + self.slide.get(w, cx)))
                    .w(px(self.width.get(w, cx)))
                    .h(px(28.))
                    .rounded(kui::foundation::surface_radius(cx))
                    .bg(p.panel),
            )
            .children(self.labels.iter().enumerate().map(|(i, l)| {
                kui::primitives::family::face(("range", i), l.clone(), 28., 5., Tone::Clear, cx)
                    .w(px(widths[i]))
                    .role(Role::RadioButton)
                    .aria_toggled((i == self.selected).into())
                    .track_focus(&self.focus[i])
                    .tab_stop(i == self.selected)
                    .text_size(px(12.))
                    .child(l.clone())
                    .on_click(cx.listener(move |s, _, _, cx| s.choose(i, cx)))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                        let n = s.labels.len();
                        let next = match e.keystroke.key.as_str() {
                            "left" => (i + n - 1) % n,
                            "right" => (i + 1) % n,
                            _ => return,
                        };
                        s.focus[next].focus(w, cx);
                        s.choose(next, cx);
                        cx.stop_propagation();
                    }))
            }))
    }
}
struct Demo {
    story: SharedString,
    views: Vec<AnyView>,
    buttons: Vec<SharedString>,
    action: Option<Action>,
    selected: Option<usize>,
    heading: SharedString,
    note: SharedString,
    rows: usize,
    labels: Vec<SharedString>,
    counter: usize,
    limit: f32,
    _subscriptions: Vec<Subscription>,
    timer: Option<Task<()>>,
    is_skeleton: bool,
    skeleton: Option<Entity<Skeleton>>,
    loaded: Slide,
}
impl Demo {
    fn options(
        &mut self,
        labels: &[&str],
        act: impl Fn(&mut Demo, usize, &mut Context<Demo>) + 'static,
    ) {
        self.buttons = labels.iter().map(|x| x.to_string().into()).collect();
        self.action = Some(Rc::new(act))
    }
    fn reload(&mut self, cx: &mut Context<Self>) {
        self.is_skeleton = true;
        self.loaded.duration(0., 0.16, cx);
        if let Some(s) = &self.skeleton {
            s.update(cx, |s, cx| s.restart(cx));
        }
        self.timer = Some(cx.spawn(async move |v, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1650))
                .await;
            let _ = v.update(cx, |s, cx| {
                s.is_skeleton = false;
                s.loaded.duration(1., 0.24, cx);
                cx.notify()
            });
        }));
        cx.notify()
    }
}
impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        if self.story.as_ref() == "toast" {
            let label = self.buttons[0].clone();
            return column()
                .items_center()
                .gap_4()
                .h(px(146.))
                .w(px(380.))
                .max_w_full()
                .child(
                    toned_command("save-draft", label, Tone::Primary, cx, |s, _, cx| {
                        if let Some(a) = s.action.clone() {
                            a(s, 0, cx);
                            cx.notify();
                        }
                    })
                    .text_size(px(12.)),
                )
                .children(self.views.iter().cloned());
        }
        let mut el = column().max_w_full().gap_3().items_center();
        let header_tools = matches!(self.story.as_ref(), "stepper" | "timeline");
        if !self.heading.is_empty() && !header_tools {
            el = el.child(title(self.heading.clone(), p))
        }
        if let Some(skel) = self.skeleton.as_ref() {
            let loaded = self.loaded.get(w, cx).clamp(0., 1.);
            el = el.child(
                div()
                    .relative()
                    .w(px(338.))
                    .max_w_full()
                    .h(px(64.))
                    .when(loaded < 0.999, |b| {
                        b.child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .w_full()
                                .opacity(1. - loaded)
                                .child(skel.clone()),
                        )
                    })
                    .when(loaded > 0.001, |b| {
                        b.child(
                            column()
                                .relative()
                                .top(px(4. * (1. - loaded)))
                                .gap_1()
                                .opacity(loaded)
                                .child(title(tr(cx, "Quarterly launch", "季度发布"), p))
                                .child(hint(
                                    tr(
                                        cx,
                                        "Emma shared the final interaction pass. Three files are ready for review.",
                                        "Emma 已分享最终交互稿。三个文件可以评审。",
                                    ),
                                    p,
                                )),
                        )
                    }),
            );
        } else {
            for (r, group) in self.views.chunks(self.rows.max(1)).enumerate() {
                let mut line = row().flex_wrap().justify_center().max_w_full().gap_2();
                for (j, v) in group.iter().enumerate() {
                    let n = r * self.rows + j;
                    line = line.child(
                        column()
                            .max_w_full()
                            .items_center()
                            .gap_1()
                            .child(v.clone())
                            .when_some(self.labels.get(n), |el, txt| {
                                if self.story.as_ref() == "avatar" {
                                    let (name, size) =
                                        txt.split_once(" · ").unwrap_or((txt.as_ref(), ""));
                                    el.w(px(132.))
                                        .child(title(name.to_owned(), p).text_size(px(13.)))
                                        .child(hint(size.to_owned(), p))
                                } else {
                                    el.child(hint(txt.clone(), p))
                                }
                            }),
                    )
                }
                el = el.child(line)
            }
        }
        let inline_note = matches!(self.story.as_ref(), "progress" | "skeleton");
        let chart_header = matches!(
            self.story.as_ref(),
            "bar-chart" | "line-chart" | "donut-chart" | "activity-heatmap"
        );
        if !self.buttons.is_empty() && !chart_header {
            let toolbar = row()
                .max_w_full()
                .flex_wrap()
                .justify_center()
                .gap_1()
                .when(self.selected.is_some(), |r| {
                    r.bg(p.tray)
                        .rounded(kui::foundation::surface_radius(cx))
                        .p(px(3.))
                })
                .when(inline_note, |r| {
                    r.w(px(if self.story.as_ref() == "progress" {
                        380.
                    } else {
                        338.
                    }))
                    .justify_start()
                })
                .when(self.story.as_ref() == "skeleton", |r| {
                    r.justify_between().child(hint(
                        if self.is_skeleton {
                            tr(cx, "Loading project summary", "正在载入项目摘要")
                        } else {
                            tr(cx, "Project summary loaded", "项目摘要已载入")
                        },
                        p,
                    ))
                })
                .children(self.buttons.iter().enumerate().map(|(i, label)| {
                    toned_command(
                        ("demo-option", i),
                        label.clone(),
                        if (self.story.as_ref() == "usage-meter" && i == 0)
                            || self.story.as_ref() == "timeline"
                        {
                            Tone::Primary
                        } else {
                            Tone::Surface
                        },
                        cx,
                        move |s, _, cx| {
                            if let Some(a) = s.action.clone() {
                                a(s, i, cx);
                                cx.notify()
                            }
                        },
                    )
                    .text_size(px(13.))
                    .when(self.selected.is_some(), |e| {
                        e.border_0()
                            .rounded(kui::foundation::surface_radius(cx))
                            .bg(if self.selected == Some(i) {
                                p.panel
                            } else {
                                gpui::transparent_black()
                            })
                    })
                }))
                .when(inline_note && !self.note.is_empty(), |r| {
                    r.child(hint(self.note.clone(), p))
                });
            if header_tools {
                let header = row()
                    .flex_wrap()
                    .justify_between()
                    .gap_1()
                    .child(title(self.heading.clone(), p))
                    .child(toolbar);
                el = frame(520., p).gap_4().child(header).child(el);
            } else if self.story.as_ref() == "skeleton" {
                el = frame(380., p).gap_2().child(el).child(toolbar);
            } else {
                el = el.child(toolbar);
            }
        }
        if !inline_note && !self.note.is_empty() {
            el = el.child(hint(self.note.clone(), p))
        }
        el
    }
}
fn participants() -> Vec<Person> {
    [
        "Emma Collins",
        "Marcus Johnson",
        "Ava Mitchell",
        "Ari Singh",
        "Sofia Ramirez",
    ]
    .into_iter()
    .enumerate()
    .map(|(i, n)| {
        if i == 3 {
            Person::new(n)
        } else {
            let (a,b,c)=[("#7c53b1","#e26094","#ffc46a"),("#62b39c","#65d1c2","#f281ba"),("#fa9d6b","#e8c956","#b8d686"),("#a6c0c9","#d1cbae","#a8b8ae")][i%4];
            let art=format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><defs><linearGradient id="a" x2=".75" y2=".9"><stop stop-color="{a}"/><stop offset=".55" stop-color="{b}"/><stop offset="1" stop-color="{c}"/></linearGradient></defs><rect width="64" height="64" fill="url(#a)"/></svg>"##);
            Person::new(n).with_image(Arc::new(Image::from_bytes(ImageFormat::Svg,art.into_bytes())))
        }
    })
    .collect()
}
fn allocations(l: UiLanguage, add: f32) -> Vec<Allocation> {
    [
        (l.text("Documents", "文档"), 4.2 + add),
        (l.text("Photos", "照片"), 5.1),
        (l.text("Backups", "备份"), 3.1),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (n, v))| Allocation::new(n, v, rgb([0x454545, 0x848484, 0xacacac][i]).into()))
    .collect()
}
fn samples(id: &str, period: usize, l: UiLanguage) -> Vec<Sample> {
    if id == "donut-chart" {
        return [
            ("Search", "搜索", 16840.),
            ("Direct", "直接访问", 8120.),
            ("Newsletter", "通讯订阅", 5260.),
            ("Referrals", "引荐", 6430.),
            ("Social", "社交", 2910.),
            ("Other", "其他", 1050.),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (en, zh, v))| {
            Sample::new(
                l.text(en, zh),
                v * [0.24, 1., 10.2][period] * (1. + (i as f32) * 0.03 * period as f32),
            )
        })
        .collect();
    }
    use kui::primitives::form_dates::CivilDate;
    if id == "activity-heatmap" {
        let jan = CivilDate::new(2025 - period as i32, 1, 1).expect("valid demo year");
        return chart_data::ACTIVITY[period]
            .iter()
            .enumerate()
            .map(|(i, &count)| {
                let day = jan.add_days(i as i32);
                Sample::new(
                    format!("{}-{:02}-{:02}", day.year, day.month, day.day),
                    count as f32,
                )
            })
            .collect();
    }
    if matches!(id, "line-chart" | "bar-chart") {
        let last = CivilDate::new(2026, 9, 21).expect("valid demo date");
        let label = |i: usize, n: usize| {
            let d = last.add_days(i as i32 + 1 - n as i32);
            if matches!(l, UiLanguage::Chinese) {
                return format!("{}月{}日", d.month, d.day);
            }
            let month = [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ][d.month as usize - 1];
            format!("{month} {}", d.day)
        };
        if id == "line-chart" {
            let values = chart_data::SIGNUPS[period];
            return values
                .iter()
                .enumerate()
                .map(|(i, &(a, b))| Sample::new(label(i, values.len()), a as f32).compare(b as f32))
                .collect();
        }
        let values = chart_data::MINUTES[period];
        return values
            .iter()
            .enumerate()
            .map(|(i, &v)| Sample::new(label(i, values.len()), v as f32))
            .collect();
    }
    if id == "sparkline" && period == 0 {
        return [1841., 2138., 2095., 2320., 2267., 2161., 2486.]
            .into_iter()
            .enumerate()
            .map(|(i, v)| Sample::new(format!("{} {}", l.text("Day", "日"), i + 1), v))
            .collect();
    }
    let count = [7, 30, 90][period];
    (0..count)
        .map(|i| {
            let value =
                128. + (i % 7) as f32 * 12. + (i as f32 * 1.83).sin() * 43. + i as f32 * 1.3;
            Sample::new(format!("{} {}", l.text("Day", "日"), i + 1), value)
        })
        .collect()
}
pub fn story(id: &str, w: &mut Window, cx: &mut Context<FamilyStory>) -> AnyView {
    let id = id.to_owned();
    cx.new(|cx: &mut Context<Demo>| {
        let l = UiLanguage::current(cx);
        let t = move |en, zh| l.text(en, zh);
        let mut d = Demo {
            story: id.clone().into(),
            views: vec![],
            buttons: vec![],
            action: None,
            selected: None,
            heading: "".into(),
            note: "".into(),
            rows: 1,
            labels: vec![],
            counter: 0,
            limit: 15.,
            _subscriptions: vec![],
            timer: None,
            is_skeleton: false,
            skeleton: None,
            loaded: Slide::new(0.),
        };
        match id.as_str() {
            "alert" => {
                let v = cx.new(|_| {
                    Alert::new(
                        t("Sync paused", "同步已暂停"),
                        t(
                            "3 changes are saved on this device. Reconnect to upload them.",
                            "有 3 项更改保存在此设备。重新连接后即可上传。",
                        ),
                        Severity::Warning,
                    )
                    .with_action(t("Reconnect", "重新连接"))
                });
                d._subscriptions
                    .push(cx.subscribe(&v, move |story, v, _: &AlertAction, cx| {
                        story.counter+=1;
                        let online=story.counter%2==1;
                        v.update(cx,|v,cx|{
                            v.replace(if online{
                                Alert::new(t("All changes synced","所有更改已同步"),t("Your 3 changes were uploaded to the shared workspace just now.","3 项更改刚刚已上传到共享工作区。"),Severity::Success).with_action(t("Pause sync","暂停同步"))
                            }else{
                                Alert::new(t("Sync paused","同步已暂停"),t("3 changes are saved on this device. Reconnect to upload them.","有 3 项更改保存在此设备。重新连接后即可上传。"),Severity::Warning).with_action(t("Reconnect","重新连接"))
                            },cx);
                        });
                    }));
                d.views.push(v.into());
            }
            "toast" => {
                let toast = cx.new(|_| {
                    Toast::new().with_description(t(
                        "Your work is ready to continue later.",
                        "草稿已保存，可稍后继续。",
                    ))
                });
                d._subscriptions
                    .push(cx.subscribe(&toast, move |s, _, event: &ToastEvent, cx| {
                        if *event == ToastEvent::Action {
                            s.note = t("Save undone", "已撤销保存").into();
                            cx.notify()
                        }
                    }));
                d.views.push(toast.clone().into());
                d.options(&[t("Save a draft", "保存草稿")], move |s, _, cx| {
                    s.note = "".into();
                    toast.update(cx, |v, cx| {
                        v.show(
                            t("Draft saved", "草稿已保存"),
                            None,
                            Duration::from_secs(5),
                            cx,
                        )
                    });
                });
            }
            "progress" => {
                let v = cx.new(|cx| {
                    Progress::new(
                        t("Preparing your workspace", "正在准备工作区"),
                        Some(0.68),
                        cx,
                    )
                });
                d.views.push(v.clone().into());
                d.note = t("Processing securely", "正在安全处理").into();
                d.counter = 68;
                d.options(&[t("Advance", "推进")], move |s, _, cx| {
                    s.counter = if s.counter == 100 {
                        0
                    } else {
                        (s.counter + 16).min(100)
                    };
                    s.buttons[0] = if s.counter == 100 {
                        t("Restart", "重新开始")
                    } else {
                        t("Advance", "推进")
                    }
                    .into();
                    s.note = if s.counter == 100 {
                        t("Workspace ready", "工作区已就绪")
                    } else {
                        t("Processing securely", "正在安全处理")
                    }
                    .into();
                    v.update(cx, |v, cx| v.set_value(Some(s.counter as f32 / 100.), cx));
                });
            }
            "skeleton" => {
                d.skeleton = Some(cx.new(|cx| {
                    Skeleton::new(
                        t("Loading project summary", "正在载入项目摘要"),
                        [0.48, 1., 0.72],
                        cx,
                    )
                    .with_avatar()
                }));
                d.options(&[t("Reload", "重新载入")], |s, _, cx| s.reload(cx));
                d.reload(cx);
            }
            "badge" => {
                d.rows = 5;
                let mut review = None;
                for (en, zh, tone, g) in [
                    ("Design system", "设计系统", BadgeTone::Neutral, None),
                    ("Beta", "测试版", BadgeTone::Info, None),
                    ("Paid", "已付款", BadgeTone::Success, None),
                    ("Trial ends soon", "试用将结束", BadgeTone::Warning, None),
                    ("Overdue", "已逾期", BadgeTone::Danger, None),
                    ("Draft", "草稿", BadgeTone::Neutral, Some(Glyph::Edit)),
                    ("In review", "审阅中", BadgeTone::Info, Some(Glyph::Clock)),
                    ("Ready", "就绪", BadgeTone::Success, Some(Glyph::Check)),
                    (
                        "Needs attention",
                        "需关注",
                        BadgeTone::Warning,
                        Some(Glyph::Alert),
                    ),
                    ("Failed", "失败", BadgeTone::Danger, Some(Glyph::Close)),
                    ("Archived", "已归档", BadgeTone::Neutral, None),
                    ("Syncing", "同步中", BadgeTone::Info, None),
                    ("Live", "在线", BadgeTone::Success, None),
                    ("Delayed", "延迟", BadgeTone::Warning, None),
                    ("Down", "离线", BadgeTone::Danger, None),
                    ("GET", "GET", BadgeTone::Info, None),
                    ("POST", "POST", BadgeTone::Success, None),
                    ("PUT", "PUT", BadgeTone::Warning, None),
                    ("PATCH", "PATCH", BadgeTone::Neutral, None),
                    ("DELETE", "DELETE", BadgeTone::Danger, None),
                ] {
                    let v = cx.new(|_| {
                        let b = Badge::new(t(en, zh), tone);
                        if let Some(g) = g { b.with_icon(g) } else { b }
                    });
                    if en == "In review" {
                        review = Some(v.clone())
                    }
                    d.views.push(v.into())
                }
                d.options(&[t("Approve review", "通过审阅")], move |s, _, cx| {
                    s.counter += 1;
                    s.buttons[0]=if s.counter%2==1{t("Reopen review","重新审阅")}else{t("Approve review","通过审阅")}.into();
                    if let Some(v) = &review {
                        v.update(cx, |v, cx| {
                            if s.counter % 2 == 1 {
                                v.set(t("Approved", "已通过"), BadgeTone::Success, cx)
                            } else {
                                v.set(t("In review", "审阅中"), BadgeTone::Info, cx)
                            }
                        })
                    }
                });
            }
            "card" => {
                let image = Arc::new(Image::from_bytes(
                    ImageFormat::Svg,
                    include_bytes!("../../assets/lounge.svg").to_vec(),
                ));
                let v = cx.new(|_| {
                    Card::new(
                        t("Lobby lounge", "大堂休息室"),
                        t(
                            "An armchair beside a sunlit window and a dark green wall.",
                            "深绿色的墙面旁，一把扶手椅沐浴在窗边的阳光下。",
                        ),
                    )
                    .with_cover(image)
                    .with_byline(t("Updated 2 hours ago", "2 小时前更新"))
                    .with_details(t("Space · Lobby, level 2", "空间 · 二层入口大厅"))
                });
                d._subscriptions
                    .push(cx.subscribe(&v, move |s, _, e: &CardEvent, cx| {
                        s.note = match e {
                            CardEvent::Open => return,
                            CardEvent::Saved(true) => t("Added to saved places", "已加入收藏地点"),
                            CardEvent::Saved(false) => t("Removed from saved places", "已取消收藏"),
                        }
                        .into();
                        cx.notify()
                    }));
                d.views.push(v.into());
            }
            "avatar" => {
                d.rows = 3;
                for (i, (person, size)) in
                    participants().into_iter().zip([28., 36., 48.]).enumerate()
                {
                    d.views.push(cx.new(|cx|{
                        let mut avatar=Avatar::new(person,size);
                        if i<2{avatar.set_status(Some(i==0),cx);}
                        avatar
                    }).into());
                    d.labels.push(
                        format!(
                            "{} · {}",
                            ["Emma Collins", "Marcus Johnson", "Ava Mitchell"][i],
                            [t("Small", "小"), t("Medium", "中"), t("Large", "大")][i]
                        )
                        .into(),
                    )
                }
            }
            "avatar-group" => {
                let people = participants();
                let v = cx.new(|_| AvatarGroup::new(people.clone(), 4));
                d.views.push(v.clone().into());
                d.counter = 5;
                d.options(&["−", "+"], move |s, i, cx| {
                    s.counter = if i == 0 {
                        s.counter.saturating_sub(1)
                    } else {
                        (s.counter + 1).min(8)
                    };
                    let mut people = people.clone();
                    people.extend(["Taylor Li", "Daniel Park", "Zoe Yang"].map(Person::new));
                    people.truncate(s.counter);
                    v.update(cx, |v, cx| v.set_people(people, cx));
                });
            }
            "usage-meter" => {
                let v = cx.new(|_| {
                    UsageMeter::new(t("Storage", "存储空间"), "GB", 15., allocations(l, 0.))
                });
                d.views.push(v.clone().into());
                d.options(
                    &[
                        t("Add files", "添加文件"),
                        t("Reset", "重置"),
                        "15 GB",
                        "50 GB",
                    ],
                    move |s, i, cx| {
                        match i {
                            0 => s.counter += 1,
                            1 => s.counter = 0,
                            2 => s.limit = 15.,
                            _ => s.limit = 50.,
                        }
                        v.update(cx, |v, cx| {
                            v.set_usage(s.limit, allocations(l, s.counter as f32 * 1.2), cx)
                        });
                    },
                );
            }
            "onboarding-checklist" => {
                let mut tasks = [
                    (
                        "Invite your team",
                        "邀请团队",
                        "Send invitations to your teammates.",
                        "向同事发送邀请。",
                        "Send invites",
                        "发送邀请",
                    ),
                    (
                        "Connect your repository",
                        "连接仓库",
                        "Select the repository for this workspace.",
                        "选择工作区使用的仓库。",
                        "Connect",
                        "连接",
                    ),
                    (
                        "Enable two-factor sign-in",
                        "启用双重验证",
                        "Add a second way to verify your identity.",
                        "添加第二种身份验证方式。",
                        "Enable",
                        "启用",
                    ),
                    (
                        "Create a workspace",
                        "创建工作区",
                        "Choose a name.",
                        "填写名称。",
                        "Create",
                        "创建",
                    ),
                ]
                .map(|(a, b, c, e, f, g)| ChecklistItem::new(t(a, b), t(c, e), t(f, g)))
                .to_vec();
                tasks[3].done = true;
                d.views.push(
                    cx.new(|_| SetupChecklist::new(t("Set up Northwind", "配置 Northwind"), tasks))
                        .into(),
                );
            }
            "stepper" => {
                let fields: Vec<_> = [
                    (t("Profile name", "姓名"), "Emma"),
                    (t("Workspace name", "工作区名称"), ""),
                    (t("Team email", "团队邮箱"), "studio@example.com"),
                ]
                .into_iter()
                .enumerate()
                .map(|(i, (label, value))| {
                    cx.new(|cx| {
                        let field = kui::components::input::Input::new(("wizard-field", i), label, w, cx)
                            .with_value(value, w, cx);
                        if i == 1 {
                            field.with_placeholder("Northwind Studio", cx).with_description("meridian.example/your-workspace")
                        } else {
                            field
                        }
                    })
                })
                .collect();
                d._subscriptions.push(cx.subscribe(&fields[1], |_, field, event, cx| {
                    if let kui::primitives::field::FieldEvent::Changed(name) = event {
                        let slug = name.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()).collect::<Vec<_>>().join("-").to_lowercase();
                        let slug = if slug.is_empty() { "your-workspace" } else { &slug };
                        field.update(cx, |f, cx| f.set_description(format!("meridian.example/{slug}"), cx));
                    }
                }));
                let v = cx.new(|_| {
                    Stepper::new(
                        [
                            ("Profile", "个人资料", "Your name and photo", "姓名和照片"),
                            ("Workspace", "工作区", "Name and web address", "名称和网址"),
                            ("Team", "团队", "Invite collaborators", "邀请协作者"),
                            ("Review", "审阅", "Confirm and finish", "确认并完成"),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(i, (a, b, c, e))| {
                            let step = Step::new(t(a, b), t(c, e));
                            if let Some(f) = fields.get(i) {
                                let field = f.clone();
                                let step = step.with_content(f.clone());
                                if i != 1 {
                                    return step;
                                }
                                step.validate_with(move |w, cx| {
                                    if !field.read(cx).value(cx).trim().is_empty() {
                                        return None;
                                    }
                                    field.read(cx).focus_handle(cx).focus(w, cx);
                                    Some(t("Enter a workspace name", "请输入工作区名称").into())
                                })
                            } else {
                                step
                            }
                        })
                        .collect(),
                        1,
                    )
                    .with_compact(true)
                });
                d._subscriptions
                    .push(cx.subscribe(&v, move |s, _, e: &StepEvent, cx| {
                        if matches!(e, StepEvent::Finished) {
                            s.note = t("Workspace ready", "工作区已就绪").into();
                            cx.notify()
                        }
                    }));
                d.views.push(v.clone().into());
                d.heading = t("Set up your workspace", "设置工作区").into();
                d.selected = Some(0);
                d.options(
                    &[t("Horizontal", "横向"), t("Vertical", "纵向")],
                    move |s, i, cx| {
                        s.selected = Some(i);
                        v.update(cx, |v, cx| v.set_vertical(i == 1, cx));
                    },
                );
            }
            "timeline" => {
                let v = cx.new(|_| {
                    Timeline::new(vec![
                        TimelineEntry::new(
                            t("Today", "今天"),
                            t("Emma merged the checkout redesign", "Emma 合并了结算页改版"),
                            "PR #482",
                            "11m",
                        ),
                        TimelineEntry::new(
                            t("Today", "今天"),
                            t("Staging deploy passed", "预发布部署通过"),
                            "Build 1,284",
                            "1h",
                        ),
                        TimelineEntry::new(
                            t("Today", "今天"),
                            t("Ava commented on billing", "Ava 评论了账单设置"),
                            t("Design review", "设计审阅"),
                            "2h",
                        ),
                        TimelineEntry::new(
                            t("Friday", "周五"),
                            t("Release notes updated", "更新发布说明"),
                            "v2.8",
                            "17:40",
                        ),
                    ])
                });
                d.views.push(v.clone().into());
                d.heading = t("Checkout redesign", "结算页改版").into();
                d.options(&[t("Add update", "添加动态")], move |s, _, cx| {
                    s.counter += 1;
                    v.update(cx, |v, cx| {
                        v.prepend(
                            TimelineEntry::new(
                                t("Today", "今天"),
                                format!("{} {}", t("Project update", "项目动态"), s.counter),
                                t("Published to the team timeline", "已发布到团队时间线"),
                                t("Now", "刚刚"),
                            ),
                            cx,
                        )
                    });
                });
            }
            "metric-card" => {
                let v = cx.new(|_| {
                    MetricCard::new(
                        t("Weekly active users", "每周活跃用户"),
                        2486.,
                        18.4,
                        t("Compared with the last 7 days", "与前 7 天比较"),
                    )
                });
                d.views.push(v.clone().into());
                d.selected = Some(0);
                d.options(&["7D", "30D", "90D"], move |s, i, cx| {
                    s.selected = Some(i);
                    v.update(cx, |v, cx| {
                        v.update_metric(
                            [2486., 8273., 21647.][i],
                            [18.4, 9.2, -2.6][i],
                            format!(
                                "{} {} {}",
                                t("Previous", "此前"),
                                [7, 30, 90][i],
                                t("days", "天")
                            ),
                            cx,
                        )
                    });
                });
            }
            _ => {
                macro_rules! chart {
                    ($ty:ty,$title:expr,$labels:expr,$idx:expr) => {{
                        let index = $idx;
                        let header = if id == "sparkline" {
                            None
                        } else {
                            let owner = cx.weak_entity();
                            Some(cx.new(|cx| RangeBar::new(&$labels, index, owner, cx)))
                        };
                        let v = cx.new(|_| {
                            let graph = <$ty>::new($title, samples(&id, index, l));
                            match header { Some(view) => graph.with_header(view), None => graph }
                        });

                        d.views.push(v.clone().into());
                        d.selected = Some(index);
                        d.options(&$labels, move |s, i, cx| {
                            s.selected = Some(i);
                            v.update(cx, |v, cx| v.set_samples(samples(&id, i, l), cx));
                        });
                    }};
                }
                match id.as_str() {
                    "sparkline" => chart!(
                        Sparkline,
                        t("Daily active users", "每日活跃用户"),
                        ["1W", "1M", "1Y"],
                        0
                    ),
                    "bar-chart" => chart!(
                        BarChart,
                        t("Active minutes", "活跃分钟数"),
                        [t("Week", "周"), t("Month", "月")],
                        0
                    ),
                    "line-chart" => chart!(
                        LineChart,
                        t("Trial signups", "试用注册"),
                        [
                            t("7 days", "7 天"),
                            t("30 days", "30 天"),
                            t("90 days", "90 天")
                        ],
                        1
                    ),
                    "activity-heatmap" => {
                        use kui::primitives::form_dates::CivilDate;
                        let weekday = |period: usize| {
                            CivilDate::new(2025 - period as i32, 1, 1)
                                .unwrap()
                                .weekday()
                        };
                        let owner=cx.weak_entity();
                        let header = cx.new(|cx| RangeBar::new(&["2025","2024","2023"],0,owner,cx));
                        let v = cx.new(|cx| {
                            let mut graph = DailyActivity::new(
                                t("Contributions", "贡献记录"),
                                samples(&id, 0, l),
                            ).with_header(header);
                            graph.set_first_weekday(weekday(0), cx);
                            graph
                        });
                        d.views.push(v.clone().into());
                        d.selected = Some(0);
                        d.options(&["2025", "2024", "2023"], move |s, period, cx| {
                            s.selected = Some(period);
                            v.update(cx, |graph, cx| {
                                graph.set_first_weekday(weekday(period), cx);
                                graph.set_samples(samples(&id, period, l), cx);
                            });
                        });
                    }
                    "donut-chart" => chart!(
                        DonutChart,
                        t("Visits by source", "各来源访问量"),
                        [t("Week", "周"), t("Month", "月"), t("Year", "年")],
                        1
                    ),
                    _ => unreachable!("unregistered insight story"),
                }
            }
        }
        d
    })
    .into()
}
