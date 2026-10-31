//! The application owns the project data, validation policy and simulated save results.
use gpui::{prelude::*, *};
use kui::{
    components::{
        expanding_search::{ExpandingSearch, SearchAnchor, SearchEvent, SearchItem},
        inline_edit::{InlineEdit, SaveRequest},
        input::Input,
        password_field::PasswordField,
        password_strength::{PasswordStrength, StrengthEvent},
        search_field::SearchField,
        textarea::Textarea,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Paint, Slide, Tone, face},
        field::FieldEvent,
        glyph::{Glyph, icon},
    },
};
use std::time::Duration;
pub const IDS: [&str; 7] = [
    "input",
    "textarea",
    "password-field",
    "password-strength",
    "search-field",
    "expanding-search",
    "inline-edit",
];
enum Demo {
    Basic(AnyView, f32),
    Password(Entity<PasswordStrength>),
    Search(Entity<ExpandingSearch>),
    Project(Entity<InlineEdit>, Entity<InlineEdit>),
}
pub struct FieldStory {
    demo: Demo,
    _subscriptions: Vec<Subscription>,
    notice: SharedString,
    fail_next: bool,
    switch: Slide,
    suggestion: usize,
}
impl FieldStory {
    pub fn new(id: &str, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = UiLanguage::current(cx);
        let t = move |en, zh| l.text(en, zh);
        let mut subs = Vec::new();
        let demo = match id {
            "input" => {
                let v = cx.new(|cx| {
                    Input::new("project", t("Project name", "项目名称"), w, cx)
                        .with_value(t("KUI launch", "KUI 发布"), w, cx)
                        .with_description(t(
                            "Use a short, descriptive name.",
                            "使用简短、清晰的名称。",
                        ))
                });
                subs.push(cx.subscribe(&v, move |_, v, e: &FieldEvent, cx| {
                    if let FieldEvent::Changed(raw) = e {
                        v.update(cx, |field, cx| {
                            field.set_error(
                                if raw.trim().is_empty() {
                                    t("Enter a project name.", "请输入项目名称。")
                                } else {
                                    ""
                                },
                                cx,
                            )
                        })
                    }
                }));
                Demo::Basic(v.into(), 538.)
            }
            "textarea" => {
                let v = cx.new(|cx| {
                    Textarea::new("note", t("Project note", "项目备注"), w, cx)
                        .with_value(
                            t(
                                "Keep the final pass calm and focused.",
                                "最后一轮检查请专注于关键细节。",
                            ),
                            w,
                            cx,
                        )
                        .with_description(t(
                            "Visible to everyone on the project.",
                            "项目中的每个人都能看到。",
                        ))
                });
                Demo::Basic(v.into(), 538.)
            }
            "password-field" => {
                let v = cx.new(|cx| {
                    PasswordField::new("access", t("Access key", "访问密钥"), w, cx)
                        .with_value("kui-library-secret", w, cx)
                        .with_description(t("Use at least 8 characters.", "至少输入 8 个字符。"))
                });
                subs.push(cx.subscribe(&v, move |_, v, e: &FieldEvent, cx| {
                    if let FieldEvent::Changed(raw) = e {
                        v.update(cx, |s, cx| {
                            s.set_error(
                                if !raw.is_empty() && raw.chars().count() < 8 {
                                    t("Access key is too short.", "访问密钥太短。")
                                } else {
                                    ""
                                },
                                cx,
                            )
                        })
                    }
                }));
                Demo::Basic(v.into(), 216.)
            }
            "search-field" => {
                let v = cx.new(|cx| {
                    SearchField::new("query", t("Search projects", "搜索项目"), w, cx)
                        .with_placeholder(t("Search by name", "按名称搜索"), cx)
                });
                Demo::Basic(v.into(), 250.)
            }
            "password-strength" => {
                let v = cx.new(|cx| {
                    PasswordStrength::new("password", t("New password", "新密码"), w, cx)
                });
                subs.push(
                    cx.subscribe(&v, |s: &mut Self, _, e: &StrengthEvent, cx| match e {
                        StrengthEvent::Submitted => s.check_password(cx),
                        StrengthEvent::Changed(..) => {
                            s.notice = "".into();
                            cx.notify()
                        }
                        _ => {}
                    }),
                );
                Demo::Password(v)
            }
            "expanding-search" => {
                let projects = t("Projects", "项目");
                let docs = t("Docs", "文档");
                let data = [
                    (
                        "billing",
                        t("Billing migration", "计费迁移"),
                        projects,
                        t("Payments · Due Oct 14", "支付 · 10 月 14 日到期"),
                    ),
                    (
                        "mobile",
                        t("Mobile onboarding", "移动端入门"),
                        projects,
                        t("Growth · Due Nov 3", "增长 · 11 月 3 日到期"),
                    ),
                    (
                        "planning",
                        t("Q4 planning", "第四季度计划"),
                        projects,
                        t("Leadership · 8 docs", "管理 · 8 份文档"),
                    ),
                    (
                        "portal",
                        t("Customer portal", "客户门户"),
                        projects,
                        t("Support · 5 open tasks", "支持 · 5 个待办任务"),
                    ),
                    (
                        "limits",
                        t("API rate limits", "API 速率限制"),
                        docs,
                        t("Platform · Edited Sep 3", "平台 · 9 月 3 日更新"),
                    ),
                    (
                        "incident",
                        t("Incident response runbook", "事件响应手册"),
                        docs,
                        t("Platform · Edited Sep 12", "平台 · 9 月 12 日更新"),
                    ),
                    (
                        "release",
                        t("Release checklist", "发布清单"),
                        docs,
                        t("Engineering · Edited Sep 18", "工程 · 9 月 18 日更新"),
                    ),
                    (
                        "pricing",
                        t("Pricing page copy", "定价页文案"),
                        docs,
                        t("Marketing · Edited Sep 20", "市场 · 9 月 20 日更新"),
                    ),
                ];
                let items = data
                    .iter()
                    .map(|&(id, title, group, detail)| {
                        SearchItem::new(id, title)
                            .description(
                                group,
                                detail,
                                if group == projects {
                                    Glyph::Folder
                                } else {
                                    Glyph::Book
                                },
                            )
                            .keywords(format!("{id} {detail}"))
                    })
                    .collect();
                let v = cx.new(|cx| {
                    ExpandingSearch::new(
                        "find",
                        t("Search projects and docs", "搜索项目和文档"),
                        items,
                        w,
                        cx,
                    )
                    .with_suggestions(["release", "billing", "pricing"].map(Into::into).into())
                    .with_layout(SearchAnchor::Center, 360., 5)
                });
                subs.push(
                    cx.subscribe(&v, move |s: &mut Self, _, ev: &SearchEvent, cx| {
                        match ev {
                            SearchEvent::Selected(id) => {
                                if let Some(row) = data.iter().find(|i| i.0 == id.as_ref()) {
                                    s.notice =
                                        format!("{}{}", t("Selected: ", "已选择："), row.1).into()
                                }
                            }
                            SearchEvent::Expanded(true) => s.notice = "".into(),
                            _ => {}
                        }
                        cx.notify()
                    }),
                );
                Demo::Search(v)
            }
            "inline-edit" => {
                let a = cx.new(|cx| {
                    InlineEdit::new(
                        "name",
                        t("Project name", "项目名称"),
                        t("Atlas web app", "Atlas 网页应用"),
                        w,
                        cx,
                    )
                    .with_validator(move |s| {
                        s.is_empty()
                            .then(|| t("Give the project a name.", "请填写项目名称。").into())
                    })
                });
                let b=cx.new(|cx|InlineEdit::new("description",t("Description","描述"),
                    t("Offline-first field notes for the Atlas survey team. Drafts sync the moment a connection returns.","为 Atlas 调研团队提供离线优先的现场笔记。网络恢复后，草稿会立即同步。"),w,cx).multiline());
                for control in [&a, &b] {
                    subs.push(cx.subscribe(
                        control,
                        move |s: &mut Self, ed, ev: &SaveRequest, cx| {
                            let fail = std::mem::take(&mut s.fail_next);
                            s.switch.set(0., cx);
                            let rev = ev.revision;
                            cx.spawn(async move |weak, cx| {
                                cx.background_executor()
                                    .timer(Duration::from_millis(450))
                                    .await;
                                ed.update(cx, |v, cx| {
                                    v.finish(
                                        rev,
                                        if fail {
                                            Err(t("Couldn’t save changes.", "未能保存更改。")
                                                .into())
                                        } else {
                                            Ok(())
                                        },
                                        cx,
                                    )
                                });
                                if !fail {
                                    let _ = weak.update(cx, |s, cx| {
                                        s.notice =
                                            t("Last edited by you just now", "你刚刚编辑过").into();
                                        cx.notify()
                                    });
                                }
                            })
                            .detach();
                            cx.notify()
                        },
                    ));
                }
                Demo::Project(a, b)
            }
            _ => unreachable!("unknown text field"),
        };
        Self {
            demo,
            _subscriptions: subs,
            notice: "".into(),
            fail_next: false,
            switch: Slide::snappy(0.),
            suggestion: 0,
        }
    }
    fn check_password(&mut self, cx: &mut Context<Self>) {
        if let Demo::Password(v) = &self.demo {
            let l = UiLanguage::current(cx);
            let message = if v.read(cx).value(cx).is_empty() {
                l.text("Enter a password first.", "请先输入密码。")
            } else if !v.read(cx).strength().met.iter().all(|v| *v) {
                l.text(
                    "Meet all four requirements first.",
                    "请先满足全部四条要求。",
                )
            } else {
                ""
            };
            v.update(cx, |s, cx| s.set_error(message, cx));
            self.notice = if message.is_empty() {
                l.text("Saved", "已保存")
            } else {
                ""
            }
            .into();
            cx.notify()
        }
    }
}
impl Render for FieldStory {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let t = move |en, zh| l.text(en, zh);
        let r = Theme::global(cx).tokens.radius.md;
        let vw = f32::from(w.viewport_size().width);
        let container = |width: f32| {
            div()
                .w(px(width.min((vw - 32.).max(1.))))
                .max_w_full()
                .flex()
                .flex_col()
        };
        match &self.demo {
            Demo::Basic(v, width) => container(if *width == 538. {
                width.min(vw * 0.86)
            } else {
                *width
            })
            .pb(px(32.))
            .child(v.clone()),
            Demo::Search(v) => {
                container(360.)
                    .h(px(268.))
                    .child(v.clone())
                    .when(!self.notice.is_empty(), |d| {
                        d.child(
                            div()
                                .mt(px(14.))
                                .text_center()
                                .text_size(px(13.))
                                .text_color(p.secondary)
                                .child(self.notice.clone()),
                        )
                    })
            }
            Demo::Password(v) => {
                let suggest = t("Suggest one", "生成示例");
                let save = t("Save", "保存");
                container(360.).gap(px(18.)).child(v.clone()).child(
                    div()
                        .flex()
                        .justify_between()
                        .child(
                            face("suggest", suggest, 44., r.into(), Tone::Surface, cx)
                                .px(px(16.))
                                .border_1()
                                .border_color(p.line)
                                .child(suggest)
                                .on_click(cx.listener(|s, _, w, cx| {
                                    if let Demo::Password(v) = &s.demo {
                                        let words =
                                            ["Juniper!Lake24", "Orbit-Forest73", "Walnut!Cloud96"];
                                        v.update(cx, |v, cx| {
                                            v.set_value(words[s.suggestion % words.len()], w, cx);
                                            v.set_error("", cx)
                                        });
                                        s.suggestion = s.suggestion.wrapping_add(1)
                                    }
                                })),
                        )
                        .child(
                            face("save", save, 44., r.into(), Tone::Primary, cx)
                                .min_w(px(99.))
                                .child(if self.notice.is_empty() {
                                    save.into()
                                } else {
                                    self.notice.clone()
                                })
                                .child(icon(
                                    if self.notice.is_empty() {
                                        Glyph::Arrow
                                    } else {
                                        Glyph::Check
                                    },
                                    14.,
                                    p.inverse,
                                ))
                                .on_click(cx.listener(|s, _, _, cx| s.check_password(cx))),
                        ),
                )
            }
            Demo::Project(name, body) => {
                let fail = t("Fail the next save", "下次保存失败");
                let progress = self.switch.get(w, cx).clamp(0., 1.);
                let label = |txt| div().text_size(px(13.)).text_color(p.secondary).child(txt);
                container(440.)
                    .bg(p.panel)
                    .rounded(Theme::global(cx).tokens.radius.lg)
                    .border_1()
                    .border_color(p.line)
                    .p(px(24.))
                    .pb(px(32.))
                    .gap(px(18.))
                    .child(
                        div()
                            .child(label(t("Project name", "项目名称")))
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .child(label(t("Description", "描述")))
                            .child(body.clone()),
                    )
                    .child(
                        div().border_t_1().border_color(p.line).pt(px(12.)).child(
                            face("fail", fail, 44., r.into(), Tone::Clear, cx)
                                .mt(px(8.))
                                .justify_start()
                                .role(Role::Switch)
                                .aria_toggled(self.fail_next.into())
                                .child(
                                    div()
                                        .relative()
                                        .w(px(42.))
                                        .h(px(24.))
                                        .rounded(r)
                                        .bg(p.hover.blend(p.bright.opacity(progress)))
                                        .child(
                                            div()
                                                .absolute()
                                                .left(px(3. + 18. * progress))
                                                .top(px(3.))
                                                .size(px(18.))
                                                .rounded_full()
                                                .shadow_sm()
                                                .bg(gpui::white()
                                                    .blend(p.inverse.opacity(progress))),
                                        ),
                                )
                                .child(fail)
                                .on_click(cx.listener(|s, _, _, cx| {
                                    s.fail_next = !s.fail_next;
                                    s.switch.set(f32::from(s.fail_next), cx);
                                    cx.notify()
                                })),
                        ),
                    )
                    .child(
                        div()
                            .mt(px(-10.))
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .child(if self.notice.is_empty() {
                                t(
                                    "Last edited by Emma Collins on Mon 15 Sep",
                                    "上次编辑：Emma Collins，9 月 15 日星期一",
                                )
                                .into()
                            } else {
                                self.notice.clone()
                            }),
                    )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Demo, Duration, FieldStory};
    use gpui::{EntityInputHandler, Modifiers, TestAppContext, VisualTestContext};
    use kui::{
        components::inline_edit::EditPhase,
        primitives::editor::{Editor, InputMode},
    };
    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor().advance_clock(Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh()
        });
        w.run_until_parked()
    }
    fn click(w: &mut VisualTestContext, label: &'static str) {
        let pt = w
            .debug_bounds(label)
            .unwrap_or_else(|| panic!("missing {label}"))
            .center();
        w.simulate_click(pt, Modifiers::default());
        tick(w, 40)
    }
    #[test]
    fn composing_replaces_a_utf16_selection_and_undo_restores_graphemes() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (e, w) = app.add_window_view(|w, cx| Editor::new("name", InputMode::Single, w, cx));
        w.update(|w, cx| {
            e.update(cx, |e, cx| {
                e.set_value("a🦀b", w, cx);
                e.focus(w, cx);
                e.set_selected_text_range(1..3, w, cx);
                e.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
                e.replace_and_mark_text_in_range(None, "你好", Some(2..2), w, cx);
                assert_eq!(e.marked_text_range(w, cx), Some(1..3));
                e.replace_text_in_range(None, "你好", w, cx);
                assert_eq!(e.value(), "a你好b");
            })
        });
        w.simulate_keystrokes("cmd-z");
        assert_eq!(w.update(|_, cx| e.read(cx).value().to_string()), "a🦀b");
        w.simulate_keystrokes("backspace");
        assert_eq!(w.update(|_, cx| e.read(cx).value().to_string()), "ab");
    }
    #[test]
    fn grouped_search_selects_a_recent_item_by_keyboard() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (s, w) = app.add_window_view(|w, cx| FieldStory::new("expanding-search", w, cx));
        click(w, "Search projects and docs");
        tick(w, 330);
        assert!(w.debug_bounds("release").is_some());
        w.simulate_keystrokes("down down enter");
        tick(w, 680);
        assert_eq!(
            w.update(|_, cx| s.read(cx).notice.clone()),
            "Selected: Billing migration"
        );
        assert!(w.debug_bounds("release").is_none());
    }
    #[test]
    fn failed_inline_save_retries_original_draft_and_ignores_stale_ack() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (s, w) = app.add_window_view(|w, cx| FieldStory::new("inline-edit", w, cx));
        let field = w.update(|_, cx| match &s.read(cx).demo {
            Demo::Project(a, _) => a.clone(),
            _ => unreachable!(),
        });
        click(w, "Fail the next save");
        click(w, "Project name");
        w.simulate_input("Polaris");
        w.simulate_keystrokes("enter");
        tick(w, 550);
        assert_eq!(w.update(|_, cx| field.read(cx).phase()), EditPhase::Failed);
        assert_eq!(
            w.update(|_, cx| field.read(cx).saved().to_string()),
            "Atlas web app"
        );
        click(w, "Retry");
        w.update(|_, cx| field.update(cx, |v, cx| v.finish(1, Err("stale".into()), cx)));
        assert_eq!(w.update(|_, cx| field.read(cx).phase()), EditPhase::Saving);
        tick(w, 520);
        assert_eq!(
            w.update(|_, cx| field.read(cx).saved().to_string()),
            "Polaris"
        );
        click(w, "Project name");
        w.simulate_input("scratch");
        w.simulate_keystrokes("escape");
        tick(w, 500);
        assert_eq!(
            w.update(|_, cx| field.read(cx).saved().to_string()),
            "Polaris"
        );
    }
    #[test]
    fn password_save_requires_every_rule() {
        let mut app = TestAppContext::single();
        app.update(kui::init);
        let (s, w) = app.add_window_view(|w, cx| FieldStory::new("password-strength", w, cx));
        click(w, "Save");
        assert!(w.update(|_, cx| s.read(cx).notice.is_empty()));
        click(w, "Suggest one");
        click(w, "Save");
        assert_eq!(w.update(|_, cx| s.read(cx).notice.clone()), "Saved");
    }
}
