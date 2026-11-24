//! Search-and-run command palette with brand logos, shortcut keycaps, and gliding highlight.
//!
//! Renders a `⌘K` command menu with live [`Editor`] filtering, substring match
//! highlighting, grouped SaaS tool actions (`Gmail`, `Linear`, `GitHub`, `Slack`,
//! `Notion`, `Google Calendar`, `HubSpot`), everyday navigation commands, a gliding
//! selection pill, and execution confirmation in the footer tray.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_check_draw, pro_eid, pro_empty_state, pro_icon_btn,
        pro_mark_selected, pro_pill_indicator, pro_selection_sensor, pro_skeleton_bar,
        pro_swap_label, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_raw_svg, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const ICON_RETURN: &str = r#"<path d="M12.5 4v3a2 2 0 0 1-2 2h-7M6 6.5 3.5 9 6 11.5" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#;

const ICON_HELP: &str = r#"<path d="M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6M12 17h.01" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round"/>"#;

const AI_SPARK_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none"><defs><linearGradient id="kui_cm_sp" x1="2" y1="2" x2="22" y2="22" gradientUnits="userSpaceOnUse"><stop stop-color="#7C5CFF"/><stop offset="1" stop-color="#2F9BFF"/></linearGradient></defs><path fill="url(#kui_cm_sp)" d="M12 2.5c.6 3.9 2.1 6.4 4.4 7.6 1.1.6 2.4.9 4.1 1.1v1.6c-4.6.6-7.3 3-8.5 8.7h-1.6c-.6-3.9-2.1-6.4-4.4-7.6-1.1-.6-2.4-.9-4.1-1.1v-1.6c4.6-.6 7.3-3 8.5-8.7z"/></svg>"##;

/// Leading visual mark for a [`CommandItem`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandVisual {
    /// Brand app tile rendered via [`pro_app_tile`] (e.g. `"gmail"`, `"linear"`, `"github"`, `"slack"`, `"notion"`).
    App(SharedString),
    /// Semantic 24x24 line icon from [`ProIcon`].
    Icon(ProIcon),
    /// Multicolor AI assistant spark glyph.
    AiSpark,
    /// Help circle question mark glyph.
    HelpCircle,
}

/// Single executable action row inside a [`CommandGroup`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandItem {
    pub id: SharedString,
    pub label: SharedString,
    pub hint: Option<SharedString>,
    pub visual: CommandVisual,
    pub keys: Vec<SharedString>,
    pub keywords: Vec<SharedString>,
}

impl CommandItem {
    /// Construct a [`CommandItem`] with a semantic [`ProIcon`].
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, icon: ProIcon) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            hint: None,
            visual: CommandVisual::Icon(icon),
            keys: Vec::new(),
            keywords: Vec::new(),
        }
    }

    /// Construct a [`CommandItem`] showing a brand logo tile (`"slack"`, `"linear"`, `"github"`, `"notion"`, `"gmail"`, etc.).
    pub fn app(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        app_name: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            hint: None,
            visual: CommandVisual::App(app_name.into()),
            keys: Vec::new(),
            keywords: Vec::new(),
        }
    }

    /// Attach a muted secondary hint beside the command label.
    pub fn with_hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Override the leading visual variant.
    pub fn with_visual(mut self, visual: CommandVisual) -> Self {
        self.visual = visual;
        self
    }

    /// Attach right-aligned shortcut keycap labels (e.g. `["⌘", "1"]` or `["G", "I"]`).
    pub fn with_keys<I, S>(mut self, keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.keys = keys.into_iter().map(Into::into).collect();
        self
    }

    /// Attach hidden search keywords matched during live filtering.
    pub fn with_keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }
}

/// Named section of [`CommandItem`]s inside [`CommandMenu`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandGroup {
    pub id: SharedString,
    pub label: SharedString,
    pub commands: Vec<CommandItem>,
}

impl CommandGroup {
    /// Construct a [`CommandGroup`] with a section heading and item list.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        commands: Vec<CommandItem>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            commands,
        }
    }
}

/// Typed events emitted by [`CommandMenu`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandMenuEvent {
    /// Highlighted command executed via Enter, click, or `⌘1`–`⌘5`.
    Run {
        id: SharedString,
        label: SharedString,
        query: SharedString,
    },
    /// Search input text changed.
    QueryChanged(SharedString),
    /// Active selection moved to command `id`.
    Highlighted(SharedString),
    /// User pressed `Esc` in an empty search field to dismiss the palette.
    Closed,
}

/// Default four-section command catalog (`Suggestions`, `Share`, `General`, `Go to`).
pub fn default_command_groups(zh: bool) -> Vec<CommandGroup> {
    if zh {
        vec![
            CommandGroup::new(
                "suggestions",
                "快捷建议",
                vec![
                    CommandItem::app("write-email", "撰写邮件", "gmail")
                        .with_hint("Gmail")
                        .with_keys(["⌘", "1"])
                        .with_keywords(["mail", "compose", "email"]),
                    CommandItem::app("create-issue", "新建工作项", "linear")
                        .with_hint("Linear")
                        .with_keys(["⌘", "2"])
                        .with_keywords(["bug", "ticket", "task", "linear"]),
                    CommandItem::app("open-pr", "创建合并请求", "github")
                        .with_hint("GitHub")
                        .with_keys(["⌘", "3"])
                        .with_keywords(["pull request", "branch", "repo", "code"]),
                    CommandItem::app("post-update", "同步团队进展", "slack")
                        .with_hint("#team-updates")
                        .with_keys(["⌘", "4"])
                        .with_keywords(["slack", "message", "channel", "status"]),
                    CommandItem::app("create-page", "新建知识库文档", "notion")
                        .with_hint("Notion")
                        .with_keys(["⌘", "5"])
                        .with_keywords(["doc", "wiki", "notes", "spec"]),
                ],
            ),
            CommandGroup::new(
                "share",
                "协作流转",
                vec![
                    CommandItem::app("schedule-meeting", "预约评审日历", "google calendar")
                        .with_hint("Google Calendar")
                        .with_keywords(["calendar", "invite", "call"]),
                    CommandItem::app("add-crm", "添加客户联系人", "hubspot")
                        .with_hint("HubSpot")
                        .with_keywords(["crm", "lead", "account"]),
                ],
            ),
            CommandGroup::new(
                "general",
                "常用操作",
                vec![
                    CommandItem::new("new-doc", "新建空白文稿", ProIcon::PageDoc)
                        .with_keys(["N"])
                        .with_keywords(["document", "draft"]),
                    CommandItem::new("invite-member", "邀请团队成员", ProIcon::Users)
                        .with_keys(["I"])
                        .with_keywords(["teammate", "share", "access"]),
                    CommandItem::new("import-files", "导入本地文件", ProIcon::Upload)
                        .with_keys(["U"])
                        .with_keywords(["upload", "csv", "pdf"]),
                    CommandItem::new("toggle-theme", "切换深浅外观", ProIcon::Sparkles)
                        .with_keys(["⌘", "J"])
                        .with_keywords(["dark", "light", "appearance"]),
                ],
            ),
            CommandGroup::new(
                "goto",
                "前往页面",
                vec![
                    CommandItem::new("go-home", "工作台首页", ProIcon::Home)
                        .with_keys(["G", "H"])
                        .with_keywords(["dashboard", "overview"]),
                    CommandItem::new("go-inbox", "收件箱", ProIcon::Inbox)
                        .with_keys(["G", "I"])
                        .with_keywords(["notifications", "messages"]),
                    CommandItem::new("go-analytics", "数据洞察", ProIcon::UsageChart)
                        .with_keys(["G", "A"])
                        .with_keywords(["metrics", "charts", "usage"]),
                    CommandItem::new("go-settings", "系统设置", ProIcon::Settings)
                        .with_keys(["G", "S"])
                        .with_keywords(["preferences", "account"]),
                    CommandItem::new("go-help", "帮助与快捷键", ProIcon::Globe)
                        .with_visual(CommandVisual::HelpCircle)
                        .with_keys(["?"])
                        .with_keywords(["docs", "shortcuts", "support"]),
                ],
            ),
        ]
    } else {
        vec![
            CommandGroup::new(
                "suggestions",
                "Suggestions",
                vec![
                    CommandItem::app("write-email", "Write an email", "gmail")
                        .with_hint("Gmail")
                        .with_keys(["⌘", "1"])
                        .with_keywords(["mail", "compose", "draft"]),
                    CommandItem::app("create-issue", "Create an issue", "linear")
                        .with_hint("Linear")
                        .with_keys(["⌘", "2"])
                        .with_keywords(["bug", "ticket", "task"]),
                    CommandItem::app("open-pr", "Open pull request", "github")
                        .with_hint("GitHub")
                        .with_keys(["⌘", "3"])
                        .with_keywords(["branch", "merge", "code", "repo"]),
                    CommandItem::app("post-update", "Post an update", "slack")
                        .with_hint("#team-updates")
                        .with_keys(["⌘", "4"])
                        .with_keywords(["slack", "channel", "message"]),
                    CommandItem::app("create-page", "Create a spec page", "notion")
                        .with_hint("Notion")
                        .with_keys(["⌘", "5"])
                        .with_keywords(["wiki", "notes", "doc"]),
                ],
            ),
            CommandGroup::new(
                "share",
                "Share",
                vec![
                    CommandItem::app("schedule-meeting", "Schedule a meeting", "google calendar")
                        .with_hint("Google Calendar")
                        .with_keywords(["calendar", "invite", "sync"]),
                    CommandItem::app("add-crm", "Add contact to CRM", "hubspot")
                        .with_hint("HubSpot")
                        .with_keywords(["lead", "deal", "customer"]),
                ],
            ),
            CommandGroup::new(
                "general",
                "General",
                vec![
                    CommandItem::new("new-doc", "New document", ProIcon::PageDoc)
                        .with_keys(["N"])
                        .with_keywords(["create", "blank", "file"]),
                    CommandItem::new("invite-member", "Invite teammate", ProIcon::Users)
                        .with_keys(["I"])
                        .with_keywords(["member", "share", "team"]),
                    CommandItem::new("import-files", "Import files", ProIcon::Upload)
                        .with_keys(["U"])
                        .with_keywords(["upload", "csv", "pdf"]),
                    CommandItem::new("toggle-theme", "Switch theme", ProIcon::Sparkles)
                        .with_keys(["⌘", "J"])
                        .with_keywords(["dark", "light", "mode"]),
                ],
            ),
            CommandGroup::new(
                "goto",
                "Go to",
                vec![
                    CommandItem::new("go-home", "Home", ProIcon::Home)
                        .with_keys(["G", "H"])
                        .with_keywords(["dashboard", "start"]),
                    CommandItem::new("go-inbox", "Inbox", ProIcon::Inbox)
                        .with_keys(["G", "I"])
                        .with_keywords(["messages", "unread"]),
                    CommandItem::new("go-analytics", "Analytics", ProIcon::UsageChart)
                        .with_keys(["G", "A"])
                        .with_keywords(["metrics", "reports", "usage"]),
                    CommandItem::new("go-settings", "Settings", ProIcon::Settings)
                        .with_keys(["G", "S"])
                        .with_keywords(["preferences", "workspace"]),
                    CommandItem::new("go-help", "Help & shortcuts", ProIcon::Globe)
                        .with_visual(CommandVisual::HelpCircle)
                        .with_keys(["?"])
                        .with_keywords(["documentation", "support"]),
                ],
            ),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VisibleEntry {
    pub group_idx: usize,
    pub group_label: SharedString,
    pub item: CommandItem,
    pub is_fallback: bool,
}

pub(crate) fn filter_command_entries(
    groups: &[CommandGroup],
    query: &str,
    zh: bool,
) -> Vec<VisibleEntry> {
    let q = query.trim();
    let q_lower = q.to_lowercase();
    let mut out = Vec::new();

    for (g_idx, group) in groups.iter().enumerate() {
        let g_match = !q_lower.is_empty() && group.label.to_lowercase().contains(&q_lower);
        for item in &group.commands {
            let matched = q_lower.is_empty()
                || g_match
                || item.label.to_lowercase().contains(&q_lower)
                || item
                    .hint
                    .as_ref()
                    .is_some_and(|h| h.to_lowercase().contains(&q_lower))
                || item
                    .keywords
                    .iter()
                    .any(|kw| kw.to_lowercase().contains(&q_lower));
            if matched {
                out.push(VisibleEntry {
                    group_idx: g_idx,
                    group_label: group.label.clone(),
                    item: item.clone(),
                    is_fallback: false,
                });
            }
        }
    }

    if !q.is_empty() {
        let ask_label: SharedString = if zh {
            "询问 AI 助手".into()
        } else {
            "Ask assistant".into()
        };
        let ai_group: SharedString = if zh {
            "智能助手".into()
        } else {
            "Assistant".into()
        };
        out.push(VisibleEntry {
            group_idx: groups.len(),
            group_label: ai_group,
            item: CommandItem {
                id: "ask".into(),
                label: ask_label,
                hint: Some(format!("“{q}”").into()),
                visual: CommandVisual::AiSpark,
                keys: vec!["↵".into()],
                keywords: Vec::new(),
            },
            is_fallback: true,
        });
    }

    out
}

/// Split `text` into `(segment, is_highlighted)` slices for case-insensitive `query`.
pub fn split_highlight_segments(text: &str, query: &str) -> Vec<(String, bool)> {
    let q = query.trim();
    if q.is_empty() || text.is_empty() {
        return vec![(text.to_string(), false)];
    }
    let text_chars: Vec<char> = text.chars().collect();
    let q_chars: Vec<char> = q.chars().flat_map(char::to_lowercase).collect();
    if q_chars.is_empty() {
        return vec![(text.to_string(), false)];
    }

    let lowered_map: Vec<(usize, Vec<char>)> = text_chars
        .iter()
        .enumerate()
        .map(|(idx, &c)| (idx, c.to_lowercase().collect::<Vec<_>>()))
        .collect();

    let mut flat_lower = Vec::new();
    let mut flat_to_char = Vec::new();
    for (c_idx, low_seq) in &lowered_map {
        for &lc in low_seq {
            flat_lower.push(lc);
            flat_to_char.push(*c_idx);
        }
    }

    let mut match_mask = vec![false; text_chars.len()];
    let mut i = 0usize;
    while i + q_chars.len() <= flat_lower.len() {
        if flat_lower[i..i + q_chars.len()] == q_chars[..] {
            let start_c = flat_to_char[i];
            let end_c = flat_to_char[i + q_chars.len() - 1];
            for flag in &mut match_mask[start_c..=end_c] {
                *flag = true;
            }
            i += q_chars.len();
        } else {
            i += 1;
        }
    }

    let mut segments: Vec<(String, bool)> = Vec::new();
    for (idx, ch) in text_chars.into_iter().enumerate() {
        let hit = match_mask[idx];
        if let Some(last) = segments.last_mut()
            && last.1 == hit
        {
            last.0.push(ch);
        } else {
            segments.push((ch.to_string(), hit));
        }
    }
    segments
}

fn render_highlighted_text(text: &str, query: &str, fg: Hsla, mark_bg: Hsla) -> AnyElement {
    let segs = split_highlight_segments(text, query);
    if segs.len() == 1 && !segs[0].1 {
        return div()
            .text_color(fg)
            .truncate()
            .child(text.to_string())
            .into_any_element();
    }
    let mut row = div().flex().items_center().overflow_hidden();
    for (part, hit) in segs {
        let safe = part.replace(' ', "\u{00a0}");
        if hit {
            row = row.child(
                div()
                    .px(px(1.5))
                    .rounded(px(3.))
                    .bg(mark_bg)
                    .text_color(fg)
                    .child(safe),
            );
        } else {
            row = row.child(div().text_color(fg).child(safe));
        }
    }
    row.into_any_element()
}

fn render_keycap(label: &str, active: bool, p: &ProPalette) -> Div {
    div()
        .min_w(px(20.))
        .h(px(20.))
        .px(px(5.))
        .rounded(px(5.))
        .border_1()
        .border_color(p.line_strong)
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(11.))
        .line_height(px(16.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(if active { p.text } else { p.muted })
        .child(label.to_string())
}

#[derive(IntoElement)]
struct CmdRowTactile {
    id: ElementId,
    pressed: bool,
    active: bool,
    dark: bool,
    content: AnyElement,
}

impl RenderOnce for CmdRowTactile {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let reduced = disabled_motion(cx);
        let target = if self.pressed { 1.0 } else { 0.0 };
        let st = w.use_keyed_state(
            ElementId::Name(format!("{}-press", self.id).into()),
            cx,
            |_, _| (self.pressed, Slide::snappy(target)),
        );
        let prev = st.read(cx).0;
        if prev != self.pressed {
            st.update(cx, |s, cx| {
                s.0 = self.pressed;
                if reduced {
                    s.1.reset(target);
                } else {
                    s.1.duration(target, 0.11, cx);
                }
            });
        }
        let amt = if reduced {
            target
        } else {
            st.update(cx, |s, cx| s.1.get(w, cx).clamp(0.0, 1.0))
        };
        let flash_bg = if amt > 0.01 {
            if self.dark {
                hsla(0., 0., 1., 0.06 * amt)
            } else {
                hsla(0., 0., 0., 0.05 * amt)
            }
        } else {
            transparent_black()
        };
        let _ = self.active;
        div()
            .size_full()
            .rounded(px(6.))
            .bg(flash_bg)
            .relative()
            .top(px(1.25 * amt))
            .px(px(8. + 2.0 * amt))
            .flex()
            .items_center()
            .gap(px(10.))
            .child(self.content)
    }
}

/// Search-and-run command palette with brand logos, `⌘1`–`⌘5` shortcuts, and gliding highlight.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::command_menu::{CommandMenu, CommandMenuEvent};
///
/// # fn doc(cx: &mut gpui::App) {
/// let menu = cx.new(|cx| CommandMenu::new("workspace-cmd", cx));
/// cx.subscribe(&menu, |_, ev: &CommandMenuEvent, _| {
///     let _ = ev;
/// })
/// .detach();
/// # }
/// ```
pub struct CommandMenu {
    id: ElementId,
    custom_groups: Option<Vec<CommandGroup>>,
    placeholder: Option<SharedString>,
    auto_focus: bool,
    did_auto_focus: bool,
    active_idx: usize,
    pressed_cmd: Option<SharedString>,
    last_ran: Option<(SharedString, SharedString, Instant)>,
    search_editor: Option<Entity<Editor>>,
    editor_lang_zh: Option<bool>,
    measured_width: Rc<Cell<f32>>,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl EventEmitter<CommandMenuEvent> for CommandMenu {}

impl CommandMenu {
    /// Construct a [`CommandMenu`] with default `Suggestions`, `Share`, `General`, and `Go to` groups.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            custom_groups: None,
            placeholder: None,
            auto_focus: false,
            did_auto_focus: false,
            active_idx: 0,
            pressed_cmd: None,
            last_ran: None,
            search_editor: None,
            editor_lang_zh: None,
            measured_width: Rc::new(Cell::new(600.)),
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Replace the command groups rendered in the palette.
    pub fn with_groups(mut self, groups: Vec<CommandGroup>) -> Self {
        self.custom_groups = Some(groups);
        self
    }

    /// Override the search input placeholder string.
    pub fn with_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Focus the search input automatically on first render.
    pub fn with_auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    /// Set the initially highlighted row index.
    pub fn with_selected(mut self, index: usize) -> Self {
        self.active_idx = index;
        self
    }

    /// Enable or disable the loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the empty state override.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently highlighted flattened command index.
    pub fn active_index(&self) -> usize {
        self.active_idx
    }

    /// Return the current search query string.
    pub fn query(&self, cx: &App) -> String {
        self.search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default()
    }

    /// Return the most recently executed `(id, label)` command if any.
    pub fn last_executed(&self) -> Option<(&SharedString, &SharedString)> {
        self.last_ran.as_ref().map(|(id, lbl, _)| (id, lbl))
    }

    fn resolved_groups(&self, cx: &App) -> Vec<CommandGroup> {
        if let Some(g) = &self.custom_groups {
            return g.clone();
        }
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        default_command_groups(zh)
    }

    fn visible_entries(&self, cx: &App) -> Vec<VisibleEntry> {
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        let q = self.query(cx);
        let groups = self.resolved_groups(cx);
        filter_command_entries(&groups, &q, zh)
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let entries = self.visible_entries(cx);
        if entries.is_empty() {
            self.active_idx = 0;
            return;
        }
        let len = entries.len() as isize;
        let cur = (self.active_idx.min(entries.len() - 1)) as isize;
        let next = (cur + delta).rem_euclid(len) as usize;
        self.active_idx = next;
        let id = entries[next].item.id.clone();
        cx.emit(CommandMenuEvent::Highlighted(id));
        cx.notify();
    }

    fn jump_edge(&mut self, to_end: bool, cx: &mut Context<Self>) {
        let entries = self.visible_entries(cx);
        if entries.is_empty() {
            return;
        }
        self.active_idx = if to_end { entries.len() - 1 } else { 0 };
        let id = entries[self.active_idx].item.id.clone();
        cx.emit(CommandMenuEvent::Highlighted(id));
        cx.notify();
    }

    fn jump_section(&mut self, backwards: bool, cx: &mut Context<Self>) {
        let entries = self.visible_entries(cx);
        if entries.is_empty() {
            return;
        }
        let cur = self.active_idx.min(entries.len() - 1);
        let cur_group = entries[cur].group_idx;
        let mut group_starts: Vec<(usize, usize)> = Vec::new();
        for (idx, e) in entries.iter().enumerate() {
            if group_starts.last().map(|&(g, _)| g) != Some(e.group_idx) {
                group_starts.push((e.group_idx, idx));
            }
        }
        if group_starts.len() <= 1 {
            return;
        }
        let pos = group_starts
            .iter()
            .position(|&(g, _)| g == cur_group)
            .unwrap_or(0);
        let next_pos = if backwards {
            (pos + group_starts.len() - 1) % group_starts.len()
        } else {
            (pos + 1) % group_starts.len()
        };
        self.active_idx = group_starts[next_pos].1;
        let id = entries[self.active_idx].item.id.clone();
        cx.emit(CommandMenuEvent::Highlighted(id));
        cx.notify();
    }

    /// Execute the command at `visible_idx` inside the current filtered list.
    pub fn run_visible_index(&mut self, visible_idx: usize, cx: &mut Context<Self>) {
        let entries = self.visible_entries(cx);
        let Some(entry) = entries.get(visible_idx) else {
            return;
        };
        self.active_idx = visible_idx;
        let query: SharedString = self.query(cx).trim().to_string().into();
        let id = entry.item.id.clone();
        let display_label: SharedString = if entry.is_fallback {
            format!("{}: “{}”", entry.item.label, query).into()
        } else if let Some(h) = &entry.item.hint {
            format!("{} · {}", entry.item.label, h).into()
        } else {
            entry.item.label.clone()
        };
        self.last_ran = Some((id.clone(), display_label, Instant::now()));
        cx.emit(CommandMenuEvent::Run {
            id,
            label: entry.item.label.clone(),
            query,
        });
        cx.notify();
    }

    fn run_suggestion_shortcut(&mut self, slot: usize, cx: &mut Context<Self>) -> bool {
        let groups = self.resolved_groups(cx);
        let Some(sug) = groups
            .iter()
            .find(|g| g.id.as_ref() == "suggestions")
            .or_else(|| groups.first())
        else {
            return false;
        };
        let Some(cmd) = sug.commands.get(slot) else {
            return false;
        };
        let target_id = cmd.id.clone();
        let entries = self.visible_entries(cx);
        if let Some(pos) = entries.iter().position(|e| e.item.id == target_id) {
            self.active_idx = pos;
        }
        let display_label: SharedString = if let Some(h) = &cmd.hint {
            format!("{} · {}", cmd.label, h).into()
        } else {
            cmd.label.clone()
        };
        let query: SharedString = self.query(cx).trim().to_string().into();
        self.last_ran = Some((cmd.id.clone(), display_label, Instant::now()));
        cx.emit(CommandMenuEvent::Run {
            id: cmd.id.clone(),
            label: cmd.label.clone(),
            query,
        });
        cx.notify();
        true
    }

    fn handle_escape(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let has_query = !self.query(cx).is_empty();
        if has_query {
            if let Some(ed) = self.search_editor.clone() {
                ed.update(cx, |e, cx| e.set_value("", w, cx));
            }
            self.active_idx = 0;
            cx.emit(CommandMenuEvent::QueryChanged("".into()));
            cx.notify();
        } else {
            cx.emit(CommandMenuEvent::Closed);
            cx.notify();
        }
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        let ph: SharedString = self
            .placeholder
            .clone()
            .unwrap_or_else(|| tr(cx, "Type a command or search…", "输入命令或搜索…").into());
        if let Some(ed) = &self.search_editor {
            if self.editor_lang_zh != Some(zh) {
                self.editor_lang_zh = Some(zh);
                ed.update(cx, |e, cx| {
                    e.set_placeholder(ph);
                    cx.notify();
                });
            }
            if self.auto_focus && !self.did_auto_focus {
                self.did_auto_focus = true;
                ed.clone().update(cx, |e, cx| e.focus(w, cx));
            }
            return;
        }
        self.editor_lang_zh = Some(zh);
        let ed = cx.new(|cx| {
            let mut e = Editor::new(tr(cx, "Command", "搜索命令"), InputMode::Combobox, w, cx);
            e.set_placeholder(ph);
            e.font_size = 13.;
            e
        });
        if self.auto_focus {
            self.did_auto_focus = true;
            ed.clone().update(cx, |e, cx| e.focus(w, cx));
        }
        cx.subscribe(&ed, |this, ed_ent, ev: &EditEvent, cx| match ev {
            EditEvent::Changed => {
                this.active_idx = 0;
                let q: SharedString = ed_ent.read(cx).value().to_string().into();
                cx.emit(CommandMenuEvent::QueryChanged(q));
                cx.notify();
            }
            EditEvent::Navigate(down) => {
                this.move_selection(if *down { 1 } else { -1 }, cx);
            }
            EditEvent::Submitted => {
                let idx = this.active_idx;
                this.run_visible_index(idx, cx);
            }
            _ => {}
        })
        .detach();
        self.search_editor = Some(ed);
    }
}

impl Render for CommandMenu {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editor(w, cx);
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let compact = width < 480.;

        if self.loading {
            if !reduced {
                w.request_animation_frame();
            }
            let phase = (self.started_at.elapsed().as_secs_f32() / 1.4).fract();
            let card_bg: Hsla = if p.dark {
                rgb(0x1c1c1e).into()
            } else {
                rgb(0xffffff).into()
            };
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(600.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .shadow_lg()
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .h(px(48.))
                                .px(px(14.))
                                .border_b_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .gap(px(12.))
                                .child(pro_skeleton_bar(16., 16., 4., phase, cx))
                                .child(pro_skeleton_bar(210., 14., 5., phase + 0.1, cx)),
                        )
                        .child(
                            div()
                                .p(px(12.))
                                .flex()
                                .flex_col()
                                .gap(px(10.))
                                .child(pro_skeleton_bar(84., 10., 4., phase + 0.16, cx))
                                .child(pro_skeleton_bar(560., 32., 6., phase + 0.22, cx))
                                .child(pro_skeleton_bar(560., 32., 6., phase + 0.28, cx))
                                .child(pro_skeleton_bar(560., 32., 6., phase + 0.34, cx))
                                .child(pro_skeleton_bar(560., 32., 6., phase + 0.40, cx)),
                        ),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(600.))
                .rounded(px(12.))
                .bg(if p.dark { rgb(0x1c1c1e) } else { rgb(0xffffff) })
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Terminal,
                    tr(cx, "No commands registered", "暂无可用命令"),
                    tr(
                        cx,
                        "Register workspace or integration commands to search and launch them from one bar.",
                        "注册工作区或集成应用命令后即可在此快速检索执行。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.08)
        };
        let pill_fill: Hsla = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let mark_bg: Hsla = if p.dark {
            hsla(218. / 360., 1.0, 0.65, 0.25)
        } else {
            hsla(214. / 360., 1.0, 0.44, 0.13)
        };

        let query_str = self.query(cx);
        let entries = self.visible_entries(cx);
        let clamped_active = if entries.is_empty() {
            0
        } else {
            self.active_idx.min(entries.len() - 1)
        };
        self.active_idx = clamped_active;

        // Check whether recent execution feedback is still fresh (2.5s window)
        let recent_done = self.last_ran.as_ref().and_then(|(_, lbl, started)| {
            let age = started.elapsed().as_secs_f32();
            if age < 2.5 { Some(lbl.clone()) } else { None }
        });
        if recent_done.is_some() && !reduced {
            w.request_animation_frame();
        }

        let editor_el = self
            .search_editor
            .as_ref()
            .map(|ed| ed.clone().into_any_element())
            .unwrap_or_else(|| div().into_any_element());

        let search_bar = div()
            .id(pro_eid(&self.id, "search-row"))
            .h(px(48.))
            .pl(px(14.))
            .pr(px(12.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(12.))
            .on_click(cx.listener(|v, _, w, cx| {
                if let Some(ed) = &v.search_editor {
                    ed.clone().update(cx, |e, cx| e.focus(w, cx));
                }
            }))
            .child(pro_icon(ProIcon::Search, 16., p.muted))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .child(editor_el),
            )
            .when(!query_str.is_empty(), |d| {
                let clear_lbl: SharedString = tr(cx, "Clear search", "清空搜索").into();
                d.child(pro_icon_btn(
                    pro_eid(&self.id, "clear-q"),
                    clear_lbl,
                    ProIcon::Close,
                    22.,
                    false,
                    cx,
                    |v, w, cx| {
                        if let Some(ed) = v.search_editor.clone() {
                            ed.update(cx, |e, cx| {
                                e.set_value("", w, cx);
                                e.focus(w, cx);
                            });
                        }
                        v.active_idx = 0;
                        cx.emit(CommandMenuEvent::QueryChanged("".into()));
                        cx.notify();
                    },
                ))
            })
            .when(!compact, |d| {
                d.child(
                    div()
                        .w(px(22.))
                        .h(px(20.))
                        .rounded(px(5.))
                        .border_1()
                        .border_color(p.line_strong)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_svg("0 0 16 16", ICON_RETURN, 12., p.muted)),
                )
            });

        // Build grouped rows inside a `.relative()` wrapper with `pro_pill_indicator`
        let sensor = pro_selection_sensor();
        let active_key: SharedString = entries
            .get(clamped_active)
            .map(|e| format!("row-{}-{}", e.group_idx, e.item.id).into())
            .unwrap_or_else(|| "none".into());

        let mut inner_col = div()
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "active-pill"),
                active_key,
                sensor.clone(),
                6.,
                pill_fill,
                None,
                false,
            ));

        let mut current_group_idx: Option<usize> = None;
        for (v_idx, entry) in entries.iter().enumerate() {
            if current_group_idx != Some(entry.group_idx) {
                current_group_idx = Some(entry.group_idx);
                inner_col = inner_col.child(
                    div()
                        .pt(px(10.))
                        .pb(px(4.))
                        .px(px(8.))
                        .text_size(px(10.5))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(entry.group_label.to_uppercase()),
                );
            }

            let is_active = v_idx == clamped_active;
            let row_id = pro_eid(&self.id, format!("cmd-row-{v_idx}-{}", entry.item.id));
            let lead_visual = match &entry.item.visual {
                CommandVisual::App(app_name) => pro_app_tile(app_name.as_ref(), 20., cx),
                CommandVisual::Icon(ic) => div()
                    .size(px(20.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(*ic, 16., if is_active { p.ink } else { p.muted }))
                    .into_any_element(),
                CommandVisual::AiSpark => div()
                    .size(px(20.))
                    .flex_none()
                    .rounded(px(5.))
                    .bg(if p.dark { rgb(0x242428) } else { rgb(0xffffff) })
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_raw_svg(AI_SPARK_SVG, 13., 13.))
                    .into_any_element(),
                CommandVisual::HelpCircle => div()
                    .size(px(20.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_svg(
                        "0 0 24 24",
                        ICON_HELP,
                        16.,
                        if is_active { p.ink } else { p.muted },
                    ))
                    .into_any_element(),
            };

            let name_el =
                render_highlighted_text(entry.item.label.as_ref(), &query_str, p.ink, mark_bg);
            let hint_el = if !compact && let Some(hint) = &entry.item.hint {
                Some(if entry.is_fallback {
                    div()
                        .text_color(p.muted)
                        .truncate()
                        .child(hint.clone())
                        .into_any_element()
                } else {
                    render_highlighted_text(hint.as_ref(), &query_str, p.muted, mark_bg)
                })
            } else {
                None
            };

            let mut keys_row = div().flex_none().flex().items_center().gap(px(2.));
            if !compact {
                for k in &entry.item.keys {
                    keys_row = keys_row.child(render_keycap(k.as_ref(), is_active, &p));
                }
            }

            let aria_lbl = entry.item.label.clone();
            let cmd_id = entry.item.id.clone();
            let cmd_id_down = cmd_id.clone();
            let cmd_id_up = cmd_id.clone();
            let exec_flash = self.last_ran.as_ref().is_some_and(|(ran_id, _, t)| {
                *ran_id == cmd_id && t.elapsed().as_secs_f32() < 0.14
            });
            let is_pressed = self.pressed_cmd.as_ref() == Some(&cmd_id) || exec_flash;
            let row_inner = div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(lead_visual)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .child(name_el)
                        .when_some(hint_el, |d, h| d.child(h)),
                )
                .child(keys_row)
                .into_any_element();

            inner_col = inner_col.child(
                div()
                    .id(row_id.clone())
                    .role(Role::ListItem)
                    .aria_label(aria_lbl)
                    .relative()
                    .h(px(36.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .when(is_active, |d| d.child(pro_mark_selected(&sensor)))
                    .on_hover(cx.listener(move |v, inside: &bool, _, cx| {
                        if *inside && v.active_idx != v_idx {
                            v.active_idx = v_idx;
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |v, _, _, cx| {
                            v.pressed_cmd = Some(cmd_id_down.clone());
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |v, _, _, cx| {
                            if v.pressed_cmd.as_ref() == Some(&cmd_id_up) {
                                v.pressed_cmd = None;
                                cx.notify();
                            }
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|v, _, _, cx| {
                            if v.pressed_cmd.is_some() {
                                v.pressed_cmd = None;
                                cx.notify();
                            }
                        }),
                    )
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.pressed_cmd = None;
                        v.run_visible_index(v_idx, cx);
                    }))
                    .child(CmdRowTactile {
                        id: row_id,
                        pressed: is_pressed,
                        active: is_active,
                        dark: p.dark,
                        content: row_inner,
                    }),
            );
        }

        let list_area = div()
            .id(pro_eid(&self.id, "cmd-scroll"))
            .max_h(px(360.))
            .px(px(4.))
            .pb(px(4.))
            .overflow_y_scroll()
            .child(inner_col);

        // Footer tray: shows execution confirmation or keyboard tips
        let has_done = recent_done.is_some();
        let done_chk = pro_check_draw(
            pro_eid(&self.id, "foot-ran-chk"),
            has_done,
            15.,
            false,
            p.green,
        );
        let footer_left = if let Some(done_label) = recent_done {
            let ran_prefix = tr(cx, "Ran", "已执行");
            let full_done: SharedString = format!("{ran_prefix} {done_label}").into();
            pro_view_transition(
                pro_eid(&self.id, "foot-vtx"),
                "done-state",
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .min_w_0()
                    .text_color(p.ink)
                    .child(done_chk)
                    .child(div().min_w_0().truncate().child(pro_swap_label(
                        pro_eid(&self.id, "foot-ran-lbl"),
                        full_done,
                        12.,
                        FontWeight::MEDIUM,
                        p.ink,
                    ))),
            )
        } else if !compact {
            pro_view_transition(
                pro_eid(&self.id, "foot-vtx"),
                "idle-tip",
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(
                        div()
                            .size(px(0.))
                            .overflow_hidden()
                            .opacity(0.)
                            .child(done_chk),
                    )
                    .child(render_keycap("Tab", false, &p))
                    .child(div().text_color(p.muted).child(tr(
                        cx,
                        "to jump between sections",
                        "跨分组跳转",
                    ))),
            )
        } else {
            div()
                .size(px(0.))
                .overflow_hidden()
                .opacity(0.)
                .child(done_chk)
                .into_any_element()
        };

        let show_footer = !compact || self.last_ran.is_some();
        let footer = if show_footer {
            Some(
                div()
                    .h(px(40.))
                    .px(px(12.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .child(div().min_w_0().flex_1().child(footer_left))
                    .when(!compact, |d| {
                        d.child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_color(p.muted)
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(2.))
                                        .child(render_keycap("⌘", false, &p))
                                        .child(render_keycap("K", false, &p)),
                                )
                                .child(tr(cx, "to open", "随时唤起")),
                        )
                    }),
            )
        } else {
            None
        };

        let width_cell = self.measured_width.clone();
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(b) = bounds.first() {
                    let w_px = f32::from(b.size.width);
                    if w_px > 10. {
                        width_cell.set(w_px);
                    }
                }
            })
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .capture_key_down(cx.listener(|v, ev: &KeyDownEvent, w, cx| {
                let m = ev.keystroke.modifiers;
                let cmd = m.platform || m.control;
                let k = ev.keystroke.key.as_str();
                if cmd && !m.alt && !m.shift {
                    let slot = match k {
                        "1" => Some(0),
                        "2" => Some(1),
                        "3" => Some(2),
                        "4" => Some(3),
                        "5" => Some(4),
                        _ => None,
                    };
                    if let Some(s) = slot
                        && v.run_suggestion_shortcut(s, cx)
                    {
                        w.prevent_default();
                        cx.stop_propagation();
                        return;
                    }
                    if k == "k" {
                        if let Some(ed) = &v.search_editor {
                            ed.clone().update(cx, |e, cx| e.focus(w, cx));
                        }
                        w.prevent_default();
                        cx.stop_propagation();
                        return;
                    }
                }
                match k {
                    "tab" if !cmd && !m.alt => {
                        v.jump_section(m.shift, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "home" if v.query(cx).is_empty() => {
                        v.jump_edge(false, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "end" if v.query(cx).is_empty() => {
                        v.jump_edge(true, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "escape" => {
                        v.handle_escape(w, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .on_key_down(cx.listener(
                |v, ev: &KeyDownEvent, w, cx| match ev.keystroke.key.as_str() {
                    "up" => {
                        v.move_selection(-1, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "down" => {
                        v.move_selection(1, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "home" => {
                        v.jump_edge(false, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "end" => {
                        v.jump_edge(true, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "enter" => {
                        let idx = v.active_idx;
                        v.run_visible_index(idx, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    _ => {}
                },
            ))
            .child(
                div()
                    .w_full()
                    .max_w(px(600.))
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow_lg()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(search_bar)
                    .child(list_area)
                    .when_some(footer, |d, f| d.child(f)),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{
        CommandMenu, default_command_groups, filter_command_entries, split_highlight_segments,
    };
    use gpui::TestAppContext;

    #[test]
    fn filtering_matches_labels_hints_and_appends_ai_fallback() {
        let groups = default_command_groups(false);
        let all = filter_command_entries(&groups, "", false);
        assert_eq!(all.len(), 16);

        let linear = filter_command_entries(&groups, "linear", false);
        assert_eq!(linear.len(), 2); // Create an issue + Ask assistant fallback
        assert_eq!(linear[0].item.id.as_ref(), "create-issue");
        assert!(linear[1].is_fallback);

        let unmatched = filter_command_entries(&groups, "summarize Q3 board deck", false);
        assert_eq!(unmatched.len(), 1);
        assert_eq!(unmatched[0].item.id.as_ref(), "ask");
    }

    #[test]
    fn highlight_segmentation_is_case_insensitive_and_exact() {
        let segs = split_highlight_segments("Open pull request", "PULL");
        assert_eq!(
            segs,
            vec![
                ("Open ".to_string(), false),
                ("pull".to_string(), true),
                (" request".to_string(), false),
            ]
        );
    }

    #[gpui::test]
    fn navigation_section_jump_and_shortcut_execution(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let menu = cx.new(|cx| CommandMenu::new("cm-test", cx));
        menu.update(cx, |m, cx| {
            assert_eq!(m.active_index(), 0);
            m.move_selection(1, cx);
            assert_eq!(m.active_index(), 1);
            m.jump_section(false, cx);
            assert_eq!(m.active_index(), 5); // Share group first item
            m.jump_edge(true, cx);
            assert_eq!(m.active_index(), 15);
            assert!(m.run_suggestion_shortcut(2, cx));
            assert_eq!(
                m.last_executed().map(|(id, _)| id.as_ref()),
                Some("open-pr")
            );
        });
    }
}
