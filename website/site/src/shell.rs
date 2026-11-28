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
        format!("{count} Rust + GPUI controls shared by desktop and web")
    } else {
        format!("{count} 个桌面与 Web 共用源码的 Rust + GPUI 组件")
    };
    let comp_label = if en {
        format!("Component gallery ({count})")
    } else {
        format!("实时组件画廊（{count} 个）")
    };
    let klang_site = if en {
        "https://kense-labs.github.io/klang/"
    } else {
        "https://kense-labs.github.io/klang/README.zh-CN.html"
    };
    let klang_tutorial = if en {
        "https://kense-labs.github.io/klang/docs/tutorial.html"
    } else {
        "https://kense-labs.github.io/klang/docs/zh/tutorial.html"
    };

    format!(
        r##"<div class="kense-mega-panel" id="kense-mega-products" data-kense-mega="products" role="region" aria-label="{prod_region}" inert aria-hidden="true">
  <div class="kense-mega-head">
    <a class="kense-mega-overview" href="{prefix}/#products"><span>{prod_overview}</span><span aria-hidden="true">→</span></a>
  </div>
  <div class="kense-mega-grid">
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item kense-mega-item--has-sub" href="{prefix}/#products-kstack" data-mega-entry>{l_kstack}<div class="kense-mega-copy"><span class="kense-mega-title">KStack</span><span class="kense-mega-desc">{d_kstack}</span></div></a>
        <div class="kense-mega-sub">
          <a class="kense-mega-item kense-mega-item--sub" href="{prefix}/docs/#kstack" data-mega-entry><span class="kense-mega-title">{s_kstack_doc}</span></a>
          <a class="kense-mega-item kense-mega-item--sub" href="{prefix}/#experience" data-mega-entry><span class="kense-mega-title">{s_kstack_demo}</span></a>
        </div>
        <a class="kense-mega-item kense-mega-item--has-sub" href="{klang_site}" data-mega-entry>{l_klang}<div class="kense-mega-copy"><span class="kense-mega-title">{n_klang}</span><span class="kense-mega-desc">{d_klang}</span></div></a>
        <div class="kense-mega-sub">
          <a class="kense-mega-item kense-mega-item--sub" href="{klang_tutorial}" data-mega-entry><span class="kense-mega-title">{s_klang_doc}</span></a>
          <a class="kense-mega-item kense-mega-item--sub" href="{prefix}/#experience-klang" data-mega-entry><span class="kense-mega-title">{s_klang_demo}</span></a>
        </div>
      </div>
      <p class="kense-mega-label">{c1_lbl}</p>
    </div>
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item kense-mega-item--has-sub" href="{prefix}/#products-kstudio" data-mega-entry>{l_kstudio}<div class="kense-mega-copy"><span class="kense-mega-title">KStudio</span><span class="kense-mega-desc">{d_kstudio}</span></div></a>
        <div class="kense-mega-sub">
          <a class="kense-mega-item kense-mega-item--sub" href="{prefix}/docs/#kstudio" data-mega-entry><span class="kense-mega-title">{s_kstudio_doc}</span></a>
        </div>
        <a class="kense-mega-item" href="{prefix}/#products-kwork" data-mega-entry>{l_kwork}<div class="kense-mega-copy"><span class="kense-mega-title">KWork</span><span class="kense-mega-desc">{d_kwork}</span></div></a>
      </div>
      <p class="kense-mega-label">{c2_lbl}</p>
    </div>
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item kense-mega-item--has-sub" href="{prefix}/kui/" data-mega-entry>{l_kui}<div class="kense-mega-copy"><span class="kense-mega-title">KUI</span><span class="kense-mega-desc">{kui_desc}</span></div></a>
        <div class="kense-mega-sub">
          <a class="kense-mega-item kense-mega-item--sub" href="{prefix}/kui/components/button/" data-mega-entry><span class="kense-mega-title">{comp_label}</span></a>
          <a class="kense-mega-item kense-mega-item--sub" href="{prefix}/kui/ai/" data-mega-entry><span class="kense-mega-title">{s_ai}</span></a>
        </div>
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
        <a class="kense-mega-item" href="{klang_tutorial}" data-mega-entry>{l_klang_dev}<div class="kense-mega-copy"><span class="kense-mega-title">{dv_klang}</span><span class="kense-mega-desc">{dv_klang_d}</span></div></a>
        <a class="kense-mega-item" href="{prefix}/#architecture" data-mega-entry>{l_kstack}<div class="kense-mega-copy"><span class="kense-mega-title">{dv_arch}</span><span class="kense-mega-desc">{dv_arch_d}</span></div></a>
      </div>
      <p class="kense-mega-label">{dc1_lbl}</p>
    </div>
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item" href="{prefix}/kui/docs/installation/" data-mega-entry>{l_kui}<div class="kense-mega-copy"><span class="kense-mega-title">{dv_install}</span><span class="kense-mega-desc">{dv_install_d}</span></div></a>
        <a class="kense-mega-item" href="{prefix}/kui/components/button/" data-mega-entry>{l_cli}<div class="kense-mega-copy"><span class="kense-mega-title">{comp_label}</span><span class="kense-mega-desc">{dv_comp_d}</span></div></a>
        <a class="kense-mega-item" href="/llms.txt" data-mega-entry>{l_docs}<div class="kense-mega-copy"><span class="kense-mega-title">llms.txt</span><span class="kense-mega-desc">{dv_llms_d}</span></div></a>
      </div>
      <p class="kense-mega-label">{dc2_lbl}</p>
    </div>
    <div class="kense-mega-col" data-mega-col>
      <div class="kense-mega-highlight" aria-hidden="true"></div>
      <div class="kense-mega-list">
        <a class="kense-mega-item" href="{prefix}/docs/#middleware" data-mega-entry>{l_kbox}<div class="kense-mega-copy"><span class="kense-mega-title">KBox</span><span class="kense-mega-desc">{dv_kbox_d}</span></div></a>
        <a class="kense-mega-item" href="{prefix}/docs/#middleware" data-mega-entry>{l_ksb}<div class="kense-mega-copy"><span class="kense-mega-title">KSandbox</span><span class="kense-mega-desc">{dv_ksb_d}</span></div></a>
        <a class="kense-mega-item" href="https://github.com/kense-labs/klang" target="_blank" rel="noreferrer" data-mega-entry>{l_klang_repo}<div class="kense-mega-copy"><span class="kense-mega-title">klang GitHub</span><span class="kense-mega-desc">{dv_klang_repo_d}</span></div></a>
      </div>
      <p class="kense-mega-label">{dc3_lbl}</p>
    </div>
  </div>
</div>"##,
        prod_region = tr(en, "Kense products", "Kense 核心产品"),
        prod_overview = tr(
            en,
            "Browse KStack, KStudio, KWork, KUI, and K",
            "查看 KStack、KStudio、KWork、KUI 与 K 语言五款核心产品"
        ),
        l_kstack = mega_logo("kstack"),
        d_kstack = tr(
            en,
            "40 MB heterogeneous GPU scheduler & /v1 model serving",
            "40 MB 异构 GPU 算力池调度与一键模型推理服务"
        ),
        s_kstack_doc = tr(en, "Product overview & specs", "架构说明与支持范围"),
        s_kstack_demo = tr(en, "Try live placement panel", "在线体验选卡调度面板"),
        l_klang = mega_logo("klang"),
        n_klang = tr(en, "K Language", "K 语言"),
        d_klang = tr(
            en,
            "Memory-safe systems language with C, WASI & TinyCC targets",
            "编译期生命周期检查与数据栈内存安全的系统编程语言"
        ),
        s_klang_doc = tr(
            en,
            "Tutorial, spec & benchmarks",
            "语法教程、规范与性能基准"
        ),
        s_klang_demo = tr(en, "Try live compiler panel", "在线体验编译验证面板"),
        c1_lbl = tr(en, "Compute & Systems", "算力与系统"),
        l_kstudio = mega_logo("kstudio"),
        d_kstudio = tr(
            en,
            "One native GPUI app for all your machines & agents",
            "在一个窗口里连上名下所有机器、终端与 18 种编程 Agent"
        ),
        s_kstudio_doc = tr(en, "Multi-machine workflow", "多机连接与后台常驻说明"),
        l_kwork = mega_logo("kwork"),
        d_kwork = tr(
            en,
            "Desktop client for daily agent sessions, tools & schedules",
            "日常跑任务的桌面 Agent 客户端，支持多模型切换与定时执行"
        ),
        c2_lbl = tr(en, "Machines & Desktop", "机器与工作台"),
        l_kui = mega_logo("kui"),
        s_ai = tr(en, "CLI & coding-agent guide", "kui CLI 与编程 Agent 接入"),
        c3_lbl = tr(en, "UI System", "跨端界面"),
        dev_region = tr(en, "Developer docs", "开发者文档"),
        dev_overview = tr(en, "Open Documentation Hub", "进入产品与技术文档中心"),
        l_docs = mega_logo("docs"),
        dv_hub = tr(en, "Product Documentation", "产品文档总览"),
        dv_hub_d = tr(
            en,
            "Guides for K, KStack, KStudio, KWork, and KUI",
            "K 语言、KStack、KStudio、KWork 与 KUI 功能说明"
        ),
        l_klang_dev = mega_logo("klang"),
        dv_klang = tr(en, "K Language Docs", "K 语言教程与参考"),
        dv_klang_d = tr(
            en,
            "Tutorial, language spec, Cordis microkernel & WASI guides",
            "语法教程、生命周期规范、Cordis 微内核与 WASI 构建"
        ),
        dv_install = tr(en, "KUI Setup & CLI", "KUI 安装与 CLI"),
        dv_install_d = tr(
            en,
            "Add the crate or copy component source with kui add",
            "按 Cargo 依赖引入或用 kui add 拷贝组件源码"
        ),
        dv_arch = tr(en, "How Products Fit Together", "五款产品怎么配合"),
        dv_arch_d = tr(
            en,
            "How desktop tools, KStack, K, and runtimes connect",
            "桌面端、模型算力池、K 语言与底层运行环境的配合方式"
        ),
        dc1_lbl = tr(en, "Documentation", "文档与架构"),
        l_cli = mega_logo("cli"),
        dv_comp_d = tr(
            en,
            "Live GPUI WebAssembly previews and Rust source",
            "直接在浏览器里点按预览并查看 Rust 源码"
        ),
        dv_llms_d = tr(
            en,
            "Plain-text component index for LLM tools",
            "供编程工具直接读取的纯文本组件目录"
        ),
        dc2_lbl = tr(en, "UI & Tooling", "组件与工具"),
        l_kbox = mega_logo("kbox"),
        dv_kbox_d = tr(
            en,
            "Container and libkrun MicroVM runtime middleware",
            "容器与轻量 MicroVM 运行时中间件（含 KRun 桥接）"
        ),
        l_ksb = mega_logo("ksandbox"),
        dv_ksb_d = tr(
            en,
            "OS-level process sandbox for tool subprocesses",
            "限制工具子进程文件与网络边界的轻量沙箱中间件"
        ),
        l_klang_repo = mega_logo("klang"),
        dv_klang_repo_d = tr(
            en,
            "k compiler, k lsp, Cordis & benchmark suite source",
            "k 编译器、k lsp 语言服务器与基准测试源码"
        ),
        dc3_lbl = tr(en, "Runtime & Source", "中间件与源码"),
    )
}

fn header(en: bool, company: bool, landing: bool) -> String {
    let prefix = path(en);
    let agent_prompt = "Read https://kense-labs.github.io/llms.txt to inspect KUI (Rust + GPUI desktop and WebAssembly components) and integrate the required controls into this project.";
    let agent_hint = tr(
        en,
        "Click to copy llms.txt prompt for your coding agent",
        "点击复制 llms.txt 提示词，发给你的编程 Agent",
    );
    let prod_btn = button(
        &format!(
            r#"class="company-nav-trigger" data-kense-menu-toggle="products" aria-expanded="false" aria-controls="kense-mega-products" data-kense-hint="{}""#,
            tr(
                en,
                "Browse KStack, KStudio, KWork, KUI, and K",
                "查看 KStack、KStudio、KWork、KUI 与 K 语言"
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
                "Product docs, K language, KUI components, and runtime middleware",
                "查看文档中心、K 语言、KUI 组件索引与底层中间件说明"
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
        "Try the live Rust + GPUI panel in your browser",
        "在网页里直接点按体验实时 GPUI 面板",
    );
    let arch_hint = tr(
        en,
        "See how K, KStack, KStudio, KWork, and KUI work together",
        "查看五款核心产品如何分工配合",
    );
    let gh_hint = tr(
        en,
        "Visit kense-labs on GitHub",
        "访问 kense-labs GitHub 主页",
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
            tr(en, "Live Demo", "体验"),
            tr(en, "Architecture", "架构"),
            tr(en, "Docs", "文档"),
            toolbar(en),
            icon("github", 15),
        );
    }

    let class = if landing {
        "product landing"
    } else {
        "product"
    };
    let home_href = format!("{prefix}/");
    let home_label = tr(en, "Kense home", "返回 Kense 首页");
    let kui_href = format!("{prefix}/kui/");
    let kui_label = tr(en, "KUI home", "返回 KUI 首页");
    let github = "https://github.com/kense-labs/KUI";
    format!(
        r#"<div class="kense-nav-veil" data-kense-nav-veil aria-hidden="true"></div><header class="topbar {class}" data-kense-topbar><div class="inner"><div class="company-brand-group"><div class="brand-split"><a class="brand" href="{home_href}" aria-label="{home_label}" title="{home_label}">{}</a><a class="brand-product wordmark" href="{kui_href}" aria-label="{kui_label}" title="{kui_label}">KUI</a></div></div><nav class="nav company-nav" data-kense-nav-cluster aria-label="{}">{prod_btn}{dev_btn}<a class="company-nav-link" data-nav="library" href="{prefix}/kui/library/">{}</a><a class="company-nav-link" data-nav="components" href="{prefix}/kui/components/button/">{}</a><a class="company-nav-link" data-nav="docs" href="{prefix}/kui/docs/">{}</a><a class="company-nav-link" href="{prefix}/kui/ai/">{}</a>{menus}</nav><div class="actions">{}<a class="cta" href="{github}" target="_blank" rel="noreferrer" aria-label="GitHub">{}<span>GitHub</span></a></div></div></header>"#,
        mark(),
        tr(en, "Primary navigation", "主导航"),
        tr(en, "Library", "组件总览"),
        tr(en, "Components", "组件"),
        tr(en, "Docs", "文档"),
        tr(en, "AI guide", "AI 接入"),
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
            format!("{count} 个组件画廊")
        };
        let klang_site = if en {
            "https://kense-labs.github.io/klang/"
        } else {
            "https://kense-labs.github.io/klang/README.zh-CN.html"
        };
        let klang_tutorial = if en {
            "https://kense-labs.github.io/klang/docs/tutorial.html"
        } else {
            "https://kense-labs.github.io/klang/docs/zh/tutorial.html"
        };
        return format!(
            r#"<footer class="footer-company-shell"><div class="footer-closing-card"><div class="footer-company-grid"><div class="footer-company-brand"><a class="footer-company-lockup" href="{prefix}/">{mark}<span>Kense</span></a><p class="footer-company-tagline">{tagline}</p></div><div class="footer-company-col"><h2>{c1}</h2><ul><li><a href="{prefix}/#products-kstack">KStack</a></li><li><a href="{prefix}/#products-kstudio">KStudio</a></li><li><a href="{prefix}/#products-kwork">KWork</a></li><li><a href="{prefix}/kui/">KUI</a></li><li><a href="{klang_site}">{klang_name}</a></li></ul></div><div class="footer-company-col"><h2>{c2}</h2><ul><li><a href="{prefix}/kui/components/button/">{components}</a></li><li><a href="{prefix}/kui/docs/installation/">{install}</a></li><li><a href="{prefix}/kui/ai/">{ai}</a></li><li><a href="/llms.txt">llms.txt</a></li></ul></div><div class="footer-company-col"><h2>{c3}</h2><ul><li><a href="{prefix}/docs/">{docs}</a></li><li><a href="{klang_tutorial}">{klang_docs}</a></li><li><a href="{prefix}/docs/#middleware">KBox &amp; KSandbox</a></li><li><a href="https://kense-labs.github.io/krun/">{krun}</a></li></ul></div></div><div class="footer-company-legal"><span>© Kense</span><a href="{prefix}/docs/">{docs}</a></div></div></footer>"#,
            mark = mark(),
            tagline = tr(
                en,
                "Systems programming language, GPU scheduling, multi-machine development, desktop agents, and cross-platform UI.",
                "做系统编程语言、算力调度、多机连接、桌面 Agent 与跨端 UI 软件。"
            ),
            c1 = tr(en, "Core Products", "核心产品"),
            c2 = tr(en, "KUI & Tooling", "界面与工具"),
            c3 = tr(en, "Docs & Middleware", "文档与中间件"),
            docs = tr(en, "Documentation", "产品文档中心"),
            klang_name = tr(en, "K Language", "K 语言"),
            klang_docs = tr(en, "K Language Docs", "K 语言技术文档"),
            install = tr(en, "KUI Installation", "KUI 安装指南"),
            components = components,
            ai = tr(en, "Coding-Agent Guide", "Agent 接入说明"),
            krun = tr(en, "KRun Local Bridge", "KRun 宿主桥接说明"),
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
