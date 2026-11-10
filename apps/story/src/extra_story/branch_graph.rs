use gpui::{prelude::*, *};
use kui::{
    components::branch_graph::{BranchGraph, BranchGraphOptions},
    primitives::control::UiLanguage,
};

pub const IDS: &[&str] = &["branch-graph"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

struct Demo(Entity<BranchGraph>);

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        crate::ensure_story_fonts(cx);
        let mut options = BranchGraphOptions::default();
        if UiLanguage::current(cx) == UiLanguage::Chinese {
            options.pull_request.title = "结账流程改版".into();
            let messages = [
                "为回访顾客保留付款卡",
                "升级结账 SDK 至 4.2",
                "添加 Apple Pay 按钮",
                "合并金额取整修复",
                "逐条计算税费后取整",
                "将支付移到独立步骤",
                "新增订单摘要",
                "发布结账流程初版",
            ];
            let dates = [
                "2 小时前",
                "5 小时前",
                "昨天",
                "昨天",
                "2 天前",
                "2 天前",
                "3 天前",
                "4 天前",
            ];
            let notes = [
                "在首次下单时保存顾客编号，以便下次结账显示已存的付款卡。",
                "更新结账 SDK 和依赖锁定文件。",
                "浏览器支持时显示 Apple Pay，否则回到银行卡表单。",
                "把税费取整修复纳入结账流程。",
                "先对每个商品行的税费取整，再汇总订单金额。",
                "将支付方式选择与订单确认分开。",
                "付款前显示小计、配送费和税费。",
                "接通第一版完整结账流程。",
            ];
            for (((row, message), date), note) in options
                .commits
                .iter_mut()
                .zip(messages)
                .zip(dates)
                .zip(notes)
            {
                row.message = message.into();
                row.date = date.into();
                row.body = Some(note.into());
            }
        }
        Self(cx.new(|cx| {
            BranchGraph::new(options, cx).with_visibility(|cx| {
                cx.try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)())
            })
        }))
    }
}

impl Render for Demo {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let stage_height = if window.viewport_size().width < px(400.) {
            648.
        } else {
            612.
        };
        div()
            .w_full()
            .min_h(px(stage_height))
            .flex()
            .justify_center()
            .items_start()
            .px(px(8.))
            .py(px(22.))
            .child(div().w_full().max_w(px(552.)).child(self.0.clone()))
    }
}
