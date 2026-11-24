//! Grouped, editable assistant memory panel with master switch, category filters, inline Editor, and Undo tray.
//!
//! Displays cross-chat assistant memories grouped by category (`Preferences`, `Work`, `Projects`,
//! `People`), with a master `Use memory` capsule switch, `pro_pill_indicator` category filter chips,
//! inline search, per-row Edit / Pause / Forget actions, `+ New memory` composer, and a timed Undo tray.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, ProSelectionSensor, pro_btn, pro_check_draw, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator,
        pro_presence, pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_f32,
        pro_swap_label, pro_switch, pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const EDIT_PENCIL_SVG: &str = r#"<path d="M14.5 4.5l5 5M4 20l1.2-5.5L15.8 3.9a2.1 2.1 0 0 1 3 0l1.3 1.3a2.1 2.1 0 0 1 0 3L9.5 18.8 4 20Z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#;
const INFO_CIRCLE_SVG: &str = r#"<circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="1.8"/><path d="M12 11v5M12 7.8h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>"#;

/// Single remembered fact managed inside [`MemoryPanel`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryEntry {
    /// Stable memory identifier.
    pub id: SharedString,
    /// Editable memory statement.
    pub text: SharedString,
    /// Category group name (for example `"Preferences"`, `"Work"`, `"Projects"`, `"People"`).
    pub category: SharedString,
    /// Provenance label (for example `"From chat"`, `"From Figma"`, `"You added"`).
    pub source: SharedString,
    /// Formatted date label (for example `"Oct 12"`).
    pub date: SharedString,
    /// Whether this memory is temporarily paused (kept in list, excluded from active prompt).
    pub paused: bool,
}

impl MemoryEntry {
    /// Construct a [`MemoryEntry`] with `id`, `text`, `category`, `source`, and `date`.
    pub fn new(
        id: impl Into<SharedString>,
        text: impl Into<SharedString>,
        category: impl Into<SharedString>,
        source: impl Into<SharedString>,
        date: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            category: category.into(),
            source: source.into(),
            date: date.into(),
            paused: false,
        }
    }

    /// Start this memory in the paused state.
    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }
}

/// Typed events emitted by [`MemoryPanel`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryPanelEvent {
    /// Master `Use memory` switch was flipped.
    EnabledChanged(bool),
    /// Active category filter changed (`None` means `"All"`).
    CategoryChanged(Option<SharedString>),
    /// A new memory was added via `+ New memory`.
    Added(MemoryEntry),
    /// An existing memory's text was edited and saved.
    Edited {
        id: SharedString,
        text: SharedString,
    },
    /// A memory was paused or resumed.
    Paused { id: SharedString, paused: bool },
    /// A memory was forgotten (can still be restored during the Undo window).
    Forgotten(SharedString),
    /// The last forgotten memory was restored via `Undo`.
    Restored(MemoryEntry),
    /// Full updated memory list after any mutation.
    Changed(Vec<MemoryEntry>),
}

#[derive(Clone)]
struct ForgottenRecord {
    entry: MemoryEntry,
    original_index: usize,
    forgotten_at: Instant,
}

fn default_memories() -> Vec<MemoryEntry> {
    vec![
        MemoryEntry::new(
            "short-answers",
            "Likes short answers first, details only when asked",
            "Preferences",
            "You added",
            "Oct 2",
        )
        .with_paused(true),
        MemoryEntry::new(
            "grid-inter",
            "Designs with Inter and a 4px spacing grid",
            "Preferences",
            "From Figma",
            "Oct 7",
        ),
        MemoryEntry::new(
            "role-studio",
            "Product designer and front-end developer at a small studio",
            "Work",
            "From chat",
            "Sep 28",
        ),
        MemoryEntry::new(
            "stack-vercel",
            "Ships with React, Vite and Tailwind; deploys on Vercel",
            "From chat",
            "From chat",
            "Oct 3",
        ),
        MemoryEntry::new(
            "pr-morning",
            "Reviews pull requests in the morning before standup",
            "Work",
            "From GitHub",
            "Oct 9",
        ),
        MemoryEntry::new(
            "fintech-flow",
            "Redesigning a fintech onboarding flow, due Oct 24",
            "Projects",
            "From chat",
            "Oct 11",
        ),
        MemoryEntry::new(
            "tokens-clean",
            "Migrating component tokens to clean neutral surfaces",
            "Projects",
            "From Linear",
            "Oct 14",
        ),
    ]
    .into_iter()
    .map(|mut m| {
        if m.id == "stack-vercel" {
            m.category = "Work".into();
        }
        m
    })
    .collect()
}

fn default_categories() -> Vec<SharedString> {
    vec![
        "Preferences".into(),
        "Work".into(),
        "Projects".into(),
        "People".into(),
    ]
}

/// Grouped and editable assistant memory manager with master switch, category pills, and Undo tray.
pub struct MemoryPanel {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    enabled: bool,
    categories: Vec<SharedString>,
    selected_category: Option<SharedString>,
    memories: Vec<MemoryEntry>,
    searching: bool,
    search_query: String,
    search_editor: Option<Entity<Editor>>,
    editing_id: Option<SharedString>,
    expanded_id: Option<SharedString>,
    row_editor: Option<Entity<Editor>>,
    row_editor_seeded: Option<SharedString>,
    adding_new: bool,
    new_category: SharedString,
    new_editor: Option<Entity<Editor>>,
    hovered_row: Option<SharedString>,
    forgotten: Option<ForgottenRecord>,
    entering_clocks: HashMap<SharedString, (Slide, PhaseClock)>,
    cat_sensor: ProSelectionSensor,
    new_cat_sensor: ProSelectionSensor,
    undo_secs: f32,
    added_source: SharedString,
    max_height: f32,
    added_seq: usize,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    loading: bool,
    empty: bool,
}

impl MemoryPanel {
    /// Create a [`MemoryPanel`] preloaded with 7 grouped memories (`Preferences`, `Work`, `Projects`).
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Memory".into(),
            enabled: true,
            categories: default_categories(),
            selected_category: None,
            memories: default_memories(),
            searching: false,
            search_query: String::new(),
            search_editor: None,
            editing_id: None,
            expanded_id: None,
            row_editor: None,
            row_editor_seeded: None,
            adding_new: false,
            new_category: "Preferences".into(),
            new_editor: None,
            hovered_row: None,
            forgotten: None,
            entering_clocks: HashMap::new(),
            cat_sensor: pro_selection_sensor(),
            new_cat_sensor: pro_selection_sensor(),
            undo_secs: 5.5,
            added_source: "You added".into(),
            max_height: 356.0,
            added_seq: 0,
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(400.0)),
            loading: false,
            empty: false,
        }
    }

    /// Override the panel heading (defaults to `"Memory"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the initial master `Use memory` switch state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Supply a custom list of [`MemoryEntry`] items.
    pub fn with_memories(mut self, memories: Vec<MemoryEntry>) -> Self {
        self.memories = memories;
        self
    }

    /// Supply ordered category names for filter chips and the `+ New memory` picker.
    pub fn with_categories<I, S>(mut self, categories: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        let list: Vec<SharedString> = categories.into_iter().map(Into::into).collect();
        if let Some(first) = list.first() {
            self.new_category = first.clone();
        }
        self.categories = list;
        self
    }

    /// Override the scrollable list max height in pixels (defaults to `356.0`).
    pub fn with_max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height.max(180.0);
        self
    }

    /// Override how long `Undo` remains active after forgetting a memory, in seconds.
    pub fn with_undo_seconds(mut self, seconds: f32) -> Self {
        self.undo_secs = seconds.max(1.5);
        self
    }

    /// Toggle loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Whether the master `Use memory` switch is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Current slice of stored memories.
    pub fn memories(&self) -> &[MemoryEntry] {
        &self.memories
    }

    /// Number of paused memories.
    pub fn paused_count(&self) -> usize {
        self.memories.iter().filter(|m| m.paused).count()
    }

    /// Flip or set the master `Use memory` switch.
    pub fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.enabled != enabled {
            self.enabled = enabled;
            cx.emit(MemoryPanelEvent::EnabledChanged(enabled));
            cx.notify();
        }
    }

    /// Select a category filter (`None` selects `"All"`).
    pub fn select_category(&mut self, category: Option<SharedString>, cx: &mut Context<Self>) {
        if self.selected_category != category {
            if let Some(cat) = &category {
                self.new_category = cat.clone();
            }
            self.selected_category = category.clone();
            cx.emit(MemoryPanelEvent::CategoryChanged(category));
            cx.notify();
        }
    }

    /// Toggle pause state on a memory by `id`.
    pub fn toggle_pause(&mut self, id: &str, cx: &mut Context<Self>) -> Option<bool> {
        let m = self.memories.iter_mut().find(|m| m.id.as_ref() == id)?;
        m.paused = !m.paused;
        let paused = m.paused;
        cx.emit(MemoryPanelEvent::Paused {
            id: id.to_owned().into(),
            paused,
        });
        cx.emit(MemoryPanelEvent::Changed(self.memories.clone()));
        cx.notify();
        Some(paused)
    }

    /// Save edited text onto an existing memory by `id`.
    pub fn save_edit(
        &mut self,
        id: &str,
        new_text: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        let trimmed: SharedString = new_text.into().trim().to_owned().into();
        if trimmed.is_empty() {
            return false;
        }
        let Some(m) = self.memories.iter_mut().find(|m| m.id.as_ref() == id) else {
            return false;
        };
        m.text = trimmed.clone();
        self.editing_id = None;
        self.row_editor_seeded = None;
        cx.emit(MemoryPanelEvent::Edited {
            id: id.to_owned().into(),
            text: trimmed,
        });
        cx.emit(MemoryPanelEvent::Changed(self.memories.clone()));
        cx.notify();
        true
    }

    /// Add a new memory to `category` with `text`.
    pub fn add_memory(
        &mut self,
        text: impl Into<SharedString>,
        category: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Option<MemoryEntry> {
        let clean = text.into().trim().to_owned();
        if clean.is_empty() {
            return None;
        }
        let cat: SharedString = category.into();
        if !self.categories.contains(&cat) {
            self.categories.push(cat.clone());
        }
        self.added_seq += 1;
        let entry = MemoryEntry::new(
            format!("mem-new-{}", self.added_seq),
            clean,
            cat,
            self.added_source.clone(),
            "Today",
        );
        let mut slide = Slide::new(0.0);
        slide.duration(1.0, 0.26, cx);
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        self.entering_clocks
            .insert(entry.id.clone(), (slide, clock));
        self.memories.insert(0, entry.clone());
        self.adding_new = false;
        cx.emit(MemoryPanelEvent::Added(entry.clone()));
        cx.emit(MemoryPanelEvent::Changed(self.memories.clone()));
        cx.notify();
        Some(entry)
    }

    /// Forget a memory by `id`, storing it in the Undo slot so `undo_forget` can bring it back.
    pub fn forget(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let Some(idx) = self.memories.iter().position(|m| m.id.as_ref() == id) else {
            return false;
        };
        let entry = self.memories.remove(idx);
        if self.editing_id.as_deref() == Some(id) {
            self.editing_id = None;
            self.row_editor_seeded = None;
        }
        if self.expanded_id.as_deref() == Some(id) {
            self.expanded_id = None;
        }
        self.entering_clocks.remove(id);
        self.forgotten = Some(ForgottenRecord {
            entry,
            original_index: idx,
            forgotten_at: Instant::now(),
        });
        cx.emit(MemoryPanelEvent::Forgotten(id.to_owned().into()));
        cx.emit(MemoryPanelEvent::Changed(self.memories.clone()));
        cx.notify();
        true
    }

    /// Restore the most recently forgotten memory at its original index.
    pub fn undo_forget(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(rec) = self.forgotten.take() else {
            return false;
        };
        let mut slide = Slide::new(0.0);
        slide.duration(1.0, 0.26, cx);
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        self.entering_clocks
            .insert(rec.entry.id.clone(), (slide, clock));
        let insert_at = rec.original_index.min(self.memories.len());
        self.memories.insert(insert_at, rec.entry.clone());
        cx.emit(MemoryPanelEvent::Restored(rec.entry));
        cx.emit(MemoryPanelEvent::Changed(self.memories.clone()));
        cx.notify();
        true
    }

    fn effective_categories(&self) -> Vec<SharedString> {
        let mut out = self.categories.clone();
        for m in &self.memories {
            if !out.contains(&m.category) {
                out.push(m.category.clone());
            }
        }
        out
    }

    fn ensure_search_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Entity<Editor> {
        if let Some(ed) = &self.search_editor {
            return ed.clone();
        }
        let ph: SharedString = tr(cx, "Search memories…", "搜索记忆条目…").into();
        let ed = cx.new(|cx| {
            let mut e = Editor::new(ph.clone(), InputMode::Search, w, cx);
            e.set_placeholder(ph);
            e.font_size = 13.0;
            e
        });
        cx.subscribe_in(&ed, w, |this, ent, ev: &EditEvent, w, cx| match ev {
            EditEvent::Changed => {
                this.search_query = ent.read(cx).value().to_owned();
                cx.notify();
            }
            EditEvent::Escape => {
                this.searching = false;
                this.search_query.clear();
                ent.update(cx, |e, cx| e.set_value("", w, cx));
                this.focus.focus(w, cx);
                cx.notify();
            }
            _ => {}
        })
        .detach();
        self.search_editor = Some(ed.clone());
        ed
    }

    fn ensure_row_editor(
        &mut self,
        seed_id: &SharedString,
        seed_text: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<Editor> {
        let ph: SharedString = tr(cx, "Edit memory…", "编辑记忆内容…").into();
        if self.row_editor.is_none() {
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 13.0;
                e
            });
            cx.subscribe_in(&ed, w, |this, ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Submitted => {
                    if let Some(id) = this.editing_id.clone() {
                        let v = ent.read(cx).value().to_owned();
                        this.save_edit(id.as_ref(), v, cx);
                        this.focus.focus(w, cx);
                    }
                }
                EditEvent::Escape => {
                    this.editing_id = None;
                    this.row_editor_seeded = None;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            cx.subscribe_in(&ed, w, |this, ent, ev: &EditorShortcutEvent, w, cx| {
                let EditorShortcutEvent::CommandSubmit = ev;
                if let Some(id) = this.editing_id.clone() {
                    let v = ent.read(cx).value().to_owned();
                    this.save_edit(id.as_ref(), v, cx);
                    this.focus.focus(w, cx);
                }
            })
            .detach();
            self.row_editor = Some(ed);
        }
        let ed = self.row_editor.as_ref().expect("initialized above").clone();
        if self.row_editor_seeded.as_ref() != Some(seed_id) {
            ed.update(cx, |e, cx| {
                e.set_value(seed_text, w, cx);
                e.focus.focus(w, cx);
            });
            self.row_editor_seeded = Some(seed_id.clone());
        }
        ed
    }

    fn ensure_new_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Entity<Editor> {
        if let Some(ed) = &self.new_editor {
            return ed.clone();
        }
        let ph: SharedString = tr(
            cx,
            "What should the assistant remember?",
            "希望助手记住什么偏好或背景？",
        )
        .into();
        let ed = cx.new(|cx| {
            let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
            e.set_placeholder(ph);
            e.set_command_submit(true);
            e.font_size = 13.0;
            e
        });
        cx.subscribe_in(&ed, w, |this, ent, ev: &EditEvent, w, cx| match ev {
            EditEvent::Submitted => {
                let val = ent.read(cx).value().to_owned();
                let cat = this.new_category.clone();
                if this.add_memory(val, cat, cx).is_some() {
                    ent.update(cx, |e, cx| e.set_value("", w, cx));
                    this.focus.focus(w, cx);
                }
            }
            EditEvent::Escape => {
                this.adding_new = false;
                this.focus.focus(w, cx);
                cx.notify();
            }
            _ => {}
        })
        .detach();
        self.new_editor = Some(ed.clone());
        ed
    }

    fn localize_cat<'a>(&self, cat: &'a str, cx: &App) -> &'a str {
        match cat {
            "Preferences" => tr(cx, "Preferences", "偏好"),
            "Work" => tr(cx, "Work", "工作"),
            "Projects" => tr(cx, "Projects", "项目"),
            "People" => tr(cx, "People", "联系人"),
            other => other,
        }
    }

    fn localize_text<'a>(&self, s: &'a str, cx: &App) -> &'a str {
        match s {
            "Memory" => tr(cx, "Memory", "记忆管理"),
            "Likes short answers first, details only when asked" => tr(
                cx,
                "Likes short answers first, details only when asked",
                "偏好先给简短结论，追问时再展开细节",
            ),
            "Designs with Inter and a 4px spacing grid" => tr(
                cx,
                "Designs with Inter and a 4px spacing grid",
                "使用 Inter 字体与 4px 间距网格设计",
            ),
            "Product designer and front-end developer at a small studio" => tr(
                cx,
                "Product designer and front-end developer at a small studio",
                "在独立工作室担任产品设计师兼前端开发",
            ),
            "Ships with React, Vite and Tailwind; deploys on Vercel" => tr(
                cx,
                "Ships with React, Vite and Tailwind; deploys on Vercel",
                "使用 React、Vite 与 Tailwind 构建并部署在 Vercel",
            ),
            "Reviews pull requests in the morning before standup" => tr(
                cx,
                "Reviews pull requests in the morning before standup",
                "习惯每天早会前审阅团队的代码合并请求",
            ),
            "Redesigning a fintech onboarding flow, due Oct 24" => tr(
                cx,
                "Redesigning a fintech onboarding flow, due Oct 24",
                "正在改版金融科技新用户引导流程，10 月 24 日截止",
            ),
            "Migrating component tokens to clean neutral surfaces" => tr(
                cx,
                "Migrating component tokens to clean neutral surfaces",
                "正在将组件设计变量迁移至极简黑白中性表面",
            ),
            "You added" => tr(cx, "You added", "手动添加"),
            "From chat" => tr(cx, "From chat", "来自对话"),
            "From Figma" => tr(cx, "From Figma", "来自 Figma"),
            "From GitHub" => tr(cx, "From GitHub", "来自 GitHub"),
            "From Linear" => tr(cx, "From Linear", "来自 Linear"),
            "Today" => tr(cx, "Today", "今天"),
            other => other,
        }
    }
}

impl EventEmitter<MemoryPanelEvent> for MemoryPanel {}

impl Focusable for MemoryPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for MemoryPanel {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);

        if self.loading {
            let phase = self.boot_at.elapsed().as_secs_f32() * 0.9;
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(400.))
                        .p(px(16.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(140., 20., 6., phase, cx))
                        .child(pro_skeleton_bar(320., 28., 6., phase + 0.1, cx))
                        .child(pro_skeleton_bar(360., 52., 8., phase + 0.2, cx))
                        .child(pro_skeleton_bar(360., 52., 8., phase + 0.3, cx))
                        .child(pro_skeleton_bar(360., 52., 8., phase + 0.4, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(400.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Memory,
                            tr(cx, "No saved memories", "暂无已保存记忆"),
                            tr(
                                cx,
                                "Add preferences, project notes, or work context for the assistant to keep.",
                                "添加你的偏好、工作角色或项目背景以便助手在后续对话中记住。",
                            ),
                            Some((
                                pro_eid(&self.id, "restore-mp"),
                                tr(cx, "Load sample memories", "加载示例记忆").into(),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    this.memories = default_memories();
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                )
                .into_any_element();
        }

        // Expire Undo banner once undo_secs elapses.
        let undo_active = if let Some(rec) = &self.forgotten {
            let age = rec.forgotten_at.elapsed().as_secs_f32();
            if age < self.undo_secs {
                if !reduced {
                    w.request_animation_frame();
                }
                true
            } else {
                false
            }
        } else {
            false
        };
        if !undo_active && self.forgotten.is_some() {
            self.forgotten = None;
        }

        let width_cell = self.measured_width.clone();
        let panel_w = width_cell.get();
        let show_chip_counts = panel_w >= 340.0;
        let show_switch_label = panel_w >= 316.0;

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let wash_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.04)
        } else {
            hsla(0., 0., 0., 0.03)
        };
        let fill_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let focus_col = p.focus;

        let total_count = self.memories.len();
        let paused_count = self.paused_count();
        let enabled_now = self.enabled;

        // 1. Header bar: Title + rolling count + Use memory switch
        let header = div()
            .px(px(16.))
            .pt(px(14.))
            .pb(px(2.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(15.))
                    .line_height(px(22.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .truncate()
                    .child(self.localize_text(&self.title, cx).to_owned()),
            )
            .child(
                div()
                    .min_w(px(20.))
                    .h(px(18.))
                    .px(px(5.))
                    .rounded(px(5.))
                    .bg(fill_bg)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "head-cnt"),
                        total_count as f64,
                        0,
                        "",
                        "",
                        11.,
                        FontWeight::MEDIUM,
                        p.text,
                    )),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .when(show_switch_label, |d| {
                        d.child(div().text_size(px(13.)).text_color(p.text).child(tr(
                            cx,
                            "Use memory",
                            "启用记忆",
                        )))
                    })
                    .child(pro_switch(
                        pro_eid(&self.id, "master-sw"),
                        tr(cx, "Use memory", "启用记忆"),
                        enabled_now,
                        cx,
                        move |this, _, cx| {
                            this.set_enabled(!enabled_now, cx);
                        },
                    )),
            );

        // Disabled note folded cleanly under header
        let off_note_txt = tr(
            cx,
            "The assistant won't use or save memories in new chats. Nothing here is deleted.",
            "助手不会在新对话中使用或保存记忆，此处已保存的条目不会被删除。",
        )
        .to_owned();
        let muted_ic = p.muted;
        let txt_col = p.text;
        let off_note = pro_fold(
            pro_eid(&self.id, "off-note"),
            !enabled_now,
            46.0,
            0.0,
            move || {
                div()
                    .px(px(16.))
                    .pt(px(10.))
                    .child(
                        div()
                            .px(px(10.))
                            .py(px(8.))
                            .rounded(px(8.))
                            .bg(wash_bg)
                            .flex()
                            .items_start()
                            .gap(px(8.))
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(txt_col)
                            .child(div().pt(px(1.)).child(pro_svg(
                                "0 0 24 24",
                                INFO_CIRCLE_SVG,
                                15.,
                                muted_ic,
                            )))
                            .child(div().flex_1().min_w_0().child(off_note_txt)),
                    )
                    .into_any_element()
            },
        );

        // 2. Category Filter Chips + Search toggle (`pro_pill_indicator` + `pro_fold` search bar)
        let cats = self.effective_categories();
        let sensor = self.cat_sensor.clone();
        let sel_key: SharedString = self
            .selected_category
            .clone()
            .unwrap_or_else(|| "__all__".into());

        let mut strip = div()
            .relative()
            .flex_1()
            .min_w_0()
            .p(px(3.))
            .rounded(px(8.))
            .bg(wash_bg)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "cat-ind"),
                sel_key.clone(),
                sensor.clone(),
                6.0,
                card_bg,
                Some(p.line),
                true,
            ));

        // "All" chip
        let all_sel = self.selected_category.is_none();
        let all_id = pro_eid(&self.id, "chip-cnt-all");
        strip = strip.child(activate(
            div()
                .id(pro_eid(&self.id, "chip-all"))
                .role(Role::RadioButton)
                .aria_selected(all_sel)
                .tab_index(if all_sel { 0 } else { -1 })
                .relative()
                .h(px(26.))
                .px(px(9.))
                .rounded(px(6.))
                .when(!all_sel, |d| d.hover(move |s| s.bg(wash_bg)))
                .focus_visible(move |s| s.border_2().border_color(focus_col))
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(5.))
                .when(all_sel, |d| d.child(pro_mark_selected(&sensor)))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if all_sel { p.strong } else { p.text })
                        .child(tr(cx, "All", "全部")),
                )
                .when(show_chip_counts, |d| {
                    d.child(pro_rolling_number(
                        all_id,
                        total_count as f64,
                        0,
                        "",
                        "",
                        11.5,
                        FontWeight::NORMAL,
                        if all_sel { p.text } else { p.muted },
                    ))
                }),
            cx,
            |this, _, cx| this.select_category(None, cx),
        ));

        for cat in &cats {
            let cnt = self.memories.iter().filter(|m| &m.category == cat).count();
            if cnt == 0 && self.selected_category.as_ref() != Some(cat) {
                continue;
            }
            let is_sel = self.selected_category.as_ref() == Some(cat);
            let cat_c = cat.clone();
            let cat_lbl = self.localize_cat(cat, cx).to_owned();
            let cnt_id = pro_eid(&self.id, format!("chip-cnt-{cat}"));
            strip = strip.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("chip-{cat}")))
                    .role(Role::RadioButton)
                    .aria_selected(is_sel)
                    .tab_index(if is_sel { 0 } else { -1 })
                    .relative()
                    .h(px(26.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .when(!is_sel, |d| d.hover(move |s| s.bg(wash_bg)))
                    .focus_visible(move |s| s.border_2().border_color(focus_col))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .when(is_sel, |d| d.child(pro_mark_selected(&sensor)))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if is_sel { p.strong } else { p.text })
                            .child(cat_lbl),
                    )
                    .when(show_chip_counts, |d| {
                        d.child(pro_rolling_number(
                            cnt_id,
                            cnt as f64,
                            0,
                            "",
                            "",
                            11.5,
                            FontWeight::NORMAL,
                            if is_sel { p.text } else { p.muted },
                        ))
                    }),
                cx,
                move |this, _, cx| this.select_category(Some(cat_c.clone()), cx),
            ));
        }

        let searching_now = self.searching;
        let search_ed_opt = if searching_now || self.search_editor.is_some() {
            Some(self.ensure_search_editor(w, cx))
        } else {
            None
        };
        let close_btn_opt = search_ed_opt.as_ref().map(|ed| {
            let ed_c = ed.clone();
            pro_icon_btn(
                pro_eid(&self.id, "close-search"),
                tr(cx, "Close search", "关闭搜索"),
                ProIcon::Close,
                24.,
                false,
                cx,
                move |this, w, cx| {
                    this.searching = false;
                    this.search_query.clear();
                    ed_c.update(cx, |e, cx| e.set_value("", w, cx));
                    cx.notify();
                },
            )
        });
        let search_line_strong = p.line_strong;
        let search_muted = p.muted;
        let search_fold = pro_fold(
            pro_eid(&self.id, "search-fold"),
            searching_now,
            40.0,
            0.0,
            move || {
                if let (Some(search_ed), Some(close_btn)) = (search_ed_opt, close_btn_opt) {
                    div()
                        .pt(px(8.))
                        .child(
                            div()
                                .h(px(30.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .bg(wash_bg)
                                .border_1()
                                .border_color(search_line_strong)
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(pro_icon(ProIcon::Search, 14., search_muted))
                                .child(div().flex_1().min_w_0().h(px(20.)).child(search_ed))
                                .child(close_btn),
                        )
                        .into_any_element()
                } else {
                    div().into_any_element()
                }
            },
        );

        let filter_bar = div()
            .px(px(14.))
            .py(px(10.))
            .border_b_1()
            .border_color(p.line)
            .opacity(if enabled_now { 1.0 } else { 0.55 })
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(strip)
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "open-search"),
                        tr(cx, "Search memories", "搜索记忆"),
                        ProIcon::Search,
                        28.,
                        searching_now,
                        cx,
                        |this, w, cx| {
                            this.searching = !this.searching;
                            if this.searching {
                                let ed = this.ensure_search_editor(w, cx);
                                ed.update(cx, |e, cx| e.focus.focus(w, cx));
                            } else {
                                this.search_query.clear();
                                if let Some(ed) = this.search_editor.clone() {
                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                }
                                this.focus.focus(w, cx);
                            }
                            cx.notify();
                        },
                    )),
            )
            .child(search_fold);

        // 3. Grouped Memory Rows (`pro_tactile` + `pro_fold` creation, archival, expansion & inline edit)
        let q_lower = self.search_query.trim().to_ascii_lowercase();
        let archiving_entry = self.forgotten.as_ref().and_then(|rec| {
            (!reduced && rec.forgotten_at.elapsed().as_secs_f32() < 0.28).then(|| rec.entry.clone())
        });
        let mut list_col = div()
            .id(pro_eid(&self.id, "mem-scroll"))
            .max_h(px(self.max_height))
            .overflow_y_scroll()
            .px(px(6.))
            .pt(px(2.))
            .pb(px(6.))
            .opacity(if enabled_now { 1.0 } else { 0.50 })
            .flex()
            .flex_col();

        let mut visible_total = 0usize;
        for cat in &cats {
            if let Some(sel) = &self.selected_category
                && sel != cat
            {
                continue;
            }
            let mut group_items: Vec<(MemoryEntry, bool)> = self
                .memories
                .iter()
                .filter(|m| &m.category == cat)
                .filter(|m| {
                    if q_lower.is_empty() {
                        true
                    } else {
                        m.text.to_ascii_lowercase().contains(&q_lower)
                            || m.source.to_ascii_lowercase().contains(&q_lower)
                            || self
                                .localize_text(&m.text, cx)
                                .to_ascii_lowercase()
                                .contains(&q_lower)
                    }
                })
                .cloned()
                .map(|m| (m, false))
                .collect();

            if let Some(ghost) = &archiving_entry
                && &ghost.category == cat
                && !group_items.iter().any(|(m, _)| m.id == ghost.id)
            {
                group_items.push((ghost.clone(), true));
            }

            let live_len = group_items.iter().filter(|(_, arch)| !*arch).count();
            if group_items.is_empty() {
                continue;
            }
            visible_total += live_len;

            let group_heading = self.localize_cat(cat, cx).to_uppercase();
            list_col = list_col.child(
                div()
                    .px(px(10.))
                    .pt(px(12.))
                    .pb(px(4.))
                    .text_size(px(10.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(group_heading),
            );

            for (item, is_archiving) in group_items {
                let mid = item.id.clone();
                let is_editing = !is_archiving && self.editing_id.as_ref() == Some(&mid);
                let is_expanded =
                    !is_archiving && !is_editing && self.expanded_id.as_ref() == Some(&mid);
                let is_hovered = !is_archiving && self.hovered_row.as_ref() == Some(&mid);

                let (enter_t, enter_open) =
                    if let Some((slide, clock)) = self.entering_clocks.get_mut(&mid) {
                        let s = slide.get(w, cx).clamp(0.0, 1.0);
                        let age = clock.time(0.30, w, cx);
                        (s, reduced || age > 0.012)
                    } else {
                        (1.0, true)
                    };
                let life_open = !is_archiving && enter_open;

                let row_inner: AnyElement = if is_editing {
                    let seed_txt = self.localize_text(&item.text, cx).to_owned();
                    let row_ed = self.ensure_row_editor(&mid, &seed_txt, w, cx);
                    let mid_save = mid.clone();
                    let ed_save = row_ed.clone();
                    let line_strong = p.line_strong;
                    let cancel_btn = pro_btn(
                        pro_eid(&self.id, format!("cancel-ed-{mid}")),
                        tr(cx, "Cancel", "取消"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, _, cx| {
                            this.editing_id = None;
                            this.row_editor_seeded = None;
                            cx.notify();
                        },
                    );
                    let save_btn = pro_btn(
                        pro_eid(&self.id, format!("save-ed-{mid}")),
                        tr(cx, "Save", "保存"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| {
                            let val = ed_save.read(cx).value().to_owned();
                            this.save_edit(mid_save.as_ref(), val, cx);
                        },
                    );
                    pro_fold(
                        pro_eid(&self.id, format!("mrow-edit-fold-{mid}")),
                        true,
                        92.0,
                        0.0,
                        move || {
                            div()
                                .px(px(10.))
                                .py(px(8.))
                                .rounded(px(8.))
                                .bg(wash_bg)
                                .border_1()
                                .border_color(line_strong)
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .child(div().w_full().h(px(42.)).child(row_ed))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_end()
                                        .gap(px(6.))
                                        .child(cancel_btn)
                                        .child(save_btn),
                                )
                                .into_any_element()
                        },
                    )
                } else {
                    let mid_hov = mid.clone();
                    let mid_ed = mid.clone();
                    let mid_ed2 = mid.clone();
                    let mid_pause = mid.clone();
                    let mid_del = mid.clone();
                    let mid_exp = mid.clone();
                    let disp_txt = self.localize_text(&item.text, cx).to_owned();
                    let src_txt = self.localize_text(&item.source, cx).to_owned();
                    let date_txt = self.localize_text(&item.date, cx).to_owned();
                    let cat_disp = self.localize_cat(&item.category, cx).to_owned();
                    let red_soft = p.red_soft;
                    let red_fg = p.red;
                    let icon_col = p.muted;
                    let ink_col = p.ink;
                    let act_op = pro_spring_f32(
                        pro_eid(&self.id, format!("mact-op-{mid}")),
                        if is_hovered || is_expanded { 1.0 } else { 0.0 },
                        0.16,
                        w,
                        cx,
                    )
                    .clamp(0.0, 1.0);

                    let actions = div()
                        .absolute()
                        .right(px(8.))
                        .top(px(28.))
                        .opacity(act_op)
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("act-ed-{mid}")))
                                .role(Role::Button)
                                .aria_label(format!("{} “{}”", tr(cx, "Edit", "编辑"), disp_txt))
                                .tab_index(0)
                                .size(px(26.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(fill_bg))
                                .active(move |s| s.bg(fill_bg).top(px(0.5)))
                                .focus_visible(move |s| s.border_1().border_color(focus_col))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_svg("0 0 24 24", EDIT_PENCIL_SVG, 13.5, icon_col)),
                            cx,
                            move |this, _, cx| {
                                this.editing_id = Some(mid_ed.clone());
                                this.expanded_id = None;
                                this.row_editor_seeded = None;
                                cx.notify();
                            },
                        ))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("act-pa-{mid}")))
                                .role(Role::Button)
                                .aria_label(format!(
                                    "{} “{}”",
                                    if item.paused {
                                        tr(cx, "Resume", "恢复")
                                    } else {
                                        tr(cx, "Pause", "暂停")
                                    },
                                    disp_txt
                                ))
                                .tab_index(0)
                                .size(px(26.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(fill_bg))
                                .active(move |s| s.bg(fill_bg).top(px(0.5)))
                                .focus_visible(move |s| s.border_1().border_color(focus_col))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(
                                    if item.paused {
                                        ProIcon::Play
                                    } else {
                                        ProIcon::Pause
                                    },
                                    13.5,
                                    if item.paused { ink_col } else { icon_col },
                                )),
                            cx,
                            move |this, _, cx| {
                                this.toggle_pause(mid_pause.as_ref(), cx);
                            },
                        ))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("act-fg-{mid}")))
                                .role(Role::Button)
                                .aria_label(format!("{} “{}”", tr(cx, "Forget", "遗忘"), disp_txt))
                                .tab_index(0)
                                .size(px(26.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(red_soft))
                                .active(move |s| s.bg(red_soft).top(px(0.5)))
                                .focus_visible(move |s| s.border_1().border_color(focus_col))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(
                                    ProIcon::Trash,
                                    13.5,
                                    if is_hovered || is_expanded {
                                        red_fg.opacity(0.85)
                                    } else {
                                        icon_col
                                    },
                                )),
                            cx,
                            move |this, _, cx| {
                                this.forget(mid_del.as_ref(), cx);
                            },
                        ));

                    let row_face = div()
                        .px(px(10.))
                        .pt(px(8.))
                        .pb(px(7.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(if item.paused { p.muted } else { p.ink })
                                .child(disp_txt.clone()),
                        )
                        .child(
                            div()
                                .min_h(px(22.))
                                .pr(px(82.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(src_txt.clone())
                                .child(div().text_color(p.faint).child("·"))
                                .child(date_txt.clone())
                                .when(item.paused, |d| {
                                    d.child(
                                        div()
                                            .h(px(18.))
                                            .px(px(6.))
                                            .rounded(px(5.))
                                            .bg(p.amber_soft)
                                            .text_color(p.amber_ink)
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .flex()
                                            .items_center()
                                            .child(tr(cx, "Paused", "已暂停")),
                                    )
                                }),
                        );

                    let tactile_row = pro_tactile(
                        pro_eid(&self.id, format!("mrow-hit-{mid}")),
                        disp_txt,
                        8.0,
                        transparent_black(),
                        wash_bg,
                        transparent_black(),
                        if is_expanded {
                            p.line
                        } else {
                            transparent_black()
                        },
                        is_expanded,
                        row_face,
                        cx,
                        move |this, _, cx| {
                            if this.expanded_id.as_ref() == Some(&mid_exp) {
                                this.expanded_id = None;
                            } else {
                                this.expanded_id = Some(mid_exp.clone());
                            }
                            cx.notify();
                        },
                    )
                    .w_full();

                    let status_word = if item.paused {
                        tr(cx, "Excluded from prompt", "当前对话已暂停注入")
                    } else {
                        tr(cx, "Active in system prompt", "已在系统上下文生效")
                    };
                    let chk_el = pro_check_draw(
                        pro_eid(&self.id, format!("mchk-{mid}")),
                        !item.paused && enabled_now,
                        13.5,
                        false,
                        p.green,
                    );
                    let status_lbl = pro_swap_label(
                        pro_eid(&self.id, format!("mst-lbl-{mid}")),
                        status_word,
                        11.5,
                        FontWeight::MEDIUM,
                        if item.paused { p.amber_ink } else { p.text },
                    );
                    let edit_inline_btn = pro_btn(
                        pro_eid(&self.id, format!("exp-ed-{mid}")),
                        tr(cx, "Edit text", "编辑内容"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.editing_id = Some(mid_ed2.clone());
                            this.expanded_id = None;
                            this.row_editor_seeded = None;
                            cx.notify();
                        },
                    );
                    let exp_line = p.line;
                    let exp_chip = p.chip;
                    let exp_muted = p.muted;
                    let exp_fold = pro_fold(
                        pro_eid(&self.id, format!("mrow-exp-{mid}")),
                        is_expanded,
                        40.0,
                        0.0,
                        move || {
                            div()
                                .mx(px(10.))
                                .pt(px(6.))
                                .pb(px(8.))
                                .border_t_1()
                                .border_color(exp_line)
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(chk_el)
                                        .child(status_lbl)
                                        .child(
                                            div()
                                                .px(px(6.))
                                                .py(px(1.5))
                                                .rounded(px(4.))
                                                .bg(exp_chip)
                                                .text_size(px(11.))
                                                .text_color(exp_muted)
                                                .child(cat_disp),
                                        ),
                                )
                                .child(edit_inline_btn)
                                .into_any_element()
                        },
                    );

                    div()
                        .id(pro_eid(&self.id, format!("mrow-{mid}")))
                        .relative()
                        .rounded(px(8.))
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            let next = (*inside).then(|| mid_hov.clone());
                            if this.hovered_row != next {
                                this.hovered_row = next;
                                cx.notify();
                            }
                        }))
                        .flex()
                        .flex_col()
                        .child(tactile_row)
                        .child(actions)
                        .child(exp_fold)
                        .into_any_element()
                };

                list_col = list_col.child(pro_fold(
                    pro_eid(&self.id, format!("mrow-life-{mid}")),
                    life_open,
                    58.0,
                    0.0,
                    move || {
                        div()
                            .relative()
                            .top(px(-5.0 * (1.0 - enter_t)))
                            .w_full()
                            .child(row_inner)
                            .into_any_element()
                    },
                ));
            }
        }

        if visible_total == 0 {
            list_col = list_col.child(
                div()
                    .py(px(24.))
                    .px(px(12.))
                    .text_center()
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(tr(cx, "No memories here", "当前分类下暂无记忆")),
            );
        }

        // 4. "+ New memory" tactile trigger + `pro_fold` inline composer with `pro_pill_indicator`
        let adding_now = self.adding_new;
        let new_ed_opt = if adding_now || self.new_editor.is_some() {
            Some(self.ensure_new_editor(w, cx))
        } else {
            None
        };
        let new_composer_fold = {
            let chosen_cat = self.new_category.clone();
            let new_sensor = self.new_cat_sensor.clone();
            let mut cat_pills = div()
                .relative()
                .p(px(2.5))
                .rounded(px(7.5))
                .bg(wash_bg)
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "newcat-ind"),
                    chosen_cat.clone(),
                    new_sensor.clone(),
                    5.5,
                    card_bg,
                    Some(p.line),
                    true,
                ));
            for cat in &cats {
                let active = &chosen_cat == cat;
                let cat_pick = cat.clone();
                cat_pills = cat_pills.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("newcat-{cat}")))
                        .role(Role::RadioButton)
                        .aria_selected(active)
                        .tab_index(if active { 0 } else { -1 })
                        .relative()
                        .px(px(8.))
                        .py(px(3.5))
                        .rounded(px(5.5))
                        .when(!active, |d| d.hover(move |s| s.bg(wash_bg)))
                        .focus_visible(move |s| s.border_1().border_color(focus_col))
                        .cursor_pointer()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if active { p.ink } else { p.text })
                        .when(active, |d| d.child(pro_mark_selected(&new_sensor)))
                        .child(self.localize_cat(cat, cx).to_owned()),
                    cx,
                    move |this, _, cx| {
                        this.new_category = cat_pick.clone();
                        cx.notify();
                    },
                ));
            }

            let cancel_new_btn = pro_btn(
                pro_eid(&self.id, "cancel-new"),
                tr(cx, "Cancel", "取消"),
                None,
                ProBtnStyle::Ghost,
                true,
                cx,
                |this, _, cx| {
                    this.adding_new = false;
                    cx.notify();
                },
            );
            let save_new_opt = new_ed_opt.as_ref().map(|ed| {
                let ed_save = ed.clone();
                pro_btn(
                    pro_eid(&self.id, "save-new"),
                    tr(cx, "Save", "保存"),
                    Some(ProIcon::Check),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    move |this, w, cx| {
                        let val = ed_save.read(cx).value().to_owned();
                        let cat = this.new_category.clone();
                        if this.add_memory(val, cat, cx).is_some() {
                            ed_save.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                    },
                )
            });
            let comp_line = p.line;
            let comp_line_strong = p.line_strong;
            pro_fold(
                pro_eid(&self.id, "new-mem-fold"),
                adding_now,
                116.0,
                0.0,
                move || {
                    if let (Some(new_ed), Some(save_new_btn)) = (new_ed_opt, save_new_opt) {
                        div()
                            .p(px(10.))
                            .border_t_1()
                            .border_color(comp_line)
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(
                                div()
                                    .px(px(8.))
                                    .py(px(6.))
                                    .rounded(px(7.))
                                    .bg(wash_bg)
                                    .border_1()
                                    .border_color(comp_line_strong)
                                    .h(px(54.))
                                    .child(new_ed),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(6.))
                                    .child(cat_pills)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(cancel_new_btn)
                                            .child(save_new_btn),
                                    ),
                            )
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    }
                },
            )
        };

        let add_trigger_fold = {
            let add_btn = pro_tactile(
                pro_eid(&self.id, "add-mem-btn"),
                tr(cx, "New memory", "新建记忆"),
                6.5,
                transparent_black(),
                wash_bg,
                transparent_black(),
                transparent_black(),
                false,
                div()
                    .h(px(32.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(p.text)
                    .child(pro_icon(ProIcon::Plus, 14., p.muted))
                    .child(tr(cx, "New memory", "新建记忆")),
                cx,
                |this, w, cx| {
                    this.adding_new = true;
                    let ed = this.ensure_new_editor(w, cx);
                    ed.update(cx, |e, cx| e.focus.focus(w, cx));
                    cx.notify();
                },
            )
            .w_full();
            let trig_line = p.line;
            pro_fold(
                pro_eid(&self.id, "add-trig-fold"),
                !adding_now,
                44.0,
                0.0,
                move || {
                    div()
                        .p(px(6.))
                        .border_t_1()
                        .border_color(trig_line)
                        .child(add_btn)
                        .into_any_element()
                },
            )
        };
        let bottom_composer = div()
            .flex()
            .flex_col()
            .child(new_composer_fold)
            .child(add_trigger_fold);

        // 5. Bottom Tray: Status counts OR animated Undo banner (`pro_presence`)
        let tray = if let Some(rec) = self.forgotten.clone()
            && undo_active
        {
            let forgot_txt = self.localize_text(&rec.entry.text, cx).to_owned();
            let undo_btn = pro_btn(
                pro_eid(&self.id, "undo-btn"),
                tr(cx, "Undo", "撤销"),
                Some(ProIcon::Undo),
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, _, cx| {
                    this.undo_forget(cx);
                },
            );
            let forgot_lbl = tr(cx, "Forgot", "已遗忘");
            pro_presence(
                pro_eid(&self.id, "undo-tray-pres"),
                true,
                0.22,
                0.18,
                move |amt| {
                    div()
                        .id(pro_eid(&"mp-tray".into(), "undo"))
                        .role(Role::Status)
                        .min_h(px(44.))
                        .pl(px(16.))
                        .pr(px(8.))
                        .border_t_1()
                        .border_color(p.line)
                        .opacity(amt)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(12.))
                                .child(div().flex_none().text_color(p.muted).child(forgot_lbl))
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(format!("“{forgot_txt}”")),
                                ),
                        )
                        .child(undo_btn)
                        .into_any_element()
                },
            )
        } else {
            let suffix = if UiLanguage::current(cx) == UiLanguage::Chinese {
                if paused_count > 0 {
                    format!(" 条记忆 · {paused_count} 条已暂停")
                } else {
                    " 条记忆".to_owned()
                }
            } else {
                let word = if total_count == 1 {
                    " memory"
                } else {
                    " memories"
                };
                if paused_count > 0 {
                    format!("{word} · {paused_count} paused")
                } else {
                    word.to_owned()
                }
            };
            div()
                .id(pro_eid(&self.id, "status-tray"))
                .role(Role::Status)
                .min_h(px(44.))
                .px(px(16.))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(pro_rolling_number(
                            pro_eid(&self.id, "tray-cnt"),
                            total_count as f64,
                            0,
                            "",
                            "",
                            12.,
                            FontWeight::MEDIUM,
                            p.ink,
                        ))
                        .child(suffix),
                )
                .into_any_element()
        };

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.localize_text(&self.title, cx).to_owned())
            .track_focus(&self.focus)
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(400.))
                    .relative()
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_xs())
                    .flex()
                    .flex_col()
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                let bw = f32::from(bounds.size.width);
                                if bw > 1.0 {
                                    width_cell.set(bw);
                                }
                            },
                            |_, _, _, _| (),
                        )
                        .absolute()
                        .left_0()
                        .right_0()
                        .top_0()
                        .h(px(1.)),
                    )
                    .child(header)
                    .child(off_note)
                    .child(filter_bar)
                    .child(list_col)
                    .child(bottom_composer)
                    .child(tray),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryPanel;
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn memory_panel_supports_edit_pause_forget_and_undo_restore(cx: &mut TestAppContext) {
        let view = cx.new(|cx| MemoryPanel::new("test-memory-panel", cx));

        view.update(cx, |mp, cx| {
            assert_eq!(mp.memories().len(), 7);
            assert_eq!(mp.paused_count(), 1);
            assert!(mp.is_enabled());

            mp.set_enabled(false, cx);
            assert!(!mp.is_enabled());
            mp.set_enabled(true, cx);

            assert_eq!(mp.toggle_pause("grid-inter", cx), Some(true));
            assert_eq!(mp.paused_count(), 2);

            assert!(mp.save_edit("grid-inter", "Uses Inter with an 8px baseline", cx));
            assert_eq!(mp.memories()[1].text, "Uses Inter with an 8px baseline");

            // Forget row 1 and verify Undo restores it at index 1!
            assert!(mp.forget("grid-inter", cx));
            assert_eq!(mp.memories().len(), 6);
            assert!(mp.undo_forget(cx));
            assert_eq!(mp.memories().len(), 7);
            assert_eq!(mp.memories()[1].id, "grid-inter");

            assert!(
                mp.add_memory("Works closely with Maya Lin", "People", cx)
                    .is_some()
            );
            assert_eq!(mp.memories().len(), 8);
        });
    }
}
