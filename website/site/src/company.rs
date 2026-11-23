//! Bilingual Kense homepage and documentation hub.

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
    let kui_summary = if en {
        format!(
            "{count} dual-runtime Rust + GPUI controls shared across macOS, Windows, Linux desktop and wasm32 browsers."
        )
    } else {
        format!(
            "一套 Rust + GPUI 源码同时运行于 macOS / Windows / Linux 桌面与 wasm32 浏览器，共 {count} 个可安装组件。"
        )
    };
    let kui_m1 = if en {
        format!("{count} components")
    } else {
        format!("{count} 个同源组件")
    };

    struct Spec<'a> {
        id: &'a str,
        name: &'a str,
        stack: &'a str,
        summary: &'a str,
        metrics: [&'a str; 3],
        cmd: &'a str,
        cmd_short: &'a str,
        href: &'a str,
        external: bool,
        cta: &'a str,
    }

    let specs = [
        Spec {
            id: "kui",
            name: "KUI",
            stack: "Rust · GPUI =0.3.10 · WASM · MIT",
            summary: &kui_summary,
            metrics: [kui_m1.as_str(), "Native + Web", "kui add <id>"],
            cmd: "cargo add kense-kui --rename kui --no-default-features",
            cmd_short: "cargo add kense-kui --rename kui …",
            href: &kui_href,
            external: false,
            cta: tr(en, "进入 KUI 产品站", "Open KUI"),
        },
        Spec {
            id: "kbox",
            name: "KBox",
            stack: "Rust · OCI / libkrun · MIT",
            summary: tr(
                en,
                "Docker 同形跨平台容器与硬件级 libkrun MicroVM 运行时，支持预热池亚 20ms 极速挂载。",
                "Docker-shaped cross-platform OCI container and hardware-isolated libkrun MicroVM runtime with prewarmed pools.",
            ),
            metrics: ["28ms warm start", "6ms exec", "19.3ms pool RTT"],
            cmd: "kbox run --rm --isolation microvm alpine:3.20 -- echo hello-kense",
            cmd_short: "kbox run --isolation microvm …",
            href: "https://github.com/kense-labs/kbox",
            external: true,
            cta: tr(en, "查看 KBox 仓库", "Explore KBox"),
        },
        Spec {
            id: "ksandbox",
            name: "KSandbox",
            stack: "Rust · OS Process Sandbox · MIT",
            summary: tr(
                en,
                "面向不可信 Agent 工具调用的跨平台失败闭合进程树沙箱（Seatbelt / Bubblewrap+seccomp / AppContainer）。",
                "Fail-closed OS process-tree sandbox confining untrusted agent tools via Seatbelt, seccomp, or AppContainer.",
            ),
            metrics: ["Deny-all net", "100 KiB cap", "Tree reap"],
            cmd: "cargo add --git https://github.com/kense-labs/ksandbox ksandbox",
            cmd_short: "cargo add … ksandbox",
            href: "https://github.com/kense-labs/ksandbox",
            external: true,
            cta: tr(en, "查看 KSandbox", "Explore KSandbox"),
        },
        Spec {
            id: "kwork",
            name: "KWork",
            stack: "Rust · KUI · GPUI · Tauri · MIT",
            summary: tr(
                en,
                "基于 KUI 与 GPUI 构建的原生桌面智能体工作台，支持工作区会话、单次工具授权、定时任务与冷恢复。",
                "Native desktop agent workbench built with KUI, GPUI, and Tauri—featuring tool approvals, schedules, and cold resume.",
            ),
            metrics: ["Native GPUI", "Stdio ACP", "Cold resume"],
            cmd: "git clone https://github.com/kense-labs/kwork",
            cmd_short: "git clone …/kwork",
            href: "https://github.com/kense-labs/kwork",
            external: true,
            cta: tr(en, "查看 KWork", "Explore KWork"),
        },
        Spec {
            id: "krun",
            name: "KRun",
            stack: "@kense/krun · Web Main",
            summary: tr(
                en,
                "为 Electron / Web Main 提供 1 分钟本地虚拟机放置、诊断门禁与会话控制面对接。",
                "Docks Electron / Web Main applications to local VM placement, doctor gates, and session orchestration.",
            ),
            metrics: ["1-min dock", "Web Main API", "Doctor gates"],
            cmd: "pnpm add @kense/krun",
            cmd_short: "pnpm add @kense/krun",
            href: "https://kense-labs.github.io/krun/",
            external: true,
            cta: tr(en, "打开 KRun 文档", "Open KRun docs"),
        },
        Spec {
            id: "aios",
            name: "Kense AI OS",
            stack: "Operating Plane · 9 Contexts · MIT",
            summary: tr(
                en,
                "一把 API Key 贯通边缘换票（RFC 8693）、URN 注册、隔离路由、唯一计量、只追加哈希链审计与 Vault 密文注入。",
                "One API key anchoring RFC 8693 token exchange, URN registry, placement, metering, hash-chain audit, and Vault refs.",
            ),
            metrics: ["RFC 8693 SVID", "0 sandbox secrets", "Hash-chain audit"],
            cmd: "git clone https://github.com/kense-labs/kense",
            cmd_short: "git clone …/kense",
            href: "https://github.com/kense-labs/kense",
            external: true,
            cta: tr(en, "阅读架构契约", "Read AI OS specs"),
        },
        Spec {
            id: "dsh-acp",
            name: "dsh-acp",
            stack: "ACP · Stdio JSON-RPC · GHCR · MIT",
            summary: tr(
                en,
                "进程即会话的无守护进程 Agent Client Protocol 发行版，零常驻端口，客户端断开即自动退出清理。",
                "Daemonless stdio JSON-RPC agent harness—zero background ports, anonymous GHCR pull, instant exit on disconnect.",
            ),
            metrics: ["Zero daemon", "Stdio JSON-RPC", "GHCR image"],
            cmd: "docker run --rm -i ghcr.io/kense-labs/dsh-acp:latest",
            cmd_short: "docker run -i ghcr.io/…/dsh-acp",
            href: "https://github.com/kense-labs/dsh-acp",
            external: true,
            cta: tr(en, "查看 dsh-acp", "Explore dsh-acp"),
        },
        Spec {
            id: "kcl",
            name: "KCL",
            stack: "kense-kcl · Rust Parser · MIT",
            summary: tr(
                en,
                "Kense 确定性配置语言：1 MiB / 64 层有界解析、默认闭合 Schema 准入，以及规范字节流 sha256 摘要。",
                "Deterministic configuration language with 1 MiB / 64-depth bounds, closed schema admission, and canonical sha256.",
            ),
            metrics: ["≤ 1 MiB bounded", "Closed schema", "Canonical sha256"],
            cmd: "cargo add --git https://github.com/kense-labs/kcl kense-kcl",
            cmd_short: "cargo add … kense-kcl",
            href: "https://github.com/kense-labs/kcl",
            external: true,
            cta: tr(en, "查看 KCL 规范", "Explore KCL"),
        },
    ];

    let mut cards = String::new();
    for s in specs {
        let ext = if s.external {
            r#" target="_blank" rel="noreferrer""#
        } else {
            ""
        };
        let copy_btn = copy_cmd_button(
            en,
            s.cmd,
            s.cmd_short,
            tr(en, "点击复制安装或运行命令", "Click to copy command"),
            "",
            "kense-pop-copy",
        );
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
  <div class="kense-pop-foot">{copy_btn}<a class="kense-pop-link" href="{href}"{ext}>{cta}</a></div>
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
        tr(en, "产品详情预览", "Product quick preview")
    )
}

/// 3D perspective flip-card tooltip (`#kense-hero-hint`) anchored above the Hero inline product belt.
fn hero_flip_hint(en: bool) -> String {
    let count = component_count();
    let kui_desc = if en {
        format!(
            "{count} GPUI components sharing one Rust tree across macOS, Windows, Linux desktop and WebAssembly."
        )
    } else {
        format!(
            "{count} 个原生桌面与 WebAssembly 同源的 Rust + GPUI 组件，支持 crate 引入与源码拷贝。"
        )
    };
    let entries = [
        (
            "kui",
            "KUI",
            tr(en, "同源界面套件", "GPUI · Native + WASM"),
            kui_desc.as_str(),
        ),
        (
            "kbox",
            "KBox",
            tr(en, "隔离容器与 MicroVM", "28ms MicroVM · 6ms exec"),
            tr(
                en,
                "Docker 同形容器与 libkrun 硬件虚拟化运行时，预热池亚 20ms 极速挂载。",
                "Docker-shaped OCI container and hardware-isolated libkrun MicroVM runtime with prewarmed pools.",
            ),
        ),
        (
            "ksandbox",
            "KSandbox",
            tr(en, "失败闭合进程沙箱", "Seatbelt · seccomp"),
            tr(
                en,
                "将不可信 Agent 工具调用锁入内核边界，默认零网络、零宿主密钥，缺失能力立即拒绝。",
                "Confines untrusted agent tool trees with deny-all network and zero host fallbacks.",
            ),
        ),
        (
            "aios",
            "Kense AI OS",
            tr(en, "统一可信控制面", "RFC 8693 · Vault · Audit"),
            tr(
                en,
                "一把 API Key 锚定边缘短时换票、URN 路由、唯一计量、哈希链审计与出网密文引用注入。",
                "One API key anchoring edge token exchange, URN routing, metering, hash-chain audit, and Vault refs.",
            ),
        ),
        (
            "kwork",
            "KWork",
            tr(en, "原生桌面工作台", "GPUI · Tauri · ACP"),
            tr(
                en,
                "基于 KUI 与 GPUI 构建的原生桌面客户端，驱动无守护进程 ACP 会话与单次工具审批。",
                "Native desktop client driving daemonless dsh-acp sessions with single-use tool approvals and KCL digests.",
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
            "kui",
            format!("{prefix}/kui/"),
            false,
            tr(en, "同源 GPUI 界面", "Dual-Runtime UI"),
            "KUI",
            tr(
                en,
                "浏览桌面与 Web 同源的 GPUI 组件体系",
                "Explore dual-runtime Rust + GPUI controls",
            ),
        ),
        (
            "kbox",
            "https://github.com/kense-labs/kbox".to_owned(),
            true,
            tr(en, "硬件隔离运行时", "MicroVM Runtimes"),
            "KBox",
            tr(
                en,
                "28ms 热启动与亚 20ms 预热池的容器与 MicroVM",
                "28ms start & ~19ms prewarmed libkrun MicroVMs",
            ),
        ),
        (
            "ksandbox",
            "https://github.com/kense-labs/ksandbox".to_owned(),
            true,
            tr(en, "失败闭合沙箱", "Process Sandboxes"),
            "KSandbox",
            tr(
                en,
                "零网络、零明文凭据的跨平台 Agent 进程树隔离",
                "Fail-closed OS process-tree confinement",
            ),
        ),
        (
            "aios",
            "https://github.com/kense-labs/kense".to_owned(),
            true,
            tr(en, "可信控制平面", "Operating Plane"),
            "Kense AI OS",
            tr(
                en,
                "一把 API Key 贯通换票、计量、审计与密文引用",
                "One API key across auth, metering, audit & Vault",
            ),
        ),
        (
            "kwork",
            "https://github.com/kense-labs/kwork".to_owned(),
            true,
            tr(en, "智能体工作台", "Agent Workbench"),
            "KWork",
            tr(
                en,
                "原生 GPUI 桌面客户端、单次工具审批与 KCL 摘要",
                "Native GPUI studio with ACP approvals & KCL",
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
        "请阅读 https://kense-labs.github.io/llms.txt，检查当前仓库的 Rust / GPUI 或智能体运行环境，并按契约接入 Kense（KUI / KBox / KSandbox / KCL）。",
        "Read https://kense-labs.github.io/llms.txt, inspect this workspace's Rust/GPUI or agent setup, and integrate Kense (KUI, KBox, KSandbox, or KCL) following its contracts.",
    );
    let label = tr(
        en,
        "复制 Kense Agent 接入指令",
        "Copy Kense agent integration command",
    );
    let dock_btn = button(
        &format!(
            r#"class="kense-dock-btn" data-dock-button data-copy="{}" data-kind="prompt" data-kense-hint="{}" aria-label="{}""#,
            esc(prompt),
            esc(tr(
                en,
                "点击复制给编程 Agent，一键读取全栈契约",
                "Click to copy for your coding agent"
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
                "粘贴给 Agent · 自动读取 kense-labs.github.io/llms.txt",
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
    let open = tr(en, "打开 Kense 向导", "Open Kense guide");
    let close = tr(en, "收起 Kense 向导", "Close Kense guide");
    let name = tr(en, "Kense 向导", "Kense guide");

    let items = [
        (
            format!("{prefix}/#products"),
            "box",
            tr(en, "核心产品与三维雕塑", "3D Product Stack"),
            "KBox · KUI",
        ),
        (
            format!("{prefix}/#experience"),
            "terminal",
            tr(en, "实时 GPUI 交互工作台", "Live GPUI Studio"),
            "WASM",
        ),
        (
            format!("{prefix}/#architecture"),
            "layers",
            tr(en, "一把 Key 控制面拓扑", "Operating Topology"),
            "AI OS",
        ),
        (
            format!("{prefix}/kui/"),
            "shield",
            tr(en, "进入 KUI 同源组件站", "Open KUI Library"),
            "GPUI",
        ),
        (
            format!("{prefix}/docs/"),
            "book",
            tr(en, "文档与开源契约中心", "Documentation Hub"),
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
            tr(en, "复制指令", "Copy")
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
            "命令已复制 · 可直接粘贴至终端运行",
            "Command copied · ready in terminal"
        ),
        msg_prompt = tr(
            en,
            "已复制 · 直接粘贴给你的编程 Agent",
            "Copied · paste directly into your coding agent"
        ),
    )
}

fn docs_product_strip(prefix: &str) -> String {
    let items = [
        ("kui", "KUI", format!("{prefix}/kui/"), false),
        (
            "kbox",
            "KBox",
            "https://github.com/kense-labs/kbox".to_owned(),
            true,
        ),
        (
            "ksandbox",
            "KSandbox",
            "https://github.com/kense-labs/ksandbox".to_owned(),
            true,
        ),
        (
            "kwork",
            "KWork",
            "https://github.com/kense-labs/kwork".to_owned(),
            true,
        ),
        (
            "krun",
            "KRun",
            "https://kense-labs.github.io/krun/".to_owned(),
            true,
        ),
        (
            "aios",
            "AI OS",
            "https://github.com/kense-labs/kense".to_owned(),
            true,
        ),
        (
            "dsh-acp",
            "dsh-acp",
            "https://github.com/kense-labs/dsh-acp".to_owned(),
            true,
        ),
        (
            "kcl",
            "KCL",
            "https://github.com/kense-labs/kcl".to_owned(),
            true,
        ),
    ];

    let mut out = String::from(
        r#"<div class="kense-product-strip" role="list" aria-label="Kense products">"#,
    );
    for (id, label, href, external) in items {
        let ext = if external {
            r#" target="_blank" rel="noreferrer""#
        } else {
            ""
        };
        out.push_str(&format!(
            r#"<a class="kense-product-badge" role="listitem" href="{href}"{ext} data-product-badge="{id}">{}<span>{label}</span></a>"#,
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
    let ksandbox_cmd = "cargo add --git https://github.com/kense-labs/ksandbox ksandbox";

    // Section 1: Full-Viewport Kinetic Hero + Inline Product Spring Belt + Heat-Flow Shader CTA + Liquid Command Dock
    let hero = format!(
        r##"<section class="kense-hero" data-kense-hero aria-labelledby="kense-hero-title">
  <div class="kense-hero-optics" data-kense-optics aria-hidden="true">
    <div class="kense-optics-glow"></div>
  </div>
  <div class="kense-intro" data-kense-intro data-intro-key="kense:home-intro:v1" hidden aria-hidden="true">
    <div class="kense-intro-backdrop"></div>
    <div class="kense-intro-word kense-intro-word--top" data-intro-word>KENSE</div>
    <div class="kense-intro-word kense-intro-word--bottom" data-intro-word data-intro-direction="right">AI OS</div>
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
        h_prefix = tr(en, "面向智能体时代的", "Open systems for"),
        belt = hero_solution_belt(en, prefix),
        h_suffix = tr(en, "开源底座", "at agent scale"),
        lead = tr(
            en,
            "一把 API Key 贯通隔离运行时、双端同源 GPUI 界面与可验证智能体治理。",
            "One API key uniting isolated runtimes, dual-runtime GPUI interfaces, and verifiable agent governance."
        ),
        explore = svg::action_content(tr(en, "探索核心产品", "Explore products")),
        open_kui = tr(en, "进入 KUI 组件站", "Open KUI"),
        read_docs = tr(en, "阅读架构文档", "Read docs"),
        h1_hint = tr(en, "向下检视三大三维硬件底座", "Inspect the 3D core stack"),
        h1_body = tr(
            en,
            "拖拽交互 KBox 隔离运行时、KUI 双端显示器与 KWork 智能体笔记本雕塑。",
            "Orbit smooth studio 3D hardware sculptures for KBox, KUI, and KWork."
        ),
        h2_hint = tr(
            en,
            "进入 KUI：桌面与 Web 同源组件库",
            "Open KUI dual-runtime GPUI kit"
        ),
        h3_hint = tr(
            en,
            "查阅 AI OS 架构契约与接入指南",
            "Browse AI OS contracts & docs"
        ),
        dock = hero_command_dock(en),
        flip_hint = hero_flip_hint(en),
    );

    // Section 2: Alternating Smooth Studio 3D Hardware Family (#products)
    let family = format!(
        r##"<section class="kense-section kense-opening-anchor" id="products" aria-labelledby="kense-family-title">
  <div class="kense-opening-backdrop" aria-hidden="true"></div>
  <div class="kense-shell">
    <div class="kense-section-head kense-section-head--center kense-reveal">
      <h2 id="kense-family-title">{title}</h2>
      <p>{desc}</p>
    </div>
    <div class="kense-family">
      <article class="kense-family-item" data-family-berth="runtime" data-solid-entrance="pending">
        <div class="kense-solid" data-kense-solid="runtime" data-kense-hint="{solid_rt_hint}" aria-hidden="true">{f_runtime}<canvas width="760" height="570"></canvas></div>
        <div class="kense-family-copy kense-reveal">
          <h3>{h_runtime}</h3>
          <p>{p_runtime}</p>
          <p class="kense-family-detail">{d_runtime}</p>
          <div class="kense-family-links">
            <a class="kense-pill kense-pill--outline" href="https://github.com/kense-labs/kbox" target="_blank" rel="noreferrer" data-product-badge="kbox" data-kense-hint="{hint_kbox}">{act_kbox}</a>
            <a class="kense-inline-link" href="https://github.com/kense-labs/ksandbox" target="_blank" rel="noreferrer" data-product-badge="ksandbox" data-kense-hint="{hint_ksb}">KSandbox</a>
            <a class="kense-inline-link" href="https://kense-labs.github.io/krun/" data-product-badge="krun" data-kense-hint="{hint_krun}">KRun</a>
          </div>
        </div>
      </article>
      <article class="kense-family-item kense-family-item--reverse" data-family-berth="ui" data-solid-entrance="pending">
        <div class="kense-solid" data-kense-solid="ui" data-kense-hint="{solid_ui_hint}" aria-hidden="true">{f_ui}<canvas width="760" height="570"></canvas></div>
        <div class="kense-family-copy kense-reveal">
          <h3>{h_ui}</h3>
          <p>{p_ui}</p>
          <p class="kense-family-detail">{d_ui}</p>
          <div class="kense-family-links">
            <a class="kense-pill kense-pill--outline" href="{prefix}/kui/" data-product-badge="kui" data-kense-hint="{hint_kui}">{act_kui}</a>
            <a class="kense-inline-link" href="{prefix}/kui/components/button/" data-kense-hint="{hint_comp}">{act_comp}</a>
            <a class="kense-inline-link" href="{prefix}/kui/docs/installation/" data-kense-hint="{hint_install}">{act_install}</a>
          </div>
        </div>
      </article>
      <article class="kense-family-item" data-family-berth="agent" data-solid-entrance="pending">
        <div class="kense-solid" data-kense-solid="agent" data-kense-hint="{solid_ag_hint}" aria-hidden="true">{f_agent}<canvas width="760" height="570"></canvas></div>
        <div class="kense-family-copy kense-reveal">
          <h3>{h_agent}</h3>
          <p>{p_agent}</p>
          <p class="kense-family-detail">{d_agent}</p>
          <div class="kense-family-links">
            <a class="kense-pill kense-pill--outline" href="https://github.com/kense-labs/kwork" target="_blank" rel="noreferrer" data-product-badge="kwork" data-kense-hint="{hint_kwork}">{act_kwork}</a>
            <a class="kense-inline-link" href="https://github.com/kense-labs/dsh-acp" target="_blank" rel="noreferrer" data-product-badge="dsh-acp" data-kense-hint="{hint_acp}">dsh-acp</a>
            <a class="kense-inline-link" href="https://github.com/kense-labs/kcl" target="_blank" rel="noreferrer" data-product-badge="kcl" data-kense-hint="{hint_kcl}">KCL</a>
          </div>
        </div>
      </article>
    </div>
  </div>
</section>"##,
        title = tr(
            en,
            "隔离运行时、同源界面与智能体工作台",
            "Isolated runtimes, dual-runtime UI, and agent workbench"
        ),
        desc = tr(
            en,
            "从内核级沙箱与 MicroVM、跨桌面与浏览器的 Rust GPUI 组件库，到支持单次审批的原生桌面终端。",
            "From kernel-enforced sandboxes and MicroVMs to shared desktop/WASM GPUI controls and native ACP workspaces."
        ),
        solid_rt_hint = tr(
            en,
            "拖拽旋转 MicroVM 隔离栈 · 快速划过赋予自转惯性",
            "Drag to orbit MicroVM stack · swipe to spin"
        ),
        solid_ui_hint = tr(
            en,
            "拖拽检视桌面显示器与移动 Web 同源 GPUI 硬件雕塑",
            "Drag to inspect dual-runtime Desktop & WASM devices"
        ),
        solid_ag_hint = tr(
            en,
            "拖拽检视 KWork 一体化笔记本、授权卡与 KCL 棱镜",
            "Drag to orbit KWork laptop, auth card & KCL prism"
        ),
        f_runtime = svg::solid_fallback("runtime"),
        h_runtime = tr(
            en,
            "KBox · 硬件级隔离运行时",
            "KBox · Isolated Runtime Stack"
        ),
        p_runtime = tr(
            en,
            "面向本地智能体与容器负载的双级隔离底座。KBox 提供与 Docker 同形的 CLI，支持默认树内 Sandbox 引擎与 libkrun 硬件级 MicroVM；KRun 为 Electron / Web Main 提供本地虚拟机与会话控制面对接。",
            "Two-tier isolation for local agents and OCI workloads. KBox pairs a Docker-shaped CLI with an in-tree Sandbox engine and libkrun MicroVMs; KRun docks local VM sessions directly into Electron and Web Main."
        ),
        d_runtime = tr(
            en,
            "KSandbox 将不可信工具调用锁入 Seatbelt、Bubblewrap+seccomp 或 AppContainer 进程树——默认零网络、零明文密钥，缺失边界时立即失败闭合。",
            "KSandbox confines untrusted agent tool calls inside Seatbelt, Bubblewrap+seccomp, or AppContainer process trees—deny-all by default, failing closed if any host boundary is missing."
        ),
        act_kbox = svg::action_content(tr(en, "了解 KBox", "Explore KBox")),
        hint_kbox = tr(
            en,
            "KBox：28ms 热启动 · 6ms 执行 · 预热池 ~19ms",
            "KBox: 28ms start · 6ms exec · ~19ms pool"
        ),
        hint_ksb = tr(
            en,
            "KSandbox：失败闭合进程树与 100 KiB 输出上限",
            "KSandbox: fail-closed OS process sandbox"
        ),
        hint_krun = tr(
            en,
            "KRun：1 分钟在 Electron / Web Main 对接本地 VM",
            "KRun: dock local VM sessions from Web Main"
        ),
        f_ui = svg::solid_fallback("ui"),
        h_ui = tr(
            en,
            "KUI · 双端同源 GPUI 界面套件",
            "KUI · Dual-Runtime GPUI Kit"
        ),
        p_ui = tr(
            en,
            "一套 Rust 控件源码同时运行于 macOS / Windows / Linux 原生桌面与 wasm32-unknown-unknown 浏览器环境。原语行为层与视觉表现层严格解耦，预览画廊与安装源码完全同源。",
            "One Rust control tree targeting macOS, Windows, and Linux native desktop alongside wasm32-unknown-unknown browsers. Unstyled behavior primitives stay cleanly separated from themed chrome."
        ),
        d_ui = if en {
            format!(
                "{count} free, MIT-licensed components including 46 full-page workspace screens. Depend on kense-kui from crates.io or own selected ui/ source via kui add."
            )
        } else {
            format!(
                "全部 {count} 个组件（含 46 个全尺寸应用屏幕）均基于 MIT 协议免费开源。既可通过 crates.io 引入 kense-kui，也可通过 kui add 将源码安装进项目。"
            )
        },
        act_kui = svg::action_content(tr(en, "进入 KUI 产品站", "Open KUI")),
        act_comp = if en {
            format!("{count} Components")
        } else {
            format!("浏览 {count} 个组件")
        },
        hint_kui = tr(
            en,
            "KUI：桌面端与 Web 零分叉的纯正 GPUI 组件体系",
            "KUI: zero-fork desktop & WASM GPUI kit"
        ),
        hint_comp = tr(
            en,
            "在浏览器内实时交互体验全部 GPUI 组件画廊",
            "Interact with live GPUI WebAssembly stories"
        ),
        act_install = tr(en, "安装与 CLI", "CLI & Docs"),
        hint_install = tr(
            en,
            "查看 crates.io 依赖与 kui add 源码直装指南",
            "Read crates.io dependency and kui add copy-source guide"
        ),
        f_agent = svg::solid_fallback("agent"),
        h_agent = tr(
            en,
            "KWork · 原生智能体桌面工作台",
            "KWork · Native Agent Workbench"
        ),
        p_agent = tr(
            en,
            "基于 KUI、GPUI 与 Tauri 构建原生桌面客户端，按工作区组织会话、定时任务、单次工具审批与多模型路由；底层通过无守护进程的 dsh-acp 在 stdio 或隔离 MicroVM 中驱动智能体。",
            "Pairs KUI, GPUI, and Tauri in a native desktop client for workspace sessions, scheduled runs, tool approvals, and multi-provider routing—driving daemonless dsh-acp agents over stdio or MicroVMs."
        ),
        d_agent = tr(
            en,
            "配置准入由 KCL（kense-kcl）把关：1 MiB / 64 层有界解析、默认闭合 Schema 校验，以及规范字节流 sha256 摘要。",
            "Agent configurations are admitted through KCL (kense-kcl): bounded parsing, closed-by-default schema validation, and canonical sha256 digests."
        ),
        act_kwork = svg::action_content(tr(en, "了解 KWork", "Explore KWork")),
        hint_kwork = tr(
            en,
            "KWork：会话冷恢复、单次工具审批与定时投递",
            "KWork: cold resume, tool approvals & schedules"
        ),
        hint_acp = tr(
            en,
            "dsh-acp：进程即会话，零常驻端口，断连即回收",
            "dsh-acp: daemonless stdio JSON-RPC harness"
        ),
        hint_kcl = tr(
            en,
            "KCL：1 MiB 有界解析、闭合 Schema 与 sha256 摘要",
            "KCL: bounded parser & canonical sha256"
        ),
    );

    // Section 3: Interactive Live GPUI Studio Workbench (#experience)
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
    </div>
    <div class="kense-exp-stage kense-reveal">
      <div class="kense-exp-copy-stack">
        <div id="experience-gateway" class="kense-exp-copy is-active" role="tabpanel" aria-labelledby="kense-tab-gateway" data-kense-panel="gateway" data-gpui-story="kense-gateway">
          <h3>{gw_h}</h3>
          <p>{gw_p}</p>
          <ul class="kense-exp-points">
            <li><p>{gw_b1}</p></li>
            <li><p>{gw_b2}</p></li>
            <li><p>{gw_b3}</p></li>
          </ul>
          <a class="kense-pill kense-pill--solid" href="https://github.com/kense-labs/kense" target="_blank" rel="noreferrer" data-product-badge="aios" data-kense-hint="{gw_hint}">{gw_cta}</a>
        </div>
        <div id="experience-sandbox" class="kense-exp-copy" role="tabpanel" aria-labelledby="kense-tab-sandbox" data-kense-panel="sandbox" data-gpui-story="kense-sandbox" hidden>
          <h3>{sb_h}</h3>
          <p>{sb_p}</p>
          <ul class="kense-exp-points">
            <li><p>{sb_b1}</p></li>
            <li><p>{sb_b2}</p></li>
            <li><p>{sb_b3}</p></li>
          </ul>
          <a class="kense-pill kense-pill--solid" href="https://github.com/kense-labs/kbox" target="_blank" rel="noreferrer" data-product-badge="kbox" data-kense-hint="{sb_hint}">{sb_cta}</a>
        </div>
        <div id="experience-interface" class="kense-exp-copy" role="tabpanel" aria-labelledby="kense-tab-interface" data-kense-panel="interface" data-gpui-story="kense-workbench" hidden>
          <h3>{ui_h}</h3>
          <p>{ui_p}</p>
          <ul class="kense-exp-points">
            <li><p>{ui_b1}</p></li>
            <li><p>{ui_b2}</p></li>
            <li><p>{ui_b3}</p></li>
          </ul>
          <a class="kense-pill kense-pill--solid" href="{prefix}/kui/" data-product-badge="kui" data-kense-hint="{ui_hint}">{ui_cta}</a>
        </div>
      </div>
      <div class="kense-exp-media" data-kense-gpui-slot>
        <div class="kense-gpui-shell">
          <iframe allow="clipboard-write" class="rust-story kense-gpui-frame" data-kense-gpui-frame data-story="kense-gateway" data-story-key="kense-gateway" data-lang="{locale}" aria-label="{gpui_label}" src="{gpui_src}"></iframe>
        </div>
      </div>
    </div>
  </div>
</section>"##,
        title = tr(
            en,
            "在真实 GPUI 实例中，验证受控执行",
            "Verify bounded execution in live GPUI"
        ),
        desc = tr(
            en,
            "右侧直接运行编译为 WebAssembly 的纯 Rust + GPUI 实例——切换侧栏或点击按钮，实时体验边缘换票、越界阻断与单次工具授权。",
            "Interact directly with a live Rust + GPUI WebAssembly studio—switch rails or click actions to test token exchange, fail-closed blocks, and ACP approvals."
        ),
        tablist_label = tr(en, "Kense 核心链路演示", "Kense architecture scenarios"),
        gpui_label = tr(
            en,
            "Kense 实时 GPUI 交互工作台",
            "Kense live GPUI interactive workbench"
        ),
        tab_gw = button(
            &format!(
                r#"id="kense-tab-gateway" class="kense-tab is-active" role="tab" aria-selected="true" aria-controls="experience-gateway" data-kense-tab="gateway" data-gpui-story="kense-gateway" data-kense-hint="{}""#,
                tr(
                    en,
                    "切换至 GPUI 网关换票与密文引用场景",
                    "Switch GPUI scene: Edge Token Exchange & Vault"
                )
            ),
            tr(en, "一把 Key 信任链", "One-Key Trust Chain")
        ),
        tab_sb = button(
            &format!(
                r#"id="kense-tab-sandbox" class="kense-tab" role="tab" aria-selected="false" tabindex="-1" aria-controls="experience-sandbox" data-kense-tab="sandbox" data-gpui-story="kense-sandbox" data-kense-hint="{}""#,
                tr(
                    en,
                    "切换至 GPUI 预热池与越界拦截场景",
                    "Switch GPUI scene: KBox MicroVM & Fail-Closed Sandbox"
                )
            ),
            tr(en, "隔离运行时与阻断", "Isolated Execution")
        ),
        tab_ui = button(
            &format!(
                r#"id="kense-tab-interface" class="kense-tab" role="tab" aria-selected="false" tabindex="-1" aria-controls="experience-interface" data-kense-tab="interface" data-gpui-story="kense-workbench" data-kense-hint="{}""#,
                tr(
                    en,
                    "切换至 GPUI 单次审批与 KCL 准入场景",
                    "Switch GPUI scene: KWork ACP Approval & KCL Studio"
                )
            ),
            tr(en, "同源界面与单次审批", "Dual-Runtime UI & ACP")
        ),
        gw_h = tr(
            en,
            "主 Key 止步边缘<br>下游共享同一条信任链",
            "Primary key stops at the edge<br>One chain for auth, quota & audit"
        ),
        gw_p = tr(
            en,
            "主 Key 只在统一网关出示一次并置换为 5–15 分钟单受众凭据；下游服务不可见主 Key，沙箱内不持有任何明文凭据。",
            "Presented once at the Unified Gateway and exchanged via RFC 8693 for a 5–15 minute single-audience token; sandboxes hold zero secrets."
        ),
        gw_b1 = tr(
            en,
            "所有能力先注册为 URN 再暴露，九类服务共用鉴权路径",
            "Capabilities register as URNs before exposure"
        ),
        gw_b2 = tr(
            en,
            "Vault 出网仅注入 secret_ref + version，明文不落日志",
            "Vault egress injects secret_ref + version only"
        ),
        gw_b3 = tr(
            en,
            "每次请求产生唯一计量事件，用量与商业账本架构解耦",
            "One meter event per request; separated usage & billing ledgers"
        ),
        gw_cta = svg::action_content(tr(en, "阅读架构契约", "Read AI OS specs")),
        gw_hint = tr(
            en,
            "查看九大模块与五个横切面完整规范",
            "Inspect 9 contexts & 5 shared planes"
        ),
        sb_h = tr(
            en,
            "两级工作负载隔离<br>缺失边界时拒绝静默裸跑",
            "Two isolation grades<br>Never silent fallback to host"
        ),
        sb_p = tr(
            en,
            "无论是本地容器、libkrun 硬件虚拟化 MicroVM，还是嵌入宿主的进程树沙箱，策略由控制面声明，边界由操作系统强制执行。",
            "Whether running OCI containers, libkrun MicroVMs, or embedded process sandboxes, policy declares intent while the OS enforces boundaries."
        ),
        sb_b1 = tr(
            en,
            "KBox 预热池保持已初始化 MicroVM 就绪，亚 20ms 接入",
            "Prewarmed KBox MicroVM pools attach in ~19ms RTT"
        ),
        sb_b2 = tr(
            en,
            "KSandbox 默认封锁外网、宿主套接字、符号链接逃逸与敏感目录",
            "KSandbox blocks network, sockets, symlink escapes & .git"
        ),
        sb_b3 = tr(
            en,
            "超时终止整棵后代进程树，客户端断连即彻底回收实例",
            "Timeouts kill the full descendant tree; VMs vanish on disconnect"
        ),
        sb_cta = svg::action_content(tr(en, "了解 KBox 运行时", "Explore KBox runtime")),
        sb_hint = tr(
            en,
            "对比 darwin arm64 实测生命周期延迟",
            "Measured lifecycle benchmarks"
        ),
        ui_h = tr(
            en,
            "一套 Rust 界面源码<br>驱动桌面工作台与浏览器预览",
            "One Rust UI source<br>Across native desktop and WebAssembly"
        ),
        ui_p = tr(
            en,
            "从 KCL 规范配置准入，到 KUI 双端同源 GPUI 渲染，再到 KWork 通过 stdio JSON-RPC 编排 dsh-acp 智能体会话。",
            "From KCL schema admission to dual-runtime KUI rendering and KWork orchestrating dsh-acp agent sessions over stdio JSON-RPC."
        ),
        ui_b1 = tr(
            en,
            "KCL 在 1 MiB / 64 层上限内解析并生成确定性 sha256",
            "Bounded KCL parser emits deterministic sha256 canonical digests"
        ),
        ui_b2 = tr(
            en,
            "KUI 锁定 gpui-pre =0.3.10，兼顾库依赖与拷源码分发",
            "Pinned gpui-pre =0.3.10 with library and copy-source install paths"
        ),
        ui_b3 = tr(
            en,
            "KWork 统一工具调用审阅、单次授权确认、定时任务与冷恢复",
            "KWork unifies tool inspection, permission prompts & scheduled tasks"
        ),
        ui_cta = svg::action_content(tr(en, "体验 KUI 组件站", "Experience KUI live")),
        ui_hint = tr(
            en,
            "在浏览器内直接交互运行 GPUI 画廊",
            "Run GPUI directly in the browser"
        ),
    );

    // Section 4: Borderless Porcelain Plinth Bento + Volumetric 3D Matte-Clay Sculptures (#capabilities)
    let capabilities = format!(
        r##"<section class="kense-section" id="capabilities" aria-labelledby="kense-cap-title">
  <div class="kense-shell">
    <div class="kense-section-head kense-section-head--center kense-reveal">
      <h2 id="kense-cap-title">{title}</h2>
      <p>{desc}</p>
    </div>
    <div class="kense-bento" data-kense-bento>
      <article class="kense-bento-card kense-bento-card--wide" data-kense-tilt data-solid-entrance="pending" data-kense-hint="{b1_hint}">
        <div class="kense-tilt-surface" data-kense-tilt-surface>
          <div class="kense-bento-art">{r1}</div>
          <h3>{c1_h}</h3>
          <p>{c1_p}</p>
        </div>
      </article>
      <article class="kense-bento-card" data-kense-tilt data-solid-entrance="pending" data-kense-hint="{b2_hint}">
        <div class="kense-tilt-surface" data-kense-tilt-surface>
          <div class="kense-bento-art">{r2}</div>
          <h3>{c2_h}</h3>
          <p>{c2_p}</p>
        </div>
      </article>
      <article class="kense-bento-card" data-kense-tilt data-solid-entrance="pending" data-kense-hint="{b3_hint}">
        <div class="kense-tilt-surface" data-kense-tilt-surface>
          <div class="kense-bento-art">{r3}</div>
          <h3>{c3_h}</h3>
          <p>{c3_p}</p>
        </div>
      </article>
      <article class="kense-bento-card kense-bento-card--wide-end" data-kense-tilt data-solid-entrance="pending" data-kense-hint="{b4_hint}">
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
            "把“默认信任”重构为“可证明边界”",
            "From implicit trust to provable boundaries"
        ),
        desc = tr(
            en,
            "不在应用层堆砌脆弱补丁，而是在操作系统平面固化凭据、沙箱、发布与同源交互原语。",
            "Operating-plane invariants for credentials, process isolation, reversible rollouts, and deterministic UI."
        ),
        b1_hint = tr(
            en,
            "RFC 8693 短时换票 + 出网密文引用注入",
            "RFC 8693 token exchange + egress secret refs"
        ),
        b2_hint = tr(
            en,
            "缺失内核隔离能力时立即拒绝执行，绝不降级",
            "Missing OS isolation fails closed immediately"
        ),
        b3_hint = tr(
            en,
            "OCI/ORAS 内容摘要 + 别名权重秒级回滚",
            "Immutable digests + instant alias-flip rollback"
        ),
        b4_hint = tr(
            en,
            "从运行时到 GPUI 组件库与官网全栈纯 Rust 构建",
            "100% Rust across runtimes, GPUI controls & web"
        ),
        r1 = svg::bento_relief("vault-chain"),
        c1_h = tr(
            en,
            "边缘换票与零凭据沙箱",
            "Edge Token Exchange & Zero-Secret Sandboxes"
        ),
        c1_p = tr(
            en,
            "主 Key 仅存哈希并在网关边缘出现一次；Vault 按作用域封存第三方密钥，出网侧仅注入引用与版本号，明文绝不进入沙箱与日志。",
            "Only key hashes are stored and verified once at the edge; Vault seals third-party secrets per scope and injects references at egress so plaintext never enters sandboxes or logs."
        ),
        r2 = svg::bento_relief("fail-closed"),
        c2_h = tr(en, "内核强制与失败闭合", "Kernel-Enforced & Fail-Closed"),
        c2_p = tr(
            en,
            "Seatbelt、Bubblewrap+seccomp、AppContainer 与 libkrun MicroVM 分层把关；缺少宿主隔离能力时立即报错拒绝执行。",
            "Enforced by Seatbelt, Bubblewrap+seccomp, AppContainer, and libkrun MicroVMs—missing host capabilities fail closed immediately."
        ),
        r3 = svg::bento_relief("release-primitive"),
        c3_h = tr(
            en,
            "不可变版本 · 别名 · 权重",
            "Version · Alias · Traffic Weight"
        ),
        c3_p = tr(
            en,
            "Actions、Packages 与 Releases 横切模型、服务与 Agent 模板，按具体内容摘要灰度放量，回滚只需一次别名翻转。",
            "Actions, Packages, and Releases ship concrete OCI/ORAS digests across models, apps, and agents—making rollback an instant alias flip."
        ),
        r4 = svg::bento_relief("dual-runtime"),
        c4_h = tr(
            en,
            "纯 Rust 与 WASM 全栈同源",
            "Single-Source Rust & WebAssembly"
        ),
        c4_p = if en {
            format!(
                "From sandboxes and the KCL parser to {count} GPUI components and this static site itself—authored in Rust and compiled to native & WebAssembly without DOM forks."
            )
        } else {
            format!(
                "从沙箱运行时、KCL 配置编译器到 {count} 个 GPUI 界面组件乃至整座官网，全部以 Rust 构建并直编原生与 WebAssembly，无第二套前端树分叉。"
            )
        },
    );

    // Section 5: Clean Editorial Ecosystem Topology + 4-Step Process Timeline (#architecture)
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
          <article class="kense-eco-card" data-product-badge="kui">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_ui}</span><h3>{l1_h}</h3></div>
            <p class="kense-eco-tags">KUI · KWork · GPUI + WASM</p>
            <p class="kense-eco-note">{l1_n}</p>
          </article>
          <article class="kense-eco-card" data-product-badge="dsh-acp">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_acp}</span><h3>{l2_h}</h3></div>
            <p class="kense-eco-tags">dsh-acp · ACP · MCP · A2A</p>
            <p class="kense-eco-note">{l2_n}</p>
          </article>
          <article class="kense-eco-card" data-product-badge="kcl">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_kcl}</span><h3>{l3_h}</h3></div>
            <p class="kense-eco-tags">Actions · Packages · Releases · KCL</p>
            <p class="kense-eco-note">{l3_n}</p>
          </article>
        </div>
        {conn_left}
        <div class="kense-eco-core-wrap">
          <div class="kense-eco-core" data-product-badge="aios" data-kense-hint="{core_hint}">
            <span class="kense-eco-core-mark">{ico_aios}</span>
            <strong>Kense</strong>
            <span>{core_sub}</span>
            <small>Gateway · SVID · Vault</small>
          </div>
        </div>
        {conn_right}
        <div class="kense-eco-col kense-eco-supply">
          <p class="kense-eco-mobile-label">{right_lbl}</p>
          <article class="kense-eco-card">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_aios}</span><h3>{r1_h}</h3></div>
            <p class="kense-eco-tags">IaaS · MaaS · TaaS · PaaS / AaaS</p>
            <p class="kense-eco-note">{r1_n}</p>
            <a class="kense-inline-action" href="https://github.com/kense-labs/kense" target="_blank" rel="noreferrer" data-product-badge="aios">{r1_cta}</a>
          </article>
          <article class="kense-eco-card">
            <div class="kense-eco-card-head"><span class="kense-eco-icon">{ico_kbox}</span><h3>{r2_h}</h3></div>
            <p class="kense-eco-tags">KBox · KSandbox · KRun · Pool</p>
            <p class="kense-eco-note">{r2_n}</p>
            <a class="kense-inline-action" href="https://kense-labs.github.io/krun/" data-product-badge="krun">{r2_cta}</a>
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
        title = tr(
            en,
            "一把 Key，贯通能力供给与可信治理",
            "One key uniting capability supply and governance"
        ),
        desc = tr(
            en,
            "下层向上层提供算力、模型与沙箱能力，上层生产轨迹回流 TaaS 构建评测基准；身份、配额、计量、审计与凭据锚定在同一条链上。",
            "Lower layers supply compute, models, and sandboxes upward while production traces flow back as TaaS benchmarks—all anchored on one credential chain."
        ),
        eco_label = tr(
            en,
            "工作负载、开发者供应链与异构资源底座通过 Kense 控制面连接",
            "Workloads, developer supply chain, and isolation substrates connected through the Kense operating plane"
        ),
        left_lbl = tr(en, "交互终端与交付供应链", "Surfaces & Supply Chain"),
        right_lbl = tr(en, "分层服务与异构隔离底座", "Service Chain & Isolation"),
        l1_h = tr(en, "原生桌面与 Web 界面", "Native & Web Interfaces"),
        l1_n = tr(
            en,
            "一套 Rust 组件源覆盖桌面端与浏览器",
            "Shared GPUI component source across desktop & web"
        ),
        l2_h = tr(en, "智能体与协议宿主", "Agent & Protocol Hosts"),
        l2_n = tr(
            en,
            "stdio ACP 会话、MCP 工具与多智能体路由",
            "Stdio ACP sessions, MCP tools & A2A routing"
        ),
        l3_h = tr(en, "不可变交付供应链", "Immutable Supply Chain"),
        l3_n = tr(
            en,
            "OCI/ORAS 制品签名、SBOM 与 KCL 规范摘要",
            "OCI/ORAS digests, SBOM attestation & KCL schemas"
        ),
        conn_left = svg::topology_connector(true),
        ico_ui = svg::product_emblem("kui"),
        ico_acp = svg::product_emblem("dsh-acp"),
        ico_kcl = svg::product_emblem("kcl"),
        ico_aios = svg::product_emblem("aios"),
        ico_kbox = svg::product_emblem("kbox"),
        core_sub = tr(en, "AI OS 控制面", "AI OS Plane"),
        core_hint = tr(
            en,
            "悬停查看 Kense AI OS 控制面详情",
            "Inspect Kense AI OS operating plane"
        ),
        conn_right = svg::topology_connector(false),
        r1_h = tr(en, "全栈能力供给链", "Layered Capability Supply"),
        r1_n = tr(
            en,
            "可解释放置、兼容推理与按有效结果结算",
            "Explainable placement, /v1 inference & outcome billing"
        ),
        r1_cta = svg::action_content(tr(en, "查看上下文映射图", "Context map")),
        r2_h = tr(en, "异构算力与隔离运行时", "Compute & Sandbox Substrate"),
        r2_n = tr(
            en,
            "控制面声明意图，资源池与内核边界强制执行事实",
            "Control plane declares placement; substrate enforces facts"
        ),
        r2_cta = svg::action_content(tr(en, "查看运行时对接", "Runtime docking")),
        proc_title = tr(
            en,
            "从一次请求，到生产闭环演进",
            "From a single request to closed-loop evolution"
        ),
        s1_h = tr(en, "注册与准入", "Register & Admit"),
        s1_p = tr(
            en,
            "能力先注册为 URN，网关校验主 Key 并签发短时 SVID",
            "Register URN contracts; exchange primary key for scoped SVID"
        ),
        s2_h = tr(en, "隔离与注入", "Isolate & Inject"),
        s2_p = tr(
            en,
            "负载在 MicroVM 或失败闭合沙箱运行，出网侧注入密文引用",
            "Execute inside MicroVM or sandbox; inject Vault refs at egress"
        ),
        s3_h = tr(en, "计量与审计", "Meter & Anchor"),
        s3_p = tr(
            en,
            "产生唯一计量事件，敏感操作先落租户只追加哈希链",
            "Emit one meter event and append to tenant hash-chain audit"
        ),
        s4_h = tr(en, "回流与灰度", "Trace & Rollout"),
        s4_p = tr(
            en,
            "生产轨迹回流 TaaS 评测，新制品按版本+别名+权重平滑发布",
            "Feed traces back to TaaS benchmarks; roll out via alias weights"
        ),
    );

    // Section 6: Monumental Borderless Scale Typography + Dot-Matrix Field + Editorial Repo Index (#principles)
    let kui_repo_desc = if en {
        format!("{count} MIT-licensed GPUI components & 46 app screens (kense-kui) + kui CLI")
    } else {
        format!("{count} 个 MIT 免费开源同源 GPUI 组件（含 46 个应用屏幕）与 kui CLI")
    };
    let kui_repo_href = format!("{prefix}/kui/");
    let repos = [
        (
            "aios",
            "kense",
            "Architecture · MIT",
            tr(
                en,
                "AI OS 控制面架构契约、九大模块图谱与限界上下文设计",
                "AI OS operating-plane architecture, 9 module blueprints & context maps",
            ),
            "https://github.com/kense-labs/kense",
            true,
        ),
        (
            "kui",
            "KUI",
            "Rust · GPUI · WASM · MIT",
            kui_repo_desc.as_str(),
            kui_repo_href.as_str(),
            false,
        ),
        (
            "kbox",
            "kbox",
            "Rust · OCI / libkrun · MIT",
            tr(
                en,
                "Docker 同形跨平台容器与 MicroVM 运行时，支持进程级预热池",
                "Docker-shaped OCI & libkrun MicroVM runtime with prewarmed pools",
            ),
            "https://github.com/kense-labs/kbox",
            true,
        ),
        (
            "ksandbox",
            "ksandbox",
            "Rust · OS Sandbox · MIT",
            tr(
                en,
                "面向 Agent 工具调用的跨平台失败闭合进程树沙箱库与 CLI",
                "Fail-closed OS process sandbox for untrusted agent tool execution",
            ),
            "https://github.com/kense-labs/ksandbox",
            true,
        ),
        (
            "kwork",
            "kwork",
            "Rust · GPUI · Tauri · MIT",
            tr(
                en,
                "基于 KUI 与 GPUI 构建的 DeepSeek Harness ACP 原生桌面客户端",
                "Native desktop ACP workbench built with Rust, KUI, GPUI, and Tauri",
            ),
            "https://github.com/kense-labs/kwork",
            true,
        ),
        (
            "dsh-acp",
            "dsh-acp",
            "TypeScript · ACP · MIT",
            tr(
                en,
                "基于 stdio JSON-RPC 的无守护进程 ACP 智能体发行版与公开容器镜像",
                "Daemonless Agent Client Protocol distribution & public GHCR container",
            ),
            "https://github.com/kense-labs/dsh-acp",
            true,
        ),
        (
            "kcl",
            "kcl",
            "Rust · Parser / Schema · MIT",
            tr(
                en,
                "Kense 配置语言：有界解析、闭合 Schema 准入与规范 sha256 摘要",
                "Kense Configuration Language: bounded parser, schema admission & sha256",
            ),
            "https://github.com/kense-labs/kcl",
            true,
        ),
        (
            "krun",
            "krun",
            "Web Main · Runtime Docs",
            tr(
                en,
                "通过 @kense/krun 在 Electron / Web Main 对接本地虚拟机与会话控制面",
                "Electron / Web Main docking for local VM placement and session control",
            ),
            "https://kense-labs.github.io/krun/",
            true,
        ),
    ];
    let mut repo_rows = String::new();
    for (badge_id, name, stack, summary, href, external) in repos {
        let ext_attrs = if external {
            r#" target="_blank" rel="noreferrer""#
        } else {
            ""
        };
        let row_hint = if en {
            format!("Open {name} ({stack})")
        } else {
            format!("打开 {name}（{stack}）")
        };
        repo_rows.push_str(&format!(
            r#"<a class="kense-repo-row" href="{href}"{ext_attrs} data-product-badge="{badge_id}" data-kense-hint="{}"><span class="kense-repo-name">{}<span>{name}</span></span><span class="kense-repo-desc">{summary}</span><span class="kense-repo-stack">{stack}</span><span class="kense-repo-go" aria-hidden="true">{}</span></a>"#,
            esc(&row_hint),
            svg::product_emblem(badge_id),
            icon(if external { "external" } else { "arrow" }, 14)
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
      <article class="kense-future-item kense-reveal"><h3><span>{count}</span>{m1_h}</h3><p>{m1_p}</p></article>
      <article class="kense-future-item kense-reveal"><h3><span>6 ms</span>{m2_h}</h3><p>{m2_p}</p></article>
      <article class="kense-future-item kense-reveal"><h3><span>0</span>{m3_h}</h3><p>{m3_p}</p></article>
      <article class="kense-future-item kense-reveal"><h3><span>MIT</span>{m4_h}</h3><p>{m4_p}</p></article>
    </div>
    <div class="kense-repos kense-reveal" aria-labelledby="kense-repos-heading">
      <div class="kense-repos-head">
        <h3 id="kense-repos-heading">{repos_title}</h3>
        <a class="kense-repos-org" href="https://github.com/kense-labs" target="_blank" rel="noreferrer">github.com/kense-labs {ext}</a>
      </div>
      <div class="kense-repos-list">{repo_rows}</div>
    </div>
  </div>
</section>"##,
        title = tr(
            en,
            "以系统工程的克制，<br>构建可长期拥有的开源基础设施",
            "System-level restraint,<br>open source you can own and verify"
        ),
        desc = tr(
            en,
            "不造黑盒锁定，不把应用逻辑混入操作系统原语。每一层皆具备明确契约、静态验证门禁与公开仓库。",
            "No black-box lock-in, no application bloat inside operating-plane primitives—every layer ships with clear contracts and MIT source."
        ),
        m1_h = tr(en, "同源 GPUI 组件", "Dual-runtime components"),
        m1_p = tr(
            en,
            "桌面端与 WebAssembly 共用单一 Rust 实现，可按需拷贝源码",
            "Shared Rust source across desktop & WASM; install library or source"
        ),
        m2_h = tr(en, "热态沙箱执行", "Warm sandbox exec"),
        m2_p = tr(
            en,
            "KBox 在 darwin arm64 实测 28ms 启动、6ms 执行与亚 20ms 预热连接",
            "28ms start, 6ms exec, and ~19ms prewarmed MicroVM attach on arm64"
        ),
        m3_h = tr(en, "沙箱内明文密钥", "Plaintext secrets in sandbox"),
        m3_p = tr(
            en,
            "边缘单次验签、出网引用注入、默认 deny-all 内核级进程树隔离",
            "Single edge key check, egress secret refs & deny-all OS sandboxes"
        ),
        m4_h = tr(en, "开放许可与契约", "Open license & specs"),
        m4_p = tr(
            en,
            "核心运行时、沙箱、配置语言与 UI 套件均以 MIT 协议开源在 kense-labs",
            "Runtimes, sandboxes, config language & UI kit open on kense-labs"
        ),
        repos_title = tr(en, "开源仓库索引", "Open-source repositories"),
        ext = icon("external", 14),
    );

    // Section 7: Fanned 3-Card Interactive Closing Deck (#start)
    let closing = format!(
        r##"<section class="kense-section kense-closing" id="start" aria-labelledby="kense-start-title">
  <div class="kense-shell">
    <div class="kense-section-head kense-section-head--center kense-reveal">
      <h2 id="kense-start-title">{title}</h2>
      <p>{desc}</p>
    </div>
    <div class="kense-access-routes">
      <article class="kense-reveal" data-access-card="ui">
        <div class="kense-access-card">
          <h3>{a1_h}</h3>
          <p>{a1_p}</p>
          {art_ui}
          <div class="kense-access-actions">
            {btn_kui_copy}
            <a class="kense-inline-link" href="{prefix}/kui/" data-product-badge="kui">{a1_link}</a>
          </div>
        </div>
      </article>
      <article class="kense-reveal" data-access-card="runtime">
        <div class="kense-access-card">
          <h3>{a2_h}</h3>
          <p>{a2_p}</p>
          {art_rt}
          <div class="kense-access-actions">
            {btn_ksb_copy}
            <a class="kense-inline-link" href="https://github.com/kense-labs/kbox" target="_blank" rel="noreferrer" data-product-badge="kbox">KBox</a>
          </div>
        </div>
      </article>
      <article class="kense-reveal" data-access-card="os">
        <div class="kense-access-card">
          <h3>{a3_h}</h3>
          <p>{a3_p}</p>
          {art_os}
          <div class="kense-access-actions">
            <a class="kense-pill kense-pill--solid" href="{prefix}/docs/" data-kense-hint="{a3_hint}">{a3_primary}</a>
            <a class="kense-inline-link" href="https://github.com/kense-labs" target="_blank" rel="noreferrer">GitHub</a>
          </div>
        </div>
      </article>
    </div>
  </div>
</section>"##,
        title = tr(
            en,
            "选择你的切入点，开始构建",
            "Choose an entry point and start building"
        ),
        desc = tr(
            en,
            "引入跨桌面与 Web 的 Rust UI 组件、嵌入失败闭合的 Agent 沙箱，或基于完整控制面契约扩展。",
            "Adopt individual GPUI components, embed a fail-closed process sandbox, or build on the full AI OS architecture."
        ),
        a1_h = tr(en, "接入 KUI 同源组件", "Integrate KUI components"),
        a1_p = tr(
            en,
            "添加 kense-kui crate 或使用 kui CLI 将所需组件源码拷入仓库，附带面向编程 Agent 的 Markdown 规格。",
            "Add the kense-kui crate or copy selected component source with the kui CLI—complete with agent-ready Markdown specs."
        ),
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
        a1_link = tr(en, "KUI 首页", "Open KUI"),
        a2_h = tr(en, "嵌入隔离沙箱与 MicroVM", "Embed sandboxes & MicroVMs"),
        a2_p = tr(
            en,
            "在智能体宿主中引入 ksandbox 强制内核边界，或运行 KBox 管理预热的 libkrun MicroVM 容器池。",
            "Embed ksandbox for OS-enforced tool boundaries, or run KBox for Docker-compatible containers and libkrun MicroVMs."
        ),
        art_rt = svg::access_graphic("runtime"),
        btn_ksb_copy = copy_cmd_button(
            en,
            ksandbox_cmd,
            tr(en, "复制 ksandbox 依赖", "Copy ksandbox dep"),
            tr(
                en,
                "点击复制：引入 KSandbox 进程沙箱库",
                "Click to copy: add KSandbox crate"
            ),
            "",
            "kense-pill kense-pill--solid kense-access-copy"
        ),
        a3_h = tr(en, "探索 AI OS 与全部文档", "Explore AI OS & docs"),
        a3_p = tr(
            en,
            "查阅 Kense AI OS 架构图谱、KRun 本地运行时对接手册、KUI 安装指南与 kense-labs 全系开源仓库。",
            "Browse the Kense AI OS architecture blueprints, KRun docking guides, KUI documentation, and all kense-labs repositories."
        ),
        art_os = svg::access_graphic("os"),
        a3_primary = svg::action_content(tr(en, "打开文档中心", "Open Docs Hub")),
        a3_hint = tr(
            en,
            "一站式直达 KUI、KRun、KBox 与 AI OS 文档",
            "Unified entry to Kense documentation"
        ),
    );

    format!(
        r#"<main class="kense-home" id="top">{hero}{family}{experience}{capabilities}{architecture}{principles}{closing}{popover}{guide}</main>"#,
        popover = product_popover(en),
        guide = guide_widget(en)
    )
}

/// Renders the `/docs/` and `/en/docs/` Kense Documentation Hub.
pub fn docs_hub(en: bool) -> String {
    let prefix = path(en);
    let count = component_count();
    let kui_doc_body = if en {
        format!(
            "Installation guide, {count} free MIT component & screen docs, desktop/WASM platform notes, and coding-agent specs."
        )
    } else {
        format!(
            "面向 GPUI 的 Rust 界面套件安装指南、{count} 个免费开源组件与应用屏幕文档及编程 Agent 接入说明。"
        )
    };
    let cards = [
        (
            "kui",
            format!("{prefix}/kui/docs/"),
            false,
            "KUI",
            tr(en, "GPUI · 桌面 + WASM", "GPUI · Desktop + WASM"),
            kui_doc_body.as_str(),
            tr(en, "打开 KUI 文档", "Open KUI docs"),
        ),
        (
            "krun",
            "https://kense-labs.github.io/krun/".to_owned(),
            true,
            "KRun",
            tr(en, "Web Main · 本地运行时", "Web Main · Local Runtime"),
            tr(
                en,
                "通过 @kense/krun 在 Electron / Web Main 中对接本地运行时：1 分钟接入、调用配方与控制面 API。",
                "Dock local VM placement and session control from Electron / Web Main with @kense/krun—quickstart, recipes, and API.",
            ),
            tr(en, "打开 KRun 文档", "Open KRun docs"),
        ),
        (
            "aios",
            "https://github.com/kense-labs/kense".to_owned(),
            true,
            "Kense AI OS",
            tr(en, "控制面 · 架构契约", "Operating Plane · Architecture"),
            tr(
                en,
                "一把 API Key 架构总图、上下文映射图，以及统一网关、身份、URN、计量、审计、Vault 等模块规范。",
                "Layered AI OS architecture, context maps, and module contracts across Gateway, Identity, URN, Metering, Audit, and Vault.",
            ),
            tr(en, "阅读架构契约", "Read architecture specs"),
        ),
        (
            "kbox",
            "https://github.com/kense-labs/kbox".to_owned(),
            true,
            "KBox",
            tr(en, "OCI · libkrun MicroVM", "OCI · libkrun MicroVM"),
            tr(
                en,
                "Docker 同形跨平台容器与 MicroVM 运行时：安装、--isolation microvm、预热池与性能基准。",
                "Cross-platform Docker-shaped container and libkrun MicroVM runtime: install, --isolation microvm, prewarmed pools, and benchmarks.",
            ),
            tr(en, "阅读 KBox 手册", "Read KBox guide"),
        ),
        (
            "ksandbox",
            "https://github.com/kense-labs/ksandbox".to_owned(),
            true,
            "KSandbox",
            tr(en, "Rust · 失败闭合沙箱", "Rust · Fail-Closed Sandbox"),
            tr(
                en,
                "Agent 工具调用进程树沙箱：NativeSandbox Rust API、Seatbelt / seccomp / AppContainer 威胁模型与默认策略。",
                "OS-enforced process sandbox for agents: NativeSandbox Rust API, threat model, and Seatbelt / seccomp / AppContainer policies.",
            ),
            tr(en, "阅读 KSandbox 文档", "Read KSandbox docs"),
        ),
        (
            "kcl",
            "https://github.com/kense-labs/kcl".to_owned(),
            true,
            "KCL",
            tr(en, "Rust · 配置与摘要", "Rust · Config & Digests"),
            tr(
                en,
                "Kense 配置语言（kense-kcl）：块语法、有界解析限额、闭合 Schema 准入与规范 sha256 摘要。",
                "Kense Configuration Language (kense-kcl): block grammar, parse limits, closed schema admission, and canonical sha256 digests.",
            ),
            tr(en, "阅读 KCL 规范", "Read KCL reference"),
        ),
    ];

    let mut list = String::new();
    for (badge_id, href, external, name, tag, body, cta) in cards {
        let ext = if external {
            r#" target="_blank" rel="noreferrer""#
        } else {
            ""
        };
        list.push_str(&format!(
            r#"<a class="kense-doc-card" href="{href}"{ext}><div class="kense-doc-top"><span class="kense-doc-name">{}<span>{name}</span></span><span class="kense-doc-tag">{tag}</span></div><p>{body}</p><span class="kense-doc-cta">{}</span></a>"#,
            svg::product_emblem(badge_id),
            svg::action_content(cta)
        ));
    }

    format!(
        r#"<main class="kense-docs-hub"><section class="kense-docs-hero"><div class="kense-shell"><h1 class="motion-fade-up">{}</h1><p class="kense-docs-lead motion-fade-up motion-fade-up-delay-1">{}</p>{strip}</div></section><section class="kense-docs-section"><div class="kense-shell"><div class="kense-docs-grid motion-fade-up motion-fade-up-delay-2">{list}</div></div></section>{popover}</main>"#,
        tr(en, "文档中心", "Documentation"),
        tr(
            en,
            "Kense 开源基础设施、运行时、界面组件与架构契约统一入口。",
            "Product guides, runtime APIs, UI specifications, and architecture contracts across Kense."
        ),
        strip = docs_product_strip(prefix),
        popover = product_popover(en),
    )
}
