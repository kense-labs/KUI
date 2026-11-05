//! Sample prompt-cache results; numbers and prices are illustrative.
use gpui::{prelude::*, *};
use kui::{
    components::prompt_cache::{
        CacheOutcome, CacheRequest, CacheResult, CacheSegment, CacheSnapshot, CacheState,
        PromptCache, PromptCacheAction,
    },
    primitives::{control::UiLanguage, family::Paint},
};
use std::{cell::Cell, rc::Rc};
pub const IDS: &[&str] = &["prompt-cache"];
pub fn story(id: &str, _w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}
struct Demo {
    chart: Entity<PromptCache>,
}
impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let segments = vec![
            CacheSegment::new("tools", t("Tools", "工具"), 3800, rgb(0x7795de).into()),
            CacheSegment::new(
                "system",
                t("System prompt", "系统提示"),
                2400,
                rgb(0xa08ad2).into(),
            ),
            CacheSegment::new("docs", t("Documents", "文档"), 36200, rgb(0x65b69a).into()),
            CacheSegment::new(
                "conversation",
                t("Conversation", "对话"),
                6000,
                rgb(0xd4a875).into(),
            ),
        ];
        let requests = (1..=4)
            .map(|i| {
                CacheRequest::new(
                    format!("request-{i}"),
                    format!("{} {i}", t("Request", "请求")),
                    180 + i * 30,
                )
            })
            .collect();
        let chart = cx.new(|_| {
            PromptCache::new(segments, CacheSnapshot::new(requests)).with_change_segment("system")
        });
        let failed = Rc::new(Cell::new(false));
        cx.subscribe(&chart, move |_, chart, action: &PromptCacheAction, cx| {
            let fail = !failed.replace(true);
            let id = action.id;
            let request = action.request.clone();
            let changed = action.change_segment.is_some();
            let weak = chart.downgrade();
            cx.spawn(async move |_, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(420))
                    .await;
                let _ = weak.update(cx, |chart, cx| {
                    if fail {
                        chart.resolve(
                            id,
                            Err(lang
                                .text(
                                    "Simulated request failed; retry to continue",
                                    "模拟请求失败，请重试",
                                )
                                .into()),
                            cx,
                        );
                        return;
                    }
                    let index = request
                        .as_ref()
                        .strip_prefix("request-")
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(1);
                    let prefix = 48_400u64;
                    let message = 180 + index * 30;
                    let (kind, cached) = if index == 1 {
                        (CacheOutcome::Write, 0)
                    } else if changed {
                        (CacheOutcome::Partial, 3_800)
                    } else {
                        (CacheOutcome::Hit, prefix)
                    };
                    let written = prefix - cached;
                    let mut result = CacheResult::new(kind, cached, written + message, written);
                    result.cost = Some(
                        (cached as f64 * 0.20 + written as f64 * 2.50 + message as f64 * 2.00)
                            / 1_000_000.,
                    );
                    result.first_token_ms = Some(420 + (written + message) / 40 + cached / 500);
                    chart.resolve(
                        id,
                        Ok((result, CacheState::Available, prefix, Some(300))),
                        cx,
                    );
                });
            })
            .detach();
        })
        .detach();
        Self { chart }
    }
}
impl Render for Demo {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .size_full()
            .min_h(px(520.))
            .flex()
            .flex_col()
            .gap_3()
            .items_center()
            .justify_center()
            .p(px(20.))
            .bg(p.stage)
            .text_color(p.ink)
            .child(lang_sample(UiLanguage::current(cx)))
            .child(self.chart.clone())
    }
}
fn lang_sample(lang: UiLanguage) -> &'static str {
    lang.text(
        "Simulated cache outcomes and sample prices",
        "模拟缓存结果与示例价格",
    )
}
