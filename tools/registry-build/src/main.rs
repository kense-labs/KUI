use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
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

#[derive(Serialize)]
struct RegistryItem {
    name: String,
    kind: String,
    title: String,
    description: String,
    categories: Vec<String>,
    registry_dependencies: Vec<String>,
    cargo_dependencies: Vec<String>,
    gpui_version: String,
    files: Vec<RegistryFile>,
    docs_url: String,
}

#[derive(Serialize)]
struct RegistryFile {
    path: String,
    content: String,
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

    let mut catalog = Vec::new();
    for id in ["kui-foundation", "button"] {
        let item = materialize(&registry, id)?;
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
        "# KUI\n\n> KUI is a copy-source Rust UI kit for GPUI. Button MVP.\n\n## Items\n\n",
    );
    for entry in &catalog {
        let name = entry["name"].as_str().unwrap_or("");
        llms.push_str(&format!(
            "- [{name}](docs/items/{name}.md): {}\n",
            entry["title"].as_str().unwrap_or(name)
        ));
    }
    fs::write(root.join("llms.txt"), llms)?;
    println!("wrote llms.txt");
    Ok(())
}

fn verify_preview(root: &Path) -> Result<()> {
    let story = fs::read_to_string(root.join("apps/story/Cargo.toml"))?;
    if !story.contains("kui") {
        anyhow::bail!("kui-story must depend on kui (preview=source)");
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
    for id in ["kui-foundation", "button"] {
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

fn materialize(registry: &Path, id: &str) -> Result<RegistryItem> {
    let (kind, title, description, categories, deps, rels) = match id {
        "kui-foundation" => (
            "foundation",
            "KUI foundation",
            "Theme tokens, motion presets, and init for KUI.",
            vec!["foundation".into()],
            Vec::<String>::new(),
            vec!["src/foundation".to_string()],
        ),
        "button" => (
            "component",
            "Button",
            "Finished KUI button with variants, sizes, loading, and disabled states.",
            vec!["actions".into()],
            vec!["kui-foundation".into()],
            vec![
                "src/components/button".into(),
                "src/primitives/button.rs".into(),
                "src/primitives/sizing.rs".into(),
                "src/primitives/mod.rs".into(),
            ],
        ),
        other => anyhow::bail!("unknown id {other}"),
    };

    let mut files = Vec::new();
    for rel in rels {
        let abs = registry.join(&rel);
        if abs.is_file() {
            files.push(RegistryFile {
                path: rel.trim_start_matches("src/").to_string(),
                content: fs::read_to_string(&abs)?,
            });
        } else {
            for entry in WalkDir::new(&abs).into_iter().filter_map(|e| e.ok()) {
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                    continue;
                }
                let rel_file = path
                    .strip_prefix(registry)
                    .with_context(|| format!("strip {}", path.display()))?
                    .to_string_lossy()
                    .trim_start_matches("src/")
                    .to_string();
                files.push(RegistryFile {
                    path: rel_file,
                    content: fs::read_to_string(path)?,
                });
            }
        }
    }

    Ok(RegistryItem {
        name: id.into(),
        kind: kind.into(),
        title: title.into(),
        description: description.into(),
        categories,
        registry_dependencies: deps,
        cargo_dependencies: Vec::new(),
        gpui_version: "=0.3.10".into(),
        files,
        docs_url: format!("docs/items/{id}.md"),
    })
}
