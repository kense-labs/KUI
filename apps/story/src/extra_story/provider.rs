//! Simulated health data; no network requests are made.
use gpui::{prelude::*, *};
use kui::{
    components::provider_status::{
        ProviderDay, ProviderHealth, ProviderIncident, ProviderIncidentUpdate, ProviderStatus,
        ProviderStatusEvent, ProviderStatusItem,
    },
    primitives::{control::UiLanguage, family::Paint},
};

pub const IDS: &[&str] = &["provider-status"];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}
struct Example {
    view: Entity<ProviderStatus>,
    language: UiLanguage,
    failed: bool,
    _subscription: Subscription,
}
fn samples(language: UiLanguage, healthy: bool) -> Vec<ProviderStatusItem> {
    let names = [
        ("anthropic", "Anthropic", ProviderHealth::Operational),
        (
            "openai",
            "OpenAI",
            if healthy {
                ProviderHealth::Operational
            } else {
                ProviderHealth::Outage
            },
        ),
        ("gemini", "Google Gemini", ProviderHealth::Degraded),
        ("mistral", "Mistral AI", ProviderHealth::Operational),
        ("xai", "xAI", ProviderHealth::Operational),
        ("bedrock", "Amazon Bedrock", ProviderHealth::Operational),
    ];
    names
        .into_iter()
        .enumerate()
        .map(|(i, (id, name, health))| {
            let mut item = ProviderStatusItem::new(id, name, health);
            item.latency_ms = Some([412., 2140., 1380., 367., 448., 543.][i]);
            item.uptime = Some([99.96, 99.61, 99.93, 99.81, 99.98, 99.97][i]);
            item.days = (1..=30)
                .map(|day| {
                    let day_health = if day == 30 && i > 0 {
                        health
                    } else if day == 4 + i * 3 {
                        ProviderHealth::Degraded
                    } else {
                        ProviderHealth::Operational
                    };
                    ProviderDay::new(
                        format!("2026-10-{day:02}"),
                        day_health,
                        Some(if day_health == ProviderHealth::Operational {
                            100.
                        } else {
                            97.2
                        }),
                    )
                })
                .collect();
            if i == 1 && !healthy {
                item.incident = Some(ProviderIncident::new(
                    language.text("Elevated model error rates", "模型错误率上升"),
                    vec![ProviderIncidentUpdate::new(
                        language.text("Investigating", "调查中"),
                        "14:02 UTC",
                        language.text(
                            "The simulated Responses endpoint is returning more 5xx errors.",
                            "模拟 Responses 接口的 5xx 错误增多。",
                        ),
                    )],
                ));
            }
            item
        })
        .collect()
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view = cx.new(|_| {
            ProviderStatus::new(
                language.text("AI providers", "AI 服务状态"),
                samples(language, false),
            )
            .with_open_incident("openai")
            .footnote(language.text(
                "Sample data. Latency is the median over the last 5 minutes.",
                "仅为样例。延迟展示最近 5 分钟的中位数。",
            ))
        });
        let sub = cx.subscribe(
            &view,
            |s: &mut Self, view, event: &ProviderStatusEvent, cx| {
                if *event == ProviderStatusEvent::Refresh {
                    let Some(request) = view.read(cx).current_request() else {
                        return;
                    };
                    if !s.failed {
                        s.failed = true;
                        view.update(cx, |v, cx| {
                            v.set_error_for(
                                request,
                                s.language.text(
                                    "Simulated check failed. Previous results retained.",
                                    "模拟检查失败，已保留上次结果。",
                                ),
                                cx,
                            );
                        });
                    } else {
                        view.update(cx, |v, cx| {
                            v.set_providers_for(request, samples(s.language, true), cx);
                        });
                    }
                }
            },
        );
        Self {
            view,
            language,
            failed: false,
            _subscription: sub,
        }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .size_full()
            .min_h(px(840.))
            .p_4()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(div().w_full().max_w(px(522.)).child(self.view.clone()))
    }
}
