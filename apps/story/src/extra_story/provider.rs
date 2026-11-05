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
        ("north", "North AI", ProviderHealth::Operational),
        (
            "river",
            "River AI",
            if healthy {
                ProviderHealth::Operational
            } else {
                ProviderHealth::Outage
            },
        ),
        ("summit", "Summit", ProviderHealth::Degraded),
    ];
    names
        .into_iter()
        .enumerate()
        .map(|(i, (id, name, health))| {
            let mut item = ProviderStatusItem::new(id, name, health);
            item.detail = language
                .text("Sample model endpoint", "样例模型接口")
                .into();
            item.latency_ms = Some([412., 2140., 1380.][i]);
            item.uptime = Some([99.96, 99.61, 99.93][i]);
            item.days = (1..=30)
                .map(|day| {
                    ProviderDay::new(
                        format!("2026-10-{day:02}"),
                        if day == 29 && i > 0 {
                            health
                        } else {
                            ProviderHealth::Operational
                        },
                        Some(if day == 29 && i > 0 { 97.2 } else { 100. }),
                    )
                })
                .collect();
            if i == 1 && !healthy {
                item.incident = Some(ProviderIncident::new(
                    language.text("Increased error rate", "错误率上升"),
                    vec![ProviderIncidentUpdate::new(
                        language.text("Investigating", "调查中"),
                        "14:02 UTC",
                        language.text(
                            "Requests may take longer than usual.",
                            "请求可能比平时更慢。",
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
            .footnote(language.text(
                "Sample data; no network requests are made.",
                "样例数据，不会发出网络请求。",
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
            .min_h(px(540.))
            .p_4()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(
                div().text_size(px(12.)).text_color(p.secondary).child(
                    self.language
                        .text("Simulated provider health", "模拟服务状态"),
                ),
            )
            .child(div().w_full().max_w(px(760.)).child(self.view.clone()))
    }
}
