//! Installable control examples shared by both preview hosts.
use gpui::{prelude::*, *};
use kui::{
    components::button::Button,
    primitives::{Size, clipboard, control::UiLanguage, family::Paint, glyph::Glyph},
};
use std::time::Duration;
#[derive(Default)]
pub struct BaseButtons {
    copied: [bool; 2],
    serial: [u64; 2],
    busy: bool,
    note: SharedString,
}
impl BaseButtons {
    fn copy(&mut self, i: usize, cx: &mut Context<Self>) {
        self.serial[i] += 1;
        let ticket = self.serial[i];
        let task = clipboard::write("kui add button", cx);
        cx.spawn(async move |v, cx| {
            let ok = task.await;
            let _ = v.update(cx, |v, cx| {
                if v.serial[i] == ticket {
                    v.copied[i] = ok;
                    let l = UiLanguage::current(cx);
                    v.note = if ok {
                        l.text("Link copied", "链接已复制")
                    } else {
                        l.text("Copy failed", "复制失败")
                    }
                    .into();
                    cx.notify()
                }
            });
            cx.background_executor()
                .timer(Duration::from_millis(1800))
                .await;
            let _ = v.update(cx, |v, cx| {
                if v.serial[i] == ticket {
                    v.copied[i] = false;
                    v.note = "".into();
                    cx.notify()
                }
            });
        })
        .detach()
    }
}
impl Render for BaseButtons {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let l = UiLanguage::current(cx);
        let t = |en, zh| l.text(en, zh);
        let p = Paint::get(cx);
        let row = || {
            div()
                .max_w_full()
                .flex()
                .items_center()
                .justify_center()
                .flex_wrap()
                .gap(px(8.))
        };
        let section = |title: &'static str, items: Div| {
            div()
                .max_w_full()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(10.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .child(title),
                )
                .child(items)
        };
        let button = |id, en, zh| Button::new(id).label(t(en, zh));
        let mut styles = row();
        for (i, (en, zh)) in [
            ("Save changes", "保存更改"),
            ("Secondary", "次要按钮"),
            ("Ghost", "幽灵按钮"),
            ("Delete", "删除"),
        ]
        .into_iter()
        .enumerate()
        {
            let mut b = button(("style", i), en, zh);
            b = match i {
                0 => b.primary(),
                1 => b.secondary(),
                2 => b.ghost(),
                _ => b.danger(),
            };
            styles = styles.child(
                b.on_click(cx.listener(move |v, _, _, cx| {
                    v.note = UiLanguage::current(cx).text(en, zh).into();
                    cx.notify()
                }))
                .into_themed(cx),
            );
        }
        let icons = row()
            .child(
                Button::new("add")
                    .secondary()
                    .label(t("Add item", "添加项目"))
                    .with_size(Size::Sm)
                    .with_icon(Glyph::Plus, false)
                    .into_themed(cx),
            )
            .child(
                Button::new("continue")
                    .label(t("Continue", "继续"))
                    .with_icon(Glyph::Arrow, true)
                    .into_themed(cx),
            )
            .child(
                Button::new("more")
                    .ghost()
                    .label(t("More options", "更多选项"))
                    .with_size(Size::Sm)
                    .icon_only(Glyph::More)
                    .into_themed(cx),
            )
            .child(
                Button::new("large")
                    .secondary()
                    .label(t("Large", "大按钮"))
                    .with_size(Size::Lg)
                    .into_themed(cx),
            );
        let mut feedback = row();
        for i in 0..2 {
            let g = if self.copied[i] {
                Glyph::Check
            } else {
                Glyph::Copy
            };
            let b = Button::new(("copy", i))
                .secondary()
                .with_size(Size::Sm)
                .label(if self.copied[i] {
                    t("Copied", "已复制")
                } else {
                    t("Copy link", "复制链接")
                });
            let b = if i == 0 {
                b.with_icon(g, false)
            } else {
                b.icon_only(g)
            };
            feedback = feedback.child(
                b.on_click(cx.listener(move |v, _, _, cx| v.copy(i, cx)))
                    .into_themed(cx),
            );
        }
        feedback = feedback
            .child(
                Button::new("upload")
                    .secondary()
                    .label(t("Upload file", "上传文件"))
                    .with_size(Size::Sm)
                    .loading(self.busy)
                    .on_click(cx.listener(|v, _, _, cx| {
                        if v.busy {
                            return;
                        }
                        v.busy = true;
                        cx.notify();
                        cx.spawn(async move |v, cx| {
                            cx.background_executor()
                                .timer(Duration::from_millis(1200))
                                .await;
                            let _ = v.update(cx, |v, cx| {
                                v.busy = false;
                                cx.notify()
                            });
                        })
                        .detach()
                    }))
                    .into_themed(cx),
            )
            .child(
                Button::new("disabled")
                    .secondary()
                    .label(t("Disabled", "已禁用"))
                    .disabled(true)
                    .with_size(Size::Sm)
                    .into_themed(cx),
            );
        div()
            .relative()
            .w_full()
            .max_w(px(580.))
            .flex()
            .flex_col()
            .items_center()
            .top(px(-17.))
            .gap(px(20.))
            .child(section(t("Styles", "样式"), styles))
            .child(section(t("Icons and sizes", "图标与尺寸"), icons))
            .child(section(t("Feedback and states", "反馈与状态"), feedback))
            .child(
                div()
                    .absolute()
                    .top_full()
                    .mt(px(8.))
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .debug_selector(|| "button feedback".into())
                    .child(self.note.clone()),
            )
    }
}
