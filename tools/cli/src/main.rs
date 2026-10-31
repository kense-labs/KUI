use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "kui", about = "KUI copy-source registry CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create ui/, foundation, kui.toml, and pin GPUI
    Init {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
    },
    /// Install registry items into ui/
    Add {
        ids: Vec<String>,
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Verify GPUI pin and foundation
    Doctor {
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
    /// Rebuild ui/mod.rs from the directory tree
    SyncMod {
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Deserialize, Serialize)]
struct KuiConfig {
    name: String,
    namespace: String,
    gpui_version: String,
    gpui_package: String,
    #[serde(default = "default_ui_dir")]
    ui_install_dir: String,
    #[serde(default)]
    registry_url: Option<String>,
    #[serde(default)]
    local_registry: Option<String>,
}

fn default_ui_dir() -> String {
    "ui".into()
}

use kui_catalog::{RegistryItem, materialize as materialize_from_source};
fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { path, registry } => cmd_init(&path, registry.as_deref())?,
        Commands::Add { ids, path, force } => cmd_add(&path, &ids, force)?,
        Commands::Doctor { path } => cmd_doctor(&path)?,
        Commands::SyncMod { path } => {
            let cfg = load_config(&path)?;
            sync_mod(&path.join(&cfg.ui_install_dir))?;
            println!("synced {}/mod.rs", cfg.ui_install_dir);
        }
    }
    Ok(())
}

fn cmd_init(path: &Path, registry: Option<&Path>) -> Result<()> {
    fs::create_dir_all(path)?;
    let cfg = KuiConfig {
        name: "app".into(),
        namespace: "@kui".into(),
        gpui_version: "=0.3.10".into(),
        gpui_package: "gpui-pre".into(),
        ui_install_dir: "ui".into(),
        registry_url: Some("https://kense.ai/r/{name}.json".into()),
        local_registry: registry.map(|p| p.display().to_string()).or_else(|| {
            let candidate = path.join("../../registry");
            if candidate.exists() {
                Some(candidate.display().to_string())
            } else {
                None
            }
        }),
    };
    let cfg_path = path.join("kui.toml");
    if !cfg_path.exists() {
        fs::write(
            &cfg_path,
            toml::to_string_pretty(&cfg).context("serialize kui.toml")?,
        )?;
        println!("wrote {}", cfg_path.display());
    }
    ensure_cargo_gpui_pin(path, &cfg)?;
    install_item(path, &cfg, "kui-foundation", true)?;
    sync_mod(&path.join(&cfg.ui_install_dir))?;
    println!("initialized KUI ui tree at {}", path.display());
    Ok(())
}

fn cmd_add(path: &Path, ids: &[String], force: bool) -> Result<()> {
    if ids.is_empty() {
        bail!("pass at least one item id");
    }
    let cfg = load_config(path)?;
    let mut planned = BTreeSet::new();
    for id in ids {
        collect_deps(&cfg, path, id, &mut planned)?;
    }
    for id in planned {
        let is_foundation = id == "kui-foundation";
        if is_foundation && !force && path.join(&cfg.ui_install_dir).join("foundation").exists() {
            // The original sync-mod generated this file without foundation's
            // public re-exports. Recognize only that exact generated file.
            let module = path.join(&cfg.ui_install_dir).join("foundation/mod.rs");
            if fs::read_to_string(&module).ok().as_deref()
                == Some("pub mod init;\npub mod motion;\npub mod theme;\npub mod tokens;\n")
            {
                let item = load_item(&cfg, path, &id)?;
                if let Some(file) = item.files.iter().find(|f| f.path == "foundation/mod.rs") {
                    fs::write(module, rewrite_paths(&file.content))?;
                }
            }
            println!("skip {id} (already installed)");
            continue;
        }
        install_item(path, &cfg, &id, force)?;
    }
    sync_mod(&path.join(&cfg.ui_install_dir))?;
    Ok(())
}

fn cmd_doctor(path: &Path) -> Result<()> {
    let cfg = load_config(path)?;
    let cargo = fs::read_to_string(path.join("Cargo.toml")).unwrap_or_default();
    if !cargo.contains(&cfg.gpui_version) && !cargo.contains("=0.3.10") {
        bail!(
            "Cargo.toml does not pin gpui to {}; run kui init",
            cfg.gpui_version
        );
    }
    let foundation = path.join(&cfg.ui_install_dir).join("foundation");
    if !foundation.exists() {
        bail!("missing {}/foundation — run kui init", cfg.ui_install_dir);
    }
    println!("ok: KUI gpui pin {} present", cfg.gpui_version);
    println!("ok: foundation at {}", foundation.display());
    Ok(())
}

fn load_config(path: &Path) -> Result<KuiConfig> {
    let raw = fs::read_to_string(path.join("kui.toml"))
        .with_context(|| format!("missing kui.toml in {}", path.display()))?;
    Ok(toml::from_str(&raw)?)
}

fn ensure_cargo_gpui_pin(path: &Path, cfg: &KuiConfig) -> Result<()> {
    let cargo_path = path.join("Cargo.toml");
    let cargo = if cargo_path.exists() {
        fs::read_to_string(&cargo_path)?
    } else {
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n"
            .into()
    };
    if !cargo.contains("gpui-pre") {
        let mut doc = cargo.parse::<toml_edit::DocumentMut>()?;
        let mut dep = toml_edit::InlineTable::new();
        dep.insert("package", cfg.gpui_package.as_str().into());
        dep.insert("version", cfg.gpui_version.as_str().into());
        dep.insert("default-features", false.into());
        doc["dependencies"]["gpui"] = toml_edit::value(dep);
        fs::write(&cargo_path, doc.to_string())?;
    }
    Ok(())
}

fn collect_deps(cfg: &KuiConfig, path: &Path, id: &str, out: &mut BTreeSet<String>) -> Result<()> {
    if out.contains(id) {
        return Ok(());
    }
    let item = load_item(cfg, path, id)?;
    for dep in &item.registry_dependencies {
        collect_deps(cfg, path, dep, out)?;
    }
    out.insert(id.to_string());
    Ok(())
}

fn load_item(cfg: &KuiConfig, app_path: &Path, id: &str) -> Result<RegistryItem> {
    for candidate in [
        app_path.join(format!("website/public/r/{id}.json")),
        app_path.join(format!("../../website/public/r/{id}.json")),
        PathBuf::from(format!("website/public/r/{id}.json")),
    ] {
        if candidate.exists() {
            let raw = fs::read_to_string(candidate)?;
            return Ok(serde_json::from_str(&raw)?);
        }
    }

    if let Some(local) = cfg.local_registry.as_deref() {
        let root = resolve_local_registry(app_path, local);
        if root.exists() {
            return materialize_from_source(&root, id);
        }
    }

    for root in [
        app_path.join("registry"),
        app_path.join("../registry"),
        app_path.join("../../registry"),
        PathBuf::from("registry"),
    ] {
        if root.exists()
            && let Ok(item) = materialize_from_source(&root, id)
        {
            return Ok(item);
        }
    }

    if let Some(url_tmpl) = &cfg.registry_url {
        let url = url_tmpl.replace("{name}", id);
        let resp = ureq::get(&url)
            .call()
            .with_context(|| format!("fetch {url}"))?;
        let item: RegistryItem = resp.into_json()?;
        return Ok(item);
    }

    bail!("could not resolve KUI registry item `{id}`");
}

fn resolve_local_registry(app_path: &Path, local: &str) -> PathBuf {
    let p = PathBuf::from(local);
    if p.is_absolute() { p } else { app_path.join(p) }
}

fn install_item(app_path: &Path, cfg: &KuiConfig, id: &str, force: bool) -> Result<()> {
    let item = load_item(cfg, app_path, id)?;
    let ui_root = app_path.join(&cfg.ui_install_dir);
    if !item.cargo_dependencies.is_empty() {
        let manifest = app_path.join("Cargo.toml");
        let mut doc = fs::read_to_string(&manifest)?.parse::<toml_edit::DocumentMut>()?;
        for spec in &item.cargo_dependencies {
            let (name, version) = spec.split_once('@').context("expected package@version")?;
            if doc.get("dependencies").and_then(|v| v.get(name)).is_none() {
                doc["dependencies"][name] = toml_edit::value(version);
            }
        }
        fs::write(manifest, doc.to_string())?;
    }
    for file in &item.files {
        let dest = ui_root.join(&file.path);
        if dest.exists() && !force {
            println!("exists {}, skipping (use --force)", dest.display());
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, rewrite_paths(&file.content))?;
        println!("wrote {}", dest.display());
    }
    Ok(())
}

fn rewrite_paths(content: &str) -> String {
    content
        .replace("use crate::{", "use crate::ui::{")
        .replace("crate::foundation", "crate::ui::foundation")
        .replace("crate::primitives", "crate::ui::primitives")
        .replace("crate::components", "crate::ui::components")
        .replace("kui::", "crate::ui::")
}

fn sync_mod(ui_root: &Path) -> Result<()> {
    fs::create_dir_all(ui_root)?;
    let mut tops = BTreeSet::new();
    for entry in WalkDir::new(ui_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path == ui_root {
            continue;
        }
        if let Ok(rel) = path.strip_prefix(ui_root)
            && let Some(first) = rel.components().next()
        {
            let name = first.as_os_str().to_string_lossy();
            if name != "mod.rs" {
                tops.insert(name.trim_end_matches(".rs").to_string());
            }
        }
    }

    let mut lib = String::from("//! Installed KUI source. Owned by this application.\n\n");
    for module in &tops {
        if module == "mod" {
            continue;
        }
        lib.push_str(&format!("pub mod {module};\n"));
    }
    fs::write(ui_root.join("mod.rs"), lib)?;

    for module in tops {
        let dir = ui_root.join(&module);
        if !dir.is_dir() {
            continue;
        }
        if module == "components" || module == "primitives" || !dir.join("mod.rs").exists() {
            write_dir_mod(&dir)?;
        }
    }
    Ok(())
}

fn write_dir_mod(dir: &Path) -> Result<()> {
    let mut names = BTreeSet::new();
    for e in fs::read_dir(dir)?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name == "mod.rs" {
            continue;
        }
        if e.path().is_dir() {
            names.insert(name);
        } else if e.path().extension().and_then(|s| s.to_str()) == Some("rs")
            && let Some(stem) = e.path().file_stem()
        {
            names.insert(stem.to_string_lossy().into_owned());
        }
    }

    let mut body = String::new();
    for name in names {
        body.push_str(&format!("pub mod {name};\n"));
        if dir.ends_with("primitives") {
            match name.as_str() {
                "button" => body.push_str("pub use button::ButtonBehavior;\n"),
                "sizing" => body.push_str("pub use sizing::Size;\n"),
                _ => {}
            }
        }
    }
    fs::write(dir.join("mod.rs"), body)?;
    Ok(())
}
