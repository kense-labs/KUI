//! Bilingual Kense and KUI site navigation and appearance controls.

use crate::controls::{button, text};
use crate::esc;
use std::fmt::Write as _;

#[path = "../shell/palette.rs"]
mod palette;

pub const CSS: &str = include_str!("../shell/shell.css");

fn tr(en: bool, english: &'static str, chinese: &'static str) -> &'static str {
    if en { english } else { chinese }
}

fn path(en: bool) -> &'static str {
    if en { "/en" } else { "" }
}

fn icon(name: &str, size: u8) -> String {
    let path = match name {
        "sun" => {
            r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M4.93 4.93l1.41 1.41m11.32 11.32 1.41 1.41M2 12h2m16 0h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/>"#
        }
        // Rounded Moon coordinates from Lucide; notice: public/assets/LUCIDE-LICENSE.txt.
        "moon" => {
            r#"<path d="M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401"/>"#
        }
        "palette" => {
            r#"<path d="M12 3a9 9 0 1 0 0 18h.9a2.2 2.2 0 0 0 1.6-3.7 1.55 1.55 0 0 1 1.1-2.65h1.5A4 4 0 0 0 21 10.55C20.65 6.25 16.65 3 12 3Z"/><g fill="currentColor" stroke="none"><circle cx="7.6" cy="11" r="1.2"/><circle cx="10.7" cy="7.8" r="1.2"/><circle cx="15.1" cy="8.2" r="1.2"/></g>"#
        }
        "check" => r#"<path d="M5 12.5 9.5 17 19 7.5"/>"#,
        "back" => r#"<path d="M5 12h14M13 5l7 7-7 7"/>"#,
        "up" => r#"<path d="M12 19V5M5 12l7-7 7 7"/>"#,
        "github" => {
            r#"<path d="M15 22v-4a4.8 4.8 0 0 0-1-3.2c3.2-.4 6.5-1.6 6.5-7a5.4 5.4 0 0 0-1.5-3.8 5 5 0 0 0-.1-3.8s-1.2-.4-4 1.5a13.4 13.4 0 0 0-7 0C4.3-.2 3.1.2 3.1.2a5 5 0 0 0-.1 3.8A5.4 5.4 0 0 0 1.5 8c0 5.4 3.3 6.6 6.5 7a4.8 4.8 0 0 0-1 3.2v4"/><path d="M9 18c-4.5 1.5-4.5-2.1-6-2.5"/>"#
        }
        _ => r#"<path d="m6 9 6 6 6-6"/>"#,
    };
    format!(
        r#"<svg width="{size}" height="{size}" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">{path}</svg>"#
    )
}

fn mark() -> &'static str {
    r#"<span class="logo-swap mark" aria-hidden="true"><img class="light" src="/assets/mark-light.png" width="22" height="22" alt=""><img class="dark" src="/assets/mark-dark.png" width="22" height="22" alt=""></span>"#
}

fn custom_editor(en: bool) -> String {
    let t = |a, b| tr(en, a, b);
    format!(
        r##"<div class="editor" data-custom-accent-editor id="site-custom-accent" role="dialog" aria-labelledby="custom-accent-heading" hidden>
<div class="heading">{back}<span id="custom-accent-heading">{custom}</span></div>
<div class="sv" data-sv-surface role="group" aria-label="{field}"><span class="marker" aria-hidden="true"></span>
<input type="range" class="sr-only" data-saturation aria-label="{sat}" aria-describedby="custom-sv-help" min="0" max="100">
<input type="range" class="sr-only" data-brightness aria-label="{val}" aria-describedby="custom-sv-help" min="0" max="100"></div>
<span class="sr-only" id="custom-sv-help">{help}</span>
<div class="values" aria-hidden="true"><span>{sat} <span data-saturation-text></span></span><span>{val} <span data-brightness-text></span></span></div>
<label class="hue-label" for="custom-accent-hue">{hue} <span data-hue-text></span></label>
<input class="hue" type="range" id="custom-accent-hue" data-hue min="0" max="360" step="1">
<label class="hex-label" for="custom-accent-hex">HEX</label>
<div class="hex-row"><span class="sample" aria-hidden="true"></span>{hex}</div>
<span class="error" id="custom-accent-error" role="status" data-hex-error hidden>{err}</span>
<div class="commands">{cancel}{apply}</div></div>"##,
        custom = t("Custom color", "自定义颜色"),
        field = t("Color field", "颜色面板"),
        sat = t("Saturation", "饱和度"),
        val = t("Brightness", "亮度"),
        hue = t("Hue", "色相"),
        help = t(
            "Left and Right adjust saturation. Up and Down adjust brightness. Hold Shift for steps of 10%.",
            "左右键调整饱和度，上下键调整亮度。按住 Shift 每次调整 10%。"
        ),
        back = button(
            &format!(
                r#"class="back" data-custom-back aria-label="{}""#,
                t("Back to colors", "返回主题色")
            ),
            &icon("back", 16)
        ),
        hex = text(
            r##"id="custom-accent-hex" data-hex-input autocomplete="off" autocapitalize="off" spellcheck="false" aria-describedby="custom-accent-error" placeholder="#3366CC""##
        ),
        err = t(
            "Enter 3 or 6 hexadecimal digits.",
            "请输入 3 或 6 位十六进制色值。"
        ),
        cancel = button(r#"class="cancel" data-custom-cancel"#, t("Cancel", "取消")),
        apply = button(r#"class="apply" data-custom-apply"#, t("Apply", "应用"))
    )
}

fn toolbar(en: bool) -> String {
    let t = |a, b| tr(en, a, b);
    let theme_label = t("Toggle theme", "切换主题");
    let accent = t("Accent color", "主题色");
    let lang = t("Language", "语言");
    let check = format!(
        r#"<span class="check" aria-hidden="true">{}</span>"#,
        icon("check", 14)
    );
    let mut options = String::new();
    for (id, english, chinese, _, _) in palette::PRESETS {
        options += &button(
            &format!(
                r#"class="accent-option" data-accent-value="{id}" role="menuitemradio" tabindex="-1" aria-checked="{}""#,
                id == "neutral"
            ),
            &format!(
                r#"<span class="swatch" style="--swatch:var(--swatch-{id})" aria-hidden="true"></span><span>{}</span>{check}"#,
                t(english, chinese)
            ),
        );
    }
    options += r#"<div class="accent-divider" role="separator"></div>"#;
    options += &button(
        r#"class="accent-option custom-option" data-accent-value="custom" role="menuitemradio" aria-checked="false" aria-controls="site-custom-accent" aria-haspopup="dialog" tabindex="-1""#,
        &format!(
            r#"<span class="swatch" data-custom-swatch aria-hidden="true"></span><span>{}</span><span class="custom-end" aria-hidden="true">{check}<span class="forward">{}</span></span>"#,
            t("Custom color", "自定义颜色"),
            icon("chevron", 13)
        ),
    );
    format!(
        r#"<div class="controls">{theme}<div class="accent-picker" data-accent-picker>{trigger}
<div class="accent-menu" id="site-accent-menu" data-accent-menu role="menu" aria-label="{accent}" inert><div class="accent-title" aria-hidden="true">{accent}</div>{options}</div>
{editor}</div><div class="lang" data-lang>{lang_trigger}
<ul class="lang-menu" data-lang-menu id="site-lang-menu" role="menu" aria-label="{lang}" hidden><li role="none"><a class="lang-option" href="/" data-locale="zh" hreflang="zh-CN" role="menuitemradio" aria-checked="{zh}" tabindex="-1">中文</a></li><li role="none"><a class="lang-option" href="/en/" data-locale="en" hreflang="en" role="menuitemradio" aria-checked="{en}" tabindex="-1">English</a></li></ul>
</div></div>"#,
        theme = button(
            &format!(
                r#"class="chip" data-theme-toggle aria-label="{theme_label}" title="{theme_label}""#
            ),
            &format!(
                r#"<span class="sun">{}</span><span class="moon">{}</span>"#,
                icon("sun", 15),
                icon("moon", 15)
            )
        ),
        trigger = button(
            &format!(
                r#"class="chip accent-trigger" data-accent-trigger aria-label="{accent}" title="{accent}" aria-haspopup="menu" aria-expanded="false" aria-controls="site-accent-menu""#
            ),
            &icon("palette", 17)
        ),
        editor = custom_editor(en),
        zh = !en,
        lang_trigger = button(
            &format!(
                r#"class="lang-trigger" data-lang-trigger aria-haspopup="menu" aria-controls="site-lang-menu" aria-expanded="false" aria-label="{lang}" title="{lang}""#
            ),
            &format!(
                r#"<span data-lang-label>{}</span><span class="lang-short" aria-hidden="true">{}</span><span class="chev" aria-hidden="true"></span>"#,
                t("English", "中文"),
                t("EN", "中")
            )
        )
    )
}

fn mega_logo(kind: &str) -> String {
    crate::brand_marks::mega_logo(kind)
}

fn company_mega_menus(en: bool, prefix: &str) -> String {
    let count = kui_catalog::items()
        .filter(|(id, _, _)| *id != "kui-foundation")
        .count();
    let kui_desc = if en {
        format!("{count} dual-runtime Rust + GPUI controls")
    } else {
        format!("{count} 个桌面与 Web 同源 GPUI 组件")
    };
    let comp_label = if en {
        format!("Live gallery ({count} items)")
    } else {
        format!("实时组件画廊（{count} 个）")
    };
    let ext = r#"<svg class="kense-mega-ext" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 17 17 7M8 7h9v9"/></svg>"#;

    format!(
        r##"<div class="kense-mega-panel" id="kense-mega-products" data-kense-mega="products" role="region" aria-label="{prod_region}" inert aria-hidden="true">
  <div class="kense-mega-head">
    <a class="kense-mega-overview" href="{prefix}/#products"><span>{prod_overview}</span><span aria-hidden="true">→</span></a>
  </div>
  <div class="kense-mega-grid">
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item kense-mega-item--has-sub" href="https://github.com/kense-labs/kbox" target="_blank" rel="noreferrer" data-mega-entry>{l_kbox}<div class="kense-mega-copy"><span class="kense-mega-title">KBox {ext}</span><span class="kense-mega-desc">{d_kbox}</span></div></a>
        <div class="kense-mega-sub">
          <a class="kense-mega-item kense-mega-item--sub" href="{prefix}/#experience-sandbox" data-mega-entry><span class="kense-mega-title">{s_pool}</span></a>
          <a class="kense-mega-item kense-mega-item--sub" href="https://kense-labs.github.io/krun/" data-mega-entry><span class="kense-mega-title">KRun · Web Main</span></a>
        </div>
        <a class="kense-mega-item" href="https://github.com/kense-labs/ksandbox" target="_blank" rel="noreferrer" data-mega-entry>{l_ksb}<div class="kense-mega-copy"><span class="kense-mega-title">KSandbox {ext}</span><span class="kense-mega-desc">{d_ksb}</span></div></a>
      </div>
      <p class="kense-mega-label">{c1_lbl}</p>
    </div>
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item kense-mega-item--has-sub" href="{prefix}/kui/" data-mega-entry>{l_kui}<div class="kense-mega-copy"><span class="kense-mega-title">KUI</span><span class="kense-mega-desc">{kui_desc}</span></div></a>
        <div class="kense-mega-sub">
          <a class="kense-mega-item kense-mega-item--sub" href="{prefix}/kui/components/button/" data-mega-entry><span class="kense-mega-title">{comp_label}</span></a>
          <a class="kense-mega-item kense-mega-item--sub" href="{prefix}/kui/ai/" data-mega-entry><span class="kense-mega-title">{s_ai}</span></a>
        </div>
        <a class="kense-mega-item" href="https://github.com/kense-labs/kwork" target="_blank" rel="noreferrer" data-mega-entry>{l_kwork}<div class="kense-mega-copy"><span class="kense-mega-title">KWork {ext}</span><span class="kense-mega-desc">{d_kwork}</span></div></a>
      </div>
      <p class="kense-mega-label">{c2_lbl}</p>
    </div>
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item kense-mega-item--has-sub" href="https://github.com/kense-labs/kense" target="_blank" rel="noreferrer" data-mega-entry>{l_aios}<div class="kense-mega-copy"><span class="kense-mega-title">Kense AI OS {ext}</span><span class="kense-mega-desc">{d_aios}</span></div></a>
        <div class="kense-mega-sub">
          <a class="kense-mega-item kense-mega-item--sub" href="{prefix}/#experience-gateway" data-mega-entry><span class="kense-mega-title">{s_key}</span></a>
        </div>
        <a class="kense-mega-item" href="https://github.com/kense-labs/dsh-acp" target="_blank" rel="noreferrer" data-mega-entry>{l_acp}<div class="kense-mega-copy"><span class="kense-mega-title">dsh-acp {ext}</span><span class="kense-mega-desc">{d_acp}</span></div></a>
        <a class="kense-mega-item" href="https://github.com/kense-labs/kcl" target="_blank" rel="noreferrer" data-mega-entry>{l_kcl}<div class="kense-mega-copy"><span class="kense-mega-title">KCL {ext}</span><span class="kense-mega-desc">{d_kcl}</span></div></a>
      </div>
      <p class="kense-mega-label">{c3_lbl}</p>
    </div>
  </div>
</div>
<div class="kense-mega-panel" id="kense-mega-developers" data-kense-mega="developers" role="region" aria-label="{dev_region}" inert aria-hidden="true">
  <div class="kense-mega-head">
    <a class="kense-mega-overview" href="{prefix}/docs/"><span>{dev_overview}</span><span aria-hidden="true">→</span></a>
  </div>
  <div class="kense-mega-grid">
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item" href="{prefix}/docs/" data-mega-entry>{l_docs}<div class="kense-mega-copy"><span class="kense-mega-title">{dv_hub}</span><span class="kense-mega-desc">{dv_hub_d}</span></div></a>
        <a class="kense-mega-item" href="{prefix}/kui/docs/installation/" data-mega-entry>{l_kui}<div class="kense-mega-copy"><span class="kense-mega-title">{dv_install}</span><span class="kense-mega-desc">{dv_install_d}</span></div></a>
        <a class="kense-mega-item" href="https://kense-labs.github.io/krun/" data-mega-entry>{l_krun}<div class="kense-mega-copy"><span class="kense-mega-title">{dv_krun}</span><span class="kense-mega-desc">{dv_krun_d}</span></div></a>
      </div>
      <p class="kense-mega-label">{dc1_lbl}</p>
    </div>
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item" href="{prefix}/kui/ai/" data-mega-entry>{l_cli}<div class="kense-mega-copy"><span class="kense-mega-title">{dv_ai}</span><span class="kense-mega-desc">{dv_ai_d}</span></div></a>
        <a class="kense-mega-item" href="/llms.txt" data-mega-entry>{l_kcl}<div class="kense-mega-copy"><span class="kense-mega-title">llms.txt</span><span class="kense-mega-desc">{dv_llms_d}</span></div></a>
        <a class="kense-mega-item" href="{prefix}/#start" data-mega-entry>{l_kbox}<div class="kense-mega-copy"><span class="kense-mega-title">{dv_start}</span><span class="kense-mega-desc">{dv_start_d}</span></div></a>
      </div>
      <p class="kense-mega-label">{dc2_lbl}</p>
    </div>
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item" href="https://github.com/kense-labs" target="_blank" rel="noreferrer" data-mega-entry>{l_aios}<div class="kense-mega-copy"><span class="kense-mega-title">kense-labs {ext}</span><span class="kense-mega-desc">{dv_org_d}</span></div></a>
        <a class="kense-mega-item" href="{prefix}/#architecture" data-mega-entry>{l_acp}<div class="kense-mega-copy"><span class="kense-mega-title">{dv_arch}</span><span class="kense-mega-desc">{dv_arch_d}</span></div></a>
        <a class="kense-mega-item" href="{prefix}/#principles" data-mega-entry>{l_ksb}<div class="kense-mega-copy"><span class="kense-mega-title">{dv_repos}</span><span class="kense-mega-desc">{dv_repos_d}</span></div></a>
      </div>
      <p class="kense-mega-label">{dc3_lbl}</p>
    </div>
  </div>
</div>"##,
        prod_region = tr(en, "Kense products", "Kense 产品矩阵"),
        prod_overview = tr(
            en,
            "Explore all Kense infrastructure pillars",
            "浏览 Kense 全栈产品与隔离运行时"
        ),
        l_kbox = mega_logo("kbox"),
        d_kbox = tr(
            en,
            "OCI & libkrun MicroVM runtime · 28ms start",
            "Docker 同形容器与 libkrun MicroVM · 28ms 热启动"
        ),
        s_pool = tr(
            en,
            "Interactive Sandbox & Pool Demo",
            "体验隔离沙箱与预热池"
        ),
        l_ksb = mega_logo("ksandbox"),
        d_ksb = tr(
            en,
            "Fail-closed OS process-tree sandbox",
            "失败闭合的跨平台 Agent 进程树沙箱"
        ),
        c1_lbl = tr(en, "Runtimes", "隔离运行时"),
        l_kui = mega_logo("kui"),
        s_ai = tr(en, "Coding-Agent Specs & CLI", "编程 Agent 集成与 kui CLI"),
        l_kwork = mega_logo("kwork"),
        d_kwork = tr(
            en,
            "Native GPUI + Tauri ACP desktop studio",
            "KUI + GPUI 原生智能体桌面工作台"
        ),
        c2_lbl = tr(en, "Interfaces", "同源界面"),
        l_aios = mega_logo("aios"),
        d_aios = tr(
            en,
            "Gateway, Identity, URN, Metering & Vault",
            "统一网关、计量、审计与密文注入"
        ),
        s_key = tr(en, "Live One-Key Trust Chain", "在线体验一把 Key 换票链路"),
        l_acp = mega_logo("acp"),
        d_acp = tr(
            en,
            "Daemonless stdio JSON-RPC agent harness",
            "进程即会话的无守护进程 ACP 发行版"
        ),
        l_kcl = mega_logo("kcl"),
        d_kcl = tr(
            en,
            "Bounded config parser & sha256 digests",
            "有界配置解析、闭合 Schema 与规范摘要"
        ),
        c3_lbl = tr(en, "AI OS", "操作系统平面"),
        dev_region = tr(en, "Developer resources", "开发者资源"),
        dev_overview = tr(
            en,
            "Open Kense Documentation Hub",
            "进入 Kense 文档与架构契约中心"
        ),
        l_docs = mega_logo("docs"),
        dv_hub = tr(en, "Documentation Hub", "文档中心总览"),
        dv_hub_d = tr(
            en,
            "Unified guides across KUI, KRun, KBox & AI OS",
            "汇集运行时、UI 组件与架构规范入口"
        ),
        dv_install = tr(en, "KUI Installation & CLI", "KUI 安装与 CLI 指南"),
        dv_install_d = tr(
            en,
            "cargo add kense-kui or kui add copy-source",
            "支持 crates.io 依赖或源码直装"
        ),
        l_krun = mega_logo("krun"),
        dv_krun = tr(en, "KRun Docking Guide", "KRun 运行时对接手册"),
        dv_krun_d = tr(
            en,
            "1-minute Electron / Web Main VM control",
            "1 分钟在桌面宿主接入本地虚拟机"
        ),
        dc1_lbl = tr(en, "Guides", "开发指南"),
        l_cli = mega_logo("cli"),
        dv_ai = tr(en, "Agent-First Integration", "Agent 优先接入规范"),
        dv_ai_d = tr(
            en,
            "Deterministic prompts, skills & doctor gates",
            "面向编程智能体的技能包与静态门禁"
        ),
        dv_llms_d = tr(
            en,
            "Machine-readable index for coding agents",
            "供 LLM 与编程智能体直接读取的站点索引"
        ),
        dv_start = tr(en, "Quickstart Commands", "一行命令快速上手"),
        dv_start_d = tr(
            en,
            "Copy kbox, ksandbox & kense-kui snippets",
            "一键复制沙箱、MicroVM 与组件安装命令"
        ),
        dc2_lbl = tr(en, "Tooling", "工具与协议"),
        dv_org_d = tr(
            en,
            "8 MIT-licensed systems & UI repositories",
            "8 个 MIT 开源核心仓库与议题追踪"
        ),
        dv_arch = tr(en, "AI OS Context Map", "AI OS 拓扑与脉搏"),
        dv_arch_d = tr(
            en,
            "Inspect 9 bounded contexts & 4-stage flow",
            "查看能力供给链与四阶段闭环流水线"
        ),
        dv_repos = tr(en, "Open-Source Matrix", "开源仓库矩阵"),
        dv_repos_d = tr(
            en,
            "Compare crates, runtimes & license boundaries",
            "浏览全栈模块定位与技术栈分布"
        ),
        dc3_lbl = tr(en, "Open Source", "开源生态"),
    )
}

fn header(en: bool, company: bool, landing: bool) -> String {
    let prefix = path(en);
    let agent_prompt = "Read https://kense-labs.github.io/llms.txt to explore Kense AI OS, KUI (Rust + GPUI native/WASM components), KBox, KSandbox, KWork, dsh-acp, and KCL.";
    let agent_hint = tr(
        en,
        "Click to copy llms.txt onboarding prompt for your coding agent",
        "点击复制 Agent 引导指令（llms.txt），直接粘贴给编程智能体",
    );
    let prod_btn = button(
        &format!(
            r#"class="company-nav-trigger" data-kense-menu-toggle="products" aria-expanded="false" aria-controls="kense-mega-products" data-kense-hint="{}""#,
            tr(
                en,
                "Explore KBox, KSandbox, KUI, KWork & AI OS",
                "展开查看 KBox、KSandbox、KUI、KWork 与 AI OS"
            )
        ),
        &format!(
            r#"<span>{}</span>{}"#,
            tr(en, "Products", "产品"),
            icon("chevron", 13)
        ),
    );
    let dev_btn = button(
        &format!(
            r#"class="company-nav-trigger" data-kense-menu-toggle="developers" aria-expanded="false" aria-controls="kense-mega-developers" data-kense-hint="{}""#,
            tr(
                en,
                "Docs, CLI, Agent specs & open-source repos",
                "查看文档中心、CLI、Agent 规范与开源仓库"
            )
        ),
        &format!(
            r#"<span>{}</span>{}"#,
            tr(en, "Developers", "开发者"),
            icon("chevron", 13)
        ),
    );
    let wb_hint = tr(
        en,
        "Jump to live Rust + GPUI WebAssembly workbench",
        "直达实时 Rust + GPUI WebAssembly 交互工作台",
    );
    let arch_hint = tr(
        en,
        "Inspect the One-Key AI OS topology & 4-step flow",
        "查看一把 Key 控制面拓扑与四步闭环链路",
    );
    let gh_hint = tr(
        en,
        "Browse all open-source repositories on GitHub",
        "前往 GitHub 浏览 kense-labs 开源仓库",
    );
    let menus = company_mega_menus(en, prefix);

    if company {
        let agent_badge = button(
            &format!(
                r#"class="kense-agent-badge" data-agent-friendly data-copy="{agent_prompt}" data-kind="prompt" data-spring-copy data-kense-hint="{agent_hint}" aria-label="{agent_hint}""#
            ),
            r#"<span class="kense-agent-sheen" aria-hidden="true"></span><svg class="kense-agent-spark" width="12" height="12" viewBox="0 0 18 18" fill="none" aria-hidden="true"><path d="M9 1.5 10.9 6.6 16.5 9l-5.6 2.4L9 16.5 7.1 11.4 1.5 9l5.6-2.4L9 1.5Z" fill="currentColor"/></svg><span class="kense-agent-label">AGENT</span>"#,
        );
        return format!(
            r#"<div class="kense-nav-veil" data-kense-nav-veil aria-hidden="true"></div><header class="topbar company" data-kense-topbar><div class="inner"><div class="company-brand-group"><a class="brand" href="{prefix}/" aria-label="Kense">{}<span class="wordmark">Kense</span></a>{agent_badge}</div><nav class="nav company-nav" data-kense-nav-cluster aria-label="{}">{prod_btn}{dev_btn}<a class="company-nav-link company-nav-extra" href="{prefix}/#experience" data-kense-hint="{wb_hint}">{}</a><a class="company-nav-link company-nav-extra" href="{prefix}/#architecture" data-kense-hint="{arch_hint}">{}</a><a class="company-nav-link" href="{prefix}/docs/">{}</a>{menus}</nav><div class="actions">{}<a class="cta" href="https://github.com/kense-labs" target="_blank" rel="noreferrer" aria-label="GitHub" data-kense-hint="{gh_hint}">{}<span>GitHub</span></a></div></div></header>"#,
            mark(),
            tr(en, "Primary navigation", "主导航"),
            tr(en, "Workbench", "体验"),
            tr(en, "Architecture", "架构"),
            tr(en, "Docs", "文档"),
            toolbar(en),
            icon("github", 15)
        );
    }

    let class = if landing {
        "product landing"
    } else {
        "product"
    };
    let href = format!("{prefix}/kui/");
    let github = "https://github.com/kense-labs/KUI";
    format!(
        r#"<div class="kense-nav-veil" data-kense-nav-veil aria-hidden="true"></div><header class="topbar {class}" data-kense-topbar><div class="inner"><div class="company-brand-group"><a class="brand" href="{href}" aria-label="KUI">{}<span class="wordmark">KUI</span></a></div><nav class="nav company-nav" data-kense-nav-cluster aria-label="{}">{prod_btn}{dev_btn}<a class="company-nav-link company-nav-extra" href="{prefix}/#experience">{}</a><a class="company-nav-link company-nav-extra" href="{prefix}/#architecture">{}</a><a class="company-nav-link" data-nav="components" href="{prefix}/kui/components/button/">{}</a><a class="company-nav-link" data-nav="docs" href="{prefix}/kui/docs/">{}</a>{menus}</nav><div class="actions">{}<a class="cta" href="{github}" target="_blank" rel="noreferrer" aria-label="GitHub">{}<span>GitHub</span></a></div></div></header>"#,
        mark(),
        tr(en, "Primary navigation", "主导航"),
        tr(en, "Workbench", "体验"),
        tr(en, "Architecture", "架构"),
        tr(en, "Components", "组件"),
        tr(en, "Docs", "文档"),
        toolbar(en),
        icon("github", 15)
    )
}

fn footer(en: bool, company: bool) -> String {
    let prefix = path(en);
    if company {
        let count = kui_catalog::items()
            .filter(|(id, _, _)| *id != "kui-foundation")
            .count();
        let components = if en {
            format!("{count} Components")
        } else {
            format!("{count} 个组件")
        };
        return format!(
            r#"<footer class="footer-company-shell"><div class="footer-closing-card"><div class="footer-company-grid"><div class="footer-company-brand"><a class="footer-company-lockup" href="{prefix}/">{mark}<span>Kense</span></a><p class="footer-company-tagline">{tagline}</p></div><div class="footer-company-col"><h2>{c1}</h2><ul><li><a href="{prefix}/kui/">KUI</a></li><li><a href="https://kense-labs.github.io/krun/">KRun</a></li><li><a href="https://github.com/kense-labs/kbox" target="_blank" rel="noreferrer">KBox</a></li><li><a href="https://github.com/kense-labs/ksandbox" target="_blank" rel="noreferrer">KSandbox</a></li></ul></div><div class="footer-company-col"><h2>{c2}</h2><ul><li><a href="https://github.com/kense-labs/kense" target="_blank" rel="noreferrer">Kense AI OS</a></li><li><a href="https://github.com/kense-labs/kwork" target="_blank" rel="noreferrer">KWork</a></li><li><a href="https://github.com/kense-labs/dsh-acp" target="_blank" rel="noreferrer">dsh-acp</a></li><li><a href="https://github.com/kense-labs/kcl" target="_blank" rel="noreferrer">KCL</a></li></ul></div><div class="footer-company-col"><h2>{c3}</h2><ul><li><a href="{prefix}/docs/">{docs}</a></li><li><a href="{prefix}/kui/components/button/">{components}</a></li><li><a href="{prefix}/kui/ai/">{ai}</a></li><li><a href="/llms.txt">llms.txt</a></li></ul></div></div><div class="footer-company-legal"><span>MIT License · Kense</span><a href="https://github.com/kense-labs" target="_blank" rel="noreferrer">github.com/kense-labs</a></div></div></footer>"#,
            mark = mark(),
            tagline = tr(
                en,
                "Open infrastructure for isolated runtimes, dual-runtime UI, and agent systems.",
                "面向隔离运行时、双端同源界面与智能体工程的开源基础设施。"
            ),
            c1 = tr(en, "Products & Runtimes", "产品与运行时"),
            c2 = tr(en, "Architecture & Agents", "架构与智能体"),
            c3 = tr(en, "Developers", "开发者与文档"),
            docs = tr(en, "Documentation Hub", "文档中心"),
            components = components,
            ai = tr(en, "Agent Integration", "Agent 集成规范"),
        );
    }
    let brand = format!("{}<span>KUI</span>", mark());
    let links = format!(
        r#"<a href="/llms.txt">llms.txt</a> · <a href="{prefix}/kui/docs/">{}</a>"#,
        tr(en, "Docs", "文档")
    );
    format!(
        r#"<footer class="footer"><div class="inner"><div class="mark">{brand}</div><p class="muted">MIT · {links}</p></div></footer>"#
    )
}

fn palette_css() -> String {
    let mut css = String::new();
    for (id, _, _, light, dark) in palette::PRESETS {
        let wash = if id == "neutral" { 0 } else { 16 };
        let _ = write!(
            css,
            r#":root,[data-kui-surface="light"]{{--swatch-{id}:#{light:06x}}}html[data-theme="dark"],[data-kui-surface="dark"]{{--swatch-{id}:#{dark:06x}}}html[data-accent="{id}"]{{--accent-light:#{light:06x};--accent-dark:#{dark:06x};--accent-wash:{wash}%}}"#
        );
    }
    css
}

pub fn wrap(
    body: &str,
    title: &str,
    description: &str,
    canonical: &str,
    en: bool,
    company: bool,
) -> String {
    let landing = !company && title == "KUI";
    let description = esc(description);
    let canonical = esc(canonical);
    let title = if company || landing {
        esc(title)
    } else {
        format!("{} · KUI", esc(title))
    };
    let content = if landing {
        format!(r#"<main class="product-landing">{body}</main>"#)
    } else {
        body.to_owned()
    };
    let label = tr(en, "Back to top", "回到顶部");
    format!(
        r##"<!doctype html><html lang="{}" data-theme="light" data-accent="neutral"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{title}</title><meta name="description" content="{description}"><link rel="canonical" href="{canonical}"><meta name="theme-color" content="#ffffff"><link rel="icon" href="/assets/icon.png"><link rel="llms" href="/llms.txt"><style>{}</style><script>{}</script><!--site-assets--></head><body><div class="site-shell"><a class="skip-nav" href="#site-content">{}</a>{}<div id="site-content" tabindex="-1"></div>{content}{}</div>{}</body></html>"##,
        tr(en, "en", "zh-CN"),
        palette_css(),
        include_str!("../shell/appearance.js"),
        tr(en, "Skip to content", "跳到正文"),
        header(en, company, landing),
        if landing || company {
            footer(en, company)
        } else {
            String::new()
        },
        button(
            &format!(
                r#"class="back-top" data-back-top aria-label="{label}" title="{label}" aria-hidden="true" tabindex="-1""#
            ),
            &icon("up", 18)
        )
    )
}
