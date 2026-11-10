//! A workspace settings example with independent notification channels.
use gpui::{prelude::*, *};
use kui::{
    components::control_center::{ControlCenter, FocusMode, FocusPerson, NotificationChannel},
    primitives::{control::UiLanguage, editor::EditorScrollRegion, glyph::Glyph},
};
use std::sync::Arc;

pub const IDS: &[&str] = &["control-center"];

/// Reports the dial rectangle to the browser gesture router.
pub struct ControlCenterPreviewHost(pub fn(Bounds<Pixels>));
impl Global for ControlCenterPreviewHost {}

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

struct Example(Entity<ControlCenter>);

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let modes = vec![
            FocusMode::new(
                "deep",
                t("Deep work", "深度工作"),
                Glyph::Lock,
                t(
                    "Only mentions from your team get through",
                    "只接收团队成员的提及",
                ),
                50,
            ),
            FocusMode::new(
                "meetings",
                t("Meetings", "会议"),
                Glyph::Clock,
                t(
                    "Chat is held, calendar alerts stay on",
                    "暂停聊天提醒，保留日历通知",
                ),
                30,
            ),
            FocusMode::new(
                "rest",
                t("Wind down", "休息"),
                Glyph::Moon,
                t(
                    "Everything waits until tomorrow morning",
                    "通知延后到明天早上",
                ),
                90,
            ),
        ];
        let channels = vec![
            NotificationChannel::new("mentions", t("Mentions", "提及"))
                .with_description(t("When someone tags you", "有人提及时提醒"))
                .with_default_on(true),
            NotificationChannel::new("dm", t("Direct messages", "私信"))
                .with_description(t("One to one and small groups", "一对一与小组对话"))
                .with_default_on(true),
            NotificationChannel::new("deploys", t("Deploy alerts", "部署提醒"))
                .with_description(t("Failed builds on main", "主分支构建失败时提醒")),
        ];
        let people = [
            ("emily", "Emily", "#d539a3", "#b9dc6c"),
            ("tyler", "Tyler", "#665bf4", "#7ccaf3"),
            ("ryan", "Ryan", "#8ad2d1", "#e2ec83"),
        ]
        .map(|(id, name, a, b)| FocusPerson::new(id, name, avatar(a, b)))
        .to_vec();
        Self(cx.new(|cx| {
            ControlCenter::new(modes, channels, cx)
                .with_id("workspace-quick-settings")
                .with_label(t("Workspace quick settings", "工作区快捷设置"))
                .with_default_volume(64)
                .with_people(people)
        }))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let center = self.0.clone();
        let regions = canvas(
            |_, _, _| (),
            move |_, _, _, cx| {
                let center = center.read(cx);
                if let Some(host) = cx.try_global::<ControlCenterPreviewHost>() {
                    (host.0)(center.dial_bounds(cx).unwrap_or_default());
                }
                if let Some(host) = cx.try_global::<EditorScrollRegion>()
                    && let Some(list) = center.channel_scroll()
                {
                    let top = -list.offset().y;
                    (host.0)(
                        list.bounds(),
                        top > px(0.1),
                        top + px(0.1) < list.max_offset().y,
                    );
                }
            },
        )
        .absolute()
        .size(px(0.));
        div()
            .relative()
            .w_full()
            .px(px(12.))
            .flex()
            .justify_center()
            .child(self.0.clone())
            .child(regions)
    }
}

fn avatar(a: &str, b: &str) -> Arc<Image> {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><defs><linearGradient id="c" x2="0.8" y2="1"><stop stop-color="{a}"/><stop offset="1" stop-color="{b}"/></linearGradient></defs><circle cx="16" cy="16" r="16" fill="url(#c)"/></svg>"#
    );
    Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()))
}
