//! Generate the organization site and independently fetchable KUI document fragments.
mod controls;
mod cover;
mod docs;
mod shell;
#[path = "../../syntax.rs"]
mod syntax;
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use syntax::escape as esc;

fn put(path: PathBuf, data: impl AsRef<[u8]>) -> Result<()> {
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(&path, data).with_context(|| format!("write {}", path.display()))
}
fn tree(from: &Path, to: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(from) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = to.join(entry.path().strip_prefix(from)?);
            fs::create_dir_all(path.parent().unwrap())?;
            fs::copy(entry.path(), path)?;
        }
    }
    Ok(())
}
fn page(html: String, en: bool, stamp: &str, includes: &str) -> String {
    let tools = format!(
        r#"<div id="site-copy-status" role="status" aria-live="polite"></div><dialog id="site-manual-copy" aria-labelledby="copy-help"><h2 id="copy-help">{}</h2><p>{}</p><textarea readonly aria-labelledby="copy-help"></textarea>{}</dialog>"#,
        if en {
            "Copy selected text"
        } else {
            "复制选中的文字"
        },
        if en {
            "Clipboard access is unavailable. Copy the selected text below."
        } else {
            "浏览器无法写入剪贴板，请手动复制下方选中的文字。"
        },
        controls::button("data-close-copy", if en { "Done" } else { "完成" })
    );
    html.replacen("<!--site-assets-->", includes, 1)
        .replacen(
            "</head>",
            &format!(r#"<meta name="kui-pages" content="{stamp}"></head>"#),
            1,
        )
        .replacen("</body>", &(tools + "</body>"), 1)
}
fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let out: PathBuf = env::args_os()
        .nth(1)
        .context("usage: kui-site OUTPUT CLIENT_BINDINGS")?
        .into();
    let pkg: PathBuf = env::args_os()
        .nth(2)
        .context("missing wasm-bindgen output directory")?
        .into();
    let public = root.join("public");
    let rev = fs::read_to_string(public.join("stories/revision.txt"))
        .context("build the GPUI web preview first")?;
    let rev = rev.trim();
    if !public
        .join(format!("stories/v/{rev}/pkg/kui_web_bg.wasm"))
        .is_file()
    {
        bail!("missing GPUI preview")
    }
    tree(&public, &out)?;
    let assets = out.join("assets");
    let mut hash = Sha256::new();
    for file in ["site.js", "site_bg.wasm"] {
        hash.update(fs::read(pkg.join(file))?);
    }
    let client_hash = &hex::encode(hash.finalize())[..16];
    let client = assets.join(format!("client/{client_hash}"));
    tree(&pkg, &client)?;
    put(
        client.join("boot.js"),
        "import init from './site.js';\ninit().catch(error => console.error('KUI', error));\n",
    )?;
    let css = [
        include_str!("../assets/controls.css"),
        shell::CSS,
        docs::CSS,
        cover::CSS,
    ]
    .join("\n");
    let css_file = format!(
        "site-{}.css",
        &hex::encode(Sha256::digest(css.as_bytes()))[..16]
    );
    put(assets.join(&css_file), &css)?;
    let includes = format!(
        r#"<link rel="stylesheet" href="/assets/{css_file}"><link rel="modulepreload" href="/assets/client/{client_hash}/site.js"><script type="module" src="/assets/client/{client_hash}/boot.js"></script>"#
    );
    let ids = docs::ids();
    let expected: std::collections::BTreeSet<_> = kui_catalog::items()
        .filter_map(|(id, _, _)| (id != "kui-foundation").then_some(id.to_string()))
        .collect();
    if expected != ids.iter().cloned().collect() || expected.len() != ids.len() {
        bail!("document and registry ids do not match");
    }
    let mut fragments = vec![];
    let mut digest = Sha256::new();
    digest.update(rev);
    digest.update(client_hash);
    for en in [false, true] {
        for id in &ids {
            let html = docs::article(id, en, rev);
            let data = serde_json::to_vec(
                &serde_json::json!({"id":id,"lang":if en{"en"}else{"zh"},"title":docs::title(id,en),"description":docs::description(id,en),"html":html}),
            )?;
            digest.update(&data);
            fragments.push((en, id, data));
        }
    }
    let stamp = hex::encode(digest.finalize());
    let stamp = &stamp[..16];
    for en in [false, true] {
        let lang = if en { "en" } else { "zh" };
        let mut bundle = std::collections::BTreeMap::new();
        for (_, id, data) in fragments.iter().filter(|(locale, _, _)| *locale == en) {
            put(assets.join(format!("pages/{stamp}/{lang}/{id}.json")), data)?;
            bundle.insert(id.as_str(), std::str::from_utf8(data)?);
        }
        put(
            assets.join(format!("pages/{stamp}/{lang}.json")),
            serde_json::to_vec(&bundle)?,
        )?;
        let docs_includes = format!(
            r#"{includes}<link rel="preload" as="fetch" crossorigin="anonymous" fetchpriority="low" href="/assets/pages/{stamp}/{lang}.json">"#
        );
        let base = if en { out.join("en") } else { out.clone() };
        let write = |path: &str, body: String, title: &str, description: &str, company: bool| {
            let origin = if en {
                "https://kense-labs.github.io/en/"
            } else {
                "https://kense-labs.github.io/"
            };
            let canonical = if path.is_empty() {
                origin.to_string()
            } else {
                format!("{origin}{path}/")
            };
            put(
                base.join(path).join("index.html"),
                page(
                    shell::wrap(&body, title, description, &canonical, en, company),
                    en,
                    stamp,
                    if path.starts_with("kui/components/") {
                        &docs_includes
                    } else {
                        &includes
                    },
                ),
            )
        };
        write(
            "",
            shell::company(en),
            "Kense",
            if en {
                "Explore products from Kense."
            } else {
                "探索 Kense 的产品。"
            },
            true,
        )?;
        write(
            "kui",
            cover::page(en),
            "KUI",
            if en {
                "Build native and web interfaces with the KUI component library for Rust and GPUI."
            } else {
                "用 KUI 的 Rust 和 GPUI 组件构建桌面和 Web 界面。"
            },
            false,
        )?;
        for (route, section) in [
            ("kui/docs", "docs"),
            ("kui/docs/installation", "installation"),
            ("kui/ai", "ai"),
        ] {
            write(
                route,
                docs::page(section, en, rev),
                docs::title(section, en),
                docs::description(section, en),
                false,
            )?;
        }
        for id in &ids {
            write(
                &format!("kui/components/{id}"),
                docs::page(id, en, rev),
                docs::title(id, en),
                docs::description(id, en),
                false,
            )?;
        }
        let prefix = if en { "/en" } else { "" };
        for (from, to) in [
            ("docs", "kui/docs"),
            ("docs/installation", "kui/docs/installation"),
            ("components", "kui/components/button"),
            ("components/button", "kui/components/button"),
            ("ai", "kui/ai"),
        ] {
            let href = format!("{prefix}/{to}/");
            put(
                base.join(from).join("index.html"),
                format!(
                    r#"<!doctype html><meta charset=utf-8><meta http-equiv=refresh content="0;url={href}"><link rel=canonical href="{href}"><a href="{href}">KUI</a>"#
                ),
            )?;
        }
    }
    put(out.join(".nojekyll"), "")?;
    put(
        out.join("site-release.txt"),
        env::var("KUI_RELEASE").unwrap_or_else(|_| stamp.to_owned()),
    )?;
    println!(
        "Generated Kense, KUI and {} bilingual components in {}",
        ids.len(),
        out.display()
    );
    Ok(())
}
