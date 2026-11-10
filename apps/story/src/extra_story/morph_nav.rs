//! A navigation sample with realistic sections, search and compact mode.
use gpui::{prelude::*, *};
use kui::{
    components::{
        morph_nav::{MorphNav, MorphNavItem, MorphNavLink, MorphNavNavigate, MorphNavSearchItem},
        segmented_control::{FormOption, SegmentedControl, SelectionChanged},
    },
    primitives::{
        control::UiLanguage,
        family::Paint,
        glyph::{Glyph, icon},
        presentation::{column, hint, row},
    },
};

pub const IDS: &[&str] = &["morph-nav"];

pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Example::new(w, cx)).into())
}

struct Example {
    navigation: Entity<MorphNav>,
    density: Entity<SegmentedControl>,
    destination: Option<SharedString>,
    _events: [Subscription; 2],
}

impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let link = |id, en, zh, detail, detail_zh, icon| {
            MorphNavLink::new(id, t(en, zh))
                .with_href(format!("/{id}"))
                .with_description(t(detail, detail_zh))
                .with_icon(icon)
        };
        let product = vec![
            link(
                "planning",
                "Planning",
                "规划",
                "Roadmaps, cycles, and dependencies",
                "路线图、迭代和依赖关系",
                Glyph::FolderKanban,
            ),
            link(
                "docs",
                "Docs",
                "文档",
                "Specs that stay in sync with the work",
                "与工作进度同步的规格",
                Glyph::FileText,
            ),
            link(
                "insights",
                "Insights",
                "洞察",
                "Cycle time and throughput by team",
                "按团队查看交付周期和产出",
                Glyph::Monitor,
            ),
            link(
                "automations",
                "Automations",
                "自动化",
                "Rules that triage, assign, and remind",
                "自动分流、分派和提醒",
                Glyph::Settings,
            ),
            link(
                "integrations",
                "Integrations",
                "集成",
                "GitHub, Slack, Figma, and 60 more",
                "GitHub、Slack、Figma 等集成",
                Glyph::Link,
            ),
            link(
                "security",
                "Security",
                "安全",
                "SSO, audit logs, and data residency",
                "单点登录、审计日志和数据驻留",
                Glyph::Lock,
            ),
        ];
        let solutions = vec![
            link(
                "engineering",
                "Engineering",
                "工程研发",
                "A clear view of what's shipping",
                "随时了解即将交付的工作",
                Glyph::Settings,
            ),
            link(
                "design",
                "Product design",
                "产品设计",
                "Connect research to delivery",
                "从研究衔接到交付",
                Glyph::Bulb,
            ),
            link(
                "operations",
                "Operations",
                "运营",
                "Keep recurring work on track",
                "持续跟进例行工作",
                Glyph::Folder,
            ),
            link(
                "startups",
                "Startups",
                "初创团队",
                "Plan and ship as one team",
                "和团队一起规划与交付",
                Glyph::Rocket,
            ),
        ];
        let resources = vec![
            link(
                "documentation",
                "Documentation",
                "开发文档",
                "Guides and API reference",
                "使用指南和接口参考",
                Glyph::Book,
            ),
            link(
                "changelog",
                "Changelog",
                "更新日志",
                "What shipped this week",
                "本周发布的功能",
                Glyph::FileText,
            ),
            link(
                "community",
                "Community",
                "社区",
                "Meet other teams and share notes",
                "和其他团队交流心得",
                Glyph::User,
            ),
            link(
                "help",
                "Help center",
                "帮助中心",
                "Answers from our support team",
                "来自支持团队的解答",
                Glyph::HelpCircle,
            ),
        ];
        let pricing = MorphNavLink::new("pricing", t("Pricing", "定价"))
            .with_description(t("Free, Team and Enterprise", "免费版、团队版和企业版"))
            .with_href("/pricing")
            .with_icon(Glyph::Wallet);
        let entry = |destination: &MorphNavLink, group, section| {
            MorphNavSearchItem::new(destination.clone())
                .with_group(group)
                .with_section(section)
        };
        let mut search = vec![
            entry(&product[0], t("Product", "产品"), "product"),
            entry(&product[3], t("Product", "产品"), "product"),
            MorphNavSearchItem::new(pricing).with_group(t("Company", "公司")),
            entry(&resources[0], t("Resources", "资源"), "resources"),
            entry(&resources[1], t("Resources", "资源"), "resources"),
        ];
        search.extend(
            [1, 2, 4, 5]
                .into_iter()
                .map(|i| entry(&product[i], t("Product", "产品"), "product")),
        );
        search.extend(
            solutions
                .iter()
                .map(|link| entry(link, t("Solutions", "解决方案"), "solutions")),
        );
        search.extend(
            resources[2..]
                .iter()
                .map(|link| entry(link, t("Resources", "资源"), "resources")),
        );
        let product_art = cx.new(|_| FeatureArt {
            mark: Glyph::Rocket,
            top: 0xd1b8ee,
            bottom: 0x7050ad,
        });
        let resource_art = cx.new(|_| FeatureArt {
            mark: Glyph::Book,
            top: 0xaacbdd,
            bottom: 0x53698a,
        });
        let nav = cx.new(|cx| {
            MorphNav::new(
                "KUI",
                vec![
                    MorphNavItem::new("product", t("Product", "产品"))
                        .with_links(product)
                        .with_feature(link(
                            "launch",
                            "A clearer release plan",
                            "清晰的发布计划",
                            "Plan, write, and ship from one place",
                            "在一个地方完成规划、协作和交付",
                            Glyph::Rocket,
                        ))
                        .with_feature_art(product_art.into())
                        .with_footer(
                            MorphNavLink::new(
                                "automation-schedule",
                                t(
                                    "Automations can now run on a schedule",
                                    "自动化现已支持定时运行",
                                ),
                            )
                            .with_icon(Glyph::Arrow),
                        ),
                    MorphNavItem::new("solutions", t("Solutions", "解决方案"))
                        .with_links(solutions),
                    MorphNavItem::new("resources", t("Resources", "资源"))
                        .with_links(resources)
                        .with_feature(link(
                            "planning-guide",
                            "A calm planning week",
                            "从容的规划周",
                            "Guide · 12 min read",
                            "指南 · 阅读约 12 分钟",
                            Glyph::Book,
                        ))
                        .with_feature_art(resource_art.into()),
                    MorphNavItem::new("pricing", t("Pricing", "定价")).with_href("/pricing"),
                ],
                w,
                cx,
            )
            .with_brand(
                MorphNavLink::new("home", "KUI")
                    .with_href("/")
                    .with_icon(Glyph::FolderKanban),
            )
            .with_label("KUI")
            .with_action(
                MorphNavLink::new("start", t("Start free", "免费开始")).with_href("/start"),
            )
            .with_search(t("Search KUI", "搜索 KUI"), search, w, cx)
        });
        let density = cx.new(|cx| {
            SegmentedControl::new(
                t("Bar size", "导航栏大小"),
                vec![
                    FormOption::new("full", t("Full", "完整")),
                    FormOption::new("compact", t("Compact", "紧凑")),
                ],
                w,
                cx,
            )
            .with_selected(&["full"])
        });
        let events = [
            cx.subscribe(&nav, |s, nav, event: &MorphNavNavigate, cx| {
                s.destination = Some(event.label.clone());
                nav.update(cx, |nav, cx| nav.set_current(event.section.clone(), cx));
                cx.notify();
            }),
            cx.subscribe(&density, |s, _, event: &SelectionChanged, cx| {
                let compact = event.0.first().is_some_and(|key| key.as_ref() == "compact");
                s.navigation
                    .update(cx, |nav, cx| nav.set_compact(compact, cx));
            }),
        ];
        Self {
            navigation: nav,
            density,
            destination: None,
            _events: events,
        }
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let caption = self.destination.as_ref().map_or_else(
            || {
                lang.text(
                    "Hover a section or press search",
                    "悬停查看栏目，或打开搜索",
                )
                .to_owned()
            },
            |target| format!("{} {target}", lang.text("Selected", "已选择")),
        );
        column()
            .w_full()
            .max_w(px(780.))
            .min_h(px(634.))
            .px(px(12.))
            .py(px(52.))
            .items_center()
            .gap(px(14.))
            .child(self.navigation.clone())
            .child(div().min_h(px(24.)).flex_1())
            .child(row().min_h(px(19.)).child(hint(caption, p)))
            .child(self.density.clone())
    }
}

struct FeatureArt {
    mark: Glyph,
    top: u32,
    bottom: u32,
}

impl Render for FeatureArt {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        row()
            .w_full()
            .h(px(84.))
            .flex_none()
            .justify_center()
            .rounded(kui::foundation::surface_radius(cx))
            .bg(linear_gradient(
                132.,
                linear_color_stop(rgb(self.top), 0.),
                linear_color_stop(rgb(self.bottom), 1.),
            ))
            .child(
                row()
                    .size(px(42.))
                    .justify_center()
                    .rounded(kui::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(white().opacity(0.4))
                    .bg(white().opacity(0.16))
                    .child(icon(self.mark, 23., white())),
            )
    }
}
