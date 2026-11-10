//! A local, bilingual integration catalog with repeatable connection outcomes.
use gpui::{prelude::*, *};
use kui::{
    components::orbit_logos::{OrbitConnectRequested, OrbitLogo, OrbitLogos},
    foundation::{Theme, ThemeMode},
    primitives::control::UiLanguage,
};
use std::{sync::Arc, time::Duration};

pub const IDS: &[&str] = &["orbit-logos"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

struct Example {
    orbit: Entity<OrbitLogos>,
    _connections: Subscription,
}

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let t = |en, zh| UiLanguage::current(cx).text(en, zh);
        let payments = t("Payments", "支付");
        let dev = t("Dev", "开发");
        let comms = t("Comms", "沟通");
        let design = t("Design", "设计");
        let storage = t("Storage", "存储");
        let data = [
            (
                "stripe",
                "Stripe",
                payments,
                t(
                    "Starts trials and bills usage from your plans",
                    "启动试用并按套餐用量计费",
                ),
                false,
                "https://docs.stripe.com",
                include_str!("../../assets/orbit-logos/stripe.svg"),
            ),
            (
                "github",
                "GitHub",
                dev,
                t(
                    "Syncs your issues and links pull requests",
                    "同步议题与拉取请求",
                ),
                true,
                "https://docs.github.com",
                include_str!("../../assets/orbit-logos/github.svg"),
            ),
            (
                "slack",
                "Slack",
                comms,
                t(
                    "Posts deploy alerts to your channels",
                    "将部署提醒发送到你的频道",
                ),
                true,
                "https://api.slack.com",
                include_str!("../../assets/orbit-logos/slack.svg"),
            ),
            (
                "figma",
                "Figma",
                design,
                t(
                    "Brings design files into your workspace",
                    "将设计文件带入工作区",
                ),
                true,
                "https://developers.figma.com",
                include_str!("../../assets/orbit-logos/figma.svg"),
            ),
            (
                "google-drive",
                "Google Drive",
                storage,
                t("Keeps shared documents in sync", "保持共享文档同步"),
                false,
                "https://developers.google.com/drive",
                include_str!("../../assets/orbit-logos/googledrive.svg"),
            ),
            (
                "linear",
                "Linear",
                dev,
                t(
                    "Turns requests into tracked issues",
                    "将请求转为可跟踪的议题",
                ),
                false,
                "https://linear.app/developers",
                include_str!("../../assets/orbit-logos/linear.svg"),
            ),
            (
                "shopify",
                "Shopify",
                payments,
                t(
                    "Syncs products and orders from your store",
                    "同步商店的商品与订单",
                ),
                false,
                "https://shopify.dev/docs",
                include_str!("../../assets/orbit-logos/shopify.svg"),
            ),
            (
                "discord",
                "Discord",
                comms,
                t(
                    "Notifies your community about releases",
                    "向社区推送发布通知",
                ),
                false,
                "https://discord.com/developers/docs",
                include_str!("../../assets/orbit-logos/discord.svg"),
            ),
            (
                "vercel",
                "Vercel",
                dev,
                t("Comments preview links on builds", "在构建中附上预览链接"),
                true,
                "https://vercel.com/docs",
                include_str!("../../assets/orbit-logos/vercel.svg"),
            ),
            (
                "dropbox",
                "Dropbox",
                storage,
                t(
                    "Shares exported files with your team",
                    "与团队共享导出的文件",
                ),
                false,
                "https://www.dropbox.com/developers/documentation",
                include_str!("../../assets/orbit-logos/dropbox.svg"),
            ),
            (
                "framer",
                "Framer",
                design,
                t("Connects your published sites", "连接已发布的网站"),
                false,
                "https://www.framer.com/developers/",
                include_str!("../../assets/orbit-logos/framer.svg"),
            ),
            (
                "cloudflare",
                "Cloudflare",
                dev,
                t(
                    "Tracks deployments and domain changes",
                    "跟踪部署与域名变更",
                ),
                false,
                "https://developers.cloudflare.com",
                include_str!("../../assets/orbit-logos/cloudflare.svg"),
            ),
            (
                "loom",
                "Loom",
                comms,
                t(
                    "Attaches video walkthroughs to updates",
                    "给进度更新附上视频讲解",
                ),
                false,
                "https://www.loom.com/developers",
                include_str!("../../assets/orbit-logos/loom.svg"),
            ),
            (
                "supabase",
                "Supabase",
                storage,
                t("Connects your database and storage", "连接数据库与对象存储"),
                false,
                "https://supabase.com/docs",
                include_str!("../../assets/orbit-logos/supabase.svg"),
            ),
        ];
        let logos = data
            .into_iter()
            .map(|(id, name, category, description, connected, docs, svg)| {
                let logo = OrbitLogo::new(id, name)
                    .with_category(category)
                    .with_description(description)
                    .with_connected(connected)
                    .with_docs(docs);
                if matches!(id, "github" | "vercel") {
                    let path = svg
                        .split_once(" d=\"")
                        .and_then(|(_, rest)| rest.split('"').next())
                        .unwrap_or("");
                    logo.with_path(path, None)
                } else {
                    logo.with_image(Arc::new(Image::from_bytes(
                        ImageFormat::Svg,
                        svg.as_bytes().to_vec(),
                    )))
                }
            })
            .collect();
        let core = [
            Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../../../../website/public/assets/mark-light.png").to_vec(),
            )),
            Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!("../../../../website/public/assets/mark-dark.png").to_vec(),
            )),
        ];
        let orbit = cx.new(|cx| {
            OrbitLogos::new(logos, cx)
                .with_host_connections()
                .with_center(move |_, cx| {
                    img(core[usize::from(Theme::global(cx).mode == ThemeMode::Dark)].clone())
                        .size(px(28.))
                        .object_fit(ObjectFit::Contain)
                        .into_any_element()
                })
        });
        let mut failed_linear = false;
        let connections = cx.subscribe(
            &orbit,
            move |_, orbit, request: &OrbitConnectRequested, cx| {
                let success = request.id.as_ref() != "linear" || failed_linear;
                failed_linear |= !success;
                let ticket = request.ticket;
                let orbit = orbit.downgrade();
                cx.spawn(async move |_, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(720))
                        .await;
                    let _ = orbit.update(cx, |view, cx| view.finish_connect(ticket, success, cx));
                })
                .detach();
            },
        );
        let weak = orbit.downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                if weak
                    .update(cx, |view, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        view.set_visible(visible, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            orbit,
            _connections: connections,
        }
    }
}

impl Render for Example {
    fn render(&mut self, w: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let side = if w.viewport_size().width < px(340.) {
            16.
        } else {
            24.
        };
        div()
            .w_full()
            .px(px(side))
            .py(px(24.))
            .flex()
            .justify_center()
            .child(self.orbit.clone())
    }
}
