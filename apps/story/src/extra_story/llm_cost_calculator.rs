//! Illustrative numbers only; no real provider prices are embedded.
use gpui::{prelude::*, *};
use kui::{
    components::llm_cost_calculator::{LlmCostCalculator, LlmModelPrice, LlmUsage},
    primitives::{control::UiLanguage, family::Paint},
};
pub const IDS: &[&str] = &["llm-cost-calculator"];
pub fn story(id: &str, _w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}
struct Demo {
    calculator: Entity<LlmCostCalculator>,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let models = vec![
            LlmModelPrice::new(
                "example-a",
                t("Example A", "样例 A"),
                t("Sample provider", "样例提供商"),
                1.2,
                4.8,
            )
            .with_cached_input(0.25),
            LlmModelPrice::new(
                "example-b",
                t("Example B", "样例 B"),
                t("Sample provider", "样例提供商"),
                0.5,
                3.,
            )
            .with_cached_input(0.15),
            LlmModelPrice::new(
                "example-c",
                t("Example C", "样例 C"),
                t("Sample provider", "样例提供商"),
                2.5,
                7.,
            ),
        ];
        let mut usage = LlmUsage::new(8_000., 500., 5_000.);
        usage.cache_enabled = true;
        usage.cache_hit_rate = 0.6;
        let calculator =
            cx.new(|cx| LlmCostCalculator::new(models, usage, cx).with_simulated(true));
        Self { calculator }
    }
}
impl Render for Demo {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .size_full()
            .min_h(px(620.))
            .flex()
            .items_center()
            .justify_center()
            .p(px(20.))
            .bg(p.stage)
            .text_color(p.ink)
            .child(self.calculator.clone())
    }
}
