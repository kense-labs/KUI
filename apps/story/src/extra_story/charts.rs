//! Interactive examples for compact numeric visualizations.
use gpui::{prelude::*, *};
use kui::{
    components::{
        activity_rings::{ActivityDay, ActivityGoal, ActivityRings},
        animated_counter::AnimatedCounter,
        brush_chart::{BrushChart, BrushPoint, BrushPreset},
        data_flow::{DataFlow, DataFlowNode, DataFlowSnapshot, DataFlowStatus},
        funnel_chart::{FunnelChart, FunnelSegment, FunnelStage},
        gauge::{Gauge, GaugeTone},
        metric_ticker::{MetricTicker, TickerDelta, TickerGood, TickerMetric},
        model_training::{ModelTraining, TrainingConfig},
        multi_region_failover::{
            FailoverPhase, FailoverRequest, FailoverSnapshot, MultiRegionFailover, RegionHealth,
        },
        race_bar_chart::{RaceBarChart, RaceEntry, RaceFrame},
        radar_chart::{RadarAxis, RadarChart, RadarProfile},
        realtime_stream::{RealtimeStream, StreamReading},
        ridgeline::{RidgeSeries, Ridgeline},
        sankey_flow::{FlowLink, FlowNode, FlowPeriodChange, SankeyFlow},
        segmented_control::{FormOption, SegmentedControl, SelectionChanged},
        slope_chart::{SlopeChart, SlopeSeries},
        split_bar::{SplitBar, SplitSegment},
        streamgraph::{StreamLayer, StreamPoint, Streamgraph},
        sunburst::{Sunburst, SunburstNode},
        treemap::{TreeTile, Treemap, TreemapColorScale},
        vector_search::{VectorDocument, VectorMatch, VectorSearch, VectorSearchSnapshot},
        waffle_chart::{WaffleCategory, WaffleChart},
    },
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
        presentation::{column, row},
    },
};
use std::{cell::Cell, rc::Rc};

pub const IDS: &[&str] = &[
    "gauge",
    "animated-counter",
    "waffle-chart",
    "slope-chart",
    "radar-chart",
    "split-bar",
    "race-bar-chart",
    "funnel-chart",
    "streamgraph",
    "treemap",
    "brush-chart",
    "ridgeline",
    "realtime-stream",
    "sunburst",
    "sankey-flow",
    "metric-ticker",
    "activity-rings",
    "model-training",
    "multi-region-failover",
    "data-flow",
    "vector-search",
];

pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Demo::new(id, w, cx)).into())
}

enum Content {
    Gauge(Vec<Entity<Gauge>>),
    Counter(Vec<Entity<AnimatedCounter>>),
    Waffle(Entity<WaffleChart>),
    Slope(Entity<SlopeChart>),
    Radar(Entity<RadarChart>),
    Split(Entity<SplitBar>),
    Race(Entity<RaceBarChart>),
    Funnel(Entity<FunnelChart>),
    Stream(Entity<Streamgraph>),
    Tree(Entity<Treemap>),
    Brush(Entity<BrushChart>),
    Ridge(Entity<Ridgeline>),
    Live(Entity<RealtimeStream>),
    Sunburst(Entity<Sunburst>),
    Sankey(Entity<SankeyFlow>),
    Ticker(Entity<MetricTicker>, Rc<Cell<bool>>),
    ActivityRings(Entity<ActivityRings>),
    Training(Entity<ModelTraining>),
    Failover(Entity<MultiRegionFailover>),
    DataFlow(Entity<DataFlow>, Rc<Cell<bool>>),
    VectorSearch(Entity<VectorSearch>),
}
struct Demo {
    content: Content,
    step: usize,
    language: UiLanguage,
    periods: Option<Entity<SegmentedControl>>,
    _period_subscription: Option<Subscription>,
}
impl Demo {
    fn new(id: &str, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en, zh| language.text(en, zh);
        let content = match id {
            "gauge" => {
                let mut meters = vec![];
                for (label, detail, value, tone) in [
                    (
                        t("Storage", "存储空间"),
                        t("Used capacity", "已使用的容量"),
                        68.,
                        GaugeTone::Accent,
                    ),
                    (
                        t("Availability", "可用性"),
                        t("Last 30 days", "过去 30 天"),
                        96.,
                        GaugeTone::Success,
                    ),
                    (
                        t("Processor", "处理器"),
                        t("Current load", "当前负载"),
                        42.,
                        GaugeTone::Warning,
                    ),
                ] {
                    meters.push(
                        cx.new(|_| Gauge::new(label, value).with_detail(detail).with_tone(tone)),
                    );
                }
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                for meter in &meters {
                    meter.update(cx, |meter, cx| meter.set_visible(visible, cx));
                }
                let weak = meters
                    .iter()
                    .map(|meter| meter.downgrade())
                    .collect::<Vec<_>>();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        let mut alive = false;
                        for meter in &weak {
                            alive |= meter
                                .update(cx, |meter, cx| {
                                    let visible = cx
                                        .try_global::<crate::StoryBridge>()
                                        .is_none_or(|bridge| (bridge.visible)());
                                    meter.set_visible(visible, cx);
                                })
                                .is_ok();
                        }
                        if !alive {
                            break;
                        }
                    }
                })
                .detach();
                Content::Gauge(meters)
            }
            "waffle-chart" => {
                let chart = cx.new(|_| {
                    WaffleChart::new(waffle_label(0), waffle_data(0, language))
                        .with_unit(t("TWh", "太瓦时"))
                });
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Waffle(chart)
            }
            "brush-chart" => Content::Brush(cx.new(|_| {
                BrushChart::new(t("Daily activity", "每日活动"), brush_data(0, language))
                    .with_presets(vec![
                        BrushPreset::new(t("30 days", "30天"), Some(30)),
                        BrushPreset::new(t("60 days", "60天"), Some(60)),
                        BrushPreset::new(t("All", "全部"), None),
                    ])
            })),
            "ridgeline" => {
                let chart = cx.new(|_| {
                    Ridgeline::new(t("Daily highs", "每日最高"), ridgeline_data(0, language))
                });
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Ridge(chart)
            }
            "sunburst" => {
                let chart = cx.new(|_| {
                    Sunburst::new(t("Cloud spend", "云服务开销"), sunburst_data(0, language))
                });
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Sunburst(chart)
            }
            "activity-rings" => Content::ActivityRings(cx.new(|cx| {
                ActivityRings::new(
                    t("Activity rings", "活动圆环"),
                    activity_goals(language),
                    activity_days(language),
                    cx,
                )
            })),
            "vector-search" => {
                let chart = cx.new(|_| VectorSearch::new(vector_search_data(language)));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::VectorSearch(chart)
            }
            "data-flow" => {
                let sources = vec![
                    DataFlowNode::new("app", t("App", "应用"))
                        .with_detail(t("Application events", "应用事件")),
                    DataFlowNode::new("db", t("Database", "数据库"))
                        .with_detail(t("Database changes", "数据库变更")),
                    DataFlowNode::new("events", t("Events", "事件"))
                        .with_detail(t("Incoming events", "外部事件")),
                ];
                let destinations = vec![
                    DataFlowNode::new("store", t("Warehouse", "数据仓库")),
                    DataFlowNode::new("dash", t("Dashboard", "仪表盘")),
                    DataFlowNode::new("alerts", t("Alerts", "告警")).with_share(0.34),
                ];
                let chart = cx.new(|_| {
                    DataFlow::new(
                        sources,
                        DataFlowNode::new("core", t("Transform", "处理")),
                        destinations,
                        DataFlowSnapshot::new(12_480, DataFlowStatus::Running),
                    )
                    .with_node_icon("app", kui::primitives::glyph::Glyph::Monitor)
                    .with_node_icon("db", kui::primitives::glyph::Glyph::Archive)
                    .with_node_icon("events", kui::primitives::glyph::Glyph::Send)
                    .with_node_icon("core", kui::primitives::glyph::Glyph::Settings)
                    .with_node_icon("store", kui::primitives::glyph::Glyph::Folder)
                    .with_node_icon("dash", kui::primitives::glyph::Glyph::FolderKanban)
                    .with_node_icon("alerts", kui::primitives::glyph::Glyph::Bell)
                });
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let failed = Rc::new(Cell::new(false));
                let error_task = failed.clone();
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    let mut tick = 0usize;
                    let mut count = 12_480u64;
                    let mut last_error = false;
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                                if !visible {
                                    return;
                                }
                                let error = error_task.get();
                                tick += 1;
                                if tick.is_multiple_of(6) && !error {
                                    count += 1;
                                }
                                if error != last_error || (tick.is_multiple_of(6) && !error) {
                                    let state = if error {
                                        DataFlowSnapshot::new(count, DataFlowStatus::Error)
                                            .with_message(
                                                language
                                                    .text("Simulated source error", "模拟来源异常"),
                                            )
                                    } else {
                                        DataFlowSnapshot::new(count, DataFlowStatus::Running)
                                    };
                                    chart.set_snapshot(state, cx);
                                    last_error = error;
                                }
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::DataFlow(chart, failed)
            }
            "multi-region-failover" => {
                let chart = cx.new(|_| {
                    MultiRegionFailover::new(failover_data(FailoverPhase::Balanced, language))
                });
                let failed_once = Rc::new(Cell::new(false));
                cx.subscribe(&chart, move |_, chart, request: &FailoverRequest, cx| {
                    let should_fail =
                        request.phase == FailoverPhase::Shifting && !failed_once.replace(true);
                    let weak = chart.downgrade();
                    let id = request.id;
                    let phase = request.phase;
                    cx.spawn(async move |_, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(450))
                            .await;
                        let _ = weak.update(cx, |chart, cx| {
                            let result = if should_fail {
                                Err(language
                                    .text(
                                        "Simulated failure — retry to continue",
                                        "模拟失败，可重试继续",
                                    )
                                    .into())
                            } else {
                                Ok(failover_data(phase, language))
                            };
                            chart.resolve(id, result, cx);
                        });
                    })
                    .detach();
                })
                .detach();
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Failover(chart)
            }
            "model-training" => {
                let config = TrainingConfig {
                    classes: [t("Positive", "正向").into(), t("Negative", "负向").into()],
                    ..TrainingConfig::default()
                };
                let chart = cx.new(|_| ModelTraining::new(config));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Training(chart)
            }
            "metric-ticker" => {
                let chart = cx.new(|cx| {
                    MetricTicker::new(
                        t("Product health", "产品概况"),
                        ticker_data(0, language),
                        cx,
                    )
                });
                let paused = Rc::new(Cell::new(false));
                let paused_task = paused.clone();
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    let mut ticks = 0usize;
                    let mut frame = 0usize;
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        let result = weak.update(cx, |chart, cx| {
                            let visible = cx
                                .try_global::<crate::StoryBridge>()
                                .is_none_or(|bridge| (bridge.visible)());
                            chart.set_visible(visible, cx);
                            if visible && !paused_task.get() {
                                ticks += 1;
                                if ticks.is_multiple_of(6) {
                                    frame += 1;
                                    chart.set_metrics(ticker_data(frame, language), cx);
                                }
                            }
                        });
                        if result.is_err() {
                            break;
                        }
                    }
                })
                .detach();
                Content::Ticker(chart, paused)
            }
            "sankey-flow" => {
                let (nodes, links) = sankey_data(1, language);
                let chart = cx.new(|_| {
                    SankeyFlow::new(t("Signup journey", "注册旅程"), nodes, links)
                        .with_unit(t(" people", "人"))
                        .with_total_label(t("signups", "注册"))
                        .with_columns(vec![
                            t("Source", "来源").into(),
                            t("Activation", "激活").into(),
                            t("Plan", "方案").into(),
                            t("90 days", "90 天").into(),
                        ])
                        .with_periods(vec!["Q2".into(), "Q3".into()], 1)
                        .with_period_details(vec![
                            t("Apr to Jun 2026", "2026 年 4 至 6 月").into(),
                            t("Jul to Sep 2026", "2026 年 7 至 9 月").into(),
                        ])
                });
                cx.subscribe(&chart, move |s, chart, period: &FlowPeriodChange, cx| {
                    let (nodes, links) = sankey_data(period.index, language);
                    chart.update(cx, |chart, cx| chart.set_data(nodes, links, cx));
                    s.step = period.index;
                })
                .detach();
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Sankey(chart)
            }
            "realtime-stream" => {
                let chart = cx.new(|_| {
                    RealtimeStream::new(t("Live activity", "实时活动"))
                        .with_capacity(60)
                        .with_unit(" req/s")
                });
                chart.update(cx, |chart, cx| {
                    chart.set_visible(false, cx);
                    for tick in 0..60 {
                        let value =
                            60. + (tick as f32 * 0.23).sin() * 8. + (tick as f32 * 0.7).cos() * 3.;
                        chart.push(StreamReading::new(format!("{tick}"), value), cx);
                    }
                });
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    let mut tick = 60usize;
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(240))
                            .await;
                        let result = weak.update(cx, |chart, cx| {
                            let visible = cx
                                .try_global::<crate::StoryBridge>()
                                .is_none_or(|bridge| (bridge.visible)());
                            chart.set_visible(visible, cx);
                            if visible {
                                let value = 60.
                                    + (tick as f32 * 0.23).sin() * 8.
                                    + (tick as f32 * 0.7).cos() * 3.;
                                let value = if tick.is_multiple_of(37) {
                                    value * 1.9
                                } else {
                                    value
                                };
                                let reading = StreamReading::new(format!("{tick}"), value);
                                let reading = if tick.is_multiple_of(37) {
                                    reading.with_anomaly("Spike")
                                } else {
                                    reading
                                };
                                chart.push(reading, cx);
                                tick += 1;
                            }
                        });
                        if result.is_err() {
                            break;
                        }
                    }
                })
                .detach();
                Content::Live(chart)
            }
            "treemap" => {
                let chart = cx.new(|_| {
                    Treemap::new(
                        t("Business by region", "各地区业务"),
                        treemap_data(0, language),
                    )
                    .with_metric_labels(vec![
                        t("Revenue", "收入").into(),
                        t("Customers", "客户数").into(),
                    ])
                    .with_shade_labels(vec![
                        t("Growth", "增长").into(),
                        t("Retention", "留存").into(),
                    ])
                    .with_color_scales(vec![
                        TreemapColorScale::new(0., 20., rgb(0x394b64).into(), rgb(0x70a3ee).into())
                            .with_suffix("%")
                            .with_branch_hues(true),
                        TreemapColorScale::new(
                            70.,
                            100.,
                            rgb(0x3e594c).into(),
                            rgb(0x6ed19b).into(),
                        )
                        .with_suffix("%")
                        .with_branch_hues(true),
                    ])
                });
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Tree(chart)
            }
            "streamgraph" => {
                let chart = cx.new(|_| {
                    Streamgraph::new(
                        t("Support tickets by topic", "各主题工单量"),
                        stream_layers(language),
                        stream_data(0, language),
                    )
                    .with_unit(t("tickets", "单"))
                });
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Stream(chart)
            }
            "funnel-chart" => {
                let chart = cx.new(|cx| {
                    FunnelChart::new(
                        t("Checkout funnel", "结账转化"),
                        funnel_data(0, language),
                        vec![
                            FunnelSegment::new("mobile", t("Mobile", "移动端")),
                            FunnelSegment::new("desktop", t("Desktop", "桌面端")),
                        ],
                        cx,
                    )
                    .with_period(t("Last 30 days", "近 30 天"))
                    .with_unit(t("sessions", "次会话"))
                });
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Funnel(chart)
            }
            "split-bar" => Content::Split(cx.new(|_| {
                SplitBar::new(t("Traffic by source", "流量来源"), split_data(0, language))
                    .with_description(t("Sessions on northwind.shop", "northwind.shop 的访问会话"))
                    .with_unit(t("sessions", "次会话"))
            })),
            "race-bar-chart" => {
                let chart = cx.new(|_| {
                    RaceBarChart::new(t("Revenue by product", "产品收入"), race_items(language))
                        .with_meta(t("Year to date, 2025", "2025 年至今"))
                        .with_groups(vec![
                            ("furniture".into(), t("Furniture", "家具").into()),
                            ("lighting".into(), t("Lighting", "灯具").into()),
                            ("textiles".into(), t("Textiles", "纺织品").into()),
                            ("tableware".into(), t("Tableware", "餐具").into()),
                        ])
                        .with_value_prefix("$")
                        .with_highlight("chair")
                        .with_frames(race_frames(language))
                });
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Race(chart)
            }
            "radar-chart" => {
                let chart = cx.new(|_| {
                    RadarChart::new(
                        t("Lisbon compared with", "里斯本对比"),
                        radar_axes(language),
                        radar_data(0, language),
                    )
                    .with_meta(t(
                        "Second office shortlist, scored out of 100",
                        "第二办公室候选名单,满分 100 分",
                    ))
                });
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Radar(chart)
            }
            "slope-chart" => {
                let chart = cx.new(|_| {
                    SlopeChart::new(
                        t("Conversion by channel", "各渠道转化率"),
                        slope_data(0, language),
                    )
                    .with_meta(t("Q1 to Q2, 2025", "2025 年第一至第二季度"))
                    .with_start_label(t("Q1", "一季度"))
                    .with_end_label(t("Q2", "二季度"))
                    .with_value_suffix("%")
                    .with_highlight("email")
                });
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
                let weak = chart.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(std::time::Duration::from_millis(250))
                            .await;
                        if weak
                            .update(cx, |chart, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                chart.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Content::Slope(chart)
            }
            _ => {
                let data = [
                    (t("Total requests", "请求总数"), 12480., "", "", 0),
                    (t("Response rate", "响应率"), 97.5, "", "%", 1),
                    (t("Revenue", "收入"), 12540.25, "$", "", 2),
                ];
                let counters = data
                    .into_iter()
                    .map(|(label, value, prefix, suffix, decimals)| {
                        cx.new(|_| {
                            AnimatedCounter::new(value)
                                .with_label(label)
                                .with_prefix(prefix)
                                .with_suffix(suffix)
                                .with_decimals(decimals)
                                .with_animate_on_view(true)
                        })
                    })
                    .collect();
                Content::Counter(counters)
            }
        };
        let period_options = match &content {
            Content::Split(_) => vec![
                FormOption::new("0", t("7 days", "7 天")),
                FormOption::new("1", t("30 days", "30 天")),
                FormOption::new("2", t("90 days", "90 天")),
            ],
            // The reference demo compares countries in one grid.
            Content::Waffle(_) => vec![
                FormOption::new("0", t("Germany", "德国")),
                FormOption::new("1", t("France", "法国")),
                FormOption::new("2", t("United Kingdom", "英国")),
                FormOption::new("3", t("United States", "美国")),
            ],
            // The reference demo compares Lisbon with one rival at a time.
            Content::Radar(_) => vec![
                FormOption::new("0", t("Berlin", "柏林")),
                FormOption::new("1", t("Toronto", "多伦多")),
                FormOption::new("2", t("Austin", "奥斯汀")),
            ],
            _ => vec![],
        };
        let step_limit = match &content {
            Content::Waffle(_) => 4,
            _ => 3,
        };
        let periods = (!period_options.is_empty()).then(|| {
            let label = if matches!(content, Content::Radar(_)) {
                t("Compare with", "对比城市")
            } else {
                ""
            };
            cx.new(|cx| SegmentedControl::new(label, period_options, w, cx).with_selected(&["0"]))
        });
        if let (Content::Radar(chart), Some(periods)) = (&content, &periods) {
            chart.update(cx, |chart, cx| {
                chart.set_header_extra(Some(periods.clone().into()), cx)
            });
        }
        let subscription = periods.as_ref().map(|periods| {
            cx.subscribe(periods, move |s, _, event: &SelectionChanged, cx| {
                if let Some(step) = event
                    .0
                    .first()
                    .and_then(|key| key.parse::<usize>().ok())
                    .filter(|step| *step < step_limit)
                {
                    s.choose_step(step, cx);
                }
            })
        });
        Self {
            content,
            step: 0,
            language,
            periods,
            _period_subscription: subscription,
        }
    }
    fn change(&mut self, cx: &mut Context<Self>) {
        self.choose_step((self.step + 1) % 3, cx);
    }
    fn choose_step(&mut self, step: usize, cx: &mut Context<Self>) {
        self.step = step;
        match &self.content {
            Content::Gauge(meters) => {
                let datasets = [[68., 96., 42.], [37., 81., 74.], [91., 99., 23.]];
                for (meter, value) in meters.iter().zip(datasets[self.step]) {
                    meter.update(cx, |m, cx| m.set_value(value, cx));
                }
            }
            Content::Counter(counters) => {
                let datasets = [
                    [12480., 97.5, 12540.25],
                    [18216., 99.2, 18375.50],
                    [9845., 94.7, 9760.80],
                ];
                for (counter, value) in counters.iter().zip(datasets[self.step]) {
                    counter.update(cx, |m, cx| m.set_value(value, cx));
                }
            }
            Content::Waffle(chart) => {
                let values = waffle_data(self.step, self.language);
                chart.update(cx, |chart, cx| {
                    chart.set_categories(values, cx);
                    chart.set_label(waffle_label(self.step), cx);
                });
            }
            Content::Brush(chart) => {
                let values = brush_data(self.step, self.language);
                chart.update(cx, |chart, cx| chart.set_points(values, cx));
            }
            Content::Ridge(chart) => {
                let values = ridgeline_data(self.step, self.language);
                chart.update(cx, |chart, cx| chart.set_series(values, cx));
            }
            Content::Tree(chart) => {
                let values = treemap_data(self.step, self.language);
                chart.update(cx, |chart, cx| chart.set_tiles(values, cx));
            }
            Content::Stream(chart) => {
                let values = stream_data(self.step, self.language);
                chart.update(cx, |chart, cx| chart.set_points(values, cx));
            }
            Content::Funnel(chart) => {
                let values = funnel_data(self.step, self.language);
                chart.update(cx, |chart, cx| chart.set_stages(values, cx));
            }
            Content::Split(chart) => {
                let values = split_data(self.step, self.language);
                chart.update(cx, |chart, cx| chart.set_segments(values, cx));
            }
            Content::Race(chart) => {
                chart.update(cx, |chart, cx| chart.set_frame(self.step, cx));
            }
            Content::Radar(chart) => {
                let values = radar_data(self.step, self.language);
                chart.update(cx, |chart, cx| chart.set_profiles(values, cx));
            }
            Content::Slope(chart) => {
                let values = slope_data(self.step, self.language);
                chart.update(cx, |chart, cx| chart.set_series(values, cx));
            }
            Content::Live(_) => {}
            Content::Sunburst(chart) => {
                let values = sunburst_data(self.step, self.language);
                chart.update(cx, |chart, cx| chart.set_root(values, cx));
            }
            Content::Sankey(_) => {}
            Content::Ticker(_, _) => {}
            Content::ActivityRings(_) => {}
            Content::Training(_) => {}
            Content::Failover(_) => {}
            Content::DataFlow(_, _) => {}
            Content::VectorSearch(_) => {}
        }
        cx.notify();
    }
}
impl Render for Demo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let visible = cx
            .try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)());
        match &self.content {
            Content::Counter(counters) => {
                for counter in counters {
                    counter.update(cx, |counter, cx| counter.set_visible(visible, cx));
                }
            }
            Content::Split(chart) => {
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
            }
            Content::Funnel(chart) => {
                chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
            }
            _ => {}
        }
        if let Content::Split(chart) = &self.content {
            return div()
                .w_full()
                .max_w(px(720.))
                .min_h(px(380.))
                .p(px(24.))
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(24.))
                .when_some(self.periods.clone(), |view, periods| view.child(periods))
                .child(chart.clone());
        }
        if let Content::Waffle(chart) = &self.content {
            chart.update(cx, |chart, cx| chart.set_visible(visible, cx));
            let (country, total) = waffle_country(self.step);
            return div()
                .w_full()
                .max_w(px(640.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .rounded(kui::foundation::surface_radius(cx))
                .p(px(24.))
                .flex()
                .flex_col()
                .gap(px(24.))
                .child(
                    row()
                        .flex_wrap()
                        .items_start()
                        .justify_between()
                        .gap(px(12.))
                        .child(
                            column()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(16.))
                                        .line_height(px(22.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(
                                            self.language.text(
                                                "Electricity generation, 2023",
                                                "2023 年发电量",
                                            ),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .line_height(px(20.))
                                        .text_color(p.faint)
                                        .child(format!(
                                            "{}, {} {}",
                                            country,
                                            total,
                                            self.language.text("TWh", "太瓦时")
                                        )),
                                ),
                        )
                        .when_some(self.periods.clone(), |view, periods| view.child(periods)),
                )
                .child(chart.clone());
        }
        let label = self.language.text("Change values", "切换数值");
        let values = div()
            .flex()
            .flex_wrap()
            .justify_center()
            .items_center()
            .gap(px(28.))
            .w_full()
            .max_w_full()
            .children(match &self.content {
                Content::Gauge(v) => v.iter().cloned().map(Into::into).collect::<Vec<AnyView>>(),
                Content::Counter(v) => v.iter().cloned().map(Into::into).collect::<Vec<AnyView>>(),
                Content::Waffle(v) => vec![v.clone().into()],
                Content::Slope(v) => vec![v.clone().into()],
                Content::Radar(v) => vec![v.clone().into()],
                Content::Split(v) => vec![v.clone().into()],
                Content::Race(v) => vec![v.clone().into()],
                Content::Funnel(v) => vec![v.clone().into()],
                Content::Stream(v) => vec![v.clone().into()],
                Content::Tree(v) => vec![v.clone().into()],
                Content::Brush(v) => vec![v.clone().into()],
                Content::Ridge(v) => vec![v.clone().into()],
                Content::Live(v) => vec![v.clone().into()],
                Content::Sunburst(v) => vec![v.clone().into()],
                Content::Sankey(v) => vec![v.clone().into()],
                Content::Ticker(v, _) => vec![v.clone().into()],
                Content::ActivityRings(v) => vec![v.clone().into()],
                Content::Training(v) => vec![v.clone().into()],
                Content::Failover(v) => vec![v.clone().into()],
                Content::DataFlow(v, _) => vec![v.clone().into()],
                Content::VectorSearch(v) => vec![v.clone().into()],
            });
        let controls = if matches!(self.content, Content::Sunburst(_)) {
            let options = [
                self.language.text("August", "八月"),
                self.language.text("September", "九月"),
            ];
            div()
                .flex()
                .gap_2()
                .children(options.into_iter().enumerate().map(|(i, text)| {
                    face(
                        ("sunburst-period", i),
                        text,
                        36.,
                        kui::foundation::surface_radius(cx).into(),
                        if self.step == i {
                            Tone::Primary
                        } else {
                            Tone::Surface
                        },
                        cx,
                    )
                    .px(px(12.))
                    .child(text)
                    .on_click(cx.listener(move |s, _, _, cx| s.choose_step(i, cx)))
                }))
                .into_any_element()
        } else if matches!(self.content, Content::Ridge(_)) {
            let options = [
                self.language.text("Zurich", "苏黎世"),
                self.language.text("Lisbon", "里斯本"),
                self.language.text("Denver", "丹佛"),
            ];
            div()
                .flex()
                .gap_2()
                .children(options.into_iter().enumerate().map(|(i, text)| {
                    face(
                        ("radar-comparison", i),
                        text,
                        36.,
                        kui::foundation::surface_radius(cx).into(),
                        if self.step == i {
                            Tone::Primary
                        } else {
                            Tone::Surface
                        },
                        cx,
                    )
                    .px(px(12.))
                    .child(text)
                    .on_click(cx.listener(move |s, _, _, cx| s.choose_step(i, cx)))
                }))
                .into_any_element()
        } else {
            face(
                "change-chart-values",
                label,
                36.,
                kui::foundation::surface_radius(cx).into(),
                Tone::Surface,
                cx,
            )
            .px(px(12.))
            .child(label)
            .on_click(cx.listener(|s, _, _, cx| s.change(cx)))
            .into_any_element()
        };
        let ticker_header = if let Content::Ticker(_, paused) = &self.content {
            let label = if paused.get() {
                self.language.text("Resume stream", "继续数据")
            } else {
                self.language.text("Pause stream", "暂停数据")
            };
            Some(
                div()
                    .w_full()
                    .max_w(px(760.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(self.language.text("Product health", "产品概况"))
                    .child(
                        face(
                            "ticker-pause",
                            label,
                            34.,
                            kui::foundation::surface_radius(cx).into(),
                            Tone::Surface,
                            cx,
                        )
                        .px(px(12.))
                        .child(label)
                        .on_click(cx.listener(|s, _, _, cx| {
                            if let Content::Ticker(_, paused) = &s.content {
                                paused.set(!paused.get());
                                cx.notify();
                            }
                        })),
                    ),
            )
        } else {
            None
        };
        let failover_demo = if matches!(self.content, Content::Failover(_)) {
            Some(self.language.text(
                "Simulated data; the first failover request is designed to fail.",
                "演示数据；首次故障切换会模拟失败。",
            ))
        } else {
            None
        };
        let flow_demo = if let Content::DataFlow(_, failed) = &self.content {
            let label = if failed.get() {
                self.language.text("Recover sample source", "恢复示例来源")
            } else {
                self.language.text("Simulate source error", "模拟来源异常")
            };
            Some(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(self.language.text("Sample data", "演示数据"))
                    .child(
                        face(
                            "flow-simulate",
                            label,
                            34.,
                            kui::foundation::surface_radius(cx).into(),
                            Tone::Surface,
                            cx,
                        )
                        .px(px(10.))
                        .child(label)
                        .on_click(cx.listener(|s, _, _, cx| {
                            if let Content::DataFlow(_, failed) = &s.content {
                                failed.set(!failed.get());
                                cx.notify();
                            }
                        })),
                    ),
            )
        } else {
            None
        };
        div()
            .size_full()
            .min_h(px(520.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(34.))
            .p(px(20.))
            .when(
                matches!(self.content, Content::Sankey(_)) && w.viewport_size().width < px(420.),
                |view| {
                    let width = f32::from(w.viewport_size().width);
                    view.px(px(((width - 344.) * 0.5).clamp(8., 23.)))
                },
            )
            .bg(p.stage)
            .text_color(p.ink)
            .when_some(ticker_header, |view, header| view.child(header))
            .when_some(failover_demo, |view, text| {
                view.child(div().text_color(p.secondary).child(text))
            })
            .when_some(flow_demo, |view, controls| view.child(controls))
            .child(values)
            .when(
                !matches!(
                    self.content,
                    Content::Live(_)
                        | Content::Sankey(_)
                        | Content::Ticker(_, _)
                        | Content::ActivityRings(_)
                        | Content::Training(_)
                        | Content::Failover(_)
                        | Content::DataFlow(_, _)
                        | Content::VectorSearch(_)
                        | Content::Radar(_)
                ),
                |view| view.child(controls),
            )
    }
}

fn vector_search_data(language: UiLanguage) -> VectorSearchSnapshot {
    let t = |en, zh| language.text(en, zh);
    let data = [
        (
            "bill-1",
            t("Refund a charge", "退款"),
            t("Billing", "账单"),
            -0.75,
            -0.45,
            0xe0a46b,
        ),
        (
            "bill-2",
            t("Update a card", "更新卡片"),
            t("Billing", "账单"),
            -0.48,
            -0.62,
            0xe0a46b,
        ),
        (
            "bill-3",
            t("Download invoices", "下载发票"),
            t("Billing", "账单"),
            -0.55,
            -0.25,
            0xe0a46b,
        ),
        (
            "access-1",
            t("Rotate an API key", "轮换 API 密钥"),
            t("Access", "访问"),
            0.05,
            0.10,
            0x9b84d6,
        ),
        (
            "access-2",
            t("Scope API tokens", "设置令牌范围"),
            t("Access", "访问"),
            0.30,
            0.18,
            0x9b84d6,
        ),
        (
            "access-3",
            t("Revoke a session", "撤销会话"),
            t("Access", "访问"),
            0.12,
            0.42,
            0x9b84d6,
        ),
        (
            "deploy-1",
            t("Roll back a deploy", "回滚部署"),
            t("Deploys", "部署"),
            0.58,
            -0.64,
            0x62b5d0,
        ),
        (
            "deploy-2",
            t("Preview environments", "预览环境"),
            t("Deploys", "部署"),
            0.80,
            -0.34,
            0x62b5d0,
        ),
        (
            "deploy-3",
            t("Add a domain", "添加域名"),
            t("Deploys", "部署"),
            0.46,
            -0.28,
            0x62b5d0,
        ),
    ];
    let documents = data
        .into_iter()
        .map(|(id, title, topic, x, y, color)| {
            VectorDocument::new(id, title, topic, x, y, rgb(color).into())
                .with_source(t("Sample docs", "示例文档"))
        })
        .collect();
    let matches = vec![
        VectorMatch::new("access-1", 0.90),
        VectorMatch::new("access-2", 0.89),
        VectorMatch::new("access-3", 0.82),
    ];
    let mut snapshot = VectorSearchSnapshot::new(
        t("How do I rotate an API key?", "如何轮换 API 密钥？"),
        documents,
        matches,
    );
    snapshot.query_x = 0.0;
    snapshot.query_y = 0.3;
    snapshot.links = vec![
        ("bill-1".into(), "bill-2".into()),
        ("bill-2".into(), "bill-3".into()),
        ("access-1".into(), "access-2".into()),
        ("access-2".into(), "access-3".into()),
        ("deploy-1".into(), "deploy-2".into()),
        ("deploy-2".into(), "deploy-3".into()),
    ];
    snapshot.message = t(
        "Example values; rankings and scores are supplied by the host.",
        "示例数据；排序和分数由宿主提供。",
    )
    .into();
    snapshot
}

fn failover_data(phase: FailoverPhase, language: UiLanguage) -> FailoverSnapshot {
    let t = |en, zh| language.text(en, zh);
    let mut snapshot =
        FailoverSnapshot::new(t("East region", "东部区域"), t("West region", "西部区域"));
    snapshot.requests_per_second = 12_000.;
    snapshot.phase = phase;
    match phase {
        FailoverPhase::Balanced => {}
        FailoverPhase::Degraded => {
            snapshot.primary_health = RegionHealth::Degraded;
            snapshot.failed_checks = 1;
        }
        FailoverPhase::Shifting => {
            snapshot.primary_health = RegionHealth::Unhealthy;
            snapshot.failed_checks = 3;
            snapshot.primary_share = 0.15;
        }
        FailoverPhase::Secondary => {
            snapshot.primary_health = RegionHealth::Unhealthy;
            snapshot.failed_checks = 3;
            snapshot.primary_share = 0.;
        }
        FailoverPhase::Recovering => {
            snapshot.primary_health = RegionHealth::Recovering;
            snapshot.passed_checks = 2;
            snapshot.primary_share = 0.;
        }
        FailoverPhase::Rebalancing => {
            snapshot.primary_health = RegionHealth::Recovering;
            snapshot.passed_checks = 3;
            snapshot.primary_share = 0.38;
        }
    }
    snapshot
}

fn activity_goals(language: UiLanguage) -> Vec<ActivityGoal> {
    let t = |en, zh| language.text(en, zh);
    vec![
        ActivityGoal::new(
            "deep",
            t("Deep work", "深度工作"),
            180.,
            t("min", "分钟"),
            rgb(0x6572e4).into(),
        ),
        ActivityGoal::new(
            "writing",
            t("Writing", "写作"),
            1200.,
            t("words", "字"),
            rgb(0xeb992e).into(),
        ),
        ActivityGoal::new(
            "reading",
            t("Reading", "阅读"),
            40.,
            t("pages", "页"),
            rgb(0x5da37f).into(),
        ),
    ]
}
fn activity_days(language: UiLanguage) -> Vec<ActivityDay> {
    let t = |en, zh| language.text(en, zh);
    [
        ("mon", t("Mon", "周一"), [180., 1200., 32.]),
        ("tue", t("Tue", "周二"), [120., 860., 40.]),
        ("wed", t("Wed", "周三"), [180., 1200., 18.]),
        ("thu", t("Thu", "周四"), [165., 1190., 40.]),
        ("fri", t("Fri", "周五"), [95., 540., 26.]),
        ("sat", t("Sat", "周六"), [60., 300., 40.]),
        ("sun", t("Sun", "周日"), [130., 940., 22.]),
    ]
    .into_iter()
    .map(|(id, label, values)| {
        ActivityDay::new(
            id,
            label,
            vec![
                ("deep".into(), values[0]),
                ("writing".into(), values[1]),
                ("reading".into(), values[2]),
            ],
        )
    })
    .collect()
}

fn ticker_data(step: usize, language: UiLanguage) -> Vec<TickerMetric> {
    let t = |en, zh| language.text(en, zh);
    let wave = (step as f64 * 0.71).sin();
    vec![
        TickerMetric::new(
            "revenue",
            t("Monthly revenue", "月收入"),
            124_860. + wave * 1250.,
        )
        .with_format("$", "", 0)
        .with_caption(t("Last 30 days", "最近 30 天")),
        TickerMetric::new("active", t("Active now", "当前活跃"), 4268. + wave * 180.)
            .with_caption(t("Active sessions", "活跃会话")),
        TickerMetric::new("conversion", t("Conversion", "转化率"), 6.4 + wave * 0.32)
            .with_format("", "%", 1)
            .with_change(TickerDelta::Absolute, TickerGood::Up)
            .with_caption(t("Recent trials", "近期试用")),
        TickerMetric::new("latency", t("API latency", "接口延迟"), 182. + wave * 12.)
            .with_format("", " ms", 0)
            .with_change(TickerDelta::Absolute, TickerGood::Down)
            .with_caption(t("95th percentile", "第 95 百分位")),
        TickerMetric::new("error", t("Error rate", "错误率"), 0.6 + wave * 0.08)
            .with_format("", "%", 2)
            .with_change(TickerDelta::Absolute, TickerGood::Down)
            .with_caption(t("Failed requests", "失败的请求")),
    ]
}

fn sankey_data(step: usize, language: UiLanguage) -> (Vec<FlowNode>, Vec<FlowLink>) {
    let t = |en, zh| language.text(en, zh);
    // Mirrors the reference demo: sources wear the series palette, later
    // columns render neutral, values close end to end (11,200 / 9,600).
    let nodes = [
        ("organic", t("Organic", "自然流量"), 0, 0x55adff),
        ("paid", t("Paid", "付费"), 0, 0x9c37be),
        ("referral", t("Referral", "引荐"), 0, 0xe76e00),
        ("active", t("Activated", "已激活"), 1, 0xa8acb4),
        ("inactive", t("Not activated", "未激活"), 1, 0x858b95),
        ("free", t("Free", "免费"), 2, 0xa8acb4),
        ("pro", t("Pro", "专业版"), 2, 0xa8acb4),
        ("team", t("Team", "团队版"), 2, 0xa8acb4),
        ("retained", t("Retained", "留存"), 3, 0xa8acb4),
        ("churned", t("Churned", "流失"), 3, 0x858b95),
    ]
    .into_iter()
    .map(|(id, label, col, color)| FlowNode::new(id, label, rgb(color).into()).with_column(col))
    .collect();
    let q3 = step != 0;
    let links: [(&str, &str, f32); 15] = if q3 {
        [
            ("organic", "active", 3380.),
            ("organic", "inactive", 1820.),
            ("paid", "active", 1990.),
            ("paid", "inactive", 1910.),
            ("referral", "active", 1640.),
            ("referral", "inactive", 460.),
            ("active", "free", 3860.),
            ("active", "pro", 2310.),
            ("active", "team", 840.),
            ("free", "retained", 2120.),
            ("free", "churned", 1740.),
            ("pro", "retained", 1960.),
            ("pro", "churned", 350.),
            ("team", "retained", 790.),
            ("team", "churned", 50.),
        ]
    } else {
        [
            ("organic", "active", 2860.),
            ("organic", "inactive", 1740.),
            ("paid", "active", 1420.),
            ("paid", "inactive", 2080.),
            ("referral", "active", 1120.),
            ("referral", "inactive", 380.),
            ("active", "free", 3310.),
            ("active", "pro", 1560.),
            ("active", "team", 530.),
            ("free", "retained", 1690.),
            ("free", "churned", 1620.),
            ("pro", "retained", 1240.),
            ("pro", "churned", 320.),
            ("team", "retained", 480.),
            ("team", "churned", 50.),
        ]
    };
    (
        nodes,
        links
            .into_iter()
            .map(|(source, target, value)| FlowLink::new(source, target, value))
            .collect(),
    )
}

fn sunburst_data(step: usize, language: UiLanguage) -> SunburstNode {
    let t = |en, zh| language.text(en, zh);
    let mut groups = vec![];
    for (i, (id, en, zh, color)) in [
        ("platform", "Platform", "平台", 0x997fdb),
        ("data", "Data", "数据", 0x68a5d6),
        ("research", "Research", "研究", 0x65be9b),
        ("product", "Product", "产品", 0xd6aa70),
    ]
    .into_iter()
    .enumerate()
    {
        let mut branches = vec![];
        for (j, (sub, en, zh)) in [
            ("compute", "Compute", "计算"),
            ("storage", "Storage", "存储"),
            ("network", "Network", "网络"),
        ]
        .into_iter()
        .enumerate()
        {
            let base = (i + 2) as f32
                * (4 - j) as f32
                * if step == 0 { 1. } else { 1.1 + i as f32 * 0.08 };
            let leaves = [
                SunburstNode::new(
                    format!("{id}/{sub}/primary"),
                    t("Primary", "主要"),
                    base * 0.65,
                    rgb(color).into(),
                ),
                SunburstNode::new(
                    format!("{id}/{sub}/backup"),
                    t("Backup", "备份"),
                    base * 0.35,
                    rgb(color).into(),
                ),
            ];
            branches.push(
                SunburstNode::new(format!("{id}/{sub}"), t(en, zh), 0., rgb(color).into())
                    .with_children(leaves.to_vec()),
            );
        }
        groups
            .push(SunburstNode::new(id, t(en, zh), 0., rgb(color).into()).with_children(branches));
    }
    SunburstNode::new("all", t("All teams", "所有团队"), 0., rgb(0x8f7ad4).into())
        .with_children(groups)
}

/// The reference demo: electricity generation by source for four countries.
/// Values are back-solved from the official shares (each country's blocks
/// tile 43/4/1/15/26/11, 14/12/65/6/1/2, 34/2/14/35/1/14 and
/// 14/6/19/43/16/2) and sum to the published totals.
fn waffle_country(step: usize) -> (SharedString, SharedString) {
    let (country, total): (&str, i64) = match step {
        1 => ("France", 494),
        2 => ("United Kingdom", 285),
        3 => ("United States", 4178),
        _ => ("Germany", 507),
    };
    let text = {
        let digits = total.abs().to_string();
        let mut grouped = String::new();
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                grouped.push(',');
            }
            grouped.push(ch);
        }
        grouped
    };
    (country.into(), text.into())
}
fn waffle_label(step: usize) -> SharedString {
    let (country, _) = waffle_country(step);
    format!("Electricity generation by source, {country}, 2023").into()
}
fn waffle_data(step: usize, language: UiLanguage) -> Vec<WaffleCategory> {
    let values = [
        [218., 20., 5., 76., 132., 56.],
        [69., 59., 321., 30., 5., 10.],
        [97., 6., 40., 100., 3., 39.],
        [585., 251., 794., 1796., 668., 84.],
    ][step.min(3)];
    [
        ("wind-solar", "Wind and solar", "风能与太阳能"),
        ("hydro", "Hydro", "水电"),
        ("nuclear", "Nuclear", "核电"),
        ("gas", "Gas", "燃气"),
        ("coal", "Coal", "煤电"),
        ("other", "Other", "其他"),
    ]
    .into_iter()
    .zip(values)
    .map(|((id, en, zh), value)| WaffleCategory::new(id, language.text(en, zh), value))
    .collect()
}

fn slope_data(step: usize, language: UiLanguage) -> Vec<SlopeSeries> {
    let values = [
        [(3.1, 4.6), (3.4, 3.6), (1.9, 1.3), (2.2, 3.9)],
        [(3.6, 5.2), (3.1, 2.8), (1.6, 2.4), (2.0, 3.1)],
        [(2.8, 3.3), (3.8, 4.1), (2.4, 1.8), (2.6, 2.2)],
    ][step];
    [
        ("email", "Email", "邮件"),
        ("search", "Organic search", "自然搜索"),
        ("social", "Paid social", "付费社交"),
        ("referral", "Referral", "引荐流量"),
    ]
    .into_iter()
    .zip(values)
    .map(|((id, en, zh), (before, after))| {
        SlopeSeries::new(id, language.text(en, zh), before, after)
    })
    .collect()
}

fn radar_axes(language: UiLanguage) -> Vec<RadarAxis> {
    let t = |en, zh| language.text(en, zh);
    [
        ("talent", t("Talent pool", "人才库")),
        ("cost", t("Affordability", "成本")),
        ("transit", t("Transit", "交通")),
        ("time", t("Time zone fit", "时区契合")),
        ("life", t("Quality of life", "生活质量")),
        ("tax", t("Tax climate", "税务环境")),
    ]
    .into_iter()
    .map(|(id, label)| RadarAxis::new(id, label))
    .collect()
}
fn radar_data(step: usize, language: UiLanguage) -> Vec<RadarProfile> {
    let rivals = [
        [86., 58., 92., 80., 78., 46.],
        [81., 42., 64., 72., 82., 55.],
        [74., 54., 26., 60., 71., 84.],
    ];
    let labels = [
        language.text("Berlin", "柏林"),
        language.text("Toronto", "多伦多"),
        language.text("Austin", "奥斯汀"),
    ];
    vec![
        RadarProfile::new(
            "baseline",
            language.text("Lisbon", "里斯本"),
            vec![64., 78., 71., 88., 86., 66.],
            rgb(0x55adff).into(),
        ),
        RadarProfile::new(
            "rival",
            labels[step],
            rivals[step].to_vec(),
            rgb(0x9c37be).into(),
        ),
    ]
}

fn split_data(step: usize, language: UiLanguage) -> Vec<SplitSegment> {
    // Reference demo figures: 7/30/90-day windows on northwind.shop. The
    // last two shares wear the neutral muted and border-strong colours.
    let values = [
        [41_300., 24_600., 19_000., 9_800., 3_100., 1_500.],
        [168_400., 88_900., 97_300., 51_200., 9_700., 5_600.],
        [462_900., 301_400., 234_800., 93_700., 38_200., 13_130.],
    ];
    [
        ("organic", "Organic search", "自然搜索", 0x55adff),
        ("direct", "Direct", "直接访问", 0x9c37be),
        ("paid", "Paid search", "付费搜索", 0xe66e00),
        ("social", "Social", "社交", 0x00a861),
        ("referral", "Referral", "引荐", 0x8c8c8c),
        ("other", "Other", "其他", 0x484848),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (id, en, zh, color))| {
        SplitSegment::new(
            id,
            language.text(en, zh),
            values[step][i],
            rgb(color).into(),
        )
    })
    .collect()
}
/// Demo race data: ten products, twelve months, cumulative year to date.
const RACE_TABLE: [(&str, &str, &str, &str, [f32; 12]); 10] = [
    (
        "chair",
        "Oak lounge chair",
        "橡木休闲椅",
        "furniture",
        [
            14_000., 30_000., 52_000., 86_000., 138_000., 208_000., 296_000., 400_000., 520_000.,
            658_000., 814_000., 992_000.,
        ],
    ),
    (
        "throw",
        "Linen throw",
        "亚麻毯",
        "textiles",
        [
            92_000., 170_000., 222_000., 256_000., 280_000., 296_000., 310_000., 326_000.,
            352_000., 400_000., 480_000., 598_000.,
        ],
    ),
    (
        "lamp",
        "Mushroom lamp",
        "蘑菇台灯",
        "lighting",
        [
            66_000., 128_000., 186_000., 240_000., 291_000., 339_000., 385_000., 435_000.,
            492_000., 560_000., 638_000., 728_000.,
        ],
    ),
    (
        "carafe",
        "Glass carafe",
        "玻璃水瓶",
        "tableware",
        [
            10_000., 22_000., 40_000., 70_000., 122_000., 206_000., 304_000., 394_000., 448_000.,
            474_000., 492_000., 522_000.,
        ],
    ),
    (
        "cups",
        "Stoneware cups",
        "陶土杯",
        "tableware",
        [
            44_000., 86_000., 130_000., 171_000., 213_000., 253_000., 291_000., 331_000., 373_000.,
            418_000., 470_000., 540_000.,
        ],
    ),
    (
        "bowls",
        "Stacked bowls",
        "叠放碗",
        "tableware",
        [
            27_000., 53_000., 81_000., 110_000., 137_000., 165_000., 191_000., 218_000., 247_000.,
            277_000., 313_000., 360_000.,
        ],
    ),
    (
        "ceramic",
        "Ceramic lamp",
        "陶瓷台灯",
        "lighting",
        [
            18_000., 38_000., 59_000., 79_000., 101_000., 122_000., 144_000., 168_000., 194_000.,
            222_000., 253_000., 287_000.,
        ],
    ),
    (
        "sideboard",
        "Walnut sideboard",
        "胡桃木边柜",
        "furniture",
        [
            75_000., 138_000., 195_000., 246_000., 291_000., 330_000., 366_000., 399_000.,
            438_000., 483_000., 534_000., 588_000.,
        ],
    ),
    (
        "pillow",
        "Striped pillow",
        "条纹抱枕",
        "textiles",
        [
            10_000., 21_000., 34_000., 50_000., 67_000., 85_000., 104_000., 121_000., 137_000.,
            153_000., 171_000., 194_000.,
        ],
    ),
    (
        "rug",
        "Wool rug",
        "羊毛地毯",
        "textiles",
        [
            40_000., 74_000., 102_000., 122_000., 137_000., 148_000., 157_000., 168_000., 186_000.,
            216_000., 256_000., 306_000.,
        ],
    ),
];
fn race_items(language: UiLanguage) -> Vec<RaceEntry> {
    RACE_TABLE
        .into_iter()
        .map(|(id, en, zh, group, _)| RaceEntry::new(id, language.text(en, zh)).with_group(group))
        .collect()
}
fn race_frames(language: UiLanguage) -> Vec<RaceFrame> {
    const MONTHS: [(&str, &str); 12] = [
        ("January", "一月"),
        ("February", "二月"),
        ("March", "三月"),
        ("April", "四月"),
        ("May", "五月"),
        ("June", "六月"),
        ("July", "七月"),
        ("August", "八月"),
        ("September", "九月"),
        ("October", "十月"),
        ("November", "十一月"),
        ("December", "十二月"),
    ];
    MONTHS
        .into_iter()
        .enumerate()
        .map(|(index, (en, zh))| {
            let values = RACE_TABLE
                .into_iter()
                .map(|(id, _, _, _, series)| (SharedString::from(id), series[index]))
                .collect();
            RaceFrame::new(language.text(en, zh), values)
        })
        .collect()
}

fn funnel_data(step: usize, language: UiLanguage) -> Vec<FunnelStage> {
    let values = [
        [
            [68420., 31580.],
            [39710., 21460.],
            [9140., 7280.],
            [4310., 4090.],
            [1540., 2050.],
        ],
        [
            [72000., 34000.],
            [43100., 23600.],
            [10320., 8000.],
            [5350., 4500.],
            [2100., 2300.],
        ],
        [
            [61800., 38000.],
            [37500., 27000.],
            [8500., 9100.],
            [4600., 5000.],
            [1800., 2600.],
        ],
    ];
    [
        ("visit", "Visit", "访问"),
        ("product", "Product", "商品"),
        ("cart", "Cart", "购物车"),
        ("checkout", "Checkout", "结账"),
        ("purchase", "Purchase", "购买"),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (id, en, zh))| FunnelStage::new(id, language.text(en, zh), values[step][i].to_vec()))
    .collect()
}

fn stream_layers(language: UiLanguage) -> Vec<StreamLayer> {
    [
        ("bugs", "Bugs", "缺陷", 0x55adff),
        ("billing", "Billing", "账单", 0x9c37be),
        ("onboarding", "Onboarding", "新手引导", 0xe66e00),
        ("account", "Account", "账户", 0x00a861),
    ]
    .into_iter()
    .map(|(id, en, zh, color)| StreamLayer::new(id, language.text(en, zh), rgb(color).into()))
    .collect()
}
fn stream_data(step: usize, language: UiLanguage) -> Vec<StreamPoint> {
    let months = [
        ("Mar", "3月"),
        ("Apr", "4月"),
        ("May", "5月"),
        ("Jun", "6月"),
    ];
    (0..12)
        .map(|week| {
            let values = (0..4)
                .map(|layer| {
                    let angle = (week as f32 * 0.55 + layer as f32 * 1.2 + step as f32 * 0.7).sin();
                    14. + layer as f32 * 3. + angle * 7. + week as f32 * (layer + 1) as f32 * 0.25
                })
                .collect();
            let label = format!("{} {}", language.text("Week", "周"), week + 1);
            // Months open on their first week; the axis drops labels that do not fit.
            let axis = (week % 4 == 0)
                .then(|| {
                    months
                        .get(week / 4)
                        .map(|(en, zh)| language.text(en, zh).to_string())
                })
                .flatten()
                .map(SharedString::from);
            StreamPoint::new(label, values).with_axis_label(axis)
        })
        .collect()
}

fn treemap_data(step: usize, language: UiLanguage) -> Vec<TreeTile> {
    let labels = [
        ("north", "North", "北区", 0x8f7ad4),
        ("south", "South", "南区", 0x62b5d0),
        ("west", "West", "西区", 0x58bd91),
        ("east", "East", "东区", 0xd1a263),
    ];
    labels
        .into_iter()
        .enumerate()
        .map(|(i, (id, en, zh, color))| {
            let children = [
                ("basic", "Basic", "基础", 0.4),
                ("team", "Team", "团队", 0.35),
                ("enterprise", "Enterprise", "企业", 0.25),
            ]
            .into_iter()
            .map(|(sub_id, sub_en, sub_zh, part)| {
                let revenue = (400. - i as f32 * 55. + step as f32 * 20.) * part;
                TreeTile::new(
                    format!("{id}-{sub_id}"),
                    language.text(sub_en, sub_zh),
                    revenue,
                    rgb(color).into(),
                )
                .with_metrics(vec![revenue, revenue * (1.2 + i as f32 * 0.3)])
                .with_shade_values(vec![
                    4. + i as f32 * 3. + part * 6.,
                    // Retention spreads across the scale so switching the
                    // shading measure visibly re-grades every branch.
                    [70., 74., 89., 96.5][i] + part * 2.,
                ])
            })
            .collect();
            TreeTile::new(id, language.text(en, zh), 0., rgb(color).into())
                .with_shade_values(vec![6. + i as f32 * 3., 78. + i as f32 * 5.])
                .with_children(children)
        })
        .collect()
}

fn brush_data(step: usize, language: UiLanguage) -> Vec<BrushPoint> {
    (0..90)
        .map(|day| {
            let value = 80.
                + day as f32 * 0.8
                + (day as f32 * 0.19 + step as f32).sin() * 15.
                + (day as f32 * 0.57).sin() * 4.;
            let point = BrushPoint::new(
                format!("{} {}", language.text("Day", "日期"), day + 1),
                value,
            );
            match day {
                61 => point.with_event(language.text("Release", "发布")),
                72 => point.with_event(language.text("Campaign", "推广")),
                83 => point.with_event(language.text("Upgrade", "升级")),
                _ => point,
            }
        })
        .collect()
}
fn ridgeline_data(step: usize, language: UiLanguage) -> Vec<RidgeSeries> {
    let en = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let zh = [
        "一月",
        "二月",
        "三月",
        "四月",
        "五月",
        "六月",
        "七月",
        "八月",
        "九月",
        "十月",
        "十一月",
        "十二月",
    ];
    (0..12)
        .map(|i| {
            let season = ((i as f32 - 1.) * std::f32::consts::TAU / 12.).sin();
            let center = 13. + season * 11. + step as f32 * 2.;
            let samples = (0..32)
                .map(|n| {
                    center
                        + (n as f32 * 0.81 + i as f32 * 0.4).sin() * 5.
                        + (n as f32 * 1.31).cos() * 2.
                })
                .collect();
            RidgeSeries::new(
                format!("month-{i}"),
                language.text(en[i], zh[i]),
                samples,
                rgb(0x609cd8).into(),
            )
        })
        .collect()
}
