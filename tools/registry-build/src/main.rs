use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use kui_catalog::{items, materialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "kui-build")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Emit website/public/r/*.json from KUI registry source
    Build {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Confirm story depends on kui path (preview = source)
    VerifyPreview {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Write docs/items stubs
    Docs {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build { root } => build(&root)?,
        Commands::VerifyPreview { root } => verify_preview(&root)?,
        Commands::Docs { root } => write_docs(&root)?,
    }
    Ok(())
}

fn build(root: &Path) -> Result<()> {
    let registry = root.join("registry");
    let out = root.join("website/public/r");
    fs::create_dir_all(&out)?;

    let md = root.join("website/public/md");
    fs::create_dir_all(&md)?;
    let mut catalog = Vec::new();
    for (id, _, _) in items() {
        let item = materialize(&registry, id)?;
        fs::copy(
            root.join(format!("docs/items/{id}.md")),
            md.join(format!("{id}.md")),
        )?;
        let path = out.join(format!("{id}.json"));
        fs::write(&path, serde_json::to_string_pretty(&item)?)?;
        catalog.push(serde_json::json!({
            "name": item.name,
            "kind": item.kind,
            "title": item.title,
            "categories": item.categories,
        }));
        println!("wrote {}", path.display());
    }
    fs::write(
        out.join("catalog.json"),
        serde_json::to_string_pretty(&catalog)?,
    )?;

    let mut llms = String::from(
        "# KUI\n\n\
         > KUI is copy-source Rust UI for GPUI (desktop + WASM). From Kense.\n\n\
         Product: KUI. Crate/CLI: `kui`. License: MIT.\n\
         Theme: pure black `#000000` / white `#ffffff` by default. Brand ink `#171717` only for important moments.\n\n\
         ## For agents\n\n\
         - AI guide: https://kense.ai/kui/ai (local: /kui/ai)\n\
         - Install: `kui init` then `kui add button`\n\
         - Rules: repository AGENTS.md (Microsoft Pragmatic Rust Guidelines)\n\
         - Design: repository DESIGN.md\n\
         - Platforms: repository docs/PLATFORMS.md\n\n\
         ## Items\n\n",
    );
    for entry in &catalog {
        let name = entry["name"].as_str().unwrap_or("");
        llms.push_str(&format!(
            "- [{name}](/md/{name}.md): {}\n",
            entry["title"].as_str().unwrap_or(name)
        ));
    }
    llms.push_str("\n## Registry JSON\n\n- [Catalog](/r/catalog.json)\n");
    for (id, _, _) in items() {
        llms.push_str(&format!("- [{id}](/r/{id}.json)\n"));
    }
    fs::write(root.join("llms.txt"), &llms)?;
    let site_llms = root.join("website/public/llms.txt");
    if site_llms.parent().is_some_and(|p| p.exists()) {
        fs::write(site_llms, &llms)?;
    }
    println!("wrote llms.txt");
    Ok(())
}

fn verify_preview(root: &Path) -> Result<()> {
    let story = fs::read_to_string(root.join("apps/story/Cargo.toml"))?;
    if !story.contains("kui") {
        anyhow::bail!("kui-story must depend on kui (preview=source)");
    }
    for (id, _, _) in items() {
        let expected = serde_json::to_value(materialize(&root.join("registry"), id)?)?;
        let shipped: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(format!("website/public/r/{id}.json")))?)?;
        anyhow::ensure!(expected == shipped, "stale registry item: {id}");
    }
    let registry = root.join("registry/src");
    let mut hashes = BTreeMap::new();
    for entry in WalkDir::new(&registry).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let rel = path.strip_prefix(&registry)?.to_string_lossy().into_owned();
        let mut hasher = Sha256::new();
        hasher.update(fs::read(path)?);
        hashes.insert(rel, hex::encode(hasher.finalize()));
    }
    let manifest = root.join("website/public/r/_preview_hashes.json");
    fs::write(&manifest, serde_json::to_string_pretty(&hashes)?)?;
    println!("ok: KUI preview=source; {} files hashed", hashes.len());
    Ok(())
}

fn write_docs(root: &Path) -> Result<()> {
    let docs = root.join("docs/items");
    fs::create_dir_all(&docs)?;
    for (id, _, _) in items() {
        let path = docs.join(format!("{id}.md"));
        if path.exists() {
            continue;
        }
        let item = materialize(&root.join("registry"), id)?;
        fs::write(
            &path,
            format!(
                r#"# {title}

{description}

## When to use

Use `{name}` in a KUI / GPUI desktop app when this control fits the job.

## Install

```bash
kui add {name}
```

## Keyboard

Buttons activate with Enter and Space when focused.

## Accessibility

Expose a clear accessible name (the label). Keep disabled state in sync with interaction.

## Motion

Respect `Theme.reduced_motion`. Prefer `foundation::MOTION.snappy` for press feedback.

## Notes for AI

This is a **KUI** registry item. Prefer `kui add {name}` over rewriting the control.
"#,
                title = item.title,
                description = item.description,
                name = item.name,
            ),
        )?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
