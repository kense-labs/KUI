//! Saved prompt template library with inline `{variable}` chips and live assembled preview.
//!
//! Switch saved templates in the sidebar, type directly into inline `{variable}` [`Editor`] chips
//! embedded inside the sentence, watch the assembled prompt preview highlight filled vs. missing
//! slots below, and copy or dispatch the prompt once every variable is filled.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_eid, pro_empty_state, pro_mark_selected, pro_mix_color, pro_pill_indicator,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_toast_banner,
        pro_view_transition, tr,
    },
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

/// Color tone for a template category pill in [`PromptLibrary`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PromptCategoryTone {
    /// Sky blue (`Notes`, `Docs`).
    #[default]
    Blue,
    /// Violet (`Product`, `Release`).
    Violet,
    /// Amber (`Debug`, `Engineering`).
    Amber,
    /// Emerald green (`Sales`, `Outreach`).
    Green,
    /// Coral red (`Incident`, `Security`).
    Red,
    /// Neutral slate.
    Gray,
}

impl PromptCategoryTone {
    fn colors(self, p: &ProPalette) -> (Hsla, Hsla) {
        match self {
            Self::Blue => (p.accent_soft, p.blue),
            Self::Violet => (p.purple.opacity(0.15), p.purple),
            Self::Amber => (p.amber_soft, p.amber_ink),
            Self::Green => (p.green_soft, p.green_ink),
            Self::Red => (p.red_soft, p.red_ink),
            Self::Gray => (p.chip, p.text),
        }
    }
}

/// Optional metadata and default value for one `{variable}` inside a [`PromptTemplate`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PromptVariableSpec {
    /// Optional human-readable description for screen readers / tooltips.
    pub description: Option<SharedString>,
    /// Optional initial value pre-filled when the template loads.
    pub default_value: Option<SharedString>,
}

impl PromptVariableSpec {
    /// Create a variable specification with a pre-filled `default_value`.
    pub fn with_default(default_value: impl Into<SharedString>) -> Self {
        Self {
            description: None,
            default_value: Some(default_value.into()),
        }
    }
}

/// Saved prompt template containing `{variable}` placeholders in `body`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptTemplate {
    /// Unique template id (e.g. `"release"`).
    pub id: SharedString,
    /// English template title.
    pub title: SharedString,
    /// Localized Chinese template title.
    pub title_zh: SharedString,
    /// Category tag (e.g. `"Product"`, `"Notes"`).
    pub category: SharedString,
    /// Localized Chinese category tag.
    pub category_zh: SharedString,
    /// Category tag color tone.
    pub tone: PromptCategoryTone,
    /// Prompt sentence containing `{variable}` tokens.
    pub body: SharedString,
    /// Per-variable defaults and descriptions keyed by variable name.
    pub variables: HashMap<SharedString, PromptVariableSpec>,
}

impl PromptTemplate {
    /// Construct a new [`PromptTemplate`].
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        category: impl Into<SharedString>,
        tone: PromptCategoryTone,
        body: impl Into<SharedString>,
    ) -> Self {
        let title = title.into();
        let category = category.into();
        Self {
            id: id.into(),
            title_zh: title.clone(),
            title,
            category_zh: category.clone(),
            category,
            tone,
            body: body.into(),
            variables: HashMap::new(),
        }
    }

    /// Set localized Chinese title and category label.
    pub fn with_zh(
        mut self,
        title_zh: impl Into<SharedString>,
        category_zh: impl Into<SharedString>,
    ) -> Self {
        self.title_zh = title_zh.into();
        self.category_zh = category_zh.into();
        self
    }

    /// Pre-populate a `{variable}` with a default value.
    pub fn with_default_var(
        mut self,
        name: impl Into<SharedString>,
        default_value: impl Into<SharedString>,
    ) -> Self {
        self.variables
            .insert(name.into(), PromptVariableSpec::with_default(default_value));
        self
    }

    /// Return ordered unique `{variable}` names referenced in `self.body`.
    pub fn variable_names(&self) -> Vec<SharedString> {
        let mut out = Vec::new();
        for seg in parse_prompt_segments(&self.body) {
            if let PromptSegment::Variable(v) = seg
                && !out.contains(&v)
            {
                out.push(v);
            }
        }
        out
    }
}

/// Parsed segment of a [`PromptTemplate`] body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptSegment {
    /// Plain literal text between variables.
    Literal(String),
    /// Named `{variable}` slot (`[A-Za-z0-9_-]+`).
    Variable(SharedString),
}

/// Parse a template string into literal runs and `{variable}` slots.
pub fn parse_prompt_segments(body: &str) -> Vec<PromptSegment> {
    let mut out = Vec::new();
    let mut literal = String::new();
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i] == '{'
            && let Some(rel_end) = chars[i + 1..].iter().position(|&c| c == '}')
        {
            let candidate: String = chars[i + 1..i + 1 + rel_end].iter().collect();
            let valid = !candidate.is_empty()
                && candidate
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            if valid {
                if !literal.is_empty() {
                    out.push(PromptSegment::Literal(std::mem::take(&mut literal)));
                }
                out.push(PromptSegment::Variable(SharedString::from(candidate)));
                i += rel_end + 2;
                continue;
            }
        }
        literal.push(chars[i]);
        i += 1;
    }
    if !literal.is_empty() {
        out.push(PromptSegment::Literal(literal));
    }
    out
}

/// Typed events emitted by [`PromptLibrary`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptLibraryEvent {
    /// Active template selection changed.
    Selected(SharedString),
    /// A variable value inside a template changed.
    VariableChanged {
        template_id: SharedString,
        variable: SharedString,
        value: SharedString,
    },
    /// Assembled prompt was copied to the clipboard.
    Copied(SharedString),
    /// User pressed `Use prompt` once all variables were filled.
    Used {
        template_id: SharedString,
        prompt: SharedString,
        values: HashMap<SharedString, SharedString>,
    },
}

struct VarEditorEntry {
    template_id: SharedString,
    var_name: SharedString,
    occurrence: usize,
    editor: Entity<Editor>,
}

/// Interactive prompt template library with inline `{variable}` slot chips and live preview.
pub struct PromptLibrary {
    id: ElementId,
    title: SharedString,
    title_zh: SharedString,
    use_label: SharedString,
    use_label_zh: SharedString,
    templates: Vec<PromptTemplate>,
    active_id: SharedString,
    values: HashMap<SharedString, HashMap<SharedString, String>>,
    search_query: String,
    search_editor: Option<Entity<Editor>>,
    var_editors: Vec<VarEditorEntry>,
    last_flash: Option<(SharedString, SharedString, Instant)>,
    copied_at: Option<Instant>,
    loading: bool,
    empty: bool,
    width_cell: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    focus: FocusHandle,
}

impl EventEmitter<PromptLibraryEvent> for PromptLibrary {}

impl PromptLibrary {
    /// Construct a [`PromptLibrary`] preloaded with 4 realistic product/notes/debug/sales templates.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let templates = default_prompt_templates();
        let active_id = templates
            .first()
            .map(|t| t.id.clone())
            .unwrap_or_else(|| "release".into());
        let values = seed_template_values(&templates);
        Self {
            id: id.into(),
            title: "Prompts".into(),
            title_zh: "提示词模板库".into(),
            use_label: "Use prompt".into(),
            use_label_zh: "使用提示词".into(),
            templates,
            active_id,
            values,
            search_query: String::new(),
            search_editor: None,
            var_editors: Vec::new(),
            last_flash: None,
            copied_at: None,
            loading: false,
            empty: false,
            width_cell: Rc::new(Cell::new(620.)),
            toast: None,
            focus: cx.focus_handle(),
        }
    }

    /// Customize the sidebar heading.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        let t = title.into();
        self.title_zh = t.clone();
        self.title = t;
        self
    }

    /// Customize the primary action button label (default `"Use prompt"`).
    pub fn with_use_label(mut self, label: impl Into<SharedString>) -> Self {
        let l = label.into();
        self.use_label_zh = l.clone();
        self.use_label = l;
        self
    }

    /// Supply custom [`PromptTemplate`] entries.
    pub fn with_templates(mut self, templates: Vec<PromptTemplate>) -> Self {
        self.values = seed_template_values(&templates);
        if let Some(first) = templates.first() {
            self.active_id = first.id.clone();
        }
        self.templates = templates;
        self
    }

    /// Set the initially selected template id.
    pub fn with_default_template(mut self, template_id: impl Into<SharedString>) -> Self {
        let tid = template_id.into();
        if self.templates.iter().any(|t| t.id == tid) {
            self.active_id = tid;
        }
        self
    }

    /// Toggle skeleton loading preview state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle forced empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently selected template id.
    pub fn active_template_id(&self) -> &str {
        &self.active_id
    }

    /// Return the currently selected [`PromptTemplate`], if any.
    pub fn active_template(&self) -> Option<&PromptTemplate> {
        self.templates
            .iter()
            .find(|t| t.id == self.active_id)
            .or_else(|| self.templates.first())
    }

    /// Select another template by id, preserving any values typed in previous templates.
    pub fn select_template(&mut self, template_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(found) = self.templates.iter().find(|t| t.id.as_ref() == template_id) else {
            return false;
        };
        if self.active_id == found.id {
            return true;
        }
        self.active_id = found.id.clone();
        cx.emit(PromptLibraryEvent::Selected(self.active_id.clone()));
        cx.notify();
        true
    }

    /// Read the current value for `variable` on `template_id`.
    pub fn variable_value(&self, template_id: &str, variable: &str) -> &str {
        self.values
            .get(template_id)
            .and_then(|m| m.get(variable))
            .map(String::as_str)
            .unwrap_or("")
    }

    /// Programmatically set a variable's value on the active (or specified) template.
    pub fn set_variable(
        &mut self,
        template_id: &str,
        variable: &str,
        value: impl Into<String>,
        w: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        let val_str = value.into();
        let tid_key = SharedString::from(template_id.to_string());
        let var_key = SharedString::from(variable.to_string());
        self.values
            .entry(tid_key.clone())
            .or_default()
            .insert(var_key.clone(), val_str.clone());
        self.last_flash = Some((tid_key.clone(), var_key.clone(), Instant::now()));

        if let Some(win) = w {
            for entry in &self.var_editors {
                if entry.template_id == tid_key && entry.var_name == var_key {
                    let cur = entry.editor.read(cx).value().to_string();
                    if cur != val_str {
                        entry
                            .editor
                            .update(cx, |e, cx| e.set_value(val_str.clone(), win, cx));
                    }
                }
            }
        }

        cx.emit(PromptLibraryEvent::VariableChanged {
            template_id: tid_key,
            variable: var_key,
            value: SharedString::from(val_str),
        });
        cx.notify();
    }

    /// Return `(filled_count, total_unique_variables)` for the active template.
    pub fn active_fill_progress(&self) -> (usize, usize) {
        let Some(tpl) = self.active_template() else {
            return (0, 0);
        };
        let vars = tpl.variable_names();
        let total = vars.len();
        let filled = vars
            .iter()
            .filter(|v| !self.variable_value(&tpl.id, v.as_ref()).trim().is_empty())
            .count();
        (filled, total)
    }

    /// Return whether every `{variable}` in the active template has a non-empty value.
    pub fn is_ready(&self) -> bool {
        let (filled, total) = self.active_fill_progress();
        filled >= total
    }

    /// Assemble the current prompt string for the active template (`{var}` left intact when unfilled).
    pub fn assembled_prompt(&self) -> String {
        let Some(tpl) = self.active_template() else {
            return String::new();
        };
        let mut out = String::with_capacity(tpl.body.len() + 32);
        for seg in parse_prompt_segments(&tpl.body) {
            match seg {
                PromptSegment::Literal(lit) => out.push_str(&lit),
                PromptSegment::Variable(v) => {
                    let val = self.variable_value(&tpl.id, v.as_ref()).trim();
                    if val.is_empty() {
                        out.push('{');
                        out.push_str(v.as_ref());
                        out.push('}');
                    } else {
                        out.push_str(val);
                    }
                }
            }
        }
        out
    }

    /// Estimate token count (`~4` characters per token, minimum `1`).
    pub fn estimated_tokens(&self) -> usize {
        let chars = self.assembled_prompt().chars().count();
        chars.div_ceil(4).max(1)
    }

    /// Copy the assembled prompt to the system clipboard.
    pub fn copy_prompt(&mut self, cx: &mut Context<Self>) -> SharedString {
        let prompt = SharedString::from(self.assembled_prompt());
        cx.write_to_clipboard(ClipboardItem::new_string(prompt.to_string()));
        self.copied_at = Some(Instant::now());
        self.toast = Some((
            SharedString::from(tr(cx, "Prompt copied to clipboard", "提示词已复制到剪贴板")),
            Instant::now(),
        ));
        cx.emit(PromptLibraryEvent::Copied(prompt.clone()));
        cx.notify();
        prompt
    }

    /// Dispatch the assembled prompt if all variables are filled.
    pub fn use_prompt(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.is_ready() {
            return false;
        }
        let Some(tpl) = self.active_template() else {
            return false;
        };
        let tid = tpl.id.clone();
        let vars = tpl.variable_names();
        let mut val_map = HashMap::with_capacity(vars.len());
        for v in vars {
            let val = self.variable_value(&tid, v.as_ref()).trim().to_string();
            val_map.insert(v, SharedString::from(val));
        }
        let prompt = SharedString::from(self.assembled_prompt());
        self.toast = Some((
            SharedString::from(tr(cx, "Prompt ready and sent", "提示词已填充并发送")),
            Instant::now(),
        ));
        cx.emit(PromptLibraryEvent::Used {
            template_id: tid,
            prompt,
            values: val_map,
        });
        cx.notify();
        true
    }

    fn focus_next_unfilled_or_use(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(tpl) = self.active_template() else {
            return;
        };
        let tid = tpl.id.clone();
        for v in tpl.variable_names() {
            if self.variable_value(&tid, v.as_ref()).trim().is_empty()
                && let Some(entry) = self
                    .var_editors
                    .iter()
                    .find(|e| e.template_id == tid && e.var_name == v && e.occurrence == 0)
            {
                entry.editor.clone().update(cx, |e, cx| e.focus(w, cx));
                return;
            }
        }
        self.use_prompt(cx);
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.search_editor.is_none() {
            let se = cx.new(|cx| {
                let mut e = Editor::new("Search templates", InputMode::Search, w, cx);
                e.set_placeholder("Search...");
                e.font_size = 12.5;
                e
            });
            cx.subscribe_in(&se, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    this.search_query = ed_ent.read(cx).value().to_string();
                    cx.notify();
                }
                EditEvent::Escape => {
                    this.search_query.clear();
                    ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(se);
        }

        let Some(tpl) = self.active_template().cloned() else {
            return;
        };
        let tid = tpl.id.clone();
        let segments = parse_prompt_segments(&tpl.body);
        let mut counts: HashMap<SharedString, usize> = HashMap::new();

        for seg in segments {
            let PromptSegment::Variable(vname) = seg else {
                continue;
            };
            let occ = *counts.get(&vname).unwrap_or(&0);
            counts.insert(vname.clone(), occ + 1);

            let exists = self
                .var_editors
                .iter()
                .any(|e| e.template_id == tid && e.var_name == vname && e.occurrence == occ);
            if exists {
                continue;
            }

            let init_val = self.variable_value(&tid, vname.as_ref()).to_string();
            let placeholder = vname.clone();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(placeholder.clone(), InputMode::Single, w, cx);
                e.set_placeholder(placeholder);
                if !init_val.is_empty() {
                    e.set_value(init_val, w, cx);
                }
                e.font_size = 13.;
                e
            });

            let tid_sub = tid.clone();
            let vname_sub = vname.clone();
            cx.subscribe_in(
                &ed,
                w,
                move |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                    EditEvent::Changed => {
                        let next_val = ed_ent.read(cx).value().to_string();
                        this.set_variable(&tid_sub, &vname_sub, next_val, Some(w), cx);
                    }
                    EditEvent::Submitted => {
                        this.focus_next_unfilled_or_use(w, cx);
                    }
                    EditEvent::Escape => {
                        this.focus.focus(w, cx);
                    }
                    _ => {}
                },
            )
            .detach();

            self.var_editors.push(VarEditorEntry {
                template_id: tid.clone(),
                var_name: vname,
                occurrence: occ,
                editor: ed,
            });
        }
    }
}

impl Render for PromptLibrary {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let width_cell = self.width_cell.clone();
        let narrow = width_cell.get() < 540.;

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
                .max_w(px(640.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(pro_skeleton_bar(220., 16., 4., phase, cx))
                .child(pro_skeleton_bar(600., 140., 8., phase, cx))
                .child(pro_skeleton_bar(600., 36., 6., phase, cx))
                .into_any_element();
        }

        if self.empty || self.templates.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(640.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "No saved prompt templates", "暂无已保存的提示词模板"),
                    tr(
                        cx,
                        "Create reusable prompt templates with {variable} slots for your team.",
                        "创建带 {variable} 占位符的可复用提示词模板。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-load-tpl"),
                        SharedString::from(tr(cx, "Load starter prompts", "加载示例模板")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            if this.templates.is_empty() {
                                this.templates = default_prompt_templates();
                                this.values = seed_template_values(&this.templates);
                                if let Some(first) = this.templates.first() {
                                    this.active_id = first.id.clone();
                                }
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let flash_alpha = if disabled_motion(cx) {
            0.0
        } else if let Some((_, _, started)) = &self.last_flash {
            let age = started.elapsed().as_secs_f32();
            if age < 0.85 {
                w.request_animation_frame();
                (1.0 - age / 0.85).powi(2)
            } else {
                0.0
            }
        } else {
            0.0
        };
        let recently_copied = self
            .copied_at
            .is_some_and(|t0| t0.elapsed().as_secs_f32() < 1.8);
        if recently_copied && !disabled_motion(cx) {
            w.request_animation_frame();
        }

        let query_lower = self.search_query.trim().to_ascii_lowercase();
        let filtered_templates: Vec<PromptTemplate> = self
            .templates
            .iter()
            .filter(|t| {
                query_lower.is_empty()
                    || t.title.as_ref().to_ascii_lowercase().contains(&query_lower)
                    || t.title_zh.contains(&query_lower)
                    || t.category
                        .as_ref()
                        .to_ascii_lowercase()
                        .contains(&query_lower)
            })
            .cloned()
            .collect();

        let list_sensor = pro_selection_sensor();
        let mut tpl_list = div()
            .mt(px(8.))
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "tpl-glide"),
                self.active_id.clone(),
                list_sensor.clone(),
                6.,
                p.select,
                None,
                false,
            ));

        if filtered_templates.is_empty() {
            tpl_list = tpl_list.child(
                div()
                    .h(px(32.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(tr(cx, "No matching prompts", "未找到匹配模板")),
            );
        } else {
            for tpl in &filtered_templates {
                let selected = tpl.id == self.active_id;
                let tid = tpl.id.clone();
                let (tag_bg, tag_fg) = tpl.tone.colors(&p);
                let row_title = tr(cx, tpl.title.as_ref(), tpl.title_zh.as_ref()).to_string();
                let row_cat = tr(cx, tpl.category.as_ref(), tpl.category_zh.as_ref()).to_string();

                tpl_list = tpl_list.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("tpl-opt-{tid}")))
                        .role(Role::Button)
                        .aria_label(row_title.clone())
                        .tab_index(0)
                        .w_full()
                        .h(px(32.))
                        .pl(px(8.))
                        .pr(px(6.))
                        .relative()
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(6.))
                        .cursor_pointer()
                        .when(!selected, move |d| d.hover(move |s| s.bg(p.hover)))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .when(selected, |d| d.child(pro_mark_selected(&list_sensor)))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(12.5))
                                .font_weight(if selected {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if selected { p.ink } else { p.text })
                                .child(row_title),
                        )
                        .child(
                            div()
                                .h(px(18.))
                                .px(px(6.))
                                .flex_none()
                                .rounded(px(5.))
                                .bg(tag_bg)
                                .flex()
                                .items_center()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(tag_fg)
                                .child(row_cat),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.select_template(&tid, cx);
                    },
                ));
            }
        }

        let search_box = self
            .search_editor
            .as_ref()
            .map(|se| {
                div()
                    .mt(px(10.))
                    .w_full()
                    .h(px(28.))
                    .pl(px(7.))
                    .pr(px(8.))
                    .rounded(px(6.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_icon(ProIcon::Search, 13.5, p.muted))
                    .child(div().flex_1().min_w_0().h(px(20.)).child(se.clone()))
                    .into_any_element()
            })
            .unwrap_or_else(|| div().into_any_element());

        let side_heading = tr(cx, self.title.as_ref(), self.title_zh.as_ref()).to_string();
        let sidebar = div()
            .when(!narrow, |d| {
                d.w(px(220.)).flex_none().border_r_1().border_color(p.line)
            })
            .when(narrow, |d| d.w_full().border_b_1().border_color(p.line))
            .pt(px(12.))
            .px(px(8.))
            .pb(px(10.))
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(8.))
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(side_heading),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(self.templates.len().to_string()),
                    ),
            )
            .child(search_box)
            .child(tpl_list);

        // Build active template right-hand editor & preview panel
        let active_tpl = self.active_template().cloned();
        let main_panel = if let Some(tpl) = active_tpl {
            let tid = tpl.id.clone();
            let (filled_cnt, total_cnt) = self.active_fill_progress();
            let (tag_bg, tag_fg) = tpl.tone.colors(&p);
            let panel_title = tr(cx, tpl.title.as_ref(), tpl.title_zh.as_ref()).to_string();
            let panel_cat = tr(cx, tpl.category.as_ref(), tpl.category_zh.as_ref()).to_string();

            let unique_vars = tpl.variable_names();
            let mut mini_bars = div().flex().items_center().gap(px(2.5));
            for v in &unique_vars {
                let is_on = !self.variable_value(&tid, v.as_ref()).trim().is_empty();
                mini_bars = mini_bars.child(div().size(px(6.)).rounded(px(1.5)).bg(if is_on {
                    p.blue
                } else {
                    p.chip_hover
                }));
            }

            let count_text = tr(
                cx,
                &format!("{filled_cnt} of {total_cnt} filled"),
                &format!("已填 {filled_cnt} / {total_cnt}"),
            )
            .to_string();

            let panel_head = div()
                .w_full()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(13.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(panel_title),
                )
                .child(
                    div()
                        .h(px(18.))
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(tag_bg)
                        .flex()
                        .items_center()
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(tag_fg)
                        .child(panel_cat),
                )
                .child(
                    div()
                        .ml_auto()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(count_text)
                        .child(mini_bars),
                );

            // Inline editable sentence with `{variable}` Editor chips
            let segments = parse_prompt_segments(&tpl.body);
            let mut occ_counter: HashMap<SharedString, usize> = HashMap::new();
            let mut sentence_flow = div()
                .mt(px(12.))
                .w_full()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x(px(4.))
                .gap_y(px(6.));

            let mut preview_flow = div()
                .w_full()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x(px(4.))
                .gap_y(px(3.));

            for seg in segments {
                match seg {
                    PromptSegment::Literal(lit) => {
                        for word in lit.split_whitespace() {
                            sentence_flow = sentence_flow.child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(22.))
                                    .text_color(p.ink)
                                    .child(word.to_string()),
                            );
                            preview_flow = preview_flow.child(
                                div()
                                    .text_size(px(12.5))
                                    .line_height(px(19.))
                                    .text_color(p.text)
                                    .child(word.to_string()),
                            );
                        }
                    }
                    PromptSegment::Variable(vname) => {
                        let occ = *occ_counter.get(&vname).unwrap_or(&0);
                        occ_counter.insert(vname.clone(), occ + 1);
                        let cur_val = self.variable_value(&tid, vname.as_ref()).to_string();
                        let trimmed_val = cur_val.trim();
                        let is_filled = !trimmed_val.is_empty();
                        let is_flashing = self
                            .last_flash
                            .as_ref()
                            .is_some_and(|(ft, fv, _)| ft == &tid && fv == &vname)
                            && flash_alpha > 0.02;

                        let char_len = if is_filled {
                            cur_val.chars().count()
                        } else {
                            vname.chars().count()
                        };
                        let chip_w = ((char_len as f32) * 7.5 + 20.0).clamp(62.0, 224.0);

                        let base_bg = if is_filled { p.accent_soft } else { p.chip };
                        let chip_bg = if is_flashing {
                            pro_mix_color(base_bg, p.blue.opacity(0.28), flash_alpha)
                        } else {
                            base_bg
                        };
                        let chip_border = if is_filled {
                            p.blue.opacity(0.36)
                        } else {
                            p.line_strong
                        };

                        let ed_slot = self
                            .var_editors
                            .iter()
                            .find(|e| {
                                e.template_id == tid && e.var_name == vname && e.occurrence == occ
                            })
                            .map(|e| e.editor.clone());

                        let chip = div()
                            .w(px(chip_w))
                            .h(px(23.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(chip_bg)
                            .border_1()
                            .border_color(chip_border)
                            .flex()
                            .items_center()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .children(ed_slot);
                        sentence_flow = sentence_flow.child(chip);

                        // Preview representation (filled highlight vs unfilled `{hole}`)
                        if is_filled {
                            let prev_bg = if is_flashing {
                                p.blue.opacity(0.22 * flash_alpha)
                            } else {
                                transparent_black()
                            };
                            preview_flow = preview_flow.child(
                                div()
                                    .px(px(2.))
                                    .rounded(px(3.))
                                    .bg(prev_bg)
                                    .text_size(px(12.5))
                                    .line_height(px(19.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(trimmed_val.to_string()),
                            );
                        } else {
                            preview_flow = preview_flow.child(
                                div()
                                    .text_size(px(12.5))
                                    .line_height(px(19.))
                                    .text_color(p.faint)
                                    .child(format!("{{{vname}}}")),
                            );
                        }
                    }
                }
            }

            let inner = div()
                .w_full()
                .pt(px(12.))
                .px(px(16.))
                .pb(px(16.))
                .flex()
                .flex_col()
                .child(panel_head)
                .child(sentence_flow)
                .child(
                    div()
                        .mt(px(14.))
                        .text_size(px(10.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(tr(cx, "PREVIEW", "实时预览")),
                )
                .child(
                    div()
                        .mt(px(5.))
                        .w_full()
                        .py(px(9.))
                        .px(px(12.))
                        .rounded(px(8.))
                        .bg(p.subtle)
                        .border_1()
                        .border_color(p.line)
                        .child(preview_flow),
                );

            div()
                .flex_1()
                .min_w_0()
                .child(pro_view_transition(
                    pro_eid(&self.id, "panel-vtx"),
                    tid,
                    inner,
                ))
                .into_any_element()
        } else {
            div().flex_1().into_any_element()
        };

        let (filled_cnt, total_cnt) = self.active_fill_progress();
        let missing = total_cnt.saturating_sub(filled_cnt);
        let ready = missing == 0;
        let est_tokens = self.estimated_tokens() as f64;

        let status_note = if ready {
            div()
                .text_size(px(12.))
                .text_color(p.green_ink)
                .child(tr(cx, "All variables filled", "变量已全部填妥"))
                .into_any_element()
        } else {
            div()
                .text_size(px(12.))
                .text_color(p.muted)
                .child(
                    tr(
                        cx,
                        &format!(
                            "{missing} variable{} missing",
                            if missing == 1 { "" } else { "s" }
                        ),
                        &format!("还差 {missing} 个变量待填写"),
                    )
                    .to_string(),
                )
                .into_any_element()
        };

        let copy_btn = activate(
            div()
                .id(pro_eid(&self.id, "foot-copy"))
                .role(Role::Button)
                .aria_label(tr(cx, "Copy prompt", "复制提示词"))
                .tab_index(0)
                .h(px(28.))
                .pl(px(8.))
                .pr(px(10.))
                .flex_none()
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover).border_color(p.line_strong))
                .focus_visible(move |s| s.border_color(p.focus))
                .child(if recently_copied {
                    pro_icon(ProIcon::Check, 13.5, p.green)
                } else {
                    pro_icon(ProIcon::Copy, 13.5, p.muted)
                })
                .child(tr(
                    cx,
                    if recently_copied { "Copied" } else { "Copy" },
                    if recently_copied {
                        "已复制"
                    } else {
                        "复制"
                    },
                )),
            cx,
            |this, _, cx| {
                this.copy_prompt(cx);
            },
        );

        let use_text = tr(cx, self.use_label.as_ref(), self.use_label_zh.as_ref()).to_string();
        let use_btn = activate(
            div()
                .id(pro_eid(&self.id, "foot-use"))
                .role(Role::Button)
                .aria_label(use_text.clone())
                .tab_index(0)
                .h(px(28.))
                .pl(px(10.))
                .pr(px(8.))
                .flex_none()
                .rounded(px(6.))
                .bg(if ready { p.primary } else { p.chip })
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if ready { p.on_primary } else { p.muted })
                .when(ready, |d| d.cursor_pointer().hover(|s| s.opacity(0.90)))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(use_text)
                .child(pro_icon(
                    ProIcon::ArrowRight,
                    13.5,
                    if ready { p.on_primary } else { p.muted },
                )),
            cx,
            |this, _, cx| {
                this.use_prompt(cx);
            },
        );

        let footer = div()
            .w_full()
            .h(px(44.))
            .pl(px(16.))
            .pr(px(8.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "tok-roll"),
                        est_tokens,
                        0,
                        "~ ",
                        " tokens",
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    ))
                    .child(div().text_size(px(12.)).text_color(p.faint).child("·"))
                    .child(status_note),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(copy_btn)
                    .child(use_btn),
            );

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
                    .max_w(px(640.))
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
                            .flex()
                            .when(narrow, |d| d.flex_col())
                            .child(sidebar)
                            .child(main_panel),
                    )
                    .child(footer),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

fn seed_template_values(
    templates: &[PromptTemplate],
) -> HashMap<SharedString, HashMap<SharedString, String>> {
    let mut all = HashMap::with_capacity(templates.len());
    for tpl in templates {
        let mut map = HashMap::new();
        for (var_name, spec) in &tpl.variables {
            if let Some(def) = &spec.default_value {
                map.insert(var_name.clone(), def.to_string());
            }
        }
        all.insert(tpl.id.clone(), map);
    }
    all
}

fn default_prompt_templates() -> Vec<PromptTemplate> {
    vec![
        PromptTemplate::new(
            "release",
            "Write release notes",
            "Product",
            PromptCategoryTone::Violet,
            "Write release notes for {product} {version} for {audience}. Keep the tone {tone} and group the changes into New, Improved and Fixed.",
        )
        .with_zh("撰写版本发布说明", "产品")
        .with_default_var("version", "v2.4")
        .with_default_var("tone", "clear and concise"),
        PromptTemplate::new(
            "meeting",
            "Summarize a meeting",
            "Notes",
            PromptCategoryTone::Blue,
            "Summarize the meeting transcript for {audience} in under {length}, ending with owned action items for {team}.",
        )
        .with_zh("总结会议纪要", "纪要")
        .with_default_var("length", "150 words"),
        PromptTemplate::new(
            "error",
            "Explain an error",
            "Debug",
            PromptCategoryTone::Amber,
            "Explain this {language} compiler or runtime error in plain words: {error}. Suggest the smallest safe patch.",
        )
        .with_zh("解析报错原因", "调试")
        .with_default_var("language", "Rust"),
        PromptTemplate::new(
            "outreach",
            "Draft a cold email",
            "Sales",
            PromptCategoryTone::Green,
            "Write a short email to {recipient_role} at {company} about {problem}, finishing with one low-friction question.",
        )
        .with_zh("起草客户触达邮件", "商务"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{PromptLibrary, PromptSegment, parse_prompt_segments};
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn template_parser_extracts_variables_and_preserves_malformed_braces() {
        let segs = parse_prompt_segments("Hello {audience}, ship {version} with {bad space}!");
        assert_eq!(
            segs,
            vec![
                PromptSegment::Literal("Hello ".into()),
                PromptSegment::Variable("audience".into()),
                PromptSegment::Literal(", ship ".into()),
                PromptSegment::Variable("version".into()),
                PromptSegment::Literal(" with {bad space}!".into()),
            ]
        );
    }

    #[gpui::test]
    fn filling_variables_updates_live_preview_and_enables_use_prompt(cx: &mut TestAppContext) {
        let lib = cx.new(|cx| PromptLibrary::new("test-prompt-library", cx));

        lib.update(cx, |pl, cx| {
            assert_eq!(pl.active_template_id(), "release");
            assert_eq!(pl.active_fill_progress(), (2, 4));
            assert!(!pl.is_ready());
            assert!(!pl.use_prompt(cx));

            pl.set_variable("release", "product", "KUI", None, cx);
            pl.set_variable("release", "audience", "design engineers", None, cx);
            assert_eq!(pl.active_fill_progress(), (4, 4));
            assert!(pl.is_ready());
            assert!(
                pl.assembled_prompt()
                    .contains("Write release notes for KUI v2.4 for design engineers.")
            );

            // Switching away and back retains typed values
            assert!(pl.select_template("error", cx));
            assert!(!pl.is_ready());
            assert!(pl.select_template("release", cx));
            assert!(pl.is_ready());
            assert!(pl.use_prompt(cx));
        });
    }
}
