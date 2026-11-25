//! Bilingual component reference and installation guides.

use crate::{controls, esc};
use serde::Deserialize;
use std::{collections::HashMap, fmt::Write, sync::LazyLock};

pub const CSS: &str = include_str!("../docs/docs.css");
const GROUPS: &[&str] = &[
    "buttons",
    "theme-switches",
    "gestures",
    "menus",
    "text-fields",
    "text-effects",
    "media",
    "overlays",
    "navigation",
    "selects",
    "toggles",
    "expand",
    "cards",
    "special-inputs",
    "sliders",
    "pickers",
    "editors",
    "messages",
    "progress",
    "avatars",
    "charts",
    "tables",
    "activity",
    "intelligence",
    "agent-systems",
    "workspace",
    "blocks",
    "illustrations",
    "pro-motion",
    "pro-shells",
    "pro-agents",
    "pro-chat",
    "pro-search",
    "pro-onboarding",
    "pro-settings",
    "pro-workspace",
];
const SCREEN_GROUPS: &[&str] = &[
    "pro-shells",
    "pro-agents",
    "pro-chat",
    "pro-search",
    "pro-onboarding",
    "pro-settings",
    "pro-workspace",
];
const LIBRARY_INSTALL: &str = "cargo add kense-kui --rename kui --no-default-features";
const BUTTON_INSTALL: &str = "cargo install --git https://github.com/kense-labs/KUI --locked kui-cli\nkui init\nkui add button";

#[derive(Deserialize)]
struct Localized<T = String> {
    en: T,
    zh: T,
}

impl<T> Localized<T> {
    fn get(&self, en: bool) -> &T {
        if en { &self.en } else { &self.zh }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    id: String,
    title: Localized,
    group: Option<String>,
    #[serde(default)]
    wide_stage: bool,
    #[serde(default)]
    mobile_disabled: bool,
    rust_type: String,
    usage_rust: String,
    lead: Localized,
    rule: Localized,
    #[serde(flatten)]
    details: Details,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Details {
    Authored {
        copy: Localized<Body>,
    },
    Standard {
        method: String,
        signature: String,
        event: String,
        motion: Localized,
        when: Option<Localized>,
        api: Option<Localized>,
        avoid: Option<Localized>,
    },
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Body {
    when: Vec<String>,
    not: Vec<String>,
    api_lead: String,
    props: Vec<Prop>,
    motion: Vec<String>,
    ai: Vec<String>,
}

#[derive(Deserialize, Clone)]
struct Prop {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    #[serde(default)]
    default: String,
    description: String,
}

impl Entry {
    fn group(&self) -> &str {
        self.group.as_deref().unwrap_or("buttons")
    }

    fn is_screen(&self) -> bool {
        SCREEN_GROUPS.contains(&self.group())
    }

    fn body(&self, en: bool) -> Body {
        let (method, signature, event, motion, when, api, avoid) = match &self.details {
            Details::Authored { copy } => return copy.get(en).clone(),
            Details::Standard {
                method,
                signature,
                event,
                motion,
                when,
                api,
                avoid,
            } => (method, signature, event, motion, when, api, avoid),
        };
        let api_lead = api.as_ref().unwrap_or(&self.rule).get(en).clone();
        let mut not: Vec<_> = avoid.iter().map(|avoid| avoid.get(en).clone()).collect();
        let mut motions = vec![motion.get(en).clone()];
        if self.group() == "buttons" {
            not.push(
                pick(
                    en,
                    "Use Button for a simple synchronous command.",
                    "单一同步命令可直接使用基础按钮。",
                )
                .into(),
            );
            let shape = match self.id.as_str() {
                "split-button"
                | "button-group"
                | "floating-button-group"
                | "expanding-button-group"
                | "confirm-morph" => pick(
                    en,
                    "The outer control uses radius.md (8px by default). Nested controls and highlights use the same radius. Joined edges stay square.",
                    "外框使用 radius.md（默认 8px），内嵌控件与高亮使用相同圆角，拼接边保持直角。",
                ),
                _ => pick(
                    en,
                    "The control uses the host theme’s radius.md (8px by default).",
                    "控件使用宿主主题的 radius.md，默认 8px。",
                ),
            };
            motions.push(shape.into());
        }
        let mut props = vec![Prop {
            name: method.clone(),
            ty: signature.clone(),
            default: String::new(),
            description: api_lead.clone(),
        }];
        if !event.is_empty() {
            let description = if self.group() == "buttons" {
                pick(
                    en,
                    "Subscribe on the component entity.",
                    "订阅控件实体的事件。",
                )
            } else {
                pick(en, "Subscribe on the control entity.", "订阅控件实体事件。")
            };
            props.push(Prop {
                name: "event".into(),
                ty: event.clone(),
                default: String::new(),
                description: description.into(),
            });
        }
        Body {
            when: vec![when.as_ref().unwrap_or(&self.lead).get(en).clone()],
            not,
            motion: motions,
            props,
            api_lead,
            ai: vec![LIBRARY_INSTALL.into(), format!("kui add {}", self.id)],
        }
    }
}

static ENTRIES: LazyLock<Vec<Entry>> = LazyLock::new(|| {
    let mut entries = vec![
        serde_json::from_str(include_str!("../docs/button.json"))
            .expect("valid embedded Button documentation"),
    ];
    for file in [
        include_str!("../docs/family-docs.json"),
        include_str!("../docs/collection-docs.json"),
        include_str!("../docs/text-field-docs.json"),
        include_str!("../docs/workspace-docs.json"),
        include_str!("../docs/library-docs.json"),
        include_str!("../docs/inputs-extra.json"),
        include_str!("../docs/charts-extra.json"),
        include_str!("../docs/media-extra.json"),
        include_str!("../docs/loader-extra.json"),
        include_str!("../docs/provider-extra.json"),
        include_str!("../docs/ghost-extra.json"),
        include_str!("../docs/feedback-extra.json"),
        include_str!("../docs/agent-loop-extra.json"),
        include_str!("../docs/reasoning-tree-extra.json"),
        include_str!("../docs/image-generation-extra.json"),
        include_str!("../docs/speculative-extra.json"),
        include_str!("../docs/image-reveal-extra.json"),
        include_str!("../docs/prompt-cache-extra.json"),
        include_str!("../docs/meeting-summary-extra.json"),
        include_str!("../docs/ai-workflow-extra.json"),
        include_str!("../docs/training-run-extra.json"),
        include_str!("../docs/mixture-of-experts-extra.json"),
        include_str!("../docs/agent-handoff-extra.json"),
        include_str!("../docs/llm-cost-calculator-extra.json"),
        include_str!("../docs/agent-cursor-extra.json"),
        include_str!("../docs/link-unfurl-extra.json"),
        include_str!("../docs/tone-tuner-extra.json"),
        include_str!("../docs/assistant-quickstart-extra.json"),
        include_str!("../docs/theme-variants.json"),
        include_str!("../docs/navigation-extra.json"),
        include_str!("../docs/glass-extra.json"),
        include_str!("../docs/pro-screens-extra.json"),
        include_str!("../docs/observability-extra.json"),
        include_str!("../docs/affect-pro-extra.json"),
    ] {
        entries.extend(
            serde_json::from_str::<Vec<Entry>>(file)
                .expect("valid embedded component documentation"),
        );
    }
    entries
});
static LABELS: LazyLock<HashMap<String, Localized>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../docs/labels.json"))
        .expect("valid embedded documentation labels")
});

fn pick<'a>(en: bool, english: &'a str, chinese: &'a str) -> &'a str {
    if en { english } else { chinese }
}

fn t(key: &str, en: bool) -> &'static str {
    LABELS[key].get(en)
}

/// IDs in installation catalog order.
pub fn ids() -> Vec<String> {
    ENTRIES.iter().map(|entry| entry.id.clone()).collect()
}

pub fn title(section: &str, en: bool) -> &'static str {
    match section {
        "library" => t("docs.library.title", en),
        "docs" => t("docs.intro.title", en),
        "installation" => t("docs.install.title", en),
        "ai" => t("ai.title", en),
        _ => ENTRIES
            .iter()
            .find(|d| d.id == section)
            .map_or("KUI", |d| d.title.get(en)),
    }
}

pub fn description(section: &str, en: bool) -> &'static str {
    match section {
        "library" => t("docs.library.lead", en),
        "docs" => t("docs.intro.lead", en),
        "installation" => t("docs.install.lead", en),
        "ai" => t("ai.lead", en),
        _ => ENTRIES
            .iter()
            .find(|d| d.id == section)
            .map_or("", |d| d.lead.get(en)),
    }
}

fn route(section: &str, en: bool) -> String {
    let prefix = pick(en, "/en/kui", "/kui");
    match section {
        "library" | "docs" | "ai" => format!("{prefix}/{section}/"),
        "installation" => format!("{prefix}/docs/installation/"),
        _ => format!("{prefix}/components/{section}/"),
    }
}

fn icon(name: &str, size: u8) -> String {
    let path = match name {
        "search" => r#"<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>"#,
        "grid" => {
            r#"<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>"#
        }
        "replay" => r#"<path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/>"#,
        "spark" => {
            r#"<path d="M12 2.5 14.3 9.7 21.5 12l-7.2 2.3L12 21.5l-2.3-7.2L2.5 12l7.2-2.3L12 2.5Z"/>"#
        }
        "book" => {
            r#"<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>"#
        }
        "box" => {
            r#"<path d="m11 21-8-4.5v-9L11 3l8 4.5v9L11 21zM11 12 3 7.5M11 12l8-4.5M11 12v9"/>"#
        }
        "screens" => {
            r#"<rect x="2" y="4" width="20" height="16" rx="2"/><path d="M10 4v4M2 8h20M6 4v4"/>"#
        }
        "copy" => {
            r#"<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/>"#
        }
        "check" => r#"<path d="M5 12.5 9.5 17 19 7.5"/>"#,
        "chevron" => r#"<path d="m6 9 6 6 6-6"/>"#,
        "preview" => {
            r#"<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>"#
        }
        "code" => r#"<path d="m8 8-4 4 4 4M16 8l4 4-4 4M14 5l-4 14"/>"#,
        "fit" => {
            r#"<path d="M8 3H5a2 2 0 0 0-2 2v3m13-5h3a2 2 0 0 1 2 2v3M3 16v3a2 2 0 0 0 2 2h3m13-5v3a2 2 0 0 1-2 2h-3"/>"#
        }
        "mobile" => r#"<rect x="6" y="2" width="12" height="20" rx="2"/><path d="M11 18h2"/>"#,
        "expand" => r#"<path d="M14 4h6v6M20 4l-7 7m-3 9H4v-6m0 6 7-7"/>"#,
        "close" => r#"<path d="m6 6 12 12M18 6 6 18"/>"#,
        "terminal" => r#"<path d="M4 17 10 11 4 5M12 19h8"/>"#,
        "file" => {
            r#"<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6M8 13h8M8 17h8M8 9h2"/>"#
        }
        _ => unreachable!("known documentation icon"),
    };
    format!(
        r#"<svg aria-hidden="true" width="{size}" height="{size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">{path}</svg>"#
    )
}

fn nav_row(id: &str, current: &str, label: &str, terms: &str, en: bool) -> String {
    format!(
        r#"<a href="{}" class="row" data-nav-item data-doc-id="{}" data-search-terms="{}"{}><span class="row-label">{}</span></a>"#,
        route(id, en),
        esc(id),
        esc(&terms.to_lowercase()),
        if id == current {
            r#" aria-current="page""#
        } else {
            ""
        },
        esc(label)
    )
}

fn section(id: &str, label: &str, count: Option<usize>, symbol: &str, content: &str) -> String {
    let count = count
        .map(|n| format!(r#"<span class="section-count" data-section-count>{n}</span>"#))
        .unwrap_or_default();
    let head = controls::button(
        r#"class="section-head" data-section-toggle aria-expanded="true""#,
        &format!(
            r#"<span class="section-icon">{}</span><span class="section-label">{label}</span>{count}<span class="chevron">{}</span>"#,
            icon(symbol, 15),
            icon("chevron", 14)
        ),
    );
    format!(
        r#"<section class="section" data-section data-section-id="{id}" data-open="true">{head}<div class="section-body" data-section-body><div class="disclosure-clip">{content}</div></div></section>"#
    )
}

fn render_groups(group_ids: &[&str], current: &str, en: bool) -> (String, usize) {
    let mut groups = String::new();
    let mut total = 0;
    for group in group_ids {
        let label = t(&format!("sidebar.group.{group}"), en);
        let mut rows = String::new();
        let mut count = 0;
        for entry in ENTRIES.iter().filter(|item| item.group() == *group) {
            let terms = format!(
                "{} {} {} {}",
                entry.id, entry.rust_type, entry.title.en, entry.title.zh
            );
            rows.push_str(&nav_row(
                &entry.id,
                current,
                entry.title.get(en),
                &terms,
                en,
            ));
            count += 1;
        }
        if count == 0 {
            continue;
        }
        total += count;
        let head = controls::button(
            r#"class="group-head" data-group-toggle aria-expanded="true""#,
            &format!(
                r#"<span class="group-chevron">{}</span><span class="group-label">{label}</span><span class="group-count" data-group-count>{count}</span>"#,
                icon("chevron", 12)
            ),
        );
        write!(groups,r#"<div class="group" data-group data-group-id="{group}" data-open="true">{head}<nav class="group-body" data-group-body aria-label="{label}"><div class="disclosure-clip"><div class="rows">{rows}</div></div></nav></div>"#).unwrap();
    }
    (groups, total)
}

fn sidebar(current: &str, en: bool) -> String {
    let doc_items = ["library", "docs", "installation", "ai"];
    let doc_label = t("sidebar.docs", en);
    let total_entries = ENTRIES.len();
    let doc_rows = doc_items
        .map(|id| {
            if id == "library" {
                format!(
                    r#"<a href="{}" class="row row-library" data-nav-item data-doc-id="library" data-search-terms="library gallery overview 组件总览 画廊 目录"{}><span class="row-label">{}</span><span class="group-count">{total_entries}</span></a>"#,
                    route("library", en),
                    if current == "library" {
                        r#" aria-current="page""#
                    } else {
                        ""
                    },
                    esc(title("library", en))
                )
            } else {
                nav_row(
                    id,
                    current,
                    title(id, en),
                    &format!("{} {}", title(id, true), title(id, false)),
                    en,
                )
            }
        })
        .join("");
    let guides = section(
        "docs",
        doc_label,
        None,
        "book",
        &format!(r#"<nav aria-label="{doc_label}" class="rows flat">{doc_rows}</nav>"#),
    );
    let component_group_ids: Vec<&str> = GROUPS
        .iter()
        .copied()
        .filter(|g| !SCREEN_GROUPS.contains(g))
        .collect();
    let (component_groups, component_count) = render_groups(&component_group_ids, current, en);
    let (screen_groups, screen_count) = render_groups(SCREEN_GROUPS, current, en);
    let components = section(
        "components",
        t("sidebar.components", en),
        Some(component_count),
        "box",
        &component_groups,
    );
    let screens = section(
        "screens",
        t("sidebar.screens", en),
        Some(screen_count),
        "screens",
        &screen_groups,
    );
    let placeholder = t("sidebar.search.placeholder", en);
    let search = controls::search(&format!(
        r#"data-sidebar-search placeholder="{placeholder}" aria-label="{placeholder}" autocomplete="off" spellcheck="false" enterkeyhint="go""#
    ));
    format!(
        r#"<aside class="aside" data-docs-sidebar aria-label="{}"><div class="panel"><div class="top"><label class="search"><span class="search-icon">{}</span>{search}<kbd data-search-kbd aria-hidden="true">/</kbd></label><span class="sr-only" data-search-status role="status"></span></div><div class="scroll" data-nav-scroll><div class="content" data-nav-root><div class="pill" data-nav-pill data-hidden="1" aria-hidden="true"></div>{guides}{components}{screens}<p class="empty" data-empty hidden>{}</p></div></div></div></aside>"#,
        pick(en, "Library", "组件库"),
        icon("search", 15),
        t("sidebar.empty", en)
    )
}

fn list(items: &[String]) -> String {
    let rows: String = items
        .iter()
        .map(|s| format!("<li>{}</li>", esc(s)))
        .collect();
    format!("<ul>{rows}</ul>")
}

fn props(body: &Body, en: bool) -> String {
    let labels = [
        "button.props.prop",
        "button.props.type",
        "button.props.default",
        "button.props.desc",
    ]
    .map(|key| t(key, en));
    let mut html = format!(
        r#"<div class="props" role="table" aria-label="{}"><div class="row head" role="row">"#,
        t("button.props", en)
    );
    for label in labels {
        write!(html, r#"<span role="columnheader">{label}</span>"#).unwrap();
    }
    html.push_str("</div>");
    for prop in &body.props {
        let default = if prop.default.is_empty() {
            r#"<span class="dash">–</span>"#.into()
        } else {
            format!(r#"<code class="type">{}</code>"#, esc(&prop.default))
        };
        write!(html,r#"<div class="row" role="row"><span role="cell" class="name"><code>{}</code></span><span role="cell" data-label="{}"><code class="type">{}</code></span><span role="cell" data-label="{}">{default}</span><span role="cell" class="desc" data-label="{}">{}</span></div>"#,esc(&prop.name),labels[1],esc(&prop.ty),labels[2],labels[3],esc(&prop.description)).unwrap();
    }
    html.push_str("</div>");
    html
}

fn code_panel(title: &str, source: &str, en: bool) -> String {
    let shell = title == "shell";
    let lines = crate::syntax::highlight(source, shell);
    let copy = controls::button(
        &format!(r#"class="copy" data-copy="{}""#, esc(source)),
        &format!(
            r#"<span class="state" data-copy-idle>{}{}</span><span class="state" data-copy-done>{}{}</span>"#,
            icon("copy", 14),
            pick(en, "Copy", "复制"),
            icon("check", 14),
            pick(en, "Copied", "已复制")
        ),
    );
    format!(
        r#"<figure class="code with-lines" data-code-panel><figcaption class="head"><span class="file">{}<span>{}</span></span>{copy}</figcaption><div class="body" data-code-body><pre tabindex="0"><code>{lines}</code></pre></div></figure>"#,
        icon(if shell { "terminal" } else { "file" }, 14),
        esc(title)
    )
}

fn source_picker(id: &str, en: bool) -> String {
    format!(
        r#"<details class="source" data-item-source="{id}" data-locale="{}"><summary>{}</summary><div class="files"><select aria-label="{}" hidden></select><a href="/r/{id}.json">{}</a></div><p role="status" hidden></p><pre hidden tabindex="0"><code></code></pre></details>"#,
        pick(en, "en", "zh"),
        pick(en, "View installable source", "查看安装源码"),
        pick(en, "Choose Rust file", "选择 Rust 文件"),
        pick(en, "Registry JSON", "注册表 JSON")
    )
}

fn preview_url(story: &str, en: bool, revision: &str) -> String {
    format!(
        "/stories/v/{}/?story={}&amp;lang={}",
        esc(revision),
        esc(&story.replace(':', "%3A")),
        pick(en, "en", "zh")
    )
}

fn stage(doc: &Entry, en: bool, revision: &str, example: &str) -> String {
    let id = &doc.id;
    let title = esc(doc.title.get(en));
    let locale = pick(en, "en", "zh");
    let mut tabs = String::new();
    for (view, selected) in [("preview", true), ("code", false)] {
        let attrs = format!(
            r#"class="tab" role="tab" data-view="{view}" id="stage-tab-{view}" aria-controls="stage-panel-{view}" aria-selected="{selected}" tabindex="{}""#,
            if selected { "0" } else { "-1" }
        );
        let inner = format!(
            r#"<span class="tab-inner">{}<span>{}</span></span>"#,
            icon(view, 15),
            t(&format!("stage.{view}"), en)
        );
        tabs.push_str(&controls::button(&attrs, &inner));
    }
    let theme_class = if id == "theme-switch" || id.starts_with("theme-switch-") {
        " theme-preview"
    } else if doc.is_screen() {
        " screen-story"
    } else if id == "gradient-floor" {
        " pro-motion-story"
    } else if doc.wide_stage
        || matches!(
            doc.group(),
            "pro-motion" | "illustrations" | "blocks" | "agent-systems"
        )
    {
        " pro-stage-story"
    } else {
        ""
    };
    let url = preview_url(id, en, revision);
    let mobile_disabled = doc.mobile_disabled
        || doc.is_screen()
        || matches!(doc.group(), "pro-motion" | "illustrations" | "blocks");
    let fit = pick(en, "Fit preview", "自适应预览");
    let mobile = if mobile_disabled {
        pick(en, "Mobile preview unavailable", "无手机版预览")
    } else {
        pick(en, "Mobile preview", "手机预览")
    };
    let expand = pick(en, "Fullscreen preview", "全屏预览");
    let collapse = pick(en, "Exit fullscreen preview", "退出全屏预览");
    let replay_label = pick(en, "Replay preview", "重播动效与重置状态");
    let ai_prompt = if en {
        format!(
            "Integrate KUI component `{id}` (`kui::components::*` -> `{}`).\nSpec: https://kense-labs.github.io/md/{id}.md\nCatalog: https://kense-labs.github.io/llms.txt\nCLI: kui add {id}",
            doc.rust_type
        )
    } else {
        format!(
            "请在当前 Rust + GPUI 项目中接入 KUI 组件 `{id}`（类型 `{}`）。\n组件规格：https://kense-labs.github.io/md/{id}.md\n目录索引：https://kense-labs.github.io/llms.txt\n源码安装：kui add {id}",
            doc.rust_type
        )
    };
    let ai_btn = controls::button(
        &format!(
            r#"class="stage-chip" data-copy="{}" data-kind="prompt" title="{}""#,
            esc(&ai_prompt),
            pick(
                en,
                "Copy prompt for coding agent",
                "复制给编程 Agent 的接入提示词"
            )
        ),
        &format!(
            r#"<span class="state" data-copy-idle>{}<span>{}</span></span><span class="state" data-copy-done>{}<span>{}</span></span>"#,
            icon("spark", 13),
            pick(en, "AI Prompt", "AI 提示词"),
            icon("check", 13),
            pick(en, "Copied", "已复制")
        ),
    );
    let cli_cmd = format!("kui add {id}");
    let cli_btn = controls::button(
        &format!(
            r#"class="stage-chip stage-cli" data-copy="{}" data-kind="command" title="{}""#,
            esc(&cli_cmd),
            pick(en, "Copy CLI install command", "复制 CLI 源码安装命令")
        ),
        &format!(
            r#"<span class="state" data-copy-idle>{}<code>kui add {id}</code></span><span class="state" data-copy-done>{}<span>{}</span></span>"#,
            icon("terminal", 13),
            icon("check", 13),
            pick(en, "Copied", "已复制")
        ),
    );
    let size_switch = [
        ("fit", fit, true, false),
        ("mobile", mobile, false, mobile_disabled),
    ]
    .map(|(mode, label, selected, disabled)| {
        let extra = if disabled {
            r#" aria-disabled="true" disabled"#
        } else {
            ""
        };
        controls::button(
            &format!(
                r#"class="preview-control" role="radio" data-preview-mode="{mode}" aria-label="{label}" title="{label}" aria-checked="{selected}" tabindex="{}"{extra}"#,
                if selected { 0 } else { -1 }
            ),
            &icon(mode, 14),
        )
    })
    .join("");
    let replay = controls::button(
        &format!(
            r#"class="preview-control preview-replay" data-preview-replay aria-label="{replay_label}" title="{replay_label}""#
        ),
        &icon("replay", 14),
    );
    let fullscreen = controls::button(
        &format!(
            r#"class="preview-control preview-expand" data-preview-expand aria-label="{expand}" title="{expand}" aria-haspopup="dialog""#
        ),
        &icon("expand", 15),
    );
    let close = controls::button(
        &format!(
            r#"class="preview-control preview-close" data-preview-close aria-label="{collapse}" title="{collapse}""#
        ),
        &icon("close", 16),
    );
    let mut html = format!(
        r#"<section class="stage" data-component-stage><div class="view-bar"><div class="tabs" role="tablist" aria-label="{}"><span class="indicator" data-view-indicator aria-hidden="true"></span>{tabs}</div><div class="stage-bar-end"><div class="stage-copy-group">{cli_btn}{ai_btn}</div><div class="preview-options" data-preview-options><div class="preview-tools"><div class="preview-modes" role="radiogroup" aria-label="{}"><span class="preview-highlight" aria-hidden="true"></span>{size_switch}</div>{replay}{fullscreen}</div></div></div></div><div class="preview-fullscreen-bar" hidden><span id="stage-fullscreen-title">{title}</span>{close}</div><div class="panel is-active" role="tabpanel" id="stage-panel-preview" data-panel="preview" aria-labelledby="stage-tab-preview"><div data-preview-slot data-preview-size="fit" data-preview-src="{url}"><div data-preview-device><div data-preview-screen><iframe allow="clipboard-write" class="rust-story family-story{theme_class}" data-story="{id}" data-story-key="{id}" data-lang="{locale}" aria-label="{title}" src="{url}"></iframe></div></div></div></div><div class="panel" role="tabpanel" id="stage-panel-code" data-panel="code" aria-labelledby="stage-tab-code" hidden>{example}{}</div></section>"#,
        t("stage.views", en),
        pick(en, "Preview size", "预览尺寸"),
        source_picker(id, en)
    );
    if id == "theme-switch" {
        write!(
            html,
            "<section><h2>{}</h2>",
            pick(en, "More transitions", "其他过渡示例")
        )
        .unwrap();
        for (mode, label) in [("eclipse", "Eclipse"), ("split", "Split"), ("rise", "Rise")] {
            let key = format!("theme-switch:{mode}");
            write!(html,r#"<details><summary>{label}</summary><iframe allow="clipboard-write" loading="lazy" data-src="{}" class="rust-story family-story theme-preview" data-story="{id}" data-story-key="{key}" data-lang="{locale}" aria-label="{title} · {label}"></iframe></details>"#,preview_url(&key,en,revision)).unwrap();
        }
        html.push_str("</section>");
    }
    html
}

fn component(doc: &Entry, en: bool, revision: &str) -> String {
    let id = &doc.id;
    let body = doc.body(en);
    let file = format!("{}.rs", id.replace('-', "_"));
    let example = code_panel(&file, &doc.usage_rust, en);
    let section_key = if doc.is_screen() {
        "sidebar.screens"
    } else {
        "sidebar.components"
    };
    let crumb = format!(
        "{} · {}",
        t(section_key, en),
        t(&format!("sidebar.group.{}", doc.group()), en)
    );
    let mut html = format!(
        r#"<article class="doc" data-doc-id="{id}"><p class="muted crumb">{crumb}</p><h1>{}</h1><p class="muted lead">{} <a href="/md/{id}.md">/md/{id}.md</a></p>{}<h2>{}</h2><p>{}</p>"#,
        esc(doc.title.get(en)),
        esc(doc.lead.get(en)),
        stage(doc, en, revision, &example),
        pick(en, "Keyboard and interaction", "键盘与操作"),
        esc(doc.rule.get(en))
    );
    for (key, items) in [("button.when", &body.when), ("button.not", &body.not)] {
        if !items.is_empty() {
            write!(html, "<h2>{}</h2>{}", t(key, en), list(items)).unwrap();
        }
    }
    let install = if id == "button" {
        BUTTON_INSTALL.into()
    } else {
        format!("kui add {id}")
    };
    write!(html,r#"<h2>{}</h2><h3>{}</h3>{}<p>{}</p><h3>{}</h3><p>{}</p>{}<h2>{}</h2><p class="muted usage-lead">{}</p>{example}<h2>{}</h2><h3><code>{}</code></h3><p class="muted usage-lead">{}</p>{}"#,t("button.install",en),t("docs.install.library",en),code_panel("shell",LIBRARY_INSTALL,en),esc(t("docs.install.library.body",en)),t("docs.install.cli",en),esc(t("docs.install.cli.body",en)),code_panel("shell",&install,en),t("button.usage",en),t("button.usage.lead",en),t("button.api",en),esc(&doc.rust_type),esc(&body.api_lead),props(&body,en)).unwrap();
    for (key, items) in [("button.motion", &body.motion), ("button.ai", &body.ai)] {
        write!(html, "<h2>{}</h2>{}", t(key, en), list(items)).unwrap();
    }
    html.push_str("</article>");
    html
}

fn guide(id: &str, en: bool) -> String {
    let mut html = format!(
        r#"<article class="doc guide" data-doc-id="{id}"><p class="muted crumb">{}</p><h1>{}</h1><p class="muted lead">{}</p>"#,
        t("docs.crumb", en),
        title(id, en),
        description(id, en)
    );
    let heading = |key| format!("<h2>{}</h2>", t(key, en));
    let link = |section| {
        format!(
            r#"<a href="{}">{}</a>"#,
            route(section, en),
            title(section, en)
        )
    };
    let link_file = |path| format!(r#"<a href="{path}"><code>{path}</code></a>"#);
    let items = |keys: &[&str]| {
        list(
            &keys
                .iter()
                .map(|key| t(key, en).to_owned())
                .collect::<Vec<_>>(),
        )
    };
    match id {
        "docs" => {
            write!(
                html,
                "{}<p>{}</p>{}<ul>",
                heading("docs.intro.what"),
                t("docs.intro.what.body", en),
                heading("docs.intro.org")
            )
            .unwrap();
            for key in ["docs.intro.org.f", "docs.intro.org.c", "docs.intro.org.p"] {
                let (name, description) = t(key, en)
                    .split_once(" — ")
                    .expect("named documentation layer");
                write!(html, "<li><strong>{name}</strong> — {description}</li>").unwrap();
            }
            write!(
                html,
                "</ul>{}{}{}<ul><li>{}</li><li>{}</li><li>{}</li></ul>",
                heading("docs.intro.read"),
                items(&[
                    "docs.intro.read.1",
                    "docs.intro.read.2",
                    "docs.intro.read.3",
                    "docs.intro.read.4"
                ]),
                heading("docs.intro.next"),
                link("installation"),
                format_args!(r#"<a href="{}">Button</a>"#, route("button", en)),
                link("ai")
            )
            .unwrap();
        }
        "installation" => {
            write!(html,r#"{}{}<p>{}</p>{}<p>{}</p>{}<p>{}</p>{}{}{}{}<ul><li>AGENTS.md · DESIGN.md · docs/PLATFORMS.md</li><li><a href="https://crates.io/crates/kense-kui">crates.io</a> · <a href="https://docs.rs/kense-kui">Rust API</a></li><li>{} · <a href="/llms.txt">llms.txt</a></li></ul>"#,heading("docs.install.library"),code_panel("shell",LIBRARY_INSTALL,en),esc(t("docs.install.library.body",en)),heading("docs.install.features"),esc(t("docs.install.features.body",en)),heading("docs.install.cli"),esc(t("docs.install.cli.body",en)),code_panel("shell",&format!("{BUTTON_INSTALL}\nkui doctor"),en),heading("docs.install.hosts"),items(&["docs.install.host.desktop","docs.install.host.wasm"]),heading("docs.install.also"),link("ai")).unwrap();
        }
        "ai" => {
            write!(html,"{}<ol><li>{}</li><li>{}</li><li>{}</li><li>AGENTS.md</li></ol>{}{}<p>{}</p><h3>{}</h3><p>{}</p>{}{}<ul>",heading("ai.start"),link_file("/llms.txt"),link_file("/md/button.md"),link("installation"),heading("ai.install"),code_panel("shell",LIBRARY_INSTALL,en),esc(t("docs.install.library.body",en)),t("docs.install.cli",en),esc(t("docs.install.cli.body",en)),code_panel("shell",BUTTON_INSTALL,en),heading("ai.registry")).unwrap();
            for path in [
                "/r/catalog.json",
                "/r/button.json",
                "/r/kui-foundation.json",
            ] {
                write!(html, "<li>{}</li>", link_file(path)).unwrap();
            }
            write!(
                html,
                "</ul>{}{}",
                heading("ai.rules"),
                items(&["ai.rule.1", "ai.rule.2", "ai.rule.3", "ai.rule.4"])
            )
            .unwrap();
        }
        _ => unreachable!("known guide"),
    }
    html.push_str("</article>");
    html
}

fn library_category(group: &str) -> &'static str {
    match group {
        "agent-systems" | "pro-agents" | "intelligence" | "activity" => "agents",
        "blocks" | "workspace" | "cards" | "tables" => "blocks",
        "pro-shells" | "pro-chat" | "pro-search" | "pro-onboarding" | "pro-settings"
        | "pro-workspace" => "screens",
        "pro-motion" | "illustrations" | "theme-switches" | "gestures" | "text-effects"
        | "media" => "motion",
        "charts" | "progress" => "charts",
        _ => "controls",
    }
}

fn library_thumb(id: &str, group: &str) -> &'static str {
    match id {
        "approval-card" | "agent-checkpoint" | "agent-steer" | "confirm-popover" => {
            r#"<svg viewBox="0 0 220 92" fill="none" aria-hidden="true"><rect x="44" y="12" width="108" height="16" rx="8" fill="currentColor" fill-opacity=".07" stroke="currentColor" stroke-opacity=".22"/><circle cx="32" cy="20" r="6" fill="currentColor" fill-opacity=".45"/><rect x="34" y="34" width="152" height="46" rx="9" fill="currentColor" fill-opacity=".06" stroke="currentColor" stroke-opacity=".28"/><rect x="46" y="44" width="64" height="6" rx="3" fill="currentColor" fill-opacity=".38"/><rect x="148" y="42" width="26" height="10" rx="5" fill="currentColor" fill-opacity=".18"/><rect x="140" y="64" width="34" height="10" rx="5" fill="currentColor" fill-opacity=".62"/></svg>"#
        }
        "app-directory" => {
            r#"<svg viewBox="0 0 220 92" fill="none" aria-hidden="true"><rect x="20" y="12" width="180" height="14" rx="5" stroke="currentColor" stroke-opacity=".22"/><rect x="20" y="34" width="86" height="22" rx="6" fill="currentColor" fill-opacity=".06" stroke="currentColor" stroke-opacity=".24"/><rect x="114" y="34" width="86" height="22" rx="6" fill="currentColor" fill-opacity=".06" stroke="currentColor" stroke-opacity=".24"/><rect x="20" y="62" width="86" height="20" rx="6" stroke="currentColor" stroke-opacity=".18"/><rect x="114" y="62" width="86" height="20" rx="6" stroke="currentColor" stroke-opacity=".18"/><rect x="28" y="40" width="10" height="10" rx="3" fill="currentColor" fill-opacity=".48"/><rect x="122" y="40" width="10" height="10" rx="3" fill="currentColor" fill-opacity=".32"/></svg>"#
        }
        "cost-waterfall" | "trace-waterfall" => {
            r#"<svg viewBox="0 0 220 92" fill="none" aria-hidden="true"><rect x="20" y="14" width="180" height="64" rx="7" stroke="currentColor" stroke-opacity=".24"/><rect x="84" y="24" width="96" height="7" rx="3.5" fill="currentColor" fill-opacity=".48"/><rect x="96" y="38" width="44" height="6" rx="3" fill="currentColor" fill-opacity=".32"/><rect x="132" y="51" width="36" height="6" rx="3" fill="currentColor" fill-opacity=".26"/><rect x="156" y="63" width="26" height="6" rx="3" fill="currentColor" fill-opacity=".38"/><path d="M32 27h36M38 41h34M38 54h30M32 66h38" stroke="currentColor" stroke-opacity=".28" stroke-width="2" stroke-linecap="round"/></svg>"#
        }
        _ => match group {
            "agent-systems" | "pro-agents" | "intelligence" => {
                r#"<svg viewBox="0 0 220 92" fill="none" aria-hidden="true"><rect x="86" y="28" width="48" height="36" rx="8" fill="currentColor" fill-opacity=".08" stroke="currentColor" stroke-opacity=".28"/><circle cx="110" cy="46" r="7" fill="currentColor" fill-opacity=".55"/><rect x="20" y="16" width="38" height="24" rx="6" stroke="currentColor" stroke-opacity=".2"/><rect x="20" y="52" width="38" height="24" rx="6" stroke="currentColor" stroke-opacity=".2"/><rect x="162" y="16" width="38" height="24" rx="6" stroke="currentColor" stroke-opacity=".2"/><rect x="162" y="52" width="38" height="24" rx="6" stroke="currentColor" stroke-opacity=".2"/><path d="M58 28h14v14h14M58 64h14V50h14M134 42h14V28h14M134 50h14v14h14" stroke="currentColor" stroke-opacity=".26" stroke-linecap="round" stroke-linejoin="round"/></svg>"#
            }
            "blocks" | "workspace" => {
                r#"<svg viewBox="0 0 220 92" fill="none" aria-hidden="true"><rect x="18" y="12" width="56" height="28" rx="6" fill="currentColor" fill-opacity=".05" stroke="currentColor" stroke-opacity=".2"/><rect x="82" y="12" width="56" height="28" rx="6" fill="currentColor" fill-opacity=".05" stroke="currentColor" stroke-opacity=".2"/><rect x="146" y="12" width="56" height="28" rx="6" fill="currentColor" fill-opacity=".05" stroke="currentColor" stroke-opacity=".2"/><rect x="18" y="48" width="184" height="32" rx="6" stroke="currentColor" stroke-opacity=".22"/><path d="M28 70c18-2 26-12 44-10s24 8 42-4 28-6 48-2 18 4 30 0" stroke="currentColor" stroke-opacity=".45" stroke-width="1.5" stroke-linecap="round"/></svg>"#
            }
            "pro-shells" | "pro-chat" | "pro-search" | "pro-onboarding" | "pro-settings"
            | "pro-workspace" => {
                r#"<svg viewBox="0 0 220 92" fill="none" aria-hidden="true"><rect x="16" y="10" width="188" height="72" rx="7" stroke="currentColor" stroke-opacity=".24"/><path d="M64 10v72" stroke="currentColor" stroke-opacity=".18"/><rect x="23" y="18" width="34" height="7" rx="3.5" fill="currentColor" fill-opacity=".14"/><rect x="23" y="31" width="28" height="5" rx="2.5" fill="currentColor" fill-opacity=".08"/><rect x="23" y="41" width="32" height="5" rx="2.5" fill="currentColor" fill-opacity=".08"/><rect x="76" y="20" width="114" height="24" rx="5" fill="currentColor" fill-opacity=".06" stroke="currentColor" stroke-opacity=".18"/><rect x="76" y="52" width="54" height="20" rx="5" stroke="currentColor" stroke-opacity=".16"/><rect x="136" y="52" width="54" height="20" rx="5" stroke="currentColor" stroke-opacity=".16"/></svg>"#
            }
            "pro-motion" | "illustrations" | "theme-switches" | "gestures" | "text-effects"
            | "media" => {
                r#"<svg viewBox="0 0 220 92" fill="none" aria-hidden="true"><rect x="64" y="16" width="92" height="54" rx="10" transform="rotate(-4 110 43)" fill="currentColor" fill-opacity=".04" stroke="currentColor" stroke-opacity=".18"/><rect x="68" y="22" width="92" height="54" rx="10" fill="currentColor" fill-opacity=".08" stroke="currentColor" stroke-opacity=".32"/><path d="M82 40h42M82 50h58M82 60h30" stroke="currentColor" stroke-opacity=".36" stroke-width="2" stroke-linecap="round"/><circle cx="144" cy="38" r="6" fill="currentColor" fill-opacity=".22"/></svg>"#
            }
            "charts" | "progress" => {
                r#"<svg viewBox="0 0 220 92" fill="none" aria-hidden="true"><path d="M22 72h176" stroke="currentColor" stroke-opacity=".16"/><path d="M26 62c22-4 32-24 54-20s28 14 50-8 34-14 64-18" stroke="currentColor" stroke-opacity=".55" stroke-width="1.75" stroke-linecap="round"/><path d="M26 68c24-2 36-10 58-8s32 4 54-6 30-6 56-8" stroke="currentColor" stroke-opacity=".24" stroke-width="1.35" stroke-dasharray="3 3" stroke-linecap="round"/><circle cx="130" cy="34" r="3.5" fill="currentColor" fill-opacity=".75"/></svg>"#
            }
            "tables" | "cards" | "activity" => {
                r#"<svg viewBox="0 0 220 92" fill="none" aria-hidden="true"><rect x="20" y="14" width="180" height="64" rx="7" stroke="currentColor" stroke-opacity=".22"/><path d="M20 32h180M20 50h180M20 66h180" stroke="currentColor" stroke-opacity=".12"/><rect x="30" y="21" width="38" height="5" rx="2.5" fill="currentColor" fill-opacity=".26"/><rect x="30" y="39" width="56" height="5" rx="2.5" fill="currentColor" fill-opacity=".18"/><rect x="30" y="56" width="48" height="5" rx="2.5" fill="currentColor" fill-opacity=".18"/><rect x="158" y="37" width="30" height="9" rx="4.5" fill="currentColor" fill-opacity=".12"/></svg>"#
            }
            _ => {
                r#"<svg viewBox="0 0 220 92" fill="none" aria-hidden="true"><rect x="34" y="28" width="88" height="36" rx="8" fill="currentColor" fill-opacity=".07" stroke="currentColor" stroke-opacity=".28"/><rect x="48" y="43" width="44" height="6" rx="3" fill="currentColor" fill-opacity=".45"/><rect x="134" y="34" width="52" height="24" rx="12" fill="currentColor" fill-opacity=".1" stroke="currentColor" stroke-opacity=".26"/><circle cx="174" cy="46" r="8" fill="currentColor" fill-opacity=".65"/></svg>"#
            }
        },
    }
}

fn library_gallery(en: bool) -> String {
    let total = ENTRIES.len();
    let cats = [
        ("all", pick(en, "All", "全部")),
        ("agents", pick(en, "Agents", "智能体")),
        ("blocks", pick(en, "Blocks", "业务区块")),
        ("screens", pick(en, "Screens", "应用屏幕")),
        ("motion", pick(en, "Motion & Illustrations", "动效与插画")),
        ("charts", pick(en, "Charts & Data", "图表与进度")),
        ("controls", pick(en, "Controls & Inputs", "基础与输入")),
    ];
    let mut filter_pills = String::new();
    for (cat_id, cat_label) in cats {
        let cnt = if cat_id == "all" {
            total
        } else {
            ENTRIES
                .iter()
                .filter(|e| library_category(e.group()) == cat_id)
                .count()
        };
        let active = cat_id == "all";
        let _ = write!(
            filter_pills,
            "{}",
            controls::button(
                &format!(
                    r#"class="lib-pill" data-library-filter="{cat_id}" aria-pressed="{active}""#
                ),
                &format!(r#"<span>{cat_label}</span><span class="lib-pill-count">{cnt}</span>"#),
            )
        );
    }

    let search_ph = pick(
        en,
        "Filter by name, Rust struct, or capability…",
        "按组件名、Rust 类型或场景筛选…",
    );
    let search_input = controls::search(&format!(
        r#"data-library-search placeholder="{search_ph}" aria-label="{search_ph}" autocomplete="off" spellcheck="false""#
    ));

    let featured_ids = [
        "mission-control",
        "agent-graph",
        "approval-card",
        "app-directory",
        "prompt-explorer",
        "glass-folder",
    ];
    let mut featured_cards = String::new();
    for fid in featured_ids {
        if let Some(entry) = ENTRIES.iter().find(|e| e.id == fid) {
            let grp = entry.group();
            let grp_label = t(&format!("sidebar.group.{grp}"), en);
            let _ = write!(
                featured_cards,
                r#"<a class="lib-card lib-card-featured" href="{}" data-nav-item data-doc-id="{}" data-library-card data-library-cat="{}"><div class="lib-thumb">{}<span class="lib-badge">{}</span></div><div class="lib-meta"><div class="lib-title-row"><span class="lib-name">{}</span><code class="lib-type">{}</code></div><p class="lib-desc">{}</p></div></a>"#,
                route(&entry.id, en),
                esc(&entry.id),
                library_category(grp),
                library_thumb(&entry.id, grp),
                esc(grp_label),
                esc(entry.title.get(en)),
                esc(&entry.rust_type),
                esc(entry.lead.get(en)),
            );
        }
    }

    let mut sections = String::new();
    for &grp in GROUPS {
        let items: Vec<&Entry> = ENTRIES.iter().filter(|e| e.group() == grp).collect();
        if items.is_empty() {
            continue;
        }
        let cat = library_category(grp);
        let grp_label = t(&format!("sidebar.group.{grp}"), en);
        let mut cards = String::new();
        for entry in &items {
            let terms = format!(
                "{} {} {} {} {}",
                entry.id,
                entry.rust_type,
                entry.title.en,
                entry.title.zh,
                entry.lead.get(en)
            )
            .to_lowercase();
            let _ = write!(
                cards,
                r#"<a class="lib-card" href="{}" data-nav-item data-doc-id="{}" data-library-card data-library-cat="{cat}" data-search-terms="{}"><div class="lib-thumb">{}</div><div class="lib-meta"><div class="lib-title-row"><span class="lib-name">{}</span><code class="lib-type">{}</code></div><p class="lib-desc">{}</p></div></a>"#,
                route(&entry.id, en),
                esc(&entry.id),
                esc(&terms),
                library_thumb(&entry.id, grp),
                esc(entry.title.get(en)),
                esc(&entry.rust_type),
                esc(entry.lead.get(en)),
            );
        }
        let _ = write!(
            sections,
            r#"<section class="lib-section" data-library-section data-library-cat="{cat}"><div class="lib-section-head"><h2>{}</h2><span class="lib-section-count" data-library-section-count>{}</span></div><div class="lib-grid">{cards}</div></section>"#,
            esc(grp_label),
            items.len()
        );
    }

    format!(
        r#"<article class="doc library-doc" data-doc-id="library" data-library-root><p class="muted crumb">{}</p><h1>{}</h1><p class="muted lead">{}</p><div class="lib-toolbar"><label class="lib-search"><span>{}</span>{search_input}</label><div class="lib-pills" role="toolbar" aria-label="{}">{filter_pills}</div></div><section class="lib-featured" data-library-featured><div class="lib-section-head"><h2>{}</h2><span class="lib-section-count">6</span></div><div class="lib-grid lib-grid-featured">{featured_cards}</div></section>{sections}<p class="muted lib-empty" data-library-empty hidden>{}</p></article>"#,
        pick(en, "Catalog · MIT Open Source", "组件目录 · MIT 开源"),
        title("library", en),
        description("library", en),
        icon("search", 15),
        pick(en, "Filter categories", "分类筛选"),
        pick(en, "Featured showcases", "旗舰精选"),
        t("sidebar.empty", en),
    )
}

/// Render the article shared by full pages and component navigation.
pub fn article(section: &str, en: bool, revision: &str) -> String {
    match section {
        "library" => library_gallery(en),
        "docs" | "installation" | "ai" => guide(section, en),
        _ => component(
            ENTRIES
                .iter()
                .find(|entry| entry.id == section)
                .expect("registered component route"),
            en,
            revision,
        ),
    }
}

/// Render the sidebar and main document for a direct visit.
pub fn page(section: &str, en: bool, revision: &str) -> String {
    format!(
        r#"<div class="docs-frame">{}<main class="docs-main"><div class="docs-pad">{}</div></main></div>"#,
        sidebar(section, en),
        article(section, en, revision)
    )
}
