//! A four-card wallet with sample balances and activity.
use gpui::{prelude::*, *};
use kui::{
    components::wallet_stack::{WalletStack, WalletStackCard, WalletTransaction, WalletVariant},
    primitives::{control::UiLanguage, glyph::Glyph},
};
use std::sync::Arc;

pub const IDS: &[&str] = &["wallet-stack"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

struct Example(Entity<WalletStack>);

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let person = "Jordan Hayes";
        let available = t("Available", "可用余额");
        let entry = |id, merchant, detail, amount, icon| {
            WalletTransaction::new(id, merchant, detail, amount).with_icon(icon)
        };
        let cards = vec![
            WalletStackCard::new(
                "debit",
                "Fieldnote",
                t("Everyday debit", "日常借记卡"),
                "4821",
                person,
                3248.60,
                available,
            )
            .with_expiration("09/29")
            .with_transactions(vec![
                WalletTransaction::new(
                    "split",
                    "Ryan Mitchell",
                    t("Dinner split · Today", "餐费分摊 · 今天"),
                    36.50,
                )
                .with_avatar(avatar("#ecee9a", "#309ba5", "#d0f0a4")),
                entry(
                    "groceries",
                    "Greenleaf Market",
                    t("Groceries · Yesterday", "杂货 · 昨天"),
                    -84.12,
                    Glyph::Bag,
                ),
                entry(
                    "payroll",
                    "Brightline Studio",
                    t("Payroll · Sep 18", "工资 · 9月18日"),
                    2860.,
                    Glyph::Download,
                ),
            ]),
            WalletStackCard::new(
                "credit",
                "Meridian",
                t("Reserve credit", "储备信用卡"),
                "0917",
                person,
                1286.44,
                t("Spent in September", "9月消费"),
            )
            .with_variant(WalletVariant::Black)
            .with_expiration("03/28")
            .with_transactions(vec![
                entry(
                    "dinner",
                    "Alder & Pine",
                    t("Dining · Sep 20", "餐饮 · 9月20日"),
                    -142.80,
                    Glyph::Bag,
                ),
                entry(
                    "flight",
                    "Cascade Air",
                    t("Travel · Sep 17", "旅行 · 9月17日"),
                    -389.,
                    Glyph::Plane,
                ),
                WalletTransaction::new(
                    "concert",
                    "Emily Parker",
                    t("Concert tickets · Sep 16", "演出门票 · 9月16日"),
                    64.,
                )
                .with_avatar(avatar("#b4e672", "#d13091", "#eba06c")),
            ]),
            WalletStackCard::new(
                "travel",
                "Tidewater",
                t("Transit pass", "交通卡"),
                "3306",
                person,
                42.25,
                t("Pass balance", "交通卡余额"),
            )
            .with_variant(WalletVariant::Glass)
            .with_expiration("11/27")
            .with_transactions(vec![
                entry(
                    "ferry",
                    "Harbor line",
                    t("Ferry · Today, 7:52 AM", "渡轮 · 今天 07:52"),
                    -3.25,
                    Glyph::Globe,
                ),
                entry(
                    "bicycle",
                    "Bike share",
                    t("22 min ride · Yesterday", "骑行 22 分钟 · 昨天"),
                    -2.50,
                    Glyph::Clock,
                ),
                entry(
                    "reload",
                    "Auto top up",
                    t("From Fieldnote · Sep 15", "来自 Fieldnote · 9月15日"),
                    40.,
                    Glyph::Plus,
                ),
            ]),
            WalletStackCard::new(
                "coffee",
                "Common Ground",
                t("Coffee club", "咖啡会员卡"),
                "7730",
                person,
                18.40,
                t("Prepaid", "预付余额"),
            )
            .with_variant(WalletVariant::Color)
            .with_tint(rgb(0x00503b))
            .with_expiration("06/28")
            .with_transactions(vec![
                entry(
                    "coffee",
                    "Oat cortado",
                    t("Pearl Street · Today, 8:10 AM", "珍珠街 · 今天 08:10"),
                    -4.60,
                    Glyph::Bag,
                ),
                entry(
                    "pastry",
                    "Morning bun",
                    t("Pearl Street · Yesterday", "珍珠街 · 昨天"),
                    -3.90,
                    Glyph::Bag,
                ),
                entry(
                    "refill",
                    "Reload",
                    t("From Fieldnote · Sep 14", "来自 Fieldnote · 9月14日"),
                    25.,
                    Glyph::Plus,
                ),
            ]),
        ];
        Self(cx.new(|cx| {
            WalletStack::new(cards, cx)
                .with_id("wallet-demo")
                .with_label(t("Jordan’s cards", "乔丹的钱包"))
        }))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .min_h(px(542.))
            .px(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .child(self.0.clone())
    }
}

fn avatar(start: &str, end: &str, glow: &str) -> Arc<Image> {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><defs><linearGradient id="a" x2="1" y2="1"><stop stop-color="{start}"/><stop offset="1" stop-color="{end}"/></linearGradient><radialGradient id="b"><stop stop-color="{glow}"/><stop offset="1" stop-color="{glow}" stop-opacity="0"/></radialGradient></defs><circle cx="16" cy="16" r="16" fill="url(#a)"/><circle cx="21" cy="12" r="16" fill="url(#b)"/></svg>"#
    );
    Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()))
}
