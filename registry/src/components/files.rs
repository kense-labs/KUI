//! Workspace folder overview and reusable file template gallery inside an application shell.
//!
//! Pairs a filterable, sortable project folder overview (`Grid` and `List` modes) with a
//! reusable document template gallery, inline rename editors, and an on-demand Quick Look
//! inspector hosted in [`AppShell`].

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, ProTone, pro_badge, pro_btn, pro_eid, pro_empty_state,
            pro_fold, pro_icon_btn, pro_mark_selected, pro_person_avatar, pro_pill_indicator,
            pro_presence, pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_toast_banner,
            pro_view_transition, tr,
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

const SORT_ASC_SVG: &str = r##"<path d="M7 4V20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17 19L17 4" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M10 6.99998C10 6.99998 7.79053 4.00001 6.99998 4C6.20942 3.99999 4 7 4 7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M20 17C20 17 17.7905 20 17 20C16.2094 20 14 17 14 17" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"##;

const SORT_DESC_SVG: &str = r##"<path d="M7 4V20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17 19L17 4" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M10 17C10 17 7.79053 20 6.99998 20C6.20942 20 4 17 4 17" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M20 7C20 7 17.7905 4 17 4C16.2094 4 14 7 14 7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"##;

const VIEW_GRID_SVG: &str = r##"<path d="M3.88884 9.66294C4.39329 10 5.09552 10 6.49998 10C7.90445 10 8.60668 10 9.11113 9.66294C9.32951 9.51702 9.51701 9.32952 9.66292 9.11114C9.99998 8.60669 9.99998 7.90446 9.99998 6.5C9.99998 5.09554 9.99998 4.39331 9.66292 3.88886C9.51701 3.67048 9.32951 3.48298 9.11113 3.33706C8.60668 3 7.90445 3 6.49998 3C5.09552 3 4.39329 3 3.88884 3.33706C3.67046 3.48298 3.48296 3.67048 3.33705 3.88886C2.99998 4.39331 2.99998 5.09554 2.99998 6.5C2.99998 7.90446 2.99998 8.60669 3.33705 9.11114C3.48296 9.32952 3.67046 9.51702 3.88884 9.66294Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M14.8888 9.66294C15.3933 10 16.0955 10 17.5 10C18.9044 10 19.6067 10 20.1111 9.66294C20.3295 9.51702 20.517 9.32952 20.6629 9.11114C21 8.60669 21 7.90446 21 6.5C21 5.09554 21 4.39331 20.6629 3.88886C20.517 3.67048 20.3295 3.48298 20.1111 3.33706C19.6067 3 18.9044 3 17.5 3C16.0955 3 15.3933 3 14.8888 3.33706C14.6705 3.48298 14.483 3.67048 14.337 3.88886C14 4.39331 14 5.09554 14 6.5C14 7.90446 14 8.60669 14.337 9.11114C14.483 9.32952 14.6705 9.51702 14.8888 9.66294Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M3.88884 20.6629C4.39329 21 5.09552 21 6.49998 21C7.90445 21 8.60668 21 9.11113 20.6629C9.32951 20.517 9.51701 20.3295 9.66292 20.1111C9.99998 19.6067 9.99998 18.9045 9.99998 17.5C9.99998 16.0955 9.99998 15.3933 9.66292 14.8889C9.51701 14.6705 9.32951 14.483 9.11113 14.3371C8.60668 14 7.90445 14 6.49998 14C5.09552 14 4.39329 14 3.88884 14.3371C3.67046 14.483 3.48296 14.6705 3.33705 14.8889C2.99998 15.3933 2.99998 16.0955 2.99998 17.5C2.99998 18.9045 2.99998 19.6067 3.33705 20.1111C3.48296 20.3295 3.67046 20.517 3.88884 20.6629Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M14.8888 20.6629C15.3933 21 16.0955 21 17.5 21C18.9044 21 19.6067 21 20.1111 20.6629C20.3295 20.517 20.517 20.3295 20.6629 20.1111C21 19.6067 21 18.9045 21 17.5C21 16.0955 21 15.3933 20.6629 14.8889C20.517 14.6705 20.3295 14.483 20.1111 14.3371C19.6067 14 18.9044 14 17.5 14C16.0955 14 15.3933 14 14.8888 14.3371C14.6705 14.483 14.483 14.6705 14.337 14.8889C14 15.3933 14 16.0955 14 17.5C14 18.9045 14 19.6067 14.337 20.1111C14.483 20.3295 14.6705 20.517 14.8888 20.6629Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path>"##;

const FUNNEL_SVG: &str = r##"<path d="M8.85746 12.5061C6.36901 10.6456 4.59564 8.59915 3.62734 7.44867C3.3276 7.09253 3.22938 6.8319 3.17033 6.3728C2.96811 4.8008 2.86701 4.0148 3.32795 3.5074C3.7889 3 4.60404 3 6.23433 3H17.7657C19.396 3 20.2111 3 20.672 3.5074C21.133 4.0148 21.0319 4.8008 20.8297 6.37281C20.7706 6.83191 20.6724 7.09254 20.3726 7.44867C19.403 8.60062 17.6261 10.6507 15.1326 12.5135C14.907 12.6821 14.7583 12.9567 14.7307 13.2614C14.4837 15.992 14.2559 17.4876 14.1141 18.2442C13.8853 19.4657 12.1532 20.2006 11.226 20.8563C10.6741 21.2466 10.0043 20.782 9.93278 20.1778C9.79643 19.0261 9.53961 16.6864 9.25927 13.2614C9.23409 12.9539 9.08486 12.6761 8.85746 12.5061Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"##;

/// Visibility and sharing scope of a folder or reusable file template.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileAccess {
    #[default]
    Shared,
    Private,
}

impl FileAccess {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Shared => tr(cx, "Shared", "已共享"),
            Self::Private => tr(cx, "Private", "私密"),
        }
    }

    fn icon(self) -> ProIcon {
        match self {
            Self::Shared => ProIcon::Users,
            Self::Private => ProIcon::KeyLock,
        }
    }

    fn toggled(self) -> Self {
        match self {
            Self::Shared => Self::Private,
            Self::Private => Self::Shared,
        }
    }
}

/// Document format shown on template preview cards and type filters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileDocKind {
    #[default]
    Document,
    Spreadsheet,
    Pdf,
    Image,
    Code,
}

impl FileDocKind {
    /// Short uppercase tag displayed on the miniature document badge.
    pub fn badge(self) -> &'static str {
        match self {
            Self::Document => "DOC",
            Self::Spreadsheet => "XLS",
            Self::Pdf => "PDF",
            Self::Image => "IMG",
            Self::Code => "CODE",
        }
    }

    /// Human-readable document format label.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Document => tr(cx, "Document", "文档"),
            Self::Spreadsheet => tr(cx, "Spreadsheet", "电子表格"),
            Self::Pdf => "PDF",
            Self::Image => tr(cx, "Image", "图片"),
            Self::Code => tr(cx, "Code", "代码"),
        }
    }

    fn tag_color(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Document => p.blue,
            Self::Spreadsheet => rgb(0x13b3a0).into(),
            Self::Pdf => rgb(0xf04438).into(),
            Self::Image => p.amber,
            Self::Code => p.purple,
        }
    }
}

/// Visibility filter tab above the `Folders` section.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FolderTab {
    #[default]
    All,
    Shared,
    Private,
}

/// Layout mode of the `Folders` section.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FolderViewMode {
    #[default]
    Grid,
    List,
}

/// Ownership filter tab above the `File templates` section.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TemplateOwnerTab {
    #[default]
    All,
    Mine,
    SharedWithMe,
}

/// Sortable attribute for folder cards and list rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileSortField {
    #[default]
    Name,
    Modified,
    Size,
    Owner,
}

/// One workspace folder card or list row in [`FilesScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesFolderItem {
    pub id: SharedString,
    pub name: SharedString,
    pub access: FileAccess,
    pub file_count: usize,
    pub owner: SharedString,
    pub modified: SharedString,
    pub modified_rank: u32,
    pub size_kb: u32,
}

impl FilesFolderItem {
    /// Construct a folder item with its access scope and file count.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        access: FileAccess,
        file_count: usize,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            access,
            file_count,
            owner: "Alex Morgan".into(),
            modified: "Today".into(),
            modified_rank: 1,
            size_kb: (file_count as u32).saturating_mul(420).max(120),
        }
    }

    /// Set the folder owner display name.
    pub fn with_owner(mut self, owner: impl Into<SharedString>) -> Self {
        self.owner = owner.into();
        self
    }

    /// Set the relative modified timestamp and sort rank (`0` = newest).
    pub fn with_modified(mut self, modified: impl Into<SharedString>, rank: u32) -> Self {
        self.modified = modified.into();
        self.modified_rank = rank;
        self
    }

    /// Set total folder size in kilobytes.
    pub fn with_size_kb(mut self, size_kb: u32) -> Self {
        self.size_kb = size_kb;
        self
    }
}

/// One reusable document template card in [`FilesScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesTemplateItem {
    pub id: SharedString,
    pub name: SharedString,
    pub kind: FileDocKind,
    pub owner: SharedString,
    pub access: FileAccess,
    pub modified: SharedString,
    pub size_kb: u32,
    pub summary: SharedString,
}

impl FilesTemplateItem {
    /// Construct a reusable template item.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        kind: FileDocKind,
        owner: impl Into<SharedString>,
        access: FileAccess,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            owner: owner.into(),
            access,
            modified: "Yesterday".into(),
            size_kb: 340,
            summary: "Reusable workspace template ready for new threads and folders.".into(),
        }
    }

    /// Override the relative modified timestamp and size in kilobytes.
    pub fn with_meta(mut self, modified: impl Into<SharedString>, size_kb: u32) -> Self {
        self.modified = modified.into();
        self.size_kb = size_kb;
        self
    }

    /// Override the Quick Look summary description.
    pub fn with_summary(mut self, summary: impl Into<SharedString>) -> Self {
        self.summary = summary.into();
        self
    }
}

/// Events emitted by [`FilesScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilesEvent {
    Navigate(SharedString),
    OpenFolder(SharedString),
    CreateFolder(SharedString),
    SelectTemplate(SharedString),
    AddTemplate(SharedString),
    RenameItem {
        id: SharedString,
        name: SharedString,
    },
    DeleteItem(SharedString),
}

impl EventEmitter<FilesEvent> for FilesScreen {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SelectedRef {
    Folder(SharedString),
    Template(SharedString),
}

fn default_folders() -> Vec<FilesFolderItem> {
    vec![
        FilesFolderItem::new("brand-assets", "Brand assets", FileAccess::Shared, 3)
            .with_owner("Jonas Weber")
            .with_modified("Yesterday", 2)
            .with_size_kb(4_820),
        FilesFolderItem::new("hiring", "Hiring", FileAccess::Private, 4)
            .with_owner("Alex Morgan")
            .with_modified("3 days ago", 3)
            .with_size_kb(1_640),
        FilesFolderItem::new("q3-campaign", "Q3 Campaign", FileAccess::Shared, 5)
            .with_owner("Alex Morgan")
            .with_modified("2 hours ago", 1)
            .with_size_kb(3_180),
        FilesFolderItem::new("research", "Research", FileAccess::Private, 2)
            .with_owner("Alex Morgan")
            .with_modified("Last week", 4)
            .with_size_kb(960),
    ]
}

fn default_templates() -> Vec<FilesTemplateItem> {
    vec![
        FilesTemplateItem::new(
            "brand-guidelines",
            "Brand-Guidelines",
            FileDocKind::Pdf,
            "Jonas Weber",
            FileAccess::Shared,
        )
        .with_meta("2 days ago", 2_480)
        .with_summary(
            "Typography hierarchy, logo clearspace rules, monochrome lockups, and approved product screenshots.",
        ),
        FilesTemplateItem::new(
            "campaign-budget",
            "Campaign-Budget",
            FileDocKind::Spreadsheet,
            "Jonas Weber",
            FileAccess::Shared,
        )
        .with_meta("Yesterday", 640)
        .with_summary(
            "Channel-by-channel Q3 spend tracker with planned, committed, and variance formulas.",
        ),
        FilesTemplateItem::new(
            "design-book",
            "Design-Book",
            FileDocKind::Document,
            "Alex Morgan",
            FileAccess::Private,
        )
        .with_meta("4 days ago", 1_120)
        .with_summary(
            "Component review rubric, token naming checklist, and dark-mode contrast sign-off template.",
        ),
        FilesTemplateItem::new(
            "weekly-report",
            "Weekly-Report",
            FileDocKind::Spreadsheet,
            "Alex Morgan",
            FileAccess::Private,
        )
        .with_meta("Last week", 510)
        .with_summary(
            "Weekly KPI rollup covering pipeline velocity, activation rate, and open launch blockers.",
        ),
    ]
}

fn format_size_kb(kb: u32) -> String {
    if kb >= 1024 {
        format!("{:.1} MB", kb as f32 / 1024.)
    } else {
        format!("{kb} KB")
    }
}

fn filter_and_sort_folders(
    items: &[FilesFolderItem],
    tab: FolderTab,
    query: &str,
    field: FileSortField,
    asc: bool,
) -> Vec<FilesFolderItem> {
    let q = query.trim().to_ascii_lowercase();
    let mut out: Vec<FilesFolderItem> = items
        .iter()
        .filter(|it| match tab {
            FolderTab::All => true,
            FolderTab::Shared => it.access == FileAccess::Shared,
            FolderTab::Private => it.access == FileAccess::Private,
        })
        .filter(|it| {
            q.is_empty()
                || it.name.to_ascii_lowercase().contains(&q)
                || it.owner.to_ascii_lowercase().contains(&q)
        })
        .cloned()
        .collect();
    out.sort_by(|a, b| {
        let ord = match field {
            FileSortField::Name => a
                .name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase()),
            FileSortField::Modified => a.modified_rank.cmp(&b.modified_rank),
            FileSortField::Size => a.size_kb.cmp(&b.size_kb),
            FileSortField::Owner => a
                .owner
                .to_ascii_lowercase()
                .cmp(&b.owner.to_ascii_lowercase()),
        };
        if asc { ord } else { ord.reverse() }
    });
    out
}

fn filter_and_sort_templates(
    items: &[FilesTemplateItem],
    tab: TemplateOwnerTab,
    current_user: &str,
    kind_filter: Option<FileDocKind>,
    query: &str,
    asc: bool,
) -> Vec<FilesTemplateItem> {
    let q = query.trim().to_ascii_lowercase();
    let me = current_user.trim().to_ascii_lowercase();
    let mut out: Vec<FilesTemplateItem> = items
        .iter()
        .filter(|it| match tab {
            TemplateOwnerTab::All => true,
            TemplateOwnerTab::Mine => it.owner.trim().to_ascii_lowercase() == me,
            TemplateOwnerTab::SharedWithMe => {
                it.owner.trim().to_ascii_lowercase() != me && it.access == FileAccess::Shared
            }
        })
        .filter(|it| kind_filter.is_none_or(|k| it.kind == k))
        .filter(|it| {
            q.is_empty()
                || it.name.to_ascii_lowercase().contains(&q)
                || it.owner.to_ascii_lowercase().contains(&q)
                || it.kind.badge().to_ascii_lowercase().contains(&q)
        })
        .cloned()
        .collect();
    out.sort_by(|a, b| {
        let ord = a
            .name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase());
        if asc { ord } else { ord.reverse() }
    });
    out
}

/// Workspace folder overview and template library screen (`Folders / Overview`).
pub struct FilesScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    loading: bool,
    empty: bool,
    learn_more_open: bool,
    folders: Vec<FilesFolderItem>,
    templates: Vec<FilesTemplateItem>,
    folder_tab: FolderTab,
    folder_view: FolderViewMode,
    folder_sort_field: FileSortField,
    folder_sort_asc: bool,
    template_tab: TemplateOwnerTab,
    template_kind_filter: Option<FileDocKind>,
    template_sort_asc: bool,
    type_menu_open: bool,
    folder_search: Option<Entity<Editor>>,
    template_search: Option<Entity<Editor>>,
    renaming_id: Option<SharedString>,
    rename_editor: Option<Entity<Editor>>,
    selected: Option<SelectedRef>,
    quick_look: Option<SelectedRef>,
    created_seq: usize,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
    boot_instant: Instant,
    toast: Option<(SharedString, Instant)>,
}

impl Focusable for FilesScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl FilesScreen {
    /// Construct a new [`FilesScreen`] hosted inside an [`AppShell`] (`Acme` preset).
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Chat, shell_cx)
                .with_active("folders")
                .with_work_area(false, false)
                .with_panel_width(348.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });
        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(FilesEvent::Navigate(label.clone()));
            }
            AppShellEvent::PanelOpen(false) => {
                this.quick_look = None;
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            standalone: false,
            workspace: "Acme".into(),
            user_name: "Alex Morgan".into(),
            user_email: "alex@acme.ai".into(),
            loading: false,
            empty: false,
            learn_more_open: false,
            folders: default_folders(),
            templates: default_templates(),
            folder_tab: FolderTab::All,
            folder_view: FolderViewMode::Grid,
            folder_sort_field: FileSortField::Name,
            folder_sort_asc: true,
            template_tab: TemplateOwnerTab::All,
            template_kind_filter: None,
            template_sort_asc: true,
            type_menu_open: false,
            folder_search: None,
            template_search: None,
            renaming_id: None,
            rename_editor: None,
            selected: None,
            quick_look: None,
            created_seq: 0,
            focus: cx.focus_handle(),
            measured_width: Rc::new(Cell::new(920.)),
            boot_instant: Instant::now(),
            toast: None,
        }
    }

    /// Render only the inner page without the outer [`AppShell`] chrome.
    pub fn standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Render only the inner page without the outer [`AppShell`] chrome.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
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

    /// Override the workspace name displayed in the [`AppShell`] sidebar.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws = workspace.into();
        self.workspace = ws.clone();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx))
                .with_preset(AppShellPreset::Chat, cx)
                .with_workspace(ws)
                .with_user(self.user_name.clone(), self.user_email.clone())
                .with_active("folders")
                .with_work_area(false, false)
                .with_panel_width(348.);
        });
        self.rebind_shell(cx);
        self
    }

    /// Override the signed-in account shown at the bottom of the sidebar.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.user_name = name.into();
        self.user_email = email.into();
        let ws = self.workspace.clone();
        let uname = self.user_name.clone();
        let uemail = self.user_email.clone();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx))
                .with_preset(AppShellPreset::Chat, cx)
                .with_workspace(ws)
                .with_user(uname, uemail)
                .with_active("folders")
                .with_work_area(false, false)
                .with_panel_width(348.);
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

    /// Replace the initial list of workspace folders.
    pub fn with_folders(mut self, folders: Vec<FilesFolderItem>) -> Self {
        self.folders = folders;
        self
    }

    /// Replace the initial list of reusable file templates.
    pub fn with_templates(mut self, templates: Vec<FilesTemplateItem>) -> Self {
        self.templates = templates;
        self
    }

    /// Set the initial folder layout mode (`Grid` or `List`).
    pub fn with_view_mode(mut self, mode: FolderViewMode) -> Self {
        self.folder_view = mode;
        self
    }

    /// Enable or disable skeleton placeholders for both sections.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the zero-data empty state for both sections.
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

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.folder_search.is_none() {
            let ph: SharedString = tr(cx, "Search folders...", "搜索文件夹...").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed | EditEvent::Submitted) {
                    cx.notify();
                }
            })
            .detach();
            self.folder_search = Some(ed);
        }
        if self.template_search.is_none() {
            let ph: SharedString = tr(cx, "Search templates...", "搜索模板...").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed | EditEvent::Submitted) {
                    cx.notify();
                }
            })
            .detach();
            self.template_search = Some(ed);
        }
    }

    fn start_rename(
        &mut self,
        target_id: SharedString,
        current_name: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let initial = current_name.to_string();
        let ed = cx.new(|cx| {
            let mut e = Editor::new("Rename", InputMode::Single, w, cx);
            e.font_size = 13.;
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
                this.renaming_id = None;
                this.rename_editor = None;
                cx.notify();
            }
            _ => {}
        })
        .detach();
        self.renaming_id = Some(target_id);
        self.rename_editor = Some(ed);
        cx.notify();
    }

    fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(target_id) = self.renaming_id.take() else {
            return;
        };
        let next_name = self
            .rename_editor
            .take()
            .map(|ed| ed.read(cx).value().trim().to_string())
            .unwrap_or_default();
        if next_name.is_empty() {
            cx.notify();
            return;
        }
        let shared: SharedString = next_name.into();
        let mut applied = false;
        for f in &mut self.folders {
            if f.id == target_id {
                f.name = shared.clone();
                applied = true;
                break;
            }
        }
        if !applied {
            for t in &mut self.templates {
                if t.id == target_id {
                    t.name = shared.clone();
                    applied = true;
                    break;
                }
            }
        }
        if applied {
            cx.emit(FilesEvent::RenameItem {
                id: target_id,
                name: shared.clone(),
            });
            let msg = format!("{} \"{}\"", tr(cx, "Renamed to", "已重命名为"), shared);
            self.show_toast(msg, cx);
        } else {
            cx.notify();
        }
    }

    fn create_folder(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.empty {
            self.folders.clear();
            self.empty = false;
        }
        self.created_seq += 1;
        let seq = self.created_seq;
        let name: SharedString = if seq == 1 {
            tr(cx, "Untitled folder", "新建文件夹").into()
        } else {
            format!("{} {seq}", tr(cx, "Untitled folder", "新建文件夹")).into()
        };
        let id: SharedString = format!("folder-new-{seq}").into();
        let access = match self.folder_tab {
            FolderTab::Private => FileAccess::Private,
            FolderTab::All | FolderTab::Shared => FileAccess::Shared,
        };
        let item = FilesFolderItem::new(id.clone(), name.clone(), access, 0)
            .with_owner(self.user_name.clone())
            .with_modified(tr(cx, "Just now", "刚刚"), 0)
            .with_size_kb(0);
        self.folders.push(item);
        self.selected = Some(SelectedRef::Folder(id.clone()));
        cx.emit(FilesEvent::CreateFolder(name.clone()));
        self.start_rename(id, name.as_ref(), w, cx);
    }

    fn add_template(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.empty {
            self.templates.clear();
            self.empty = false;
        }
        self.created_seq += 1;
        let seq = self.created_seq;
        let kinds = [
            FileDocKind::Document,
            FileDocKind::Spreadsheet,
            FileDocKind::Pdf,
        ];
        let kind = self
            .template_kind_filter
            .unwrap_or(kinds[seq % kinds.len()]);
        let name: SharedString = match kind {
            FileDocKind::Document => format!("Project-Spec-{seq}").into(),
            FileDocKind::Spreadsheet => format!("Forecast-Model-{seq}").into(),
            FileDocKind::Pdf => format!("Executive-Deck-{seq}").into(),
            FileDocKind::Image => format!("Visual-Board-{seq}").into(),
            FileDocKind::Code => format!("Automation-Script-{seq}").into(),
        };
        let id: SharedString = format!("tpl-new-{seq}").into();
        let tpl = FilesTemplateItem::new(
            id.clone(),
            name.clone(),
            kind,
            self.user_name.clone(),
            FileAccess::Private,
        )
        .with_meta(tr(cx, "Just now", "刚刚"), 280);
        self.templates.push(tpl);
        if self.template_tab == TemplateOwnerTab::SharedWithMe {
            self.template_tab = TemplateOwnerTab::All;
        }
        self.selected = Some(SelectedRef::Template(id.clone()));
        self.quick_look = Some(SelectedRef::Template(id.clone()));
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(FilesEvent::AddTemplate(name.clone()));
        self.start_rename(id, name.as_ref(), w, cx);
    }

    fn delete_item(&mut self, target: &SelectedRef, cx: &mut Context<Self>) {
        match target {
            SelectedRef::Folder(fid) => {
                self.folders.retain(|f| &f.id != fid);
                cx.emit(FilesEvent::DeleteItem(fid.clone()));
            }
            SelectedRef::Template(tid) => {
                self.templates.retain(|t| &t.id != tid);
                cx.emit(FilesEvent::DeleteItem(tid.clone()));
            }
        }
        if self.quick_look.as_ref() == Some(target) {
            self.quick_look = None;
            self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
        }
        if self.selected.as_ref() == Some(target) {
            self.selected = None;
        }
        self.show_toast(tr(cx, "Item removed", "已移除项目"), cx);
    }

    fn toggle_access(&mut self, target: &SelectedRef, cx: &mut Context<Self>) {
        match target {
            SelectedRef::Folder(fid) => {
                if let Some(f) = self.folders.iter_mut().find(|f| &f.id == fid) {
                    f.access = f.access.toggled();
                }
            }
            SelectedRef::Template(tid) => {
                if let Some(t) = self.templates.iter_mut().find(|t| &t.id == tid) {
                    t.access = t.access.toggled();
                }
            }
        }
        cx.notify();
    }

    fn visible_folders(&self, cx: &App) -> Vec<FilesFolderItem> {
        if self.empty {
            return Vec::new();
        }
        let q = self
            .folder_search
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        filter_and_sort_folders(
            &self.folders,
            self.folder_tab,
            &q,
            self.folder_sort_field,
            self.folder_sort_asc,
        )
    }

    fn visible_templates(&self, cx: &App) -> Vec<FilesTemplateItem> {
        if self.empty {
            return Vec::new();
        }
        let q = self
            .template_search
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        filter_and_sort_templates(
            &self.templates,
            self.template_tab,
            self.user_name.as_ref(),
            self.template_kind_filter,
            &q,
            self.template_sort_asc,
        )
    }

    fn any_editor_focused(&self, w: &Window, cx: &App) -> bool {
        let check = |opt: &Option<Entity<Editor>>| {
            opt.as_ref()
                .is_some_and(|ed| ed.read(cx).focus.is_focused(w))
        };
        check(&self.folder_search) || check(&self.template_search) || check(&self.rename_editor)
    }

    fn step_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let mut refs: Vec<SelectedRef> = self
            .visible_folders(cx)
            .into_iter()
            .map(|f| SelectedRef::Folder(f.id))
            .collect();
        refs.extend(
            self.visible_templates(cx)
                .into_iter()
                .map(|t| SelectedRef::Template(t.id)),
        );
        if refs.is_empty() {
            return;
        }
        let next_idx = if let Some(cur) = &self.selected
            && let Some(pos) = refs.iter().position(|r| r == cur)
        {
            (pos as isize + delta).rem_euclid(refs.len() as isize) as usize
        } else if delta >= 0 {
            0
        } else {
            refs.len() - 1
        };
        let target = refs[next_idx].clone();
        if self.quick_look.is_some() {
            self.quick_look = Some(target.clone());
        }
        self.selected = Some(target);
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.key == "escape" {
            if self.renaming_id.is_some() {
                self.renaming_id = None;
                self.rename_editor = None;
                cx.notify();
                return;
            }
            if self.type_menu_open {
                self.type_menu_open = false;
                cx.notify();
                return;
            }
            if self.quick_look.is_some() {
                self.quick_look = None;
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
            "/" => {
                if let Some(ed) = &self.folder_search {
                    {
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                }
            }
            "v" | "V" => {
                self.folder_view = match self.folder_view {
                    FolderViewMode::Grid => FolderViewMode::List,
                    FolderViewMode::List => FolderViewMode::Grid,
                };
                cx.notify();
            }
            " " | "space" => {
                if let Some(sel) = self.selected.clone() {
                    if self.quick_look.as_ref() == Some(&sel) {
                        self.quick_look = None;
                        self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
                    } else {
                        self.quick_look = Some(sel);
                        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                    }
                    cx.notify();
                }
            }
            "enter" => {
                if let Some(sel) = self.selected.clone() {
                    match sel {
                        SelectedRef::Folder(fid) => {
                            if let Some(f) = self.folders.iter().find(|f| f.id == fid) {
                                let name = f.name.clone();
                                cx.emit(FilesEvent::OpenFolder(name.clone()));
                                cx.emit(FilesEvent::Navigate(name));
                            }
                        }
                        SelectedRef::Template(tid) => {
                            if let Some(t) = self.templates.iter().find(|t| t.id == tid) {
                                let name = t.name.clone();
                                self.quick_look = Some(SelectedRef::Template(tid));
                                self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                                cx.emit(FilesEvent::SelectTemplate(name));
                                cx.notify();
                            }
                        }
                    }
                }
            }
            "r" | "R" => {
                if let Some(sel) = self.selected.clone() {
                    match sel {
                        SelectedRef::Folder(fid) => {
                            if let Some(name) = self
                                .folders
                                .iter()
                                .find(|f| f.id == fid)
                                .map(|f| f.name.clone())
                            {
                                self.start_rename(fid, name.as_ref(), w, cx);
                            }
                        }
                        SelectedRef::Template(tid) => {
                            if let Some(name) = self
                                .templates
                                .iter()
                                .find(|t| t.id == tid)
                                .map(|t| t.name.clone())
                            {
                                self.start_rename(tid, name.as_ref(), w, cx);
                            }
                        }
                    }
                }
            }
            "backspace" => {
                if self.quick_look.is_some() {
                    self.quick_look = None;
                    self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
                    cx.notify();
                } else if self.folder_tab != FolderTab::All || self.template_kind_filter.is_some() {
                    self.folder_tab = FolderTab::All;
                    self.template_kind_filter = None;
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn render_doc_artwork(&self, kind: FileDocKind, p: &ProPalette) -> Div {
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
            FileDocKind::Pdf | FileDocKind::Image => {
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
            FileDocKind::Spreadsheet => {
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
            FileDocKind::Document | FileDocKind::Code => {
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

    fn render_search_box(
        &self,
        box_id: impl Into<ElementId>,
        editor: Option<Entity<Editor>>,
        p: &ProPalette,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let has_val = editor
            .as_ref()
            .is_some_and(|e| !e.read(cx).value().is_empty());
        let ed_for_clear = editor.clone();
        let box_el_id: ElementId = box_id.into();
        let clear_id: ElementId = (box_el_id.clone(), "clear").into();
        div()
            .id(box_el_id)
            .w(px(172.))
            .h(px(28.))
            .px(px(8.))
            .rounded(px(6.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .hover({
                let ls = p.line_strong;
                move |s| s.border_color(ls)
            })
            .flex()
            .items_center()
            .gap(px(6.))
            .child(pro_icon(ProIcon::Search, 14., p.muted))
            .when_some(editor, |d, ed| d.child(div().flex_1().min_w_0().child(ed)))
            .when(has_val, |d| {
                d.child(activate(
                    div()
                        .id(clear_id)
                        .role(Role::Button)
                        .aria_label("Clear search")
                        .tab_index(0)
                        .size(px(18.))
                        .rounded(px(4.))
                        .hover({
                            let hv = p.hover;
                            move |s| s.bg(hv)
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Close, 11., p.muted)),
                    cx,
                    move |_, w, cx| {
                        if let Some(ed) = &ed_for_clear {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                    },
                ))
            })
    }

    fn render_folder_section(
        &mut self,
        main_w: f32,
        phase: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let visible = self.visible_folders(cx);
        let count_val = if self.loading {
            self.folders.len()
        } else {
            visible.len()
        };

        // Left filter tabs: All | Shared | Private
        let tab_specs = [
            (FolderTab::All, tr(cx, "All", "全部"), "ftab-all"),
            (FolderTab::Shared, tr(cx, "Shared", "已共享"), "ftab-shared"),
            (
                FolderTab::Private,
                tr(cx, "Private", "私密"),
                "ftab-private",
            ),
        ];
        let ftab_sensor = pro_selection_sensor();
        let ftab_key = match self.folder_tab {
            FolderTab::All => "ftab-all",
            FolderTab::Shared => "ftab-shared",
            FolderTab::Private => "ftab-private",
        };
        let mut tabs_row =
            div()
                .relative()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "ftab-pill"),
                    ftab_key,
                    ftab_sensor.clone(),
                    6.,
                    p.select,
                    Some(p.line_strong),
                    false,
                ));
        for (tab_val, label, suffix) in tab_specs {
            let active = self.folder_tab == tab_val;
            let ink = p.ink;
            let txt = p.text;
            let ln = p.line;
            let focus_c = p.focus;
            tabs_row = tabs_row.child(activate(
                div()
                    .id(pro_eid(&self.id, suffix))
                    .role(Role::Tab)
                    .aria_label(label)
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if active { transparent_black() } else { ln })
                    .when(active, |d| d.child(pro_mark_selected(&ftab_sensor)))
                    .text_size(px(13.))
                    .text_color(if active { ink } else { txt })
                    .when(!active, |d| d.hover(move |s| s.text_color(ink)))
                    .focus_visible(move |s| s.border_color(focus_c))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .child(label),
                cx,
                move |this, _, cx| {
                    this.folder_tab = tab_val;
                    cx.notify();
                },
            ));
        }

        // Right toolbar: Sort | View (Grid / List) | Search folders... | + Create folder
        let sort_aria = if self.folder_sort_asc {
            tr(cx, "Sorted A to Z – sort Z to A", "当前正序 – 切换倒序")
        } else {
            tr(cx, "Sorted Z to A – sort A to Z", "当前倒序 – 切换正序")
        };
        let sort_svg = if self.folder_sort_asc {
            SORT_ASC_SVG
        } else {
            SORT_DESC_SVG
        };
        let sort_btn = activate(
            div()
                .id(pro_eid(&self.id, "folder-sort"))
                .role(Role::Button)
                .aria_label(sort_aria)
                .tab_index(0)
                .size(px(28.))
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .hover({
                    let ls = p.line_strong;
                    move |s| s.border_color(ls)
                })
                .focus_visible({
                    let fc = p.focus;
                    move |s| s.border_color(fc)
                })
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .child(pro_svg("0 0 24 24", sort_svg, 15., p.muted)),
            cx,
            |this, _, cx| {
                this.folder_sort_asc = !this.folder_sort_asc;
                cx.notify();
            },
        );

        let is_grid = self.folder_view == FolderViewMode::Grid;
        let fview_sensor = pro_selection_sensor();
        let view_toggle = div()
            .relative()
            .h(px(28.))
            .p(px(2.))
            .rounded(px(6.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "fview-pill"),
                if is_grid { "grid" } else { "list" },
                fview_sensor.clone(),
                4.,
                p.card,
                Some(p.line),
                true,
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "view-grid"))
                    .role(Role::RadioButton)
                    .aria_label(tr(cx, "Grid", "网格"))
                    .tab_index(0)
                    .relative()
                    .w(px(26.))
                    .h(px(24.))
                    .rounded(px(4.))
                    .when(is_grid, |d| d.child(pro_mark_selected(&fview_sensor)))
                    .when(!is_grid, |d| {
                        d.hover({
                            let hv = p.hover;
                            move |s| s.bg(hv)
                        })
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_svg(
                        "0 0 24 24",
                        VIEW_GRID_SVG,
                        14.,
                        if is_grid { p.ink } else { p.faint },
                    )),
                cx,
                |this, _, cx| {
                    this.folder_view = FolderViewMode::Grid;
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "view-list"))
                    .role(Role::RadioButton)
                    .aria_label(tr(cx, "List", "列表"))
                    .tab_index(0)
                    .relative()
                    .w(px(26.))
                    .h(px(24.))
                    .rounded(px(4.))
                    .when(!is_grid, |d| d.child(pro_mark_selected(&fview_sensor)))
                    .when(is_grid, |d| {
                        d.hover({
                            let hv = p.hover;
                            move |s| s.bg(hv)
                        })
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_icon(
                        ProIcon::List,
                        14.,
                        if !is_grid { p.ink } else { p.faint },
                    )),
                cx,
                |this, _, cx| {
                    this.folder_view = FolderViewMode::List;
                    cx.notify();
                },
            ));

        let search_box = self.render_search_box(
            pro_eid(&self.id, "folder-search-box"),
            self.folder_search.clone(),
            p,
            cx,
        );
        let create_btn = pro_btn(
            pro_eid(&self.id, "create-folder"),
            tr(cx, "Create folder", "新建文件夹"),
            Some(ProIcon::Plus),
            ProBtnStyle::Primary,
            true,
            cx,
            |this, w, cx| this.create_folder(w, cx),
        );

        let toolbar = div()
            .mt(px(12.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(tabs_row)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .child(sort_btn)
                    .child(view_toggle)
                    .child(search_box)
                    .child(create_btn),
            );

        // Section body
        let cols = if !is_grid || main_w < 480. {
            1usize
        } else if main_w < 760. {
            2usize
        } else {
            4usize
        };

        let body = if self.loading {
            let mut skel_wrap = div().mt(px(16.)).flex().flex_col().gap(px(11.));
            for chunk in [0usize, 1, 2, 3].chunks(cols) {
                let mut row = div().flex().gap(px(11.));
                for &idx in chunk {
                    row = row.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h(px(if is_grid { 56. } else { 48. }))
                            .px(px(12.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(pro_skeleton_bar(
                                26.,
                                26.,
                                6.,
                                phase + idx as f32 * 0.08,
                                cx,
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(
                                        90.,
                                        11.,
                                        4.,
                                        phase + idx as f32 * 0.08,
                                        cx,
                                    ))
                                    .child(pro_skeleton_bar(
                                        68.,
                                        9.,
                                        4.,
                                        phase + idx as f32 * 0.08,
                                        cx,
                                    )),
                            ),
                    );
                }
                skel_wrap = skel_wrap.child(row);
            }
            skel_wrap.into_any_element()
        } else if self.empty || self.folders.is_empty() {
            div()
                .mt(px(16.))
                .rounded(px(12.))
                .border_1()
                .border_color(p.line)
                .bg(p.card.opacity(0.5))
                .child(pro_empty_state(
                    ProIcon::Folder,
                    tr(cx, "No folders yet", "暂无文件夹"),
                    tr(
                        cx,
                        "Create a folder to organize project files and shared references.",
                        "创建文件夹以整理项目文件与共享资料。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-new-folder"),
                        tr(cx, "New folder", "新建文件夹").into(),
                        Box::new(|this: &mut Self, w, cx| this.create_folder(w, cx)),
                    )),
                    cx,
                ))
                .into_any_element()
        } else if visible.is_empty() {
            div()
                .mt(px(16.))
                .rounded(px(12.))
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Search,
                    tr(cx, "No matching folders", "无匹配文件夹"),
                    tr(
                        cx,
                        "Try clearing the search query or switching visibility tabs.",
                        "可改清空搜索词或切换可见性标签。",
                    ),
                    Some((
                        pro_eid(&self.id, "reset-folder-filter"),
                        tr(cx, "Show all folders", "显示全部文件夹").into(),
                        Box::new(|this: &mut Self, w, cx| {
                            this.folder_tab = FolderTab::All;
                            if let Some(ed) = &this.folder_search {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else if !is_grid {
            self.render_folder_list_table(&visible, main_w, p, cx)
                .into_any_element()
        } else {
            let mut grid = div().mt(px(16.)).flex().flex_col().gap(px(11.));
            for chunk in visible.chunks(cols) {
                let mut row = div().flex().gap(px(11.));
                for folder in chunk {
                    row = row.child(self.render_folder_card(folder, p, cx));
                }
                if chunk.len() < cols {
                    for _ in chunk.len()..cols {
                        row = row.child(div().flex_1().min_w_0());
                    }
                }
                grid = grid.child(row);
            }
            grid.into_any_element()
        };

        div()
            .mt(px(if main_w < 600. { 32. } else { 36. }))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "FOLDERS", "文件夹")),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.faint)
                            .child(count_val.to_string()),
                    ),
            )
            .child(toolbar)
            .child(pro_view_transition(
                pro_eid(&self.id, "folders-body-vtx"),
                format!(
                    "{:?}-{:?}-{}-{}",
                    self.folder_view, self.folder_tab, self.loading, self.empty
                ),
                body,
            ))
    }

    fn render_folder_card(
        &self,
        folder: &FilesFolderItem,
        p: &ProPalette,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let fid = folder.id.clone();
        let fname = folder.name.clone();
        let is_selected = self.selected == Some(SelectedRef::Folder(fid.clone()));
        let is_renaming = self.renaming_id.as_ref() == Some(&fid);
        let file_word = if folder.file_count == 1 {
            tr(cx, "file", "个文件")
        } else {
            tr(cx, "files", "个文件")
        };
        let meta_txt = format!(
            "{} · {} {}",
            folder.access.label(cx),
            folder.file_count,
            file_word
        );
        let rename_ed = self.rename_editor.clone();

        let title_el = if is_renaming && let Some(ed) = rename_ed {
            div()
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
                .text_size(px(13.))
                .line_height(px(20.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .truncate()
                .child(folder.name.clone())
                .into_any_element()
        };

        let border_c = if is_selected { p.focus } else { p.line };
        let hover_border = if is_selected { p.focus } else { p.line_strong };
        let focus_c = p.focus;

        activate(
            div()
                .id(pro_eid(&self.id, format!("fcard-{fid}")))
                .role(Role::Button)
                .aria_label(folder.name.clone())
                .tab_index(0)
                .flex_1()
                .min_w_0()
                .h(px(56.))
                .pl(px(12.))
                .pr(px(14.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(border_c)
                .hover(move |s| s.border_color(hover_border))
                .focus_visible(move |s| s.border_color(focus_c))
                .flex()
                .items_center()
                .gap(px(12.))
                .cursor_pointer()
                .child(pro_svg(
                    "0 0 67 67",
                    FOLDER_MARK_SVG,
                    26.,
                    rgb(0xffffff).into(),
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(title_el)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(pro_icon(folder.access.icon(), 12., p.faint))
                                .child(div().truncate().child(meta_txt)),
                        ),
                ),
            cx,
            move |this, _, cx| {
                this.selected = Some(SelectedRef::Folder(fid.clone()));
                cx.emit(FilesEvent::OpenFolder(fname.clone()));
                cx.emit(FilesEvent::Navigate(fname.clone()));
                cx.notify();
            },
        )
    }

    fn render_folder_list_table(
        &self,
        folders: &[FilesFolderItem],
        main_w: f32,
        p: &ProPalette,
        cx: &Context<Self>,
    ) -> Div {
        let show_secondary = main_w >= 620.;
        let headers = [
            (
                FileSortField::Name,
                tr(cx, "Name", "名称"),
                "col-name",
                true,
            ),
            (
                FileSortField::Modified,
                tr(cx, "Modified", "修改时间"),
                "col-mod",
                show_secondary,
            ),
            (
                FileSortField::Size,
                tr(cx, "Size", "大小"),
                "col-size",
                show_secondary,
            ),
            (
                FileSortField::Owner,
                tr(cx, "Owner", "所有者"),
                "col-owner",
                show_secondary,
            ),
        ];

        let mut head_row = div()
            .h(px(30.))
            .px(px(12.))
            .flex()
            .items_center()
            .gap(px(12.));

        for (field, label, suffix, visible) in headers {
            if !visible {
                continue;
            }
            let active = self.folder_sort_field == field;
            let arrow = if active {
                if self.folder_sort_asc { " ↑" } else { " ↓" }
            } else {
                ""
            };
            let header_btn = activate(
                div()
                    .id(pro_eid(&self.id, suffix))
                    .role(Role::ColumnHeader)
                    .tab_index(0)
                    .when(field == FileSortField::Name, |d| d.flex_1().min_w_0())
                    .when(field != FileSortField::Name, |d| d.w(px(110.)).flex_none())
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if active { p.ink } else { p.muted })
                    .hover({
                        let ink = p.ink;
                        move |s| s.text_color(ink)
                    })
                    .cursor_pointer()
                    .child(format!("{label}{arrow}")),
                cx,
                move |this, _, cx| {
                    if this.folder_sort_field == field {
                        this.folder_sort_asc = !this.folder_sort_asc;
                    } else {
                        this.folder_sort_field = field;
                        this.folder_sort_asc = true;
                    }
                    cx.notify();
                },
            );
            head_row = head_row.child(header_btn);
        }
        head_row = head_row.child(div().w(px(60.)).flex_none());

        let mut list = div()
            .mt(px(12.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(head_row);

        for folder in folders {
            let fid = folder.id.clone();
            let fname = folder.name.clone();
            let is_selected = self.selected == Some(SelectedRef::Folder(fid.clone()));
            let is_renaming = self.renaming_id.as_ref() == Some(&fid);
            let rename_ed = self.rename_editor.clone();
            let file_word = if folder.file_count == 1 {
                tr(cx, "file", "个文件")
            } else {
                tr(cx, "files", "个文件")
            };
            let sub_meta = format!(
                "{} · {} {}",
                folder.access.label(cx),
                folder.file_count,
                file_word
            );
            let fid_ql = fid.clone();
            let fid_rn = fid.clone();
            let fname_rn = fname.clone();

            let name_cell = if is_renaming && let Some(ed) = rename_ed {
                div()
                    .h(px(24.))
                    .px(px(6.))
                    .rounded(px(5.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.focus)
                    .flex()
                    .items_center()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(div().w(px(180.)).child(ed))
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(folder.name.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(pro_icon(folder.access.icon(), 11., p.faint))
                            .child(sub_meta),
                    )
                    .into_any_element()
            };

            let border_c = if is_selected { p.focus } else { p.line };
            let hover_border = if is_selected { p.focus } else { p.line_strong };

            list = list.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("frow-{fid}")))
                    .role(Role::Row)
                    .tab_index(0)
                    .h(px(48.))
                    .px(px(12.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(border_c)
                    .hover(move |s| s.border_color(hover_border))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .cursor_pointer()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(pro_svg(
                                "0 0 67 67",
                                FOLDER_MARK_SVG,
                                24.,
                                rgb(0xffffff).into(),
                            ))
                            .child(name_cell),
                    )
                    .when(show_secondary, |d| {
                        d.child(
                            div()
                                .w(px(110.))
                                .flex_none()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .truncate()
                                .child(folder.modified.clone()),
                        )
                        .child(
                            div()
                                .w(px(110.))
                                .flex_none()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(format_size_kb(folder.size_kb)),
                        )
                        .child(
                            div()
                                .w(px(110.))
                                .flex_none()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .truncate()
                                .child(folder.owner.clone()),
                        )
                    })
                    .child(
                        div()
                            .w(px(60.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(2.))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(pro_icon_btn(
                                pro_eid(&self.id, format!("frow-ql-{fid_ql}")),
                                tr(cx, "Quick Look", "快速预览"),
                                ProIcon::Eye,
                                26.,
                                false,
                                cx,
                                move |this, _, cx| {
                                    let r = SelectedRef::Folder(fid_ql.clone());
                                    this.selected = Some(r.clone());
                                    this.quick_look = Some(r);
                                    this.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                                    cx.notify();
                                },
                            ))
                            .child(pro_icon_btn(
                                pro_eid(&self.id, format!("frow-rn-{fid_rn}")),
                                tr(cx, "Rename", "重命名"),
                                ProIcon::Note,
                                26.,
                                false,
                                cx,
                                move |this, w, cx| {
                                    this.start_rename(fid_rn.clone(), fname_rn.as_ref(), w, cx);
                                },
                            )),
                    ),
                cx,
                move |this, _, cx| {
                    this.selected = Some(SelectedRef::Folder(fid.clone()));
                    cx.emit(FilesEvent::OpenFolder(fname.clone()));
                    cx.emit(FilesEvent::Navigate(fname.clone()));
                    cx.notify();
                },
            ));
        }
        list
    }

    fn render_templates_section(
        &mut self,
        main_w: f32,
        phase: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let visible = self.visible_templates(cx);
        let count_val = if self.loading {
            self.templates.len()
        } else {
            visible.len()
        };

        // Left tabs: All | Mine | Shared with me
        let owner_tabs = [
            (TemplateOwnerTab::All, tr(cx, "All", "全部"), "ttab-all"),
            (TemplateOwnerTab::Mine, tr(cx, "Mine", "我的"), "ttab-mine"),
            (
                TemplateOwnerTab::SharedWithMe,
                tr(cx, "Shared with me", "与我共享"),
                "ttab-shared",
            ),
        ];
        let ttab_sensor = pro_selection_sensor();
        let ttab_key = match self.template_tab {
            TemplateOwnerTab::All => "ttab-all",
            TemplateOwnerTab::Mine => "ttab-mine",
            TemplateOwnerTab::SharedWithMe => "ttab-shared",
        };
        let mut tabs_row =
            div()
                .relative()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "ttab-pill"),
                    ttab_key,
                    ttab_sensor.clone(),
                    6.,
                    p.select,
                    Some(p.line_strong),
                    false,
                ));
        for (tab_val, label, suffix) in owner_tabs {
            let active = self.template_tab == tab_val;
            let ink = p.ink;
            let txt = p.text;
            let ln = p.line;
            let fc = p.focus;
            tabs_row = tabs_row.child(activate(
                div()
                    .id(pro_eid(&self.id, suffix))
                    .role(Role::Tab)
                    .aria_label(label)
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if active { transparent_black() } else { ln })
                    .when(active, |d| d.child(pro_mark_selected(&ttab_sensor)))
                    .text_size(px(13.))
                    .text_color(if active { ink } else { txt })
                    .when(!active, |d| d.hover(move |s| s.text_color(ink)))
                    .focus_visible(move |s| s.border_color(fc))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .child(label),
                cx,
                move |this, _, cx| {
                    this.template_tab = tab_val;
                    cx.notify();
                },
            ));
        }

        // Right toolbar: Type filter | Sort | Search templates... | + Add template
        let filter_active = self.template_kind_filter.is_some();
        let type_btn_label = if let Some(k) = self.template_kind_filter {
            format!("{}: {}", tr(cx, "Type", "类型"), k.badge())
        } else {
            tr(cx, "Type", "类型").to_string()
        };

        let mut type_anchor = div().relative().child(activate(
            div()
                .id(pro_eid(&self.id, "type-filter-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "Filter by type", "按类型筛选"))
                .tab_index(0)
                .h(px(28.))
                .px(px(10.))
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(if filter_active { p.line_strong } else { p.line })
                .hover({
                    let ls = p.line_strong;
                    move |s| s.border_color(ls)
                })
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(13.))
                .text_color(if filter_active { p.ink } else { p.text })
                .cursor_pointer()
                .child(pro_svg("0 0 24 24", FUNNEL_SVG, 14., p.muted))
                .child(type_btn_label)
                .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
            cx,
            |this, _, cx| {
                this.type_menu_open = !this.type_menu_open;
                cx.notify();
            },
        ));

        {
            let options: [(Option<FileDocKind>, &str, &str); 6] = [
                (None, tr(cx, "All types", "全部类型"), "topt-all"),
                (
                    Some(FileDocKind::Document),
                    tr(cx, "Document (DOC)", "文档 (DOC)"),
                    "topt-doc",
                ),
                (
                    Some(FileDocKind::Spreadsheet),
                    tr(cx, "Spreadsheet (XLS)", "表格 (XLS)"),
                    "topt-xls",
                ),
                (Some(FileDocKind::Pdf), "PDF", "topt-pdf"),
                (
                    Some(FileDocKind::Image),
                    tr(cx, "Image (IMG)", "图片 (IMG)"),
                    "topt-img",
                ),
                (Some(FileDocKind::Code), tr(cx, "Code", "代码"), "topt-code"),
            ];
            let type_sensor = pro_selection_sensor();
            let type_key = match self.template_kind_filter {
                None => "topt-all",
                Some(FileDocKind::Document) => "topt-doc",
                Some(FileDocKind::Spreadsheet) => "topt-xls",
                Some(FileDocKind::Pdf) => "topt-pdf",
                Some(FileDocKind::Image) => "topt-img",
                Some(FileDocKind::Code) => "topt-code",
            };
            let mut menu = div()
                .id(pro_eid(&self.id, "type-menu"))
                .role(Role::Menu)
                .occlude()
                .absolute()
                .left_0()
                .w(px(184.))
                .p(px(4.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(1.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "type-menu-pill"),
                    type_key,
                    type_sensor.clone(),
                    8.,
                    p.select,
                    None,
                    false,
                ));

            for (opt_kind, label, suffix) in options {
                let checked = self.template_kind_filter == opt_kind;
                let hv = p.hover;
                let ink = p.ink;
                let txt = p.text;
                menu = menu.child(activate(
                    div()
                        .id(pro_eid(&self.id, suffix))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .relative()
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .when(checked, |d| d.child(pro_mark_selected(&type_sensor)))
                        .when(!checked, |d| d.hover(move |s| s.bg(hv).text_color(ink)))
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(13.))
                        .font_weight(if checked {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if checked { ink } else { txt })
                        .cursor_pointer()
                        .child(label)
                        .when(checked, |d| d.child(pro_icon(ProIcon::Check, 13., ink))),
                    cx,
                    move |this, _, cx| {
                        this.template_kind_filter = opt_kind;
                        this.type_menu_open = false;
                        cx.notify();
                    },
                ));
            }
            type_anchor = type_anchor.child(pro_presence(
                pro_eid(&self.id, "type-menu-pres"),
                self.type_menu_open,
                0.18,
                0.12,
                move |a| {
                    deferred(menu.top(px(32. - 4. * (1. - a))).opacity(a))
                        .with_priority(3)
                        .into_any_element()
                },
            ));
        }

        let tpl_sort_svg = if self.template_sort_asc {
            SORT_ASC_SVG
        } else {
            SORT_DESC_SVG
        };
        let tpl_sort_btn = activate(
            div()
                .id(pro_eid(&self.id, "tpl-sort"))
                .role(Role::Button)
                .aria_label(tr(cx, "Sort templates", "排序模板"))
                .tab_index(0)
                .size(px(28.))
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .hover({
                    let ls = p.line_strong;
                    move |s| s.border_color(ls)
                })
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .child(pro_svg("0 0 24 24", tpl_sort_svg, 15., p.muted)),
            cx,
            |this, _, cx| {
                this.template_sort_asc = !this.template_sort_asc;
                cx.notify();
            },
        );

        let search_box = self.render_search_box(
            pro_eid(&self.id, "tpl-search-box"),
            self.template_search.clone(),
            p,
            cx,
        );
        let add_tpl_btn = pro_btn(
            pro_eid(&self.id, "add-template"),
            tr(cx, "Add template", "添加模板"),
            Some(ProIcon::Plus),
            ProBtnStyle::Secondary,
            true,
            cx,
            |this, w, cx| this.add_template(w, cx),
        );

        let toolbar = div()
            .mt(px(12.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(tabs_row)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .child(type_anchor)
                    .child(tpl_sort_btn)
                    .child(search_box)
                    .child(add_tpl_btn),
            );

        let cols = if main_w < 460. {
            1usize
        } else if main_w < 760. {
            2usize
        } else {
            4usize
        };
        let preview_h = if main_w < 600. { 128. } else { 152. };

        let body = if self.loading {
            let mut skel_grid = div().mt(px(16.)).flex().flex_col().gap(px(16.));
            for chunk in [0usize, 1, 2, 3].chunks(cols) {
                let mut row = div().flex().gap(px(12.));
                for &idx in chunk {
                    row = row.child(
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
                                                phase + idx as f32 * 0.1,
                                                cx,
                                            )),
                                    ),
                            )
                            .child(div().mt(px(10.)).child(pro_skeleton_bar(
                                116.,
                                12.,
                                4.,
                                phase + idx as f32 * 0.1,
                                cx,
                            )))
                            .child(div().mt(px(6.)).child(pro_skeleton_bar(
                                148.,
                                10.,
                                4.,
                                phase + idx as f32 * 0.1,
                                cx,
                            ))),
                    );
                }
                skel_grid = skel_grid.child(row);
            }
            skel_grid.into_any_element()
        } else if self.empty || self.templates.is_empty() {
            div()
                .mt(px(16.))
                .rounded(px(12.))
                .border_1()
                .border_color(p.line)
                .bg(p.card.opacity(0.5))
                .child(pro_empty_state(
                    ProIcon::PageDoc,
                    tr(cx, "No templates yet", "暂无模板"),
                    tr(
                        cx,
                        "Save reusable briefs, budgets, and reports for your workspace.",
                        "保存可复用的简报、预算表与报告模板。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-add-template"),
                        tr(cx, "Add template", "添加模板").into(),
                        Box::new(|this: &mut Self, w, cx| this.add_template(w, cx)),
                    )),
                    cx,
                ))
                .into_any_element()
        } else if visible.is_empty() {
            div()
                .mt(px(16.))
                .rounded(px(12.))
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Filter,
                    tr(cx, "No matching templates", "无匹配模板"),
                    tr(
                        cx,
                        "Reset the type or ownership filter to see all workspace templates.",
                        "重置类型或归属筛选以查看全部工作区模板。",
                    ),
                    Some((
                        pro_eid(&self.id, "reset-tpl-filter"),
                        tr(cx, "Reset filters", "重置筛选").into(),
                        Box::new(|this: &mut Self, w, cx| {
                            this.template_tab = TemplateOwnerTab::All;
                            this.template_kind_filter = None;
                            if let Some(ed) = &this.template_search {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else {
            let mut grid = div().mt(px(16.)).flex().flex_col().gap(px(16.));
            for chunk in visible.chunks(cols) {
                let mut row = div().flex().gap(px(12.));
                for tpl in chunk {
                    row = row.child(self.render_template_card(tpl, preview_h, p, cx));
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

        div()
            .mt(px(if main_w < 600. { 32. } else { 36. }))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "FILE TEMPLATES", "文件模板")),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.faint)
                            .child(count_val.to_string()),
                    ),
            )
            .child(toolbar)
            .child(pro_view_transition(
                pro_eid(&self.id, "tpls-body-vtx"),
                format!(
                    "{:?}-{:?}-{}-{}",
                    self.template_tab, self.template_kind_filter, self.loading, self.empty
                ),
                body,
            ))
    }

    fn render_template_card(
        &self,
        tpl: &FilesTemplateItem,
        preview_h: f32,
        p: &ProPalette,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let tid = tpl.id.clone();
        let tname = tpl.name.clone();
        let is_selected = self.selected == Some(SelectedRef::Template(tid.clone()))
            || self.quick_look == Some(SelectedRef::Template(tid.clone()));
        let is_renaming = self.renaming_id.as_ref() == Some(&tid);
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
                .child(tpl.name.clone())
                .into_any_element()
        };

        let left_meta = format!("{} · {}", tpl.kind.label(cx), tpl.owner);

        activate(
            div()
                .id(pro_eid(&self.id, format!("tcard-{tid}")))
                .role(Role::Button)
                .aria_label(tpl.name.clone())
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
                                .child(self.render_doc_artwork(tpl.kind, p)),
                        ),
                )
                .child(name_el)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .min_w_0()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .child(div().min_w_0().truncate().child(left_meta))
                        .child(div().flex_none().text_color(p.faint).child("·"))
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(pro_icon(tpl.access.icon(), 12., p.faint))
                                .child(tpl.access.label(cx)),
                        ),
                ),
            cx,
            move |this, _, cx| {
                let target = SelectedRef::Template(tid.clone());
                this.selected = Some(target.clone());
                this.quick_look = Some(target);
                this.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                cx.emit(FilesEvent::SelectTemplate(tname.clone()));
                cx.notify();
            },
        )
    }

    fn render_inspector(
        &mut self,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let ql = self.quick_look.clone()?;
        let p = ProPalette::get(cx);

        match &ql {
            SelectedRef::Folder(fid) => {
                let folder = self.folders.iter().find(|f| &f.id == fid)?.clone();
                let fid_open = folder.id.clone();
                let fname_open = folder.name.clone();
                let fid_rn = folder.id.clone();
                let fname_rn = folder.name.clone();
                let target_acc = ql.clone();
                let target_del = ql.clone();

                let sample_files: &[(&str, FileDocKind)] = match folder.name.as_ref() {
                    "Q3 Campaign" => &[
                        ("Campaign-Brief", FileDocKind::Document),
                        ("Launch-Plan", FileDocKind::Pdf),
                        ("Media-Budget", FileDocKind::Spreadsheet),
                        ("Ad-Copy", FileDocKind::Document),
                        ("Results-Q2", FileDocKind::Spreadsheet),
                    ],
                    "Brand assets" => &[
                        ("Logo-System", FileDocKind::Pdf),
                        ("Color-Tokens", FileDocKind::Document),
                        ("Type-Specimens", FileDocKind::Pdf),
                    ],
                    "Hiring" => &[
                        ("Role-Scorecard", FileDocKind::Document),
                        ("Headcount-Plan", FileDocKind::Spreadsheet),
                        ("Interview-Loop", FileDocKind::Document),
                        ("Offer-Bands", FileDocKind::Spreadsheet),
                    ],
                    _ => &[
                        ("User-Interviews", FileDocKind::Document),
                        ("Survey-Synthesis", FileDocKind::Spreadsheet),
                    ],
                };

                let body = div()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .p(px(14.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(pro_svg(
                                "0 0 67 67",
                                FOLDER_MARK_SVG,
                                34.,
                                rgb(0xffffff).into(),
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(folder.name.clone()),
                                    )
                                    .child(div().text_size(px(12.)).text_color(p.muted).child(
                                        format!(
                                            "{} · {} · {}",
                                            folder.access.label(cx),
                                            format_size_kb(folder.size_kb),
                                            folder.modified
                                        ),
                                    )),
                            ),
                    )
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
                                                folder.owner.as_ref(),
                                                18.,
                                                false,
                                                None,
                                                cx,
                                            ))
                                            .child(folder.owner.clone()),
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
                                        "Access",
                                        "访问权限",
                                    )))
                                    .child(pro_badge(
                                        folder.access.label(cx),
                                        if folder.access == FileAccess::Shared {
                                            ProTone::Blue
                                        } else {
                                            ProTone::Neutral
                                        },
                                        false,
                                        cx,
                                    )),
                            )
                            .child({
                                let total_f_kb =
                                    self.folders.iter().map(|f| f.size_kb).sum::<u32>().max(1);
                                let share_frac =
                                    (folder.size_kb as f32 / total_f_kb as f32).clamp(0.04, 1.0);
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
                                                "Storage share",
                                                "存储占比",
                                            )))
                                            .child(
                                                div()
                                                    .text_color(p.ink)
                                                    .child(format_size_kb(folder.size_kb)),
                                            ),
                                    )
                                    .child(pro_spring_bar(
                                        pro_eid(&self.id, format!("ql-fquota-{}", folder.id)),
                                        share_frac,
                                        4.,
                                        2.,
                                        p.chip,
                                        p.blue,
                                    ))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "FILES IN FOLDER", "文件夹内容")),
                            )
                            .children(sample_files.iter().take(folder.file_count.max(1)).map(
                                |(n, k)| {
                                    div()
                                        .h(px(32.))
                                        .px(px(10.))
                                        .rounded(px(8.))
                                        .bg(p.card)
                                        .border_1()
                                        .border_color(p.line_soft)
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div().text_size(px(12.5)).text_color(p.ink).child(*n),
                                        )
                                        .child(
                                            div()
                                                .px(px(5.))
                                                .py(px(1.5))
                                                .rounded(px(4.))
                                                .bg(k.tag_color(&p).opacity(0.16))
                                                .text_size(px(10.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(k.tag_color(&p))
                                                .child(k.badge()),
                                        )
                                },
                            )),
                    )
                    .child(
                        div()
                            .pt(px(4.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_btn(
                                pro_eid(&self.id, format!("ql-open-folder-{fid_open}")),
                                tr(cx, "Open folder", "打开文件夹"),
                                Some(ProIcon::Folder),
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                move |_, _, cx| {
                                    cx.emit(FilesEvent::OpenFolder(fname_open.clone()));
                                    cx.emit(FilesEvent::Navigate(fname_open.clone()));
                                },
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, format!("ql-rn-folder-{fid_rn}")),
                                tr(cx, "Rename", "重命名"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                move |this, w, cx| {
                                    this.start_rename(fid_rn.clone(), fname_rn.as_ref(), w, cx);
                                },
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "ql-toggle-folder-acc"),
                                tr(cx, "Toggle access", "切换权限"),
                                None,
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                move |this, _, cx| this.toggle_access(&target_acc, cx),
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "ql-del-folder"),
                                tr(cx, "Delete", "删除"),
                                Some(ProIcon::Trash),
                                ProBtnStyle::Danger,
                                true,
                                cx,
                                move |this, _, cx| this.delete_item(&target_del, cx),
                            )),
                    );

                Some((
                    folder.name,
                    ProIcon::Folder,
                    pro_view_transition(
                        pro_eid(&self.id, "files-insp-vtx"),
                        format!("folder-{fid}"),
                        body,
                    ),
                ))
            }
            SelectedRef::Template(tid) => {
                let tpl = self.templates.iter().find(|t| &t.id == tid)?.clone();
                let tid_rn = tpl.id.clone();
                let tname_rn = tpl.name.clone();
                let target_acc = ql.clone();
                let target_del = ql.clone();

                let content_preview = if tpl.kind == FileDocKind::Spreadsheet {
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
                                .child(div().flex_1().child("Line item"))
                                .child(div().w(px(68.)).child("Planned"))
                                .child(div().w(px(68.)).child("Actual")),
                        )
                        .children(
                            [
                                ("Paid acquisition", "$18,000", "$17,400"),
                                ("Partner launch", "$24,500", "$24,500"),
                                ("Content studio", "$12,000", "$11,250"),
                            ]
                            .into_iter()
                            .map(|(item, plan, act)| {
                                div()
                                    .h(px(28.))
                                    .px(px(8.))
                                    .border_t_1()
                                    .border_color(p.line_soft)
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.))
                                    .child(div().flex_1().text_color(p.ink).truncate().child(item))
                                    .child(div().w(px(68.)).text_color(p.text).child(plan))
                                    .child(div().w(px(68.)).text_color(p.text).child(act))
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
                                .child(tpl.summary.clone()),
                        )
                        .child(
                            div()
                                .pt(px(4.))
                                .border_t_1()
                                .border_color(p.line_soft)
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .children(
                                    [
                                        tr(
                                            cx,
                                            "Context variables & owner placeholders",
                                            "上下文变量与负责人占位符",
                                        ),
                                        tr(
                                            cx,
                                            "Ready to attach to any project folder",
                                            "可直接挂载至任意项目文件夹",
                                        ),
                                    ]
                                    .into_iter()
                                    .map(|line| {
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .child(pro_icon(ProIcon::Check, 12., p.green))
                                            .child(line)
                                    }),
                                ),
                        )
                        .into_any_element()
                };

                let body = div()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .h(px(136.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(self.render_doc_artwork(tpl.kind, &p)),
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
                                    .child(tpl.name.clone()),
                            )
                            .child(div().text_size(px(12.)).text_color(p.muted).child(format!(
                                "{} · {} · {}",
                                tpl.kind.label(cx),
                                format_size_kb(tpl.size_kb),
                                tpl.modified
                            ))),
                    )
                    .child(content_preview)
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
                                                tpl.owner.as_ref(),
                                                18.,
                                                false,
                                                None,
                                                cx,
                                            ))
                                            .child(tpl.owner.clone()),
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
                                        "Visibility",
                                        "可见性",
                                    )))
                                    .child(pro_badge(
                                        tpl.access.label(cx),
                                        if tpl.access == FileAccess::Shared {
                                            ProTone::Blue
                                        } else {
                                            ProTone::Neutral
                                        },
                                        false,
                                        cx,
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
                                pro_eid(&self.id, format!("ql-rn-tpl-{tid_rn}")),
                                tr(cx, "Rename", "重命名"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                move |this, w, cx| {
                                    this.start_rename(tid_rn.clone(), tname_rn.as_ref(), w, cx);
                                },
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "ql-toggle-tpl-acc"),
                                tr(cx, "Toggle access", "切换权限"),
                                None,
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                move |this, _, cx| this.toggle_access(&target_acc, cx),
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "ql-del-tpl"),
                                tr(cx, "Delete", "删除"),
                                Some(ProIcon::Trash),
                                ProBtnStyle::Danger,
                                true,
                                cx,
                                move |this, _, cx| this.delete_item(&target_del, cx),
                            )),
                    );

                Some((
                    tpl.name,
                    ProIcon::PageDoc,
                    pro_view_transition(
                        pro_eid(&self.id, "files-insp-vtx"),
                        format!("tpl-{tid}"),
                        body,
                    ),
                ))
            }
        }
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
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
        let pad_x = if main_w < 600. {
            16.
        } else if main_w < 900. {
            24.
        } else {
            32.
        };
        let pad_top = if main_w < 600. { 52. } else { 46. };
        let pad_bot = if main_w < 600. { 40. } else { 56. };

        let used_kb: u32 = if self.empty {
            0
        } else {
            self.folders.iter().map(|f| f.size_kb).sum::<u32>()
                + self.templates.iter().map(|t| t.size_kb).sum::<u32>()
        };
        let quota_kb: u32 = 25_600;
        let quota_ratio = (used_kb as f32 / quota_kb as f32).clamp(0.02, 1.0);
        let quota_strip = div()
            .mt(px(12.))
            .max_w(px(340.))
            .flex()
            .flex_col()
            .gap(px(5.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .text_size(px(11.5))
                    .child(div().text_color(p.muted).child(tr(
                        cx,
                        "Workspace storage",
                        "工作区存储配额",
                    )))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(format!(
                                "{} / {}",
                                format_size_kb(used_kb),
                                format_size_kb(quota_kb)
                            )),
                    ),
            )
            .child(pro_spring_bar(
                pro_eid(&self.id, "ws-quota-bar"),
                quota_ratio,
                5.,
                2.5,
                p.chip,
                p.blue,
            ));

        let learn_more_card = div()
            .mt(px(12.))
            .p(px(12.))
            .rounded(px(10.))
            .bg(p.chip)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .text_size(px(12.5))
            .text_color(p.text)
            .child(tr(
                cx,
                "Shared folders sync automatically across workspace members; templates clone cleanly into new threads.",
                "共享文件夹会自动向工作区成员同步；文件模板可一键复制至新对话。",
            ))
            .child(pro_icon_btn(
                pro_eid(&self.id, "close-learn-more"),
                tr(cx, "Dismiss", "关闭"),
                ProIcon::Close,
                24.,
                false,
                cx,
                |this, _, cx| {
                    this.learn_more_open = false;
                    cx.notify();
                },
            ));
        let learn_more_fold = pro_fold(
            pro_eid(&self.id, "learn-more-fold"),
            self.learn_more_open,
            54.,
            0.,
            move || learn_more_card.into_any_element(),
        );

        let folders_el = self.render_folder_section(main_w, phase, &p, cx);
        let templates_el = self.render_templates_section(main_w, phase, &p, cx);

        div()
            .id(pro_eid(&self.id, "page-root"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.type_menu_open {
                        this.type_menu_open = false;
                        cx.notify();
                    }
                    if this.renaming_id.is_some() {
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
            .child(
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pt(px(pad_top))
                    .px(px(pad_x))
                    .pb(px(pad_bot))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(if main_w < 600. { 20. } else { 24. }))
                            .line_height(px(if main_w < 600. { 26. } else { 32. }))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "Files", "文件")),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.text)
                            .child(tr(
                                cx,
                                "See files you recently used in chats and manage files saved in folders.",
                                "查看近期对话中使用过的文件，并管理保存在文件夹中的资料。",
                            ))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "learn-more"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .border_b_1()
                                    .border_color(p.line_strong)
                                    .hover({
                                        let ink = p.ink;
                                        move |s| s.text_color(ink).border_color(ink)
                                    })
                                    .cursor_pointer()
                                    .child(tr(cx, "Learn more", "了解更多")),
                                cx,
                                |this, _, cx| {
                                    this.learn_more_open = !this.learn_more_open;
                                    cx.notify();
                                },
                            )),
                    )
                    .child(quota_strip)
                    .child(learn_more_fold)
                    .child(folders_el)
                    .child(templates_el),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

impl Render for FilesScreen {
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
        FileAccess, FileDocKind, FileSortField, FolderTab, TemplateOwnerTab, default_folders,
        default_templates, filter_and_sort_folders, filter_and_sort_templates,
    };

    #[test]
    fn folder_filtering_and_column_sorting_work() {
        let folders = default_folders();
        assert_eq!(folders.len(), 4);

        let shared =
            filter_and_sort_folders(&folders, FolderTab::Shared, "", FileSortField::Name, true);
        assert_eq!(shared.len(), 2);
        assert_eq!(shared[0].name.as_ref(), "Brand assets");
        assert_eq!(shared[1].name.as_ref(), "Q3 Campaign");

        let by_size_desc =
            filter_and_sort_folders(&folders, FolderTab::All, "", FileSortField::Size, false);
        assert_eq!(by_size_desc[0].name.as_ref(), "Brand assets");
        assert_eq!(by_size_desc[3].name.as_ref(), "Research");

        let searched = filter_and_sort_folders(
            &folders,
            FolderTab::Private,
            "hir",
            FileSortField::Name,
            true,
        );
        assert_eq!(searched.len(), 1);
        assert_eq!(searched[0].access, FileAccess::Private);
        assert_eq!(searched[0].file_count, 4);
    }

    #[test]
    fn template_ownership_and_type_filters_match_spec() {
        let templates = default_templates();
        assert_eq!(templates.len(), 4);

        let mine = filter_and_sort_templates(
            &templates,
            TemplateOwnerTab::Mine,
            "Alex Morgan",
            None,
            "",
            true,
        );
        assert_eq!(mine.len(), 2);
        assert_eq!(mine[0].name.as_ref(), "Design-Book");
        assert_eq!(mine[1].name.as_ref(), "Weekly-Report");

        let shared_sheets = filter_and_sort_templates(
            &templates,
            TemplateOwnerTab::SharedWithMe,
            "Alex Morgan",
            Some(FileDocKind::Spreadsheet),
            "",
            true,
        );
        assert_eq!(shared_sheets.len(), 1);
        assert_eq!(shared_sheets[0].name.as_ref(), "Campaign-Budget");
    }
}
