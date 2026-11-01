//! The bilingual KUI landing page and its component tasks.
use crate::{controls::button, esc};

pub const CSS: &str = concat!(
    include_str!("../cover/cover.css"),
    include_str!("../cover/light.css"),
    include_str!("../cover/reel.css"),
    include_str!("../cover/specimen.css"),
    include_str!("../cover/guide.css"),
);

struct Task {
    id: &'static str,
    title: [&'static str; 2],
    behavior: [&'static str; 2],
}

const SAVE: Task = Task {
    id: "button",
    title: ["给项目加一个保存按钮", "Add a save button"],
    behavior: [
        "接入实际的保存操作。支持键盘触发，保存期间避免重复提交。",
        "Connect it to the save action. Keep keyboard activation working and prevent duplicate saves while a request is pending.",
    ],
};
const SELECTION: [(Task, [&str; 2], &str, &str, &str); 3] = [
    (
        Task {
            id: "icon-morph",
            title: ["让菜单图标变成关闭图标", "Animate the menu icon"],
            behavior: [
                "让图标跟随菜单的开关状态。点击、回车或空格都可切换。动效中途反切时，从当前位置接续。",
                "Keep the menu and close icons in sync with the menu state. Support clicks, Enter and Space, and reverse smoothly if toggled mid-animation.",
            ],
        },
        ["图标形变", "Icon morph"],
        "morph",
        "Motion",
        include_str!("../cover/morph.svg"),
    ),
    (
        Task {
            id: "date-picker",
            title: ["给表单加一个日期选择器", "Add a date picker"],
            behavior: [
                "把选中的日期写回表单，支持键盘选日、跨月和清空日期。",
                "Bind the selected date to the form. Support keyboard navigation, month changes and clearing the value.",
            ],
        },
        ["日期选择器", "Date picker"],
        "date",
        "Input",
        include_str!("../cover/date.svg"),
    ),
    (
        Task {
            id: "reasoning-block",
            title: ["给回复加一段过程摘要", "Add a progress summary"],
            behavior: [
                "在回复旁展示应用提供的进度说明。完成后自动展开。停止时保留已收到的文字，重试时重新开始。",
                "Show user-facing progress updates beside the response. Expand the summary on completion. Stopping should keep the text received so far. Retrying should start a fresh summary.",
            ],
        },
        ["过程摘要", "Reasoning block"],
        "reasoning",
        "AI",
        include_str!("../cover/reasoning.svg"),
    ),
];

fn label<'a>(en: bool, zh: &'a str, english: &'a str) -> &'a str {
    if en { english } else { zh }
}

impl Task {
    fn prompt(&self, en: bool) -> String {
        let i = usize::from(en);
        let id = self.id;
        let setup = if en {
            format!(
                "Check how this project uses Rust and GPUI. Reuse its existing KUI setup, or follow the KUI guide to set it up. Integrate the KUI component “{id}” into the project."
            )
        } else {
            format!(
                "请先检查当前项目的 Rust 和 GPUI 结构。如果已接入 KUI，就沿用现有配置，否则按 KUI 指南完成接入。请使用 KUI 的 {id} 组件实现。"
            )
        };
        let check = label(
            en,
            "沿用项目主题和无障碍约定，运行相关构建和交互检查，告诉我验证结果。",
            "Follow the project's theme and accessibility conventions. Run the relevant build and interaction checks, and report what you verified.",
        );
        let spec = label(en, "组件规格", "Component spec");
        format!(
            "{}\n{}\n{setup}\n{check}\n\n{spec}: https://kense-labs.github.io/md/{id}.md\nKUI: https://kense-labs.github.io/llms.txt",
            self.title[i], self.behavior[i]
        )
    }
}

fn icon(name: &str, size: u8) -> String {
    let paths = match name {
        "copy" => {
            r#"<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/>"#
        }
        "check" => r#"<path d="M5 12.5 9.5 17 19 7.5"/>"#,
        "terminal" => r#"<path d="M4 17 10 11 4 5M12 19h8"/>"#,
        "book" => {
            r#"<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>"#
        }
        "box" => {
            r#"<path d="m11 21-8-4.5v-9L11 3l8 4.5v9L11 21z"/><path d="M11 12 3 7.5M11 12l8-4.5M11 12v9"/>"#
        }
        "code" => r#"<path d="m8 8-4 4 4 4M16 8l4 4-4 4M14 5l-4 14"/>"#,
        _ => r#"<path d="M5 12h14M13 5l7 7-7 7"/>"#,
    };
    format!(
        r#"<span class="icon icon-{name}" aria-hidden="true"><svg width="{size}" height="{size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">{paths}</svg></span>"#
    )
}

fn copy_mark(name: &str) -> String {
    format!(
        r#"<span class="copy-mark">{}{}</span>"#,
        icon(name, 15),
        icon("check", 15)
    )
}

fn prompt_button(en: bool, task: &Task, class: &str, caption: &str, hint: bool) -> String {
    let title = task.title[usize::from(en)];
    let label = format!("{} · {title}", label(en, "复制提示词", "Copy prompt"));
    let hint = if hint {
        format!(r#" data-guide-hint="{}""#, esc(title))
    } else {
        String::new()
    };
    button(
        &format!(
            r#"class="{class}" data-kind="prompt" data-copy="{}" aria-label="{}"{hint}"#,
            esc(&task.prompt(en)),
            esc(&label)
        ),
        &format!("{caption}{}", copy_mark("copy")),
    )
}

fn command_button(en: bool, show_command: bool) -> String {
    let copy = label(en, "复制命令", "Copy command");
    let content = if show_command {
        format!("<code>kui add button</code>{}", copy_mark("copy"))
    } else {
        format!("{}{copy}", copy_mark("terminal"))
    };
    button(
        &format!(
            r#"class="agent-cta cli" data-copy="kui add button" data-kind="command" aria-label="{copy} · kui add button""#
        ),
        &content,
    )
}

fn reel(en: bool) -> String {
    const WEB: &str = r#"<circle cx="16" cy="16" r="10"/><ellipse cx="16" cy="16" rx="4.7" ry="10"/><path d="M6.4 13h19.2M6.4 19h19.2"/>"#;
    const DESKTOP: &str = r#"<rect x="4" y="5.5" width="24" height="17" rx="3"/><path d="M5 18.5h22m-13 4-1 4m5-4 1 4m-9 0h12"/><path d="m10 10 3 2.2-3 2.3m5.5 0h3.5" stroke-width="1.3"/>"#;
    let tickets: String = (-1..=2).map(|slot| {
        let (mode, glyph, face) = if slot % 2 == 0 { ("desktop", DESKTOP, label(en,"桌面应用","desktop")) } else { ("web", WEB, label(en,"Web 应用","the web")) };
        format!(r#"<span class="ticket {mode}" data-slot="{slot}"><span class="tile"><svg width="32" height="32" viewBox="0 0 32 32" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">{glyph}</svg></span><span>{face}</span></span>"#)
    }).collect();
    format!(
        r#"<span class="reel" aria-hidden="true"><span class="aperture">{tickets}</span><label for="cover-web" class="hit native-face"></label><label for="cover-desktop" class="hit web-face"></label></span>"#
    )
}

fn guide(en: bool, base: &str) -> String {
    let open = label(en, "打开 KUI 向导", "Open KUI guide");
    let close = label(en, "收起 KUI 向导", "Close KUI guide");
    let name = label(en, "KUI 向导", "KUI guide");
    let links: String = [
        (
            "components/button",
            "box",
            label(en, "浏览组件", "Explore components"),
        ),
        (
            "docs/installation",
            "book",
            label(en, "安装指南", "Installation guide"),
        ),
        (
            "ai",
            "code",
            label(en, "AI 接入指南", "Guide for AI agents"),
        ),
    ]
    .iter()
    .map(|(path, glyph, text)| {
        format!(
            r#"<a href="{base}/kui/{path}/">{}<span>{text}</span>{}</a>"#,
            icon(glyph, 16),
            icon("arrowRight", 14)
        )
    })
    .collect();
    let trigger = button(
        &format!(
            r#"class="trigger" aria-expanded="false" aria-controls="cover-guide-choices" aria-label="{open}" data-open-label="{open}" data-close-label="{close}""#
        ),
        &format!(
            r#"<span class="gaze" aria-hidden="true"><span class="eyes"><i></i><i></i></span></span>
        <span class="guide-name" aria-hidden="true">{name}</span>
        <svg class="cross" width="15" height="15" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="m4 4 8 8m0-8-8 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>"#
        ),
    );
    let speech_copy = button(
        r#"class="speech-copy" hidden data-kind="prompt""#,
        &format!(
            "{}{}",
            copy_mark("copy"),
            label(en, "复制提示词", "Copy prompt")
        ),
    );
    format!(
        include_str!("../cover/guide.html"),
        name = name,
        links = links,
        trigger = trigger,
        speech_copy = speech_copy
    )
}

/// Renders the complete cover inside the shared site shell.
pub fn page(en: bool) -> String {
    let i = usize::from(en);
    let base = if en { "/en" } else { "" };
    let count = kui_catalog::items()
        .filter(|(id, _, _)| *id != "kui-foundation")
        .count();
    let description = if en {
        format!(
            "{count} GPUI components for native apps and the browser. Install the Rust source you need."
        )
    } else {
        format!("{count} 个可单独安装的 GPUI 组件，桌面与浏览器共用一套 Rust 源码。")
    };
    let arrow = icon("arrowRight", 15);
    let cards: String = SELECTION.iter().enumerate().map(|(n,(task,name,kind,category,svg))| {
        let id=task.id;
        let title=esc(task.title[i]);
        let body=esc(task.behavior[i]);
        let copy=prompt_button(en,task,"sample-copy","",false);
        format!(r#"<article class="sample" data-guide-prompt="{id}" data-guide-hint="{title}" data-guide-body="{body}">
          <div class="print {kind}" aria-hidden="true">{svg}</div><div class="caption"><span class="serial">0{} / {category}</span>
          <a class="sample-name" href="{base}/kui/components/{id}/">{} {arrow}</a>{copy}</div></article>"#,n+1,name[i])
    }).collect();
    let pause = label(en, "暂停封面动效", "Pause cover motion");
    let play = label(en, "播放封面动效", "Play cover motion");
    let transport = button(
        &format!(
            r#"class="transport" aria-label="{pause}" data-pause="{pause}" data-play="{play}""#
        ),
        r#"<svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor" aria-hidden="true"><path class="pause-mark" d="M4 3h3v10H4zm5 0h3v10H9z"/><path class="play-mark" d="M4.5 2.8 13 8l-8.5 5.2z"/></svg>"#,
    );
    format!(
        include_str!("../cover/page.html"),
        base = base,
        description = description,
        arrow = arrow,
        cards = cards,
        guide = guide(en, base),
        transport = transport,
        title = label(
            en,
            "用 Rust 构建桌面和 Web 应用",
            "Build desktop and web apps with Rust"
        ),
        prefix = label(en, "用 Rust 构建", "Rust for"),
        reel = reel(en),
        try_one = label(en, "挑个组件试试", "Try a component"),
        explore = label(en, "浏览组件", "Explore components"),
        docs_hint = label(en, "从文档开始", "Start with the docs"),
        book = icon("book", 15),
        docs = label(en, "阅读文档", "Read the docs"),
        hero_prompt = prompt_button(
            en,
            &SAVE,
            "agent-cta",
            label(en, "复制 Agent 提示词", "Copy agent prompt"),
            true
        ),
        hero_command = command_button(en, true),
        platform_label = label(en, "封面演示平台", "Cover illustration platform"),
        desktop = label(en, "桌面", "Desktop"),
        selected = label(en, "组件选集", "Selected components"),
        all_hint = label(en, "查看组件库", "Open the library"),
        view_all = if en {
            format!("View all {count}")
        } else {
            format!("查看全部 {count} 项")
        },
        tip = label(
            en,
            "点卡片查看组件。在气泡中阅读具体任务，点复制按钮，把提示词带给你的 Agent。",
            "Open a card to explore the component. Read the task in its bubble and copy the prompt to your agent."
        ),
        start = label(en, "从一个保存按钮开始", "Start with a save button"),
        intro = label(
            en,
            "用你的编程 Agent 打开项目，粘贴提示词。里面已附上组件规格和接入指南。",
            "Open your project in your coding agent and paste the prompt. It includes the task and the KUI docs your agent needs."
        ),
        install = label(en, "安装指南", "Installation guide"),
        ai = label(en, "AI 接入指南", "Guide for AI agents"),
        task_label = label(en, "给 Agent 的任务", "PROMPT FOR YOUR AGENT"),
        task = SAVE.title[i],
        behavior = SAVE.behavior[i],
        start_prompt = prompt_button(
            en,
            &SAVE,
            "agent-cta",
            label(en, "复制提示词", "Copy prompt"),
            false
        ),
        start_command = command_button(en, false)
    )
}
