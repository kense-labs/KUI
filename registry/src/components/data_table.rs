//! Spreadsheet-grade data table with cell-range selection, in-place editing, TSV copy/paste, undo/redo, and sorting.
//!
//! Combines rectangular pointer-drag and `Shift+Arrow` cell selection, inline [`Editor`] and tag-picker editing,
//! Excel/Sheets-compatible TSV clipboard copy and paste, multi-step undo/redo history, row checkboxes, and column sorting.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_eid, pro_empty_state, pro_skeleton_bar, pro_toast_banner, tr,
    },
    primitives::{
        clipboard,
        editor::{EditEvent, Editor, InputMode},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_raw_svg, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::collections::{BTreeMap, BTreeSet};
use web_time::Instant;

const TITLE_BADGE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" viewBox="0 0 20 20" fill="none"><rect x="2" y="2" width="16" height="16" rx="4.5" fill="#0060e2"/><path d="M6 7.5h8M6 10h8M6 12.5h5" stroke="#ffffff" stroke-width="1.4" stroke-linecap="round"/></svg>"##;
const UNDO_SVG: &str = r#"<path d="M5.5 3.5 2.5 6.5l3 3M2.5 6.5h7a3.5 3.5 0 0 1 0 7H7" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const REDO_SVG: &str = r#"<path d="m10.5 3.5 3 3-3 3M13.5 6.5h-7a3.5 3.5 0 0 0 0 7H9" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const CHECK_TINY_SVG: &str = r#"<path d="m3.5 8.25 2.75 2.75L12.5 4.75" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const MINUS_TINY_SVG: &str =
    r#"<path d="M4 8h8" stroke="currentColor" stroke-width="2" stroke-linecap="round"></path>"#;
const COL_TEXT_SVG: &str = r#"<path d="M3 4h10M3 7h10M3 10h10M3 13h6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"></path>"#;
const COL_TAG_SVG: &str = r#"<path d="M3 3h4v4H3zM9 3h4v4H9zM3 9h4v4H3zM9 9h4v4H9z" stroke="currentColor" stroke-width="1.25" stroke-linejoin="round"></path>"#;
const COL_LINK_SVG: &str = r#"<path d="M6.5 9.5l3-3M7.25 4.75l1-1a2.5 2.5 0 0 1 3.5 3.5l-1 1M8.75 11.25l-1 1a2.5 2.5 0 0 1-3.5-3.5l1-1" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"></path>"#;
const COL_CALENDAR_SVG: &str = r#"<path d="M3 4.5h10v9H3zM3 7.5h10M6 2.5v3M10 2.5v3" stroke="currentColor" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const COL_NUMBER_SVG: &str = r#"<path d="M5.5 2.8 4.3 13.2M11.7 2.8l-1.2 10.4M2.8 6.2h10.4M2.3 9.8h10.4" stroke="currentColor" stroke-width="1.25" stroke-linecap="round"></path>"#;
const SORT_DOWN_SVG: &str = r#"<path d="M8 3.5v9M5 9.5l3 3 3-3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const SORT_UP_SVG: &str = r#"<path d="M8 12.5v-9M5 6.5l3-3 3 3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const PLUS_ROW_SVG: &str = r#"<path d="M8 3.5v9M3.5 8h9" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"></path>"#;

const FIGMA_LOGO_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 38 57" fill="none"><path d="M19 28.5a9.5 9.5 0 1 1 19 0 9.5 9.5 0 0 1-19 0Z" fill="#1ABCFE"/><path d="M0 47.5A9.5 9.5 0 0 1 9.5 38H19v9.5a9.5 9.5 0 1 1-19 0Z" fill="#0ACF83"/><path d="M19 0v19h9.5a9.5 9.5 0 1 0 0-19H19Z" fill="#FF7262"/><path d="M0 9.5A9.5 9.5 0 0 0 9.5 19H19V0H9.5A9.5 9.5 0 0 0 0 9.5Z" fill="#F24E1E"/><path d="M0 28.5A9.5 9.5 0 0 0 9.5 38H19V19H9.5A9.5 9.5 0 0 0 0 28.5Z" fill="#A259FF"/></svg>"##;
const LINEAR_LOGO_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="#5E6AD2"><path d="M2.886 4.18A11.982 11.982 0 0 1 11.99 0C18.624 0 24 5.376 24 12.009c0 3.64-1.62 6.903-4.18 9.105L2.887 4.18ZM1.817 5.626l16.556 16.556c-.524.33-1.075.62-1.65.866L.951 7.277c.247-.575.537-1.126.866-1.65ZM.322 9.163l14.515 14.515c-.71.172-1.443.282-2.195.322L0 11.358a12 12 0 0 1 .322-2.195Zm-.17 4.862 9.823 9.824a12.02 12.02 0 0 1-9.824-9.824Z"/></svg>"##;
const VERCEL_INNER_SVG: &str =
    r#"<path d="M24 22.525H0l12-21.05 12 21.05z" fill="currentColor"></path>"#;
const FRAMER_INNER_SVG: &str =
    r#"<path d="M4 0h16v8h-8zM4 8h8l8 8H4zM4 16h8v8z" fill="currentColor"></path>"#;
const LOOM_LOGO_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="#625DF5"><path d="M24 10.665h-7.018l6.078-3.509-1.335-2.312-6.078 3.509 3.508-6.077L16.843.94l-3.508 6.077V0h-2.67v7.018L7.156.94 4.844 2.275l3.509 6.077-6.078-3.508L.94 7.156l6.078 3.509H0v2.67h7.017L.94 16.844l1.335 2.313 6.077-3.508-3.509 6.077 2.312 1.335 3.509-6.078V24h2.67v-7.017l3.508 6.077 2.312-1.335-3.509-6.078 6.078 3.509 1.335-2.313-6.077-3.508h7.017v-2.67H24zm-12 4.966a3.645 3.645 0 1 1 0-7.29 3.645 3.645 0 0 1 0 7.29z"/></svg>"##;

const MAX_UNDO_STEPS: usize = 64;

/// Cell value presentation and editing behavior for a [`DataColumn`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DataColumnType {
    /// Primary record name column with optional leading row logo.
    Title,
    /// Colored badge pill column with optional dropdown option picker.
    Tag,
    /// Underlined domain or URL link column.
    Link,
    /// Tabular numeric column formatted with thousands separators and real minus signs (`−`).
    Number,
    /// Standard single-line text or date column.
    #[default]
    Text,
}

/// Semantic badge color tone for [`DataColumnType::Tag`] values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DataTagTone {
    Blue,
    Violet,
    Pink,
    Green,
    Amber,
    #[default]
    Gray,
}

impl DataTagTone {
    fn colors(self, dark: bool) -> (Hsla, Hsla) {
        match (self, dark) {
            (Self::Blue, false) => (hsla(215. / 360., 1.0, 0.44, 0.08), rgb(0x0190fe).into()),
            (Self::Blue, true) => (hsla(218. / 360., 1.0, 0.65, 0.16), rgb(0x8bcbfe).into()),
            (Self::Violet, false) => (hsla(272. / 360., 0.85, 0.63, 0.10), rgb(0x8a35d6).into()),
            (Self::Violet, true) => (hsla(268. / 360., 0.78, 0.74, 0.18), rgb(0xb98af0).into()),
            (Self::Pink, false) => (hsla(327. / 360., 0.84, 0.59, 0.09), rgb(0xc21f78).into()),
            (Self::Pink, true) => (hsla(327. / 360., 0.81, 0.68, 0.16), rgb(0xf06db5).into()),
            (Self::Green, false) => (hsla(145. / 360., 0.49, 0.58, 0.11), rgb(0x1e8a4f).into()),
            (Self::Green, true) => (hsla(145. / 360., 0.46, 0.65, 0.15), rgb(0x7dcf9f).into()),
            (Self::Amber, false) => (hsla(45. / 360., 1.0, 0.45, 0.12), rgb(0x8f6c00).into()),
            (Self::Amber, true) => (hsla(46. / 360., 0.75, 0.60, 0.15), rgb(0xe6c24d).into()),
            (Self::Gray, false) => (hsla(0., 0., 0., 0.05), hsla(0., 0., 0., 0.61)),
            (Self::Gray, true) => (hsla(0., 0., 1., 0.08), hsla(0., 0., 1., 0.62)),
        }
    }
}

/// Leading header glyph displayed beside a [`DataColumn`] label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DataColumnIcon {
    /// Omit the leading column icon.
    None,
    /// Horizontal text lines icon.
    #[default]
    Text,
    /// 2x2 category grid icon.
    Tag,
    /// Diagonal chain link icon.
    Link,
    /// Renewal / date calendar icon.
    Calendar,
    /// Numeric hash icon.
    Number,
    /// Any shared [`ProIcon`].
    Pro(ProIcon),
}

/// Optional leading brand or app mark rendered in [`DataColumnType::Title`] cells.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum DataRowLogo {
    /// No row mark (renders a subtle placeholder tile when empty).
    #[default]
    None,
    /// Multicolor Figma mark.
    Figma,
    /// Indigo Linear mark.
    Linear,
    /// Monochrome Vercel triangle mark.
    Vercel,
    /// Monochrome Framer mark.
    Framer,
    /// Purple Loom sunburst mark.
    Loom,
    /// Lookup an app mark by name via [`pro_app_artwork`].
    App(SharedString),
}

/// Active sort order on a [`DataTable`] column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    /// Lowest-to-highest (`A -> Z`, smallest number first).
    Asc,
    /// Highest-to-lowest (`Z -> A`, largest number first).
    Desc,
}

/// Zero-based `(row, col)` coordinate in the visible grid of [`DataTable`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DataCellPos {
    /// Zero-based visible row index.
    pub row: usize,
    /// Zero-based column index.
    pub col: usize,
}

impl DataCellPos {
    /// Construct a cell coordinate at `(row, col)`.
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// Column definition for [`DataTable`].
#[derive(Clone, Debug, PartialEq)]
pub struct DataColumn {
    /// Unique field key into each [`DataRow`].
    pub key: SharedString,
    /// Column header label.
    pub label: SharedString,
    /// Leading header icon.
    pub icon: DataColumnIcon,
    /// Optional fixed pixel width; `None` shares proportional track width.
    pub width: Option<f32>,
    /// Presentation and editor behavior for cells in this column.
    pub col_type: DataColumnType,
    /// Whether cells in this column can be edited, pasted into, or cleared.
    pub editable: bool,
    /// Preset option labels when `col_type` is [`DataColumnType::Tag`].
    pub options: Vec<SharedString>,
}

impl DataColumn {
    /// Create an editable column with `key` and `label`.
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            icon: DataColumnIcon::Text,
            width: None,
            col_type: DataColumnType::Text,
            editable: true,
            options: Vec::new(),
        }
    }

    /// Set the column presentation type and matching default icon.
    pub fn with_type(mut self, col_type: DataColumnType) -> Self {
        self.col_type = col_type;
        self.icon = match col_type {
            DataColumnType::Title | DataColumnType::Text => DataColumnIcon::Text,
            DataColumnType::Tag => DataColumnIcon::Tag,
            DataColumnType::Link => DataColumnIcon::Link,
            DataColumnType::Number => DataColumnIcon::Number,
        };
        self
    }

    /// Override the leading column header icon.
    pub fn with_icon(mut self, icon: DataColumnIcon) -> Self {
        self.icon = icon;
        self
    }

    /// Fix the column width in pixels.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = Some(width.max(72.));
        self
    }

    /// Configure whether cells in this column accept edits.
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Provide selectable tag options for [`DataColumnType::Tag`] columns.
    pub fn with_options(
        mut self,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.options = options.into_iter().map(Into::into).collect();
        self
    }
}

/// Single record row inside [`DataTable`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataRow {
    /// Stable unique row identifier.
    pub id: SharedString,
    /// Optional brand/app mark shown in [`DataColumnType::Title`] cells.
    pub logo: DataRowLogo,
    /// Cell string values keyed by [`DataColumn::key`].
    pub cells: BTreeMap<SharedString, SharedString>,
}

impl DataRow {
    /// Construct an empty row with `id`.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            logo: DataRowLogo::None,
            cells: BTreeMap::new(),
        }
    }

    /// Attach a leading logo mark to this row.
    pub fn with_logo(mut self, logo: DataRowLogo) -> Self {
        self.logo = logo;
        self
    }

    /// Insert or replace a cell value for `key`.
    pub fn with_cell(
        mut self,
        key: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        self.cells.insert(key.into(), value.into());
        self
    }

    /// Read the string value for `key`, returning `""` when unset.
    pub fn cell(&self, key: &str) -> &str {
        self.cells.get(key).map(|s| s.as_ref()).unwrap_or("")
    }

    /// Mutate the string value for `key`.
    pub fn set_cell(&mut self, key: impl Into<SharedString>, value: impl Into<SharedString>) {
        self.cells.insert(key.into(), value.into());
    }
}

/// Typed interaction and mutation events emitted by [`DataTable`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataTableEvent {
    /// Emitted after any edit, paste, clear, row addition, deletion, undo, or redo.
    RowsChanged(Vec<DataRow>),
    /// Emitted when an individual cell edit or tag selection commits.
    CellEdited {
        row_id: SharedString,
        column_key: SharedString,
        value: SharedString,
    },
    /// Emitted when checked row ids change.
    SelectionChanged(Vec<SharedString>),
    /// Emitted when the active cell or rectangular selection range changes.
    RangeChanged {
        anchor: DataCellPos,
        active: DataCellPos,
    },
    /// Emitted when column sorting changes.
    SortChanged(Option<(SharedString, SortDirection)>),
    /// Emitted when a cell or range is copied to the clipboard.
    Copied { rows: usize, cols: usize },
    /// Emitted when TSV data is pasted into the table.
    Pasted { rows: usize, cols: usize },
}

impl EventEmitter<DataTableEvent> for DataTable {}

#[derive(Clone)]
struct TableSnapshot {
    rows: Vec<DataRow>,
    checked: BTreeSet<SharedString>,
}

enum MoveAfterCommit {
    Stay,
    Down,
    NextCol(bool),
}

/// Format a numeric string with grouped thousands and a typographic minus sign (`−4,200`).
pub fn format_number_cell(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let cleaned: String = trimmed
        .chars()
        .filter(|&c| c != ',' && c != '_' && c != ' ')
        .collect();
    let normalized = cleaned.replace('\u{2212}', "-");
    let Ok(val) = normalized.parse::<f64>() else {
        return trimmed.to_string();
    };
    if !val.is_finite() {
        return trimmed.to_string();
    }
    let neg = val < 0.0;
    let abs_str = if let Some((_, frac)) = normalized.split_once('.') {
        format!("{:.*}", frac.len().min(6), val.abs())
    } else {
        format!("{:.0}", val.abs())
    };
    let (int_part, frac_part) = match abs_str.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (abs_str.as_str(), None),
    };
    let mut grouped = String::with_capacity(int_part.len() + int_part.len() / 3);
    for (idx, ch) in int_part.chars().enumerate() {
        if idx > 0 && (int_part.len() - idx) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let sign = if neg && grouped != "0" {
        "\u{2212}"
    } else {
        ""
    };
    match frac_part {
        Some(f) if !f.is_empty() => format!("{sign}{grouped}.{f}"),
        _ => format!("{sign}{grouped}"),
    }
}

fn parse_numeric_sort(raw: &str) -> Option<f64> {
    let cleaned: String = raw
        .trim()
        .replace('\u{2212}', "-")
        .chars()
        .filter(|&c| c != ',' && c != '$' && c != '%' && c != ' ')
        .collect();
    cleaned.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Spreadsheet-style editable data table component (`data-table`).
pub struct DataTable {
    id: ElementId,
    title: SharedString,
    columns: Vec<DataColumn>,
    rows: Vec<DataRow>,
    tag_tones: BTreeMap<SharedString, DataTagTone>,
    editable: bool,
    loading: bool,
    empty: bool,
    checked: BTreeSet<SharedString>,
    anchor_cell: Option<DataCellPos>,
    active_cell: Option<DataCellPos>,
    dragging_cells: bool,
    editing_cell: Option<DataCellPos>,
    tag_picker_cell: Option<(DataCellPos, usize)>,
    sort: Option<(usize, SortDirection)>,
    undo_stack: Vec<TableSnapshot>,
    redo_stack: Vec<TableSnapshot>,
    internal_clipboard: String,
    next_row_seq: usize,
    focus: FocusHandle,
    cell_editor: Option<Entity<Editor>>,
    toast: Option<(SharedString, Instant)>,
    started_at: Instant,
}

impl DataTable {
    /// Create a [`DataTable`] preloaded with the canonical `"Design Stack"` workspace dataset.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let columns = vec![
            DataColumn::new("tool", "Tool")
                .with_type(DataColumnType::Title)
                .with_width(216.),
            DataColumn::new("category", "Category")
                .with_type(DataColumnType::Tag)
                .with_width(188.)
                .with_options([
                    "Design",
                    "Issue tracking",
                    "Hosting",
                    "Website",
                    "Video",
                    "Analytics",
                ]),
            DataColumn::new("website", "Website")
                .with_type(DataColumnType::Link)
                .with_width(148.),
            DataColumn::new("renews", "Renews")
                .with_type(DataColumnType::Text)
                .with_icon(DataColumnIcon::Calendar)
                .with_width(128.),
        ];
        let rows = vec![
            DataRow::new("figma")
                .with_logo(DataRowLogo::Figma)
                .with_cell("tool", "Figma")
                .with_cell("category", "Design")
                .with_cell("website", "figma.com")
                .with_cell("renews", "Mar 12, 2026"),
            DataRow::new("linear")
                .with_logo(DataRowLogo::Linear)
                .with_cell("tool", "Linear")
                .with_cell("category", "Issue tracking")
                .with_cell("website", "linear.app")
                .with_cell("renews", "Jan 4, 2026"),
            DataRow::new("vercel")
                .with_logo(DataRowLogo::Vercel)
                .with_cell("tool", "Vercel")
                .with_cell("category", "Hosting")
                .with_cell("website", "vercel.com")
                .with_cell("renews", "Aug 21, 2026"),
            DataRow::new("framer")
                .with_logo(DataRowLogo::Framer)
                .with_cell("tool", "Framer")
                .with_cell("category", "Website")
                .with_cell("website", "framer.com")
                .with_cell("renews", "May 2, 2026"),
            DataRow::new("loom")
                .with_logo(DataRowLogo::Loom)
                .with_cell("tool", "Loom")
                .with_cell("category", "Video")
                .with_cell("website", "loom.com")
                .with_cell("renews", "Nov 18, 2026"),
        ];
        let mut tag_tones = BTreeMap::new();
        tag_tones.insert("Design".into(), DataTagTone::Violet);
        tag_tones.insert("Issue tracking".into(), DataTagTone::Blue);
        tag_tones.insert("Hosting".into(), DataTagTone::Gray);
        tag_tones.insert("Website".into(), DataTagTone::Pink);
        tag_tones.insert("Video".into(), DataTagTone::Green);
        tag_tones.insert("Analytics".into(), DataTagTone::Amber);

        Self {
            id: id.into(),
            title: "Design Stack".into(),
            columns,
            rows,
            tag_tones,
            editable: true,
            loading: false,
            empty: false,
            checked: BTreeSet::new(),
            anchor_cell: None,
            active_cell: None,
            dragging_cells: false,
            editing_cell: None,
            tag_picker_cell: None,
            sort: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            internal_clipboard: String::new(),
            next_row_seq: 6,
            focus: cx.focus_handle().tab_stop(true),
            cell_editor: None,
            toast: None,
            started_at: Instant::now(),
        }
    }

    /// Set the card heading and accessible grid title.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Replace the table column definitions.
    pub fn with_columns(mut self, columns: Vec<DataColumn>) -> Self {
        self.columns = columns;
        self.anchor_cell = None;
        self.active_cell = None;
        self.sort = None;
        self
    }

    /// Replace the initial backing rows.
    pub fn with_rows(mut self, rows: Vec<DataRow>) -> Self {
        self.next_row_seq = rows.len() + 1;
        self.rows = rows;
        self.checked.clear();
        self.anchor_cell = None;
        self.active_cell = None;
        self
    }

    /// Map tag labels to semantic [`DataTagTone`] colors.
    pub fn with_tag_tones<K, I>(mut self, tones: I) -> Self
    where
        K: Into<SharedString>,
        I: IntoIterator<Item = (K, DataTagTone)>,
    {
        for (k, tone) in tones {
            self.tag_tones.insert(k.into(), tone);
        }
        self
    }

    /// Enable or disable in-place editing, pasting, clearing, and row addition/deletion.
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Render synchronized skeleton placeholders while data loads.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty-table state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the current backing rows in insertion order.
    pub fn rows(&self) -> &[DataRow] {
        &self.rows
    }

    /// Return the ids of all checked rows.
    pub fn checked_ids(&self) -> Vec<SharedString> {
        self.rows
            .iter()
            .filter(|r| self.checked.contains(&r.id))
            .map(|r| r.id.clone())
            .collect()
    }

    /// Return the normalized `(top_left, bottom_right)` selected cell bounds, if any.
    pub fn selected_range(&self) -> Option<(DataCellPos, DataCellPos)> {
        let a = self.anchor_cell?;
        let b = self.active_cell?;
        Some((
            DataCellPos::new(a.row.min(b.row), a.col.min(b.col)),
            DataCellPos::new(a.row.max(b.row), a.col.max(b.col)),
        ))
    }

    /// Whether at least one undo step is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether at least one redo step is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    fn visible_row_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.rows.len()).collect();
        if let Some((col_idx, dir)) = self.sort
            && let Some(col) = self.columns.get(col_idx)
        {
            let key = col.key.as_ref();
            let numeric = col.col_type == DataColumnType::Number;
            indices.sort_by(|&ia, &ib| {
                let va = self.rows[ia].cell(key);
                let vb = self.rows[ib].cell(key);
                let cmp = if numeric {
                    match (parse_numeric_sort(va), parse_numeric_sort(vb)) {
                        (Some(na), Some(nb)) => {
                            na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        _ => va.to_lowercase().cmp(&vb.to_lowercase()),
                    }
                } else {
                    va.to_lowercase().cmp(&vb.to_lowercase())
                };
                match dir {
                    SortDirection::Asc => cmp,
                    SortDirection::Desc => cmp.reverse(),
                }
            });
        }
        indices
    }

    fn push_undo(&mut self) {
        if self.undo_stack.len() >= MAX_UNDO_STEPS {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(TableSnapshot {
            rows: self.rows.clone(),
            checked: self.checked.clone(),
        });
        self.redo_stack.clear();
    }

    /// Revert the most recent table mutation.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if !self.editable {
            return;
        }
        let Some(prev) = self.undo_stack.pop() else {
            return;
        };
        self.redo_stack.push(TableSnapshot {
            rows: std::mem::replace(&mut self.rows, prev.rows),
            checked: std::mem::replace(&mut self.checked, prev.checked),
        });
        self.editing_cell = None;
        self.tag_picker_cell = None;
        self.clamp_selection();
        cx.emit(DataTableEvent::RowsChanged(self.rows.clone()));
        cx.notify();
    }

    /// Reapply the most recently undone table mutation.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if !self.editable {
            return;
        }
        let Some(next) = self.redo_stack.pop() else {
            return;
        };
        self.undo_stack.push(TableSnapshot {
            rows: std::mem::replace(&mut self.rows, next.rows),
            checked: std::mem::replace(&mut self.checked, next.checked),
        });
        self.editing_cell = None;
        self.tag_picker_cell = None;
        self.clamp_selection();
        cx.emit(DataTableEvent::RowsChanged(self.rows.clone()));
        cx.notify();
    }

    fn clamp_selection(&mut self) {
        if self.rows.is_empty() || self.columns.is_empty() {
            self.anchor_cell = None;
            self.active_cell = None;
            return;
        }
        let max_r = self.rows.len() - 1;
        let max_c = self.columns.len() - 1;
        if let Some(a) = &mut self.anchor_cell {
            a.row = a.row.min(max_r);
            a.col = a.col.min(max_c);
        }
        if let Some(b) = &mut self.active_cell {
            b.row = b.row.min(max_r);
            b.col = b.col.min(max_c);
        }
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Entity<Editor> {
        if let Some(ed) = &self.cell_editor {
            return ed.clone();
        }
        let label: SharedString = tr(cx, "Edit cell", "编辑单元格").into();
        let ed = cx.new(|cx| {
            let mut e = Editor::new(label, InputMode::Single, w, cx);
            e.font_size = 13.;
            e
        });
        cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
            EditEvent::Submitted => this.commit_inline_edit(MoveAfterCommit::Down, w, cx),
            EditEvent::Escape => this.cancel_inline_edit(w, cx),
            EditEvent::Navigate(back) => {
                this.commit_inline_edit(MoveAfterCommit::NextCol(*back), w, cx);
            }
            EditEvent::Blurred => this.commit_inline_edit(MoveAfterCommit::Stay, w, cx),
            _ => {}
        })
        .detach();
        self.cell_editor = Some(ed.clone());
        ed
    }

    fn select_cell(&mut self, pos: DataCellPos, extend: bool, cx: &mut Context<Self>) {
        if self.rows.is_empty() || self.columns.is_empty() {
            return;
        }
        let clamped = DataCellPos::new(
            pos.row.min(self.rows.len() - 1),
            pos.col.min(self.columns.len() - 1),
        );
        let anchor = if extend {
            self.anchor_cell.unwrap_or(clamped)
        } else {
            clamped
        };
        self.anchor_cell = Some(anchor);
        self.active_cell = Some(clamped);
        cx.emit(DataTableEvent::RangeChanged {
            anchor,
            active: clamped,
        });
        cx.notify();
    }

    fn start_edit_cell(
        &mut self,
        pos: DataCellPos,
        seed_text: Option<&str>,
        prefer_picker: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.editable {
            return;
        }
        let vis = self.visible_row_indices();
        let (Some(&backing_row), Some(col)) = (vis.get(pos.row), self.columns.get(pos.col)) else {
            return;
        };
        if !col.editable {
            return;
        }
        self.anchor_cell = Some(pos);
        self.active_cell = Some(pos);

        if prefer_picker
            && seed_text.is_none()
            && col.col_type == DataColumnType::Tag
            && !col.options.is_empty()
        {
            let cur = self.rows[backing_row].cell(&col.key);
            let idx = col
                .options
                .iter()
                .position(|opt| opt.as_ref() == cur)
                .unwrap_or(0);
            self.editing_cell = None;
            self.tag_picker_cell = Some((pos, idx));
            self.focus.focus(w, cx);
            cx.notify();
            return;
        }

        self.tag_picker_cell = None;
        self.editing_cell = Some(pos);
        let initial = match seed_text {
            Some(s) => s.to_string(),
            None => self.rows[backing_row].cell(&col.key).to_string(),
        };
        let select_all = seed_text.is_none();
        let ed = self.ensure_editor(w, cx);
        ed.update(cx, |e, cx| {
            e.set_value(initial, w, cx);
            if select_all {
                e.select_all(w, cx);
            }
            e.focus_handle(cx).focus(w, cx);
        });
        cx.notify();
    }

    fn commit_inline_edit(
        &mut self,
        next: MoveAfterCommit,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(pos) = self.editing_cell.take() else {
            return;
        };
        let Some(ed) = &self.cell_editor else {
            return;
        };
        let new_val: SharedString = ed.read(cx).value().to_string().into();
        let vis = self.visible_row_indices();
        if let (Some(&backing_row), Some(col)) = (vis.get(pos.row), self.columns.get(pos.col))
            && col.editable
        {
            let key = col.key.clone();
            let old_val = self.rows[backing_row].cell(&key);
            if old_val != new_val.as_ref() {
                self.push_undo();
                let row_id = self.rows[backing_row].id.clone();
                self.rows[backing_row].set_cell(key.clone(), new_val.clone());
                cx.emit(DataTableEvent::CellEdited {
                    row_id,
                    column_key: key,
                    value: new_val,
                });
                cx.emit(DataTableEvent::RowsChanged(self.rows.clone()));
            }
        }
        match next {
            MoveAfterCommit::Stay => {}
            MoveAfterCommit::Down => {
                self.focus.focus(w, cx);
                let next_r = (pos.row + 1).min(self.rows.len().saturating_sub(1));
                self.select_cell(DataCellPos::new(next_r, pos.col), false, cx);
            }
            MoveAfterCommit::NextCol(back) => {
                self.focus.focus(w, cx);
                self.step_tab(back, cx);
            }
        }
        cx.notify();
    }

    fn cancel_inline_edit(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.editing_cell = None;
        self.tag_picker_cell = None;
        self.focus.focus(w, cx);
        cx.notify();
    }

    fn apply_tag_option(
        &mut self,
        pos: DataCellPos,
        option: SharedString,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tag_picker_cell = None;
        let vis = self.visible_row_indices();
        if let (Some(&backing_row), Some(col)) = (vis.get(pos.row), self.columns.get(pos.col))
            && col.editable
        {
            let key = col.key.clone();
            if self.rows[backing_row].cell(&key) != option.as_ref() {
                self.push_undo();
                let row_id = self.rows[backing_row].id.clone();
                self.rows[backing_row].set_cell(key.clone(), option.clone());
                cx.emit(DataTableEvent::CellEdited {
                    row_id,
                    column_key: key,
                    value: option,
                });
                cx.emit(DataTableEvent::RowsChanged(self.rows.clone()));
            }
        }
        self.focus.focus(w, cx);
        cx.notify();
    }

    fn step_tab(&mut self, backward: bool, cx: &mut Context<Self>) {
        if self.rows.is_empty() || self.columns.is_empty() {
            return;
        }
        let cols = self.columns.len();
        let rows = self.rows.len();
        let cur = self.active_cell.unwrap_or(DataCellPos::new(0, 0));
        let flat = cur.row * cols + cur.col;
        let total = rows * cols;
        let next_flat = if backward {
            if flat == 0 { total - 1 } else { flat - 1 }
        } else {
            (flat + 1) % total
        };
        self.select_cell(
            DataCellPos::new(next_flat / cols, next_flat % cols),
            false,
            cx,
        );
    }

    /// Copy the currently selected cell or rectangular range as TSV.
    pub fn copy_selection(&mut self, cx: &mut Context<Self>) -> Option<String> {
        let (p0, p1) = self.selected_range()?;
        let vis = self.visible_row_indices();
        let mut lines = Vec::with_capacity(p1.row - p0.row + 1);
        for r in p0.row..=p1.row {
            let Some(&backing) = vis.get(r) else {
                continue;
            };
            let mut fields = Vec::with_capacity(p1.col - p0.col + 1);
            for c in p0.col..=p1.col {
                if let Some(col) = self.columns.get(c) {
                    fields.push(self.rows[backing].cell(&col.key).to_string());
                }
            }
            lines.push(fields.join("\t"));
        }
        let tsv = lines.join("\n");
        let r_cnt = p1.row - p0.row + 1;
        let c_cnt = p1.col - p0.col + 1;
        self.internal_clipboard = tsv.clone();
        clipboard::write(&tsv, cx).detach();
        let msg = if r_cnt * c_cnt == 1 {
            SharedString::from(tr(cx, "Copied cell", "已复制单元格"))
        } else {
            SharedString::from(format!(
                "{} {}",
                r_cnt * c_cnt,
                tr(cx, "cells copied", "个单元格已复制")
            ))
        };
        self.toast = Some((msg, Instant::now()));
        cx.emit(DataTableEvent::Copied {
            rows: r_cnt,
            cols: c_cnt,
        });
        cx.notify();
        Some(tsv)
    }

    fn clear_selected_cells(&mut self, cx: &mut Context<Self>) {
        if !self.editable {
            return;
        }
        let Some((p0, p1)) = self.selected_range() else {
            return;
        };
        let vis = self.visible_row_indices();
        let mut changed = false;
        for r in p0.row..=p1.row {
            let Some(&backing) = vis.get(r) else {
                continue;
            };
            for c in p0.col..=p1.col {
                let col_info = self
                    .columns
                    .get(c)
                    .map(|col| (col.editable, col.key.clone()));
                if let Some((true, key)) = col_info
                    && !self.rows[backing].cell(&key).is_empty()
                {
                    if !changed {
                        self.push_undo();
                        changed = true;
                    }
                    self.rows[backing].set_cell(key, "");
                }
            }
        }
        if changed {
            cx.emit(DataTableEvent::RowsChanged(self.rows.clone()));
            cx.notify();
        }
    }

    /// Paste tab-separated lines starting at the top-left of the current selection.
    pub fn paste_tsv(&mut self, raw: &str, cx: &mut Context<Self>) {
        if !self.editable || self.columns.is_empty() || raw.is_empty() {
            return;
        }
        let trimmed = raw.strip_suffix('\n').unwrap_or(raw);
        let trimmed = trimmed.strip_suffix('\r').unwrap_or(trimmed);
        let parsed: Vec<Vec<&str>> = trimmed
            .split('\n')
            .map(|line| {
                line.strip_suffix('\r')
                    .unwrap_or(line)
                    .split('\t')
                    .collect()
            })
            .collect();
        if parsed.is_empty() {
            return;
        }

        let start = self
            .selected_range()
            .map(|(p0, _)| p0)
            .unwrap_or(DataCellPos::new(0, 0));

        self.push_undo();
        let mut vis = self.visible_row_indices();
        let mut max_cols = 0usize;
        for (dr, line_cols) in parsed.iter().enumerate() {
            max_cols = max_cols.max(line_cols.len());
            let target_vis_row = start.row + dr;
            let backing_row = if let Some(&existing) = vis.get(target_vis_row) {
                existing
            } else {
                let id = format!("row-{}", self.next_row_seq);
                self.next_row_seq += 1;
                self.rows.push(DataRow::new(id));
                let idx = self.rows.len() - 1;
                vis.push(idx);
                idx
            };
            for (dc, &val) in line_cols.iter().enumerate() {
                let target_col = start.col + dc;
                if let Some(col) = self.columns.get(target_col)
                    && col.editable
                {
                    let key = col.key.clone();
                    self.rows[backing_row].set_cell(key, val.to_string());
                }
            }
        }
        let end_r =
            (start.row + parsed.len().saturating_sub(1)).min(self.rows.len().saturating_sub(1));
        let end_c =
            (start.col + max_cols.saturating_sub(1)).min(self.columns.len().saturating_sub(1));
        self.anchor_cell = Some(start);
        self.active_cell = Some(DataCellPos::new(end_r, end_c));
        let pasted_cells = parsed.len() * max_cols.max(1);
        self.toast = Some((
            SharedString::from(format!(
                "{} {}",
                pasted_cells,
                tr(cx, "cells pasted", "个单元格已粘贴")
            )),
            Instant::now(),
        ));
        cx.emit(DataTableEvent::Pasted {
            rows: parsed.len(),
            cols: max_cols.max(1),
        });
        cx.emit(DataTableEvent::RowsChanged(self.rows.clone()));
        cx.notify();
    }

    fn paste_from_clipboard(&mut self, cx: &mut Context<Self>) {
        let clip_text = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.internal_clipboard.clone());
        if !clip_text.is_empty() {
            self.paste_tsv(&clip_text, cx);
        }
    }

    fn toggle_header_check(&mut self, cx: &mut Context<Self>) {
        if self.rows.is_empty() {
            return;
        }
        if self.checked.len() == self.rows.len() {
            self.checked.clear();
        } else {
            for r in &self.rows {
                self.checked.insert(r.id.clone());
            }
        }
        cx.emit(DataTableEvent::SelectionChanged(self.checked_ids()));
        cx.notify();
    }

    fn toggle_row_check(&mut self, row_id: &SharedString, cx: &mut Context<Self>) {
        if !self.checked.remove(row_id) {
            self.checked.insert(row_id.clone());
        }
        cx.emit(DataTableEvent::SelectionChanged(self.checked_ids()));
        cx.notify();
    }

    fn toggle_sort(&mut self, col_idx: usize, cx: &mut Context<Self>) {
        self.editing_cell = None;
        self.tag_picker_cell = None;
        self.sort = match self.sort {
            Some((c, SortDirection::Asc)) if c == col_idx => Some((col_idx, SortDirection::Desc)),
            Some((c, SortDirection::Desc)) if c == col_idx => None,
            _ => Some((col_idx, SortDirection::Asc)),
        };
        let payload = self
            .sort
            .and_then(|(i, dir)| self.columns.get(i).map(|col| (col.key.clone(), dir)));
        cx.emit(DataTableEvent::SortChanged(payload));
        cx.notify();
    }

    fn add_row(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.editable {
            return;
        }
        self.push_undo();
        self.sort = None;
        let id = format!("row-{}", self.next_row_seq);
        self.next_row_seq += 1;
        let mut row = DataRow::new(id);
        for col in &self.columns {
            if col.col_type == DataColumnType::Tag
                && let Some(first) = col.options.first()
            {
                row.set_cell(col.key.clone(), first.clone());
            }
        }
        self.rows.push(row);
        let new_vis_idx = self.rows.len() - 1;
        cx.emit(DataTableEvent::RowsChanged(self.rows.clone()));
        if !self.columns.is_empty() {
            self.start_edit_cell(DataCellPos::new(new_vis_idx, 0), None, false, w, cx);
        } else {
            cx.notify();
        }
    }

    fn delete_checked_rows(&mut self, cx: &mut Context<Self>) {
        if !self.editable || self.checked.is_empty() {
            return;
        }
        self.push_undo();
        self.rows.retain(|r| !self.checked.contains(&r.id));
        self.checked.clear();
        self.editing_cell = None;
        self.tag_picker_cell = None;
        self.clamp_selection();
        cx.emit(DataTableEvent::SelectionChanged(Vec::new()));
        cx.emit(DataTableEvent::RowsChanged(self.rows.clone()));
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.editing_cell.is_some() {
            return;
        }
        let ks = &ev.keystroke;
        let key = ks.key.as_str();
        let cmd = ks.modifiers.platform || ks.modifiers.control;
        let shift = ks.modifiers.shift;

        if let Some((pos, idx)) = self.tag_picker_cell {
            let opt_len = self
                .columns
                .get(pos.col)
                .map(|c| c.options.len())
                .unwrap_or(0);
            match key {
                "escape" => {
                    self.tag_picker_cell = None;
                    cx.notify();
                }
                "up" if opt_len > 0 => {
                    let next = if idx == 0 { opt_len - 1 } else { idx - 1 };
                    self.tag_picker_cell = Some((pos, next));
                    cx.notify();
                }
                "down" if opt_len > 0 => {
                    let next = (idx + 1) % opt_len;
                    self.tag_picker_cell = Some((pos, next));
                    cx.notify();
                }
                "enter" | "space" if opt_len > 0 => {
                    if let Some(opt) = self
                        .columns
                        .get(pos.col)
                        .and_then(|c| c.options.get(idx))
                        .cloned()
                    {
                        self.apply_tag_option(pos, opt, w, cx);
                    }
                }
                _ => {}
            }
            return;
        }

        if cmd {
            match key {
                "z" => {
                    if shift {
                        self.redo(cx);
                    } else {
                        self.undo(cx);
                    }
                    return;
                }
                "y" => {
                    self.redo(cx);
                    return;
                }
                "c" => {
                    let _ = self.copy_selection(cx);
                    return;
                }
                "x" => {
                    if self.copy_selection(cx).is_some() {
                        self.clear_selected_cells(cx);
                    }
                    return;
                }
                "v" => {
                    self.paste_from_clipboard(cx);
                    return;
                }
                "a" => {
                    if !self.rows.is_empty() && !self.columns.is_empty() {
                        let a = DataCellPos::new(0, 0);
                        let b = DataCellPos::new(self.rows.len() - 1, self.columns.len() - 1);
                        self.anchor_cell = Some(a);
                        self.active_cell = Some(b);
                        cx.emit(DataTableEvent::RangeChanged {
                            anchor: a,
                            active: b,
                        });
                        cx.notify();
                    }
                    return;
                }
                _ => {}
            }
        }

        if self.rows.is_empty() || self.columns.is_empty() {
            return;
        }
        let max_r = self.rows.len() - 1;
        let max_c = self.columns.len() - 1;
        let cur = self.active_cell.unwrap_or(DataCellPos::new(0, 0));

        match key {
            "up" => {
                let r = if cmd { 0 } else { cur.row.saturating_sub(1) };
                self.select_cell(DataCellPos::new(r, cur.col), shift, cx);
            }
            "down" => {
                let r = if cmd { max_r } else { (cur.row + 1).min(max_r) };
                self.select_cell(DataCellPos::new(r, cur.col), shift, cx);
            }
            "left" => {
                let c = if cmd { 0 } else { cur.col.saturating_sub(1) };
                self.select_cell(DataCellPos::new(cur.row, c), shift, cx);
            }
            "right" => {
                let c = if cmd { max_c } else { (cur.col + 1).min(max_c) };
                self.select_cell(DataCellPos::new(cur.row, c), shift, cx);
            }
            "tab" => {
                self.step_tab(shift, cx);
            }
            "enter" | "f2" => {
                self.start_edit_cell(cur, None, true, w, cx);
            }
            "backspace" | "delete" => {
                self.clear_selected_cells(cx);
            }
            "escape" => {
                if let (Some(a), Some(b)) = (self.anchor_cell, self.active_cell) {
                    if a != b {
                        self.anchor_cell = Some(b);
                    } else {
                        self.anchor_cell = None;
                        self.active_cell = None;
                    }
                    cx.notify();
                }
            }
            _ => {
                if !cmd
                    && !ks.modifiers.alt
                    && let Some(kc) = &ks.key_char
                    && !kc.is_empty()
                    && kc.chars().all(|ch| !ch.is_control())
                {
                    self.start_edit_cell(cur, Some(kc.as_str()), false, w, cx);
                }
            }
        }
    }

    fn summary_text(&self, cx: &App) -> SharedString {
        if let Some((p0, p1)) = self.selected_range() {
            let count = (p1.row - p0.row + 1) * (p1.col - p0.col + 1);
            if count > 1 {
                let vis = self.visible_row_indices();
                let mut num_sum = 0.0;
                let mut num_cnt = 0usize;
                for r in p0.row..=p1.row {
                    if let Some(&b) = vis.get(r) {
                        for c in p0.col..=p1.col {
                            if let Some(col) = self.columns.get(c)
                                && col.col_type == DataColumnType::Number
                                && let Some(v) = parse_numeric_sort(self.rows[b].cell(&col.key))
                            {
                                num_sum += v;
                                num_cnt += 1;
                            }
                        }
                    }
                }
                if num_cnt > 0 {
                    return format!(
                        "{count} {} · {} {}",
                        tr(cx, "cells", "单元格"),
                        tr(cx, "Sum", "合计"),
                        format_number_cell(format!("{num_sum:.2}").trim_end_matches(".00"))
                    )
                    .into();
                }
                return format!("{count} {}", tr(cx, "cells selected", "个单元格已选中")).into();
            }
        }
        if self.editable {
            tr(cx, "Double-click a cell to edit", "双击单元格即可编辑").into()
        } else {
            format!("{} {}", self.rows.len(), tr(cx, "rows", "行")).into()
        }
    }

    fn render_row_logo(&self, logo: &DataRowLogo, p: &ProPalette, cx: &App) -> AnyElement {
        let fg = if p.dark {
            rgb(0xffffff).into()
        } else {
            rgb(0x000000).into()
        };
        match logo {
            DataRowLogo::Figma => pro_raw_svg(FIGMA_LOGO_SVG, 16., 16.),
            DataRowLogo::Linear => pro_raw_svg(LINEAR_LOGO_SVG, 16., 16.),
            DataRowLogo::Vercel => pro_svg("0 0 24 24", VERCEL_INNER_SVG, 15., fg),
            DataRowLogo::Framer => pro_svg("0 0 24 24", FRAMER_INNER_SVG, 15., fg),
            DataRowLogo::Loom => pro_raw_svg(LOOM_LOGO_SVG, 16., 16.),
            DataRowLogo::App(name) => pro_app_artwork(name, 16., cx).unwrap_or_else(|| {
                div()
                    .size(px(16.))
                    .rounded(px(4.))
                    .bg(p.chip)
                    .into_any_element()
            }),
            DataRowLogo::None => div()
                .size(px(16.))
                .rounded(px(4.))
                .bg(p.chip)
                .into_any_element(),
        }
    }

    fn render_col_icon(&self, icon: DataColumnIcon, color: Hsla) -> Option<AnyElement> {
        match icon {
            DataColumnIcon::None => None,
            DataColumnIcon::Text => Some(pro_svg("0 0 16 16", COL_TEXT_SVG, 14., color)),
            DataColumnIcon::Tag => Some(pro_svg("0 0 16 16", COL_TAG_SVG, 14., color)),
            DataColumnIcon::Link => Some(pro_svg("0 0 16 16", COL_LINK_SVG, 14., color)),
            DataColumnIcon::Calendar => Some(pro_svg("0 0 16 16", COL_CALENDAR_SVG, 14., color)),
            DataColumnIcon::Number => Some(pro_svg("0 0 16 16", COL_NUMBER_SVG, 14., color)),
            DataColumnIcon::Pro(ic) => Some(pro_icon(ic, 14., color)),
        }
    }

    fn render_checkbox(
        &self,
        id: ElementId,
        aria: SharedString,
        state: Option<bool>,
        p: &ProPalette,
        on_toggle: impl Fn(&mut Self, &ClickEvent, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let on = state.unwrap_or(true);
        let check_bg: Hsla = if on {
            if p.dark {
                hsla(0., 0., 1., 0.90)
            } else {
                hsla(0., 0., 0., 0.88)
            }
        } else if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let mark_fg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xfcfcfc).into()
        };
        let ring: Hsla = if p.dark {
            hsla(0., 0., 1., 0.28)
        } else {
            hsla(0., 0., 0., 0.22)
        };
        let ring_hover: Hsla = if p.dark {
            hsla(0., 0., 1., 0.45)
        } else {
            hsla(0., 0., 0., 0.40)
        };

        div()
            .id(id)
            .role(Role::CheckBox)
            .aria_label(aria)
            .tab_index(0)
            .size(px(28.))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(on_toggle))
            .child(
                div()
                    .size(px(14.))
                    .rounded(px(4.))
                    .bg(check_bg)
                    .when(!on, |d| {
                        d.border_1()
                            .border_color(ring)
                            .hover(move |s| s.border_color(ring_hover))
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(state == Some(true), |d| {
                        d.child(pro_svg("0 0 16 16", CHECK_TINY_SVG, 11., mark_fg))
                    })
                    .when(state.is_none(), |d| {
                        d.child(pro_svg("0 0 16 16", MINUS_TINY_SVG, 11., mark_fg))
                    }),
            )
    }
}

impl Render for DataTable {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = p.dark;
        let card_bg: Hsla = if dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let row_line: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let row_hover: Hsla = if dark {
            hsla(0., 0., 1., 0.025)
        } else {
            hsla(0., 0., 0., 0.016)
        };
        let wash: Hsla = if dark {
            hsla(0., 0., 1., 0.05)
        } else {
            hsla(0., 0., 0., 0.03)
        };
        let sel_fill: Hsla = if dark {
            hsla(218. / 360., 1.0, 0.65, 0.14)
        } else {
            hsla(215. / 360., 1.0, 0.44, 0.055)
        };
        let accent: Hsla = if dark {
            rgb(0x4fb0fe).into()
        } else {
            hsla(215. / 360., 1.0, 0.44, 0.82)
        };
        let focus_within = self.focus.contains_focused(w, cx);

        if self.loading {
            let phase = if disabled_motion(cx) {
                0.25
            } else {
                w.request_animation_frame();
                (self.started_at.elapsed().as_secs_f32() / 1.4).fract()
            };
            let mut skel = div()
                .id(pro_eid(&self.id, "loading"))
                .w_full()
                .max_w(px(720.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .overflow_hidden()
                .child(
                    div()
                        .h(px(52.))
                        .px(px(16.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(136., 16., 5., phase, cx))
                        .child(pro_skeleton_bar(56., 22., 6., phase, cx)),
                );
            for r in 0..5usize {
                skel = skel.child(
                    div()
                        .h(px(40.))
                        .px(px(16.))
                        .border_t_1()
                        .border_color(row_line)
                        .flex()
                        .items_center()
                        .gap(px(18.))
                        .child(pro_skeleton_bar(14., 14., 4., phase, cx))
                        .child(pro_skeleton_bar(
                            120. + (r % 3) as f32 * 22.,
                            13.,
                            4.,
                            phase,
                            cx,
                        ))
                        .child(pro_skeleton_bar(78., 20., 5., phase, cx))
                        .child(pro_skeleton_bar(96., 13., 4., phase, cx))
                        .child(pro_skeleton_bar(84., 13., 4., phase, cx)),
                );
            }
            return skel.into_any_element();
        }

        let can_undo = self.can_undo() && self.editable;
        let can_redo = self.can_redo() && self.editable;
        let icon_color: Hsla = if dark {
            hsla(0., 0., 1., 0.50)
        } else {
            hsla(0., 0., 0., 0.45)
        };

        let header_bar = div()
            .min_h(px(52.))
            .pl(px(16.))
            .pr(px(12.))
            .py(px(12.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(pro_raw_svg(TITLE_BADGE_SVG, 18., 18.))
            .child(
                div()
                    .text_size(px(15.))
                    .line_height(px(24.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .truncate()
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(
                        div()
                            .id(pro_eid(&self.id, "undo-btn"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Undo (Cmd+Z)", "撤销 (Cmd+Z)"))
                            .tab_index(if can_undo { 0 } else { -1 })
                            .size(px(28.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .opacity(if can_undo { 1.0 } else { 0.38 })
                            .when(can_undo, |d| {
                                d.cursor_pointer()
                                    .hover(move |s| s.bg(wash))
                                    .on_click(cx.listener(|this, _, _, cx| this.undo(cx)))
                            })
                            .child(pro_svg("0 0 16 16", UNDO_SVG, 16., icon_color)),
                    )
                    .child(
                        div()
                            .id(pro_eid(&self.id, "redo-btn"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Redo (Cmd+Shift+Z)", "重做 (Cmd+Shift+Z)"))
                            .tab_index(if can_redo { 0 } else { -1 })
                            .size(px(28.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .opacity(if can_redo { 1.0 } else { 0.38 })
                            .when(can_redo, |d| {
                                d.cursor_pointer()
                                    .hover(move |s| s.bg(wash))
                                    .on_click(cx.listener(|this, _, _, cx| this.redo(cx)))
                            })
                            .child(pro_svg("0 0 16 16", REDO_SVG, 16., icon_color)),
                    ),
            );

        if self.empty || self.rows.is_empty() {
            let empty_el = pro_empty_state(
                ProIcon::Database,
                tr(cx, "No records yet", "暂无数据行"),
                tr(
                    cx,
                    "Add a row or paste cells from a spreadsheet to get started.",
                    "添加新行或直接从电子表格粘贴数据以开始编辑。",
                ),
                self.editable
                    .then(|| -> crate::components::pro_chrome::ProEmptyCta<Self> {
                        (
                            pro_eid(&self.id, "empty-add"),
                            tr(cx, "Add row", "添加行").into(),
                            Box::new(|this, w, cx| {
                                this.empty = false;
                                this.add_row(w, cx);
                            }),
                        )
                    }),
                cx,
            );
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(720.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .when(!dark, |d| d.shadow_xs())
                .overflow_hidden()
                .child(header_bar)
                .child(div().border_t_1().border_color(row_line).child(empty_el))
                .into_any_element();
        }

        let vis_indices = self.visible_row_indices();
        let range = self.selected_range();
        let multi_cell_range = range.is_some_and(|(a, b)| a != b);
        let active_col_idx = self.active_cell.map(|c| c.col);

        let head_check_state = if self.checked.is_empty() {
            Some(false)
        } else if self.checked.len() == self.rows.len() {
            Some(true)
        } else {
            None
        };

        let mut thead_row = div()
            .w_full()
            .min_w(px(600.))
            .h(px(36.))
            .border_t_1()
            .border_color(row_line)
            .flex()
            .items_center()
            .child(
                div()
                    .w(px(40.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.render_checkbox(
                        pro_eid(&self.id, "chk-all"),
                        tr(cx, "Select all rows", "全选所有行").into(),
                        head_check_state,
                        &p,
                        |this, _, _, cx| this.toggle_header_check(cx),
                        cx,
                    )),
            );

        for (col_idx, col) in self.columns.iter().enumerate() {
            let sorted_dir = self
                .sort
                .and_then(|(sc, dir)| (sc == col_idx).then_some(dir));
            let col_active = active_col_idx == Some(col_idx);
            let label_fg = if col_active || sorted_dir.is_some() {
                p.ink
            } else {
                p.text
            };
            let sort_glyph = match sorted_dir {
                Some(SortDirection::Asc) => Some(pro_svg("0 0 16 16", SORT_UP_SVG, 12., accent)),
                Some(SortDirection::Desc) => Some(pro_svg("0 0 16 16", SORT_DOWN_SVG, 12., accent)),
                None => None,
            };
            let icon_el = self.render_col_icon(col.icon, icon_color);
            let col_label = col.label.clone();

            let mut th_cell = div()
                .h_full()
                .px(px(6.))
                .flex()
                .items_center()
                .min_w(px(96.));
            if let Some(w_px) = col.width {
                th_cell = th_cell.flex_1().flex_basis(px(w_px));
            } else {
                th_cell = th_cell.flex_1();
            }

            th_cell = th_cell.child(
                div()
                    .id(pro_eid(&self.id, format!("th-{col_idx}")))
                    .role(Role::Button)
                    .aria_label(format!("{} {}", tr(cx, "Sort by", "排序按"), col_label))
                    .tab_index(0)
                    .w_full()
                    .h(px(28.))
                    .px(px(6.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(label_fg)
                    .cursor_pointer()
                    .hover(move |s| s.bg(wash).text_color(p.ink))
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_sort(col_idx, cx)))
                    .when_some(icon_el, |d, ic| d.child(ic))
                    .child(div().truncate().child(col_label))
                    .when_some(sort_glyph, |d, sg| d.child(div().ml_auto().child(sg))),
            );
            thead_row = thead_row.child(th_cell);
        }

        let mut tbody = div().w_full().min_w(px(600.)).flex().flex_col();
        let underline_c: Hsla = if dark {
            hsla(0., 0., 1., 0.22)
        } else {
            hsla(0., 0., 0., 0.16)
        };

        for (vis_r, &backing_r) in vis_indices.iter().enumerate() {
            let row = self.rows[backing_r].clone();
            let row_id = row.id.clone();
            let is_row_checked = self.checked.contains(&row_id);

            let mut tr_el = div()
                .id(pro_eid(&self.id, format!("tr-{}", row.id)))
                .w_full()
                .h(px(40.))
                .border_t_1()
                .border_color(row_line)
                .flex()
                .items_center()
                .bg(if is_row_checked {
                    sel_fill
                } else {
                    transparent_black()
                })
                .when(!is_row_checked, |d| d.hover(move |s| s.bg(row_hover)))
                .child(
                    div()
                        .w(px(40.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child({
                            let rid = row_id.clone();
                            self.render_checkbox(
                                pro_eid(&self.id, format!("chk-{}", row.id)),
                                format!("{} {}", tr(cx, "Select row", "选择行"), vis_r + 1).into(),
                                Some(is_row_checked),
                                &p,
                                move |this, _, _, cx| this.toggle_row_check(&rid, cx),
                                cx,
                            )
                        }),
                );

            for (col_idx, col) in self.columns.iter().enumerate() {
                let pos = DataCellPos::new(vis_r, col_idx);
                let is_active = self.active_cell == Some(pos);
                let in_range = range.is_some_and(|(p0, p1)| {
                    vis_r >= p0.row && vis_r <= p1.row && col_idx >= p0.col && col_idx <= p1.col
                });
                let is_editing = self.editing_cell == Some(pos);
                let raw_val = row.cell(&col.key).to_string();

                let (top_edge, bot_edge, left_edge, right_edge) = match range {
                    Some((p0, p1)) if multi_cell_range && in_range => (
                        vis_r == p0.row,
                        vis_r == p1.row,
                        col_idx == p0.col,
                        col_idx == p1.col,
                    ),
                    _ => (false, false, false, false),
                };

                let mut td = div()
                    .id(pro_eid(&self.id, format!("td-{vis_r}-{col_idx}")))
                    .h_full()
                    .min_w(px(96.))
                    .relative()
                    .flex()
                    .items_center()
                    .cursor(CursorStyle::Crosshair)
                    .when(in_range && multi_cell_range && !is_row_checked, |d| {
                        d.bg(sel_fill)
                    })
                    .when(is_editing, |d| d.bg(card_bg));

                if let Some(w_px) = col.width {
                    td = td.flex_1().flex_basis(px(w_px));
                } else {
                    td = td.flex_1();
                }

                if is_editing {
                    if let Some(ed) = &self.cell_editor {
                        td = td
                            .px(px(12.))
                            .child(div().w_full().h(px(22.)).child(ed.clone()));
                    }
                } else {
                    td = td
                        .px(px(12.))
                        .overflow_hidden()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, ev: &MouseDownEvent, w, cx| {
                                if this.editing_cell.is_some() && this.editing_cell != Some(pos) {
                                    this.commit_inline_edit(MoveAfterCommit::Stay, w, cx);
                                }
                                this.tag_picker_cell = None;
                                this.focus.focus(w, cx);
                                if ev.click_count >= 2 {
                                    this.dragging_cells = false;
                                    this.start_edit_cell(pos, None, true, w, cx);
                                } else {
                                    this.dragging_cells = true;
                                    this.select_cell(pos, ev.modifiers.shift, cx);
                                }
                            }),
                        )
                        .on_mouse_move(cx.listener(move |this, _, _, cx| {
                            if this.dragging_cells && this.active_cell != Some(pos) {
                                this.select_cell(pos, true, cx);
                            }
                        }));

                    let cell_body: AnyElement = match col.col_type {
                        DataColumnType::Title => {
                            let is_empty = raw_val.trim().is_empty();
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .min_w_0()
                                .child(self.render_row_logo(&row.logo, &p, cx))
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(13.))
                                        .font_weight(if is_empty {
                                            FontWeight::NORMAL
                                        } else {
                                            FontWeight::MEDIUM
                                        })
                                        .text_color(if is_empty { p.faint } else { p.ink })
                                        .child(if is_empty {
                                            tr(cx, "Untitled", "未命名").to_string()
                                        } else {
                                            raw_val
                                        }),
                                )
                                .into_any_element()
                        }
                        DataColumnType::Tag => {
                            if raw_val.trim().is_empty() {
                                div()
                                    .text_size(px(12.5))
                                    .text_color(p.faint)
                                    .child("—")
                                    .into_any_element()
                            } else {
                                let tone = self
                                    .tag_tones
                                    .get(raw_val.as_str())
                                    .copied()
                                    .unwrap_or_default();
                                let (tbg, tfg) = tone.colors(dark);
                                div()
                                    .h(px(20.))
                                    .px(px(6.))
                                    .rounded(px(5.))
                                    .bg(tbg)
                                    .text_color(tfg)
                                    .text_size(px(12.))
                                    .line_height(px(20.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .flex()
                                    .items_center()
                                    .truncate()
                                    .child(raw_val)
                                    .into_any_element()
                            }
                        }
                        DataColumnType::Link => div()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .text_color(p.text)
                            .border_b_1()
                            .border_color(underline_c)
                            .truncate()
                            .child(raw_val)
                            .into_any_element(),
                        DataColumnType::Number => div()
                            .text_size(px(13.))
                            .text_color(p.text)
                            .truncate()
                            .child(format_number_cell(&raw_val))
                            .into_any_element(),
                        DataColumnType::Text => div()
                            .text_size(px(13.))
                            .text_color(p.text)
                            .truncate()
                            .child(raw_val)
                            .into_any_element(),
                    };
                    td = td.child(cell_body);
                }

                if multi_cell_range && in_range {
                    let frame_c = accent.opacity(if focus_within { 0.85 } else { 0.45 });
                    if top_edge {
                        td = td.child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .right_0()
                                .h(px(1.5))
                                .bg(frame_c),
                        );
                    }
                    if bot_edge {
                        td = td.child(
                            div()
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .right_0()
                                .h(px(1.5))
                                .bg(frame_c),
                        );
                    }
                    if left_edge {
                        td = td.child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left_0()
                                .w(px(1.5))
                                .bg(frame_c),
                        );
                    }
                    if right_edge {
                        td = td.child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .right_0()
                                .w(px(1.5))
                                .bg(frame_c),
                        );
                    }
                }

                if is_active {
                    let ring_c = if focus_within || is_editing {
                        accent
                    } else {
                        accent.opacity(0.45)
                    };
                    td = td.child(
                        div()
                            .absolute()
                            .inset(px(1.))
                            .rounded(px(4.))
                            .border_color(ring_c)
                            .border(px(if focus_within || is_editing { 1.5 } else { 1.0 })),
                    );
                }

                tr_el = tr_el.child(td);
            }
            tbody = tbody.child(tr_el);
        }

        let picker_overlay: Option<AnyElement> = self.tag_picker_cell.and_then(|(pos, hi_idx)| {
            let col = self.columns.get(pos.col)?;
            let backing_r = *vis_indices.get(pos.row)?;
            let current_val = self.rows[backing_r].cell(&col.key).to_string();
            let pop_top = 52.
                + 36.
                + ((pos.row + 1) as f32 * 40.).min((vis_indices.len() as f32 * 40. - 16.).max(40.));
            let mut pop_left = 40.0f32;
            for col in self.columns.iter().take(pos.col) {
                pop_left += col.width.unwrap_or(140.);
            }
            let mut menu = div()
                .id(pro_eid(&self.id, "tag-picker"))
                .absolute()
                .top(px(pop_top))
                .left(px(pop_left.min(520.)))
                .min_w(px(164.))
                .p(px(4.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(2.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());

            for (opt_i, opt) in col.options.iter().enumerate() {
                let opt_str = opt.clone();
                let selected = opt.as_ref() == current_val.as_str();
                let highlighted = opt_i == hi_idx;
                let tone = self
                    .tag_tones
                    .get(opt.as_ref())
                    .copied()
                    .unwrap_or_default();
                let (tbg, tfg) = tone.colors(dark);
                menu = menu.child(
                    div()
                        .id(pro_eid(&self.id, format!("tag-opt-{opt_i}")))
                        .role(Role::MenuItem)
                        .aria_label(opt_str.clone())
                        .h(px(32.))
                        .px(px(6.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .bg(if highlighted {
                            p.chip
                        } else {
                            transparent_black()
                        })
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.chip))
                        .on_click(cx.listener(move |this, _, w, cx| {
                            this.apply_tag_option(pos, opt_str.clone(), w, cx);
                        }))
                        .child(
                            div()
                                .h(px(20.))
                                .px(px(6.))
                                .rounded(px(5.))
                                .bg(tbg)
                                .text_color(tfg)
                                .text_size(px(12.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(opt.clone()),
                        )
                        .when(selected, |d| {
                            d.child(pro_svg("0 0 16 16", CHECK_TINY_SVG, 12., accent))
                        }),
                );
            }
            Some(menu.into_any_element())
        });

        let checked_cnt = self.checked.len();
        let danger_fg: Hsla = if dark {
            rgb(0xff6b5f).into()
        } else {
            rgb(0xf04438).into()
        };
        let danger_wash = danger_fg.opacity(if dark { 0.14 } else { 0.08 });

        let footer_left: AnyElement = if checked_cnt > 0 {
            div()
                .pl(px(8.))
                .flex()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .mr(px(6.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(format!("{checked_cnt} {}", tr(cx, "selected", "项已选"))),
                )
                .child(
                    div()
                        .id(pro_eid(&self.id, "bulk-clear"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Clear selection", "清除勾选"))
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(10.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .cursor_pointer()
                        .hover(move |s| s.bg(wash).text_color(p.ink))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.checked.clear();
                            cx.emit(DataTableEvent::SelectionChanged(Vec::new()));
                            cx.notify();
                        }))
                        .child(tr(cx, "Clear", "取消选择")),
                )
                .when(self.editable, |d| {
                    d.child(
                        div()
                            .id(pro_eid(&self.id, "bulk-delete"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Delete selected rows", "删除已选行"))
                            .tab_index(0)
                            .h(px(28.))
                            .px(px(10.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(danger_fg)
                            .cursor_pointer()
                            .hover(move |s| s.bg(danger_wash))
                            .on_click(cx.listener(|this, _, _, cx| this.delete_checked_rows(cx)))
                            .child(tr(cx, "Delete", "删除")),
                    )
                })
                .into_any_element()
        } else if self.editable {
            div()
                .id(pro_eid(&self.id, "add-row"))
                .role(Role::Button)
                .aria_label(tr(cx, "Add row", "添加行"))
                .tab_index(0)
                .h(px(28.))
                .pl(px(8.))
                .pr(px(10.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.text)
                .cursor_pointer()
                .hover(move |s| s.bg(wash).text_color(p.ink))
                .on_click(cx.listener(|this, _, w, cx| this.add_row(w, cx)))
                .child(pro_svg("0 0 16 16", PLUS_ROW_SVG, 16., icon_color))
                .child(tr(cx, "Add row", "添加行"))
                .into_any_element()
        } else {
            div().pl(px(8.)).into_any_element()
        };

        let footer_bar = div()
            .min_h(px(44.))
            .pl(px(8.))
            .pr(px(16.))
            .py(px(8.))
            .border_t_1()
            .border_color(row_line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(footer_left)
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.muted)
                    .truncate()
                    .child(self.summary_text(cx)),
            );

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Grid)
            .aria_label(self.title.clone())
            .w_full()
            .max_w(px(720.))
            .relative()
            .rounded(px(10.))
            .bg(card_bg)
            .border_1()
            .border_color(card_border)
            .when(!dark, |d| d.shadow_xs())
            .overflow_hidden()
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.dragging_cells {
                        this.dragging_cells = false;
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.dragging_cells {
                        this.dragging_cells = false;
                        cx.notify();
                    }
                }),
            )
            .child(header_bar)
            .child(
                div()
                    .id(pro_eid(&self.id, "hscroll"))
                    .w_full()
                    .overflow_x_scroll()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .child(thead_row)
                            .child(tbody),
                    ),
            )
            .child(footer_bar)
            .when_some(picker_overlay, |d, pop| d.child(pop))
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{DataCellPos, DataColumn, DataColumnType, DataRow, DataTable, format_number_cell};
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn number_formatter_groups_digits_and_uses_real_minus() {
        assert_eq!(format_number_cell("12345679"), "12,345,679");
        assert_eq!(format_number_cell("-4200"), "\u{2212}4,200");
        assert_eq!(format_number_cell("98450.50"), "98,450.50");
        assert_eq!(format_number_cell(""), "");
    }

    #[gpui::test]
    fn range_copy_paste_and_undo_redo_roundtrip(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| {
            DataTable::new("dt-test", cx)
                .with_columns(vec![
                    DataColumn::new("name", "Name").with_type(DataColumnType::Title),
                    DataColumn::new("seats", "Seats").with_type(DataColumnType::Number),
                ])
                .with_rows(vec![
                    DataRow::new("r1")
                        .with_cell("name", "Figma")
                        .with_cell("seats", "24"),
                    DataRow::new("r2")
                        .with_cell("name", "Linear")
                        .with_cell("seats", "48"),
                ])
        });

        view.update(cx, |table, cx| {
            table.select_cell(DataCellPos::new(0, 0), false, cx);
            table.select_cell(DataCellPos::new(1, 1), true, cx);
            let tsv = table.copy_selection(cx).expect("selection copied");
            assert_eq!(tsv, "Figma\t24\nLinear\t48");

            table.select_cell(DataCellPos::new(1, 0), false, cx);
            table.paste_tsv("Vercel\t12\nLoom\t36", cx);
            assert_eq!(table.rows().len(), 3);
            assert_eq!(table.rows()[1].cell("name"), "Vercel");
            assert_eq!(table.rows()[2].cell("seats"), "36");
            assert!(table.can_undo());

            table.undo(cx);
            assert_eq!(table.rows().len(), 2);
            assert_eq!(table.rows()[1].cell("name"), "Linear");
            assert!(table.can_redo());

            table.redo(cx);
            assert_eq!(table.rows().len(), 3);
            assert_eq!(table.rows()[2].cell("name"), "Loom");
        });
    }
}
