//! Authentic interactive KStack Cluster Management Platform (`KStack 集群管理`).

use super::{is_dark, is_zh, tr};
use gpui::{prelude::*, *};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum NavPage {
    Overview,
    Monitor,
    Nodes,
    Models,
    Deploy,
    Deployments,
    Playground,
    ApiTokens,
}

#[derive(Clone)]
pub(super) struct DeploymentRow {
    name: SharedString,
    model: SharedString,
    engine: &'static str,
    public_name: SharedString,
    node: &'static str,
    sleeping: bool,
}

pub(super) struct KStackConsole {
    pub(super) page: NavPage,
    quick_mode: bool,
    pub(super) model_idx: usize,
    pub(super) chip_idx: usize,
    topo_idx: usize,
    pub(super) dgpu_share: bool,
    pub(super) gpu_mask: [bool; 8],
    pub(super) ai_drawer: bool,
    pub(super) deployments: Vec<DeploymentRow>,
    playground_idx: usize,
    pub(super) token_count: usize,
}

struct ModelPreset {
    id: &'static str,
    pub_name: &'static str,
    weight_gib: f32,
    engine_gib: f32,
    image: &'static str,
}

const MODELS: [ModelPreset; 4] = [
    ModelPreset {
        id: "Qwen/Qwen3-32B",
        pub_name: "qwen3-32b",
        weight_gib: 32.4,
        engine_gib: 3.6,
        image: "gpuworks/gpuworks-runner:cuda13.0-vllm0.30.0-r2",
    },
    ModelPreset {
        id: "deepseek-ai/DeepSeek-R1-Distill-70B",
        pub_name: "deepseek-r1-70b",
        weight_gib: 39.8,
        engine_gib: 4.2,
        image: "gpuworks/gpuworks-runner:cuda13.0-sglang0.4.6",
    },
    ModelPreset {
        id: "Qwen/Qwen3-8B",
        pub_name: "qwen3-8b",
        weight_gib: 8.2,
        engine_gib: 2.0,
        image: "gpuworks/gpuworks-runner:cann8.1-vllm-ascend",
    },
    ModelPreset {
        id: "THUDM/GLM-4.5-Air",
        pub_name: "glm-4.5-air",
        weight_gib: 22.0,
        engine_gib: 3.1,
        image: "gpuworks/gpuworks-runner:hggc-sglang-mooncake",
    },
];

struct ChipPreset {
    vendor: &'static str,
    zh_name: &'static str,
    en_name: &'static str,
    vram_gib: f32,
    node_name: &'static str,
}

const CHIPS: [ChipPreset; 4] = [
    ChipPreset {
        vendor: "NVIDIA",
        zh_name: "NVIDIA H800 · 80 GiB",
        en_name: "NVIDIA H800 · 80 GiB",
        vram_gib: 80.0,
        node_name: "hz-gpu-node-04",
    },
    ChipPreset {
        vendor: "Ascend",
        zh_name: "华为 Ascend 910C · 64 GiB",
        en_name: "Huawei Ascend 910C · 64 GiB",
        vram_gib: 64.0,
        node_name: "bj-ascend-02",
    },
    ChipPreset {
        vendor: "PPU",
        zh_name: "平头哥 PPU ZW810E · 96 GiB",
        en_name: "T-Head PPU ZW810E · 96 GiB",
        vram_gib: 96.0,
        node_name: "sh-ppu-node-09",
    },
    ChipPreset {
        vendor: "AMD",
        zh_name: "AMD MI300X · 192 GiB",
        en_name: "AMD MI300X · 192 GiB",
        vram_gib: 192.0,
        node_name: "sg-mi300x-01",
    },
];

impl KStackConsole {
    pub(super) fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            page: NavPage::Deploy,
            quick_mode: true,
            model_idx: 0,
            chip_idx: 0,
            topo_idx: 0,
            dgpu_share: false,
            gpu_mask: [true, false, false, false, false, false, false, false],
            ai_drawer: false,
            deployments: vec![
                DeploymentRow {
                    name: "qwen3-prod".into(),
                    model: "Qwen/Qwen3-32B".into(),
                    engine: "vllm · NIXL",
                    public_name: "qwen3-32b".into(),
                    node: "hz-gpu-node-04",
                    sleeping: false,
                },
                DeploymentRow {
                    name: "r1-ascend".into(),
                    model: "DeepSeek-R1-Distill-32B".into(),
                    engine: "vllm-ascend · Mooncake",
                    public_name: "deepseek-r1".into(),
                    node: "bj-ascend-02",
                    sleeping: false,
                },
                DeploymentRow {
                    name: "embed-bge".into(),
                    model: "BAAI/bge-m3".into(),
                    engine: "vllm · dGPU 12GiB",
                    public_name: "bge-m3".into(),
                    node: "hz-gpu-node-04",
                    sleeping: true,
                },
            ],
            playground_idx: 0,
            token_count: 3,
        }
    }

    fn selected_gpu_count(&self) -> usize {
        self.gpu_mask.iter().filter(|&&on| on).count().max(1)
    }

    fn render_topbar(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let brand_bg = if dark { rgb(0x11151d) } else { rgb(0xffffff) };
        let brand_ink = if dark { rgb(0xf8fafc) } else { rgb(0x0f172a) };
        let border_c = if dark { rgb(0x1e293b) } else { rgb(0xe2e8f0) };

        div()
            .h(px(40.))
            .w_full()
            .flex_none()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(border_c)
            .child(
                div()
                    .w(px(184.))
                    .h_full()
                    .flex_none()
                    .px(px(14.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .bg(brand_bg)
                    .border_r_1()
                    .border_color(border_c)
                    .child(
                        div()
                            .w(px(20.))
                            .h(px(20.))
                            .rounded(px(5.))
                            .bg(rgb(0x0b57d0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(11.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("K"),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(brand_ink)
                            .child("KStack"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .px(px(16.))
                    .bg(rgb(0x0b57d0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_color(rgb(0xffffff))
                    .child(
                        div()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(tr(
                                zh,
                                "KStack 集群管理",
                                "KStack Cluster Management Platform",
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .text_size(px(11.5))
                            .child(
                                div()
                                    .id("kstack-ai-btn")
                                    .cursor_pointer()
                                    .px(px(8.))
                                    .py(px(3.))
                                    .rounded(px(5.))
                                    .bg(if self.ai_drawer {
                                        rgba(0xffffff38)
                                    } else {
                                        rgba(0xffffff1c)
                                    })
                                    .hover(|s| s.bg(rgba(0xffffff30)))
                                    .on_click(cx.listener(|v, _, _, cx| {
                                        v.ai_drawer = !v.ai_drawer;
                                        cx.notify();
                                    }))
                                    .child(tr(zh, "* AI 助手", "* AI Assistant")),
                            )
                            .child(
                                div()
                                    .text_color(rgba(0xffffffd8))
                                    .child(tr(zh, "中文", "EN")),
                            )
                            .child(div().text_color(rgba(0xffffffd8)).child(tr(
                                zh,
                                "管理员",
                                "Administrator",
                            ))),
                    ),
            )
    }

    fn render_sidebar(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let sb_bg = if dark { rgb(0x0f131a) } else { rgb(0xffffff) };
        let border_c = if dark { rgb(0x1e293b) } else { rgb(0xe5e7eb) };
        let group_c = if dark { rgb(0x64748b) } else { rgb(0x94a3b8) };
        let row_ink = if dark { rgb(0xcbd5e1) } else { rgb(0x334155) };
        let act_bg = if dark { rgb(0x13294b) } else { rgb(0xe8f1ff) };
        let act_ink = if dark { rgb(0x60a5fa) } else { rgb(0x0b57d0) };
        let hover_bg = if dark { rgb(0x171e2b) } else { rgb(0xf1f5f9) };

        type NavGroup = (
            &'static str,
            &'static str,
            &'static [(NavPage, &'static str, &'static str)],
        );
        let groups: &[NavGroup] = &[
            (
                "概览",
                "Overview",
                &[
                    (NavPage::Overview, "概览", "Overview"),
                    (NavPage::Monitor, "监控中心", "Monitor Center"),
                ],
            ),
            (
                "资源",
                "Resources",
                &[
                    (NavPage::Nodes, "计算节点 (1,003)", "Compute Nodes"),
                    (NavPage::Models, "模型与规格 (392)", "Models (392)"),
                ],
            ),
            (
                "推理服务",
                "Inference",
                &[
                    (NavPage::Deploy, "部署", "Deploy"),
                    (NavPage::Deployments, "部署列表", "Deployments"),
                    (NavPage::Playground, "试验场", "Playground"),
                ],
            ),
            (
                "平台",
                "Platform",
                &[(NavPage::ApiTokens, "API 令牌", "API Tokens")],
            ),
        ];

        let mut col = div()
            .w(px(184.))
            .h_full()
            .flex_none()
            .bg(sb_bg)
            .border_r_1()
            .border_color(border_c)
            .py(px(10.))
            .px(px(8.))
            .flex()
            .flex_col()
            .gap(px(10.));

        for (g_idx, (gzh, g_en, rows)) in groups.iter().enumerate() {
            let mut sec = div().flex().flex_col().gap(px(2.)).child(
                div()
                    .px(px(8.))
                    .pb(px(2.))
                    .text_size(px(10.5))
                    .text_color(group_c)
                    .child(tr(zh, gzh, g_en)),
            );
            for (r_idx, (page, lzh, len)) in rows.iter().enumerate() {
                let active = self.page == *page;
                let target = *page;
                let suffix = if target == NavPage::Deployments {
                    format!(" ({})", self.deployments.len())
                } else {
                    String::new()
                };
                let nav_sel = format!("kstack-nav-{}-{}", g_idx, r_idx);
                sec = sec.child(
                    div()
                        .id(("kstack-nav", g_idx * 10 + r_idx))
                        .debug_selector(move || nav_sel.clone())
                        .cursor_pointer()
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .bg(if active { act_bg } else { rgba(0x00000000) })
                        .text_color(if active { act_ink } else { row_ink })
                        .font_weight(if active {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .hover(move |s| if active { s } else { s.bg(hover_bg) })
                        .on_click(cx.listener(move |v, _, _, cx| {
                            v.page = target;
                            cx.notify();
                        }))
                        .child(div().w(px(5.)).h(px(5.)).rounded_full().bg(if active {
                            act_ink
                        } else {
                            group_c
                        }))
                        .child(format!("{}{suffix}", tr(zh, lzh, len))),
                );
            }
            col = col.child(sec);
        }
        col
    }

    fn page_banner(
        icon_txt: &'static str,
        crumb: &str,
        title: &str,
        sub: &str,
        right: Option<AnyElement>,
        dark: bool,
    ) -> Div {
        let card_bg = if dark { rgb(0x131822) } else { rgb(0xffffff) };
        let border_c = if dark { rgb(0x1f2937) } else { rgb(0xe5e7eb) };
        let muted = if dark { rgb(0x94a3b8) } else { rgb(0x64748b) };
        let ink = if dark { rgb(0xf8fafc) } else { rgb(0x0f172a) };

        let mut row = div()
            .p(px(12.))
            .rounded(px(8.))
            .bg(card_bg)
            .border_1()
            .border_color(border_c)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .w(px(34.))
                            .h(px(34.))
                            .rounded(px(7.))
                            .bg(rgb(0x0b57d0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(13.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child(icon_txt),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(muted)
                                    .child(crumb.to_owned()),
                            )
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(ink)
                                    .child(title.to_owned()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(muted)
                                    .child(sub.to_owned()),
                            ),
                    ),
            );
        if let Some(r) = right {
            row = row.child(r);
        }
        row
    }

    fn render_deploy_page(&self, zh: bool, dark: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let card_bg = if dark { rgb(0x131822) } else { rgb(0xffffff) };
        let sub_bg = if dark { rgb(0x0d1118) } else { rgb(0xf8fafc) };
        let border_c = if dark { rgb(0x1f2937) } else { rgb(0xe5e7eb) };
        let muted = if dark { rgb(0x94a3b8) } else { rgb(0x64748b) };
        let ink = if dark { rgb(0xf8fafc) } else { rgb(0x0f172a) };

        let model = &MODELS[self.model_idx];
        let chip = &CHIPS[self.chip_idx];
        let gpu_cnt = self.selected_gpu_count() as f32;
        let card_cap = if self.dgpu_share {
            (chip.vram_gib * 0.6).round()
        } else {
            chip.vram_gib
        };
        let total_cap = card_cap * gpu_cnt;
        let fixed_need = model.weight_gib + model.engine_gib;
        let fits = total_cap >= fixed_need + 4.0;
        let kv_pool = (total_cap - fixed_need).max(0.0);
        let est_tokens = (kv_pool * 9_350.0) as usize;

        let mode_switch = div()
            .flex()
            .items_center()
            .rounded(px(6.))
            .border_1()
            .border_color(border_c)
            .overflow_hidden()
            .child(
                div()
                    .id("ks-quick-mode")
                    .cursor_pointer()
                    .px(px(10.))
                    .py(px(5.))
                    .text_size(px(11.5))
                    .bg(if self.quick_mode {
                        rgb(0x0b57d0)
                    } else {
                        card_bg
                    })
                    .text_color(if self.quick_mode { rgb(0xffffff) } else { ink })
                    .on_click(cx.listener(|v, _, _, cx| {
                        v.quick_mode = true;
                        cx.notify();
                    }))
                    .child(tr(zh, "快捷部署", "Quick Deploy")),
            )
            .child(
                div()
                    .id("ks-expert-mode")
                    .cursor_pointer()
                    .px(px(10.))
                    .py(px(5.))
                    .text_size(px(11.5))
                    .bg(if !self.quick_mode {
                        rgb(0x0b57d0)
                    } else {
                        card_bg
                    })
                    .text_color(if !self.quick_mode { rgb(0xffffff) } else { ink })
                    .on_click(cx.listener(|v, _, _, cx| {
                        v.quick_mode = false;
                        cx.notify();
                    }))
                    .child(tr(zh, "专家部署", "Expert Deploy")),
            )
            .into_any_element();

        // Model selector pills
        let mut model_pills = div().flex().flex_wrap().gap(px(5.));
        for (idx, m) in MODELS.iter().enumerate() {
            let sel = self.model_idx == idx;
            model_pills = model_pills.child(
                div()
                    .id(("ks-model", idx))
                    .cursor_pointer()
                    .px(px(9.))
                    .py(px(4.))
                    .rounded(px(5.))
                    .border_1()
                    .border_color(if sel { rgb(0x0b57d0) } else { border_c })
                    .bg(if sel {
                        if dark { rgb(0x13294b) } else { rgb(0xe8f1ff) }
                    } else {
                        sub_bg
                    })
                    .text_size(px(11.5))
                    .text_color(if sel {
                        if dark { rgb(0x60a5fa) } else { rgb(0x0b57d0) }
                    } else {
                        ink
                    })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.model_idx = idx;
                        cx.notify();
                    }))
                    .child(m.id),
            );
        }

        // Target chip selector pills
        let mut chip_pills = div().flex().flex_wrap().gap(px(5.));
        for (idx, c) in CHIPS.iter().enumerate() {
            let sel = self.chip_idx == idx;
            chip_pills = chip_pills.child(
                div()
                    .id(("ks-chip", idx))
                    .cursor_pointer()
                    .px(px(9.))
                    .py(px(4.))
                    .rounded(px(5.))
                    .border_1()
                    .border_color(if sel { rgb(0x0b57d0) } else { border_c })
                    .bg(if sel {
                        if dark { rgb(0x13294b) } else { rgb(0xe8f1ff) }
                    } else {
                        sub_bg
                    })
                    .text_size(px(11.5))
                    .text_color(if sel {
                        if dark { rgb(0x60a5fa) } else { rgb(0x0b57d0) }
                    } else {
                        ink
                    })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.chip_idx = idx;
                        cx.notify();
                    }))
                    .child(tr(zh, c.zh_name, c.en_name)),
            );
        }

        // Topology cards (Single-node / Multi-node / PD-disaggregation)
        let topos = [
            (
                tr(zh, "单机", "Single-node"),
                tr(zh, "每个副本跑在一台节点上", "Each replica on one node"),
            ),
            (
                tr(zh, "多机并行", "Multi-node"),
                tr(zh, "2 台节点 · TP/PP", "2 nodes · TP/PP"),
            ),
            (
                tr(zh, "PD 分离", "PD Split"),
                tr(zh, "分离 prefill 与 decode", "Disaggregated prefill/decode"),
            ),
        ];
        let mut topo_row = div().flex().flex_wrap().gap(px(6.));
        for (idx, (t_title, t_sub)) in topos.iter().enumerate() {
            let sel = self.topo_idx == idx;
            topo_row = topo_row.child(
                div()
                    .id(("ks-topo", idx))
                    .cursor_pointer()
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if sel { rgb(0x0b57d0) } else { border_c })
                    .bg(if sel {
                        if dark { rgb(0x13294b) } else { rgb(0xf0f6ff) }
                    } else {
                        sub_bg
                    })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.topo_idx = idx;
                        if idx == 1 {
                            v.gpu_mask[1] = true;
                        }
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if sel {
                                if dark { rgb(0x60a5fa) } else { rgb(0x0b57d0) }
                            } else {
                                ink
                            })
                            .child(*t_title),
                    )
                    .child(div().text_size(px(10.)).text_color(muted).child(*t_sub)),
            );
        }

        // 8 clickable GPU slot indices (exact tutorial-deploy-node.png!)
        let mut gpu_slots = div().flex().items_center().gap(px(4.));
        for g in 0..8 {
            let on = self.gpu_mask[g];
            gpu_slots = gpu_slots.child(
                div()
                    .id(("ks-gpu-slot", g))
                    .cursor_pointer()
                    .w(px(22.))
                    .h(px(22.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(if on { rgb(0x0b57d0) } else { border_c })
                    .bg(if on { rgb(0x0b57d0) } else { sub_bg })
                    .flex()
                    .items_center()
                    .justify_center()
                    .font_family("Fira Code")
                    .text_size(px(10.5))
                    .text_color(if on { rgb(0xffffff) } else { muted })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.gpu_mask[g] = !v.gpu_mask[g];
                        if v.gpu_mask.iter().all(|&x| !x) {
                            v.gpu_mask[g] = true;
                        }
                        cx.notify();
                    }))
                    .child(format!("{g}")),
            );
        }

        let w_pct = ((model.weight_gib / total_cap) * 100.0).clamp(6.0, 82.0);
        let e_pct = ((model.engine_gib / total_cap) * 100.0).clamp(4.0, 18.0);
        let k_pct = (100.0 - w_pct - e_pct).max(0.0);

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(Self::page_banner(
                "▲",
                tr(zh, "推理服务 / 部署", "Inference Services / Deploy"),
                tr(zh, "部署", "Deploy"),
                tr(
                    zh,
                    "选择模型与计算节点，校验显存和网络条件后创建推理服务",
                    "Select a model and compute nodes, validate VRAM, then create an inference service",
                ),
                Some(mode_switch),
                dark,
            ))
            // Basic Configuration Card
            .child(
                div()
                    .p(px(12.))
                    .rounded(px(8.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(border_c)
                    .flex()
                    .flex_col()
                    .gap(px(9.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(ink)
                                    .child(tr(zh, "基础配置", "Basic Configuration")),
                            )
                            .child(
                                div()
                                    .font_family("Fira Code")
                                    .text_size(px(10.5))
                                    .text_color(muted)
                                    .child(format!("image: {}", model.image)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(muted)
                                    .child(tr(zh, "* 模型（内置 1,235 份真机验证规格）", "* Model (1,235 verified launch specs)")),
                            )
                            .child(model_pills),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(muted)
                                    .child(tr(zh, "* 目标加速卡厂商与型号", "* Target Accelerator")),
                            )
                            .child(chip_pills),
                    ),
            )
            // Bottom Split: Left Node & GPU Selector + Right `配置评估` Card
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .items_start()
                    .child(
                        div()
                            .flex_1()
                            .p(px(12.))
                            .rounded(px(8.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(border_c)
                            .flex()
                            .flex_col()
                            .gap(px(9.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(ink)
                                            .child(tr(zh, "节点与拓扑选择", "Node & Topology Placement")),
                                    )
                                    .child(
                                        div()
                                            .id("ks-dgpu-toggle")
                                            .cursor_pointer()
                                            .px(px(8.))
                                            .py(px(3.))
                                            .rounded(px(5.))
                                            .border_1()
                                            .border_color(if self.dgpu_share {
                                                rgb(0x0b57d0)
                                            } else {
                                                border_c
                                            })
                                            .bg(if self.dgpu_share {
                                                if dark { rgb(0x13294b) } else { rgb(0xe8f1ff) }
                                            } else {
                                                sub_bg
                                            })
                                            .text_size(px(11.))
                                            .text_color(if self.dgpu_share {
                                                if dark { rgb(0x60a5fa) } else { rgb(0x0b57d0) }
                                            } else {
                                                muted
                                            })
                                            .on_click(cx.listener(|v, _, _, cx| {
                                                v.dgpu_share = !v.dgpu_share;
                                                cx.notify();
                                            }))
                                            .child(if self.dgpu_share {
                                                tr(zh, "✓ dGPU 显存配额 60%", "✓ dGPU Quota 60%")
                                            } else {
                                                tr(zh, "开启单卡 dGPU 切分", "Enable dGPU Slicing")
                                            }),
                                    ),
                            )
                            .child(topo_row)
                            .child(
                                div()
                                    .p(px(9.))
                                    .rounded(px(6.))
                                    .bg(sub_bg)
                                    .border_1()
                                    .border_color(border_c)
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .font_family("Fira Code")
                                                    .text_size(px(11.5))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(ink)
                                                    .child(format!(
                                                        "{} · 8 × {}",
                                                        chip.node_name, chip.vendor
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.5))
                                                    .text_color(muted)
                                                    .child(tr(
                                                        zh,
                                                        "点击右侧卡号切换分配的加速卡：",
                                                        "Click GPU slot indices to toggle cards:",
                                                    )),
                                            ),
                                    )
                                    .child(gpu_slots),
                            ),
                    )
                    // Authentic KStack `配置评估` Card with left blue bar & 3-color VRAM bar!
                    .child(
                        div()
                            .w(px(292.))
                            .flex_none()
                            .p(px(12.))
                            .rounded(px(8.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(border_c)
                            .border_l_4()
                            .border_color(rgb(0x0b57d0))
                            .flex()
                            .flex_col()
                            .gap(px(7.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(ink)
                                            .child(tr(zh, "配置评估", "Configuration Evaluation")),
                                    )
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .py(px(1.))
                                            .rounded(px(4.))
                                            .bg(if fits {
                                                rgba(0x22c55e24)
                                            } else {
                                                rgba(0xef444424)
                                            })
                                            .text_size(px(10.5))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(if fits {
                                                rgb(0x16a34a)
                                            } else {
                                                rgb(0xef4444)
                                            })
                                            .child(if fits {
                                                tr(zh, "估算置信度: 实测", "Confidence: Verified")
                                            } else {
                                                tr(zh, "显存不足: 请多选卡", "VRAM Insufficient")
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(if fits { rgb(0x16a34a) } else { rgb(0xef4444) })
                                    .child(format!(
                                        "{}: {:.1} / {:.1} GiB ({} GPU)",
                                        tr(zh, "固定启动需求", "Startup requirement"),
                                        fixed_need,
                                        total_cap,
                                        self.selected_gpu_count()
                                    )),
                            )
                            // 3-color stacked VRAM bar (Weight blue + Engine indigo + KV green)
                            .child(
                                div()
                                    .w_full()
                                    .h(px(14.))
                                    .rounded(px(4.))
                                    .overflow_hidden()
                                    .bg(sub_bg)
                                    .flex()
                                    .child(div().h_full().w(relative(w_pct / 100.0)).bg(rgb(0x1677ff)))
                                    .child(div().h_full().w(relative(e_pct / 100.0)).bg(rgb(0x4f46e5)))
                                    .child(
                                        div()
                                            .h_full()
                                            .w(relative(if fits { k_pct / 100.0 } else { 0.0 }))
                                            .bg(rgb(0x22c55e)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap(px(8.))
                                    .text_size(px(10.5))
                                    .text_color(muted)
                                    .child(format!("■ {} {:.1}G", tr(zh, "权重", "Weights"), model.weight_gib))
                                    .child(format!("■ {} {:.1}G", tr(zh, "引擎预留", "Engine"), model.engine_gib))
                                    .child(format!("■ KV {:.1}G", kv_pool)),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(muted)
                                    .child(format!(
                                        "KV pool {:.1} GiB · {} {} tokens",
                                        kv_pool,
                                        tr(zh, "预计承载", "Est."),
                                        est_tokens
                                    )),
                            )
                            .child(
                                div()
                                    .id("ks-launch-btn")
                                    .debug_selector(|| "ks-launch-btn".into())
                                    .cursor_pointer()
                                    .mt(px(2.))
                                    .h(px(32.))
                                    .rounded(px(6.))
                                    .bg(if fits { rgb(0x0b57d0) } else { rgb(0x64748b) })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xffffff))
                                    .on_click(cx.listener(move |v, _, _, cx| {
                                        if !fits {
                                            v.gpu_mask[1] = true;
                                            cx.notify();
                                            return;
                                        }
                                        let m = &MODELS[v.model_idx];
                                        let c = &CHIPS[v.chip_idx];
                                        v.deployments.insert(
                                            0,
                                            DeploymentRow {
                                                name: format!("{}-{}", m.pub_name, v.deployments.len() + 1).into(),
                                                model: m.id.into(),
                                                engine: if v.dgpu_share { "vllm · dGPU" } else { "vllm · LTO" },
                                                public_name: m.pub_name.into(),
                                                node: c.node_name,
                                                sleeping: false,
                                            },
                                        );
                                        v.page = NavPage::Deployments;
                                        cx.notify();
                                    }))
                                    .child(if fits {
                                        tr(zh, "创建推理部署 →", "Create Deployment →")
                                    } else {
                                        tr(zh, "自动扩容至 2 张 GPU", "Auto-add 2nd GPU")
                                    }),
                            ),
                    ),
            )
    }

    fn render_overview_page(&self, zh: bool, dark: bool) -> impl IntoElement {
        let card_bg = if dark { rgb(0x131822) } else { rgb(0xffffff) };
        let border_c = if dark { rgb(0x1f2937) } else { rgb(0xe5e7eb) };
        let muted = if dark { rgb(0x94a3b8) } else { rgb(0x64748b) };
        let ink = if dark { rgb(0xf8fafc) } else { rgb(0x0f172a) };

        let running_cnt = self.deployments.iter().filter(|d| !d.sleeping).count();
        let total_dep = self.deployments.len();
        let dep_str = format!("{total_dep}");
        let run_str = format!("{running_cnt}");

        let stats = [
            (
                tr(zh, "计算节点总数", "Total compute nodes"),
                "1,003",
                false,
            ),
            (
                tr(zh, "在线计算节点", "Online compute nodes"),
                "1,003",
                true,
            ),
            (tr(zh, "GPU 总数", "Managed GPUs"), "4,012", false),
            (
                tr(zh, "显存总量 (GiB)", "Total VRAM (GiB)"),
                "320,960",
                false,
            ),
            (
                tr(zh, "平均 GPU 利用率", "Average GPU utilization"),
                "74 %",
                true,
            ),
            (tr(zh, "最高 GPU 温度", "Max GPU temperature"), "62 C", true),
            (tr(zh, "GPU 总功耗", "Total GPU power"), "1,180 kW", false),
            (
                tr(zh, "控制面内存占用", "Server RAM (40MB bin)"),
                "1.6 / 16 GiB",
                false,
            ),
            (tr(zh, "公有云与 K8s", "Clouds & K8s"), "4 Valid", true),
            (
                tr(zh, "部署总数", "Total deployments"),
                dep_str.as_str(),
                false,
            ),
            (
                tr(zh, "运行中部署", "Running deployments"),
                run_str.as_str(),
                true,
            ),
            (
                tr(zh, "网关并发请求", "/v1 Active Concurrency"),
                "2,000",
                false,
            ),
        ];

        let mut grid = div().flex().flex_wrap().gap(px(8.));
        for (lbl, val, green) in stats {
            grid = grid.child(
                div()
                    .w(px(196.))
                    .p(px(10.))
                    .rounded(px(7.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(border_c)
                    .child(div().text_size(px(10.5)).text_color(muted).child(lbl))
                    .child(
                        div()
                            .mt(px(3.))
                            .font_family("Fira Code")
                            .text_size(px(17.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(if green { rgb(0x16a34a) } else { ink })
                            .child(val.to_owned()),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(Self::page_banner(
                "●",
                tr(zh, "概览", "Overview"),
                tr(zh, "概览", "Overview"),
                tr(
                    zh,
                    "实时查看跨 7 地域集群容量、GPU 利用率、温度、功耗和服务健康状态",
                    "Monitor multi-region cluster capacity, GPU utilization, temperature, and /v1 health",
                ),
                None,
                dark,
            ))
            .child(
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .rounded(px(7.))
                    .bg(rgba(0x22c55e1a))
                    .border_1()
                    .border_color(rgba(0x22c55e55))
                    .text_size(px(11.5))
                    .text_color(if dark { rgb(0x86efac) } else { rgb(0x15803d) })
                    .child(tr(
                        zh,
                        "✓ 平台运行正常 · 计算节点、推理部署、公有云与共享 Kubernetes 集群均处于健康状态。",
                        "✓ Platform healthy · All compute nodes, deployments, clouds, and shared K8s pools are online.",
                    )),
            )
            .child(grid)
            .child(
                div()
                    .p(px(12.))
                    .rounded(px(8.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(border_c)
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(ink)
                            .child(tr(zh, "异构厂商容量分布", "Capacity by Accelerator Vendor")),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(10.))
                            .rounded_full()
                            .overflow_hidden()
                            .flex()
                            .child(div().h_full().w(relative(0.58)).bg(rgb(0x1677ff)))
                            .child(div().h_full().w(relative(0.22)).bg(rgb(0x10b981)))
                            .child(div().h_full().w(relative(0.12)).bg(rgb(0x6366f1)))
                            .child(div().h_full().w(relative(0.08)).bg(rgb(0xf59e0b))),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .text_size(px(10.5))
                            .text_color(muted)
                            .child("NVIDIA H800/4090 (612 nodes)")
                            .child("Ascend 910C (224)")
                            .child("T-Head PPU (96)")
                            .child("AMD / Hygon / Apple (71)"),
                    ),
            )
    }

    fn render_deployments_page(
        &self,
        zh: bool,
        dark: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let card_bg = if dark { rgb(0x131822) } else { rgb(0xffffff) };
        let border_c = if dark { rgb(0x1f2937) } else { rgb(0xe5e7eb) };
        let muted = if dark { rgb(0x94a3b8) } else { rgb(0x64748b) };
        let ink = if dark { rgb(0xf8fafc) } else { rgb(0x0f172a) };

        let new_btn = div()
            .id("ks-goto-deploy")
            .cursor_pointer()
            .px(px(10.))
            .py(px(6.))
            .rounded(px(6.))
            .bg(rgb(0x0b57d0))
            .text_size(px(11.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(0xffffff))
            .on_click(cx.listener(|v, _, _, cx| {
                v.page = NavPage::Deploy;
                cx.notify();
            }))
            .child(tr(zh, "+ 新建部署", "+ New Deployment"))
            .into_any_element();

        let mut rows = div().flex().flex_col();
        for (idx, d) in self.deployments.iter().enumerate() {
            let sleeping = d.sleeping;
            rows = rows.child(
                div()
                    .px(px(12.))
                    .py(px(9.))
                    .border_t_1()
                    .border_color(border_c)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .w(px(120.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0x0b57d0))
                            .child(d.name.clone()),
                    )
                    .child(
                        div()
                            .w(px(170.))
                            .text_size(px(11.5))
                            .text_color(ink)
                            .child(d.model.clone()),
                    )
                    .child(
                        div()
                            .w(px(130.))
                            .font_family("Fira Code")
                            .text_size(px(10.5))
                            .text_color(muted)
                            .child(d.engine),
                    )
                    .child(
                        div()
                            .w(px(110.))
                            .font_family("Fira Code")
                            .text_size(px(11.))
                            .text_color(ink)
                            .child(d.node),
                    )
                    .child(
                        div()
                            .w(px(82.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if sleeping {
                                rgb(0xf59e0b)
                            } else {
                                rgb(0x16a34a)
                            })
                            .child(if sleeping {
                                tr(zh, "● 已休眠", "● Sleeping")
                            } else {
                                tr(zh, "● 运行中", "● Running")
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .id(("ks-sleep-row", idx))
                                    .cursor_pointer()
                                    .px(px(8.))
                                    .py(px(3.))
                                    .rounded(px(5.))
                                    .border_1()
                                    .border_color(border_c)
                                    .text_size(px(11.))
                                    .text_color(ink)
                                    .on_click(cx.listener(move |v, _, _, cx| {
                                        if let Some(r) = v.deployments.get_mut(idx) {
                                            r.sleeping = !r.sleeping;
                                            cx.notify();
                                        }
                                    }))
                                    .child(if sleeping {
                                        tr(zh, "唤醒 (Wake)", "Wake")
                                    } else {
                                        tr(zh, "休眠 (Sleep)", "Sleep")
                                    }),
                            )
                            .child(
                                div()
                                    .id(("ks-play-row", idx))
                                    .cursor_pointer()
                                    .px(px(8.))
                                    .py(px(3.))
                                    .rounded(px(5.))
                                    .bg(rgba(0x0b57d01f))
                                    .text_size(px(11.))
                                    .text_color(if dark { rgb(0x60a5fa) } else { rgb(0x0b57d0) })
                                    .on_click(cx.listener(move |v, _, _, cx| {
                                        v.playground_idx = idx;
                                        v.page = NavPage::Playground;
                                        cx.notify();
                                    }))
                                    .child(tr(zh, "试验场 →", "Test /v1 →")),
                            ),
                    ),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(Self::page_banner(
                "=",
                tr(zh, "推理服务 / 部署列表", "Inference / Deployments"),
                tr(zh, "部署列表", "Deployments"),
                tr(
                    zh,
                    "集中查看模型服务、/v1 API 端点状态与 sleep/wake 轮换操作",
                    "Inspect active /v1 model deployments and toggle sleep/wake GPU sharing",
                ),
                Some(new_btn),
                dark,
            ))
            .child(
                div()
                    .rounded(px(8.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(border_c)
                    .overflow_hidden()
                    .child(
                        div()
                            .px(px(12.))
                            .py(px(8.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .text_size(px(11.))
                            .text_color(muted)
                            .child(div().w(px(120.)).child(tr(zh, "名称", "Name")))
                            .child(div().w(px(170.)).child(tr(zh, "模型", "Model")))
                            .child(div().w(px(130.)).child(tr(zh, "引擎 / 协议", "Engine")))
                            .child(div().w(px(110.)).child(tr(zh, "计算节点", "Node")))
                            .child(div().w(px(82.)).child(tr(zh, "状态", "Status")))
                            .child(div().child(tr(zh, "操作", "Actions"))),
                    )
                    .child(rows),
            )
    }

    fn render_secondary_page(
        &self,
        zh: bool,
        dark: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let card_bg = if dark { rgb(0x131822) } else { rgb(0xffffff) };
        let sub_bg = if dark { rgb(0x0d1118) } else { rgb(0xf8fafc) };
        let border_c = if dark { rgb(0x1f2937) } else { rgb(0xe5e7eb) };
        let muted = if dark { rgb(0x94a3b8) } else { rgb(0x64748b) };
        let ink = if dark { rgb(0xf8fafc) } else { rgb(0x0f172a) };

        match self.page {
            NavPage::Playground => {
                let dep = self
                    .deployments
                    .get(self.playground_idx)
                    .or_else(|| self.deployments.first());
                let model_name = dep
                    .map(|d| d.public_name.to_string())
                    .unwrap_or_else(|| "qwen3-32b".into());
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(Self::page_banner(
                        ">_",
                        tr(zh, "推理服务 / 试验场", "Inference / Playground"),
                        tr(zh, "试验场 (/v1/chat/completions)", "Playground (/v1/chat/completions)"),
                        tr(zh, "直连统一 OpenAI 兼容网关，实时检视首字延迟 TTFT 与吞吐", "Send live requests through KStack's /v1 gateway with token telemetry"),
                        None,
                        dark,
                    ))
                    .child(
                        div()
                            .p(px(14.))
                            .rounded(px(8.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(border_c)
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(
                                div()
                                    .font_family("Fira Code")
                                    .text_size(px(11.))
                                    .text_color(rgb(0x16a34a))
                                    .child(format!("POST /v1/chat/completions · model=\"{model_name}\" · TTFT 142ms · 156.4 tok/s")),
                            )
                            .child(
                                div()
                                    .p(px(10.))
                                    .rounded(px(6.))
                                    .bg(sub_bg)
                                    .text_size(px(12.))
                                    .text_color(ink)
                                    .child(tr(
                                        zh,
                                        "助手回复：已完成显存预算核对。单张 H800 80GiB 在开启 dGPU 切分后可同时承载 qwen3-32b (INT8) 与 bge-m3 向量服务。",
                                        "Assistant: VRAM placement verified. A single H800 80GiB with dGPU slicing serves both qwen3-32b and bge-m3 concurrently.",
                                    )),
                            ),
                    )
            }
            NavPage::Nodes => div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(Self::page_banner(
                    "#",
                    tr(zh, "资源 / 计算节点", "Resources / Compute Nodes"),
                    tr(zh, "计算节点 (1,003 台在线)", "Compute Nodes (1,003 Online)"),
                    tr(zh, "物理机、RunPod/阿里云实例与已有 Kubernetes 节点统一纳管", "Bare-metal, cloud VMs, and shared Kubernetes nodes in one pool"),
                    None,
                    dark,
                ))
                .child(
                    div()
                        .p(px(12.))
                        .rounded(px(8.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(border_c)
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .children(CHIPS.iter().map(|c| {
                            div()
                                .p(px(8.))
                                .rounded(px(6.))
                                .bg(sub_bg)
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .font_family("Fira Code")
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(ink)
                                        .child(c.node_name),
                                )
                                .child(div().text_size(px(11.5)).text_color(muted).child(tr(zh, c.zh_name, c.en_name)))
                                .child(div().text_size(px(11.)).text_color(rgb(0x16a34a)).child(tr(zh, "● 在线 · Agent WebSocket", "● Online · Agent WS")))
                        })),
                ),
            NavPage::ApiTokens => div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(Self::page_banner(
                    "#",
                    tr(zh, "平台 / API 令牌", "Platform / API Tokens"),
                    tr(zh, "API 令牌与用量配额", "API Tokens & Token Accounting"),
                    tr(zh, "签发 gw-... 推理密钥并按令牌统计输入/输出 token 消耗", "Issue gw-... gateway keys with per-token prompt/completion accounting"),
                    Some(
                        div()
                            .id("ks-new-token")
                            .cursor_pointer()
                            .px(px(10.))
                            .py(px(5.))
                            .rounded(px(6.))
                            .bg(rgb(0x0b57d0))
                            .text_size(px(11.5))
                            .text_color(rgb(0xffffff))
                            .on_click(cx.listener(|v, _, _, cx| {
                                v.token_count += 1;
                                cx.notify();
                            }))
                            .child(tr(zh, "+ 创建令牌", "+ Create Token"))
                            .into_any_element(),
                    ),
                    dark,
                ))
                .child(
                    div()
                        .p(px(12.))
                        .rounded(px(8.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(border_c)
                        .font_family("Fira Code")
                        .text_size(px(11.5))
                        .text_color(ink)
                        .child(format!(
                            "Active tokens: {} · gw-prod-kwork-9f2a... (14.8M tokens) · gw-ci-bench-41c0... (3.2M tokens)",
                            self.token_count
                        )),
                ),
            _ => div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(Self::page_banner(
                    "*",
                    tr(zh, "目录与实时监控", "Catalog & Live Telemetry"),
                    if self.page == NavPage::Models {
                        tr(zh, "392 个模型 · 1,235 份真机启动规格", "392 Models · 1,235 Verified Launch Specs")
                    } else {
                        tr(zh, "监控中心 · 单请求 OTLP 追踪", "Monitor Center · Per-Request OTLP Trace")
                    },
                    tr(zh, "点击左侧「部署」或「概览」可直接交互体验选卡与显存评估", "Select Deploy or Overview in the left sidebar to configure placements"),
                    None,
                    dark,
                ))
                .child(
                    div()
                        .p(px(12.))
                        .rounded(px(8.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(border_c)
                        .text_size(px(12.))
                        .text_color(muted)
                        .child(tr(
                            zh,
                            "覆盖 Qwen3、DeepSeek-R1、GLM-4.5、Kimi、Llama、Gemma、语音与多模态模型；每个规格均绑定真机验证过的镜像、引擎与启动参数。",
                            "Covers Qwen3, DeepSeek-R1, GLM-4.5, Kimi, Llama, Gemma, audio, and vision models with hardware-verified images and engine flags.",
                        )),
                ),
        }
    }
}

impl Render for KStackConsole {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let zh = is_zh(cx);
        let dark = is_dark(cx);
        let canvas_bg = if dark { rgb(0x0b0e14) } else { rgb(0xf3f5f9) };

        let body: AnyElement = match self.page {
            NavPage::Deploy => self.render_deploy_page(zh, dark, cx).into_any_element(),
            NavPage::Overview => self.render_overview_page(zh, dark).into_any_element(),
            NavPage::Deployments => self
                .render_deployments_page(zh, dark, cx)
                .into_any_element(),
            _ => self.render_secondary_page(zh, dark, cx).into_any_element(),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(canvas_bg)
            .child(self.render_topbar(zh, dark, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.render_sidebar(zh, dark, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .id("ks-stage-scroll")
                            .overflow_y_scroll()
                            .p(px(12.))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .when(self.ai_drawer, |el| {
                                el.child(
                                    div()
                                        .px(px(12.))
                                        .py(px(7.))
                                        .rounded(px(7.))
                                        .bg(rgba(0x0b57d020))
                                        .border_1()
                                        .border_color(rgb(0x0b57d0))
                                        .text_size(px(11.5))
                                        .text_color(if dark { rgb(0x93c5fd) } else { rgb(0x0b57d0) })
                                        .child(tr(
                                            zh,
                                            "* KStack AI 助手 (只读 MCP 已连接)：当前集群空闲 76 GiB H800 显存与 61 GiB Ascend 910C 显存，可直接一键部署 Qwen3-32B。",
                                            "* KStack AI Assistant (MCP ready): 76 GiB H800 and 61 GiB Ascend 910C VRAM available for immediate Qwen3-32B placement.",
                                        )),
                                )
                            })
                            .child(body),
                    ),
            )
    }
}
