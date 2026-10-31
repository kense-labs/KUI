//! Registry definitions and deterministic source materialization.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use walkdir::WalkDir;
#[derive(Serialize, Deserialize)]
pub struct RegistryItem {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub registry_dependencies: Vec<String>,
    #[serde(default)]
    pub cargo_dependencies: Vec<String>,
    pub gpui_version: String,
    #[serde(default)]
    pub files: Vec<RegistryFile>,
    #[serde(default)]
    pub docs_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RegistryFile {
    pub path: String,
    pub content: String,
}

pub const ITEMS: &[(&str, &str, &str)] = &[
    (
        "kui-foundation",
        "KUI foundation",
        "Theme tokens, motion presets, and init for KUI.",
    ),
    (
        "button",
        "Button",
        "Finished KUI button with variants, sizes, loading, and disabled states.",
    ),
    (
        "action-button",
        "ActionButton",
        "Save changes → Saving → Saved; the acknowledgement resets after 2.4 seconds.",
    ),
    (
        "split-button",
        "SplitButton",
        "Publish immediately or open the adjacent menu for three alternatives.",
    ),
    (
        "button-group",
        "ButtonGroup",
        "Related commands share a continuous surface. Try the edit, zoom and save/preview rows.",
    ),
    (
        "floating-button-group",
        "FloatingButtonGroup",
        "Spaced controls in a tray; examples include a vertical toolbar and a compact message toolbar.",
    ),
    (
        "expanding-button-group",
        "ExpandingButtonGroup",
        "Reveal a label in place. Mail, document and publishing actions have separate strips.",
    ),
    (
        "copy-button",
        "CopyButton",
        "Copy a link or an invite code using the system clipboard.",
    ),
    (
        "action-swap",
        "ActionSwap",
        "Follow a person, save an article, or send an invitation and recover from a simulated network failure.",
    ),
    (
        "icon-morph",
        "IconMorph",
        "Eight reversible icon pairs, including playback, alerts, details and likes.",
    ),
    (
        "confirm-morph",
        "ConfirmMorph",
        "Delete a sample project, cancel, retry or undo without opening a separate dialog.",
    ),
    (
        "theme-switch",
        "ThemeSwitch",
        "Change appearance from the trigger. Compare Reveal, Eclipse, Split and Rise.",
    ),
    (
        "hold-to-confirm",
        "HoldToConfirm",
        "A deliberate hold fills the button before it commits.",
    ),
    (
        "swipe-actions",
        "SwipeActions",
        "Triage the inbox by swiping a message or choosing the same command from its menu.",
    ),
    (
        "reactions",
        "Reactions",
        "Add a reaction to a release note, change your vote, or pick another symbol.",
    ),
    (
        "dropdown-menu",
        "DropdownMenu",
        "Keep a short list of project commands beside a labelled trigger.",
    ),
    (
        "context-menu",
        "ContextMenu",
        "Right-click a project to open its commands at the pointer, or tap the project to open below it.",
    ),
    (
        "user-menu",
        "UserMenu",
        "An account entry point with destinations, a theme preference and asynchronous sign-out.",
    ),
    (
        "input",
        "Input",
        "Edit a single line with a visible label, helper text and validation.",
    ),
    (
        "textarea",
        "Textarea",
        "Write a multiline project note, select text across lines and drag the bottom corner to resize.",
    ),
    (
        "password-field",
        "PasswordField",
        "Reveal or mask an access key without losing the editing position.",
    ),
    (
        "password-strength",
        "PasswordStrength",
        "Check a new password against a visible policy as you type.",
    ),
    (
        "search-field",
        "SearchField",
        "Type a search query and clear it without shifting the toolbar.",
    ),
    (
        "expanding-search",
        "ExpandingSearch",
        "Open a compact search button, browse recent items and choose from grouped results.",
    ),
    (
        "inline-edit",
        "InlineEdit",
        "Rename a project or edit its description in place, with validation and retry.",
    ),
];
pub fn materialize(registry: &Path, id: &str) -> Result<RegistryItem> {
    let (_, title, description) = ITEMS
        .iter()
        .copied()
        .find(|item| item.0 == id)
        .with_context(|| format!("unknown registry id: {id}"))?;
    let foundation = id == "kui-foundation";
    let kind = if foundation {
        "foundation"
    } else {
        "component"
    };
    let mut cargo = vec![];
    let paths = match id {
        "kui-foundation" => vec!["src/foundation".to_owned()],
        "button" => {
            cargo.push("web-time@1".into());
            [
                "src/components/button",
                "src/primitives/button.rs",
                "src/primitives/sizing.rs",
                "src/primitives/family.rs",
                "src/primitives/glyph.rs",
            ]
            .map(str::to_owned)
            .to_vec()
        }
        "action-button"
        | "split-button"
        | "button-group"
        | "floating-button-group"
        | "expanding-button-group"
        | "copy-button"
        | "action-swap"
        | "icon-morph"
        | "confirm-morph"
        | "theme-switch"
        | "hold-to-confirm"
        | "swipe-actions"
        | "reactions"
        | "dropdown-menu"
        | "context-menu"
        | "user-menu"
        | "input"
        | "textarea"
        | "password-field"
        | "password-strength"
        | "search-field"
        | "expanding-search"
        | "inline-edit" => {
            let mut files = vec![format!("src/components/{}.rs", id.replace('-', "_"))];
            let mut primitives = vec!["control", "family", "glyph"];
            cargo.push("web-time@1".into());
            match id {
                "input" | "textarea" | "password-field" | "password-strength" | "search-field"
                | "expanding-search" | "inline-edit" => {
                    primitives.push("editor");
                    cargo.push("unicode-segmentation@1".to_owned());
                    if !matches!(id, "inline-edit" | "expanding-search") {
                        primitives.push("field");
                    }
                }
                "copy-button" => {
                    primitives.push("clipboard");
                    cargo.extend(
                        ["wasm-bindgen@0.2", "js-sys@0.3", "wasm-bindgen-futures@0.4"]
                            .map(str::to_owned),
                    );
                }
                "split-button"
                | "button-group"
                | "floating-button-group"
                | "expanding-button-group" => primitives.push("strip"),
                "action-swap" | "action-button" => primitives.push("commit"),
                "hold-to-confirm" => primitives.push("direct_touch"),
                "swipe-actions" => primitives.extend(["direct_touch", "menu"]),
                "context-menu" | "dropdown-menu" | "user-menu" => {
                    primitives.extend(["direct_touch", "menu"])
                }
                _ => {}
            }
            files.extend(
                primitives
                    .into_iter()
                    .map(|p| format!("src/primitives/{p}.rs")),
            );
            files
        }
        _ => anyhow::bail!("missing source mapping for {id}"),
    };
    let mut files = Vec::new();
    for rel in paths {
        let abs = registry.join(&rel);
        if abs.is_file() {
            files.push(RegistryFile {
                path: rel.trim_start_matches("src/").to_string(),
                content: fs::read_to_string(&abs)?,
            });
        } else {
            for entry in WalkDir::new(&abs).sort_by_file_name() {
                let entry = entry?;
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
        categories: vec![
            match id {
                "kui-foundation" => "foundation",
                "input" | "textarea" | "password-field" | "password-strength" | "search-field"
                | "expanding-search" | "inline-edit" => "text-fields",
                "hold-to-confirm" | "swipe-actions" | "reactions" => "gestures",
                "dropdown-menu" | "context-menu" | "user-menu" => "menus",
                _ => "actions",
            }
            .into(),
        ],
        registry_dependencies: if foundation {
            vec![]
        } else if id == "password-strength" {
            vec!["kui-foundation".into(), "password-field".into()]
        } else if id == "swipe-actions" {
            vec!["kui-foundation".into(), "dropdown-menu".into()]
        } else {
            vec!["kui-foundation".into()]
        },
        cargo_dependencies: cargo,
        gpui_version: "=0.3.10".into(),
        files,
        docs_url: Some(format!("docs/items/{id}.md")),
    })
}
