//! Simulate an upgrade locally with the shared plan prompt.
use gpui::{prelude::*, *};
use kui::{
    components::upgrade_prompt::{
        UpgradeFeature, UpgradeLimit, UpgradePrompt, UpgradePromptOptions,
    },
    primitives::control::UiLanguage,
};

pub const IDS: &[&str] = &["upgrade-prompt"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

struct Example(Entity<UpgradePrompt>);

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let mut options = UpgradePromptOptions::default();
        if let Some(models) = options
            .plan
            .features
            .iter_mut()
            .find(|feature| feature.id == "models")
        {
            models.label = "Claude Opus 5.5 and GPT-6.1 Sol".into();
        }
        if language == UiLanguage::Chinese {
            options.unit = "AI 点数".into();
            options.resets_on = "11 月 1 日".into();
            options.plan.features = vec![
                UpgradeFeature::new("credits", "每月 2,000 AI 点数"),
                UpgradeFeature::new("models", "Claude Opus 5.5 and GPT-6.1 Sol"),
                UpgradeFeature::new("queue", "优先队列，无每日上限"),
            ];
            options.plan.limits = vec![
                UpgradeLimit::new("AI 点数", "每月 2,000"),
                UpgradeLimit::new("模型", "高级模型"),
                UpgradeLimit::new("队列", "优先"),
            ];
        }
        Self(cx.new(|cx| UpgradePrompt::new(options, cx)))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .px(px(8.))
            .py(px(20.))
            .flex()
            .justify_center()
            .child(self.0.clone())
    }
}
