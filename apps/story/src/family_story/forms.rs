//! Host examples for the form controls. Only this story simulates upload work.
use super::FamilyStory;
use gpui::{prelude::*, *};
use kui::{
    components::*,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{Paint, Tone, face},
        form_choices::{FormOption, SelectionChanged},
        form_chrome::*,
        form_dates::{CivilDate, DateSpan},
        form_files::PickedFile,
    },
};
use std::{rc::Rc, time::Duration};
mod shortcuts;
struct RecentEmoji(kui::components::emoji_picker::SymbolPreferences);
impl gpui::Global for RecentEmoji {}
type Action = Rc<dyn Fn(&mut Demo, &mut Window, &mut Context<Demo>)>;
struct Demo {
    id: String,
    views: Vec<AnyView>,
    actions: Vec<(String, Action)>,
    text: String,
    filters: Vec<SharedString>,
    single: bool,
    fail: bool,
    messages: Vec<String>,
    symbols: Vec<String>,
    lang: UiLanguage,
    phone_valid: bool,
    volume: f64,
    prior_volume: f64,
    price: f64,
    _subs: Vec<Subscription>,
}
impl Demo {
    fn notice(&mut self, s: impl Into<String>, cx: &mut Context<Self>) {
        self.text = s.into();
        cx.notify()
    }
}
fn t_option(key: &str, en: &'static str, zh: &'static str, l: UiLanguage) -> FormOption {
    FormOption::new(key.to_owned(), l.text(en, zh))
}
fn sample_photo(i: usize) -> PickedFile {
    let bytes: &[u8] = match i {
        0 => include_bytes!("forms/sample-0.png"),
        1 => include_bytes!("forms/sample-1.png"),
        _ => include_bytes!("forms/sample-2.png"),
    };
    PickedFile {
        name: format!("studio-{}.png", i + 1),
        size: bytes.len() as u64,
        bytes: bytes.into(),
    }
}
pub fn story(id: &str, w: &mut Window, cx: &mut Context<FamilyStory>) -> AnyView {
    if id == "shortcut-recorder" {
        return cx.new(|cx| shortcuts::ShortcutDemo::new(w, cx)).into();
    }
    cx.new(|cx| build(id, w, cx)).into()
}
fn build(id: &str, w: &mut Window, cx: &mut Context<Demo>) -> Demo {
    let l = UiLanguage::current(cx);
    let t = move |a: &'static str, b: &'static str| l.text(a, b);
    let mut s = Demo {
        id: id.into(),
        views: vec![],
        actions: vec![],
        text: String::new(),
        filters: vec![],
        single: false,
        fail: false,
        messages: vec![],
        symbols: vec![],
        lang: l,
        phone_valid: false,
        volume: 60.,
        prior_volume: 60.,
        price: 100.,
        _subs: vec![],
    };
    macro_rules! view {
        ($e:expr) => {
            s.views.push($e.into())
        };
    }
    macro_rules! action {
        ($n:expr,$e:expr) => {
            s.actions.push(($n.to_string(), Rc::new($e)))
        };
    }
    let today = CivilDate::today_utc();
    match id {
        "checkbox" => {
            use checkbox::*;
            let e = cx.new(|_| {
                Checkbox::new(
                    t("Include archived projects", "包含归档项目"),
                    CheckValue::On,
                )
                .with_description(t(
                    "Show projects that are no longer active.",
                    "显示不再活跃的项目。",
                ))
            });
            view!(e);
        }
        "multi-select" => {
            let items = [
                ("type", "Typography", "排版"),
                ("motion", "Motion", "动效"),
                ("layout", "Layout", "布局"),
                ("color", "Color", "色彩"),
                ("accessibility", "Accessibility", "无障碍"),
            ]
            .into_iter()
            .map(|(k, a, b)| t_option(k, a, b, l))
            .collect();
            let e = cx.new(|cx| {
                multi_select::MultiSelect::new(t("Focus areas", "评审重点"), items, w, cx)
                    .with_selected(&["motion"])
                    .with_description(t(
                        "Pick the areas to include in the review.",
                        "选择要纳入评审的领域。",
                    ))
            });
            view!(e);
        }
        "number-field" => {
            use number_field::*;
            let seats = cx.new(|cx| {
                NumberField::new(t("Seats", "席位"), 8., 1.0..=100.0, 1., w, cx)
                    .with_suffix(t("seats", "席位"))
                    .with_description(t(
                        "$100 per month, billed monthly",
                        "每月 100 美元，按月计费",
                    ))
            });
            s._subs.push(cx.subscribe(&seats, move |s, e, v: &f64, cx| {
                s.price = v * 12.5;
                let msg = if l == UiLanguage::Chinese {
                    format!("每月 {} 美元，按月计费", s.price)
                } else {
                    format!("${} per month, billed monthly", s.price)
                };
                e.update(cx, |e, cx| e.set_description(msg, cx));
                cx.notify()
            }));
            view!(seats);
            let passes = cx.new(|cx| {
                NumberField::new(t("Guest passes", "访客通行证"), 1., 0.0..=3.0, 1., w, cx)
                    .with_description(t(
                        "$4 each, up to 3 per workspace",
                        "每份 4 美元，每个工作区最多 3 份",
                    ))
            });
            view!(passes);
        }
        "tag-input" => {
            let e = cx.new(|cx| {
                tag_input::TagInput::new(t("Project tags", "项目标签"), w, cx)
                    .with_tags(&["release", "web"])
            });
            view!(e);
        }
        "radio-group" => {
            let items = [
                (
                    "focused",
                    "Focused review",
                    "专注评审",
                    "One reviewer at a time.",
                    "一次由一人评审。",
                ),
                (
                    "open",
                    "Open review",
                    "开放评审",
                    "Everyone can comment together.",
                    "所有人都能一起评论。",
                ),
                (
                    "async",
                    "Async review",
                    "异步评审",
                    "Reviewers leave notes when they can.",
                    "成员可在方便时留下意见。",
                ),
            ]
            .into_iter()
            .map(|(k, a, b, c, d)| t_option(k, a, b, l).detail(t(c, d)))
            .collect();
            let e = cx.new(|cx| {
                radio_group::RadioGroup::new(t("Review mode", "评审模式"), items, w, cx)
                    .with_selected(&["focused"])
            });
            view!(e);
        }
        "segmented-control" => {
            let items = [
                ("review", "Review", "评审"),
                ("activity", "Activity", "动态"),
                ("files", "Files", "文件"),
            ]
            .into_iter()
            .map(|(k, a, b)| t_option(k, a, b, l))
            .collect();
            let e = cx.new(|cx| {
                segmented_control::SegmentedControl::new(
                    t("Project view", "项目视图"),
                    items,
                    w,
                    cx,
                )
                .with_selected(&["review"])
            });
            view!(e);
        }
        "chip-group" => {
            let items = [
                ("typography", "Typography", "排版"),
                ("color", "Color", "色彩"),
                ("motion", "Motion", "动效"),
                ("accessibility", "Accessibility", "无障碍"),
                ("layout", "Layout", "布局"),
                ("components", "Components", "组件"),
                ("systems", "Systems", "系统"),
                ("writing", "Writing", "写作"),
                ("research", "Research", "研究"),
                ("brand", "Brand", "品牌"),
                ("icons", "Icons", "图标"),
            ]
            .into_iter()
            .map(|(k, a, b)| t_option(k, a, b, l))
            .collect();
            let e = cx.new(|cx| {
                chip_group::ChipGroup::new(t("Topics", "主题"), items, w, cx)
                    .with_selected(&["motion"])
            });
            s.filters = vec!["motion".into()];
            s._subs
                .push(cx.subscribe(&e, |s, _, v: &SelectionChanged, cx| {
                    s.filters = v.0.clone();
                    cx.notify()
                }));
            for (name, single) in [
                (t("Any topic", "任意主题"), false),
                (t("One topic", "单一主题"), true),
            ] {
                let f = e.clone();
                action!(name, move |s, _, cx| {
                    s.single = single;
                    f.update(cx, |x, cx| x.set_multiple(!single, cx));
                    cx.notify()
                })
            }
            let f = e.clone();
            action!(t("Clear filters", "清空筛选"), move |_, _, cx| f
                .update(cx, |s, cx| s.clear(cx)));
            view!(e);
        }
        "slider" => {
            use slider::*;
            let budget = cx.new(|cx| {
                Slider::new(
                    t("Budget", "预算"),
                    0.0..=10000.0,
                    250.,
                    SlideValue {
                        low: 2000.,
                        high: Some(8000.),
                    },
                    cx,
                )
                .with_prefix("$")
            });
            s.text = t(
                "23 of 32 shortlisted studios quote in this range",
                "32 家候选工作室中有 23 家报价在此范围内",
            )
            .into();
            s._subs
                .push(cx.subscribe(&budget, move |s, _, v: &SlideValue, cx| {
                    let prices = [
                        300, 700, 1200, 1500, 1900, 2000, 2150, 2300, 2450, 2600, 2850, 3100, 3300,
                        3700, 3900, 4100, 4300, 4500, 4900, 5000, 5400, 5700, 6200, 6500, 7100,
                        7500, 7900, 8000, 8200, 8700, 9300, 10000,
                    ];
                    let n = prices
                        .iter()
                        .filter(|&&p| {
                            f64::from(p) >= v.low && f64::from(p) <= v.high.unwrap_or(10000.)
                        })
                        .count();
                    s.notice(
                        if l == UiLanguage::Chinese {
                            format!("32 家候选工作室中有 {n} 家报价在此范围内")
                        } else {
                            format!("{n} of 32 shortlisted studios quote in this range")
                        },
                        cx,
                    )
                }));
            view!(budget);
            let vol = cx.new(|cx| {
                Slider::new(
                    t("Volume", "音量"),
                    0.0..=100.0,
                    1.,
                    SlideValue {
                        low: 60.,
                        high: None,
                    },
                    cx,
                )
                .with_suffix("%")
            });
            s._subs.push(cx.subscribe(&vol, |s, _, v: &SlideValue, cx| {
                s.volume = v.low;
                cx.notify()
            }));
            let f = vol.clone();
            action!(t("Mute / unmute", "静音／恢复"), move |s, _, cx| {
                let n = if s.volume > 0. {
                    s.prior_volume = s.volume;
                    0.
                } else {
                    s.prior_volume.max(1.)
                };
                f.update(cx, |v, cx| v.set_low(n, cx))
            });
            view!(vol);
        }
        "mention-input" => {
            use mention_input::*;
            let roster = [
                (
                    "emma",
                    "Emma Collins",
                    '@',
                    t("Product designer", "产品设计师"),
                ),
                (
                    "marcus",
                    "Marcus Johnson",
                    '@',
                    t("Frontend engineer", "前端工程师"),
                ),
                (
                    "jasmine",
                    "Jasmine Brooks",
                    '@',
                    t("Design lead", "设计负责人"),
                ),
                (
                    "olivia",
                    "Olivia Bennett",
                    '@',
                    t("Platform engineer", "平台工程师"),
                ),
                (
                    "sofia",
                    "Sofia Ramirez",
                    '@',
                    t("Operations lead", "运营负责人"),
                ),
                (
                    "ryan",
                    "Ryan Sullivan",
                    '@',
                    t("Account executive", "客户经理"),
                ),
                ("launch", "launch", '#', t("Release planning", "发布计划")),
                ("design", "design", '#', t("Design reviews", "设计评审")),
                (
                    "engineering",
                    "engineering",
                    '#',
                    t("Engineering updates", "工程动态"),
                ),
                (
                    "general",
                    "general",
                    '#',
                    t("Workspace announcements", "工作区公告"),
                ),
            ];
            let items = roster
                .iter()
                .map(|(id, name, trigger, _)| Mention {
                    id: (*id).into(),
                    label: (*name).into(),
                    trigger: *trigger,
                })
                .collect();
            let e = cx.new(|cx| {
                MentionInput::new(t("Write an update", "撰写动态"), items, w, cx).with_details(
                    roster
                        .into_iter()
                        .map(|(id, _, _, detail)| (id.into(), detail.into())),
                )
            });
            s._subs.push(cx.subscribe(&e, |s, _, v: &MentionEvent, cx| {
                if let MentionEvent::Submitted(msg) = v {
                    s.messages.push(msg.clone());
                    cx.notify()
                }
            }));
            view!(e);
        }
        "emoji-picker" => {
            use emoji_picker::*;
            let edit = cx.new(|cx| {
                let mut e = Editor::new(t("Message", "消息"), InputMode::InlineMulti, w, cx);
                e.font_size = 14.;
                e.set_placeholder(t("Message", "消息"));
                e.set_value(t("Looks great, ship it ", "看起来不错，发布吧 "), w, cx);
                e
            });
            let e = cx.new(|cx| {
                SymbolPicker::standard(w, cx).with_preferences(
                    cx.try_global::<RecentEmoji>()
                        .map(|v| v.0.clone())
                        .unwrap_or_default(),
                )
            });
            s._subs
                .push(cx.subscribe(&e, |_, _, prefs: &SymbolPreferences, cx| {
                    cx.set_global(RecentEmoji(prefs.clone()))
                }));
            s._subs.push(
                cx.subscribe_in(&edit, w, move |s, e, ev: &EditEvent, w, cx| {
                    if matches!(ev, EditEvent::Submitted) {
                        let draft = e.read(cx).value().trim().to_owned();
                        if !draft.is_empty() {
                            s.messages.push(draft);
                            e.update(cx, |q, cx| q.set_value("", w, cx));
                        }
                    }
                    cx.notify();
                }),
            );
            let input = edit.clone();
            s._subs.push(
                cx.subscribe_in(&e, w, move |s, picker, v: &SymbolPicked, w, cx| {
                    s.symbols.push(v.0.to_string());
                    let closed = !picker.read(cx).open;
                    input.update(cx, |e, cx| {
                        e.replace_text_in_range(None, &v.0, w, cx);
                        if closed {
                            e.focus(w, cx)
                        }
                    });
                    cx.notify()
                }),
            );
            let input = edit.clone();
            action!(t("Send", "发送"), move |s, w, cx| {
                let msg = input.read(cx).value().trim().to_owned();
                if !msg.is_empty() {
                    s.messages.push(msg);
                    s.symbols.clear();
                    input.update(cx, |e, cx| e.set_value("", w, cx));
                    cx.notify()
                }
            });
            view!(edit);
            view!(e);
        }
        "calendar" => {
            let e = cx.new(|cx| {
                calendar::Calendar::new(t("Review calendar", "评审日历"), today, cx)
                    .with_value(DateSpan::new(today.add_days(3), today.add_days(3)))
            });
            view!(e);
        }
        "date-picker" => {
            let d = today.add_days(3);
            let e = cx.new(|cx| {
                date_picker::DatePicker::new(t("Launch date", "发布日期"), today, cx)
                    .with_bounds(Some(today), None)
                    .with_value(DateSpan::new(d, d))
                    .with_description(t(
                        "Pick the date for the review handoff.",
                        "选择评审交接的日期。",
                    ))
            });
            view!(e);
        }
        "date-range-picker" => {
            let e = cx.new(|cx| {
                date_range_picker::DateRangePicker::new("", today, cx)
                    .with_bounds(None, Some(today))
                    .with_value(DateSpan::new(today.add_days(-6), today))
            });
            view!(e);
        }
        "time-picker" => {
            use time_picker::*;
            let e = cx.new(|cx| {
                TimePicker::new(
                    t("Review time", "评审时间"),
                    WallTime::new(9, 30).unwrap(),
                    cx,
                )
                .with_description(t("Local time for the review.", "评审使用当地时间。"))
            });
            view!(e);
        }
        "color-picker" => {
            use color_picker::*;
            let e = cx.new(|cx| {
                ColorPicker::new(t("Accent", "强调色"), ColorValue(0x2f6bff), w, cx).with_swatches(
                    [0x17171a, 0x2f6bff, 0x61af84, 0xf7af65, 0x9f72e5]
                        .map(ColorValue)
                        .to_vec(),
                )
            });
            view!(e);
        }
        "phone-input" => {
            use phone_input::*;
            let e = cx.new(|cx| PhoneInput::new(t("Mobile number", "手机号码"), "US", w, cx));
            s._subs.push(cx.subscribe(&e, |s, _, v: &PhoneValue, cx| {
                s.phone_valid = v.possible;
                s.text.clear();
                cx.notify()
            }));
            let f = e.clone();
            action!(t("Send code", "发送验证码"), move |s, _, cx| {
                if s.phone_valid {
                    s.notice(
                        format!(
                            "{}: {}",
                            t("Demo verification", "模拟验证"),
                            f.read(cx).value(cx).international
                        ),
                        cx,
                    )
                }
            });
            view!(e);
        }
        "rich-text-editor" => {
            let md = t(
                "# Release notes\n\nVersion 2.4 ships **Thursday**. Select text to format it, or use / on an empty line for blocks.\n\n## What changed\n- Exports stream up to _one million_ rows\n- Shared views keep their filters\n- Comments on [dashboards](https://example.com/changelog)\n\n> Keep the public note under 120 words.",
                "# 版本说明\n\n2.4 版本将于**周四**发布。选择文本以更改格式，或在空行输入 / 插入内容块。\n\n## 更新内容\n- 流式导出支持_一百万_行\n- 共享视图保留筛选条件\n- 支持在[仪表盘](https://example.com/changelog)评论\n\n> 请将公开说明控制在 120 字以内。",
            );
            let e = cx.new(|cx| {
                rich_text_editor::RichTextEditor::new(t("Release notes", "版本说明"), md, w, cx)
            });
            view!(e);
        }
        "file-dropzone" => {
            use file_dropzone::*;
            let e = cx.new(|cx| {
                let mut d = FileDropzone::new(
                    t("Add launch assets", "添加发布素材"),
                    10 * 1024 * 1024,
                    8,
                    cx,
                )
                .with_extensions(&["pdf", "png", "jpg", "jpeg", "webp", "mov", "mp4"])
                .with_description(t(
                    "PDF, images or video up to 10 MB.",
                    "PDF、图片或视频，最大 10 MB。",
                ));
                for (name, sz, fail) in [
                    ("lounge-chair-hero.jpg", 1_800_000, false),
                    ("brand-guidelines.pdf", 2_400_000, false),
                    ("launch-video.mov", 8_600_000, true),
                ] {
                    d.seed(
                        PickedFile {
                            name: name.into(),
                            bytes: [].as_slice().into(),
                            size: sz,
                        },
                        if fail {
                            UploadState::Failed(t("Connection lost", "连接中断").into())
                        } else {
                            UploadState::Uploaded
                        },
                    )
                }
                d
            });
            s._subs.push(cx.subscribe(&e, |_, ent, ev: &FileEvent, cx| {
                if let FileEvent::Upload { ticket, .. } = ev {
                    let ticket = *ticket;
                    let weak = ent.downgrade();
                    cx.spawn(async move |_, cx| {
                        for i in 1..=5 {
                            cx.background_executor()
                                .timer(Duration::from_millis(140))
                                .await;
                            if weak
                                .update(cx, |e, cx| {
                                    e.report(
                                        ticket,
                                        if i == 5 {
                                            UploadState::Uploaded
                                        } else {
                                            UploadState::Uploading(i as f32 * 0.2)
                                        },
                                        cx,
                                    )
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                    })
                    .detach()
                }
            }));
            view!(e);
        }
        "avatar-upload" => {
            use avatar_upload::*;
            let e = cx.new(|cx| {
                let mut v = AvatarUpload::new("Jordan Reyes", cx);
                v.set_photo(sample_photo(0));
                v
            });
            s._subs
                .push(cx.subscribe(&e, move |s, ent, ev: &AvatarEvent, cx| {
                    if let AvatarEvent::Upload { ticket, .. } = ev {
                        let ticket = *ticket;
                        let fail = std::mem::take(&mut s.fail);
                        let weak = ent.downgrade();
                        cx.spawn(async move |_, cx| {
                            for i in 1..=5 {
                                cx.background_executor()
                                    .timer(Duration::from_millis(145))
                                    .await;
                                if weak
                                    .update(cx, |e, cx| {
                                        e.report(ticket, i as f32 * 0.2, cx);
                                        if i == 5 {
                                            e.finish(
                                                ticket,
                                                if fail {
                                                    Err(t("Connection lost", "连接中断").into())
                                                } else {
                                                    Ok(())
                                                },
                                                cx,
                                            )
                                        }
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        })
                        .detach()
                    }
                }));
            for i in 0..3 {
                let f = e.clone();
                action!(
                    format!("{} {}", t("Photo", "照片"), i + 1),
                    move |_, _, cx| f.update(cx, |v, cx| v.select_photo(sample_photo(i), cx))
                )
            }
            for (name, file, size) in [
                (t("12 MB photo", "12 MB 照片"), "large.png", 12_000_000),
                (t("PDF file", "PDF 文件"), "example.pdf", 8),
            ] {
                let f = e.clone();
                action!(name, move |_, _, cx| f.update(cx, |v, cx| v.select_photo(
                    PickedFile {
                        name: file.into(),
                        bytes: b"%PDF-1.7".as_slice().into(),
                        size
                    },
                    cx
                )))
            }
            action!(t("Fail next upload", "下次上传失败"), |s, _, cx| {
                s.fail = !s.fail;
                cx.notify()
            });
            view!(e);
        }
        _ => unreachable!("Unknown form story: {id}"),
    }
    s
}
impl Demo {
    fn action_bar(&self, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        let mut bar = row()
            .when(self.id == "phone-input", |r| r.w(px(380.)).max_w_full())
            .flex_wrap()
            .gap(px(5.));
        for (i, (n, call)) in self.actions.iter().enumerate() {
            let f = call.clone();
            let primary = self.id == "phone-input" || self.id == "emoji-picker";
            let selected = (self.id == "chip-group" && i == usize::from(self.single))
                || (self.id == "avatar-upload" && self.fail && i + 1 == self.actions.len());
            let mut b = button(("action", i), n.clone(), cx)
                .when(primary, |b| b.bg(p.bright).text_color(p.inverse))
                .when(selected, |b| b.bg(p.hover));
            if self.id == "avatar-upload" && i < 3 {
                let preview = sample_photo(i);
                let img = std::sync::Arc::new(Image::from_bytes(
                    ImageFormat::Png,
                    preview.bytes.to_vec(),
                ));
                b = div()
                    .id(("sample-photo", i))
                    .role(Role::Button)
                    .tab_index(0)
                    .aria_label(n.clone())
                    .size(px(30.))
                    .cursor_pointer()
                    .child(gpui::img(img).rounded_full().size_full());
            }
            if self.id == "phone-input" {
                b = b.w_full()
            }
            if self.id == "phone-input" && !self.phone_valid {
                b = b.opacity(0.45).tab_stop(false).cursor_default()
            } else {
                b = b.on_click(cx.listener(move |s, _, w, cx| f(s, w, cx)))
            }
            bar = bar.child(b)
        }
        bar
    }
}
impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = self.lang;
        let t = move |en: &'static str, zh: &'static str| l.text(en, zh);
        let mut root = col()
            .w_full()
            .min_w_0()
            .max_w_full()
            .items_center()
            .justify_center()
            .gap(px(16.))
            .text_color(p.ink)
            .text_size(px(14.));
        let room = match self.id.as_str() {
            "multi-select" => 130.,
            "date-picker" => 435.,
            "time-picker" => 313.,
            "date-range-picker" => 317.,
            "phone-input" => 309.,
            "color-picker" => 400.,
            _ => 0.,
        };
        if room > 0. {
            root = root.min_h(px(room)).justify_start();
        }
        match self.id.as_str() {
            "chip-group" => {
                root = root.child(self.views[0].clone());
                let mut list = panel(cx).w(px(510.)).max_w_full();
                let mut rows = 0;
                for (k, a, b, by) in [
                    (
                        "motion",
                        "Spring motion cheat sheet",
                        "弹簧动效速查",
                        "Jasmine Brooks · Template",
                    ),
                    (
                        "accessibility motion",
                        "Reduced motion patterns",
                        "减少动态效果模式",
                        "Ava Mitchell · Guide",
                    ),
                    (
                        "motion components",
                        "Prototyping gestures",
                        "手势原型设计",
                        "Marcus Johnson · Video",
                    ),
                    (
                        "typography layout",
                        "Editorial type scales",
                        "编辑排版字号",
                        "Chloe Nguyen · Guide",
                    ),
                    (
                        "color brand",
                        "Designing color ramps",
                        "设计色阶",
                        "Jordan Reyes · Guide",
                    ),
                    (
                        "systems icons components",
                        "Icon spacing",
                        "图标间距",
                        "Ava Mitchell · Template",
                    ),
                    (
                        "writing research",
                        "Writing for feedback",
                        "反馈文案",
                        "Marcus Johnson · Guide",
                    ),
                ] {
                    if self.filters.is_empty()
                        || self
                            .filters
                            .iter()
                            .any(|v| k.split_whitespace().any(|k| k == v.as_ref()))
                    {
                        list = list.child(
                            row()
                                .justify_between()
                                .flex_wrap()
                                .child(t(a, b))
                                .child(hint(by, cx)),
                        );
                        rows += 1
                    }
                }
                if rows == 0 {
                    list = list.child(hint(t("No matching resources", "没有匹配的资料"), cx))
                }
                root = root.child(list).child(self.action_bar(cx))
            }
            "slider" => {
                root = root
                    .child(
                        section(420., cx)
                            .gap(px(4.))
                            .child(self.views[0].clone())
                            .child(hint(self.text.clone(), cx)),
                    )
                    .child(
                        section(420., cx)
                            .gap(px(2.))
                            .child(self.views[1].clone())
                            .child(row().justify_between().children(
                                ["0%", "25%", "50%", "75%", "100%"].map(|mark| hint(mark, cx)),
                            )),
                    )
                    .child(self.action_bar(cx))
            }
            "emoji-picker" => {
                let mut chat = panel(cx).w(px(478.)).max_w_full().p(px(12.)).gap(px(14.));
                for (i,(who,msg)) in [("Jordan Reyes",t("Launch checklist is green. Shipping the pricing page at 3pm unless anyone objects.","发布清单已完成。如果没有异议，定价页面将在三点上线。")),("Chloe Nguyen",t("Copy is final and the screenshots are swapped in.","文案已定稿，截图已更新。"))].into_iter().enumerate(){
                    let (a,b)=if i==0{(0xe94da8,0xa1c783)}else{(0x9475c9,0x8fda67)};
                    chat=chat.child(row().items_start().px(px(5.)).gap(px(12.)).child(div().size(px(28.)).flex_none().rounded_full().bg(linear_gradient(25.,linear_color_stop(rgb(a),0.),linear_color_stop(rgb(b),1.))))
                        .child(col().flex_1().min_w_0().gap(px(2.)).child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(who))
                        .child(div().text_size(px(14.)).line_height(px(20.)).text_color(p.secondary).child(msg))));
                }
                for msg in self.messages.iter().rev().take(3).rev() {
                    chat = chat.child(
                        row().justify_end().child(
                            div()
                                .px(px(9.))
                                .py(px(4.))
                                .rounded(kui::foundation::surface_radius(cx))
                                .bg(p.stage)
                                .child(msg.clone()),
                        ),
                    )
                }
                let composer = self.views[0].clone().downcast::<Editor>().ok().unwrap();
                let empty = composer.read(cx).value().trim().is_empty();
                let send = self.actions[0].1.clone();
                chat = chat.child(
                    panel(cx)
                        .w_full()
                        .p(px(10.))
                        .gap(px(8.))
                        .child(div().w_full().h(px(44.)).child(composer))
                        .child(
                            row()
                                .gap(px(8.))
                                .child(self.views[1].clone())
                                .child(
                                    hint(t("Shift picks several", "按住 Shift 可连续选择"), cx)
                                        .flex_1()
                                        .min_w_0(),
                                )
                                .child(
                                    face(
                                        "send",
                                        t("Send message", "发送消息"),
                                        32.,
                                        kui::foundation::surface_radius(cx).into(),
                                        Tone::Clear,
                                        cx,
                                    )
                                    .debug_selector(|| "Send message".to_owned())
                                    .p_0()
                                    .size(px(32.))
                                    .bg(if empty { p.stage } else { p.bright })
                                    .hover(move |v| v.bg(if empty { p.stage } else { p.bright }))
                                    .active(move |v| {
                                        v.bg(if empty {
                                            p.stage
                                        } else {
                                            p.bright.opacity(0.88)
                                        })
                                    })
                                    .when(empty, |v| v.tab_stop(false).cursor_default())
                                    .child(
                                        div()
                                            .debug_selector(|| "emoji-send-icon".to_owned())
                                            .size(px(16.))
                                            .child(kui::primitives::glyph::icon(
                                                kui::primitives::glyph::Glyph::ArrowUp,
                                                16.,
                                                if empty { p.faint } else { p.inverse },
                                            )),
                                    )
                                    .when(!empty, |v| {
                                        v.on_click(cx.listener(move |s, _, w, cx| send(s, w, cx)))
                                    }),
                                ),
                        ),
                );
                root = root.child(chat)
            }
            "mention-input" => {
                root = root.child(self.views[0].clone());
                for msg in self.messages.iter().rev().take(3).rev() {
                    root = root.child(panel(cx).w(px(470.)).max_w_full().child(msg.clone()))
                }
            }
            "avatar-upload" => {
                root = root.child(
                    panel(cx)
                        .w(px(500.))
                        .max_w_full()
                        .p(px(16.))
                        .gap(px(10.))
                        .child(
                            div()
                                .text_size(px(18.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(t("Profile", "个人资料")),
                        )
                        .child(hint(
                            t(
                                "Click or drop a photo on the avatar. Uploads are simulated.",
                                "点击头像或拖入图片。此示例模拟上传。",
                            ),
                            cx,
                        ))
                        .child(self.views[0].clone())
                        .child(hint(
                            t("Sample photos and failure cases", "样例照片和错误示例"),
                            cx,
                        ))
                        .child(self.action_bar(cx)),
                )
            }
            _ => {
                for view in &self.views {
                    root = root.child(view.clone())
                }
                if !self.actions.is_empty() {
                    root = root.child(self.action_bar(cx))
                }
                if !self.text.is_empty() {
                    root = root.child(hint(self.text.clone(), cx))
                }
            }
        }
        root
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Modifiers, TestAppContext, VisualTestContext};
    use kui::components::{date_range_picker, file_dropzone, tag_input};
    use kui::primitives::form_dates::{CivilDate, DateSpan};
    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor()
            .advance_clock(std::time::Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh()
        });
        w.run_until_parked();
    }
    fn click(w: &mut VisualTestContext, name: &'static str) {
        let p = w
            .debug_bounds(name)
            .unwrap_or_else(|| panic!("missing {name}"))
            .center();
        w.simulate_click(p, Modifiers::default());
        w.run_until_parked();
        tick(w, 600);
    }
    #[test]
    fn selected_chips_keep_real_eight_pixel_edges_and_fit_the_trigger() {
        use gpui::{px, size};
        use kui::components::multi_select::{FormOption, MultiSelect};
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (_, w) = app.add_window_view(|w, cx| {
            MultiSelect::new(
                "Topics",
                vec![
                    FormOption::new("type", "排版"),
                    FormOption::new("motion", "Motion"),
                    FormOption::new("long", "Accessibility and assistive technology"),
                ],
                w,
                cx,
            )
            .with_selected(&["type", "motion", "long"])
        });
        w.simulate_resize(size(px(240.), px(440.)));
        w.run_until_parked();
        let trigger = w.debug_bounds("Topics").unwrap();
        let mut bounds = vec![];
        for key in [
            "Selected value: type",
            "Selected value: motion",
            "Selected value: long",
        ] {
            let b = w.debug_bounds(key).unwrap();
            assert_eq!(b.size.height, px(24.));
            assert!(b.size.width > px(16.) && b.size.width <= px(144.));
            assert!(
                trigger.contains(&b.origin) && trigger.contains(&b.bottom_right()),
                "{b:?} {trigger:?}"
            );
            bounds.push(b);
        }
        assert!(
            bounds[2].top() >= bounds[0].bottom(),
            "wrapped chips: {bounds:?}"
        );
        w.update(|win, _| {
            let scale = win.scale_factor();
            let quads = win.painted_quads();
            for b in bounds {
                let surface = quads
                    .iter()
                    .find(|q| q.bounds == b.scale(scale) && q.corner_radii.top_left.0 > 0.)
                    .unwrap();
                for corner in [
                    surface.corner_radii.top_left,
                    surface.corner_radii.top_right,
                    surface.corner_radii.bottom_left,
                    surface.corner_radii.bottom_right,
                ] {
                    assert_eq!(corner.0 / scale, 8.);
                }
            }
        });
    }
    #[test]
    fn clearing_the_last_chip_reveals_the_placeholder_without_pushing_the_chip() {
        use kui::components::multi_select::{FormOption, MultiSelect};
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (control, w) = app.add_window_view(|w, cx| {
            MultiSelect::new("Topics", vec![FormOption::new("type", "排版")], w, cx)
                .with_selected(&["type"])
        });
        let start = w.debug_bounds("Selected value: type").unwrap();
        w.update(|_, cx| control.update(cx, |s, cx| s.clear(cx)));
        tick(w, 75);
        assert_eq!(w.debug_bounds("Selected value: type"), Some(start));
        tick(w, 1500);
        assert!(w.debug_bounds("Selected value: type").is_none());
    }
    #[test]
    fn segment_indicator_reverses_in_flight_and_reduced_motion_settles_immediately() {
        use kui::components::segmented_control::*;
        use kui::foundation::Theme;
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (_, w) = app.add_window_view(|w, cx| {
            SegmentedControl::new(
                "View",
                vec![
                    FormOption::new("a", "First"),
                    FormOption::new("b", "Second"),
                    FormOption::new("c", "Third"),
                ],
                w,
                cx,
            )
            .with_selected(&["a"])
        });
        let position = |w: &mut VisualTestContext| {
            f32::from(
                w.debug_bounds("Segment selection indicator")
                    .unwrap()
                    .center()
                    .x,
            )
        };
        let press = |w: &mut VisualTestContext, label: &'static str| {
            let point = w.debug_bounds(label).unwrap().center();
            w.simulate_click(point, Modifiers::default());
            w.run_until_parked();
        };
        let first = position(w);
        press(w, "Third");
        tick(w, 64);
        let middle = position(w);
        assert!(middle > first + 1.);
        press(w, "First");
        let reversal = position(w);
        assert!((reversal - middle).abs() < 1.);
        tick(w, 700);
        assert!((position(w) - first).abs() < 1.);
        w.update(|_, cx| Theme::global_mut(cx).reduced_motion = true);
        press(w, "Third");
        tick(w, 0);
        let end = position(w);
        assert!(end > middle + 1.);
        tick(w, 64);
        assert_eq!(position(w), end);
    }
    #[test]
    fn failed_upload_retries_with_new_ticket_and_ignores_stale_completion() {
        use file_dropzone::*;
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (dz, w) = app.add_window_view(|_, cx| {
            FileDropzone::new("Files", 4, 2, cx).with_extensions(&["pdf"])
        });
        let f = |name: &str, size: u64| PickedFile {
            name: name.into(),
            size,
            bytes: b"pdf".as_slice().into(),
        };
        w.update(|_, cx| {
            dz.update(cx, |c, cx| {
                c.enqueue(
                    vec![f("huge.pdf", 20), f("script.js", 3), f("good.pdf", 3)],
                    cx,
                );
                assert_eq!(c.rows().len(), 1);
                assert_eq!(c.rows()[0].file.name, "good.pdf");
                c.report(1, UploadState::Failed("offline".into()), cx);
            })
        });
        w.run_until_parked();
        tick(w, 600);
        click(w, "Retry");
        w.update(|_, cx| {
            dz.update(cx, |c, cx| {
                c.report(1, UploadState::Uploaded, cx);
                assert!(matches!(c.rows()[0].state, UploadState::Uploading(0.0)));
                c.report(2, UploadState::Uploaded, cx);
                assert!(matches!(c.rows()[0].state, UploadState::Uploaded));
                c.report(2, UploadState::Failed("late".into()), cx);
                assert!(matches!(c.rows()[0].state, UploadState::Uploaded));
            })
        });
    }
    #[test]
    fn rich_selection_formats_plain_text_and_undo_restores_both_exports() {
        use kui::components::rich_text_editor::RichTextEditor;
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (editor, w) =
            app.add_window_view(|w, cx| RichTextEditor::new("Draft", "One paragraph.", w, cx));
        click(w, "Draft · 1");
        w.simulate_keystrokes("cmd-a cmd-b");
        w.update(|_, cx| {
            let d = editor.read(cx).document();
            assert_eq!(d.text, "One paragraph.");
            assert_eq!(d.markdown, "**One paragraph.**");
            assert_eq!(d.html, "<p><strong>One paragraph.</strong></p>")
        });
        w.simulate_keystrokes("cmd-z");
        w.update(|_, cx| assert_eq!(editor.read(cx).document().markdown, "One paragraph."));
        w.simulate_keystrokes("cmd-shift-z");
        w.update(|_, cx| assert_eq!(editor.read(cx).document().markdown, "**One paragraph.**"));
    }
    #[test]
    fn emoji_toolbar_and_send_keep_the_same_optical_center_while_hovering_or_pressing() {
        use gpui::{MouseButton, px, size};
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (demo, w) = app.add_window_view(|w, cx| super::build("emoji-picker", w, cx));
        let centered = |w: &mut VisualTestContext, control: &'static str, icon: &'static str| {
            let b = w.debug_bounds(control).unwrap();
            let g = w.debug_bounds(icon).unwrap();
            assert!(
                (b.center().x - g.center().x).abs() <= px(0.5)
                    && (b.center().y - g.center().y).abs() <= px(0.5),
                "{control}: {b:?}, {g:?}"
            );
            assert_eq!(g.size, size(px(16.), px(16.)));
            b
        };
        for width in [628., 288.] {
            w.simulate_resize(size(px(width), px(650.)));
            tick(w, 40);
            let send = centered(w, "Send message", "emoji-send-icon");
            assert_eq!(send.size, size(px(32.), px(32.)));
            w.simulate_mouse_down(send.center(), MouseButton::Left, Modifiers::default());
            tick(w, 30);
            assert_eq!(centered(w, "Send message", "emoji-send-icon"), send);
            w.simulate_mouse_up(send.center(), MouseButton::Left, Modifiers::default());
            tick(w, 600);
            assert_eq!(w.update(|_, cx| demo.read(cx).messages.len()), 1);
            centered(w, "Send message", "emoji-send-icon");
            click(w, "Choose emoji");
            let cats = [
                "Popular",
                "Smileys and emotion",
                "People and body",
                "Animals and nature",
                "Food and drink",
                "Travel and places",
                "Activities",
                "Objects",
                "Symbols",
                "Flags",
            ];
            let icons = [
                "emoji-category-icon-0",
                "emoji-category-icon-1",
                "emoji-category-icon-2",
                "emoji-category-icon-3",
                "emoji-category-icon-4",
                "emoji-category-icon-5",
                "emoji-category-icon-6",
                "emoji-category-icon-7",
                "emoji-category-icon-8",
                "emoji-category-icon-9",
            ];
            for (name, icon) in cats.into_iter().zip(icons) {
                let cell = centered(w, name, icon);
                assert_eq!(cell.size.height, px(36.));
            }
            let smile = centered(w, "Smileys and emotion", "emoji-category-icon-1");
            w.simulate_mouse_move(smile.center(), None, Modifiers::default());
            tick(w, 40);
            assert_eq!(
                centered(w, "Smileys and emotion", "emoji-category-icon-1"),
                smile
            );
            w.simulate_mouse_down(smile.center(), MouseButton::Left, Modifiers::default());
            tick(w, 20);
            assert_eq!(
                centered(w, "Smileys and emotion", "emoji-category-icon-1"),
                smile
            );
            w.simulate_mouse_up(smile.center(), MouseButton::Left, Modifiers::default());
            tick(w, 0);
            tick(w, 32);
            tick(w, 800);
            let marker = w.debug_bounds("emoji-category-current").unwrap();
            assert!(
                (smile.center().x - marker.center().x).abs() <= px(1.),
                "{marker:?} vs {smile:?}"
            );
            assert_eq!(marker.size, size(px(18.), px(2.)));
            assert!(marker.bottom() <= smile.bottom());
            w.simulate_keystrokes("escape");
            tick(w, 600);
        }
    }
    #[test]
    fn symbol_grid_searches_full_host_dataset_and_respects_shift_pick() {
        use kui::components::emoji_picker::{Symbol, SymbolPicked, SymbolPicker};
        use std::{cell::RefCell, rc::Rc};
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (picker, w) = app.add_window_view(|w, cx| {
            SymbolPicker::new(
                (0..80)
                    .map(|i| Symbol {
                        text: format!("{i}").into(),
                        name: format!("Symbol {i}").into(),
                        category: if i % 2 == 0 { "even" } else { "odd" }.into(),
                        glyph: None,
                    })
                    .collect(),
                w,
                cx,
            )
        });
        let vals = Rc::new(RefCell::new(Vec::new()));
        let copy = vals.clone();
        let _sub = w.update(|_, cx| {
            cx.subscribe(&picker, move |_, ev: &SymbolPicked, _| {
                copy.borrow_mut().push(ev.0.to_string())
            })
        });
        click(w, "Choose emoji");
        click(w, "odd");
        click(w, "Search emoji");
        w.simulate_input("Symbol 7");
        w.simulate_keystrokes("end shift-enter");
        assert!(w.update(|_, cx| picker.read(cx).open));
        assert_eq!(&*vals.borrow(), &["79"]);
        w.simulate_keystrokes("home enter");
        assert!(!w.update(|_, cx| picker.read(cx).open));
        assert_eq!(&*vals.borrow(), &["79", "7"]);
    }
    #[test]
    fn rich_block_menu_and_link_edit_the_structured_document() {
        use kui::components::rich_text_editor::RichTextEditor;
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (editor, w) = app.add_window_view(|w, cx| RichTextEditor::new("Note", "Link", w, cx));
        click(w, "Note · 1");
        w.simulate_keystrokes("cmd-a cmd-k");
        w.simulate_input("example.org");
        w.simulate_keystrokes("enter");
        w.update(|_, cx| {
            assert_eq!(
                editor.read(cx).document().html,
                "<p><a href=\"https://example.org\">Link</a></p>"
            )
        });
        w.simulate_keystrokes("right enter");
        w.simulate_input("/quote");
        w.simulate_keystrokes("enter");
        w.simulate_input("新段落");
        w.update(|_, cx| {
            assert!(
                editor
                    .read(cx)
                    .document()
                    .html
                    .ends_with("<blockquote>新段落</blockquote>")
            );
            assert!(editor.read(cx).document().markdown.ends_with("> 新段落"))
        });
    }
    #[test]
    fn tag_backspace_requires_two_presses_in_empty_editor() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (tags, w) = app.add_window_view(|w, cx| {
            tag_input::TagInput::new("Tags", w, cx).with_tags(&["release", "web"])
        });
        click(w, "Tags");
        w.simulate_keystrokes("backspace");
        assert_eq!(w.update(|_, cx| tags.read(cx).tags().len()), 2);
        w.simulate_keystrokes("backspace");
        assert_eq!(w.update(|_, cx| tags.read(cx).tags().len()), 1);
    }
    #[test]
    fn slider_capture_resists_beyond_track_then_releases_and_regrabs_without_a_jump() {
        use gpui::{MouseButton, point, px};
        use kui::components::slider::{SlideValue, Slider};
        use kui::foundation::Theme;
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (slider, w) = app.add_window_view(|_, cx| {
            Slider::new(
                "Range",
                0.0..=100.0,
                1.0,
                SlideValue {
                    low: 50.,
                    high: None,
                },
                cx,
            )
        });
        let thumb = |w: &mut VisualTestContext| w.debug_bounds("Slider thumb 0").unwrap().center();
        let right = w.debug_bounds("Slider track").unwrap().right() - px(11.);
        let center = thumb(w);
        let outside = point(right + px(65.), center.y);
        let left = MouseButton::Left;
        let mods = Modifiers::default();
        w.simulate_mouse_down(center, left, mods);
        tick(w, 16);
        w.simulate_mouse_move(outside, left, mods);
        tick(w, 100);
        let stretched = thumb(w);
        assert!(
            stretched.x > right + px(6.) && stretched.x < right + px(9.),
            "stretch {stretched:?}, edge {right:?}"
        );
        assert_eq!(w.update(|_, cx| slider.read(cx).value().low), 100.);
        w.simulate_mouse_up(outside, left, mods);
        tick(w, 64);
        let returning = thumb(w);
        assert!(returning.x < stretched.x && returning.x > right);
        w.simulate_mouse_down(returning, left, mods);
        tick(w, 0);
        assert!((thumb(w).x - returning.x).abs() < px(1.));
        w.simulate_mouse_move(point(right - px(30.), center.y), left, mods);
        tick(w, 100);
        assert!(thumb(w).x < right);
        w.simulate_mouse_up(center, left, mods);
        w.update(|_, cx| Theme::global_mut(cx).reduced_motion = true);
        let current = thumb(w);
        w.simulate_mouse_down(current, left, mods);
        tick(w, 0);
        w.simulate_mouse_move(outside, left, mods);
        tick(w, 16);
        assert!((thumb(w).x - right).abs() < px(1.));
    }
    #[test]
    fn tag_reflow_crosses_rows_continuously_and_reverses_at_the_visible_position() {
        use gpui::{px, size};
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (_, w) = app.add_window_view(|w, cx| {
            tag_input::TagInput::new("Tags", w, cx).with_tags(&[
                "design system",
                "development",
                "review",
            ])
        });
        let chip = |w: &mut VisualTestContext| w.debug_bounds("Tag review").unwrap().origin;
        let resize = |w: &mut VisualTestContext, width: f32| {
            w.simulate_resize(size(px(width), px(400.)));
            tick(w, 0);
            tick(w, 0);
        };
        resize(w, 420.);
        tick(w, 700);
        let first = chip(w);
        resize(w, 184.);
        let start = chip(w);
        assert!((first.x - start.x).abs() < px(1.) && (first.y - start.y).abs() < px(1.));
        tick(w, 64);
        let mid = chip(w);
        assert!(
            mid.x < first.x - px(1.) && mid.y > first.y + px(1.),
            "start {first:?}, intermediate {mid:?}"
        );
        resize(w, 420.);
        let reverse = chip(w);
        assert!((reverse.x - mid.x).abs() < px(1.) && (reverse.y - mid.y).abs() < px(1.));
        tick(w, 800);
        let settled = chip(w);
        assert!((settled.x - first.x).abs() < px(1.) && (settled.y - first.y).abs() < px(1.));
        resize(w, 184.);
        tick(w, 800);
        let wrapped = chip(w);
        let remove = w.debug_bounds("Remove design system").unwrap().center();
        w.simulate_click(remove, Modifiers::default());
        tick(w, 0);
        tick(w, 0);
        tick(w, 64);
        let removing = chip(w);
        assert!(removing.y < wrapped.y && removing.y > first.y);
        tick(w, 800);
        assert!(chip(w).y < removing.y);
    }
    #[test]
    fn picker_time_wraps_midnight_releases_scroll_area_and_preserves_the_committed_dot() {
        use gpui::{Bounds, Pixels, px};
        use kui::components::time_picker::{TimePicker, WallTime};
        use kui::primitives::editor::EditorScrollRegion;
        use std::cell::Cell;
        thread_local! { static REGION: Cell<Bounds<Pixels>> = Cell::default(); }
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::init(cx);
            cx.set_global(EditorScrollRegion(|r, _, _| REGION.set(r)));
        });
        let initial = WallTime::new(9, 30).unwrap();
        let (picker, w) = app.add_window_view(|_, cx| TimePicker::new("Time", initial, cx));
        click(w, "time-trigger");
        let row = w.debug_bounds("9:30 AM").unwrap();
        let menu = w.debug_bounds("time-menu").unwrap();
        let dot = w.debug_bounds("time-selected-dot").unwrap();
        assert_eq!(row.size.height, px(36.));
        assert_eq!(menu.size.height, px(250.));
        assert_eq!(menu.size.width - row.size.width, px(12.));
        assert_eq!(dot.size.width, px(6.));
        assert!(REGION.get().size.height > px(200.));
        w.simulate_keystrokes("end down enter");
        tick(w, 600);
        assert_eq!(
            w.update(|_, cx| picker.read(cx).value()),
            WallTime::new(0, 0).unwrap()
        );
        assert_eq!(REGION.get(), Bounds::default());
        w.simulate_keystrokes("space");
        tick(w, 600);
        w.simulate_keystrokes("home up enter");
        tick(w, 600);
        assert_eq!(
            w.update(|_, cx| picker.read(cx).value()),
            WallTime::new(23, 45).unwrap()
        );
        click(w, "time-trigger");
        w.simulate_keystrokes("home escape");
        tick(w, 600);
        assert_eq!(REGION.get(), Bounds::default());
        assert_eq!(
            w.update(|_, cx| picker.read(cx).value()),
            WallTime::new(23, 45).unwrap()
        );
    }
    #[test]
    fn picker_calendar_keeps_week_headers_aligned_while_navigating_and_respects_minimum() {
        use gpui::{px, size};
        use kui::components::date_picker::DatePicker;
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let start = CivilDate::new(2026, 11, 1).unwrap();
        let (picker, w) = app.add_window_view(|_, cx| {
            DatePicker::new("Date", start, cx).with_bounds(Some(start), None)
        });
        w.simulate_resize(size(px(288.), px(520.)));
        tick(w, 0);
        click(w, "date-trigger");
        for (cell, label) in [
            ("calendar-cell-0-0-0", "weekday-0-0"),
            ("calendar-cell-0-0-1", "weekday-0-1"),
            ("calendar-cell-0-0-2", "weekday-0-2"),
            ("calendar-cell-0-0-3", "weekday-0-3"),
            ("calendar-cell-0-0-4", "weekday-0-4"),
            ("calendar-cell-0-0-5", "weekday-0-5"),
            ("calendar-cell-0-0-6", "weekday-0-6"),
        ] {
            let day = w.debug_bounds(cell).unwrap();
            let week = w.debug_bounds(label).unwrap();
            assert!(
                (day.size.width - day.size.height).abs() <= px(1.),
                "cell {cell}: {day:?}, weekday {week:?}"
            );
            assert!(
                (day.center().x - week.center().x).abs() < px(1.),
                "cell {cell}: {day:?}, weekday {week:?}"
            );
        }
        let header = w.debug_bounds("calendar-head-0").unwrap();
        let weekday = w.debug_bounds("weekday-0-0").unwrap();
        w.simulate_keystrokes("pagedown");
        tick(w, 64);
        assert_eq!(header, w.debug_bounds("calendar-head-0").unwrap());
        assert_eq!(weekday, w.debug_bounds("weekday-0-0").unwrap());
        tick(w, 800);
        w.simulate_keystrokes("pageup left enter");
        tick(w, 500);
        assert_eq!(
            w.update(|_, cx| picker.read(cx).value()),
            Some(DateSpan::new(start, start))
        );
        tick(w, 400);
        click(w, "date-trigger");
        click(w, "Clear");
        assert_eq!(w.update(|_, cx| picker.read(cx).value()), None);
    }
    #[test]
    fn picker_chinese_range_reserves_both_glyphs_and_a_full_cross_year_value() {
        use gpui::{px, size};
        use kui::primitives::control::UiLanguage;
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::init(cx);
            cx.set_global(UiLanguage::Chinese);
        });
        let start = CivilDate::new(2026, 12, 24).unwrap();
        let end = CivilDate::new(2027, 1, 7).unwrap();
        let (_, w) = app.add_window_view(|_, cx| {
            date_range_picker::DateRangePicker::new("范围", end, cx)
                .with_value(DateSpan::new(start, end))
        });
        w.simulate_resize(size(px(288.), px(520.)));
        tick(w, 50);
        let bounds = w.debug_bounds("date-trigger").unwrap();
        let start_icon = w.debug_bounds("date-leading").unwrap();
        let value = w.debug_bounds("date-value").unwrap();
        let end_icon = w.debug_bounds("date-trailing").unwrap();
        assert!(value.left() - start_icon.right() >= px(8.));
        assert!(end_icon.left() - value.right() >= px(8.));
        assert!(end_icon.right() + px(12.) <= bounds.right() && bounds.right() <= px(288.));
        let short = w.update(|w, _| {
            kui::primitives::family::measure(&"2026/12/24 – 2027/1/7".into(), 14., w)
        });
        assert!(f32::from(value.size.width) >= short);
        click(w, "date-trigger");
        assert_eq!(w.debug_bounds("date-trigger").unwrap(), bounds);
        w.simulate_keystrokes("escape");
        tick(w, 600);
        assert_eq!(w.debug_bounds("date-trigger").unwrap(), bounds);
    }
    #[test]
    fn picker_range_surface_keeps_shrinking_after_content_fades_and_reopens_in_flight() {
        use gpui::px;
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let today = CivilDate::new(2026, 11, 1).unwrap();
        let (_, w) = app.add_window_view(|_, cx| {
            date_range_picker::DateRangePicker::new("Range", today, cx)
                .with_value(DateSpan::new(today.add_days(-6), today))
        });
        click(w, "date-trigger");
        tick(w, 600);
        let full = w.debug_bounds("date-range-surface").unwrap();
        w.simulate_keystrokes("escape");
        tick(w, 160);
        let closing = w.debug_bounds("date-range-surface").unwrap();
        assert!(w.debug_bounds("Apply").is_none());
        assert!(closing.size.height > px(64.) && closing.size.height < full.size.height - px(20.));
        let trigger = w.debug_bounds("date-trigger").unwrap().center();
        w.simulate_click(trigger, Modifiers::default());
        tick(w, 0);
        let reversed = w.debug_bounds("date-range-surface").unwrap();
        assert!(
            (reversed.size.height - closing.size.height).abs() < px(1.),
            "was {closing:?}; reopened {reversed:?}"
        );
        tick(w, 750);
        assert!(w.debug_bounds("Apply").is_some());
        assert!(
            (w.debug_bounds("date-range-surface").unwrap().size.height - full.size.height).abs()
                <= px(1.)
        );
    }
    #[test]
    fn picker_range_value_travels_between_measured_slots_in_the_visible_surface() {
        use gpui::{px, size};
        use kui::foundation::Theme;
        use kui::primitives::control::UiLanguage;
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::init(cx);
            cx.set_global(UiLanguage::Chinese);
        });
        let end = CivilDate::new(2026, 11, 1).unwrap();
        let original = DateSpan::new(end.add_days(-6), end);
        let (picker, w) = app.add_window_view(|_, cx| {
            date_range_picker::DateRangePicker::new("", end, cx).with_value(original)
        });
        w.simulate_resize(size(px(288.), px(520.)));
        tick(w, 0);
        let name = "calendar-visible-value";
        let start = w.debug_bounds(name).unwrap();
        let trigger = w.debug_bounds("date-trigger").unwrap().center();
        w.simulate_click(trigger, Modifiers::default());
        tick(w, 0);
        tick(w, 80);
        let forward = w.debug_bounds(name).unwrap();
        let shell = w.debug_bounds("date-range-surface").unwrap();
        assert!(
            forward.top() > start.top() + px(8.),
            "opening {forward:?}, start {start:?}"
        );
        assert!(shell.contains(&forward.center()));
        tick(w, 1200);
        let footer = w.debug_bounds("range-summary-slot").unwrap();
        let landed = w.debug_bounds(name).unwrap();
        assert_eq!(landed.origin, footer.origin);
        w.simulate_keystrokes("escape");
        tick(w, 0);
        assert!((w.debug_bounds(name).unwrap().top() - landed.top()).abs() < px(1.));
        tick(w, 65);
        let returning = w.debug_bounds(name).unwrap();
        let shell = w.debug_bounds("date-range-surface").unwrap();
        assert!(returning.top() > start.top() && returning.top() < landed.top() - px(12.));
        assert!(shell.contains(&returning.center()));
        w.simulate_click(trigger, Modifiers::default());
        tick(w, 0);
        assert!((w.debug_bounds(name).unwrap().top() - returning.top()).abs() <= px(1.));
        tick(w, 1200);
        click(w, "昨天");
        click(w, "取消");
        tick(w, 700);
        assert_eq!(w.update(|_, cx| picker.read(cx).value()), Some(original));
        assert_eq!(w.debug_bounds(name).unwrap().origin, start.origin);
        w.update(|_, cx| Theme::global_mut(cx).reduced_motion = true);
        click(w, "date-trigger");
        let open_slot = w.debug_bounds("range-summary-slot").unwrap();
        assert_eq!(w.debug_bounds(name).unwrap().origin, open_slot.origin);
        click(w, "昨天");
        click(w, "应用");
        assert_eq!(
            w.update(|_, cx| picker.read(cx).value()),
            Some(DateSpan::new(end.add_days(-1), end.add_days(-1)))
        );
        assert!(w.debug_bounds("date-range-surface").is_none());
        let closed_slot = w.debug_bounds("date-value").unwrap();
        assert_eq!(w.debug_bounds(name).unwrap().origin, closed_slot.origin);
    }
    #[test]
    fn date_range_preset_is_a_draft_until_applied() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let today = CivilDate::new(2024, 3, 2).unwrap();
        let initial = DateSpan::new(today, today);
        let (picker, w) = app.add_window_view(|_, cx| {
            date_range_picker::DateRangePicker::new("Range", today, cx).with_value(initial)
        });
        click(w, "date-trigger");
        click(w, "Last 7 days");
        click(w, "Cancel");
        assert_eq!(w.update(|_, cx| picker.read(cx).value()), Some(initial));
        click(w, "date-trigger");
        click(w, "Last 7 days");
        click(w, "Apply");
        assert_eq!(
            w.update(|_, cx| picker.read(cx).value()),
            Some(DateSpan::new(CivilDate::new(2024, 2, 25).unwrap(), today))
        );
    }
}

#[cfg(test)]
mod composer_regressions {
    use super::{Duration, Rc, UiLanguage};
    use gpui::{Modifiers, TestAppContext, VisualTestContext};
    use kui::components::{
        emoji_picker::{SymbolPicked, SymbolPicker},
        mention_input::{Mention, MentionInput, MentionValue},
    };
    fn click(w: &mut VisualTestContext, label: &'static str) {
        let b = w.debug_bounds(label).expect(label);
        w.simulate_click(b.center(), Modifiers::default());
        w.run_until_parked()
    }
    fn settle(w: &mut VisualTestContext) {
        for _ in 0..15 {
            w.executor().advance_clock(Duration::from_millis(24));
            w.update(|w, cx| w.simulate_next_frame(cx));
            w.run_until_parked()
        }
    }
    #[test]
    fn mention_tokens_track_unicode_edits_and_post_once() {
        use std::cell::RefCell;
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (v, w) = app.add_window_view(|w, cx| {
            MentionInput::new(
                "Update",
                vec![
                    Mention {
                        id: "emma".into(),
                        label: "Emma Collins".into(),
                        trigger: '@',
                    },
                    Mention {
                        id: "launch".into(),
                        label: "launch".into(),
                        trigger: '#',
                    },
                ],
                w,
                cx,
            )
        });
        let sent = Rc::new(RefCell::new(vec![]));
        let got = sent.clone();
        let _sub = w.update(|_, cx| {
            cx.subscribe(&v, move |_, e: &MentionValue, _| {
                got.borrow_mut().push(e.clone())
            })
        });
        click(w, "Update");
        w.simulate_input("中文🙂");
        click(w, "Mention someone");
        settle(w);
        w.simulate_keystrokes("tab");
        settle(w);
        let val = w.update(|_, cx| v.read(cx).value(cx));
        assert_eq!(val.text, "中文🙂 @Emma Collins ");
        assert_eq!(&val.text[val.mentions[0].bytes.clone()], "@Emma Collins");
        w.simulate_keystrokes("left backspace");
        assert!(w.update(|_, cx| v.read(cx).value(cx)).mentions.is_empty());
        click(w, "Add a channel");
        settle(w);
        w.simulate_keystrokes("enter");
        settle(w);
        w.simulate_keystrokes("shift-enter");
        w.simulate_input("Ship");
        w.simulate_keystrokes("enter");
        assert_eq!(sent.borrow().len(), 1);
        let posted = &sent.borrow()[0];
        assert_eq!(posted.mentions.len(), 1);
        assert_eq!(posted.mentions[0].id.as_ref(), "launch");
        assert!(posted.text.ends_with("\nShip"), "{:?}", posted.text);
        assert_eq!(w.update(|_, cx| v.read(cx).value(cx)).text, "");
    }
    #[test]
    fn emoji_standard_search_handles_chinese_and_preserves_shift_selection() {
        use std::cell::RefCell;
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::init(cx);
            cx.set_global(UiLanguage::Chinese)
        });
        let (v, w) = app.add_window_view(SymbolPicker::standard);
        let picks = Rc::new(RefCell::new(Vec::new()));
        let got = picks.clone();
        let _sub = w.update(|_, cx| {
            cx.subscribe(&v, move |_, e: &SymbolPicked, _| {
                got.borrow_mut().push(e.0.to_string())
            })
        });
        click(w, "选择表情");
        settle(w);
        w.simulate_input("火箭");
        settle(w);
        w.simulate_keystrokes("shift-enter");
        assert!(w.update(|_, cx| v.read(cx).open));
        w.simulate_keystrokes("enter");
        assert_eq!(*picks.borrow(), ["🚀", "🚀"]);
        assert!(!w.update(|_, cx| v.read(cx).open));
    }
}
