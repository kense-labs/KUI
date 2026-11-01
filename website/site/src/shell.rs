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
        "moon" => r#"<path d="M21 14.5A8.5 8.5 0 1 1 9.5 3 7 7 0 0 0 21 14.5z"/>"#,
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

fn header(en: bool, company: bool, landing: bool) -> String {
    let prefix = path(en);
    let (name, class, href, github, links) = if company {
        (
            "Kense",
            "company",
            format!("{prefix}/"),
            "https://github.com/kense-labs",
            format!(
                r##"<a href="{prefix}/#products">{}</a>"##,
                tr(en, "Products", "产品")
            ),
        )
    } else {
        (
            "KUI",
            if landing {
                "product landing"
            } else {
                "product"
            },
            format!("{prefix}/kui/"),
            "https://github.com/kense-labs/KUI",
            format!(
                r#"<a data-nav="components" href="{prefix}/kui/components/button/">{}</a><a data-nav="docs" href="{prefix}/kui/docs/">{}</a>"#,
                tr(en, "Components", "组件"),
                tr(en, "Docs", "文档")
            ),
        )
    };
    format!(
        r#"<header class="topbar {class}"><div class="inner"><a class="brand" href="{href}" aria-label="{name}">{}<span class="wordmark">{name}</span></a><nav class="nav" aria-label="{}">{links}</nav><div class="actions">{}<a class="cta" href="{github}" target="_blank" rel="noreferrer" aria-label="GitHub">{}<span>GitHub</span></a></div></div></header>"#,
        mark(),
        tr(en, "Primary navigation", "主导航"),
        toolbar(en),
        icon("github", 15)
    )
}

fn footer(en: bool, company: bool) -> String {
    let prefix = path(en);
    let (brand, links) = if company {
        (
            "Kense".into(),
            format!(r#"<a href="{prefix}/kui/">KUI</a>"#),
        )
    } else {
        (
            format!("{}<span>KUI</span>", mark()),
            format!(
                r#"<a href="/llms.txt">llms.txt</a> · <a href="{prefix}/kui/docs/">{}</a>"#,
                tr(en, "Docs", "文档")
            ),
        )
    };
    format!(
        r#"<footer class="footer"><div class="inner"><div class="mark">{brand}</div><p class="muted">MIT · {links}</p></div></footer>"#
    )
}

pub fn company(en: bool) -> String {
    format!(
        r#"<main class="company-main"><section class="hero wrap"><h1 class="motion-fade-up">Kense</h1><p class="muted lead motion-fade-up motion-fade-up-delay-1">{}</p></section>
<section id="products" class="wrap products"><h2 class="motion-fade-up motion-fade-up-delay-1">{}</h2>
<a class="product-card interactive lift motion-fade-up motion-fade-up-delay-2" href="{}/kui/"><div class="row"><span class="name">KUI</span><span class="tag">{}</span></div><p class="muted">{}</p><span class="cta-line">{}</span></a></section></main>"#,
        tr(
            en,
            "Focused products. Source you can own and extend.",
            "专注的产品。源码由你拥有、可扩展。"
        ),
        tr(en, "Products", "产品"),
        path(en),
        tr(en, "GPUI · Desktop + WASM", "GPUI · 桌面 + WASM"),
        tr(
            en,
            "Rust UI kit for GPUI. Use the component library or own selected component source.",
            "面向 GPUI 的 Rust UI 工具包。使用组件库，或将所需组件源码安装到项目中。"
        ),
        tr(en, "Open KUI →", "打开 KUI →")
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
