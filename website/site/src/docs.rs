//! Bilingual component reference and installation guides.

use crate::{controls, esc};
use serde::Deserialize;
use std::{collections::HashMap, fmt::Write, sync::LazyLock};

pub const CSS: &str = include_str!("../docs/docs.css");
const GROUPS: &[&str] = &[
    "buttons",
    "gestures",
    "menus",
    "text-fields",
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
    "workspace",
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
        "docs" | "ai" => format!("{prefix}/{section}/"),
        "installation" => format!("{prefix}/docs/installation/"),
        _ => format!("{prefix}/components/{section}/"),
    }
}

fn icon(name: &str, size: u8) -> String {
    let path = match name {
        "search" => r#"<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>"#,
        "book" => {
            r#"<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>"#
        }
        "box" => {
            r#"<path d="m11 21-8-4.5v-9L11 3l8 4.5v9L11 21zM11 12 3 7.5M11 12l8-4.5M11 12v9"/>"#
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

fn sidebar(current: &str, en: bool) -> String {
    let doc_label = t("sidebar.docs", en);
    let doc_rows = ["docs", "installation", "ai"]
        .map(|id| {
            nav_row(
                id,
                current,
                title(id, en),
                &format!("{} {}", title(id, true), title(id, false)),
                en,
            )
        })
        .join("");
    let guides = section(
        "docs",
        doc_label,
        None,
        "book",
        &format!(r#"<nav aria-label="{doc_label}" class="rows flat">{doc_rows}</nav>"#),
    );
    let mut groups = String::new();
    for group in GROUPS {
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
        let head = controls::button(
            r#"class="group-head" data-group-toggle aria-expanded="true""#,
            &format!(
                r#"<span class="group-chevron">{}</span><span class="group-label">{label}</span><span class="group-count" data-group-count>{count}</span>"#,
                icon("chevron", 12)
            ),
        );
        write!(groups,r#"<div class="group" data-group data-group-id="{group}" data-open="true">{head}<nav class="group-body" data-group-body aria-label="{label}"><div class="disclosure-clip"><div class="rows">{rows}</div></div></nav></div>"#).unwrap();
    }
    let components = section(
        "components",
        t("sidebar.components", en),
        Some(ENTRIES.len()),
        "box",
        &groups,
    );
    let placeholder = t("sidebar.search.placeholder", en);
    let search = controls::search(&format!(
        r#"data-sidebar-search placeholder="{placeholder}" aria-label="{placeholder}" autocomplete="off" spellcheck="false" enterkeyhint="go""#
    ));
    format!(
        r#"<aside class="aside" data-docs-sidebar aria-label="{}"><div class="panel"><div class="top"><label class="search"><span class="search-icon">{}</span>{search}<kbd data-search-kbd aria-hidden="true">/</kbd></label><span class="sr-only" data-search-status role="status"></span></div><div class="scroll" data-nav-scroll><div class="content" data-nav-root><div class="pill" data-nav-pill data-hidden="1" aria-hidden="true"></div>{guides}{components}<p class="empty" data-empty hidden>{}</p></div></div></div></aside>"#,
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

fn highlighted_line(mut line: &str, shell: bool) -> String {
    let mut result = String::new();
    while let Some(ch) = line.chars().next() {
        let (end, kind) = if (!shell && line.starts_with("//")) || (shell && ch == '#') {
            (line.len(), "comment")
        } else if ch == '"' {
            let mut escaped = false;
            let end = line[1..]
                .char_indices()
                .find_map(|(i, c)| {
                    let close = c == '"' && !escaped;
                    escaped = c == '\\' && !escaped;
                    close.then_some(i + 2)
                })
                .unwrap_or(line.len());
            (end, "string")
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            let end = line
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(line.len());
            let kind = match &line[..end] {
                "use" | "let" | "mut" | "fn" | "pub" | "move" | "if" | "else" | "match"
                | "impl" | "crate" | "self" => "keyword",
                "true" | "false" => "number",
                _ if ch.is_ascii_digit() => "number",
                _ if ch.is_ascii_uppercase() || line[end..].starts_with('(') => "tag",
                _ => "",
            };
            (end, kind)
        } else {
            (ch.len_utf8(), "")
        };
        let token = esc(&line[..end]);
        if kind.is_empty() {
            result.push_str(&token);
        } else {
            write!(result, r#"<span class="syn-{kind}">{token}</span>"#).unwrap();
        }
        line = &line[end..];
    }
    result
}

fn code_panel(title: &str, source: &str, en: bool) -> String {
    let shell = title == "shell";
    let lines = source
        .lines()
        .map(|line| {
            format!(
                r#"<span class="line">{}</span>"#,
                highlighted_line(line, shell)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
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
    let theme_class = if id == "theme-switch" {
        " theme-preview"
    } else {
        ""
    };
    let url = preview_url(id, en, revision);
    let mut html = format!(
        r#"<section class="stage" data-component-stage><div class="view-bar"><div class="tabs" role="tablist" aria-label="{}">{tabs}<span class="indicator" data-view-indicator aria-hidden="true"></span></div><p class="hint muted">{}</p></div><div class="panel is-active" role="tabpanel" id="stage-panel-preview" data-panel="preview" aria-labelledby="stage-tab-preview"><div data-preview-slot data-preview-src="{url}"><iframe allow="clipboard-write" class="rust-story family-story{theme_class}" data-story="{id}" data-story-key="{id}" data-lang="{locale}" aria-label="{title}" src="{url}"></iframe></div></div><div class="panel" role="tabpanel" id="stage-panel-code" data-panel="code" aria-labelledby="stage-tab-code" hidden>{example}{}</div></section>"#,
        t("stage.views", en),
        t("stage.hint", en),
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
    let crumb = format!(
        "{} · {}",
        t("sidebar.components", en),
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

/// Render the article shared by full pages and component navigation.
pub fn article(section: &str, en: bool, revision: &str) -> String {
    match section {
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
