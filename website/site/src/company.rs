//! Bilingual Kense homepage and product documentation hub.

#[path = "../company/svg.rs"]
mod svg;

use crate::{controls::button, esc};

pub const CSS: &str = include_str!("../company/company.css");

fn tr<'a>(en: bool, zh: &'a str, english: &'a str) -> &'a str {
    if en { english } else { zh }
}

fn path(en: bool) -> &'static str {
    if en { "/en" } else { "" }
}

fn klang_site(en: bool) -> &'static str {
    if en {
        "https://kense-labs.github.io/klang/"
    } else {
        "https://kense-labs.github.io/klang/README.zh-CN.html"
    }
}

fn component_count() -> usize {
    kui_catalog::items()
        .filter(|(id, _, _)| *id != "kui-foundation")
        .count()
}

fn icon(name: &str, size: u8) -> String {
    let paths = match name {
        "copy" => {
            r#"<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/>"#
        }
        "check" => r#"<path d="M5 12.5 9.5 17 19 7.5"/>"#,
        "terminal" => r#"<path d="M4 17 10 11 4 5M12 19h8"/>"#,
        "layers" => r#"<path d="m12 2 10 5-10 5L2 7l10-5Zm10 10-10 5-10-5m20 5-10 5-10-5"/>"#,
        "box" => {
            r#"<path d="m11 21-8-4.5v-9L11 3l8 4.5v9L11 21z"/><path d="M11 12 3 7.5M11 12l8-4.5M11 12v9"/>"#
        }
        "book" => {
            r#"<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>"#
        }
        "shield" => r#"<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>"#,
        "external" => r#"<path d="M7 17 17 7M7 7h10v10"/>"#,
        _ => r#"<path d="M5 12h14M13 5l7 7-7 7"/>"#,
    };
    format!(
        r#"<span class="kense-icon kense-icon-{name}" aria-hidden="true"><svg width="{size}" height="{size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">{paths}</svg></span>"#
    )
}

fn copy_mark(glyph: &str) -> String {
    format!(
        r#"<span class="kense-copy-mark">{}{}</span>"#,
        icon(glyph, 15),
        icon("check", 15)
    )
}

fn copy_cmd_button(
    en: bool,
    cmd: &str,
    caption: &str,
    hint: &str,
    body: &str,
    class: &str,
) -> String {
    let aria = format!("{} · {cmd}", tr(en, "复制命令", "Copy command"));
    button(
        &format!(
            r#"class="{class}" data-copy="{}" data-kind="command" data-spring-copy data-kense-hint="{}" data-kense-body="{}" aria-label="{}""#,
            esc(cmd),
            esc(hint),
            esc(body),
            esc(&aria),
        ),
        &format!("<code>{}</code>{}", esc(caption), copy_mark("terminal")),
    )
}

/// Interactive floating preview popover shown when hovering/focusing any `[data-product-badge]`.
fn product_popover(en: bool) -> String {
    let prefix = path(en);
    let count = component_count();
    let kui_href = format!("{prefix}/kui/");
    let kstack_href = format!("{prefix}/docs/#kstack");
    let kstudio_href = format!("{prefix}/docs/#kstudio");
    let kwork_href = format!("{prefix}/docs/#kwork");
    let mw_href = format!("{prefix}/docs/#middleware");

    let kui_summary = if en {
        format!(
            "One Rust + GPUI codebase compiled to macOS, Windows, Linux desktop and WebAssembly in the browser, with {count} components and page templates."
        )
    } else {
        format!(
            "同一份 Rust + GPUI 源码既跑在 macOS、Windows、Linux 原生桌面，也能编译成 WebAssembly 在浏览器里直接交互，共 {count} 个组件。"
        )
    };
    let kui_m1 = if en {
        format!("{count} components")
    } else {
        format!("{count} 个组件")
    };

    struct Spec<'a> {
        id: &'a str,
        name: &'a str,
        stack: &'a str,
        summary: &'a str,
        metrics: [&'a str; 3],
        cmd: Option<(&'a str, &'a str)>,
        note: &'a str,
        href: &'a str,
        external: bool,
        cta: &'a str,
    }

    let specs = [
        Spec {
            id: "kstack",
            name: "KStack",
            stack: tr(
                en,
                "40 MB · 算力调度与模型服务",
                "40 MB · GPU Scheduler & MaaS",
            ),
            summary: tr(
                en,
                "把自有机房、公有云主机、私有云虚拟机与已有 Kubernetes 集群并入同一个算力池，支持多厂商加速卡混部与一键模型推理部署。",
                "Pools bare-metal servers, cloud instances, and existing Kubernetes clusters across NVIDIA, AMD, Ascend, PPU, Hygon, and Apple Silicon.",
            ),
            metrics: [
                tr(en, "40 MB 控制面", "40 MB binary"),
                tr(en, "392 模型 · 1,235 规格", "392 models"),
                tr(en, "/v1 统一网关", "/v1 gateway"),
            ],
            cmd: None,
            note: tr(en, "核心产品 · 算力与模型", "Core product · Compute & MaaS"),
            href: &kstack_href,
            external: false,
            cta: tr(en, "查看 KStack 介绍", "About KStack"),
        },
        Spec {
            id: "kstudio",
            name: "KStudio",
            stack: tr(
                en,
                "macOS · Linux · Windows · GPUI",
                "macOS · Linux · Windows · GPUI",
            ),
            summary: tr(
                en,
                "把你名下所有工作站与服务器连进一个原生窗口，同时跑 18 种编程 Agent、真实终端、轻量虚拟机与远程桌面；关掉窗口后会话照跑。",
                "One native GPUI app for every machine you own: run 18 coding agents, persistent terminals, remote Linux desktops, and file sync.",
            ),
            metrics: [
                tr(en, "18 种 Agent CLI", "18 agent CLIs"),
                tr(en, "跨机终端与桌面", "Remote desktop & files"),
                tr(en, "关窗后台常驻", "Runs in background"),
            ],
            cmd: None,
            note: tr(en, "核心产品 · 多机终端", "Core product · Machine hub"),
            href: &kstudio_href,
            external: false,
            cta: tr(en, "查看 KStudio 介绍", "About KStudio"),
        },
        Spec {
            id: "kwork",
            name: "KWork",
            stack: tr(
                en,
                "Rust · KUI · GPUI · 桌面端",
                "Rust · KUI · GPUI · Desktop",
            ),
            summary: tr(
                en,
                "日常跑任务用的桌面 Agent 工作台。按项目目录整理对话与本地历史，随时切换模型与推理档位，查看每一步工具调用并支持定时运行。",
                "Desktop client for daily agent sessions, with workspace history, model switching, single-use tool approvals, and scheduled runs.",
            ),
            metrics: [
                tr(en, "工作区归档", "Workspaces"),
                tr(en, "单次权限确认", "Tool approvals"),
                tr(en, "定时自动投递", "Scheduled runs"),
            ],
            cmd: None,
            note: tr(en, "核心产品 · 桌面工作台", "Core product · Desktop client"),
            href: &kwork_href,
            external: false,
            cta: tr(en, "查看 KWork 介绍", "About KWork"),
        },
        Spec {
            id: "kui",
            name: "KUI",
            stack: "Rust · GPUI =0.3.10 · Desktop + WASM",
            summary: &kui_summary,
            metrics: [kui_m1.as_str(), "Desktop + Web", "kui add <id>"],
            cmd: Some((
                "cargo add kense-kui --rename kui --no-default-features",
                "cargo add kense-kui --rename kui …",
            )),
            note: "",
            href: &kui_href,
            external: false,
            cta: tr(en, "进入 KUI 组件站", "Open KUI"),
        },
        Spec {
            id: "klang",
            name: tr(en, "K 语言", "K Language"),
            stack: "klang · .k · Native + WASI + TinyCC",
            summary: tr(
                en,
                "内存安全的系统编程语言，编译期检查引用生命周期并在作用域退出时回收数据栈存储；一套前端支持本机可执行文件、WASI 模块与 TinyCC 进程内运行。",
                "Memory-safe systems language with checked reference lifetimes, scope-exit data stacks, Cordis services, and native, WASI, and TinyCC targets.",
            ),
            metrics: [
                tr(en, "生命周期检查", "Checked lifetimes"),
                "Native + WASI",
                tr(en, "Cordis 微内核", "Cordis kernel"),
            ],
            cmd: Some((
                "curl -fsSL https://raw.githubusercontent.com/kense-labs/klang/main/scripts/install.sh | sh",
                "curl -fsSL …/klang/…/install.sh | sh",
            )),
            note: "",
            href: klang_site(en),
            external: false,
            cta: tr(en, "打开 K 语言站点", "Open K Language"),
        },
        Spec {
            id: "kbox",
            name: "KBox",
            stack: tr(en, "中间件 · OCI 与 MicroVM", "Middleware · OCI & MicroVM"),
            summary: tr(
                en,
                "底层容器与 libkrun 轻量虚拟机运行时，支持预热池快速挂载，供 KStudio 等上层应用按需调用。",
                "Container and libkrun MicroVM runtime middleware used underneath KStudio for isolated execution.",
            ),
            metrics: ["OCI", "libkrun MicroVM", "Warm pool"],
            cmd: None,
            note: tr(en, "底层中间件", "Runtime middleware"),
            href: &mw_href,
            external: false,
            cta: tr(en, "查看中间件说明", "Middleware notes"),
        },
        Spec {
            id: "ksandbox",
            name: "KSandbox",
            stack: tr(en, "中间件 · 进程沙箱", "Middleware · Process sandbox"),
            summary: tr(
                en,
                "基于 Seatbelt、Bubblewrap+seccomp 与 AppContainer 的轻量进程沙箱，限制工具子进程的目录读写与网络连接。",
                "Lightweight OS process sandbox restricting filesystem and network access via Seatbelt, seccomp, or AppContainer.",
            ),
            metrics: ["Seatbelt", "seccomp", "AppContainer"],
            cmd: None,
            note: tr(en, "底层中间件", "Runtime middleware"),
            href: &mw_href,
            external: false,
            cta: tr(en, "查看中间件说明", "Middleware notes"),
        },
    ];

    let mut cards = String::new();
    for s in specs {
        let ext = if s.external {
            r#" target="_blank" rel="noreferrer""#
        } else {
            ""
        };
        let left_foot = if let Some((cmd, cmd_short)) = s.cmd {
            copy_cmd_button(
                en,
                cmd,
                cmd_short,
                tr(en, "点击复制安装命令", "Click to copy command"),
                "",
                "kense-pop-copy",
            )
        } else {
            format!(r#"<span class="kense-pop-note">{}</span>"#, s.note)
        };
        cards.push_str(&format!(
            r#"<div class="kense-pop-card" data-popover-card="{id}" hidden>
  <div class="kense-pop-head" data-pop-domain="{id}">
    <span class="kense-pop-emblem">{emblem}</span>
    <div class="kense-pop-titles">
      <strong>{name}</strong>
      <span>{stack}</span>
    </div>
  </div>
  <p class="kense-pop-summary">{summary}</p>
  <div class="kense-pop-metrics"><span>{m0}</span><span>{m1}</span><span>{m2}</span></div>
  <div class="kense-pop-foot">{left_foot}<a class="kense-pop-link" href="{href}"{ext}>{cta}</a></div>
</div>"#,
            id = s.id,
            emblem = svg::product_emblem(s.id),
            name = s.name,
            stack = s.stack,
            summary = s.summary,
            m0 = s.metrics[0],
            m1 = s.metrics[1],
            m2 = s.metrics[2],
            href = s.href,
            cta = svg::action_content(s.cta),
        ));
    }

    format!(
        r#"<div class="kense-product-popover" id="kense-product-popover" data-kense-popover role="region" aria-label="{}" inert aria-hidden="true"><div class="kense-pop-arrow" aria-hidden="true"></div>{cards}</div>"#,
        tr(en, "产品概览", "Product overview")
    )
}

/// 3D perspective flip-card tooltip (`#kense-hero-hint`) anchored above the Hero inline product belt.
fn hero_flip_hint(en: bool) -> String {
    let count = component_count();
    let kui_desc = if en {
        format!(
            "{count} GPUI components sharing one Rust codebase across macOS, Windows, Linux desktop and WebAssembly."
        )
    } else {
        format!(
            "同一套 Rust 源码编译到 macOS、Windows、Linux 原生桌面与浏览器 WebAssembly，内置 {count} 个组件。"
        )
    };
    let entries = [
        (
            "kstack",
            "KStack",
            tr(en, "算力调度与模型服务", "40 MB · GPU Pool & MaaS"),
            tr(
                en,
                "把自有服务器与多云 GPU 接进同一个集群，启动前算清显存与选卡理由，一键部署推理服务。",
                "Pools bare-metal and cloud GPUs into one scheduler with upfront VRAM estimates and 1-click model serving.",
            ),
        ),
        (
            "kstudio",
            "KStudio",
            tr(en, "多机终端与 Agent 管理", "macOS · Linux · Windows"),
            tr(
                en,
                "连上你手头所有工作站与服务器，在一个原生窗口里跑 18 种编程 Agent、常驻终端与远程桌面。",
                "Connects every machine you own into one GPUI window for 18 agent CLIs, persistent shells, and remote desktops.",
            ),
        ),
        (
            "kwork",
            "KWork",
            tr(en, "桌面 Agent 工作台", "Workspaces · Schedules"),
            tr(
                en,
                "按项目工作区整理对话与历史，随时切换模型与推理档位，查看每一步工具调用并支持定时任务。",
                "Organizes sessions by workspace with model switching, single-use tool approvals, and scheduled runs.",
            ),
        ),
        (
            "kui",
            "KUI",
            tr(en, "Rust + GPUI 组件库", "Desktop + WebAssembly"),
            kui_desc.as_str(),
        ),
        (
            "klang",
            tr(en, "K 语言", "K Language"),
            tr(
                en,
                "系统编程 · Native & WASI",
                ".k · Native · WASI · TinyCC",
            ),
            tr(
                en,
                "编译期检查引用生命周期，在地址稳定的数据栈上管理可增长值；共享检查器生成本机程序、WASI 模块或经 TinyCC 即时运行。",
                "Checks reference lifetimes at compile time with address-stable data stacks, building native binaries, WASI modules, or running via TinyCC.",
            ),
        ),
    ];

    let mut cards = String::new();
    for (idx, (domain, title, badge, copy)) in entries.into_iter().enumerate() {
        let hidden = if idx == 0 { "" } else { " hidden" };
        cards.push_str(&format!(
            r#"<div class="kense-hero-hint-card" data-hint-card="{domain}"{hidden}>
  <div class="kense-hero-hint-head" data-hint-domain="{domain}">
    <span class="kense-hero-hint-emblem">{emblem}</span>
    <div class="kense-hero-hint-titles">
      <p class="kense-hero-hint-title">{title}</p>
      <span class="kense-hero-hint-sub">{badge}</span>
    </div>
  </div>
  <p class="kense-hero-hint-copy">{copy}</p>
</div>"#,
            emblem = svg::product_emblem(domain),
        ));
    }

    format!(
        r#"<div class="kense-hero-hint" id="kense-hero-hint" data-hero-hint role="tooltip" aria-hidden="true">{cards}</div>"#
    )
}

/// Inline enameled product belt inside the Hero `<h1>` (`[data-hero-belt-region]`).
fn hero_solution_belt(en: bool, prefix: &str) -> String {
    let domains = [
        (
            "kstack",
            format!("{prefix}/#products-kstack"),
            false,
            tr(en, "KStack 模型算力", "KStack Compute"),
            "KStack",
            tr(
                en,
                "40 MB 异构 GPU 算力调度与一键模型推理平台",
                "40 MB heterogeneous GPU scheduler and /v1 model serving",
            ),
        ),
        (
            "kstudio",
            format!("{prefix}/#products-kstudio"),
            false,
            tr(en, "KStudio 多机终端", "KStudio Hub"),
            "KStudio",
            tr(
                en,
                "在一个窗口里连上名下所有机器、终端与 18 种编程 Agent",
                "One native GPUI app for all your machines and agents",
            ),
        ),
        (
            "kwork",
            format!("{prefix}/#products-kwork"),
            false,
            tr(en, "KWork 桌面工作台", "KWork Desktop"),
            "KWork",
            tr(
                en,
                "按工作区整理会话、单次工具确认与定时自动投递",
                "Daily desktop agent client with tool approvals & schedules",
            ),
        ),
        (
            "kui",
            format!("{prefix}/kui/"),
            false,
            tr(en, "KUI 跨端界面", "KUI Interface"),
            "KUI",
            tr(
                en,
                "一套 Rust + GPUI 源码构建桌面与浏览器界面",
                "Shared Rust + GPUI component library for desktop and web",
            ),
        ),
        (
            "klang",
            format!("{prefix}/#products-klang"),
            false,
            tr(en, "K 系统编程语言", "K Language"),
            tr(en, "K 语言", "K Language"),
            tr(
                en,
                "编译期引用生命周期检查与数据栈内存安全的系统编程语言",
                "Memory-safe systems language with checked lifetimes & C/WASI targets",
            ),
        ),
    ];

    let mut belt_items = String::new();
    for (idx, (id, href, external, phrase, prod, hint)) in domains.iter().enumerate() {
        let active = if idx == 0 { " data-active" } else { "" };
        let ext = if *external { "1" } else { "0" };
        belt_items.push_str(&format!(
            r#"<span class="kense-belt-word" data-belt-item="{id}" data-belt-index="{idx}" data-belt-href="{href}" data-belt-ext="{ext}" data-belt-prod="{prod}" data-belt-hint="{hint}" style="--belt-row:{idx}"{active}><span class="kense-belt-inner"><span class="kense-belt-logo">{emblem}</span><span class="kense-belt-phrase">{phrase}</span></span></span>"#,
            emblem = svg::product_emblem(id),
            hint = esc(hint),
        ));
    }

    let first = &domains[0];
    format!(
        r#"<span class="kense-belt-region" data-hero-belt-region>
  <a class="kense-belt-anchor" data-hero-solution-word data-domain="{id}" href="{href}" aria-label="{phrase} · {prod}" data-kense-hint="{hint}">
    <span class="kense-belt-inner">
      <span class="kense-belt-logo">{emblem}</span>
      <span class="kense-belt-phrase">{phrase}</span>
    </span>
  </a>
  <span class="kense-belt-drum" data-hero-belt aria-hidden="true">{belt_items}</span>
</span>"#,
        id = first.0,
        href = first.1,
        phrase = first.3,
        prod = first.4,
        hint = esc(first.5),
        emblem = svg::product_emblem(first.0),
    )
}

/// Bottom-viewport WebGL Liquid-Meniscus Command Dock (`[data-command-dock]`).
fn hero_command_dock(en: bool) -> String {
    let prompt = tr(
        en,
        "请阅读 https://kense-labs.github.io/llms.txt，检查当前项目的 Rust 与 GPUI 配置，并按说明接入所需的 KUI 组件。",
        "Read https://kense-labs.github.io/llms.txt, inspect this project's Rust and GPUI setup, and integrate the needed KUI components.",
    );
    let label = tr(
        en,
        "复制 KUI Agent 接入提示词",
        "Copy KUI coding-agent prompt",
    );
    let dock_btn = button(
        &format!(
            r#"class="kense-dock-btn" data-dock-button data-copy="{}" data-kind="prompt" data-kense-hint="{}" aria-label="{}""#,
            esc(prompt),
            esc(tr(
                en,
                "点击复制提示词，发给编程 Agent 自动接入 KUI 组件",
                "Click to copy prompt for your coding agent"
            )),
            esc(label),
        ),
        &format!(
            r#"<canvas class="kense-dock-shape" data-dock-shape aria-hidden="true"></canvas>
<span class="kense-dock-tag"><svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 2.5 14.7 9.3 21.5 12l-6.8 2.7L12 21.5l-2.7-6.8L2.5 12l6.8-2.7L12 2.5Z"/></svg><span>AGENT</span></span>
<code class="kense-dock-cmd">{}</code>
<span class="kense-dock-icons" aria-hidden="true">
  <svg class="kense-dock-copy" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/></svg>
  <svg class="kense-dock-check" width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5 9.5 17 19 7.5"/></svg>
</span>"#,
            tr(
                en,
                "发给编程 Agent · 自动读取 kense-labs.github.io/llms.txt",
                "Paste to your Agent · reads kense-labs.github.io/llms.txt"
            )
        ),
    );
    format!(
        r#"<div class="kense-command-dock" data-command-dock>{dock_btn}<span class="sr-only" data-dock-status aria-live="polite"></span></div>"#
    )
}

/// Unified in-place morphing companion bot (`#kense-guide`) with silky comet-ribbon flight trail.
fn guide_widget(en: bool) -> String {
    let prefix = path(en);
    let count = component_count();
    let ui_tag = format!("{count} UI");
    let open = tr(en, "打开 Kense 向导", "Open Kense guide");
    let close = tr(en, "收起 Kense 向导", "Close Kense guide");
    let name = tr(en, "Kense 向导", "Kense guide");

    let items = [
        (
            format!("{prefix}/#products"),
            "box",
            tr(en, "浏览五款核心产品", "Core Products"),
            "K · KStack",
        ),
        (
            format!("{prefix}/#experience"),
            "terminal",
            tr(en, "在线点按体验 GPUI 面板", "Live GPUI Demo"),
            "WASM",
        ),
        (
            format!("{prefix}/#architecture"),
            "layers",
            tr(en, "五款产品怎么配合", "How They Fit"),
            "Stack",
        ),
        (
            format!("{prefix}/kui/"),
            "shield",
            tr(en, "进入 KUI 跨端组件站", "Open KUI"),
            ui_tag.as_str(),
        ),
        (
            format!("{prefix}/docs/"),
            "book",
            tr(en, "查看产品与技术文档", "Documentation"),
            "Docs",
        ),
    ];

    let mut links = String::new();
    for (idx, (href, glyph, label, tag)) in items.iter().enumerate() {
        links.push_str(&format!(
            r#"<a href="{href}" style="--choice-delay:{}ms"><span class="kense-choice-bracket" aria-hidden="true">&gt;</span>{}<span class="kense-choice-text">{label}</span><span class="kense-choice-tag">{tag}</span></a>"#,
            65 + idx * 24,
            icon(glyph, 15),
        ));
    }

    let trigger = button(
        &format!(
            r#"class="kense-guide-trigger" aria-expanded="false" aria-controls="kense-guide-choices" aria-label="{open}" data-open-label="{open}" data-close-label="{close}""#
        ),
        &format!(
            r#"<span class="kense-guide-gaze" aria-hidden="true"><span class="kense-guide-eyes"><i></i><i></i></span></span><span class="kense-guide-name" aria-hidden="true">{name}</span><svg class="kense-guide-cross" width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="m4 4 8 8m0-8-8 8" stroke="currentColor" stroke-width="1.45" stroke-linecap="round"/></svg>"#
        ),
    );

    let speech_copy = button(
        r#"class="kense-guide-copy" hidden data-kind="command" data-spring-copy"#,
        &format!(
            "{}<span>{}</span>",
            copy_mark("copy"),
            tr(en, "复制命令", "Copy")
        ),
    );

    format!(
        r#"<canvas class="kense-guide-trail" data-guide-trail aria-hidden="true"></canvas>
<div class="kense-guide" id="kense-guide" data-copy-cmd-msg="{msg_cmd}" data-copy-prompt-msg="{msg_prompt}">
  <div class="kense-guide-shell">
    <svg class="kense-guide-ink" aria-hidden="true" focusable="false">
      <rect width="100%" height="100%" rx="10" fill="currentColor"/>
      <rect class="kense-guide-rim" x="0.5" y="0.5" width="calc(100% - 1px)" height="calc(100% - 1px)" rx="9.5" fill="none"/>
      <svg x="49%" y="100%" width="1" height="1" overflow="visible">
        <path class="kense-guide-tail" fill="currentColor" d="M-7-1H10v1H9q-1.3 0-2.4 1.1l-11 10.4q-2.1 1.9-1.7-1.3l1.2-8.8Q-4.7 0-6.3 0H-7Z"/>
      </svg>
    </svg>
    {trigger}
    <div class="kense-guide-speech" hidden><strong class="kense-speech-title"></strong><p class="kense-speech-body" hidden></p>{speech_copy}</div>
    <nav class="kense-guide-choices" id="kense-guide-choices" aria-label="{name}" hidden>{links}</nav>
  </div>
</div>"#,
        msg_cmd = tr(
            en,
            "命令已复制 · 可直接粘贴到终端运行",
            "Command copied · ready in terminal"
        ),
        msg_prompt = tr(
            en,
            "已复制 · 直接粘贴给你的编程 Agent",
            "Copied · paste directly into your coding agent"
        ),
    )
}

fn docs_product_strip(en: bool, prefix: &str) -> String {
    let items = [
        ("kstack", "KStack", format!("{prefix}/docs/#kstack")),
        ("kstudio", "KStudio", format!("{prefix}/docs/#kstudio")),
        ("kwork", "KWork", format!("{prefix}/docs/#kwork")),
        ("kui", "KUI", format!("{prefix}/kui/")),
        (
            "klang",
            tr(en, "K 语言", "K Language"),
            format!("{prefix}/docs/#klang"),
        ),
    ];

    let mut out = String::from(
        r#"<div class="kense-product-strip" role="list" aria-label="Kense core products">"#,
    );
    for (id, label, href) in items {
        out.push_str(&format!(
            r#"<a class="kense-product-badge" role="listitem" href="{href}" data-product-badge="{id}">{}<span>{label}</span></a>"#,
            svg::product_emblem(id)
        ));
    }
    out.push_str("</div>");
    out
}

/// Renders the complete Kense flagship homepage (`/` and `/en/`).
pub fn page(en: bool, rev: &str) -> String {
    let prefix = path(en);
    let count = component_count();
    let locale = if en { "en" } else { "zh" };
    let gpui_src = format!(
        "/stories/v/{}/?story=kense-gateway&amp;lang={locale}",
        esc(rev)
    );
    let kui_cmd = "cargo add kense-kui --rename kui --no-default-features";

    // Section 1: Hero + Inline 4-Product Spring Belt + Heat-Flow Shader CTA + Command Dock
    let hero = format!(
        r##"<section class="kense-hero" data-kense-hero aria-labelledby="kense-hero-title">
  <div class="kense-hero-optics" data-kense-optics aria-hidden="true">
    <div class="kense-optics-glow"></div>
    <canvas class="kense-hero-floor" data-hero-floor width="160" height="96" hidden></canvas>
  </div>
  <div class="kense-intro" data-kense-intro data-intro-key="kense:home-intro:v1" hidden aria-hidden="true">
    <div class="kense-intro-backdrop"></div>
    <div class="kense-intro-word kense-intro-word--top" data-intro-word>KENSE</div>
    <div class="kense-intro-word kense-intro-word--bottom" data-intro-word data-intro-direction="right">STUDIO</div>
    <div class="kense-intro-stage">{draft}</div>
  </div>
  <div class="kense-hero-stage">
    <div class="kense-guide-home" data-kense-guide-home aria-hidden="true"></div>
    <h1 class="kense-hero-solution" id="kense-hero-title" data-intro-title>
      <span class="kense-solution-prefix">{h_prefix}</span>
      {belt}
      <span class="kense-solution-suffix">{h_suffix}</span>
    </h1>
    <p class="kense-hero-lead" data-intro-name>{lead}</p>
    <div class="kense-hero-actions">
      <a class="kense-pill kense-pill--heat" data-heat-flow href="{prefix}/#products" data-kense-hint="{h1_hint}" data-kense-body="{h1_body}">
        <canvas class="kense-heat-canvas" aria-hidden="true"></canvas>
        <span class="kense-pill-content">{explore}</span>
      </a>
      <a class="kense-pill kense-pill--secondary" href="{prefix}/kui/" data-product-badge="kui" data-kense-hint="{h2_hint}">{open_kui}</a>
      <a class="kense-pill kense-pill--ghost" href="{prefix}/docs/" data-kense-hint="{h3_hint}">{read_docs}</a>
    </div>
  </div>
  {dock}
  {flip_hint}
</section>"##,
        draft = svg::intro_draft(),
        h_prefix = tr(
            en,
            "从算力集群到桌面端的",
            "From GPU clusters to your desk,"
        ),
        belt = hero_solution_belt(en, prefix),
        h_suffix = tr(en, "自研软件栈", "built to work together"),
        lead = tr(
            en,
            "K 语言用编译期生命周期检查构建系统与推理组件，KStack 调度多云异构 GPU 与模型服务，KStudio 接管你名下的每一台机器，KWork 处理日常桌面任务，KUI 用同一份 Rust + GPUI 源码构建原生与网页界面。",
            "K builds memory-safe systems and inference components, KStack schedules GPUs and serves models across clouds, KStudio manages every machine you own in one window, KWork handles daily desktop tasks, and KUI builds native and web UI from one Rust + GPUI codebase."
        ),
        explore = svg::action_content(tr(en, "浏览核心产品", "Explore products")),
        open_kui = tr(en, "进入 KUI 组件站", "Open KUI"),
        read_docs = tr(en, "产品文档", "Documentation"),
        h1_hint = tr(
            en,
            "向下查看 KStack、KStudio、KWork、KUI 与 K 语言",
            "Inspect KStack, KStudio, KWork, KUI, and K"
        ),
        h1_body = tr(
            en,
            "可拖拽旋转每款产品的三维模型，鼠标移上去时小 Bot 会跳进模型。",
            "Drag any 3D model to orbit. Hovering sends the companion bot straight into the model."
        ),
        h2_hint = if en {
            format!("Open KUI: {count} desktop & WebAssembly GPUI controls")
        } else {
            format!("打开 KUI：{count} 个跨桌面与 Web 的 GPUI 组件")
        },
        h3_hint = tr(
            en,
            "查看五款核心产品与底层中间件文档",
            "Browse product guides and middleware notes"
        ),
        dock = hero_command_dock(en),
        flip_hint = hero_flip_hint(en),
    );

    // Section 2: Four Core Products (#products) + Understated Runtime Middleware Note (#middleware)
    let family = format!(
        r##"<section class="kense-section kense-opening-anchor" id="products" aria-labelledby="kense-family-title">
  <div class="kense-opening-backdrop" aria-hidden="true"></div>
  <div class="kense-shell">
    <div class="kense-section-head kense-section-head--center kense-reveal">
      <h2 id="kense-family-title">{title}</h2>
      <p>{desc}</p>
    </div>
    <div class="kense-family">
      <article class="kense-family-item" id="products-kstack" data-family-berth="kstack" data-solid-entrance="pending">
        <div class="kense-solid" data-kense-solid="runtime" aria-hidden="true">{f_runtime}<canvas width="760" height="570"></canvas></div>
        <div class="kense-family-copy kense-reveal">
          <h3>{h_kstack}</h3>
          <p>{p_kstack}</p>
          <p class="kense-family-detail">{d_kstack}</p>
          <div class="kense-family-links">
            <a class="kense-pill kense-pill--outline" href="{prefix}/docs/#kstack" data-product-badge="kstack" data-kense-hint="{hint_kstack}">{act_kstack}</a>
            <a class="kense-inline-link" href="{prefix}/#experience" data-kense-hint="{hint_kstack_demo}">{act_kstack_demo}</a>
          </div>
        </div>
      </article>
      <article class="kense-family-item kense-family-item--reverse" id="products-kstudio" data-family-berth="kstudio" data-solid-entrance="pending">
        <div class="kense-solid" data-kense-solid="studio" aria-hidden="true">{f_studio}<canvas width="760" height="570"></canvas></div>
        <div class="kense-family-copy kense-reveal">
          <h3>{h_kstudio}</h3>
          <p>{p_kstudio}</p>
          <p class="kense-family-detail">{d_kstudio}</p>
          <div class="kense-family-links">
            <a class="kense-pill kense-pill--outline" href="{prefix}/docs/#kstudio" data-product-badge="kstudio" data-kense-hint="{hint_kstudio}">{act_kstudio}</a>
            <a class="kense-inline-link" href="{prefix}/#experience-sandbox" data-kense-hint="{hint_kstudio_demo}">{act_kstudio_demo}</a>
          </div>
        </div>
      </article>
      <article class="kense-family-item" id="products-kwork" data-family-berth="kwork" data-solid-entrance="pending">
        <div class="kense-solid" data-kense-solid="agent" aria-hidden="true">{f_agent}<canvas width="760" height="570"></canvas></div>
        <div class="kense-family-copy kense-reveal">
          <h3>{h_kwork}</h3>
          <p>{p_kwork}</p>
          <p class="kense-family-detail">{d_kwork}</p>
          <div class="kense-family-links">
            <a class="kense-pill kense-pill--outline" href="{prefix}/docs/#kwork" data-product-badge="kwork" data-kense-hint="{hint_kwork}">{act_kwork}</a>
            <a class="kense-inline-link" href="{prefix}/#experience-interface" data-kense-hint="{hint_kwork_demo}">{act_kwork_demo}</a>
          </div>
        </div>
      </article>
      <article class="kense-family-item kense-family-item--reverse" id="products-kui" data-family-berth="kui" data-solid-entrance="pending">
        <div class="kense-solid" data-kense-solid="ui" aria-hidden="true">{f_ui}<canvas width="760" height="570"></canvas></div>
        <div class="kense-family-copy kense-reveal">
          <h3>{h_kui}</h3>
          <p>{p_kui}</p>
          <p class="kense-family-detail">{d_kui}</p>
          <div class="kense-family-links">
            <a class="kense-pill kense-pill--outline" href="{prefix}/kui/" data-product-badge="kui" data-kense-hint="{hint_kui}">{act_kui}</a>
            <a class="kense-inline-link" href="{prefix}/kui/components/button/" data-kense-hint="{hint_comp}">{act_comp}</a>
            <a class="kense-inline-link" href="{prefix}/kui/docs/installation/" data-kense-hint="{hint_install}">{act_install}</a>
          </div>
        </div>
      </article>
      <article class="kense-family-item" id="products-klang" data-family-berth="klang" data-solid-entrance="pending">
        <div class="kense-solid" data-kense-solid="klang" aria-hidden="true">{f_klang}<canvas width="760" height="570"></canvas></div>
        <div class="kense-family-copy kense-reveal">
          <h3>{h_klang}</h3>
          <p>{p_klang}</p>
          <p class="kense-family-detail">{d_klang}</p>
          <div class="kense-family-links">
            <a class="kense-pill kense-pill--outline" href="{klang_url}" data-product-badge="klang" data-kense-hint="{hint_klang}">{act_klang}</a>
            <a class="kense-inline-link" href="{prefix}/#experience-klang" data-kense-hint="{hint_klang_demo}">{act_klang_demo}</a>
            <a class="kense-inline-link" href="https://github.com/kense-labs/klang" target="_blank" rel="noreferrer" data-kense-hint="{hint_klang_gh}">GitHub</a>
          </div>
        </div>
      </article>
    </div>
    <div class="kense-middleware kense-reveal" id="middleware">
      <div class="kense-middleware-head">
        <h3>{mw_title}</h3>
        <p>{mw_sub}</p>
      </div>
      <div class="kense-middleware-grid">
        <div class="kense-middleware-item">
          <h4>{ico_kbox}<span>KBox</span></h4>
          <p>{mw_kbox}</p>
        </div>
        <div class="kense-middleware-item">
          <h4>{ico_ksb}<span>KSandbox</span></h4>
          <p>{mw_ksb}</p>
        </div>
      </div>
    </div>
  </div>
</section>"##,
        title = tr(en, "五款核心产品", "Five core products"),
        desc = tr(
            en,
            "每款产品都能单独拿来解决具体问题；放在一起时，从底层系统编程语言与异构 GPU 集群一直连到你桌上的原生窗口。",
            "Each product solves a concrete job on its own, and fits cleanly with the others from systems code and rack-scale GPUs to your desktop."
        ),
        f_runtime = svg::solid_fallback("runtime"),
        h_kstack = tr(
            en,
            "KStack · 算力调度与模型服务平台",
            "KStack · GPU Scheduling & Model Serving"
        ),
        p_kstack = tr(
            en,
            "整个服务端连同中英双语 Web 控制台只有 40 MB 左右。一条命令就能把自有机房服务器、租用的公有云实例、私有云虚拟机以及已有 Kubernetes 集群并进同一个算力池；NVIDIA、AMD、昇腾、平头哥 PPU、海光与 Apple Silicon 可以在同一集群里混合调度，用卡时借出、空闲时归还。",
            "A single ~40 MB binary with an embedded bilingual web console. Pool bare-metal GPU servers, cloud instances, private VMs, and existing Kubernetes clusters together across NVIDIA, AMD, Ascend, PPU, Hygon, and Apple Silicon."
        ),
        d_kstack = tr(
            en,
            "内置 392 个模型、1,235 份真机跑通的启动配置。启动前先算清权重与 KV cache 显存占用、说明选卡原因；支持单卡多模型切分（dGPU）、多机张量并行与 PD 分离，统一提供兼容 OpenAI 的 /v1 接口。",
            "Ships 1,235 hardware-tested launch specs across 392 models. Estimates VRAM before launch, explains GPU selection, and supports single-card dGPU sharing, multi-node parallelism, and an OpenAI-compatible /v1 gateway."
        ),
        act_kstack = svg::action_content(tr(en, "了解 KStack", "About KStack")),
        hint_kstack = tr(
            en,
            "KStack：单机实测纳管跨 7 地域 1,000+ 台 GPU 节点",
            "KStack: tested across 1,000+ GPU nodes in 7 regions"
        ),
        act_kstack_demo = tr(en, "体验选卡面板", "Try placement demo"),
        hint_kstack_demo = tr(
            en,
            "在下方 GPUI 面板中查看显存估算与选卡过程",
            "Test VRAM estimation and GPU selection below"
        ),
        f_studio = svg::solid_fallback("studio"),
        h_kstudio = tr(
            en,
            "KStudio · 多机终端与 Agent 客户端",
            "KStudio · Multi-Machine & Agent Hub"
        ),
        p_kstudio = tr(
            en,
            "连上你手头所有的机器（桌下的 Mac Studio、机柜里的 Linux 服务器、几台笔记本或云主机），在一个 GPUI 原生窗口里一起管起来。支持在任意机器上运行 18 种编程 Agent CLI 与真实终端，直接复用你现有的 ssh 配置；就算关掉主窗口，连接、端口转发和后台任务依然继续运行。",
            "Point it at every machine you own: a Mac Studio under the desk, Linux servers, cloud boxes, or laptops. Built natively with GPUI to run 18 coding agent CLIs and persistent terminals over your existing SSH setup, staying alive in the background when the window closes."
        ),
        d_kstudio = tr(
            en,
            "可以在终端旁直接查看并操作远程 Linux 桌面，在任意两台机器之间拖传文件与共享剪贴板，查看自建 Runner 负载，或把 GitHub Issue 一键交给选定机器上的 Agent 在独立环境里处理。",
            "View live Linux desktops beside terminals, copy files and clipboards directly between any two machines, monitor self-hosted runners, or hand a GitHub issue to an agent in its own isolated box."
        ),
        act_kstudio = svg::action_content(tr(en, "了解 KStudio", "About KStudio")),
        hint_kstudio = tr(
            en,
            "KStudio：支持 macOS、Linux 与 Windows 原生客户端",
            "KStudio: native GPUI app on macOS, Linux, and Windows"
        ),
        act_kstudio_demo = tr(en, "查看多机隔离演示", "See machine isolation"),
        hint_kstudio_demo = tr(
            en,
            "查看下方 GPUI 面板中的远程沙箱与虚拟机运行状态",
            "Inspect remote box execution in the live GPUI panel"
        ),
        f_agent = svg::solid_fallback("agent"),
        h_kwork = tr(
            en,
            "KWork · 桌面 Agent 工作台",
            "KWork · Desktop Agent Workbench"
        ),
        p_kwork = tr(
            en,
            "日常跑任务、写代码用的桌面 Agent 客户端，界面用 KUI 与 GPUI 写成。按本地项目目录归档对话与历史，可接入 DeepSeek、OpenAI、OpenRouter 或本地模型服务，在输入框旁随时切换模型与推理档位。",
            "A desktop client for everyday agent tasks, built with KUI and GPUI. Organize sessions by workspace, connect DeepSeek, OpenAI, OpenRouter, or local endpoints, and switch models or reasoning effort inline."
        ),
        d_kwork = tr(
            en,
            "任务执行过程中的每一次工具调用与返回结果都一目了然，涉及写入或执行命令时弹出单次确认；周期性的巡检、整理与构建可以直接设为定时任务，跑完自动投递回会话。",
            "Inspect every tool call and output in real time, approve sensitive actions one by one, and turn recurring checks or builds into scheduled tasks delivered straight back to your workspace."
        ),
        act_kwork = svg::action_content(tr(en, "了解 KWork", "About KWork")),
        hint_kwork = tr(
            en,
            "KWork：工作区会话归档、单次授权确认与定时任务",
            "KWork: workspace sessions, tool approvals & schedules"
        ),
        act_kwork_demo = tr(en, "体验授权交互", "Try approval flow"),
        hint_kwork_demo = tr(
            en,
            "在下方 GPUI 面板中体验工具单次确认流程",
            "Test single-use tool approvals in the GPUI panel"
        ),
        f_ui = svg::solid_fallback("ui"),
        h_kui = tr(
            en,
            "KUI · 跨桌面与 Web 的 GPUI 组件库",
            "KUI · Cross-Platform GPUI Component Library"
        ),
        p_kui = tr(
            en,
            "一套 Rust 组件源码，同时编译运行在 macOS、Windows、Linux 原生桌面与浏览器 WebAssembly 中，不用把同一套界面用前端框架重写第二遍。底层交互状态与视觉主题清晰分开，网页上点按预览的就是实际装进工程里的代码。",
            "One Rust component codebase running natively on macOS, Windows, and Linux as well as WebAssembly in the browser, without maintaining a separate web UI fork. What you interact with in the gallery is the exact source you ship."
        ),
        d_kui = if en {
            format!(
                "Includes {count} components and full application screen templates. Add kense-kui via Cargo or copy individual component source into your repo with kui add."
            )
        } else {
            format!(
                "内置 {count} 个基础控件、复杂交互组件与完整应用页面模板。既可以通过 Cargo 作为库引入，也可以用 kui add 把所需源码直接拷进仓库自由定制。"
            )
        },
        act_kui = svg::action_content(tr(en, "进入 KUI 组件站", "Open KUI")),
        act_comp = if en {
            format!("Browse {count} components")
        } else {
            format!("浏览 {count} 个组件")
        },
        hint_kui = tr(
            en,
            "打开 KUI 产品站与实时画廊",
            "Open KUI product site & live gallery"
        ),
        hint_comp = tr(
            en,
            "直接在浏览器里点按试用全部 GPUI 组件",
            "Try all GPUI components live in the browser"
        ),
        act_install = tr(en, "安装说明", "Installation"),
        hint_install = tr(
            en,
            "查看 Cargo 引入与 kui add 源码安装步骤",
            "Read Cargo dependency and kui add setup steps"
        ),
        f_klang = svg::solid_fallback("klang"),
        h_klang = tr(
            en,
            "K 语言 · 内存安全的系统编程语言",
            "K · Memory-Safe Systems Programming Language"
        ),
        p_klang = tr(
            en,
            "面向操作系统组件、云服务与 AI 基础设施设计的系统编程语言，命令行工具为 k，源文件扩展名为 .k。编译器在编译期检查引用生命周期并消除可证明安全的边界检查；可增长的局部值放在地址稳定的数据栈上，作用域退出即回收存储，没有垃圾回收停顿。",
            "A systems programming language built for OS components, cloud services, and AI infrastructure, with the k CLI and .k source files. The compiler checks reference lifetimes and eliminates proven bounds checks ahead of time, storing growable values on address-stable data stacks reclaimed at scope exit without GC pauses."
        ),
        d_klang = tr(
            en,
            "同一套前端与类型检查器覆盖三条执行路径：经 C 后端构建支持工作线程与类型化队列的本机可执行文件、生成在 Wasmtime 运行的 WASI WebAssembly 模块，或经内置 TinyCC 在当前进程直接运行；配合 Cordis 微内核用经验证的服务锁装配云端推理网关等宿主服务。",
            "One checker drives three execution paths: native executables with worker threads and typed queues via the C backend, WASI WebAssembly modules verified on Wasmtime, and in-process runs via TinyCC—paired with the Cordis microkernel for verified host service graphs such as cloud inference gateways."
        ),
        klang_url = klang_site(en),
        act_klang = svg::action_content(tr(en, "访问 K 语言站点", "Explore K Language")),
        hint_klang = tr(
            en,
            "打开 K 语言官网：架构图解、语言参考与性能基准",
            "Open K language site: architecture, reference & benchmarks"
        ),
        act_klang_demo = tr(en, "体验编译检查面板", "Try compiler demo"),
        hint_klang_demo = tr(
            en,
            "在下方 GPUI 面板中体验生命周期检查与多目标编译",
            "Test lifetime checks and target selection in the GPUI panel"
        ),
        hint_klang_gh = tr(
            en,
            "查看 kense-labs/klang 编译器与运行时仓库",
            "Inspect kense-labs/klang compiler & runtime source on GitHub"
        ),
        mw_title = tr(en, "底层运行时中间件", "Runtime middleware"),
        mw_sub = tr(
            en,
            "作为基础隔离模块供 KStudio、KWork 等上层产品内部调用：",
            "Foundational isolation modules used underneath KStudio and KWork:"
        ),
        ico_kbox = crate::brand_marks::emblem("kbox", 18),
        mw_kbox = tr(
            en,
            r#"兼容 OCI 镜像的容器与 libkrun 轻量 MicroVM 运行时，支持预热池快速挂载独立环境；需要在桌面 Web 宿主中接入本地虚拟机时，也可通过轻量桥接模块 <a href="https://kense-labs.github.io/krun/">KRun</a> 调用。"#,
            r#"OCI container and libkrun MicroVM runtime with prewarmed pools for isolated task environments, with optional <a href="https://kense-labs.github.io/krun/">KRun</a> bridging for desktop web hosts."#
        ),
        ico_ksb = crate::brand_marks::emblem("ksandbox", 18),
        mw_ksb = tr(
            en,
            "跨平台轻量进程沙箱，利用 macOS Seatbelt、Linux Bubblewrap+seccomp 与 Windows AppContainer 限制命令子进程的目录读写与网络连接。",
            "Lightweight cross-platform process sandbox using macOS Seatbelt, Linux Bubblewrap+seccomp, and Windows AppContainer to restrict subprocess file and network access."
        ),
    );

    // Section 3: Interactive Live GPUI Studio Panel (#experience)
    let experience = format!(
        r##"<section class="kense-section kense-section--panel" id="experience" aria-labelledby="kense-exp-title">
  <div class="kense-shell">
    <div class="kense-section-head kense-section-head--center kense-reveal">
      <h2 id="kense-exp-title">{title}</h2>
      <p>{desc}</p>
    </div>
    <div class="kense-tabs kense-reveal" data-kense-tabs role="tablist" aria-label="{tablist_label}">
      <span class="kense-tab-indicator" data-kense-tab-indicator aria-hidden="true"></span>
      {tab_gw}
      {tab_sb}
      {tab_ui}
      {tab_klang}
    </div>
    <div class="kense-exp-stage kense-reveal">
      <div class="kense-exp-copy-stack">
        <div id="experience-gateway" class="kense-exp-copy is-active" role="tabpanel" aria-labelledby="kense-tab-gateway" data-kense-panel="gateway" data-gpui-story="kense-gateway">
          <div class="kense-exp-copy-head">
            <div class="kense-exp-copy-lead">
              <h3>{gw_h}</h3>
              <p>{gw_p}</p>
            </div>
            <a class="kense-pill kense-pill--solid" href="{prefix}/docs/#kstack" data-product-badge="kstack" data-kense-hint="{gw_hint}">{gw_cta}</a>
          </div>
          <ul class="kense-exp-points">
            <li><p>{gw_b1}</p></li>
            <li><p>{gw_b2}</p></li>
            <li><p>{gw_b3}</p></li>
          </ul>
        </div>
        <div id="experience-sandbox" class="kense-exp-copy" role="tabpanel" aria-labelledby="kense-tab-sandbox" data-kense-panel="sandbox" data-gpui-story="kense-sandbox" hidden>
          <div class="kense-exp-copy-head">
            <div class="kense-exp-copy-lead">
              <h3>{sb_h}</h3>
              <p>{sb_p}</p>
            </div>
            <a class="kense-pill kense-pill--solid" href="{prefix}/docs/#kstudio" data-product-badge="kstudio" data-kense-hint="{sb_hint}">{sb_cta}</a>
          </div>
          <ul class="kense-exp-points">
            <li><p>{sb_b1}</p></li>
            <li><p>{sb_b2}</p></li>
            <li><p>{sb_b3}</p></li>
          </ul>
        </div>
        <div id="experience-interface" class="kense-exp-copy" role="tabpanel" aria-labelledby="kense-tab-interface" data-kense-panel="interface" data-gpui-story="kense-workbench" hidden>
          <div class="kense-exp-copy-head">
            <div class="kense-exp-copy-lead">
              <h3>{ui_h}</h3>
              <p>{ui_p}</p>
            </div>
            <a class="kense-pill kense-pill--solid" href="{prefix}/kui/" data-product-badge="kui" data-kense-hint="{ui_hint}">{ui_cta}</a>
          </div>
          <ul class="kense-exp-points">
            <li><p>{ui_b1}</p></li>
            <li><p>{ui_b2}</p></li>
            <li><p>{ui_b3}</p></li>
          </ul>
        </div>
        <div id="experience-klang" class="kense-exp-copy" role="tabpanel" aria-labelledby="kense-tab-klang" data-kense-panel="klang" data-gpui-story="kense-klang" hidden>
          <div class="kense-exp-copy-head">
            <div class="kense-exp-copy-lead">
              <h3>{kl_h}</h3>
              <p>{kl_p}</p>
            </div>
            <a class="kense-pill kense-pill--solid" href="{kl_url}" data-product-badge="klang" data-kense-hint="{kl_hint}">{kl_cta}</a>
          </div>
          <ul class="kense-exp-points">
            <li><p>{kl_b1}</p></li>
            <li><p>{kl_b2}</p></li>
            <li><p>{kl_b3}</p></li>
          </ul>
        </div>
      </div>
      <div class="kense-exp-media" data-kense-gpui-slot>
        <div class="kense-gpui-shell">
          <iframe allow="clipboard-write" class="rust-story kense-gpui-frame" data-kense-gpui-frame data-story="kense-gateway" data-story-key="kense-gateway" data-lang="{locale}" aria-label="{gpui_label}" data-src="{gpui_src}"></iframe>
        </div>
      </div>
    </div>
  </div>
</section>"##,
        title = tr(
            en,
            "直接在网页里点按试试真实的 GPUI 界面",
            "Try real Kense product interfaces right in your browser"
        ),
        desc = tr(
            en,
            "下方不是静态截图，而是编译成 WebAssembly 实时运行的真实产品界面。切换选项卡或直接点击窗口内的会话、算力节点、远程终端与编译器控件即可交互。",
            "Below is not a static screenshot—each window is a real Kense product workspace compiled to WebAssembly with Rust + GPUI. Switch tabs or interact directly with nodes, terminals, diffs, and compiler checks."
        ),
        tablist_label = tr(en, "核心产品交互演示", "Interactive product scenarios"),
        gpui_label = tr(en, "实时 GPUI 交互面板", "Live GPUI interactive panel"),
        tab_gw = button(
            &format!(
                r#"id="kense-tab-gateway" class="kense-tab is-active" role="tab" aria-selected="true" aria-controls="experience-gateway" data-kense-tab="gateway" data-gpui-story="kense-gateway" data-kense-hint="{}""#,
                tr(
                    en,
                    "切换至 KStack 显存估算与异构选卡工作台",
                    "Switch to KStack VRAM estimate & GPU placement console"
                )
            ),
            tr(en, "KStack 算力控制台", "KStack Console")
        ),
        tab_sb = button(
            &format!(
                r#"id="kense-tab-sandbox" class="kense-tab" role="tab" aria-selected="false" tabindex="-1" aria-controls="experience-sandbox" data-kense-tab="sandbox" data-gpui-story="kense-sandbox" data-kense-hint="{}""#,
                tr(
                    en,
                    "切换至 KStudio 多机终端与沙箱隔离工作台",
                    "Switch to KStudio multi-machine terminal & isolation workspace"
                )
            ),
            tr(en, "KStudio 多机终端", "KStudio Machines")
        ),
        tab_ui = button(
            &format!(
                r#"id="kense-tab-interface" class="kense-tab" role="tab" aria-selected="false" tabindex="-1" aria-controls="experience-interface" data-kense-tab="interface" data-gpui-story="kense-workbench" data-kense-hint="{}""#,
                tr(
                    en,
                    "切换至 KWork 桌面会话、离线听写与 Git Diff 审阅工作台",
                    "Switch to KWork workspace, offline dictation & Git diff review"
                )
            ),
            tr(en, "KWork 桌面工作台", "KWork Workbench")
        ),
        tab_klang = button(
            &format!(
                r#"id="kense-tab-klang" class="kense-tab" role="tab" aria-selected="false" tabindex="-1" aria-controls="experience-klang" data-kense-tab="klang" data-gpui-story="kense-klang" data-kense-hint="{}""#,
                tr(
                    en,
                    "切换至 K 语言生命周期检查与多目标构建工作台",
                    "Switch to K lifetime verification & target build studio"
                )
            ),
            tr(en, "K 语言编译验证", "K Compiler")
        ),
        gw_h = tr(
            en,
            "先算清显存开销，再把模型放到合适的卡上",
            "Estimate memory upfront, then place each model on the right GPU"
        ),
        gw_p = tr(
            en,
            "不用反复猜启动参数。KStack 在拉起引擎前先估算权重、KV cache 与运行时开销，挑出匹配的加速卡并给出其余节点被排除的具体原因。",
            "No guessing engine flags by trial and error. KStack calculates weight, KV cache, and runtime memory before launch, picks matching GPUs, and explains why other cards were skipped."
        ),
        gw_b1 = tr(
            en,
            "NVIDIA、AMD、昇腾、平头哥 PPU、海光与 Apple Silicon 同池混部",
            "Mix NVIDIA, AMD, Ascend, PPU, Hygon, and Apple Silicon in one pool"
        ),
        gw_b2 = tr(
            en,
            "支持 dGPU 显存切分与休眠轮换，让多个模型安全共用一张卡",
            "Share a single card across models with dGPU quotas and sleep/wake"
        ),
        gw_b3 = tr(
            en,
            "统一暴露 /v1 兼容网关，自带按令牌统计与压测工具",
            "OpenAI-compatible /v1 gateway with per-key token accounting & benchmarks"
        ),
        gw_cta = svg::action_content(tr(en, "查看 KStack 详情", "About KStack")),
        gw_hint = tr(
            en,
            "查看 KStack 支持的模型与硬件范围",
            "Read KStack hardware & model specs"
        ),
        sb_h = tr(
            en,
            "把任务交给远端机器，在独立环境里踏实跑完",
            "Hand tasks to your machines inside isolated environments"
        ),
        sb_p = tr(
            en,
            "在 KStudio 里挑一台空闲机器，Agent 会在独立的 KBox 轻量虚拟机或 KSandbox 沙箱中干活；空闲时自动暂停，继续追问时接着上次现场往下跑。",
            "Pick an available machine in KStudio and let an agent work inside its own KBox MicroVM or KSandbox boundary, pausing when idle and resuming where it left off."
        ),
        sb_b1 = tr(
            en,
            "直接沿用本机 ssh 配置与密钥，跨网段可经 Tailscale 直连",
            "Uses your existing SSH config and keys, and works over Tailscale"
        ),
        sb_b2 = tr(
            en,
            "底层沙箱默认拦截宿主机敏感目录（如 ~/.ssh）与未授权外网连接",
            "Underlying sandbox blocks sensitive host paths and unexpected outbound sockets"
        ),
        sb_b3 = tr(
            en,
            "关掉桌面窗口后会话与端口转发仍在后台保持，随时重连",
            "Sessions and port forwards stay up in the menu bar even after closing the window"
        ),
        sb_cta = svg::action_content(tr(en, "查看 KStudio 详情", "About KStudio")),
        sb_hint = tr(
            en,
            "查看 KStudio 多机管理与远程桌面功能",
            "Read KStudio multi-machine capabilities"
        ),
        ui_h = tr(
            en,
            "会话、离线听写与 Git Diff 审阅同屏协作",
            "Conversations, offline dictation, and Git diff review in one workspace"
        ),
        ui_p = tr(
            en,
            "KWork 基于 KUI 构建原生桌面工作台：左侧切换本地目录会话与 Git 改动，中间流式对话并支持本地 Whisper 离线语音听写，右侧同屏审阅代码变更并一键起草审查。",
            "KWork is built on KUI as a native desktop workbench: browse workspace chats on the left, stream agent turns with offline Whisper dictation in the center, and review unified Git diffs side by side."
        ),
        ui_b1 = tr(
            en,
            "三栏桌面工作区：会话收件箱、结构化对话流与行级 Git Diff 同屏对照",
            "Three-pane desktop layout uniting chat inbox, structured turns, and unified Git diffs"
        ),
        ui_b2 = tr(
            en,
            "内置本地 Whisper 离线语音听写与多模型切换，敏感命令逐次确认",
            "Built-in local Whisper dictation, inline model picker, and per-command approval"
        ),
        ui_b3 = tr(
            en,
            "界面全部由 KUI 组件渲染，同一套源码同时编译桌面端与 WebAssembly",
            "Every control is built with KUI and compiles from one source to desktop and WebAssembly"
        ),
        ui_cta = svg::action_content(tr(en, "打开 KUI 组件画廊", "Explore KUI gallery")),
        ui_hint = if en {
            format!("Try all {count} KUI controls in the browser")
        } else {
            format!("在浏览器内交互体验全部 {count} 个 KUI 组件")
        },
        kl_h = tr(
            en,
            "编译期查清引用生命周期，一套检查器输出本机与 WASI",
            "Check lifetimes ahead of time, emit native C, WASI, or TinyCC"
        ),
        kl_p = tr(
            en,
            "K 把可增长的局部值放在地址稳定的数据栈上，作用域退出即回收，省去垃圾回收停顿；同一份检查后的特化程序可按需输出本机二进制、WASI 模块或进程内直跑。",
            "K stores growable local values on address-stable data stacks reclaimed at scope exit without GC pauses, feeding one checked program into native C, WASI modules, or in-process TinyCC."
        ),
        kl_b1 = tr(
            en,
            "编译期验证引用根、所有权转移与跨 await 挂起点存活的值",
            "Validates reference roots, ownership transfers, and values live across await"
        ),
        kl_b2 = tr(
            en,
            "结合控制流与别名分析证明并消除边界检查（checkfree），直接对接 C ABI",
            "Proves checkfree bounds elimination and links directly with native C ABIs"
        ),
        kl_b3 = tr(
            en,
            "Cordis 微内核用经验证的服务锁装配推理网关与可选宿主服务",
            "Cordis microkernel composes verified service graphs for inference gateways"
        ),
        kl_url = klang_site(en),
        kl_cta = svg::action_content(tr(en, "访问 K 语言站点", "Open K Language")),
        kl_hint = tr(
            en,
            "打开 K 语言官网查看架构、语法规范与性能基准",
            "Open K language docs, spec, and benchmarks"
        ),
    );

    // Section 4: Borderless Porcelain Plinth Bento + 3D Bot-Fusion Sculptures (#capabilities)
    let capabilities = format!(
        r##"<section class="kense-section" id="capabilities" aria-labelledby="kense-cap-title">
  <div class="kense-shell">
    <div class="kense-section-head kense-section-head--center kense-reveal">
      <h2 id="kense-cap-title">{title}</h2>
      <p>{desc}</p>
    </div>
    <div class="kense-bento" data-kense-bento>
      <article class="kense-bento-card kense-bento-card--wide" data-kense-tilt data-solid-entrance="pending">
        <div class="kense-tilt-surface" data-kense-tilt-surface>
          <div class="kense-bento-art">{r1}</div>
          <h3>{c1_h}</h3>
          <p>{c1_p}</p>
        </div>
      </article>
      <article class="kense-bento-card" data-kense-tilt data-solid-entrance="pending">
        <div class="kense-tilt-surface" data-kense-tilt-surface>
          <div class="kense-bento-art">{r2}</div>
          <h3>{c2_h}</h3>
          <p>{c2_p}</p>
        </div>
      </article>
      <article class="kense-bento-card" data-kense-tilt data-solid-entrance="pending">
        <div class="kense-tilt-surface" data-kense-tilt-surface>
          <div class="kense-bento-art">{r3}</div>
          <h3>{c3_h}</h3>
          <p>{c3_p}</p>
        </div>
      </article>
      <article class="kense-bento-card kense-bento-card--wide-end" data-kense-tilt data-solid-entrance="pending">
        <div class="kense-tilt-surface" data-kense-tilt-surface>
          <div class="kense-bento-art">{r4}</div>
          <h3>{c4_h}</h3>
          <p>{c4_p}</p>
        </div>
      </article>
    </div>
  </div>
</section>"##,
        title = tr(
            en,
            "少造概念，把工程细节做扎实",
            "Practical details that hold up in daily use"
        ),
        desc = tr(
            en,
            "把最容易踩坑的四件事提前处理好：算清显存再启动、默认把运行环境隔开、关掉窗口不断连、一套界面源码通吃桌面与网页。",
            "Four everyday details handled properly: upfront memory math, isolated execution by default, persistent background sessions, and one UI codebase across desktop and web."
        ),
        r1 = svg::bento_relief("vault-chain"),
        c1_h = tr(
            en,
            "启动前先把显存账算清",
            "Know the VRAM math before launch"
        ),
        c1_p = tr(
            en,
            "KStack 部署前直接列出模型权重、KV cache 和推理引擎各自占多少显存，哪张卡合适、哪张卡为什么排掉写得明明白白，省去报错后再回头排查。",
            "Before starting any engine, KStack shows exact memory needs for weights, KV cache, and overhead, plus which GPUs matched and why others were skipped."
        ),
        r2 = svg::bento_relief("fail-closed"),
        c2_h = tr(en, "默认隔离，越界直接拦住", "Isolated by default"),
        c2_p = tr(
            en,
            "自动执行的任务跑在独立的 KBox 虚拟机或 KSandbox 进程沙箱里，不给宿主机私钥目录和随意出网权限；宿主缺少隔离支持时直接停下报错，不会悄悄裸跑。",
            "Automated agent tasks run inside KBox MicroVMs or KSandbox process boundaries without access to host secrets, stopping immediately if host isolation is unavailable."
        ),
        r3 = svg::bento_relief("release-primitive"),
        c3_h = tr(
            en,
            "关掉主窗口，会话照常跑",
            "Close the window, sessions keep running"
        ),
        c3_p = tr(
            en,
            "KStudio 在菜单栏常驻保持机器连接、端口转发与远端终端，哪台机器上的任务跑完了需要你看一眼会亮起提示；重新点开窗口就能直接接上。",
            "KStudio keeps SSH links, port forwards, and agent terminals running from the menu bar or tray, badging when a session needs you and reattaching cleanly when reopened."
        ),
        r4 = svg::bento_relief("dual-runtime"),
        c4_h = tr(
            en,
            "一套 Rust 界面跑通桌面与网页",
            "One Rust UI across desktop and web"
        ),
        c4_p = if en {
            format!(
                "KUI, KStudio, and KWork share the same GPUI foundation: {count} controls written once in Rust and compiled to native desktop apps and browser WebAssembly."
            )
        } else {
            format!(
                "KUI、KStudio 与 KWork 共用同一套 Rust + GPUI 基础，{count} 个组件写一次即可编译成三端原生桌面程序与浏览器 WebAssembly 预览。"
            )
        },
    );

    // Section 5: Clean Product Collaboration Diagram + 4-Step Workflow (#architecture)
    let architecture = format!(
        r##"<section class="kense-section" id="architecture" aria-labelledby="kense-arch-title">
  <div class="kense-shell">
    <div class="kense-section-head kense-section-head--center kense-reveal">
      <h2 id="kense-arch-title">{title}</h2>
      <p>{desc}</p>
    </div>
    <div class="kense-ecosystem kense-reveal" data-kense-flow role="group" aria-label="{eco_label}">
      <div class="kense-eco-labels" aria-hidden="true"><span>{left_lbl}</span><span>{right_lbl}</span></div>
      <div class="kense-eco-graph">
        <div class="kense-eco-col kense-eco-agents">
          <p class="kense-eco-mobile-label">{left_lbl}</p>
          <article class="kense-eco-card" data-product-badge="kstudio">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_studio}</span><h3>{l1_h}</h3></div>
            <p class="kense-eco-tags">macOS · Linux · Windows · 18 Agent CLIs</p>
            <p class="kense-eco-note">{l1_n}</p>
          </article>
          <article class="kense-eco-card" data-product-badge="kwork">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_work}</span><h3>{l2_h}</h3></div>
            <p class="kense-eco-tags">Workspaces · Tool Approvals · Schedules</p>
            <p class="kense-eco-note">{l2_n}</p>
          </article>
          <article class="kense-eco-card" data-product-badge="kui">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_ui}</span><h3>{l3_h}</h3></div>
            <p class="kense-eco-tags">Rust · GPUI · Desktop + WebAssembly</p>
            <p class="kense-eco-note">{l3_n}</p>
          </article>
        </div>
        {conn_left}
        <div class="kense-eco-core-wrap">
          <div class="kense-eco-core">
            <span class="kense-eco-core-mark">{ico_kstack}</span>
            <strong>Kense</strong>
            <span>{core_sub}</span>
            <small>SSH · Cordis · /v1 API</small>
          </div>
        </div>
        {conn_right}
        <div class="kense-eco-col kense-eco-supply">
          <p class="kense-eco-mobile-label">{right_lbl}</p>
          <article class="kense-eco-card" data-product-badge="kstack">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_kstack_card}</span><h3>{r1_h}</h3></div>
            <p class="kense-eco-tags">NVIDIA · AMD · Ascend · PPU · Hygon · Apple</p>
            <p class="kense-eco-note">{r1_n}</p>
            <a class="kense-inline-action" href="{prefix}/docs/#kstack" data-product-badge="kstack">{r1_cta}</a>
          </article>
          <article class="kense-eco-card" data-product-badge="klang">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_klang_card}</span><h3>{r_klang_h}</h3></div>
            <p class="kense-eco-tags">.k · Lifetimes · Native · WASI · Cordis</p>
            <p class="kense-eco-note">{r_klang_n}</p>
            <a class="kense-inline-action" href="{r_klang_href}" data-product-badge="klang">{r_klang_cta}</a>
          </article>
          <article class="kense-eco-card" data-product-badge="kbox">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_kbox}</span><h3>{r2_h}</h3></div>
            <p class="kense-eco-tags">KBox · KSandbox · OCI &amp; MicroVM</p>
            <p class="kense-eco-note">{r2_n}</p>
            <a class="kense-inline-action" href="{prefix}/#middleware">{r2_cta}</a>
          </article>
        </div>
      </div>
    </div>
    <div class="kense-transaction kense-reveal" data-kense-flow>
      <h3 class="kense-transaction-title">{proc_title}</h3>
      <div class="kense-process">
        <div class="kense-process-timeline" aria-hidden="true"><span class="kense-process-runner"><span class="kense-process-arrow"></span></span></div>
        <ol class="kense-process-steps">
          <li><span class="kense-process-node" style="--process-delay:0s" aria-hidden="true"></span><h4>{s1_h}</h4><p>{s1_p}</p></li>
          <li><span class="kense-process-node" style="--process-delay:1.7s" aria-hidden="true"></span><h4>{s2_h}</h4><p>{s2_p}</p></li>
          <li><span class="kense-process-node" style="--process-delay:3.4s" aria-hidden="true"></span><h4>{s3_h}</h4><p>{s3_p}</p></li>
          <li><span class="kense-process-node" style="--process-delay:5.1s" aria-hidden="true"></span><h4>{s4_h}</h4><p>{s4_p}</p></li>
        </ol>
      </div>
    </div>
  </div>
</section>"##,
        title = tr(en, "五款产品如何分工配合", "How the products fit together"),
        desc = tr(
            en,
            "桌面端用 KStudio 管机器、用 KWork 跑日常任务，界面由 KUI 构建；底层系统组件与 Cordis 服务用 K 语言编写，模型推理交给 KStack 统一调度，隔离环境由 KBox 与 KSandbox 提供。",
            "Use KStudio to manage machines and KWork for daily tasks, both built with KUI; write memory-safe system modules and Cordis services in K, route model inference through KStack, and isolate execution with KBox and KSandbox."
        ),
        eco_label = tr(
            en,
            "KStudio、KWork、KUI 与 KStack、K 语言及底层运行时中间件的配合关系",
            "How KStudio, KWork, KUI, KStack, K, and runtime middleware connect"
        ),
        left_lbl = tr(en, "桌面端与跨端界面", "Desktop Apps & UI"),
        right_lbl = tr(
            en,
            "算力调度、系统语言与隔离环境",
            "Compute, Systems Language & Isolation"
        ),
        l1_h = tr(en, "KStudio · 多机终端", "KStudio · Multi-Machine Hub"),
        l1_n = tr(
            en,
            "管理多台机器的常驻终端、Agent 会话、远程桌面与跨机文件",
            "Persistent terminals, 18 agent CLIs, remote desktops & file sync across machines"
        ),
        l2_h = tr(en, "KWork · 桌面工作台", "KWork · Desktop Workbench"),
        l2_n = tr(
            en,
            "日常项目对话、多模型切换、单次工具确认与定时任务",
            "Project workspaces, inline model switching, tool approvals & scheduled tasks"
        ),
        l3_h = tr(en, "KUI · 跨端组件库", "KUI · GPUI Component Kit"),
        l3_n = tr(
            en,
            "同一套 Rust 控件源码同时跑在原生桌面窗口与 Web 交互预览",
            "Shared Rust + GPUI controls powering native apps and browser previews"
        ),
        conn_left = svg::topology_connector(true),
        ico_studio = svg::product_emblem("kstudio"),
        ico_work = svg::product_emblem("kwork"),
        ico_ui = svg::product_emblem("kui"),
        ico_kstack = svg::product_emblem("kstack"),
        ico_kstack_card = svg::product_emblem("kstack"),
        ico_klang_card = svg::product_emblem("klang"),
        ico_kbox = svg::product_emblem("kbox"),
        core_sub = tr(en, "连接协议", "Protocols"),
        conn_right = svg::topology_connector(false),
        r1_h = tr(en, "KStack · 算力与模型平台", "KStack · GPU Pool & MaaS"),
        r1_n = tr(
            en,
            "把自有服务器与多云加速卡并入同一算力池，算清显存后一键提供 /v1 服务",
            "Pools bare-metal and cloud GPUs with upfront VRAM estimates and /v1 serving"
        ),
        r1_cta = svg::action_content(tr(en, "了解 KStack", "About KStack")),
        r_klang_h = tr(
            en,
            "K 语言 · 系统与服务组件",
            "K · Systems & Cordis Services"
        ),
        r_klang_n = tr(
            en,
            "用编译期生命周期检查与数据栈构建底层系统模块、WASI 组件与 Cordis 推理网关",
            "Builds memory-safe system modules, WASI commands, and Cordis gateways with checked lifetimes"
        ),
        r_klang_href = klang_site(en),
        r_klang_cta = svg::action_content(tr(en, "访问 K 语言站点", "Explore K Language")),
        r2_h = tr(
            en,
            "KBox & KSandbox · 运行时中间件",
            "KBox & KSandbox · Middleware"
        ),
        r2_n = tr(
            en,
            "为上层任务提供独立轻量虚拟机与操作系统级子进程沙箱边界",
            "Supplies lightweight MicroVMs and OS-level subprocess sandboxes on demand"
        ),
        r2_cta = svg::action_content(tr(en, "查看中间件介绍", "Middleware overview")),
        proc_title = tr(en, "一次任务从发出到跑完", "How a task runs end to end"),
        s1_h = tr(en, "发起任务", "Assign a task"),
        s1_p = tr(
            en,
            "在 KStudio 挑一台目标机器，或在 KWork 当前工作区里直接发出指令",
            "Pick a machine in KStudio or send a task from your KWork workspace"
        ),
        s2_h = tr(en, "拉起隔离环境", "Start a clean box"),
        s2_p = tr(
            en,
            "按需在独立 KBox 虚拟机或 KSandbox 沙箱中执行，隔开宿主机敏感目录",
            "Run commands inside an isolated KBox MicroVM or KSandbox directory boundary"
        ),
        s3_h = tr(en, "调度模型推理", "Route model calls"),
        s3_p = tr(
            en,
            "经 KStack 的 /v1 网关转发到已就绪的加速卡实例，自动统计调用用量",
            "Forward requests through KStack's /v1 gateway to matched GPU replicas"
        ),
        s4_h = tr(en, "确认并收回结果", "Review & deliver"),
        s4_p = tr(
            en,
            "关键操作经你确认后放行，代码改动与运行日志直接回到桌面窗口",
            "Approve sensitive steps once and receive code changes and logs in your window"
        ),
    );

    // Section 6: Real Engineering Numbers + Product & Middleware Directory (#principles)
    let kui_dir_desc = if en {
        format!(
            "{count} Rust + GPUI components and screen templates shared across desktop and WebAssembly"
        )
    } else {
        format!("{count} 个跨桌面与浏览器 WebAssembly 共用源码的 Rust + GPUI 组件与页面模板")
    };
    let kui_dir_href = format!("{prefix}/kui/");
    let kstack_dir_href = format!("{prefix}/docs/#kstack");
    let kstudio_dir_href = format!("{prefix}/docs/#kstudio");
    let kwork_dir_href = format!("{prefix}/docs/#kwork");
    let mw_dir_href = format!("{prefix}/docs/#middleware");

    let directory_items = [
        (
            "kstack",
            "KStack",
            tr(en, "核心产品 · 算力与模型", "Product · Compute & MaaS"),
            tr(
                en,
                "40 MB 异构 GPU 算力池调度与模型即服务平台，内置 392 个模型启动配置",
                "40 MB heterogeneous GPU scheduler and /v1 model serving platform across clouds",
            ),
            kstack_dir_href.as_str(),
        ),
        (
            "kstudio",
            "KStudio",
            tr(en, "核心产品 · 多机终端", "Product · Machine Hub"),
            tr(
                en,
                "管理名下所有机器的原生 GPUI 桌面软件，同时跑 18 种编程 Agent、终端与远程桌面",
                "Native GPUI desktop app for all your machines, 18 agent CLIs, terminals, and remote desktops",
            ),
            kstudio_dir_href.as_str(),
        ),
        (
            "kwork",
            "KWork",
            tr(en, "核心产品 · 桌面工作台", "Product · Workbench"),
            tr(
                en,
                "日常跑任务的桌面 Agent 客户端，支持工作区归档、单次工具确认与定时运行",
                "Desktop agent client with workspace history, single-use tool approvals, and schedules",
            ),
            kwork_dir_href.as_str(),
        ),
        (
            "kui",
            "KUI",
            tr(en, "核心产品 · 跨端组件库", "Product · UI Library"),
            kui_dir_desc.as_str(),
            kui_dir_href.as_str(),
        ),
        (
            "klang",
            tr(en, "K 语言", "K Language"),
            tr(en, "核心产品 · 系统编程语言", "Product · Systems Language"),
            tr(
                en,
                "编译期引用生命周期检查与地址稳定数据栈，一套前端覆盖本机 C、WASI 与 TinyCC 即时运行",
                "Memory-safe systems language with checked reference lifetimes, data stacks, Cordis services, and C/WASI/TinyCC targets",
            ),
            klang_site(en),
        ),
        (
            "kbox",
            "KBox / KSandbox",
            tr(en, "底层中间件 · 隔离运行", "Middleware · Isolation"),
            tr(
                en,
                "供上层产品内部调用的 OCI 容器、libkrun 轻量虚拟机与跨平台子进程沙箱",
                "OCI container, libkrun MicroVM, and OS subprocess sandbox modules used by upper products",
            ),
            mw_dir_href.as_str(),
        ),
    ];

    let mut dir_rows = String::new();
    for (badge_id, name, role, summary, href) in directory_items {
        let row_hint = if en {
            format!("View {name} ({role})")
        } else {
            format!("查看 {name}（{role}）")
        };
        dir_rows.push_str(&format!(
            r#"<a class="kense-repo-row" href="{href}" data-product-badge="{badge_id}" data-kense-hint="{}"><span class="kense-repo-name">{}<span>{name}</span></span><span class="kense-repo-desc">{summary}</span><span class="kense-repo-stack">{role}</span><span class="kense-repo-go" aria-hidden="true">{}</span></a>"#,
            esc(&row_hint),
            svg::product_emblem(badge_id),
            icon("arrow", 14)
        ));
    }

    let principles = format!(
        r##"<section class="kense-section kense-future" id="principles" aria-labelledby="kense-future-title">
  <canvas class="kense-dot-field" data-kense-dots aria-hidden="true"></canvas>
  <div class="kense-shell">
    <div class="kense-section-head kense-section-head--center kense-reveal">
      <h2 id="kense-future-title">{title}</h2>
      <p>{desc}</p>
    </div>
    <div class="kense-future-grid">
      <article class="kense-future-item kense-reveal"><h3><span>40 MB</span>{m1_h}</h3><p>{m1_p}</p></article>
      <article class="kense-future-item kense-reveal"><h3><span>1,235</span>{m2_h}</h3><p>{m2_p}</p></article>
      <article class="kense-future-item kense-reveal"><h3><span>18</span>{m3_h}</h3><p>{m3_p}</p></article>
      <article class="kense-future-item kense-reveal"><h3><span>{count}</span>{m4_h}</h3><p>{m4_p}</p></article>
    </div>
    <div class="kense-repos kense-reveal" aria-labelledby="kense-repos-heading">
      <div class="kense-repos-head">
        <h3 id="kense-repos-heading">{dir_title}</h3>
        <a class="kense-repos-org" href="{prefix}/docs/">{dir_more} {arrow}</a>
      </div>
      <div class="kense-repos-list">{dir_rows}</div>
    </div>
  </div>
</section>"##,
        title = tr(
            en,
            "轻量、直接，经得起真实机器规模检验",
            "Small binaries, tested on real machine fleets"
        ),
        desc = tr(
            en,
            "不堆厚重运行时依赖。从算力控制面到桌面客户端，每一层都做到单文件可部署、低内存占用和跟手的交互响应。",
            "No bloated runtime dependencies. From cluster control planes to desktop apps, built for small footprints and fast response."
        ),
        m1_h = tr(en, "KStack 服务端体积", "KStack server binary"),
        m1_p = tr(
            en,
            "单静态二进制内嵌双语控制台，阿里云演练单机纳管跨 7 地域 1,003 台 GPU 主机",
            "Single static binary with web UI; tested managing 1,003 GPU hosts across 7 regions"
        ),
        m2_h = tr(en, "真机验证启动规格", "Verified launch specs"),
        m2_p = tr(
            en,
            "覆盖 392 个模型与多厂商加速卡，镜像、引擎和参数均经真机跑通，选好即跑",
            "Covering 392 models across GPU vendors with known-good images, engines, and flags"
        ),
        m3_h = tr(en, "原生支持 Agent CLI", "Supported agent CLIs"),
        m3_p = tr(
            en,
            "KStudio 直接托管 Claude Code、Codex、Gemini CLI 等会话，关掉窗口后台照常运行",
            "Run Claude Code, Codex, Gemini CLI, and 15 more across any machine you own"
        ),
        m4_h = tr(en, "跨桌面与 Web 组件", "Desktop & web controls"),
        m4_p = tr(
            en,
            "KUI 一套 Rust + GPUI 源码同时覆盖 macOS、Windows、Linux 与浏览器 WebAssembly",
            "Shared Rust + GPUI controls running natively on desktop and in browser WebAssembly"
        ),
        dir_title = tr(en, "产品与底层模块一览", "Products & runtime modules"),
        dir_more = tr(en, "查看全部文档", "Full documentation"),
        arrow = icon("arrow", 14),
    );

    // Section 7: Fanned 3-Card Closing Deck (#start)
    let closing = format!(
        r##"<section class="kense-section kense-closing" id="start" aria-labelledby="kense-start-title">
  <div class="kense-shell">
    <div class="kense-section-head kense-section-head--center kense-reveal">
      <h2 id="kense-start-title">{title}</h2>
      <p>{desc}</p>
    </div>
    <div class="kense-access-routes">
      <article class="kense-reveal" data-access-card="os">
        <div class="kense-access-card">
          <h3>{a1_h}</h3>
          <p>{a1_p}</p>
          {art_os}
          <div class="kense-access-actions">
            <a class="kense-pill kense-pill--solid" href="{prefix}/docs/#kstack" data-product-badge="kstack">{a1_primary}</a>
            <a class="kense-inline-link" href="{a1_klang_href}" data-product-badge="klang">{a1_klang}</a>
          </div>
        </div>
      </article>
      <article class="kense-reveal" data-access-card="runtime">
        <div class="kense-access-card">
          <h3>{a2_h}</h3>
          <p>{a2_p}</p>
          {art_rt}
          <div class="kense-access-actions">
            <a class="kense-pill kense-pill--solid" href="{prefix}/docs/#kstudio" data-product-badge="kstudio">{a2_primary}</a>
            <a class="kense-inline-link" href="{prefix}/docs/#kwork" data-product-badge="kwork">KWork</a>
          </div>
        </div>
      </article>
      <article class="kense-reveal" data-access-card="ui">
        <div class="kense-access-card">
          <h3>{a3_h}</h3>
          <p>{a3_p}</p>
          {art_ui}
          <div class="kense-access-actions">
            {btn_kui_copy}
            <a class="kense-inline-link" href="{prefix}/kui/" data-product-badge="kui">{a3_link}</a>
          </div>
        </div>
      </article>
    </div>
  </div>
</section>"##,
        title = tr(
            en,
            "按你手头的需求挑一款开始",
            "Start with what you need today"
        ),
        desc = tr(
            en,
            "可以把几台分散的 GPU 服务器并成推理池、用 K 语言写内存安全的系统组件，也可以在一个窗口里管好所有开发机，或者直接拿 KUI 写跨端界面。",
            "Pool scattered GPUs into one serving cluster, write memory-safe systems services in K, manage dev machines in one app, or build Rust + GPUI interfaces."
        ),
        a1_h = tr(
            en,
            "模型算力与系统编程 · KStack & K",
            "GPU serving & systems · KStack & K"
        ),
        a1_p = tr(
            en,
            "用 KStack 把自有机房与多云加速卡并成统一推理池，或用内存安全的 K 语言构建高性能系统组件、WASI 模块与 Cordis 推理网关。",
            "Unify bare-metal and cloud GPUs for /v1 model serving in KStack, or build memory-safe systems modules, WASI commands, and Cordis gateways in K."
        ),
        art_os = svg::access_graphic("os"),
        a1_primary = svg::action_content(tr(en, "查看 KStack 说明", "KStack overview")),
        a1_klang_href = klang_site(en),
        a1_klang = tr(en, "K 语言", "K Language"),
        a2_h = tr(
            en,
            "管理多机与日常任务 · KStudio & KWork",
            "Run machines & daily tasks · KStudio & KWork"
        ),
        a2_p = tr(
            en,
            "用 KStudio 集中连接你的工作站与远端服务器，用 KWork 在桌面工作区里处理日常对话、工具确认与定时任务。",
            "Connect every workstation and server in KStudio, and run workspace sessions, tool approvals, and schedules in KWork."
        ),
        art_rt = svg::access_graphic("runtime"),
        a2_primary = svg::action_content(tr(en, "查看 KStudio 说明", "KStudio overview")),
        a3_h = tr(en, "构建跨端界面 · KUI", "Build native & web UI · KUI"),
        a3_p = if en {
            format!(
                "Add the kense-kui crate or copy component source with the kui CLI, and preview all {count} controls live in the browser."
            )
        } else {
            format!(
                "引入 kense-kui crate 或用 kui CLI 将所需组件源码拷进工程，直接在浏览器里预览全部 {count} 个组件。"
            )
        },
        art_ui = svg::access_graphic("ui"),
        btn_kui_copy = copy_cmd_button(
            en,
            kui_cmd,
            tr(en, "复制 cargo add", "Copy cargo add"),
            tr(
                en,
                "点击复制：安装 kense-kui 组件库",
                "Click to copy: install kense-kui"
            ),
            "",
            "kense-pill kense-pill--solid kense-access-copy"
        ),
        a3_link = tr(en, "打开 KUI", "Open KUI"),
    );

    format!(
        r#"<main class="kense-home" id="top">{hero}{family}{experience}{capabilities}{architecture}{principles}{closing}{popover}{guide}</main>"#,
        popover = product_popover(en),
        guide = guide_widget(en)
    )
}

/// Renders the `/docs/` and `/en/docs/` Kense Product & Middleware Documentation Hub.
pub fn docs_hub(en: bool) -> String {
    let prefix = path(en);
    let count = component_count();
    let kui_doc_body = if en {
        format!(
            "Installation guide, {count} interactive component previews, 46 application screen templates, desktop/WASM platform notes, and coding-agent specs."
        )
    } else {
        format!(
            "同一套 Rust 源码覆盖三端原生桌面与浏览器 WebAssembly：包含安装指南、{count} 个实时交互组件、46 个页面模板及编程 Agent 接入说明。"
        )
    };

    let core_cards = [
        (
            "kstack",
            "kstack",
            format!("{prefix}/#products-kstack"),
            "KStack",
            tr(en, "40 MB · 异构算力与 MaaS", "40 MB · GPU Pool & MaaS"),
            tr(
                en,
                "把自有机房服务器、公有云实例、私有云虚拟机与已有 Kubernetes 集群并入同一个算力池，支持 NVIDIA、AMD、昇腾、平头哥 PPU、海光与 Apple Silicon 混部。内置 392 个模型、1,235 份真机验证启动规格，启动前估算显存并说明选卡原因，支持单卡 dGPU 显存切分、跨机并行、PD 分离与统一 /v1 网关。",
                "Unifies bare-metal servers, cloud instances, private VMs, and existing Kubernetes clusters across NVIDIA, AMD, Ascend, PPU, Hygon, and Apple Silicon. Ships 1,235 verified launch specs for 392 models with upfront VRAM estimation, single-GPU sharing (dGPU), multi-node parallelism, and an OpenAI-compatible /v1 gateway.",
            ),
            tr(
                en,
                "查看首页三维演示与选卡面板",
                "View 3D showcase & live demo",
            ),
        ),
        (
            "kstudio",
            "kstudio",
            format!("{prefix}/#products-kstudio"),
            "KStudio",
            tr(
                en,
                "macOS · Linux · Windows · GPUI",
                "macOS · Linux · Windows · GPUI",
            ),
            tr(
                en,
                "连接你名下所有工作站、Linux 主机与云端服务器的原生桌面软件。复用本机 ssh 与 Tailscale，在一个窗口里同时管理 18 种编程 Agent CLI、常驻终端、KBox 轻量虚拟机、远程 Linux 桌面、跨机文件互传与自建 Runner；关掉窗口后会话与端口转发继续在菜单栏后台运行。",
                "One native GPUI app for every machine you own. Reuses your system SSH and Tailscale to run 18 coding agent CLIs, persistent shells, KBox MicroVMs, live Linux desktops, cross-machine file & clipboard sync, and runner monitoring, staying alive in the background when closed.",
            ),
            tr(en, "查看 KStudio 产品演示", "View KStudio showcase"),
        ),
        (
            "kwork",
            "kwork",
            format!("{prefix}/#products-kwork"),
            "KWork",
            tr(
                en,
                "Rust · KUI · GPUI · 桌面端",
                "Rust · KUI · GPUI · Desktop",
            ),
            tr(
                en,
                "日常跑任务与写代码的桌面 Agent 工作台。按本地目录组织工作区与历史归档，可接入 DeepSeek、OpenAI、OpenRouter 或本地模型服务并随时切换推理强度；实时展示工具调用入参、支持敏感操作单次确认，并可将常用流程设为定时自动运行。",
                "Desktop agent client built with KUI and GPUI for daily project work. Organizes local workspace history, connects DeepSeek, OpenAI, OpenRouter, or custom endpoints, previews tool calls for single-use approval, and runs scheduled tasks.",
            ),
            tr(en, "查看 KWork 工作台演示", "View KWork showcase"),
        ),
        (
            "kui",
            "kui",
            format!("{prefix}/kui/docs/"),
            "KUI",
            tr(
                en,
                "GPUI · 桌面 + WebAssembly",
                "GPUI · Desktop + WebAssembly",
            ),
            kui_doc_body.as_str(),
            tr(en, "打开 KUI 完整文档", "Open KUI documentation"),
        ),
        (
            "klang",
            "klang",
            klang_site(en).to_string(),
            tr(en, "K 语言", "K Language"),
            tr(
                en,
                ".k · Native · WASI · Cordis",
                ".k · Native · WASI · Cordis",
            ),
            tr(
                en,
                "内存安全的系统编程语言（命令行 k，源文件 .k）。编译期检查引用生命周期并消除可证明安全的边界检查，可增长值放在地址稳定的数据栈上并在作用域退出时回收；支持 C 后端本机构建、WASI WebAssembly 模块、TinyCC 进程内运行、k lsp 语言服务器与 Cordis 微内核服务编排。",
                "Memory-safe systems programming language (CLI k, .k source files). Checks reference lifetimes and eliminates proven bounds checks at compile time, storing growable values on address-stable data stacks reclaimed at scope exit. Ships native C, WASI WebAssembly, and TinyCC execution paths, k lsp, VS Code tooling, and the Cordis service microkernel.",
            ),
            tr(en, "打开 K 语言完整文档", "Open K language docs"),
        ),
    ];

    let mut list = String::new();
    for (dom_id, badge_id, href, name, tag, body, cta) in core_cards {
        list.push_str(&format!(
            r#"<a class="kense-doc-card" id="{dom_id}" href="{href}"><div class="kense-doc-top"><span class="kense-doc-name">{}<span>{name}</span></span><span class="kense-doc-tag">{tag}</span></div><p>{body}</p><span class="kense-doc-cta">{}</span></a>"#,
            svg::product_emblem(badge_id),
            svg::action_content(cta)
        ));
    }

    let middleware = format!(
        r#"<div class="kense-docs-middleware" id="middleware">
  <h2 class="kense-docs-group-title">{mw_head}</h2>
  <div class="kense-middleware">
    <div class="kense-middleware-head">
      <h3>{mw_subhead}</h3>
      <p>{mw_note}</p>
    </div>
    <div class="kense-middleware-grid">
      <div class="kense-middleware-item">
        <h4>{ico_kbox}<span>KBox</span></h4>
        <p>{mw_kbox}</p>
      </div>
      <div class="kense-middleware-item">
        <h4>{ico_ksb}<span>KSandbox</span></h4>
        <p>{mw_ksb}</p>
      </div>
    </div>
  </div>
</div>"#,
        mw_head = tr(en, "底层运行时中间件", "Runtime middleware"),
        mw_subhead = tr(
            en,
            "基础容器、虚拟机与沙箱模块",
            "Container, MicroVM & sandbox modules"
        ),
        mw_note = tr(
            en,
            "供 KStudio、KWork 在内部调用的隔离运行模块，不单独做成桌面软件。",
            "Internal runtime building blocks powering KStudio and KWork rather than standalone end-user apps."
        ),
        ico_kbox = crate::brand_marks::emblem("kbox", 18),
        mw_kbox = tr(
            en,
            r#"提供兼容 OCI 镜像的容器与基于 libkrun（HVF / KVM）的轻量 MicroVM 运行环境，支持镜像快照与预热池快速分配；若需在桌面 Web Main 宿主进程中桥接本地虚拟机管理，可参阅 <a href="https://kense-labs.github.io/krun/">KRun 对接说明</a>。"#,
            r#"Provides OCI-compatible containers and libkrun (HVF / KVM) MicroVMs with snapshots and prewarmed pools; for bridging local VMs from desktop Web Main processes, see the <a href="https://kense-labs.github.io/krun/">KRun bridge notes</a>."#
        ),
        ico_ksb = crate::brand_marks::emblem("ksandbox", 18),
        mw_ksb = tr(
            en,
            "跨平台轻量进程树沙箱，利用 macOS Seatbelt、Linux Bubblewrap+seccomp 与 Windows AppContainer 限制命令子进程的文件系统访问范围、网络外连与输出上限，退出时完整回收后代进程。",
            "Cross-platform process-tree sandbox using macOS Seatbelt, Linux Bubblewrap+seccomp, and Windows AppContainer to confine filesystem access, outbound network sockets, and descendant processes."
        ),
    );

    format!(
        r#"<main class="kense-docs-hub"><section class="kense-docs-hero"><div class="kense-shell"><h1 class="motion-fade-up">{h1}</h1><p class="kense-docs-lead motion-fade-up motion-fade-up-delay-1">{lead}</p>{strip}</div></section><section class="kense-docs-section"><div class="kense-shell"><h2 class="kense-docs-group-title">{core_head}</h2><div class="kense-docs-grid motion-fade-up motion-fade-up-delay-2">{list}</div>{middleware}</div></section>{popover}</main>"#,
        h1 = tr(en, "产品文档", "Documentation"),
        lead = tr(
            en,
            "K 语言、KStack、KStudio、KWork 与 KUI 五款核心产品的功能说明、交互入口及底层中间件介绍。",
            "Overview and documentation entry points for K, KStack, KStudio, KWork, and KUI, along with runtime middleware notes."
        ),
        core_head = tr(en, "核心产品", "Core products"),
        strip = docs_product_strip(en, prefix),
        popover = product_popover(en),
    )
}
