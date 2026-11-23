//! Project folder detail view with breadcrumbs, document previews, uploads, and scoped AI workspace.
//!
//! Displays the contents of an open workspace folder (`Q3 Campaign` by default) with sticky
//! breadcrumbs, document/PDF/spreadsheet preview cards, share and upload actions, inline
//! renaming, and an on-demand right-hand file preview and folder-scoped assistant panel.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_eid,
            pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_person_avatar,
            pro_pill_indicator, pro_presence, pro_selection_sensor, pro_skeleton_bar,
            pro_spring_bar, pro_switch, pro_toast_banner, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const FOLDER_MARK_SVG: &str = r##"<path d="M11.17 15.35v15.36h44.67v-6.98c0-3.86-3.13-6.98-6.98-6.98H33.5l-3.93-5.89a5.58 5.58 0 0 0-4.64-2.49h-6.78c-3.86 0-6.98 3.13-6.98 6.98Z" fill="#00A1FF"></path><path d="M33.5 16.75l-3.93-5.89a5.58 5.58 0 0 0-4.64-2.49h-6.78c-3.86 0-6.98 3.13-6.98 6.98v15.36h44.67v-6.98c0-3.86-3.13-6.98-6.98-6.98H33.5Zm0 0h-9.77" stroke="#00A1FF" stroke-width="4.19" stroke-linecap="round" stroke-linejoin="round"></path><path d="M6.59 42.38c-1.05-5.26-1.57-7.89-.08-9.67l.31-.34c1.66-1.66 4.59-1.66 10.45-1.66h32.46c5.86 0 8.79 0 10.45 1.66l.31.34c1.49 1.78.97 4.41-.08 9.67-1.51 7.56-2.26 11.34-5.1 13.66l-.6.46c-3.01 2.13-7.23 2.13-15.66 2.13h-11.1c-8.43 0-12.64 0-15.66-2.13l-.6-.46c-2.84-2.32-3.6-6.1-5.1-13.66Z" fill="#3CC7FF" stroke="#3CC7FF" stroke-width="4.19"></path>"##;

const BACK_ARROW_SVG: &str = r##"<path d="M5.5 12.002H19" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M10.9999 18.002C10.9999 18.002 4.99998 13.583 4.99997 12.0019C4.99996 10.4208 11 6.00195 11 6.00195" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"##;

/// Document format of a file stored inside a [`FolderScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FolderFileKind {
    #[default]
    Document,
    Pdf,
    Spreadsheet,
    Image,
    Code,
}

impl FolderFileKind {
    /// Short uppercase tag displayed on the file's miniature document badge.
    pub fn badge(self) -> &'static str {
        match self {
            Self::Document => "DOC",
            Self::Pdf => "PDF",
            Self::Spreadsheet => "XLS",
            Self::Image => "IMG",
            Self::Code => "CODE",
        }
    }

    /// Human-readable file format label.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Document => tr(cx, "Document", "文档"),
            Self::Pdf => "PDF",
            Self::Spreadsheet => tr(cx, "Spreadsheet", "电子表格"),
            Self::Image => tr(cx, "Image", "图片"),
            Self::Code => tr(cx, "Code", "代码"),
        }
    }

    fn tag_color(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Document => p.blue,
            Self::Pdf => rgb(0xf04438).into(),
            Self::Spreadsheet => rgb(0x13b3a0).into(),
            Self::Image => p.amber,
            Self::Code => p.purple,
        }
    }
}

/// One file card displayed inside [`FolderScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderFileItem {
    pub id: SharedString,
    pub name: SharedString,
    pub kind: FolderFileKind,
    pub modified: SharedString,
    pub size_kb: u32,
    pub owner: SharedString,
    pub excerpt: SharedString,
}

impl FolderFileItem {
    /// Construct a folder file item with its format and relative modified label.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        kind: FolderFileKind,
        modified: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            modified: modified.into(),
            size_kb: 420,
            owner: "Alex Morgan".into(),
            excerpt: "Project reference document synced with folder context.".into(),
        }
    }

    /// Set file size in kilobytes and owner display name.
    pub fn with_details(mut self, size_kb: u32, owner: impl Into<SharedString>) -> Self {
        self.size_kb = size_kb;
        self.owner = owner.into();
        self
    }

    /// Set the preview excerpt shown in the right-hand inspector.
    pub fn with_excerpt(mut self, excerpt: impl Into<SharedString>) -> Self {
        self.excerpt = excerpt.into();
        self
    }
}

/// External connector attached to the project folder context (`Notion`, `Google Drive`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderConnectedSource {
    pub app: SharedString,
    pub scope: SharedString,
    pub connected: bool,
}

/// Scoped conversation thread saved under the open project folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FolderChatThread {
    pub id: SharedString,
    pub title: SharedString,
    pub updated: SharedString,
    pub messages: Vec<(bool, SharedString)>,
}

/// Events emitted by [`FolderScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FolderEvent {
    Navigate(SharedString),
    BackToFiles,
    OpenFile(SharedString),
    UploadFile(SharedString),
    ShareFolder(SharedString),
    StartChat(SharedString),
    UpdateInstructions(SharedString),
    RenameFolder(SharedString),
    RenameFile {
        id: SharedString,
        name: SharedString,
    },
    DeleteFile(SharedString),
}

impl EventEmitter<FolderEvent> for FolderScreen {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FolderPanelMode {
    FilePreview(SharedString),
    Workspace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RenameTarget {
    Folder,
    File(SharedString),
}

fn preset_files_for_folder(folder_name: &str) -> Vec<FolderFileItem> {
    match folder_name.trim() {
        "Brand assets" => vec![
            FolderFileItem::new("logo-system", "Logo-System", FolderFileKind::Pdf, "Yesterday")
                .with_details(3_240, "Jonas Weber")
                .with_excerpt(
                    "Primary wordmark geometry, monochrome lockups, and minimum clearspace ratios.",
                ),
            FolderFileItem::new(
                "color-tokens",
                "Color-Tokens",
                FolderFileKind::Document,
                "3 days ago",
            )
            .with_details(860, "Jonas Weber")
            .with_excerpt(
                "Neutral surface roles, dark-mode contrast pairs, and semantic status tokens.",
            ),
            FolderFileItem::new(
                "type-specimens",
                "Type-Specimens",
                FolderFileKind::Pdf,
                "Last week",
            )
            .with_details(1_920, "Alex Morgan")
            .with_excerpt(
                "Display and body scale pairings with tabular figure rules for financial tables.",
            ),
        ],
        "Hiring" => vec![
            FolderFileItem::new(
                "role-scorecard",
                "Role-Scorecard",
                FolderFileKind::Document,
                "4 hours ago",
            )
            .with_details(390, "Alex Morgan")
            .with_excerpt(
                "Evaluation rubric for Senior Product Engineer across systems design and craft.",
            ),
            FolderFileItem::new(
                "headcount-plan",
                "Headcount-Plan",
                FolderFileKind::Spreadsheet,
                "Yesterday",
            )
            .with_details(540, "Alex Morgan")
            .with_excerpt("Approved Q3–Q4 engineering and design hiring plan by team."),
            FolderFileItem::new(
                "interview-loop",
                "Interview-Loop",
                FolderFileKind::Document,
                "3 days ago",
            )
            .with_details(310, "Alex Morgan")
            .with_excerpt("Structured 4-stage panel schedule with calibration prompts."),
            FolderFileItem::new(
                "offer-bands",
                "Offer-Bands",
                FolderFileKind::Spreadsheet,
                "Last week",
            )
            .with_details(400, "Jonas Weber")
            .with_excerpt("Compensation benchmarks and equity band ranges by level."),
        ],
        "Research" => vec![
            FolderFileItem::new(
                "user-interviews",
                "User-Interviews",
                FolderFileKind::Document,
                "Yesterday",
            )
            .with_details(610, "Alex Morgan")
            .with_excerpt(
                "Synthesis of 14 customer sessions on multi-folder context and template reuse.",
            ),
            FolderFileItem::new(
                "survey-synthesis",
                "Survey-Synthesis",
                FolderFileKind::Spreadsheet,
                "5 days ago",
            )
            .with_details(350, "Alex Morgan")
            .with_excerpt("Cohort retention and feature adoption scores across 320 teams."),
        ],
        _ => vec![
            FolderFileItem::new(
                "campaign-brief",
                "Campaign-Brief",
                FolderFileKind::Document,
                "2 hours ago",
            )
            .with_details(680, "Alex Morgan")
            .with_excerpt(
                "Positioning narrative, target enterprise personas, launch milestones, and channel KPIs for Q3.",
            ),
            FolderFileItem::new("launch-plan", "Launch-Plan", FolderFileKind::Pdf, "Yesterday")
                .with_details(1_840, "Jonas Weber")
                .with_excerpt(
                    "Day-by-day rollout sequence across product blog, partner announcements, and customer webinars.",
                ),
            FolderFileItem::new(
                "media-budget",
                "Media-Budget",
                FolderFileKind::Spreadsheet,
                "Yesterday",
            )
            .with_details(520, "Jonas Weber")
            .with_excerpt(
                "Committed Q3 paid, partner co-marketing, and developer benchmark spend ($53,150 of $54,500).",
            ),
            FolderFileItem::new("ad-copy", "Ad-Copy", FolderFileKind::Document, "3 days ago")
                .with_details(290, "Alex Morgan")
                .with_excerpt(
                    "Approved headline variants, social proof callouts, and landing page CTAs.",
                ),
            FolderFileItem::new(
                "results-q2",
                "Results-Q2",
                FolderFileKind::Spreadsheet,
                "Last week",
            )
            .with_details(740, "Alex Morgan")
            .with_excerpt(
                "Baseline Q2 conversion funnel, CAC payback by channel, and attribution lift.",
            ),
        ],
    }
}

fn default_sources() -> Vec<FolderConnectedSource> {
    vec![
        FolderConnectedSource {
            app: "Notion".into(),
            scope: "Launch wiki".into(),
            connected: true,
        },
        FolderConnectedSource {
            app: "Google Drive".into(),
            scope: "Creative assets".into(),
            connected: true,
        },
    ]
}

fn default_threads(folder_name: &str) -> Vec<FolderChatThread> {
    vec![
        FolderChatThread {
            id: "thread-1".into(),
            title: format!("{folder_name} budget vs brief").into(),
            updated: "18m ago".into(),
            messages: vec![
                (
                    true,
                    "Summarize any budget variance against our launch milestones.".into(),
                ),
                (
                    false,
                    "Across Media-Budget and Campaign-Brief, $53,150 of the $54,500 cap is committed (-$1,350 under budget), with Thursday 09:00 UTC locked for launch.".into(),
                ),
            ],
        },
        FolderChatThread {
            id: "thread-2".into(),
            title: "Headline variants check".into(),
            updated: "Yesterday".into(),
            messages: vec![
                (
                    true,
                    "Do the Ad-Copy headlines match the tone in Launch-Plan?".into(),
                ),
                (
                    false,
                    "Yes — all three primary headlines emphasize fast setup and shared workspace governance without unverified claims.".into(),
                ),
            ],
        },
    ]
}

fn format_size_kb(kb: u32) -> String {
    if kb >= 1024 {
        format!("{:.1} MB", kb as f32 / 1024.)
    } else {
        format!("{kb} KB")
    }
}

fn append_prompt_to_thread(
    threads: &mut [FolderChatThread],
    active_idx: usize,
    folder_name: &str,
    file_count: usize,
    prompt: &str,
) -> Option<SharedString> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return None;
    }
    let thread = threads.get_mut(active_idx)?;
    let user_msg: SharedString = trimmed.to_owned().into();
    let reply: SharedString = format!(
        "Checked {file_count} files in {folder_name}: aligned \"{trimmed}\" with Campaign-Brief, Launch-Plan, and Media-Budget."
    )
    .into();
    thread.messages.push((true, user_msg.clone()));
    thread.messages.push((false, reply));
    thread.updated = "Just now".into();
    Some(user_msg)
}

/// Detail screen for an open project folder (`Folders / Detail`).
pub struct FolderScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    folder: SharedString,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    shared: bool,
    loading: bool,
    empty: bool,
    files: Vec<FolderFileItem>,
    instructions: SharedString,
    editing_instructions: bool,
    instructions_editor: Option<Entity<Editor>>,
    prompt_editor: Option<Entity<Editor>>,
    sources: Vec<FolderConnectedSource>,
    threads: Vec<FolderChatThread>,
    active_thread: usize,
    selected_file: Option<SharedString>,
    panel_mode: Option<FolderPanelMode>,
    more_menu_open: bool,
    share_menu_open: bool,
    renaming: Option<RenameTarget>,
    rename_editor: Option<Entity<Editor>>,
    upload_seq: usize,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
    boot_instant: Instant,
    toast: Option<(SharedString, Instant)>,
}

impl Focusable for FolderScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl FolderScreen {
    /// Construct a new [`FolderScreen`] opened to `"Q3 Campaign"` inside [`AppShell`].
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let initial_folder: SharedString = "Q3 Campaign".into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Chat, shell_cx)
                .with_active(initial_folder.clone())
                .with_work_area(false, false)
                .with_panel_width(364.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });
        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                let lower = label.to_ascii_lowercase();
                if lower == "folders" || label.as_ref() == "文件夹" {
                    cx.emit(FolderEvent::BackToFiles);
                    cx.emit(FolderEvent::Navigate(label.clone()));
                } else if matches!(
                    label.as_ref(),
                    "Q3 Campaign" | "Brand assets" | "Hiring" | "Research"
                ) {
                    this.set_folder(label.clone(), cx);
                    cx.emit(FolderEvent::Navigate(label.clone()));
                } else {
                    cx.emit(FolderEvent::Navigate(label.clone()));
                }
            }
            AppShellEvent::PanelOpen(false) => {
                this.panel_mode = None;
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            standalone: false,
            folder: initial_folder.clone(),
            workspace: "Acme".into(),
            user_name: "Alex Morgan".into(),
            user_email: "alex@acme.ai".into(),
            shared: true,
            loading: false,
            empty: false,
            files: preset_files_for_folder(initial_folder.as_ref()),
            instructions: "Ground every answer in Q3 Campaign brief milestones, approved ad copy, and the $54,500 channel budget cap.".into(),
            editing_instructions: false,
            instructions_editor: None,
            prompt_editor: None,
            sources: default_sources(),
            threads: default_threads(initial_folder.as_ref()),
            active_thread: 0,
            selected_file: None,
            panel_mode: None,
            more_menu_open: false,
            share_menu_open: false,
            renaming: None,
            rename_editor: None,
            upload_seq: 0,
            focus: cx.focus_handle(),
            measured_width: Rc::new(Cell::new(920.)),
            boot_instant: Instant::now(),
            toast: None,
        }
    }

    /// Render only the inner folder page without the outer [`AppShell`] chrome.
    pub fn standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Render only the inner folder page without the outer [`AppShell`] chrome.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Return the open folder name.
    pub fn folder(&self) -> &str {
        self.folder.as_ref()
    }

    /// Return the workspace name shown in the sidebar.
    pub fn workspace(&self) -> &str {
        self.workspace.as_ref()
    }

    /// Return whether skeleton placeholders are active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Return whether the zero-data empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Set the open folder name, updating breadcrumbs, title, active sidebar item, and default files.
    pub fn with_folder(mut self, folder: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        let name = folder.into();
        self.files = preset_files_for_folder(name.as_ref());
        self.threads = default_threads(name.as_ref());
        self.folder = name.clone();
        self.shell.update(cx, |s, _| s.set_active(name));
        self
    }

    /// Switch the open folder dynamically.
    pub fn set_folder(&mut self, folder: impl Into<SharedString>, cx: &mut Context<Self>) {
        let name = folder.into();
        self.folder = name.clone();
        self.files = preset_files_for_folder(name.as_ref());
        self.threads = default_threads(name.as_ref());
        self.active_thread = 0;
        self.selected_file = None;
        if matches!(self.panel_mode, Some(FolderPanelMode::FilePreview(_))) {
            self.panel_mode = None;
            self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
        }
        self.shell.update(cx, |s, cx| {
            s.set_active(name);
            cx.notify();
        });
        cx.notify();
    }

    /// Override the workspace name displayed in the [`AppShell`] sidebar.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws = workspace.into();
        self.workspace = ws.clone();
        let active = self.folder.clone();
        let uname = self.user_name.clone();
        let uemail = self.user_email.clone();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx))
                .with_preset(AppShellPreset::Chat, cx)
                .with_workspace(ws)
                .with_user(uname, uemail)
                .with_active(active)
                .with_work_area(false, false)
                .with_panel_width(364.);
        });
        self.rebind_shell(cx);
        self
    }

    /// Override the signed-in user displayed in the sidebar footer.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.user_name = name.into();
        self.user_email = email.into();
        let ws = self.workspace.clone();
        let active = self.folder.clone();
        let uname = self.user_name.clone();
        let uemail = self.user_email.clone();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx))
                .with_preset(AppShellPreset::Chat, cx)
                .with_workspace(ws)
                .with_user(uname, uemail)
                .with_active(active)
                .with_work_area(false, false)
                .with_panel_width(364.);
        });
        self.rebind_shell(cx);
        self
    }

    fn rebind_shell(&self, cx: &mut Context<Self>) {
        let this = cx.entity();
        self.shell.update(cx, |s, shell_cx| {
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
        });
    }

    /// Replace the files shown inside the open folder.
    pub fn with_files(mut self, files: Vec<FolderFileItem>) -> Self {
        self.files = files;
        self
    }

    /// Override the custom instructions for the folder assistant.
    pub fn with_instructions(mut self, instructions: impl Into<SharedString>) -> Self {
        self.instructions = instructions.into();
        self
    }

    /// Enable or disable skeleton file placeholders while keeping header and breadcrumbs intact.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the empty folder state (`"No files yet"`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Update the loading state dynamically.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Update the empty state dynamically.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Access the underlying [`AppShell`] entity.
    pub fn shell(&self) -> &Entity<AppShell> {
        &self.shell
    }

    fn show_toast(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.notify();
    }

    fn ensure_workspace_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.prompt_editor.is_none() {
            let ph: SharedString = format!(
                "{} {}…",
                tr(cx, "Ask anything in", "在此文件夹提问："),
                self.folder
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Submitted) {
                    this.submit_prompt_from_value(cx);
                } else if matches!(ev, EditEvent::Changed) {
                    cx.notify();
                }
            })
            .detach();
            self.prompt_editor = Some(ed);
        }
        if self.instructions_editor.is_none() {
            let initial = self.instructions.to_string();
            let ph: SharedString = tr(
                cx,
                "Custom instructions for this folder…",
                "此文件夹的专属指令…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e.set_value(initial, w, cx);
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed) {
                    cx.notify();
                }
            })
            .detach();
            self.instructions_editor = Some(ed);
        }
    }

    fn submit_prompt_with_window(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.prompt_editor.clone() else {
            return;
        };
        let raw = ed.read(cx).value().to_string();
        if let Some(sent) = append_prompt_to_thread(
            &mut self.threads,
            self.active_thread,
            self.folder.as_ref(),
            if self.empty { 0 } else { self.files.len() },
            &raw,
        ) {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
            cx.emit(FolderEvent::StartChat(sent));
            cx.notify();
        }
    }

    fn submit_prompt_from_value(&mut self, cx: &mut Context<Self>) {
        let Some(ed) = self.prompt_editor.clone() else {
            return;
        };
        let raw = ed.read(cx).value().to_string();
        if let Some(sent) = append_prompt_to_thread(
            &mut self.threads,
            self.active_thread,
            self.folder.as_ref(),
            if self.empty { 0 } else { self.files.len() },
            &raw,
        ) {
            self.prompt_editor = None;
            cx.emit(FolderEvent::StartChat(sent));
            cx.notify();
        }
    }

    fn open_workspace_chat(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.ensure_workspace_editors(w, cx);
        self.panel_mode = Some(FolderPanelMode::Workspace);
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        if let Some(ed) = &self.prompt_editor {
            {
                let fh = ed.read(cx).focus.clone();
                fh.focus(w, cx);
            }
        }
        cx.emit(FolderEvent::StartChat(self.folder.clone()));
        cx.notify();
    }

    fn upload_file(&mut self, cx: &mut Context<Self>) {
        if self.empty {
            self.files.clear();
            self.empty = false;
        }
        self.upload_seq += 1;
        let seq = self.upload_seq;
        let candidates = [
            ("Creative-Assets", FolderFileKind::Pdf, 1_420),
            ("Partner-Tracker", FolderFileKind::Spreadsheet, 480),
            ("Messaging-FAQ", FolderFileKind::Document, 310),
            ("UTM-Matrix", FolderFileKind::Spreadsheet, 260),
        ];
        let (base, kind, size_kb) = candidates[(seq - 1) % candidates.len()];
        let name: SharedString = if seq <= candidates.len() {
            base.into()
        } else {
            format!("{base}-{seq}").into()
        };
        let id: SharedString = format!("upload-{seq}").into();
        let item = FolderFileItem::new(id.clone(), name.clone(), kind, tr(cx, "Just now", "刚刚"))
            .with_details(size_kb, self.user_name.clone())
            .with_excerpt("Newly uploaded file indexed for project folder threads.");
        self.files.insert(0, item);
        self.selected_file = Some(id);
        cx.emit(FolderEvent::UploadFile(name.clone()));
        let toast_msg = format!("{} \"{}\"", tr(cx, "Uploaded", "已上传"), name);
        self.show_toast(toast_msg, cx);
    }

    fn start_rename(
        &mut self,
        target: RenameTarget,
        current_name: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let initial = current_name.to_string();
        let fs = match target {
            RenameTarget::Folder => 18.,
            RenameTarget::File(_) => 13.,
        };
        let ed = cx.new(|cx| {
            let mut e = Editor::new("Rename", InputMode::Single, w, cx);
            e.font_size = fs;
            e.set_value(initial, w, cx);
            e.select_all(w, cx);
            e
        });
        {
            let fh = ed.read(cx).focus.clone();
            fh.focus(w, cx);
        }
        cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
            EditEvent::Submitted => this.commit_rename(cx),
            EditEvent::Escape => {
                this.renaming = None;
                this.rename_editor = None;
                cx.notify();
            }
            _ => {}
        })
        .detach();
        self.renaming = Some(target);
        self.rename_editor = Some(ed);
        cx.notify();
    }

    fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.renaming.take() else {
            return;
        };
        let next = self
            .rename_editor
            .take()
            .map(|ed| ed.read(cx).value().trim().to_string())
            .unwrap_or_default();
        if next.is_empty() {
            cx.notify();
            return;
        }
        let shared: SharedString = next.into();
        match target {
            RenameTarget::Folder => {
                self.folder = shared.clone();
                self.shell.update(cx, |s, cx| {
                    s.set_active(shared.clone());
                    cx.notify();
                });
                cx.emit(FolderEvent::RenameFolder(shared.clone()));
                self.show_toast(
                    format!(
                        "{} \"{}\"",
                        tr(cx, "Renamed folder to", "已重命名文件夹为"),
                        shared
                    ),
                    cx,
                );
            }
            RenameTarget::File(fid) => {
                if let Some(f) = self.files.iter_mut().find(|f| f.id == fid) {
                    f.name = shared.clone();
                    cx.emit(FolderEvent::RenameFile {
                        id: fid,
                        name: shared.clone(),
                    });
                    self.show_toast(
                        format!("{} \"{}\"", tr(cx, "Renamed to", "已重命名为"), shared),
                        cx,
                    );
                } else {
                    cx.notify();
                }
            }
        }
    }

    fn delete_file(&mut self, fid: &SharedString, cx: &mut Context<Self>) {
        self.files.retain(|f| &f.id != fid);
        if self.selected_file.as_ref() == Some(fid) {
            self.selected_file = None;
        }
        if self.panel_mode == Some(FolderPanelMode::FilePreview(fid.clone())) {
            self.panel_mode = None;
            self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
        }
        cx.emit(FolderEvent::DeleteFile(fid.clone()));
        self.show_toast(tr(cx, "File removed from folder", "已从文件夹移除文件"), cx);
    }

    fn any_editor_focused(&self, w: &Window, cx: &App) -> bool {
        let check = |opt: &Option<Entity<Editor>>| {
            opt.as_ref()
                .is_some_and(|ed| ed.read(cx).focus.is_focused(w))
        };
        check(&self.rename_editor) || check(&self.prompt_editor) || check(&self.instructions_editor)
    }

    fn step_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.empty || self.files.is_empty() {
            return;
        }
        let next_idx = if let Some(cur) = &self.selected_file
            && let Some(pos) = self.files.iter().position(|f| &f.id == cur)
        {
            (pos as isize + delta).rem_euclid(self.files.len() as isize) as usize
        } else if delta >= 0 {
            0
        } else {
            self.files.len() - 1
        };
        let fid = self.files[next_idx].id.clone();
        self.selected_file = Some(fid.clone());
        if matches!(self.panel_mode, Some(FolderPanelMode::FilePreview(_))) {
            self.panel_mode = Some(FolderPanelMode::FilePreview(fid));
        }
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.key == "escape" {
            if self.renaming.is_some() {
                self.renaming = None;
                self.rename_editor = None;
                cx.notify();
                return;
            }
            if self.more_menu_open || self.share_menu_open {
                self.more_menu_open = false;
                self.share_menu_open = false;
                cx.notify();
                return;
            }
            if self.panel_mode.is_some() {
                self.panel_mode = None;
                self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
                cx.notify();
                return;
            }
        }
        if self.any_editor_focused(w, cx) || ev.keystroke.modifiers.platform {
            return;
        }
        match ev.keystroke.key.as_str() {
            "j" | "J" | "down" | "right" => self.step_selection(1, cx),
            "k" | "K" | "up" | "left" => self.step_selection(-1, cx),
            "enter" => {
                if let Some(fid) = self.selected_file.clone()
                    && let Some(f) = self.files.iter().find(|f| f.id == fid)
                {
                    let fname = f.name.clone();
                    self.panel_mode = Some(FolderPanelMode::FilePreview(fid));
                    self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                    cx.emit(FolderEvent::OpenFile(fname));
                    cx.notify();
                }
            }
            " " | "space" => {
                if let Some(fid) = self.selected_file.clone() {
                    let target = FolderPanelMode::FilePreview(fid);
                    if self.panel_mode.as_ref() == Some(&target) {
                        self.panel_mode = None;
                        self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
                    } else {
                        self.panel_mode = Some(target);
                        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                    }
                    cx.notify();
                }
            }
            "backspace" => {
                cx.emit(FolderEvent::BackToFiles);
                cx.emit(FolderEvent::Navigate("Folders".into()));
            }
            "c" | "C" => self.open_workspace_chat(w, cx),
            "u" | "U" => self.upload_file(cx),
            "r" | "R" => {
                if let Some(fid) = self.selected_file.clone()
                    && let Some(name) = self
                        .files
                        .iter()
                        .find(|f| f.id == fid)
                        .map(|f| f.name.clone())
                {
                    self.start_rename(RenameTarget::File(fid), name.as_ref(), w, cx);
                } else {
                    let fname = self.folder.clone();
                    self.start_rename(RenameTarget::Folder, fname.as_ref(), w, cx);
                }
            }
            _ => {}
        }
    }

    fn render_doc_artwork(&self, kind: FolderFileKind, p: &ProPalette) -> Div {
        let tag_bg = kind.tag_color(p);
        let mut page = div()
            .w(px(52.))
            .pt(px(10.))
            .px(px(10.))
            .pb(px(22.))
            .rounded(px(6.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(8.));

        match kind {
            FolderFileKind::Pdf | FolderFileKind::Image => {
                page = page
                    .child(
                        div()
                            .w_full()
                            .h(px(14.))
                            .rounded(px(2.))
                            .bg(p.ink.opacity(0.07)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .child(
                                div()
                                    .w_full()
                                    .h(px(2.))
                                    .rounded(px(1.))
                                    .bg(p.ink.opacity(0.11)),
                            )
                            .child(
                                div()
                                    .w(px(26.))
                                    .h(px(2.))
                                    .rounded(px(1.))
                                    .bg(p.ink.opacity(0.11)),
                            )
                            .child(
                                div()
                                    .w(px(20.))
                                    .h(px(2.))
                                    .rounded(px(1.))
                                    .bg(p.ink.opacity(0.11)),
                            ),
                    );
            }
            FolderFileKind::Spreadsheet => {
                let mut rows = div().flex().flex_col().gap(px(5.));
                for r in 0..4usize {
                    let cell_bg = if r == 0 {
                        p.ink.opacity(0.22)
                    } else {
                        p.ink.opacity(0.10)
                    };
                    rows = rows.child(
                        div()
                            .flex()
                            .gap(px(3.))
                            .child(div().flex_1().h(px(2.)).rounded(px(1.)).bg(cell_bg))
                            .child(div().flex_1().h(px(2.)).rounded(px(1.)).bg(cell_bg))
                            .child(div().flex_1().h(px(2.)).rounded(px(1.)).bg(cell_bg)),
                    );
                }
                page = page.child(rows);
            }
            FolderFileKind::Document | FolderFileKind::Code => {
                page = page.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(5.))
                        .child(
                            div()
                                .w(px(20.))
                                .h(px(2.5))
                                .rounded(px(1.))
                                .bg(p.ink.opacity(0.18)),
                        )
                        .child(
                            div()
                                .w_full()
                                .h(px(2.))
                                .rounded(px(1.))
                                .bg(p.ink.opacity(0.11)),
                        )
                        .child(
                            div()
                                .w_full()
                                .h(px(2.))
                                .rounded(px(1.))
                                .bg(p.ink.opacity(0.11)),
                        )
                        .child(
                            div()
                                .w(px(26.))
                                .h(px(2.))
                                .rounded(px(1.))
                                .bg(p.ink.opacity(0.11)),
                        )
                        .child(
                            div()
                                .w(px(18.))
                                .h(px(2.))
                                .rounded(px(1.))
                                .bg(p.ink.opacity(0.11)),
                        ),
                );
            }
        }

        div().relative().w(px(52.)).ml(px(8.)).child(page).child(
            div()
                .absolute()
                .left(px(-8.))
                .bottom(px(10.))
                .px(px(4.5))
                .py(px(2.5))
                .rounded(px(3.))
                .bg(tag_bg)
                .text_size(px(9.))
                .line_height(px(10.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(0xffffff))
                .child(kind.badge()),
        )
    }

    fn render_file_card(
        &self,
        file: &FolderFileItem,
        preview_h: f32,
        p: &ProPalette,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let fid = file.id.clone();
        let fname = file.name.clone();
        let is_selected = self.selected_file.as_ref() == Some(&fid)
            || self.panel_mode == Some(FolderPanelMode::FilePreview(fid.clone()));
        let is_renaming = self.renaming == Some(RenameTarget::File(fid.clone()));
        let rename_ed = self.rename_editor.clone();

        let frame_bg = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.02)
        };
        let border_c = if is_selected { p.focus } else { p.line };
        let hover_border = if is_selected { p.focus } else { p.line_strong };

        let name_el = if is_renaming && let Some(ed) = rename_ed {
            div()
                .mt(px(8.))
                .h(px(22.))
                .px(px(4.))
                .rounded(px(4.))
                .bg(p.panel)
                .border_1()
                .border_color(p.focus)
                .flex()
                .items_center()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(div().w_full().min_w_0().child(ed))
                .into_any_element()
        } else {
            div()
                .mt(px(8.))
                .text_size(px(13.))
                .line_height(px(20.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .truncate()
                .child(file.name.clone())
                .into_any_element()
        };

        let meta_txt = format!("{} · {}", file.kind.label(cx), file.modified);

        activate(
            div()
                .id(pro_eid(&self.id, format!("file-card-{fid}")))
                .role(Role::Button)
                .aria_label(file.name.clone())
                .tab_index(0)
                .flex_1()
                .min_w_0()
                .max_w(px(239.))
                .flex()
                .flex_col()
                .cursor_pointer()
                .child(
                    div()
                        .h(px(preview_h))
                        .p(px(3.))
                        .rounded(px(12.))
                        .bg(frame_bg)
                        .border_1()
                        .border_color(border_c)
                        .hover(move |s| s.border_color(hover_border))
                        .child(
                            div()
                                .size_full()
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_soft)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(self.render_doc_artwork(file.kind, p)),
                        ),
                )
                .child(name_el)
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .truncate()
                        .child(meta_txt),
                ),
            cx,
            move |this, _, cx| {
                this.selected_file = Some(fid.clone());
                this.panel_mode = Some(FolderPanelMode::FilePreview(fid.clone()));
                this.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                cx.emit(FolderEvent::OpenFile(fname.clone()));
                cx.notify();
            },
        )
    }

    fn render_inspector(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let mode = self.panel_mode.clone()?;
        let p = ProPalette::get(cx);
        let is_ws = matches!(mode, FolderPanelMode::Workspace);

        let fallback_fid = self
            .selected_file
            .clone()
            .or_else(|| self.files.first().map(|f| f.id.clone()));

        let mode_sensor = pro_selection_sensor();
        let mode_bar = div()
            .px(px(12.))
            .pt(px(4.))
            .pb(px(8.))
            .border_b_1()
            .border_color(p.line_soft)
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "insp-mode-pill"),
                        if is_ws { "ws" } else { "preview" },
                        mode_sensor.clone(),
                        6.,
                        p.select,
                        Some(p.line_strong),
                        false,
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "insp-tab-preview"))
                            .role(Role::Tab)
                            .tab_index(0)
                            .relative()
                            .h(px(26.))
                            .px(px(9.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(if !is_ws { transparent_black() } else { p.line })
                            .when(!is_ws, |d| d.child(pro_mark_selected(&mode_sensor)))
                            .when(is_ws, |d| {
                                d.hover({
                                    let hv = p.hover;
                                    move |s| s.bg(hv)
                                })
                            })
                            .text_size(px(12.))
                            .font_weight(if !is_ws {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if !is_ws { p.ink } else { p.muted })
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .cursor_pointer()
                            .child(pro_icon(
                                ProIcon::Eye,
                                12.,
                                if !is_ws { p.ink } else { p.muted },
                            ))
                            .child(tr(cx, "Preview", "文件预览")),
                        cx,
                        move |this, _, cx| {
                            if let Some(fid) = fallback_fid.clone() {
                                this.panel_mode = Some(FolderPanelMode::FilePreview(fid));
                                cx.notify();
                            }
                        },
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "insp-tab-ws"))
                            .role(Role::Tab)
                            .tab_index(0)
                            .relative()
                            .h(px(26.))
                            .px(px(9.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(if is_ws { transparent_black() } else { p.line })
                            .when(is_ws, |d| d.child(pro_mark_selected(&mode_sensor)))
                            .when(!is_ws, |d| {
                                d.hover({
                                    let hv = p.hover;
                                    move |s| s.bg(hv)
                                })
                            })
                            .text_size(px(12.))
                            .font_weight(if is_ws {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if is_ws { p.ink } else { p.muted })
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .cursor_pointer()
                            .child(pro_icon(
                                ProIcon::Sparkles,
                                12.,
                                if is_ws { p.ink } else { p.muted },
                            ))
                            .child(tr(cx, "Folder assistant", "文件夹助手")),
                        cx,
                        |this, w, cx| this.open_workspace_chat(w, cx),
                    )),
            );

        match mode {
            FolderPanelMode::FilePreview(fid) => {
                let file = self.files.iter().find(|f| f.id == fid)?.clone();
                let fid_rn = file.id.clone();
                let fname_rn = file.name.clone();
                let fid_del = file.id.clone();

                let artifact = if file.kind == FolderFileKind::Spreadsheet {
                    div()
                        .rounded(px(9.))
                        .border_1()
                        .border_color(p.line)
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .h(px(26.))
                                .px(px(8.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(div().flex_1().child("Channel"))
                                .child(div().w(px(64.)).child("Planned"))
                                .child(div().w(px(64.)).child("Actual")),
                        )
                        .children(
                            [
                                ("Launch video", "$18,000", "$17,400"),
                                ("Partner co-mktg", "$24,500", "$24,500"),
                                ("Benchmarks", "$12,000", "$11,250"),
                                ("Total", "$54,500", "$53,150"),
                            ]
                            .into_iter()
                            .map(|(ch, pl, ac)| {
                                div()
                                    .h(px(28.))
                                    .px(px(8.))
                                    .border_t_1()
                                    .border_color(p.line_soft)
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.))
                                    .child(div().flex_1().text_color(p.ink).truncate().child(ch))
                                    .child(div().w(px(64.)).text_color(p.text).child(pl))
                                    .child(div().w(px(64.)).text_color(p.text).child(ac))
                            }),
                        )
                        .into_any_element()
                } else {
                    div()
                        .p(px(12.))
                        .rounded(px(9.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .line_height(px(19.))
                                .text_color(p.text)
                                .child(file.excerpt.clone()),
                        )
                        .child(
                            div()
                                .pt(px(6.))
                                .border_t_1()
                                .border_color(p.line_soft)
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .children(
                                    [
                                        (
                                            tr(
                                                cx,
                                                "Approved messaging & positioning",
                                                "已审定核心传播口径",
                                            ),
                                            true,
                                        ),
                                        (tr(cx, "Partner quote sign-off", "合作方引言确认"), true),
                                        (
                                            tr(
                                                cx,
                                                "Final Thursday 09:00 UTC publish",
                                                "周四 09:00 UTC 正式发布",
                                            ),
                                            false,
                                        ),
                                    ]
                                    .into_iter()
                                    .map(|(label, done)| {
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .text_size(px(12.))
                                            .text_color(if done { p.muted } else { p.ink })
                                            .child(pro_icon(
                                                if done { ProIcon::Check } else { ProIcon::Clock },
                                                12.,
                                                if done { p.green } else { p.faint },
                                            ))
                                            .child(label)
                                    }),
                                ),
                        )
                        .into_any_element()
                };

                let total_f_kb = self.files.iter().map(|f| f.size_kb).sum::<u32>().max(1);
                let file_share = (file.size_kb as f32 / total_f_kb as f32).clamp(0.05, 1.0);
                let preview_body = div()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .h(px(132.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(self.render_doc_artwork(file.kind, &p)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(file.name.clone()),
                            )
                            .child(div().text_size(px(12.)).text_color(p.muted).child(format!(
                                "{} · {} · {}",
                                file.kind.label(cx),
                                format_size_kb(file.size_kb),
                                file.modified
                            ))),
                    )
                    .child(artifact)
                    .child(
                        div()
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .p(px(10.))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(12.))
                                    .child(div().text_color(p.muted).child(tr(
                                        cx,
                                        "Owner",
                                        "所有者",
                                    )))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .text_color(p.ink)
                                            .child(pro_person_avatar(
                                                file.owner.as_ref(),
                                                18.,
                                                false,
                                                None,
                                                cx,
                                            ))
                                            .child(file.owner.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(12.))
                                    .child(div().text_color(p.muted).child(tr(
                                        cx,
                                        "Folder",
                                        "所属文件夹",
                                    )))
                                    .child(div().text_color(p.ink).child(self.folder.clone())),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(5.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .text_size(px(12.))
                                            .child(div().text_color(p.muted).child(tr(
                                                cx,
                                                "Folder size share",
                                                "文件夹容量占比",
                                            )))
                                            .child(
                                                div()
                                                    .text_color(p.ink)
                                                    .child(format_size_kb(file.size_kb)),
                                            ),
                                    )
                                    .child(pro_spring_bar(
                                        pro_eid(&self.id, format!("fshare-{}", file.id)),
                                        file_share,
                                        4.,
                                        2.,
                                        p.chip,
                                        file.kind.tag_color(&p),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_btn(
                                pro_eid(&self.id, "ql-work-in-folder"),
                                tr(cx, "Work in folder", "在文件夹中协作"),
                                Some(ProIcon::NewChat),
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                |this, w, cx| this.open_workspace_chat(w, cx),
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, format!("ql-rn-file-{fid_rn}")),
                                tr(cx, "Rename", "重命名"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                move |this, w, cx| {
                                    this.start_rename(
                                        RenameTarget::File(fid_rn.clone()),
                                        fname_rn.as_ref(),
                                        w,
                                        cx,
                                    );
                                },
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, format!("ql-del-file-{fid_del}")),
                                tr(cx, "Delete", "删除"),
                                Some(ProIcon::Trash),
                                ProBtnStyle::Danger,
                                true,
                                cx,
                                move |this, _, cx| this.delete_file(&fid_del, cx),
                            )),
                    );

                let content = div()
                    .flex()
                    .flex_col()
                    .child(mode_bar)
                    .child(pro_view_transition(
                        pro_eid(&self.id, "folder-insp-vtx"),
                        format!("preview-{}", file.id),
                        preview_body,
                    ));

                Some((file.name, ProIcon::PageDoc, content.into_any_element()))
            }
            FolderPanelMode::Workspace => {
                self.ensure_workspace_editors(w, cx);
                let instr_ed = self.instructions_editor.clone();
                let prompt_ed = self.prompt_editor.clone();
                let file_count = if self.empty { 0 } else { self.files.len() };

                let instr_card = div()
                    .p(px(10.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(pro_icon(ProIcon::Note, 13., p.muted))
                                    .child(tr(cx, "Project instructions", "项目专属指令")),
                            )
                            .child(pro_btn(
                                pro_eid(&self.id, "toggle-instr-edit"),
                                if self.editing_instructions {
                                    tr(cx, "Save", "保存")
                                } else {
                                    tr(cx, "Edit", "编辑")
                                },
                                None,
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                |this, w, cx| {
                                    if this.editing_instructions {
                                        if let Some(ed) = &this.instructions_editor {
                                            let next: SharedString =
                                                ed.read(cx).value().trim().to_string().into();
                                            if !next.is_empty() {
                                                this.instructions = next.clone();
                                                cx.emit(FolderEvent::UpdateInstructions(next));
                                            }
                                        }
                                        this.editing_instructions = false;
                                    } else {
                                        this.ensure_workspace_editors(w, cx);
                                        if let Some(ed) = &this.instructions_editor {
                                            {
                                                let fh = ed.read(cx).focus.clone();
                                                fh.focus(w, cx);
                                            }
                                        }
                                        this.editing_instructions = true;
                                    }
                                    cx.notify();
                                },
                            )),
                    )
                    .when(!self.editing_instructions, |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(p.muted)
                                .child(self.instructions.clone()),
                        )
                    })
                    .child({
                        let ed_box = div()
                            .h(px(68.))
                            .p(px(8.))
                            .rounded(px(7.))
                            .bg(p.panel)
                            .border_1()
                            .border_color(p.focus)
                            .when_some(instr_ed, |d, ed| d.child(div().size_full().child(ed)));
                        pro_fold(
                            pro_eid(&self.id, "instr-edit-fold"),
                            self.editing_instructions,
                            72.,
                            6.,
                            move || ed_box.into_any_element(),
                        )
                    });

                // Connected knowledge sources
                let mut sources_row = div().flex().flex_wrap().items_center().gap(px(6.));
                sources_row = sources_row.child(
                    div()
                        .h(px(26.))
                        .px(px(8.))
                        .rounded(px(7.))
                        .bg(p.chip)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(11.5))
                        .text_color(p.ink)
                        .child(pro_icon(ProIcon::Folder, 12., p.blue))
                        .child(format!(
                            "{file_count} {}",
                            tr(cx, "folder files", "个文件夹文档")
                        )),
                );
                for (si, src) in self.sources.iter().enumerate() {
                    let on = src.connected;
                    sources_row = sources_row.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("src-chip-{si}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(26.))
                            .pl(px(5.))
                            .pr(px(8.))
                            .rounded(px(7.))
                            .bg(if on { p.card } else { p.chip })
                            .border_1()
                            .border_color(if on { p.line_strong } else { p.line })
                            .opacity(if on { 1.0 } else { 0.55 })
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(11.5))
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(pro_app_tile(src.app.as_ref(), 18., cx))
                            .child(src.app.clone()),
                        cx,
                        move |this, _, cx| {
                            if let Some(s) = this.sources.get_mut(si) {
                                s.connected = !s.connected;
                                cx.notify();
                            }
                        },
                    ));
                }

                // Threads switcher
                let th_sensor = pro_selection_sensor();
                let mut thread_tabs = div()
                    .relative()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(5.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "fthread-pill"),
                        format!("th-{}", self.active_thread),
                        th_sensor.clone(),
                        6.,
                        p.select,
                        Some(p.line_strong),
                        false,
                    ));
                for (ti, th) in self.threads.iter().enumerate() {
                    let active = self.active_thread == ti;
                    thread_tabs = thread_tabs.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("fthread-{ti}")))
                            .role(Role::Tab)
                            .tab_index(0)
                            .relative()
                            .h(px(26.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .bg(if active { transparent_black() } else { p.card })
                            .border_1()
                            .border_color(if active { transparent_black() } else { p.line })
                            .when(active, |d| d.child(pro_mark_selected(&th_sensor)))
                            .when(!active, |d| {
                                d.hover({
                                    let hv = p.hover;
                                    move |s| s.bg(hv)
                                })
                            })
                            .text_size(px(11.5))
                            .font_weight(if active {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if active { p.ink } else { p.muted })
                            .max_w(px(150.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .child(div().truncate().child(th.title.clone())),
                        cx,
                        move |this, _, cx| {
                            this.active_thread = ti;
                            cx.notify();
                        },
                    ));
                }
                thread_tabs = thread_tabs.child(pro_icon_btn(
                    pro_eid(&self.id, "new-fthread"),
                    tr(cx, "New folder thread", "新建文件夹对话"),
                    ProIcon::Plus,
                    26.,
                    false,
                    cx,
                    |this, _, cx| {
                        let idx = this.threads.len() + 1;
                        this.threads.push(FolderChatThread {
                            id: format!("thread-{idx}").into(),
                            title: format!("{} #{idx}", this.folder).into(),
                            updated: tr(cx, "Just now", "刚刚").into(),
                            messages: vec![(
                                false,
                                format!(
                                    "Ready to work with {} files in {}.",
                                    if this.empty { 0 } else { this.files.len() },
                                    this.folder
                                )
                                .into(),
                            )],
                        });
                        this.active_thread = this.threads.len() - 1;
                        cx.notify();
                    },
                ));

                let messages = self
                    .threads
                    .get(self.active_thread)
                    .map(|t| t.messages.clone())
                    .unwrap_or_default();

                let mut msg_col = div().flex().flex_col().gap(px(8.));
                for (is_user, text) in messages {
                    let avatar = if is_user {
                        pro_person_avatar(self.user_name.as_ref(), 22., false, None, cx)
                    } else {
                        pro_agent_avatar("folder-assistant", p.blue, 22., false, 0., cx)
                    };
                    msg_col = msg_col.child(
                        div()
                            .p(px(9.))
                            .rounded(px(9.))
                            .bg(if is_user { p.chip } else { p.card })
                            .border_1()
                            .border_color(p.line_soft)
                            .flex()
                            .items_start()
                            .gap(px(8.))
                            .child(avatar)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(12.5))
                                    .line_height(px(18.5))
                                    .text_color(p.ink)
                                    .child(text),
                            ),
                    );
                }

                let composer = div()
                    .p(px(8.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .when_some(prompt_ed, |d, ed| {
                        d.child(div().flex_1().min_w_0().h(px(24.)).child(ed))
                    })
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "send-folder-prompt"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Send prompt", "发送提问"))
                            .tab_index(0)
                            .size(px(26.))
                            .rounded_full()
                            .bg(p.primary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child(pro_icon(ProIcon::ArrowUp, 13., p.on_primary)),
                        cx,
                        |this, w, cx| this.submit_prompt_with_window(w, cx),
                    ));

                let msg_vtx = pro_view_transition(
                    pro_eid(&self.id, "thread-msg-vtx"),
                    format!("th-{}", self.active_thread),
                    msg_col,
                );

                let ws_inner = div()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(instr_card)
                    .child(sources_row)
                    .child(thread_tabs)
                    .child(msg_vtx)
                    .child(composer);

                let ws_body = div()
                    .flex()
                    .flex_col()
                    .child(mode_bar)
                    .child(pro_view_transition(
                        pro_eid(&self.id, "folder-insp-vtx"),
                        format!("ws-{}", self.folder),
                        ws_inner,
                    ));

                Some((
                    self.folder.clone(),
                    ProIcon::ProjectFolder,
                    ws_body.into_any_element(),
                ))
            }
        }
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;

        if let Some((_, started)) = &self.toast
            && started.elapsed().as_secs_f32() >= 2.45
        {
            self.toast = None;
        }
        let phase = if self.loading && !reduced {
            w.request_animation_frame();
            (self.boot_instant.elapsed().as_secs_f32() * 0.75).fract()
        } else {
            0.25
        };

        let main_w = self.measured_width.get();
        let measure_cell = self.measured_width.clone();

        // Top sticky breadcrumb bar (.ui-as-crumbs)
        let crumbs = div()
            .h(px(48.))
            .flex_none()
            .pl(px(if main_w < 680. { 50. } else { 16. }))
            .pr(px(16.))
            .bg(p.panel)
            .flex()
            .items_center()
            .gap(px(8.))
            .text_size(px(13.))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "crumb-files"))
                    .role(Role::Button)
                    .tab_index(0)
                    .text_color(p.muted)
                    .hover({
                        let ink = p.ink;
                        move |s| s.text_color(ink)
                    })
                    .cursor_pointer()
                    .child(tr(cx, "Files", "文件")),
                cx,
                |_, _, cx| {
                    cx.emit(FolderEvent::BackToFiles);
                    cx.emit(FolderEvent::Navigate("Folders".into()));
                },
            ))
            .child(div().text_color(p.faint).child("/"))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(self.folder.clone()),
            );

        // Back button (.ui-as-back)
        let back_btn = activate(
            div()
                .id(pro_eid(&self.id, "back-files"))
                .role(Role::Button)
                .tab_index(0)
                .w(px(64.))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(13.))
                .text_color(p.muted)
                .hover({
                    let ink = p.ink;
                    move |s| s.text_color(ink)
                })
                .cursor_pointer()
                .child(pro_svg("0 0 24 24", BACK_ARROW_SVG, 14., p.muted))
                .child(tr(cx, "Files", "文件")),
            cx,
            |_, _, cx| {
                cx.emit(FolderEvent::BackToFiles);
                cx.emit(FolderEvent::Navigate("Folders".into()));
            },
        );

        // Folder header title (or inline rename Editor)
        let folder_renaming = self.renaming == Some(RenameTarget::Folder);
        let title_el = if folder_renaming && let Some(ed) = self.rename_editor.clone() {
            div()
                .flex_1()
                .min_w_0()
                .max_w(px(340.))
                .h(px(32.))
                .px(px(8.))
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(p.focus)
                .flex()
                .items_center()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(div().w_full().child(ed))
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(if main_w < 600. { 20. } else { 24. }))
                .line_height(px(if main_w < 600. { 26. } else { 32. }))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .truncate()
                .child(self.folder.clone())
                .into_any_element()
        };

        // More (...) menu anchor
        let mut more_anchor = div()
            .relative()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(pro_icon_btn(
                pro_eid(&self.id, "folder-more-btn"),
                tr(cx, "More", "更多操作"),
                ProIcon::MoreHorizontal,
                28.,
                self.more_menu_open,
                cx,
                |this, _, cx| {
                    this.more_menu_open = !this.more_menu_open;
                    this.share_menu_open = false;
                    cx.notify();
                },
            ));

        {
            let folder_for_rn = self.folder.clone();
            let is_empty_now = self.empty || self.files.is_empty();
            let menu = div()
                .id(pro_eid(&self.id, "folder-more-menu"))
                .role(Role::Menu)
                .occlude()
                .absolute()
                .right_0()
                .w(px(208.))
                .p(px(4.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "menu-work-folder"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .hover({
                            let hv = p.hover;
                            let ink = p.ink;
                            move |s| s.bg(hv).text_color(ink)
                        })
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .text_size(px(13.))
                        .text_color(p.text)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::NewChat, 15., p.muted))
                        .child(tr(cx, "Work in folder", "在文件夹中协作")),
                    cx,
                    |this, w, cx| {
                        this.more_menu_open = false;
                        this.open_workspace_chat(w, cx);
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "menu-instructions"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .hover({
                            let hv = p.hover;
                            let ink = p.ink;
                            move |s| s.bg(hv).text_color(ink)
                        })
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .text_size(px(13.))
                        .text_color(p.text)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Note, 15., p.muted))
                        .child(tr(cx, "Project instructions", "项目专属指令")),
                    cx,
                    |this, w, cx| {
                        this.more_menu_open = false;
                        this.editing_instructions = true;
                        this.open_workspace_chat(w, cx);
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "menu-rename-folder"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .hover({
                            let hv = p.hover;
                            let ink = p.ink;
                            move |s| s.bg(hv).text_color(ink)
                        })
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .text_size(px(13.))
                        .text_color(p.text)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::PageDoc, 15., p.muted))
                        .child(tr(cx, "Rename folder", "重命名文件夹")),
                    cx,
                    move |this, w, cx| {
                        this.more_menu_open = false;
                        this.start_rename(RenameTarget::Folder, folder_for_rn.as_ref(), w, cx);
                    },
                ))
                .child(div().h(px(1.)).my(px(3.)).mx(px(6.)).bg(p.line))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "menu-toggle-empty"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .hover({
                            let hv = p.hover;
                            move |s| s.bg(hv)
                        })
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .text_size(px(13.))
                        .text_color(if is_empty_now { p.ink } else { p.red })
                        .cursor_pointer()
                        .child(pro_icon(
                            if is_empty_now {
                                ProIcon::Refresh
                            } else {
                                ProIcon::Trash
                            },
                            15.,
                            if is_empty_now { p.muted } else { p.red },
                        ))
                        .child(if is_empty_now {
                            tr(cx, "Restore sample files", "恢复示例文件")
                        } else {
                            tr(cx, "Clear folder files", "清空文件夹")
                        }),
                    cx,
                    move |this, _, cx| {
                        this.more_menu_open = false;
                        if is_empty_now {
                            this.empty = false;
                            this.files = preset_files_for_folder(this.folder.as_ref());
                        } else {
                            this.empty = true;
                        }
                        cx.notify();
                    },
                ));

            more_anchor = more_anchor.child(pro_presence(
                pro_eid(&self.id, "more-menu-pres"),
                self.more_menu_open,
                0.18,
                0.12,
                move |a| {
                    deferred(menu.top(px(32. - 4. * (1. - a))).opacity(a))
                        .with_priority(3)
                        .into_any_element()
                },
            ));
        }

        // Share button + popover
        let mut share_anchor = div()
            .relative()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(pro_btn(
                pro_eid(&self.id, "share-folder-btn"),
                tr(cx, "Share", "共享"),
                Some(ProIcon::Users),
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, _, cx| {
                    this.share_menu_open = !this.share_menu_open;
                    this.more_menu_open = false;
                    cx.notify();
                },
            ));

        {
            let folder_share_name = self.folder.clone();
            let ws_name = self.workspace.clone();
            let share_pop = div()
                .id(pro_eid(&self.id, "share-popover"))
                .role(Role::Dialog)
                .occlude()
                .absolute()
                .right_0()
                .w(px(268.))
                .p(px(12.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(format!(
                                            "{} {}",
                                            tr(cx, "Share with", "共享给"),
                                            ws_name
                                        )),
                                )
                                .child(div().text_size(px(11.5)).text_color(p.muted).child(tr(
                                    cx,
                                    "Members can view & chat with files",
                                    "工作区成员可查看并引用文件",
                                ))),
                        )
                        .child(pro_switch(
                            pro_eid(&self.id, "share-ws-switch"),
                            tr(cx, "Share with workspace", "与工作区共享"),
                            self.shared,
                            cx,
                            |this, _, cx| {
                                this.shared = !this.shared;
                                cx.notify();
                            },
                        )),
                )
                .child(div().h(px(1.)).bg(p.line))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.faint)
                                .child(tr(cx, "CONNECTED SOURCES", "已连接数据源")),
                        )
                        .children(self.sources.iter().enumerate().map(|(si, src)| {
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(7.))
                                        .child(pro_app_tile(src.app.as_ref(), 20., cx))
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .text_color(p.ink)
                                                .child(format!("{} · {}", src.app, src.scope)),
                                        ),
                                )
                                .child(pro_switch(
                                    pro_eid(&self.id, format!("share-src-sw-{si}")),
                                    src.app.clone(),
                                    src.connected,
                                    cx,
                                    move |this, _, cx| {
                                        if let Some(s) = this.sources.get_mut(si) {
                                            s.connected = !s.connected;
                                            cx.notify();
                                        }
                                    },
                                ))
                        })),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "copy-folder-link"),
                    tr(cx, "Copy folder link", "复制文件夹链接"),
                    Some(ProIcon::Link),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    move |this, _, cx| {
                        this.share_menu_open = false;
                        cx.emit(FolderEvent::ShareFolder(folder_share_name.clone()));
                        this.show_toast(tr(cx, "Folder link copied", "已复制文件夹链接"), cx);
                    },
                ));

            share_anchor = share_anchor.child(pro_presence(
                pro_eid(&self.id, "share-menu-pres"),
                self.share_menu_open,
                0.18,
                0.12,
                move |a| {
                    deferred(share_pop.top(px(32. - 4. * (1. - a))).opacity(a))
                        .with_priority(3)
                        .into_any_element()
                },
            ));
        }

        let upload_btn = pro_btn(
            pro_eid(&self.id, "upload-files-btn"),
            tr(cx, "Upload files", "上传文件"),
            Some(ProIcon::Upload),
            ProBtnStyle::Primary,
            true,
            cx,
            |this, _, cx| this.upload_file(cx),
        );

        let folder_head = div()
            .mt(px(if main_w < 600. { 18. } else { 22. }))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(12.))
            .child(pro_svg(
                "0 0 67 67",
                FOLDER_MARK_SVG,
                26.,
                rgb(0xffffff).into(),
            ))
            .child(title_el)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(more_anchor)
                    .child(share_anchor)
                    .child(upload_btn),
            );

        // Files grid / loading / empty
        let cols = if main_w < 460. {
            1usize
        } else if main_w < 760. {
            2usize
        } else {
            4usize
        };
        let preview_h = if main_w < 600. { 128. } else { 152. };

        let content_body = if self.loading {
            let mut skel_grid = div().mt(px(20.)).flex().flex_col().gap(px(16.));
            for chunk in [0usize, 1, 2, 3, 4].chunks(cols) {
                let mut row = div().flex().gap(px(12.));
                for &idx in chunk {
                    row =
                        row.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .max_w(px(239.))
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .h(px(preview_h))
                                        .p(px(3.))
                                        .rounded(px(12.))
                                        .bg(if p.dark {
                                            hsla(0., 0., 1., 0.03)
                                        } else {
                                            hsla(0., 0., 0., 0.02)
                                        })
                                        .border_1()
                                        .border_color(p.line)
                                        .child(
                                            div()
                                                .size_full()
                                                .rounded(px(10.))
                                                .bg(p.card)
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .child(pro_skeleton_bar(
                                                    52.,
                                                    66.,
                                                    6.,
                                                    phase + idx as f32 * 0.09,
                                                    cx,
                                                )),
                                        ),
                                )
                                .child(div().mt(px(8.)).h(px(20.)).flex().items_center().child(
                                    pro_skeleton_bar(112., 12., 4., phase + idx as f32 * 0.09, cx),
                                ))
                                .child(div().h(px(16.)).flex().items_center().child(
                                    pro_skeleton_bar(136., 10., 4., phase + idx as f32 * 0.09, cx),
                                )),
                        );
                }
                if chunk.len() < cols {
                    for _ in chunk.len()..cols {
                        row = row.child(div().flex_1().min_w_0().max_w(px(239.)));
                    }
                }
                skel_grid = skel_grid.child(row);
            }
            skel_grid.into_any_element()
        } else if self.empty || self.files.is_empty() {
            div()
                .mt(px(20.))
                .rounded(px(12.))
                .border_1()
                .border_color(p.line)
                .bg(p.card.opacity(0.5))
                .child(pro_empty_state(
                    ProIcon::Folder,
                    tr(cx, "No files yet", "暂无文件"),
                    tr(
                        cx,
                        "Upload documents or spreadsheets, or start a chat scoped to this folder.",
                        "上传文档或表格，或在此文件夹内发起对话。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-work-in-folder"),
                        tr(cx, "Work in folder", "在文件夹中协作").into(),
                        Box::new(|this: &mut Self, w, cx| this.open_workspace_chat(w, cx)),
                    )),
                    cx,
                ))
                .into_any_element()
        } else {
            let mut grid = div().mt(px(20.)).flex().flex_col().gap(px(16.));
            for chunk in self.files.chunks(cols) {
                let mut row = div().flex().gap(px(12.));
                for file in chunk {
                    row = row.child(self.render_file_card(file, preview_h, &p, cx));
                }
                if chunk.len() < cols {
                    for _ in chunk.len()..cols {
                        row = row.child(div().flex_1().min_w_0().max_w(px(239.)));
                    }
                }
                grid = grid.child(row);
            }
            grid.into_any_element()
        };

        let pad_x = if main_w < 600. {
            16.
        } else if main_w < 900. {
            24.
        } else {
            32.
        };
        let pad_top = if main_w < 900. { 8. } else { 12. };
        let pad_bot = if main_w < 600. { 40. } else { 56. };

        let files_vtx = pro_view_transition(
            pro_eid(&self.id, "folder-files-vtx"),
            format!("{}-{}-{}", self.folder, self.loading, self.empty),
            content_body,
        );

        div()
            .id(pro_eid(&self.id, "folder-page-root"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.more_menu_open || this.share_menu_open {
                        this.more_menu_open = false;
                        this.share_menu_open = false;
                        cx.notify();
                    }
                    if this.renaming.is_some() {
                        this.commit_rename(cx);
                    }
                }),
            )
            .relative()
            .size_full()
            .bg(p.panel)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10. && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .size_0(),
            )
            .child(crumbs)
            .child(
                div()
                    .id(pro_eid(&self.id, "folder-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pt(px(pad_top))
                    .px(px(pad_x))
                    .pb(px(pad_bot))
                    .flex()
                    .flex_col()
                    .child(back_btn)
                    .child(folder_head)
                    .child(files_vtx),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

impl Render for FolderScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FolderFileKind, append_prompt_to_thread, default_threads, preset_files_for_folder,
    };

    #[test]
    fn preset_folders_match_spec_counts_and_formats() {
        let q3 = preset_files_for_folder("Q3 Campaign");
        assert_eq!(q3.len(), 5);
        assert_eq!(q3[0].name.as_ref(), "Campaign-Brief");
        assert_eq!(q3[0].kind, FolderFileKind::Document);
        assert_eq!(q3[1].name.as_ref(), "Launch-Plan");
        assert_eq!(q3[1].kind, FolderFileKind::Pdf);
        assert_eq!(q3[2].name.as_ref(), "Media-Budget");
        assert_eq!(q3[2].kind, FolderFileKind::Spreadsheet);

        assert_eq!(preset_files_for_folder("Brand assets").len(), 3);
        assert_eq!(preset_files_for_folder("Hiring").len(), 4);
        assert_eq!(preset_files_for_folder("Research").len(), 2);
    }

    #[test]
    fn scoped_folder_thread_appends_grounded_turn() {
        let mut threads = default_threads("Q3 Campaign");
        let initial_len = threads[0].messages.len();

        assert!(append_prompt_to_thread(&mut threads, 0, "Q3 Campaign", 5, "   ").is_none());

        let sent = append_prompt_to_thread(
            &mut threads,
            0,
            "Q3 Campaign",
            5,
            "Check partner budget delta",
        );
        assert_eq!(sent.as_deref(), Some("Check partner budget delta"));
        assert_eq!(threads[0].messages.len(), initial_len + 2);
        assert!(threads[0].messages.last().is_some_and(|(is_user, text)| {
            !*is_user && text.contains("5 files in Q3 Campaign")
        }));
    }
}
