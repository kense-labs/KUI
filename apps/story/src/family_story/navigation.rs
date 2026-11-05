//! Navigation, disclosure, media and structured-data examples.
use gpui::{prelude::*, *};
use kui::components::{
    accordion::{Accordion, AccordionItem},
    breadcrumb::{Breadcrumb, Navigated},
    code_block::CodeBlock,
    confirm_popover::{ConfirmPopover, ConfirmRequested},
    data_grid::{CsvDownload, DataGrid, GridExport},
    drawer::Drawer,
    expandable_card::DetailCard,
    expandable_text::ExpandableText,
    filter_toolbar::{Facet, FilterToolbar},
    hover_card::PreviewCard,
    json_viewer::{JsonValue, JsonViewer},
    lightbox_gallery::{GalleryPhoto, PhotoExhibit},
    pagination::{PageChanged, Pagination},
    reorderable_list::{OrderedItem, ReorderableList},
    scroll_area::ScrollArea,
    sortable_data_table::{SortableTable, TableColumn},
    toc_progress::{ReadingOutline, ReadingSection},
    tooltip::Tooltip,
    tree_view::{TreeNode, TreeSelected, TreeView},
};
use kui::foundation::surface_radius;
use kui::primitives::{
    control::UiLanguage,
    family::Paint,
    nav::{button, column, fit, label, panel, row, tr},
};
use std::{sync::Arc, time::Duration};
pub fn story(id: &str, w: &mut Window, cx: &mut Context<super::FamilyStory>) -> AnyView {
    cx.new(|cx| NavDemo::new(id, w, cx)).into()
}
struct NavDemo {
    id: String,
    views: Vec<AnyView>,
    notice: SharedString,
    page: usize,
    fail: bool,
    removed: [bool; 2],
    crumb: usize,
    trail: Option<Entity<Breadcrumb>>,
    _subs: Vec<Subscription>,
}
fn strs(items: &[&str]) -> Vec<SharedString> {
    items.iter().map(|s| (*s).to_owned().into()).collect()
}
fn activity(l: UiLanguage) -> Vec<(&'static str, &'static str, &'static str)> {
    let t = |a, b| l.text(a, b);
    vec![
        (
            "EC",
            t(
                "Emma approved the onboarding update",
                "艾玛批准了入门流程更新",
            ),
            "2m",
        ),
        (
            "MJ",
            t("Marcus merged the checkout fix", "马库斯合并了结账修复"),
            "9m",
        ),
        (
            "CN",
            t("Chloe shared the retention report", "克洛伊分享了留存报告"),
            "21m",
        ),
        (
            "DK",
            t(
                "Daniel rotated the staging API keys",
                "丹尼尔轮换了预发布密钥",
            ),
            "38m",
        ),
        (
            "SR",
            t(
                "Sofia moved the review to Thursday",
                "索菲娅将评审调整到周四",
            ),
            "1h",
        ),
        (
            "JB",
            t(
                "Jasmine commented on the pricing page",
                "杰丝敏评论了定价页",
            ),
            "1h",
        ),
        (
            "OB",
            t("Olivia increased the worker pool", "奥利维娅扩容了工作节点"),
            "2h",
        ),
        ("RS", t("Ryan closed the renewal", "瑞安完成了续约"), "3h"),
        (
            "HW",
            t("Hannah resolved the queue", "汉娜处理完了队列"),
            "4h",
        ),
        (
            "MA",
            t("Mateo published a mobile build", "马特奥发布了移动端构建"),
            "5h",
        ),
        (
            "NC",
            t("Nathan scheduled planning", "内森安排了规划会议"),
            "6h",
        ),
        (
            "AM",
            t("Ava launched the campaign", "艾娃启动了推广活动"),
            "1d",
        ),
    ]
}
const COLORS: [u32; 6] = [0x725784, 0x547890, 0x8f6f47, 0x547260, 0xad666f, 0x606d93];
fn avatar(initials: &'static str, index: usize) -> Div {
    row()
        .size(px(26.))
        .flex_none()
        .rounded_full()
        .justify_center()
        .bg(rgb(COLORS[index % COLORS.len()]))
        .text_size(px(10.))
        .text_color(white())
        .child(initials)
}
impl NavDemo {
    fn new(id: &str, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut s = Self {
            id: id.to_string(),
            views: vec![],
            notice: SharedString::default(),
            page: 2,
            fail: false,
            removed: [false; 2],
            crumb: 2,
            trail: None,
            _subs: vec![],
        };
        let l = UiLanguage::current(cx);
        let t = |a, b| l.text(a, b);
        let primary: AnyView = match id {
            "tooltip" => cx
                .new(|cx| {
                    Tooltip::new(
                        t("Project visibility information", "项目可见性说明"),
                        t("Only workspace members can view this project.", "只有工作区成员能查看此项目。"),
                        cx,
                    )
                })
                .into(),

            "scroll-area" => {
                let list = cx.new(|_| {
                    ScrollArea::new(
                        277.,
                        Box::new(move |_, cx| {
                            let p = Paint::get(cx);

                            column()
                                .children(activity(l).into_iter().enumerate().map(|(i, (initials, text, time))| {
                                    row()
                                        .h(px(40.))
                                        .flex_none()
                                        .px_2()
                                        .gap_2()
                                        .child(avatar(initials, i))
                                        .child(label(text, 13., p.ink).flex_1())
                                        .child(label(time, 11., p.faint))
                                }))
                                .into_any_element()
                        }),
                    )
                });

                let rooms = ROOMS.map(|(name, zh, bytes)| (name, zh, local_art(bytes)));

                let strip = cx.new(|_| {
                    ScrollArea::new(
                        201.,
                        Box::new(move |_, cx| {
                            let p = Paint::get(cx);
                            row()
                                .gap_2()
                                .children(rooms.iter().map(|(name, zh, image)| {
                                    column()
                                        .flex_none()
                                        .w(px(167.))
                                        .gap_1()
                                        .child(
                                            img(image.clone())
                                                .h(px(163.))
                                                .w_full()
                                                .rounded(surface_radius(cx))
                                                .object_fit(ObjectFit::Cover),
                                        )
                                        .child(label(l.text(name, zh), 13., p.ink))
                                        .child(label(l.text("Museum collection", "博物馆馆藏"), 11., p.secondary))
                                }))
                                .into_any_element()
                        }),
                    )
                    .horizontal()
                });
                s.views.push(list.into());
                strip.into()
            }

            "drawer" => cx
                .new(|cx| {
                    Drawer::new(
                        t("Open details", "查看详情"),
                        t("Project details", "项目详情"),
                        Box::new(move |_, cx| {
                            let p = Paint::get(cx);
                            column()
                                .gap_3()
                                .child(label(l.text("Launch workspace", "发布工作区"), 16., p.ink))
                                .child(label(
                                    l.text(
                                        "Review the owner, release stage, and next review from one place.",
                                        "在这里查看负责人、发布阶段和下一次评审。",
                                    ),
                                    14.,
                                    p.secondary,
                                ))
                                .children(
                                    [
                                        ("Owner · Maya Chen", "负责人 · 陈玛雅"),
                                        ("Stage · In review", "阶段 · 审核中"),
                                        ("Next review · October 4", "下次评审 · 10 月 4 日"),
                                    ]
                                    .map(|(a, b)| label(l.text(a, b), 13., p.ink).py_2().border_b_1().border_color(p.line)),
                                )
                                .into_any_element()
                        }),
                        cx,
                    ).contained()
                })
                .into(),

            "accordion" => cx
                .new(|cx| {
                    Accordion::new(
                        vec![
                            AccordionItem::new(
                                t("What is included?", "包含哪些内容？"),
                                t(
                                    "Keyboard-accessible controls, responsive layouts, and animation that follows your motion preference.",
                                    "包含支持键盘操作的控件、自适应布局，以及遵循系统减少动效设置的动画。",
                                ),
                            ),
                            AccordionItem::new(
                                t("Can I use this in a product?", "可以在产品中使用吗？"),
                                t(
                                    "Install the component in your workspace and connect it to your own data.",
                                    "将组件安装到工作区，再连接自己的业务数据。",
                                ),
                            ),
                        ],
                        Some(0),
                        cx,
                    )
                })
                .into(),

            "breadcrumb" => {
                let v = cx.new(|_| {
                    Breadcrumb::new(strs(&[
                        t("Projects", "项目"),
                        t("Launch", "发布"),
                        t("Design review", "设计评审"),
                        t("Motion pass", "动效检查"),
                        t("Final copy", "最终文案"),
                    ]))
                });
                v.update(cx, |v, cx| v.navigate(2, cx));
                s._subs.push(cx.subscribe(&v, |s: &mut Self, _, e: &Navigated, cx| {
                    s.crumb = e.0;
                    cx.notify()
                }));
                s.trail = Some(v.clone());
                v.into()
            }

            "tree-view" => {
                let v = cx.new(|cx| {
                    TreeView::new(
                        vec![
                            TreeNode::new(
                                "src",
                                "src",
                                vec![
                                    TreeNode::new(
                                        "components",
                                        "components",
                                        vec![
                                            TreeNode::new("button", "button.rs", vec![]),
                                            TreeNode::new("style", "style.rs", vec![]),
                                        ],
                                    ),
                                    TreeNode::new("app", "app.rs", vec![]),
                                ],
                            ),
                            TreeNode::new("readme", "README.md", vec![]),
                        ],
                        cx,
                    )
                    .expanded(strs(&["src", "components"]))
                });
                s._subs.push(cx.subscribe(&v, |s: &mut Self, _, e: &TreeSelected, cx| {
                    s.notice = e.0.clone();
                    cx.notify()
                }));
                v.into()
            }

            "confirm-popover" => {
                for i in 0..2 {
                    let v = cx.new(|cx| {
                        ConfirmPopover::new(
                            if i == 0 { t("Remove", "移除") } else { t("Delete", "删除") },
                            if i == 0 {
                                t("Remove Jordan’s access to this workspace?", "移除乔丹访问此工作区的权限？")
                            } else {
                                t(
                                    "This will delete the project and its deployment history.",
                                    "这将删除项目及其部署记录。",
                                )
                            },
                            (i == 1).then(|| "Atlas".into()),
                            w,
                            cx,
                        )
                    });
                    s._subs.push(cx.subscribe(&v, move |s: &mut Self, v, e: &ConfirmRequested, cx| {
                        let ticket = e.0;
                        let ok = !s.fail;
                        s.fail = false;
                        cx.spawn(async move |this, cx| {
                            cx.background_executor().timer(Duration::from_millis(640)).await;
                            let _ = this.update(cx, |s, cx| {
                                v.update(cx, |v, cx| v.resolve(ticket, ok, cx));

                                s.notice = if ok {
                                    tr(cx, "Changes saved", "已保存更改")
                                } else {
                                    tr(cx, "Request failed. You can retry.", "请求失败，可以重试。")
                                }
                                .into();
                                cx.notify()
                            });
                            if ok {
                                cx.background_executor().timer(Duration::from_millis(1100)).await;
                                let _ = this.update(cx, |s, cx| {
                                    s.removed[i] = true;
                                    cx.notify()
                                });
                            }
                        })
                        .detach()
                    }));
                    s.views.push(v.into())
                }
                return s;
            }

            "pagination" => {
                let v = cx.new(|_| Pagination::new(24, 2));
                s._subs.push(cx.subscribe(&v, |s: &mut Self, _, e: &PageChanged, cx| {
                    s.page = e.0;
                    cx.notify()
                }));
                v.into()
            }

            "filter-toolbar" => cx
                .new(|_| {
                    FilterToolbar::new(
                        vec![
                            Facet::new(
                                t("Status", "状态"),
                                strs(&[t("In review", "审核中"), t("On track", "进度正常"), t("Done", "已完成")]),
                            ),
                            Facet::new(t("Owner", "负责人"), strs(&["Emma Collins", "Marcus Johnson", "Ari Singh"])),
                            Facet::new(t("Priority", "优先级"), strs(&[t("High", "高"), t("Medium", "中"), t("Low", "低")])),
                        ],
                        vec![(0, 0), (1, 0)],
                    )
                })
                .into(),

            "sortable-data-table" => {
                let cols = [
                    (t("Project", "项目"), false),
                    (t("Owner", "负责人"), false),
                    (t("Status", "状态"), false),
                    (t("Due", "截止日期"), false),
                    (t("Score", "评分"), true),
                ]
                .map(|(n, v)| TableColumn::new(n, v))
                .into_iter()
                .collect::<Vec<_>>();
                let rows = [
                    [t("Docs refresh", "文档更新"), "Marcus Johnson", t("Done", "已完成"), "Sep 18", "97"],
                    [
                        t("Aurora redesign", "极光改版"),
                        "Emma Collins",
                        t("In review", "审核中"),
                        "Oct 2",
                        "92",
                    ],
                    [
                        t("Billing migration", "账单迁移"),
                        "Noah Williams",
                        t("On track", "进度正常"),
                        "Oct 14",
                        "86",
                    ],
                    [
                        t("Mobile onboarding", "移动端入门"),
                        "Ari Singh",
                        t("On track", "进度正常"),
                        "Nov 6",
                        "78",
                    ],
                    [
                        t("Cinder analytics", "分析面板"),
                        "Hadi Park",
                        t("At risk", "有风险"),
                        "Sep 28",
                        "64",
                    ],
                ]
                .map(|a| strs(&a))
                .to_vec();
                cx.new(|_| SortableTable::new(t("Project health", "项目健康度"), cols, rows)).into()
            }

            "reorderable-list" => cx
                .new(|cx| {
                    ReorderableList::new(
                        vec![
                            OrderedItem::new("brief", t("Clarify onboarding", "梳理入门流程"), t("Product brief", "产品简报")),
                            OrderedItem::new("empty", t("Review empty states", "评审空状态"), t("Design system", "设计系统")),
                            OrderedItem::new("ship", t("Ship the update", "发布更新"), t("Release checklist", "发布检查清单")),
                        ],
                        cx,
                    )
                })
                .into(),

            "expandable-card" => cx
                .new(|_| {
                    DetailCard::new(
                        t("Launch checklist", "发布检查清单"),
                        t("4 of 6 items complete", "已完成 4 / 6 项"),
                        Box::new(move |_, cx| {
                            let p = Paint::get(cx);
                            column()
                                .gap_2()
                                .child(label(l.text("Two steps remain", "还有两项待完成"), 15., p.ink))
                                .child(label(l.text("Invite reviewers", "邀请评审人"), 13., p.ink))
                                .child(label("Maya Chen · Jonas Weber", 12., p.secondary))
                                .child(label(l.text("Add release notes", "添加发布说明"), 13., p.ink))
                                .child(label(l.text("Draft saved on Tuesday", "草稿已于周二保存"), 12., p.secondary))
                                .into_any_element()
                        }),
                    )
                })
                .into(),

            "hover-card" => cx
                .new(|cx| {
                    PreviewCard::new(
                        "Jasmine Brooks",
                        "JB",
                        Box::new(move |_, cx| {
                            let p = Paint::get(cx);
                            column()
                                .gap_2()
                                .child(avatar("JB", 3))
                                .child(label("Jasmine Brooks", 15., p.ink))
                                .child(label(
                                    l.text("Product designer · Design systems", "产品设计师 · 设计系统"),
                                    13.,
                                    p.secondary,
                                ))
                                .child(label(
                                    l.text("Working on the next generation of workspace tools.", "正在设计下一代工作区工具。"),
                                    13.,
                                    p.ink,
                                ))
                                .into_any_element()
                        }),
                        cx,
                    )
                })
                .into(),

            "expandable-text" => {
                for (a, b, lines) in [
                    (PRODUCT, PRODUCT_ZH, 3),
                    (REVIEW, REVIEW_ZH, 2),
                    (
                        "Soft lines at the rim catch the light. I keep noticing new details in the glaze as I turn the bowl toward the window. The photographs reveal a much deeper and more varied surface than I expected.",
                        "杯沿线条柔和。转动茶碗迎向窗边时，釉面的细节随光线变化。和最初的印象相比，照片展示出的釉色更加深邃丰富。",
                        2,
                    ),
                    (
                        "Love the quiet contrast between the dark glaze and the rim.",
                        "喜欢深色釉面和杯沿之间含蓄的对比。",
                        3,
                    ),
                ] {
                    s.views.push(cx.new(|_| ExpandableText::new(t(a, b), lines)).into())
                }
                return s;
            }

            "toc-progress" => {
                let parts = [
                    ("Why cold starts mattered", "冷启动的影响"),
                    ("Measuring the boot", "测量启动耗时"),
                    ("Tracing every phase", "跟踪各个阶段"),
                    ("Module resolution", "模块解析"),
                    ("Loading lazily", "按需加载"),
                    ("Splitting the bundle", "拆分产物"),
                    ("Warming the cache", "预热缓存"),
                    ("Results", "测试结果"),
                ];
                let bodies = [
                    (
                        "A request spends time resolving modules before it can answer the first checkout. We recorded a trace for each step and compared warm requests against fresh workers.",
                        "第一次结账需要先解析模块。我们记录每个步骤的耗时，对比缓存命中和新工作进程的结果。",
                    ),
                    (
                        "We replayed production requests on clean workers. Timing spans around imports, initialization and network calls showed where the latency accumulated.",
                        "我们在新进程上重放生产请求，逐段测量导入、初始化与网络调用，定位主要延迟。",
                    ),
                    (
                        "The tracing timeline separated cache reads from module compilation. Smaller startup bundles let the first request finish earlier.",
                        "跟踪时间线将缓存读取与模块编译分开。减小启动包以后，首个请求可以更早完成。",
                    ),
                ];
                let entries = parts
                    .iter()
                    .enumerate()
                    .map(|(i, (a, b))| {
                        ReadingSection::new(t(a, b), t(bodies[i % 3].0, bodies[i % 3].1), usize::from(matches!(i, 2 | 3 | 5)))
                    })
                    .collect();
                cx.new(|_| ReadingOutline::new(t("Reducing checkout cold starts", "缩短结账服务的冷启动时间"), entries, 6))
                    .into()
            }

            "code-block" => cx
                .new(|cx| {
                    CodeBlock::new(
                        "button.rs",
                        "Rust",
                        "Button::new(\"review\")\n    .label(\"Review changes\").secondary()",
                        cx,
                    )
                })
                .into(),

            "json-viewer" => {
                cx.new(|cx| JsonViewer::new("order", sample_order(), w, cx).with_max_height(380.)).into()
            }

            "data-grid" => {
                let names = [
                    "Northwind",
                    "Lumen",
                    "Cedar",
                    "Brightline",
                    "Aurora",
                    "Redwood",
                    "Bluebird",
                    "Cinder",
                ];
                let owners = ["Owen Park", "Tyler Hayes", "Marcus Johnson", "Priya Raman"];
                let regions = [
                    t("North America", "北美"),
                    t("Europe", "欧洲"),
                    t("Asia Pacific", "亚太"),
                    t("Latin America", "拉丁美洲"),
                ];
                let stages = [
                    t("Closed won", "已签约"),
                    t("Proposal", "提案"),
                    t("Discovery", "调研"),
                    t("Negotiation", "谈判"),
                ];
                let data = (0..10_000)
                    .map(|i| {
                        vec![
                            format!("{} {}", names[i % 8], t("Freight", "货运")).into(),
                            owners[(i / 3) % 4].into(),
                            regions[(i / 5) % 4].into(),
                            stages[(i / 7) % 4].into(),
                            (40 + (i * 65) % 720).to_string().into(),
                            format!("${}", (40 + (i * 65) % 720) * 240).into(),
                            format!("{}%", 58 + (i * 7) % 43).into(),
                            format!("2027-{:02}-{:02}", 1 + i % 12, 1 + (i / 12) % 28).into(),
                        ]
                    })
                    .collect();
                let grid = cx.new(|cx| {
                    DataGrid::new(
                        t("Renewals", "客户续约"),
                        strs(&[
                            t("Account", "客户"),
                            t("Owner", "负责人"),
                            t("Region", "区域"),
                            t("Stage", "阶段"),
                            t("Seats", "席位"),
                            "ARR",
                            t("Health", "健康度"),
                            t("Renews", "续约日期"),
                        ]),
                        data,
                        w,
                        cx,
                    )
                });
                s._subs.push(cx.subscribe(&grid, |_: &mut Self, _, export: &GridExport, cx| {
                    if let Some(host) = cx.try_global::<CsvDownload>() {
                        (host.0)(&export.filename, &export.csv);
                    } else {
                        #[cfg(not(target_family = "wasm"))]
                        {
                        let dir = std::env::current_dir().unwrap_or_else(|_| ".".into());
                        let path = cx.prompt_for_new_path(&dir, Some(&export.filename));
                        let csv = export.csv.clone();
                        cx.spawn(async move |view, cx| {
                            let failure = match path.await {
                                Ok(Ok(Some(file))) => cx
                                    .background_executor()
                                    .spawn(async move { std::fs::write(file, csv) })
                                    .await
                                    .is_err(),
                                Ok(Ok(None)) => return,
                                _ => true,
                            };
                            let _ = view.update(cx, |s, cx| {
                                s.notice = if failure {
                                    tr(cx, "Could not save CSV", "无法保存 CSV")
                                } else {
                                    tr(cx, "CSV saved", "已保存 CSV")
                                }
                                .into();
                                cx.notify()
                            });
                        })
                        .detach()
                        }
                    }
                }));
                grid.into()
            }

            "lightbox-gallery" => cx.new(|cx| PhotoExhibit::new(photos(l), cx)).into(),
            _ => unreachable!("unregistered navigation example"),
        };

        s.views.push(primary);
        s
    }
}
const PRODUCT: &str = "A study in glaze and clay. Dark patches gather across the surface of this teabowl while the lighter edge traces its rounded profile. Look closely at the changing color around the rim and the foot. The photograph comes from the open-access collection of the Cleveland Museum of Art.";
const PRODUCT_ZH: &str = "这只茶碗展示了釉色与陶土的关系。深色斑纹散落在器身表面，较浅的杯沿勾勒出流畅的轮廓。仔细观察杯沿和圈足附近，颜色会随光线悄然变化。图片来自克利夫兰艺术博物馆的开放馆藏。";
const REVIEW: &str = "The walls feel substantial without making the rim heavy. We have used these for coffee every morning. The small handle fits two fingers comfortably. I reach for a saucer when the cup is fresh from the kettle.";
const REVIEW_ZH: &str = "杯壁厚实，杯沿却很轻巧。我们每天早晨都用它喝咖啡。小杯把刚好能放下两根手指。刚倒入热水时，我习惯再搭配一个杯碟。";
fn sample_order() -> JsonValue {
    use serde_json::json;
    let events: Vec<_> = (0..180)
        .map(|i| {
            let event_type = if i % 5 == 3 {
                "payment_authorized"
            } else {
                ["order_read", "cart_updated", "email_opened"][i % 3]
            };
            let mut value = json!({
                "id": format!("evt_{i:04}"),
                "type": event_type,
                "at": format!("2026-09-14T{:02}:{:02}:00Z", (i / 6) % 24, i * 7 % 60),
                "agent": (["Desktop","Mobile","Tablet"][i % 3])
            });
            if i == 131 {
                value["note"] = json!("Refund requested for a damaged item");
            }
            value
        })
        .collect();
    json!({
        "id":"ord_8Kd2Q7","object":"order","status":"fulfilled","livemode":false,
        "created":1757851200u64,"currency":"usd","amount_total":48600,
        "customer":{"id":"cus_70","name":"Sam Reyes","email":"sam@example.com","address":{"city":"Seattle","country":"US"},"language":"en"},
        "items":[{"sku":"linen","quantity":2},{"sku":"stone","quantity":1},{"sku":"oak","quantity":3}],
        "shipping":{"carrier":"Ground","service":"tracked","tracking":"0004472","delivered":true},
        "discount":null,"metadata":{"campaign id":"spring","gift wrap":false},"events":events
    })
}
fn local_art(bytes: &'static [u8]) -> ImageSource {
    Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec())).into()
}
const ROOMS: [(&str, &str, &[u8]); 6] = [
    (
        "Interior",
        "室内",
        include_bytes!("../../assets/navigation/room-0.jpg"),
    ),
    (
        "Drawing room",
        "会客厅",
        include_bytes!("../../assets/navigation/room-1.jpg"),
    ),
    (
        "Kitchen",
        "厨房",
        include_bytes!("../../assets/navigation/room-2.jpg"),
    ),
    (
        "Pantheon",
        "万神殿",
        include_bytes!("../../assets/navigation/room-3.jpg"),
    ),
    (
        "Church",
        "教堂",
        include_bytes!("../../assets/navigation/room-4.jpg"),
    ),
    (
        "Synagogue",
        "会堂",
        include_bytes!("../../assets/navigation/room-5.jpg"),
    ),
];
impl Render for NavDemo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let t = |a, b| l.text(a, b);
        let root = column().items_center().gap_3();
        match self.id.as_str() {
            "drawer" => panel(cx)
                .debug_selector(|| "drawer-stage".into())
                .w(fit(w, 536.))
                .h(px(360.))
                .justify_center()
                .items_center()
                .child(self.views[0].clone()),
            "tooltip" => root.child(self.views[0].clone()).child(label(
                t("Hover or focus the icon", "悬停或聚焦图标"),
                12.,
                p.secondary,
            )),
            "scroll-area" => {
                let mut blocks = column().w(fit(w, 520.)).gap_3();
                for (i, (a, b)) in [("Activity", "动态"), ("Rooms", "房间")]
                    .into_iter()
                    .enumerate()
                {
                    blocks = blocks.child(
                        panel(cx)
                            .p_3()
                            .gap_1()
                            .child(label(t(a, b), 14., p.ink))
                            .child(self.views[i].clone()),
                    )
                }
                blocks
            }
            "breadcrumb" => {
                let mut shell = panel(cx)
                    .w(fit(w, 480.))
                    .p_3()
                    .gap_2()
                    .child(self.views[0].clone());
                for (i, (a, b)) in [
                    ("Projects", "项目"),
                    ("Launch", "发布"),
                    ("Design review", "设计评审"),
                    ("Motion pass", "动效检查"),
                    ("Final copy", "最终文案"),
                ]
                .into_iter()
                .enumerate()
                .skip(self.crumb + 1)
                {
                    if i > self.crumb + 2 {
                        break;
                    }
                    let name = t(a, b);
                    shell = shell.child(
                        button(("path", i), name, cx)
                            .justify_between()
                            .child(name)
                            .child("›")
                            .on_click(cx.listener(move |s, _, _, cx| {
                                if let Some(v) = &s.trail {
                                    v.update(cx, |v, cx| v.navigate(i, cx))
                                }
                            })),
                    )
                }
                shell
            }
            "tree-view" => root
                .child(self.views[0].clone())
                .when(!self.notice.is_empty(), |b| {
                    b.child(label(self.notice.clone(), 12., p.secondary))
                }),
            "confirm-popover" => {
                let mut card = panel(cx).w(fit(w, 480.)).p_3().gap_2().child(label(
                    t("Atlas settings", "Atlas 设置"),
                    16.,
                    p.ink,
                ));
                for (i, (name, detail)) in [
                    (
                        t("Jordan Reyes", "乔丹·雷耶斯"),
                        "Editor · jordan@example.com",
                    ),
                    (
                        t("Delete project", "删除项目"),
                        t("14 deployments and 3 environments", "14 次部署和 3 个环境"),
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    if self.removed[i] {
                        continue;
                    }
                    card = card.child(
                        row()
                            .py_2()
                            .gap_2()
                            .when(i == 0, |e| e.child(avatar("JR", 4)))
                            .child(
                                column()
                                    .flex_1()
                                    .child(label(name, 14., p.ink))
                                    .child(label(detail, 12., p.secondary)),
                            )
                            .child(self.views[i].clone()),
                    )
                }
                let opts = if self.removed.iter().any(|x| *x) {
                    t("Reset example", "重置示例")
                } else {
                    t("Fail the next attempt", "下次请求模拟失败")
                };
                root.child(card).child(
                    row()
                        .flex_wrap()
                        .gap_3()
                        .child(
                            button("failure", opts, cx)
                                .child(if self.fail { "☑" } else { "□" })
                                .child(opts)
                                .on_click(cx.listener(|s, _, _, cx| {
                                    if s.removed.iter().any(|x| *x) {
                                        s.removed = [false; 2];
                                        s.notice = Default::default()
                                    } else {
                                        s.fail = !s.fail
                                    }
                                    cx.notify()
                                })),
                        )
                        .child(label(
                            if self.notice.is_empty() {
                                t("Nothing has changed yet", "尚未发生更改").into()
                            } else {
                                self.notice.clone()
                            },
                            12.,
                            p.secondary,
                        )),
                )
            }
            "pagination" => root
                .child(label(
                    format!(
                        "{} {}–{}",
                        t("Showing projects", "显示项目"),
                        (self.page - 1) * 10 + 1,
                        self.page * 10
                    ),
                    12.,
                    p.secondary,
                ))
                .child(self.views[0].clone()),
            "expandable-text" => {
                let narrow = w.viewport_size().width < px(360.);
                let figure = local_art(include_bytes!("../../assets/navigation/teabowl.jpg"));
                let intro = row()
                    .items_start()
                    .gap_3()
                    .child(
                        div()
                            .w(px(if narrow { 80. } else { 152. }))
                            .h(px(if narrow { 116. } else { 178. }))
                            .flex_none()
                            .rounded(surface_radius(cx))
                            .overflow_hidden()
                            .child(
                                img(figure)
                                    .size_full()
                                    .rounded(surface_radius(cx))
                                    .object_fit(ObjectFit::Cover),
                            ),
                    )
                    .child(
                        column()
                            .min_w_0()
                            .flex_1()
                            .gap_2()
                            .child(label(t("Tortoiseshell teabowl", "玳瑁釉茶碗"), 16., p.ink))
                            .child(self.views[0].clone()),
                    );
                let mut reviews = column().w(fit(w, 520.)).gap_4().child(intro).child(label(
                    t("Reviews   4.7 from 128", "点评   4.7 分 · 128 条"),
                    14.,
                    p.ink,
                ));
                for (i, (name, initials, when, summary)) in [
                    (
                        "Hannah Walsh",
                        "HW",
                        "Sep 18",
                        t("A glaze full of detail", "细节丰富的釉面"),
                    ),
                    (
                        "Daniel Kim",
                        "DK",
                        "Sep 9",
                        t("Lovely curves", "柔和的轮廓"),
                    ),
                    (
                        "Sofia Ramirez",
                        "SR",
                        "Aug 30",
                        t("Beautiful contrast", "漂亮的对比"),
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    reviews = reviews.child(
                        column()
                            .gap_1()
                            .pb_3()
                            .border_b_1()
                            .border_color(p.line)
                            .child(
                                row()
                                    .gap_2()
                                    .child(avatar(initials, i))
                                    .child(label(name, 13., p.ink).flex_1())
                                    .child(label(when, 12., p.faint)),
                            )
                            .child(label(
                                format!(
                                    "{}   {summary}",
                                    if i == 1 {
                                        "★★★★☆"
                                    } else {
                                        "★★★★★"
                                    }
                                ),
                                13.,
                                p.ink,
                            ))
                            .child(self.views[i + 1].clone()),
                    );
                }
                reviews
            }
            "data-grid" => root
                .child(self.views[0].clone())
                .when(!self.notice.is_empty(), |r| {
                    r.child(label(self.notice.clone(), 12., p.secondary))
                }),
            "json-viewer" => {
                let width = fit(w, 522.);
                root.child(
                    row()
                        .w(width)
                        .gap(px(8.))
                        .px(px(4.))
                        .child(label("GET", 12., p.ink).font_weight(FontWeight::SEMIBOLD))
                        .child(
                            label("/v1/orders/ord_8Kd2Q7", 12., p.secondary)
                                .font_family("Fira Code")
                                .truncate()
                                .flex_1(),
                        )
                        .child(div().size(px(5.)).rounded_full().bg(p.success))
                        .child(label("200", 12., p.ink))
                        .child(label("84 ms", 12., p.faint)),
                )
                .child(self.views[0].clone())
                .child(
                    label(
                        t(
                            "Try searching for refund, then use the arrow keys",
                            "试试搜索 refund，然后用方向键浏览",
                        ),
                        12.,
                        p.secondary,
                    )
                    .w(width)
                    .px(px(4.)),
                )
            }
            _ => root.child(self.views[0].clone()),
        }
    }
}

fn photos(l: UiLanguage) -> Vec<GalleryPhoto> {
    let records: &[(&[u8], &str, &str, &str, f32)] = &[
        (
            include_bytes!("../../assets/navigation/jsc2004e52772.jpg"),
            "Panorama I",
            "月面全景一",
            "NASA · Apollo 17 · jsc2004e52772",
            1.509,
        ),
        (
            include_bytes!("../../assets/navigation/S72-37257.jpg"),
            "Ejecta experiment",
            "溅射物实验",
            "NASA · Apollo 17 · S72-37257",
            0.801,
        ),
        (
            include_bytes!("../../assets/navigation/jsc2004e52775.jpg"),
            "Panorama II",
            "月面全景二",
            "NASA · Apollo 17 · jsc2004e52775",
            1.000,
        ),
        (
            include_bytes!("../../assets/navigation/jsc2004e52777.jpg"),
            "Panorama III",
            "月面全景三",
            "NASA · Apollo 17 · jsc2004e52777",
            1.374,
        ),
        (
            include_bytes!("../../assets/navigation/jsc2004e52776.jpg"),
            "Panorama IV",
            "月面全景四",
            "NASA · Apollo 17 · jsc2004e52776",
            0.668,
        ),
        (
            include_bytes!("../../assets/navigation/9312448.jpg"),
            "Lunar surface",
            "月球表面",
            "NASA · Apollo 17 · 9312448",
            0.668,
        ),
        (
            include_bytes!("../../assets/navigation/7035876.jpg"),
            "Sampling the boulder",
            "巨石采样",
            "NASA · Apollo 17 · 7035876",
            0.751,
        ),
        (
            include_bytes!("../../assets/navigation/7036672.jpg"),
            "Lunar traverse",
            "月面行走",
            "NASA · Apollo 17 · 7036672",
            1.509,
        ),
        (
            include_bytes!("../../assets/navigation/as17-134-20481.jpg"),
            "At the lunar module",
            "登月舱旁",
            "NASA · Apollo 17 · as17-134-20481",
            1.509,
        ),
        (
            include_bytes!("../../assets/navigation/as17-137-20989.jpg"),
            "Orange soil at Shorty crater",
            "短坑的橙色月壤",
            "NASA · Apollo 17 · as17-137-20989",
            0.668,
        ),
        (
            include_bytes!("../../assets/navigation/as17-147-22465.jpg"),
            "Command module over Taurus–Littrow",
            "金牛–利特罗上空的指令舱",
            "NASA · Apollo 17 · as17-147-22465",
            1.779,
        ),
        (
            include_bytes!("../../assets/navigation/as17-134-20476.jpg"),
            "Beside the rover",
            "月球车旁",
            "NASA · Apollo 17 · as17-134-20476",
            0.668,
        ),
    ];
    records
        .iter()
        .map(|(bytes, name, zh, credit, ratio)| {
            GalleryPhoto::new(
                Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec())),
                l.text(name, zh).to_string(),
                credit.to_string(),
                *ratio,
            )
        })
        .collect()
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
