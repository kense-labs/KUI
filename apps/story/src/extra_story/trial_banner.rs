//! Bilingual trial banner states with local-only billing simulation.
use gpui::{prelude::*, *};
use kui::{
    components::{
        segmented_control::{FormOption, SegmentedControl, SelectionChanged},
        trial_banner::{CivilDate, TrialBanner, TrialEvent, TrialVariant},
    },
    primitives::{control::UiLanguage, family::Paint},
};

pub const IDS: &[&str] = &["trial-banner"];

pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Example::new(w, cx)).into())
}

struct Example {
    banner: Entity<TrialBanner>,
    variants: Entity<SegmentedControl>,
    trials: Entity<SegmentedControl>,
    note: Option<&'static str>,
    _subscriptions: [Subscription; 4],
}

impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = move |en, zh| language.text(en, zh);
        let banner = cx.new(|cx| TrialBanner::new("trial-example", cx));
        let variants = cx.new(|cx| {
            SegmentedControl::new(
                t("Variant", "外观"),
                vec![
                    FormOption::new("banner", t("Banner", "横幅")),
                    FormOption::new("card", t("Sidebar card", "侧栏卡片")),
                    FormOption::new("pill", t("Pill", "胶囊")),
                ],
                w,
                cx,
            )
            .with_selected(&["banner"])
        });
        let trials = cx.new(|cx| {
            SegmentedControl::new(
                t("Trial", "试用状态"),
                vec![
                    FormOption::new("9", t("9 days left", "剩 9 天")),
                    FormOption::new("2", t("2 days left", "剩 2 天")),
                    FormOption::new("0", t("Expired", "已结束")),
                ],
                w,
                cx,
            )
            .with_selected(&["9"])
        });
        let subscriptions = [
            cx.subscribe(&variants, |s, _, event: &SelectionChanged, cx| {
                let variant = match event.0.first().map(AsRef::as_ref) {
                    Some("card") => TrialVariant::Card,
                    Some("pill") => TrialVariant::Pill,
                    _ => TrialVariant::Banner,
                };
                s.note = None;
                s.banner.update(cx, |b, cx| b.set_variant(variant, cx));
                cx.notify();
            }),
            cx.subscribe(&trials, |s, _, event: &SelectionChanged, cx| {
                let Some(days) = event.0.first().and_then(|s| s.parse::<i32>().ok()) else {
                    return;
                };
                let today = CivilDate {
                    year: 2026,
                    month: 10,
                    day: 27,
                };
                s.note = None;
                s.banner.update(cx, |b, cx| {
                    b.set_schedule(today, today.add_days(days), days, 14, cx)
                });
                cx.notify();
            }),
            cx.subscribe(&banner, move |s, _, event: &TrialEvent, cx| {
                s.note = match event {
                    TrialEvent::ComparePlans => {
                        Some(t("Simulated: open the plans page.", "模拟打开方案页。"))
                    }
                    TrialEvent::ManageBilling => {
                        Some(t("Simulated: open billing settings.", "模拟打开账单设置。"))
                    }
                    _ => None,
                };
                cx.notify();
            }),
            cx.observe(&banner, |_, _, cx| cx.notify()),
        ];
        Self {
            banner,
            variants,
            trials,
            note: None,
            _subscriptions: subscriptions,
        }
    }
}

impl Render for Example {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let picker = |label, control| {
            div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .child(label),
                )
                .child(control)
        };
        let height = if self.banner.read(cx).is_expanded() {
            if w.viewport_size().width < px(510.) {
                628.
            } else {
                574.
            }
        } else {
            510.
        };
        div()
            .flex()
            .flex_col()
            .w_full()
            .max_w(px(780.))
            .min_h(px(height))
            .p(px(12.))
            .gap(px(12.))
            .items_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .w_full()
                    .justify_center()
                    .gap(px(16.))
                    .flex_wrap()
                    .child(picker(lang.text("Variant", "外观"), self.variants.clone()))
                    .child(picker(lang.text("Trial", "试用状态"), self.trials.clone())),
            )
            .child(self.banner.clone())
            .children(self.note.map(|note| {
                div()
                    .id("trial-story-note")
                    .role(Role::Status)
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(note)
            }))
    }
}
