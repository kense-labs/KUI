//! A quickstart with local, synthetic key generation.
use gpui::{prelude::*, *};
use kui::{
    components::install_snippet::{
        InstallCommand, InstallFramework, InstallSnippet, InstallSnippetEvent, InstallStep,
    },
    primitives::control::UiLanguage,
};
use std::time::Duration;

pub const IDS: &[&str] = &["install-snippet"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}
struct Example {
    snippet: Entity<InstallSnippet>,
    _requests: Subscription,
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let frameworks = [
            (
                "desktop",
                t("Desktop", "桌面"),
                "cargo add kense-kui --rename kui",
                "src/main.rs",
                "use kui::components::button::Button;\n\nfn save_button() -> Button {\n    Button::new(\"save\")\n        .primary()\n        .label(\"Save changes\")\n}",
            ),
            (
                "web",
                t("Web", "网页"),
                "rustup target add wasm32-unknown-unknown",
                "src/lib.rs",
                "use gpui::App;\n\nfn configure_preview(cx: &mut App) {\n    kui::init(cx);\n}",
            ),
            (
                "library",
                t("Library", "组件库"),
                "cargo add kense-kui --rename kui --no-default-features",
                "src/lib.rs",
                "use kui::components::button::Button;\n\nfn cancel_button() -> Button {\n    Button::new(\"cancel\")\n        .secondary()\n        .label(\"Cancel\")\n}",
            ),
            (
                "cli",
                "CLI",
                "cargo install --path tools/cli",
                "Terminal",
                "kui init\nkui add button",
            ),
        ]
        .into_iter()
        .map(|(id, label, install, file, snippet)| {
            let commands = if id == "desktop" || id == "library" {
                vec![
                    InstallCommand::new("Cargo", install),
                    InstallCommand::new("KUI CLI", "kui add button"),
                ]
            } else {
                vec![InstallCommand::new("Terminal", install)]
            };
            InstallFramework::new(
                id,
                label,
                vec![
                    InstallStep::new("install", t("Install", "安装"), commands).description(t(
                        "Add the dependency to your project.",
                        "把依赖添加到你的项目。",
                    )),
                    InstallStep::new(
                        "key",
                        t("Add a sample key", "添加样例密钥"),
                        vec![InstallCommand::new("", "DEMO_API_KEY={{API_KEY}}")],
                    )
                    .filename(".env.example")
                    .description(t(
                        "The preview creates a sample key locally.",
                        "此预览在本地生成样例密钥。",
                    ))
                    .generate_key(true),
                    InstallStep::new(
                        "code",
                        t("Start building", "开始开发"),
                        vec![InstallCommand::new("", snippet)],
                    )
                    .filename(file),
                ],
            )
        })
        .collect();
        let snippet = cx.new(|cx| {
            InstallSnippet::new(frameworks, cx)
                .with_title(t("Get started in 30 seconds", "30 秒快速开始"))
                .with_description(t(
                    "Three steps to your first component.",
                    "三步接入你的第一个组件。",
                ))
                .with_version("v0.1.0")
                .with_key_placeholder("YOUR_SAMPLE_KEY")
        });
        let requests = cx.subscribe(&snippet, |_, view, event: &InstallSnippetEvent, cx| {
            let InstallSnippetEvent::KeyRequested { id, .. } = event else {
                return;
            };
            let id = *id;
            let view = view.downgrade();
            cx.spawn(async move |_, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(320))
                    .await;
                let _ = view.update(cx, |s, cx| {
                    s.finish_key(id, format!("sample_kui_{id:04}"), cx)
                });
            })
            .detach();
        });
        Self {
            snippet,
            _requests: requests,
        }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .p(px(20.))
            .flex()
            .justify_center()
            .child(div().w_full().max_w(px(512.)).child(self.snippet.clone()))
    }
}
