//! Tactile file upload dropzone with fanning document sheets, type badges, progress, and retry.
//!
//! Renders an interactive dashed drop target with a three-sheet fanning illustration (`Slide`),
//! colored `PDF` / `CSV` / `PNG` / `JSON` file badges, live progress bars with remaining-time
//! estimates, and recoverable error rows with `Retry` and `Remove`.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_eid, pro_empty_state, pro_mix_color, pro_shimmer_text, pro_skeleton_bar,
        pro_spring_f32, pro_toast_banner, tr,
    },
    primitives::{
        control::activate,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_raw_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SHEET_PDF_LIGHT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 36 42" width="36" height="42"><g transform="translate(18 21) rotate(-10) translate(-15 -18)"><path d="M5 1h14l10 10v20a4 4 0 0 1-4 4H5a4 4 0 0 1-4-4V5a4 4 0 0 1 4-4z" fill="#ffffff" stroke="rgba(0,0,0,0.14)" stroke-width="0.95"/><path d="M19 1v6a4 4 0 0 0 4 4h6" fill="none" stroke="rgba(0,0,0,0.14)" stroke-width="0.95" stroke-linejoin="round"/><rect x="2" y="20" width="20" height="9.5" rx="2.6" fill="#f04438" fill-opacity="0.14"/><path d="M5.5 26.8v-4.2h1.6a1.3 1.3 0 0 1 0 2.6H5.5M10 26.8v-4.2h1.3a2.1 2.1 0 0 1 0 4.2H10M15.2 26.8v-4.2h2.4M15.2 24.8h1.8" fill="none" stroke="#f04438" stroke-width="1.05" stroke-linecap="round" stroke-linejoin="round"/></g></svg>"##;
const SHEET_PDF_DARK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 36 42" width="36" height="42"><g transform="translate(18 21) rotate(-10) translate(-15 -18)"><path d="M5 1h14l10 10v20a4 4 0 0 1-4 4H5a4 4 0 0 1-4-4V5a4 4 0 0 1 4-4z" fill="#232327" stroke="rgba(255,255,255,0.20)" stroke-width="0.95"/><path d="M19 1v6a4 4 0 0 0 4 4h6" fill="none" stroke="rgba(255,255,255,0.20)" stroke-width="0.95" stroke-linejoin="round"/><rect x="2" y="20" width="20" height="9.5" rx="2.6" fill="#ff6b5f" fill-opacity="0.20"/><path d="M5.5 26.8v-4.2h1.6a1.3 1.3 0 0 1 0 2.6H5.5M10 26.8v-4.2h1.3a2.1 2.1 0 0 1 0 4.2H10M15.2 26.8v-4.2h2.4M15.2 24.8h1.8" fill="none" stroke="#ff6b5f" stroke-width="1.05" stroke-linecap="round" stroke-linejoin="round"/></g></svg>"##;
const SHEET_PNG_LIGHT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 36 42" width="36" height="42"><g transform="translate(18 21) rotate(10) translate(-15 -18)"><path d="M5 1h14l10 10v20a4 4 0 0 1-4 4H5a4 4 0 0 1-4-4V5a4 4 0 0 1 4-4z" fill="#ffffff" stroke="rgba(0,0,0,0.14)" stroke-width="0.95"/><path d="M19 1v6a4 4 0 0 0 4 4h6" fill="none" stroke="rgba(0,0,0,0.14)" stroke-width="0.95" stroke-linejoin="round"/><rect x="7.5" y="20" width="20.5" height="9.5" rx="2.6" fill="#a651f0" fill-opacity="0.14"/><path d="M10.6 26.8v-4.2h1.5a1.3 1.3 0 0 1 0 2.6h-1.5M15.1 26.8v-4.2l2.6 4.2v-4.2M22.8 23.4a2.1 2.1 0 1 0 0 2.8v-1.2h-1.1" fill="none" stroke="#a651f0" stroke-width="1.05" stroke-linecap="round" stroke-linejoin="round"/></g></svg>"##;
const SHEET_PNG_DARK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 36 42" width="36" height="42"><g transform="translate(18 21) rotate(10) translate(-15 -18)"><path d="M5 1h14l10 10v20a4 4 0 0 1-4 4H5a4 4 0 0 1-4-4V5a4 4 0 0 1 4-4z" fill="#232327" stroke="rgba(255,255,255,0.20)" stroke-width="0.95"/><path d="M19 1v6a4 4 0 0 0 4 4h6" fill="none" stroke="rgba(255,255,255,0.20)" stroke-width="0.95" stroke-linejoin="round"/><rect x="7.5" y="20" width="20.5" height="9.5" rx="2.6" fill="#b98af0" fill-opacity="0.20"/><path d="M10.6 26.8v-4.2h1.5a1.3 1.3 0 0 1 0 2.6h-1.5M15.1 26.8v-4.2l2.6 4.2v-4.2M22.8 23.4a2.1 2.1 0 1 0 0 2.8v-1.2h-1.1" fill="none" stroke="#b98af0" stroke-width="1.05" stroke-linecap="round" stroke-linejoin="round"/></g></svg>"##;

/// Recognized file badge categories for [`Dropzone`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKindBadge {
    /// Portable Document Format (`PDF`, coral red).
    Pdf,
    /// Comma-separated or spreadsheet export (`CSV`, emerald green).
    Csv,
    /// Raster image (`PNG`, violet).
    Png,
    /// Structured data manifest (`JSON`, amber).
    Json,
    /// Any other file extension (`FILE`, blue/neutral).
    Other,
}

impl FileKindBadge {
    /// Infer badge category from a file name extension.
    pub fn from_filename(name: &str) -> Self {
        let ext = name
            .rsplit_once('.')
            .map(|(_, e)| e.trim().to_ascii_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "pdf" => Self::Pdf,
            "csv" | "xlsx" | "xls" => Self::Csv,
            "png" | "jpg" | "jpeg" | "webp" | "svg" => Self::Png,
            "json" | "jsonl" | "yaml" | "yml" => Self::Json,
            _ => Self::Other,
        }
    }

    /// Short uppercase badge text.
    pub fn label(self) -> &'static str {
        match self {
            Self::Pdf => "PDF",
            Self::Csv => "CSV",
            Self::Png => "PNG",
            Self::Json => "JSON",
            Self::Other => "FILE",
        }
    }

    fn colors(self, p: &ProPalette) -> (Hsla, Hsla) {
        match self {
            Self::Pdf => (p.red_soft, p.red_ink),
            Self::Csv => (p.green_soft, p.green_ink),
            Self::Png => (p.purple.opacity(0.15), p.purple),
            Self::Json => (p.amber_soft, p.amber_ink),
            Self::Other => (p.accent_soft, p.blue),
        }
    }
}

/// Lifecycle state of one upload item in [`Dropzone`].
#[derive(Clone, Debug, PartialEq)]
pub enum UploadStatus {
    /// Actively uploading (`progress` in `0.0..1.0`).
    Uploading,
    /// Completed successfully (`progress == 1.0`).
    Done,
    /// Failed or rejected with an explanatory message.
    Error(SharedString),
}

/// Single file entry managed by [`Dropzone`].
#[derive(Clone, Debug, PartialEq)]
pub struct UploadItem {
    /// Stable row identifier.
    pub id: SharedString,
    /// File name including extension.
    pub name: SharedString,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Upload progress in `0.0..=1.0`.
    pub progress: f32,
    /// Current lifecycle status.
    pub status: UploadStatus,
    /// Badge category inferred or explicitly set.
    pub kind: FileKindBadge,
}

impl UploadItem {
    /// Construct a new [`UploadItem`].
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        size_bytes: u64,
        progress: f32,
        status: UploadStatus,
    ) -> Self {
        let name = name.into();
        let kind = FileKindBadge::from_filename(&name);
        Self {
            id: id.into(),
            name,
            size_bytes,
            progress: progress.clamp(0.0, 1.0),
            status,
            kind,
        }
    }

    /// Return whether this upload has completed cleanly.
    pub fn is_done(&self) -> bool {
        self.status == UploadStatus::Done
    }

    /// Return whether this upload is currently in an error state.
    pub fn is_error(&self) -> bool {
        matches!(self.status, UploadStatus::Error(_))
    }

    /// Return whether this upload is actively transferring bytes.
    pub fn is_uploading(&self) -> bool {
        self.status == UploadStatus::Uploading
    }
}

#[derive(Clone)]
struct UploadSimTrack {
    item_id: SharedString,
    started_at: Instant,
    start_progress: f32,
    duration_secs: f32,
    fail_at: Option<(f32, SharedString)>,
}

/// Typed events emitted by [`Dropzone`].
#[derive(Clone, Debug, PartialEq)]
pub enum DropzoneEvent {
    /// Emitted whenever the upload queue changes (added, progressed to completion/error, retried, or cleared).
    Changed(Vec<UploadItem>),
    /// Emitted when one or more files are added to the queue.
    FilesAdded(Vec<UploadItem>),
    /// Emitted when a failed file upload is retried.
    Retried(SharedString),
    /// Emitted when an item is removed from the list.
    Removed(SharedString),
    /// Emitted when the queue is cleared.
    Cleared,
}

/// Tactile multi-file upload dropzone with fanning document stack, progress ETA, and retry.
pub struct Dropzone {
    id: ElementId,
    accept: Vec<SharedString>,
    max_size_bytes: u64,
    hint: Option<SharedString>,
    items: Vec<UploadItem>,
    tracks: Vec<UploadSimTrack>,
    error_shakes: Vec<(SharedString, Instant)>,
    sample_cursor: usize,
    next_seq: usize,
    area_hovered: bool,
    drag_active: bool,
    fan_slide: Slide,
    autoplay: bool,
    loading: bool,
    empty: bool,
    width_cell: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    focus: FocusHandle,
}

impl EventEmitter<DropzoneEvent> for Dropzone {}

impl Dropzone {
    /// Create a [`Dropzone`] seeded with realistic `PDF`, `CSV`, `PNG`, and `JSON` uploads.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        let items = default_upload_items();
        let tracks = vec![
            UploadSimTrack {
                item_id: "up-2".into(),
                started_at: now,
                start_progress: 0.42,
                duration_secs: 4.8,
                fail_at: None,
            },
            UploadSimTrack {
                item_id: "up-3".into(),
                started_at: now,
                start_progress: 0.18,
                duration_secs: 6.2,
                fail_at: None,
            },
        ];
        Self {
            id: id.into(),
            accept: Vec::new(),
            max_size_bytes: 25 * 1024 * 1024,
            hint: None,
            items,
            tracks,
            error_shakes: Vec::new(),
            sample_cursor: 0,
            next_seq: 10,
            area_hovered: false,
            drag_active: false,
            fan_slide: Slide::snappy(0.),
            autoplay: true,
            loading: false,
            empty: false,
            width_cell: Rc::new(Cell::new(520.)),
            toast: None,
            focus: cx.focus_handle(),
        }
    }

    /// Restrict accepted extensions (for example `[".pdf", ".csv", ".png", ".json"]`).
    pub fn with_accept(
        mut self,
        extensions: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.accept = extensions.into_iter().map(Into::into).collect();
        self
    }

    /// Configure the per-file size limit in bytes (default `25 MB`).
    pub fn with_max_size(mut self, max_size_bytes: u64) -> Self {
        self.max_size_bytes = max_size_bytes.max(1024);
        self
    }

    /// Customize the subtitle hint beneath `"Drop files here or browse"`.
    pub fn with_hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Replace the initial upload list.
    pub fn with_items(mut self, items: Vec<UploadItem>) -> Self {
        self.tracks.clear();
        self.items = items;
        self
    }

    /// Enable or disable live upload simulation ticks on mount.
    pub fn with_autoplay(mut self, autoplay: bool) -> Self {
        self.autoplay = autoplay;
        self
    }

    /// Toggle skeleton loading preview state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle forced empty queue state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        if empty {
            self.items.clear();
            self.tracks.clear();
        }
        self
    }

    /// Inspect current upload items.
    pub fn items(&self) -> &[UploadItem] {
        &self.items
    }

    /// Toggle simulated drag-over highlight and document fan-out.
    pub fn set_drag_active(&mut self, active: bool, cx: &mut Context<Self>) {
        self.drag_active = active;
        let target = if active {
            1.0
        } else if self.area_hovered {
            0.48
        } else {
            0.0
        };
        self.fan_slide.duration(target, 0.28, cx);
        cx.notify();
    }

    /// Enqueue a file and validate extension & `max_size_bytes`.
    pub fn add_file(
        &mut self,
        name: impl Into<SharedString>,
        size_bytes: u64,
        simulate_failure: bool,
        cx: &mut Context<Self>,
    ) -> UploadItem {
        let name = name.into();
        self.next_seq += 1;
        let id = SharedString::from(format!("up-{}", self.next_seq));
        self.empty = false;

        if !self.accept.is_empty() {
            let lower = name.to_ascii_lowercase();
            let allowed = self.accept.iter().any(|ext| {
                let norm = ext.trim().to_ascii_lowercase();
                if norm.starts_with('.') {
                    lower.ends_with(&norm)
                } else {
                    lower.ends_with(&format!(".{norm}"))
                }
            });
            if !allowed {
                let err_msg = SharedString::from("File type not supported");
                let rejected = UploadItem::new(
                    id.clone(),
                    name,
                    size_bytes,
                    1.0,
                    UploadStatus::Error(err_msg),
                );
                self.error_shakes.push((id, Instant::now()));
                self.items.insert(0, rejected.clone());
                cx.emit(DropzoneEvent::FilesAdded(vec![rejected.clone()]));
                cx.emit(DropzoneEvent::Changed(self.items.clone()));
                cx.notify();
                return rejected;
            }
        }

        if size_bytes > self.max_size_bytes {
            let max_label = format_bytes(self.max_size_bytes);
            let err_msg = SharedString::from(format!("Too large – max {max_label}"));
            let rejected = UploadItem::new(
                id.clone(),
                name,
                size_bytes,
                1.0,
                UploadStatus::Error(err_msg),
            );
            self.error_shakes.push((id, Instant::now()));
            self.items.insert(0, rejected.clone());
            cx.emit(DropzoneEvent::FilesAdded(vec![rejected.clone()]));
            cx.emit(DropzoneEvent::Changed(self.items.clone()));
            cx.notify();
            return rejected;
        }

        let reduced = disabled_motion(cx);
        if reduced && !simulate_failure {
            let done_item = UploadItem::new(id, name, size_bytes, 1.0, UploadStatus::Done);
            self.items.insert(0, done_item.clone());
            cx.emit(DropzoneEvent::FilesAdded(vec![done_item.clone()]));
            cx.emit(DropzoneEvent::Changed(self.items.clone()));
            cx.notify();
            return done_item;
        }

        let dur = (2.2 + (size_bytes as f32 / (4.0 * 1024.0 * 1024.0))).clamp(2.0, 5.5);
        let item = UploadItem::new(id.clone(), name, size_bytes, 0.04, UploadStatus::Uploading);
        self.tracks.push(UploadSimTrack {
            item_id: id,
            started_at: Instant::now(),
            start_progress: 0.04,
            duration_secs: dur,
            fail_at: if simulate_failure {
                Some((
                    0.56,
                    SharedString::from("Upload failed – connection interrupted"),
                ))
            } else {
                None
            },
        });
        self.items.insert(0, item.clone());
        cx.emit(DropzoneEvent::FilesAdded(vec![item.clone()]));
        cx.emit(DropzoneEvent::Changed(self.items.clone()));
        cx.notify();
        item
    }

    /// Add the next curated sample file (`PDF`, `CSV`, `PNG`, `JSON`, or simulated error).
    pub fn drop_next_sample(&mut self, cx: &mut Context<Self>) {
        let samples: [(&str, u64, bool); 5] = [
            ("Architecture-Spec-v4.pdf", 3_420_000, false),
            ("Revenue-Ledger-2026.csv", 5_180_000, false),
            ("Dashboard-Preview.png", 2_890_000, false),
            ("Agent-Workflow.json", 940_000, false),
            ("Raw-Telemetry-Dump.json", 4_120_000, true),
        ];
        let (name, bytes, fail) = samples[self.sample_cursor % samples.len()];
        self.sample_cursor = self.sample_cursor.wrapping_add(1);
        self.drag_active = false;
        self.fan_slide
            .duration(if self.area_hovered { 0.48 } else { 0.0 }, 0.26, cx);
        self.add_file(name, bytes, fail, cx);
    }

    /// Retry a failed upload item, allowing it to progress cleanly to `100%`.
    pub fn retry_item(&mut self, item_id: &str, cx: &mut Context<Self>) -> bool {
        let reduced = disabled_motion(cx);
        let Some(item) = self.items.iter_mut().find(|i| i.id.as_ref() == item_id) else {
            return false;
        };
        if !item.is_error() {
            return false;
        }
        if item.size_bytes > self.max_size_bytes {
            // Keep size-limit validation honest while shaking row feedback
            self.error_shakes.retain(|(eid, _)| eid.as_ref() != item_id);
            self.error_shakes.push((item.id.clone(), Instant::now()));
            cx.notify();
            return false;
        }
        let id_cloned = item.id.clone();
        if reduced {
            item.progress = 1.0;
            item.status = UploadStatus::Done;
        } else {
            item.progress = 0.08;
            item.status = UploadStatus::Uploading;
            self.tracks.retain(|t| t.item_id != id_cloned);
            self.tracks.push(UploadSimTrack {
                item_id: id_cloned.clone(),
                started_at: Instant::now(),
                start_progress: 0.08,
                duration_secs: 2.4,
                fail_at: None,
            });
        }
        cx.emit(DropzoneEvent::Retried(id_cloned));
        cx.emit(DropzoneEvent::Changed(self.items.clone()));
        cx.notify();
        true
    }

    /// Retry every recoverable failed upload in the list.
    pub fn retry_all_failed(&mut self, cx: &mut Context<Self>) {
        let failed_ids: Vec<String> = self
            .items
            .iter()
            .filter(|i| i.is_error() && i.size_bytes <= self.max_size_bytes)
            .map(|i| i.id.to_string())
            .collect();
        for fid in failed_ids {
            self.retry_item(&fid, cx);
        }
    }

    /// Remove a single upload row by id.
    pub fn remove_item(&mut self, item_id: &str, cx: &mut Context<Self>) -> bool {
        let before = self.items.len();
        self.items.retain(|i| i.id.as_ref() != item_id);
        self.tracks.retain(|t| t.item_id.as_ref() != item_id);
        if self.items.len() == before {
            return false;
        }
        cx.emit(DropzoneEvent::Removed(SharedString::from(
            item_id.to_string(),
        )));
        cx.emit(DropzoneEvent::Changed(self.items.clone()));
        cx.notify();
        true
    }

    /// Clear all items from the upload queue.
    pub fn clear_all(&mut self, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        self.items.clear();
        self.tracks.clear();
        cx.emit(DropzoneEvent::Cleared);
        cx.emit(DropzoneEvent::Changed(Vec::new()));
        cx.notify();
    }

    fn advance_simulations(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.autoplay && self.sample_cursor == 0 {
            return;
        }
        let reduced = disabled_motion(cx);
        let mut any_running = false;
        let mut status_transitioned = false;

        let mut remaining_tracks = Vec::with_capacity(self.tracks.len());
        for track in self.tracks.drain(..) {
            let Some(item) = self.items.iter_mut().find(|i| i.id == track.item_id) else {
                continue;
            };
            if !item.is_uploading() {
                continue;
            }
            if reduced {
                if let Some((fail_p, reason)) = track.fail_at {
                    item.progress = fail_p;
                    item.status = UploadStatus::Error(reason);
                } else {
                    item.progress = 1.0;
                    item.status = UploadStatus::Done;
                }
                status_transitioned = true;
                continue;
            }
            let elapsed = track.started_at.elapsed().as_secs_f32();
            let span = 1.0 - track.start_progress;
            let cur =
                (track.start_progress + (elapsed / track.duration_secs) * span).clamp(0.0, 1.0);
            if let Some((fail_p, reason)) = &track.fail_at
                && cur >= *fail_p
            {
                item.progress = *fail_p;
                item.status = UploadStatus::Error(reason.clone());
                self.error_shakes.push((item.id.clone(), Instant::now()));
                status_transitioned = true;
                continue;
            }
            if cur >= 0.999 {
                item.progress = 1.0;
                item.status = UploadStatus::Done;
                status_transitioned = true;
            } else {
                item.progress = cur;
                any_running = true;
                remaining_tracks.push(track);
            }
        }
        self.tracks = remaining_tracks;

        self.error_shakes
            .retain(|(_, t0)| t0.elapsed().as_secs_f32() < 0.42);
        if !reduced && (any_running || !self.error_shakes.is_empty()) {
            w.request_animation_frame();
        }
        if status_transitioned {
            cx.emit(DropzoneEvent::Changed(self.items.clone()));
        }
    }

    fn estimated_secs_left(&self, item: &UploadItem) -> u32 {
        if let Some(track) = self.tracks.iter().find(|t| t.item_id == item.id) {
            let elapsed = track.started_at.elapsed().as_secs_f32();
            (track.duration_secs - elapsed).ceil().max(1.0) as u32
        } else {
            let rem = (1.0 - item.progress).clamp(0.0, 1.0);
            (rem * 5.0).ceil().max(1.0) as u32
        }
    }

    fn render_row(
        &self,
        item: &UploadItem,
        index: usize,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let iid = item.id.clone();
        let (badge_bg, badge_fg) = item.kind.colors(&p);

        let shake_dx = if disabled_motion(cx) {
            0.0
        } else if let Some((_, started)) = self.error_shakes.iter().find(|(id, _)| id == &iid) {
            let u = (started.elapsed().as_secs_f32() / 0.38).clamp(0.0, 1.0);
            (u * std::f32::consts::TAU * 2.0).sin() * 3.0 * (1.0 - u)
        } else {
            0.0
        };

        let smooth_prog = pro_spring_f32(
            pro_eid(&self.id, format!("prog-{iid}")),
            item.progress.clamp(0.0, 1.0),
            0.22,
            w,
            cx,
        )
        .clamp(0.0, 1.0);

        let badge = div()
            .h(px(20.))
            .min_w(px(36.))
            .px(px(6.))
            .flex_none()
            .rounded(px(5.))
            .bg(badge_bg)
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(10.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(badge_fg)
            .child(item.kind.label());

        let meta_line = match &item.status {
            UploadStatus::Uploading => {
                let done_b = ((item.size_bytes as f64) * f64::from(item.progress)) as u64;
                let secs = self.estimated_secs_left(item);
                let eta =
                    tr(cx, &format!("~{secs}s left"), &format!("剩余约 {secs} 秒")).to_string();
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(format!(
                        "{} {} {}",
                        format_bytes(done_b),
                        tr(cx, "of", "/"),
                        format_bytes(item.size_bytes)
                    ))
                    .child(div().text_color(p.faint).child("·"))
                    .child(eta)
                    .into_any_element()
            }
            UploadStatus::Done => div()
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(format_bytes(item.size_bytes))
                .child(div().text_color(p.faint).child("·"))
                .child(
                    div()
                        .text_color(p.green_ink)
                        .child(tr(cx, "Uploaded", "上传完成")),
                )
                .into_any_element(),
            UploadStatus::Error(reason) => div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.))
                .child(
                    div()
                        .text_color(p.muted)
                        .child(format_bytes(item.size_bytes)),
                )
                .child(div().text_color(p.faint).child("·"))
                .child(div().text_color(p.red_ink).child(reason.clone()))
                .into_any_element(),
        };

        let show_bar = !item.is_done();
        let bar_color = if item.is_error() { p.red } else { p.blue };

        let can_retry = item.is_error() && item.size_bytes <= self.max_size_bytes;
        let retry_btn = if can_retry {
            let iid_retry = iid.clone();
            Some(activate(
                div()
                    .id(pro_eid(&self.id, format!("retry-{iid}")))
                    .role(Role::Button)
                    .aria_label(format!("{} {}", tr(cx, "Retry", "重试"), item.name))
                    .tab_index(0)
                    .h(px(26.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.chip_hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .child(pro_icon(ProIcon::Refresh, 12.5, p.ink))
                    .child(tr(cx, "Retry", "重试")),
                cx,
                move |this, _, cx| {
                    this.retry_item(&iid_retry, cx);
                },
            ))
        } else {
            None
        };

        let iid_rm = iid.clone();
        let remove_btn = activate(
            div()
                .id(pro_eid(&self.id, format!("rm-{iid}")))
                .role(Role::Button)
                .aria_label(format!("{} {}", tr(cx, "Remove", "移除"), item.name))
                .tab_index(0)
                .size(px(28.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(pro_icon(ProIcon::Close, 14., p.muted)),
            cx,
            move |this, _, cx| {
                this.remove_item(&iid_rm, cx);
            },
        );

        div()
            .id(pro_eid(&self.id, format!("dz-row-{iid}")))
            .w_full()
            .when(index > 0, |d| d.border_t_1().border_color(p.line))
            .py(px(9.))
            .pl(px(8.))
            .pr(px(4.))
            .relative()
            .left(px(shake_dx))
            .flex()
            .items_center()
            .gap(px(10.))
            .child(badge)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .min_w_0()
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(item.name.clone()),
                            )
                            .when(item.is_done(), |d| {
                                d.child(pro_icon(ProIcon::Check, 14., p.green))
                            }),
                    )
                    .child(meta_line)
                    .when(show_bar, |d| {
                        d.child(
                            div()
                                .mt(px(4.))
                                .w_full()
                                .h(px(3.))
                                .rounded(px(1.5))
                                .bg(p.chip)
                                .overflow_hidden()
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative(smooth_prog))
                                        .rounded(px(1.5))
                                        .bg(bar_color),
                                ),
                        )
                    }),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .children(retry_btn)
                    .child(remove_btn),
            )
            .into_any_element()
    }
}

impl Render for Dropzone {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.advance_simulations(w, cx);
        let p = ProPalette::get(cx);
        let width_cell = self.width_cell.clone();
        let narrow = width_cell.get() < 420.;

        let sensor = canvas(
            move |bounds, _, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 10. {
                    width_cell.set(bw);
                }
            },
            |_, _, _, _| (),
        )
        .w_full()
        .h(px(0.));

        if self.loading {
            let phase = if disabled_motion(cx) { 0.3 } else { 0.5 };
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(520.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(10.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(pro_skeleton_bar(500., 132., 10., phase, cx))
                .child(pro_skeleton_bar(480., 46., 6., phase, cx))
                .child(pro_skeleton_bar(480., 46., 6., phase, cx))
                .into_any_element();
        }

        let fan = self.fan_slide.get(w, cx).clamp(0.0, 1.2);
        let left_x = -13.0 - 15.0 * fan;
        let right_x = 13.0 + 15.0 * fan;
        let mid_y = -2.0 - 6.0 * fan;
        let side_y = 2.0 - 2.0 * fan;

        let left_svg = if p.dark {
            SHEET_PDF_DARK
        } else {
            SHEET_PDF_LIGHT
        };
        let right_svg = if p.dark {
            SHEET_PNG_DARK
        } else {
            SHEET_PNG_LIGHT
        };

        let upload_glyph_color = if self.drag_active {
            p.blue
        } else {
            pro_mix_color(p.muted, p.ink, (fan * 1.6).clamp(0.0, 1.0))
        };

        let fanning_stack = div()
            .w(px(92.))
            .h(px(46.))
            .mb(px(10.))
            .relative()
            .child(
                div()
                    .absolute()
                    .left(px(28. + left_x))
                    .top(px(4. + side_y))
                    .child(pro_raw_svg(left_svg, 36., 42.)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(28. + right_x))
                    .top(px(4. + side_y))
                    .child(pro_raw_svg(right_svg, 36., 42.)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(31.))
                    .top(px(6. + mid_y))
                    .w(px(30.))
                    .h(px(36.))
                    .rounded(px(6.))
                    .bg(p.card)
                    .border_1()
                    .border_color(if self.drag_active {
                        p.blue
                    } else {
                        p.line_strong
                    })
                    .when(!p.dark, |d| d.shadow_xs())
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Upload, 15.5, upload_glyph_color)),
            );

        let dash_color = if self.drag_active {
            p.blue
        } else if self.area_hovered {
            if p.dark {
                hsla(0., 0., 1., 0.26)
            } else {
                hsla(0., 0., 0., 0.22)
            }
        } else if p.dark {
            hsla(0., 0., 1., 0.16)
        } else {
            hsla(0., 0., 0., 0.13)
        };
        let solid_drag = self.drag_active;

        let dashed_outline = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                if solid_drag {
                    window.paint_quad(quad(
                        bounds,
                        Corners::all(px(10.)),
                        transparent_black(),
                        Edges::all(px(1.5)),
                        dash_color,
                        BorderStyle::Solid,
                    ));
                    return;
                }
                let x0 = f32::from(bounds.left()) + 0.5;
                let y0 = f32::from(bounds.top()) + 0.5;
                let bw = (f32::from(bounds.size.width) - 1.0).max(4.0);
                let bh = (f32::from(bounds.size.height) - 1.0).max(4.0);
                let inset = 8.0_f32;
                let dash_len = 5.0_f32;
                let gap_len = 4.0_f32;
                let step = dash_len + gap_len;

                let mut x = x0 + inset;
                while x < x0 + bw - inset {
                    let end = (x + dash_len).min(x0 + bw - inset);
                    let mut top_seg = PathBuilder::stroke(px(1.0));
                    top_seg.move_to(point(px(x), px(y0)));
                    top_seg.line_to(point(px(end), px(y0)));
                    if let Ok(path) = top_seg.build() {
                        window.paint_path(path, dash_color);
                    }
                    let mut bot_seg = PathBuilder::stroke(px(1.0));
                    bot_seg.move_to(point(px(x), px(y0 + bh)));
                    bot_seg.line_to(point(px(end), px(y0 + bh)));
                    if let Ok(path) = bot_seg.build() {
                        window.paint_path(path, dash_color);
                    }
                    x += step;
                }

                let mut y = y0 + inset;
                while y < y0 + bh - inset {
                    let end = (y + dash_len).min(y0 + bh - inset);
                    let mut left_seg = PathBuilder::stroke(px(1.0));
                    left_seg.move_to(point(px(x0), px(y)));
                    left_seg.line_to(point(px(x0), px(end)));
                    if let Ok(path) = left_seg.build() {
                        window.paint_path(path, dash_color);
                    }
                    let mut right_seg = PathBuilder::stroke(px(1.0));
                    right_seg.move_to(point(px(x0 + bw), px(y)));
                    right_seg.line_to(point(px(x0 + bw), px(end)));
                    if let Ok(path) = right_seg.build() {
                        window.paint_path(path, dash_color);
                    }
                    y += step;
                }

                // Crisp quarter-circle corner arcs
                let r = inset;
                let corners = [
                    (
                        x0 + r,
                        y0 + r,
                        std::f32::consts::PI,
                        std::f32::consts::FRAC_PI_2 * 3.0,
                    ),
                    (
                        x0 + bw - r,
                        y0 + r,
                        std::f32::consts::FRAC_PI_2 * 3.0,
                        std::f32::consts::TAU,
                    ),
                    (x0 + bw - r, y0 + bh - r, 0.0, std::f32::consts::FRAC_PI_2),
                    (
                        x0 + r,
                        y0 + bh - r,
                        std::f32::consts::FRAC_PI_2,
                        std::f32::consts::PI,
                    ),
                ];
                for (cx_pt, cy_pt, a0, a1) in corners {
                    let mut arc = PathBuilder::stroke(px(1.0));
                    for s in 0..=4 {
                        let a = a0 + (a1 - a0) * (s as f32 / 4.0);
                        let pt = point(px(cx_pt + r * a.cos()), px(cy_pt + r * a.sin()));
                        if s == 0 {
                            arc.move_to(pt);
                        } else {
                            arc.line_to(pt);
                        }
                    }
                    if let Ok(path) = arc.build() {
                        window.paint_path(path, dash_color);
                    }
                }
            },
        )
        .absolute()
        .inset_0();

        let hint_str = match &self.hint {
            Some(h) => h.to_string(),
            None => {
                let max_str = format_bytes(self.max_size_bytes);
                tr(
                    cx,
                    &format!("PDF, CSV, PNG, JSON · up to {max_str} each"),
                    &format!("支持 PDF、CSV、PNG、JSON · 单文件上限 {max_str}"),
                )
                .to_string()
            }
        };

        let drop_area = activate(
            div()
                .id(pro_eid(&self.id, "drop-area"))
                .role(Role::Button)
                .aria_label(tr(
                    cx,
                    "Drop files here or browse to add a sample upload",
                    "拖拽文件到此处或点击浏览添加示例上传文件",
                ))
                .tab_index(0)
                .w_full()
                .py(if narrow { px(18.) } else { px(22.) })
                .px(if narrow { px(12.) } else { px(16.) })
                .relative()
                .rounded(px(10.))
                .bg(if self.drag_active {
                    p.accent_soft
                } else if self.area_hovered {
                    p.hover.opacity(0.6)
                } else {
                    p.card
                })
                .cursor_pointer()
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_hover(cx.listener(|this, inside: &bool, w, cx| {
                    this.area_hovered = *inside;
                    let target = if this.drag_active {
                        1.0
                    } else if *inside {
                        0.48
                    } else {
                        0.0
                    };
                    this.fan_slide.duration(target, 0.28, cx);
                    cx.notify();
                    w.refresh();
                }))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(dashed_outline)
                .child(fanning_stack)
                .child(if self.drag_active {
                    div()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.blue)
                        .child(tr(cx, "Release to drop files", "松开即可上传文件"))
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_center()
                        .gap(px(4.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(tr(cx, "Drop files here or", "拖拽文件到此处或"))
                        .child(div().text_color(p.blue).child(tr(cx, "browse", "点击浏览")))
                        .into_any_element()
                })
                .child(
                    div()
                        .mt(px(2.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(hint_str),
                ),
            cx,
            |this, _, cx| {
                this.drop_next_sample(cx);
            },
        );

        // Tactile quick-drop chips for testing specific file badges & drag fan-out
        let quick_samples: [(&str, &str, u64, bool); 4] = [
            ("pdf", "+ PDF", 2_640_000, false),
            ("csv", "+ CSV", 4_350_000, false),
            ("png", "+ PNG", 3_120_000, false),
            ("json", "+ JSON", 1_180_000, false),
        ];
        let mut quick_strip = div()
            .w_full()
            .pt(px(6.))
            .px(px(4.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(6.));

        let mut badge_adders = div().flex().flex_wrap().items_center().gap(px(4.));
        for (ext, label, sz, fail) in quick_samples {
            let fname = match ext {
                "pdf" => "Contract-Addendum.pdf",
                "csv" => "Usage-Export-Q4.csv",
                "png" => "Brand-Keynote.png",
                _ => "Pipeline-Config.json",
            };
            badge_adders = badge_adders.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("qadd-{ext}")))
                    .role(Role::Button)
                    .aria_label(format!("Add {ext} file"))
                    .tab_index(0)
                    .h(px(22.))
                    .px(px(7.))
                    .rounded(px(5.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text)
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.chip_hover).text_color(p.ink))
                    .child(label),
                cx,
                move |this, _, cx| {
                    this.add_file(fname, sz, fail, cx);
                },
            ));
        }

        let drag_toggle_active = self.drag_active;
        let drag_sim_btn = activate(
            div()
                .id(pro_eid(&self.id, "sim-drag"))
                .role(Role::Button)
                .aria_label(tr(cx, "Preview drag state", "预览拖拽悬停态"))
                .tab_index(0)
                .h(px(22.))
                .px(px(8.))
                .rounded(px(5.))
                .bg(if drag_toggle_active {
                    p.accent_soft
                } else {
                    p.chip
                })
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if drag_toggle_active { p.blue } else { p.muted })
                .cursor_pointer()
                .hover(move |s| s.bg(p.chip_hover).text_color(p.ink))
                .child(tr(
                    cx,
                    if drag_toggle_active {
                        "Dragging active"
                    } else {
                        "Preview drag"
                    },
                    if drag_toggle_active {
                        "拖拽态已开启"
                    } else {
                        "预览拖拽态"
                    },
                )),
            cx,
            |this, _, cx| {
                let next = !this.drag_active;
                this.set_drag_active(next, cx);
            },
        );
        quick_strip = quick_strip.child(badge_adders).child(drag_sim_btn);

        let items_snapshot = self.items.clone();
        let rows_section = if self.empty {
            pro_empty_state(
                ProIcon::Upload,
                tr(cx, "No uploads yet", "暂无上传文件"),
                tr(
                    cx,
                    "Click the dropzone above or pick a file type chip to start uploading.",
                    "点击上方拖拽区域或文件类型快捷标签开始上传。",
                ),
                Some((
                    pro_eid(&self.id, "empty-drop"),
                    SharedString::from(tr(cx, "Drop sample files", "填入示例文件")),
                    Box::new(|this: &mut Self, _, cx| {
                        this.empty = false;
                        this.items = default_upload_items();
                        cx.notify();
                    }),
                )),
                cx,
            )
            .into_any_element()
        } else if items_snapshot.is_empty() {
            div().into_any_element()
        } else {
            let mut list = div()
                .id(pro_eid(&self.id, "upload-list"))
                .mt(px(4.))
                .w_full()
                .max_h(px(320.))
                .overflow_y_scroll()
                .flex()
                .flex_col();
            for (idx, item) in items_snapshot.iter().enumerate() {
                list = list.child(self.render_row(item, idx, w, cx));
            }
            list.into_any_element()
        };

        let footer = if !items_snapshot.is_empty() {
            let total_count = items_snapshot.len();
            let uploading_count = items_snapshot.iter().filter(|i| i.is_uploading()).count();
            let done_count = items_snapshot.iter().filter(|i| i.is_done()).count();
            let error_count = items_snapshot.iter().filter(|i| i.is_error()).count();
            let total_bytes: u64 = items_snapshot.iter().map(|i| i.size_bytes).sum();
            let transferred_bytes: u64 = items_snapshot
                .iter()
                .map(|i| {
                    if i.is_error() {
                        0
                    } else {
                        ((i.size_bytes as f64) * f64::from(i.progress)) as u64
                    }
                })
                .sum();

            let summary_el = if uploading_count > 0 {
                let active_label = tr(
                    cx,
                    &format!("Uploading {uploading_count} of {total_count}"),
                    &format!("正在上传 {uploading_count} / {total_count}"),
                )
                .to_string();
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .min_w_0()
                    .child(pro_shimmer_text(
                        pro_eid(&self.id, "foot-shim"),
                        active_label,
                        p.text,
                        p.strong,
                        12.,
                        FontWeight::MEDIUM,
                        true,
                    ))
                    .child(div().text_size(px(12.)).text_color(p.faint).child("·"))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .truncate()
                            .child(format!(
                                "{} {} {}",
                                format_bytes(transferred_bytes),
                                tr(cx, "of", "/"),
                                format_bytes(total_bytes)
                            )),
                    )
                    .into_any_element()
            } else if error_count > 0 {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .min_w_0()
                    .text_size(px(12.))
                    .child(pro_icon(ProIcon::AlertTriangle, 14., p.amber))
                    .child(
                        div().text_color(p.ink).truncate().child(
                            tr(
                                cx,
                                &format!("{done_count} uploaded · {error_count} failed"),
                                &format!("已完成 {done_count} 个 · 失败 {error_count} 个"),
                            )
                            .to_string(),
                        ),
                    )
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .min_w_0()
                    .text_size(px(12.))
                    .child(pro_icon(ProIcon::Check, 14., p.green))
                    .child(
                        div()
                            .text_color(p.ink)
                            .font_weight(FontWeight::MEDIUM)
                            .child(
                                tr(
                                    cx,
                                    &format!("All {done_count} files uploaded"),
                                    &format!("全部 {done_count} 个文件已上传"),
                                )
                                .to_string(),
                            ),
                    )
                    .child(div().text_color(p.faint).child("·"))
                    .child(div().text_color(p.muted).child(format_bytes(total_bytes)))
                    .into_any_element()
            };

            let recoverable = items_snapshot
                .iter()
                .any(|i| i.is_error() && i.size_bytes <= self.max_size_bytes);

            div()
                .w_full()
                .h(px(44.))
                .pl(if narrow { px(12.) } else { px(16.) })
                .pr(px(8.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(summary_el)
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .when(recoverable, |d| {
                            d.child(activate(
                                div()
                                    .id(pro_eid(&self.id, "foot-retry-all"))
                                    .role(Role::Button)
                                    .aria_label(tr(cx, "Retry failed", "全部重试"))
                                    .tab_index(0)
                                    .h(px(28.))
                                    .px(px(10.))
                                    .rounded(px(6.))
                                    .bg(p.accent_soft)
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.blue)
                                    .cursor_pointer()
                                    .hover(move |s| s.opacity(0.85))
                                    .child(tr(cx, "Retry failed", "重试失败项")),
                                cx,
                                |this, _, cx| {
                                    this.retry_all_failed(cx);
                                },
                            ))
                        })
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "foot-clear"))
                                .role(Role::Button)
                                .aria_label(tr(cx, "Clear all", "清空列表"))
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(10.))
                                .rounded(px(6.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover).border_color(p.line_strong))
                                .child(tr(cx, "Clear", "清空")),
                            cx,
                            |this, _, cx| {
                                this.clear_all(cx);
                            },
                        )),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .child(sensor)
            .child(
                div()
                    .w_full()
                    .max_w(px(520.))
                    .mx_auto()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_xs())
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .p(px(8.))
                            .pb(px(6.))
                            .flex()
                            .flex_col()
                            .child(drop_area)
                            .child(quick_strip)
                            .child(rows_section),
                    )
                    .child(footer),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

fn format_bytes(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= MB {
        let m = b / MB;
        if (m.fract()).abs() < 0.05 {
            format!("{m:.0} MB")
        } else {
            format!("{m:.1} MB")
        }
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

fn default_upload_items() -> Vec<UploadItem> {
    vec![
        UploadItem::new(
            "up-1",
            "Q3-Board-Report.pdf",
            2_516_582,
            1.0,
            UploadStatus::Done,
        ),
        UploadItem::new(
            "up-2",
            "Customer-Cohort-Export.csv",
            5_033_164,
            0.42,
            UploadStatus::Uploading,
        ),
        UploadItem::new(
            "up-3",
            "Hero-Product-Shot.png",
            3_250_585,
            0.18,
            UploadStatus::Uploading,
        ),
        UploadItem::new(
            "up-4",
            "Schema-Manifest.json",
            1_258_291,
            0.54,
            UploadStatus::Error("Upload failed – connection lost".into()),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{Dropzone, FileKindBadge, UploadStatus, format_bytes};
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn badge_classification_and_byte_formatting_match_spec() {
        assert_eq!(
            FileKindBadge::from_filename("Q3-Report.pdf"),
            FileKindBadge::Pdf
        );
        assert_eq!(
            FileKindBadge::from_filename("cohort.CSV"),
            FileKindBadge::Csv
        );
        assert_eq!(FileKindBadge::from_filename("shot.png"), FileKindBadge::Png);
        assert_eq!(
            FileKindBadge::from_filename("schema.json"),
            FileKindBadge::Json
        );
        assert_eq!(format_bytes(25 * 1024 * 1024), "25 MB");
    }

    #[gpui::test]
    fn validation_retry_and_removal_update_upload_queue(cx: &mut TestAppContext) {
        let dz = cx.new(|cx| {
            Dropzone::new("test-dropzone", cx)
                .with_accept([".pdf", ".json"])
                .with_max_size(10 * 1024 * 1024)
        });

        dz.update(cx, |d, cx| {
            assert_eq!(d.items().len(), 4);

            // Retry the default failed JSON row
            assert!(d.retry_item("up-4", cx));
            let retried = d
                .items()
                .iter()
                .find(|i| i.id == "up-4")
                .expect("up-4 exists");
            assert!(matches!(
                retried.status,
                UploadStatus::Uploading | UploadStatus::Done
            ));

            // Unsupported extension gets rejected immediately
            let rejected_ext = d.add_file("photo.png", 1_024_000, false, cx);
            assert!(rejected_ext.is_error());

            // Oversized file gets rejected and cannot be retried past size limit
            let oversized = d.add_file("huge.pdf", 20 * 1024 * 1024, false, cx);
            assert!(oversized.is_error());
            assert!(!d.retry_item(&oversized.id, cx));

            // Remove item & clear all
            assert!(d.remove_item("up-1", cx));
            d.clear_all(cx);
            assert!(d.items().is_empty());
        });
    }
}
