//! Collaborative and AI comment feed with filter pills, `/edit` commands, and `@mentions`.
//!
//! Provides a document or workspace comments sidebar featuring sliding filter
//! pills (`Open`, `Resolved`, `All`), runnable `/edit` slash commands inside
//! comments and replies, resolve/reopen transitions, `@mention` pills, and an
//! inline [`Editor`] composer.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_agent_avatar, pro_card_shadow, pro_check_draw, pro_eid, pro_empty_state,
        pro_fold, pro_icon_btn, pro_mark_selected, pro_person_avatar, pro_pill_indicator,
        pro_popover_presence, pro_selection_sensor, pro_shimmer_text, pro_skeleton_bar,
        pro_swap_label, pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SVG_SORT_LINES: &str = r#"<path d="M2.5 4.5h11M4.5 8h7M6.5 11.5h3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"></path>"#;
const SVG_SPIN_ARC: &str = r#"<circle cx="8" cy="8" r="5.8" stroke="currentColor" stroke-opacity="0.22" stroke-width="1.5"></circle><path d="M8 2.2A5.8 5.8 0 0 1 13.8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"></path>"#;

/// Active status filter tab in [`CommentsPanel`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CommentFilter {
    /// Unresolved comments requiring attention.
    #[default]
    Open,
    /// Completed and resolved threads.
    Resolved,
    /// Every comment regardless of resolution status.
    All,
}

impl CommentFilter {
    fn key(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::All => "all",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Open => tr(cx, "Open", "待处理"),
            Self::Resolved => tr(cx, "Resolved", "已解决"),
            Self::All => tr(cx, "All", "全部"),
        }
    }
}

/// Ordering applied to visible threads in [`CommentsPanel`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CommentSort {
    /// Most recent comments at the top.
    #[default]
    Newest,
    /// Chronological order from oldest to newest.
    Oldest,
    /// Threads containing runnable `/edit` commands first.
    EditsFirst,
}

impl CommentSort {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Newest => tr(cx, "Newest first", "最新优先"),
            Self::Oldest => tr(cx, "Oldest first", "最早优先"),
            Self::EditsFirst => tr(cx, "Runnable /edit first", "/edit 指令优先"),
        }
    }
}

/// Reply nested inside a [`PanelComment`] thread, supporting `@mentions` and `/edit` commands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommentReply {
    /// Stable reply identifier.
    pub id: SharedString,
    /// Display name of the reply author.
    pub author: SharedString,
    /// Render an AI pixel avatar instead of person initials when `true`.
    pub is_ai: bool,
    /// Relative timestamp label (such as `"1m ago"`).
    pub created_at: SharedString,
    /// Reply body; starting with `/edit` unlocks a runnable edit action.
    pub text: SharedString,
    /// Whether this reply's `/edit` instruction has been applied.
    pub applied: bool,
}

impl CommentReply {
    /// Construct a [`CommentReply`].
    pub fn new(
        id: impl Into<SharedString>,
        author: impl Into<SharedString>,
        created_at: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            author: author.into(),
            is_ai: false,
            created_at: created_at.into(),
            text: text.into(),
            applied: false,
        }
    }

    /// Mark this reply as authored by an AI assistant.
    pub fn with_ai(mut self, is_ai: bool) -> Self {
        self.is_ai = is_ai;
        self
    }

    /// Set whether a leading `/edit` command starts already applied.
    pub fn with_applied(mut self, applied: bool) -> Self {
        self.applied = applied;
        self
    }
}

/// Top-level collaborative or AI comment item in [`CommentsPanel`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PanelComment {
    /// Stable comment identifier.
    pub id: SharedString,
    /// Author display name.
    pub author: SharedString,
    /// Render an AI pixel avatar when `true`.
    pub is_ai: bool,
    /// Relative timestamp label (such as `"2m ago"`).
    pub created_at: SharedString,
    /// Optional document section or selection anchor badge (such as `"Pricing hero"`).
    pub anchor: Option<SharedString>,
    /// Comment text; starting with `/edit` becomes a runnable edit action.
    pub text: SharedString,
    /// Whether this comment thread has been marked resolved.
    pub resolved: bool,
    /// Whether this comment's `/edit` instruction has been applied.
    pub applied: bool,
    /// Follow-up replies inside this thread.
    pub replies: Vec<CommentReply>,
}

impl PanelComment {
    /// Construct an open [`PanelComment`].
    pub fn new(
        id: impl Into<SharedString>,
        author: impl Into<SharedString>,
        created_at: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            author: author.into(),
            is_ai: false,
            created_at: created_at.into(),
            anchor: None,
            text: text.into(),
            resolved: false,
            applied: false,
            replies: Vec::new(),
        }
    }

    /// Mark this comment as authored by an AI agent.
    pub fn with_ai(mut self, is_ai: bool) -> Self {
        self.is_ai = is_ai;
        self
    }

    /// Attach a document section or element anchor label.
    pub fn with_anchor(mut self, anchor: impl Into<SharedString>) -> Self {
        self.anchor = Some(anchor.into());
        self
    }

    /// Set initial resolved state.
    pub fn with_resolved(mut self, resolved: bool) -> Self {
        self.resolved = resolved;
        self
    }

    /// Set whether a leading `/edit` command starts applied.
    pub fn with_applied(mut self, applied: bool) -> Self {
        self.applied = applied;
        self
    }

    /// Attach replies to this comment thread.
    pub fn with_replies(mut self, replies: Vec<CommentReply>) -> Self {
        self.replies = replies;
        self
    }

    fn has_slash_edit(&self) -> bool {
        extract_slash_edit(&self.text).is_some()
            || self
                .replies
                .iter()
                .any(|r| extract_slash_edit(&r.text).is_some())
    }
}

/// Typed events emitted by [`CommentsPanel`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommentsPanelEvent {
    /// Active filter tab (`Open`, `Resolved`, `All`) changed.
    FilterChanged(CommentFilter),
    /// Sort mode changed.
    SortChanged(CommentSort),
    /// User executed a runnable `/edit` command on a comment or reply.
    RunEdit {
        comment_id: SharedString,
        reply_id: Option<SharedString>,
        instruction: SharedString,
    },
    /// User reverted a previously applied `/edit` command.
    UndoEdit {
        comment_id: SharedString,
        reply_id: Option<SharedString>,
    },
    /// Comment thread was resolved or reopened.
    ResolveChanged { id: SharedString, resolved: bool },
    /// New top-level comment submitted from the composer.
    CommentAdded {
        id: SharedString,
        text: SharedString,
    },
    /// New reply submitted to an existing comment thread.
    ReplyAdded {
        comment_id: SharedString,
        reply_id: SharedString,
        text: SharedString,
    },
    /// Optional close button clicked in the panel header.
    CloseRequested,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RichSpan {
    SlashEdit,
    Mention(String),
    Word(String),
}

/// Extract the actionable instruction if `text` starts with `/edit`.
pub(crate) fn extract_slash_edit(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.len() >= 5 && trimmed[..5].eq_ignore_ascii_case("/edit") {
        let after = &trimmed[5..];
        if after.is_empty() || after.starts_with(char::is_whitespace) {
            let clean = after.trim();
            return Some(if clean.is_empty() {
                "Apply suggested edit".to_owned()
            } else {
                clean.to_owned()
            });
        }
    }
    None
}

/// Tokenize comment prose into `/edit` command badges, `@mention` pills, and plain words.
pub(crate) fn tokenize_comment_body(text: &str) -> Vec<RichSpan> {
    let mut out = Vec::new();
    let trimmed = text.trim();
    let rest = if extract_slash_edit(trimmed).is_some() {
        out.push(RichSpan::SlashEdit);
        trimmed[5..].trim_start()
    } else {
        trimmed
    };

    for token in rest.split_whitespace() {
        if let Some(after_at) = token.strip_prefix('@') {
            let clean_handle: String = after_at
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if !clean_handle.is_empty() {
                let trailing = &after_at[clean_handle.len()..];
                out.push(RichSpan::Mention(format!("@{clean_handle}")));
                if !trailing.is_empty() {
                    out.push(RichSpan::Word(trailing.to_owned()));
                }
                continue;
            }
        }
        out.push(RichSpan::Word(token.to_owned()));
    }
    out
}

fn default_comments() -> Vec<PanelComment> {
    vec![
        PanelComment::new(
            "c1",
            "Mara Lindqvist",
            "2m ago",
            "Should we align the tier terminology for @Jonas before publishing?",
        )
        .with_anchor("Pricing hero")
        .with_replies(vec![CommentReply::new(
            "r1",
            "Jonas Weber",
            "1m ago",
            "/edit Rename every \"customers\" to \"members\" across the pricing cards.",
        )]),
        PanelComment::new(
            "c2",
            "Theo Brandt",
            "9m ago",
            "/edit Shorten every FAQ answer to two crisp sentences for @Lena.",
        )
        .with_anchor("FAQ section"),
        PanelComment::new(
            "c3",
            "Kense AI",
            "14m ago",
            "Spotted 2 badge contrast issues in the comparison table for @Mara.",
        )
        .with_ai(true)
        .with_anchor("Comparison matrix")
        .with_replies(vec![
            CommentReply::new(
                "r2",
                "Lena Hart",
                "11m ago",
                "/edit Upgrade secondary enterprise badge contrast to WCAG AA.",
            )
            .with_applied(true),
        ]),
        PanelComment::new(
            "c4",
            "Sofia Marin",
            "26m ago",
            "The responsive illustrations look crisp on narrow viewports.",
        )
        .with_resolved(true),
    ]
}

/// Collaborative + AI comments sidebar with `Open`/`Resolved`/`All` pills, `/edit` execution, and `@mentions`.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::comments_panel::{CommentsPanel, CommentsPanelEvent};
///
/// fn mount(cx: &mut gpui::App) {
///     let panel = cx.new(|cx| CommentsPanel::new("doc-comments", cx).with_current_user("Lena Hart"));
///     cx.subscribe(&panel, |_, ev: &CommentsPanelEvent, _| {
///         let _ = ev;
///     })
///     .detach();
/// }
/// ```
pub struct CommentsPanel {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    current_user: SharedString,
    comments: Vec<PanelComment>,
    entering_ids: Vec<(SharedString, bool)>,
    folding_out_ids: Vec<(SharedString, Instant)>,
    filter: CommentFilter,
    sort: CommentSort,
    sort_menu_open: bool,
    show_close: bool,
    replying_to: Option<(SharedString, SharedString)>,
    mention_picker_open: bool,
    running_edits: Vec<(SharedString, Option<SharedString>, Instant)>,
    editor: Option<Entity<Editor>>,
    editor_focused: bool,
    next_seq: u32,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    boot_at: Instant,
    toast: Option<(SharedString, Instant)>,
}

impl EventEmitter<CommentsPanelEvent> for CommentsPanel {}

impl CommentsPanel {
    /// Create a [`CommentsPanel`] preloaded with collaborative threads, `@mentions`, and `/edit` commands.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Comments".into(),
            current_user: "Lena Hart".into(),
            comments: default_comments(),
            entering_ids: Vec::new(),
            folding_out_ids: Vec::new(),
            filter: CommentFilter::Open,
            sort: CommentSort::Newest,
            sort_menu_open: false,
            show_close: true,
            replying_to: None,
            mention_picker_open: false,
            running_edits: Vec::new(),
            editor: None,
            editor_focused: false,
            next_seq: 10,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(420.)),
            boot_at: Instant::now(),
            toast: None,
        }
    }

    /// Override the heading displayed at the top of the panel (`"Comments"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the author name used when posting new comments and replies.
    pub fn with_current_user(mut self, user: impl Into<SharedString>) -> Self {
        self.current_user = user.into();
        self
    }

    /// Supply custom [`PanelComment`] threads.
    pub fn with_comments(mut self, comments: Vec<PanelComment>) -> Self {
        self.comments = comments;
        self
    }

    /// Set the initial [`CommentFilter`] tab (`Open`, `Resolved`, or `All`).
    pub fn with_filter(mut self, filter: CommentFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set the initial [`CommentSort`] ordering.
    pub fn with_sort(mut self, sort: CommentSort) -> Self {
        self.sort = sort;
        self
    }

    /// Show or hide the close icon button in the header.
    pub fn with_close_button(mut self, show: bool) -> Self {
        self.show_close = show;
        self
    }

    /// Toggle loading skeleton placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state presentation.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return `(open_count, resolved_count, total_count)` across all threads.
    pub fn counts(&self) -> (usize, usize, usize) {
        let resolved = self.comments.iter().filter(|c| c.resolved).count();
        let total = self.comments.len();
        (total.saturating_sub(resolved), resolved, total)
    }

    /// Switch the active filter tab (`Open`, `Resolved`, `All`).
    pub fn set_filter(&mut self, filter: CommentFilter, cx: &mut Context<Self>) {
        if self.filter != filter {
            self.filter = filter;
            self.folding_out_ids.clear();
            cx.emit(CommentsPanelEvent::FilterChanged(filter));
            cx.notify();
        }
    }

    /// Toggle the resolved state of a comment thread by id.
    pub fn toggle_resolve(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(item) = self.comments.iter_mut().find(|c| c.id.as_ref() == id) {
            item.resolved = !item.resolved;
            let resolved = item.resolved;
            let cid = item.id.clone();
            let leaves_filter = (resolved && self.filter == CommentFilter::Open)
                || (!resolved && self.filter == CommentFilter::Resolved);
            self.folding_out_ids.retain(|(fid, _)| fid != &cid);
            if leaves_filter && !disabled_motion(cx) {
                self.folding_out_ids.push((cid.clone(), Instant::now()));
            }
            self.toast = Some((
                if resolved {
                    tr(cx, "Marked thread resolved", "已标记为已解决").into()
                } else {
                    tr(cx, "Reopened thread", "已重新打开评论").into()
                },
                Instant::now(),
            ));
            cx.emit(CommentsPanelEvent::ResolveChanged { id: cid, resolved });
            cx.notify();
        }
    }

    /// Trigger a runnable `/edit` command on a comment (`reply_id = None`) or reply (`Some(id)`).
    pub fn run_slash_edit(
        &mut self,
        comment_id: SharedString,
        reply_id: Option<SharedString>,
        instruction: SharedString,
        cx: &mut Context<Self>,
    ) {
        if disabled_motion(cx) {
            self.mark_edit_applied(&comment_id, reply_id.as_deref(), true);
        } else if !self
            .running_edits
            .iter()
            .any(|(c, r, _)| c == &comment_id && r == &reply_id)
        {
            self.running_edits
                .push((comment_id.clone(), reply_id.clone(), Instant::now()));
        }
        cx.emit(CommentsPanelEvent::RunEdit {
            comment_id,
            reply_id,
            instruction,
        });
        cx.notify();
    }

    /// Revert an applied `/edit` command back to runnable `Idle` state.
    pub fn undo_slash_edit(
        &mut self,
        comment_id: SharedString,
        reply_id: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.mark_edit_applied(&comment_id, reply_id.as_deref(), false);
        cx.emit(CommentsPanelEvent::UndoEdit {
            comment_id,
            reply_id,
        });
        cx.notify();
    }

    fn mark_edit_applied(&mut self, comment_id: &str, reply_id: Option<&str>, applied: bool) {
        if let Some(c) = self
            .comments
            .iter_mut()
            .find(|c| c.id.as_ref() == comment_id)
        {
            if let Some(rid) = reply_id {
                if let Some(r) = c.replies.iter_mut().find(|r| r.id.as_ref() == rid) {
                    r.applied = applied;
                }
            } else {
                c.applied = applied;
            }
        }
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Add a comment, @mention, or /edit…",
                "添加评论、@提及 或输入 /edit…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    this.sort_menu_open = false;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => this.submit_composer(w, cx),
                EditEvent::Escape => {
                    if this.mention_picker_open {
                        this.mention_picker_open = false;
                    } else if this.replying_to.take().is_some() {
                    } else {
                        this.focus.focus(w, cx);
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.editor = Some(ed);
        }
    }

    fn insert_composer_prefix(&mut self, token: &str, w: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.editor.clone() else {
            return;
        };
        let cur = ed.read(cx).value().to_owned();
        let next = if token == "/edit" {
            if extract_slash_edit(&cur).is_some() {
                cur
            } else if cur.trim().is_empty() {
                "/edit ".to_owned()
            } else {
                format!("/edit {}", cur.trim_start())
            }
        } else if cur.is_empty() || cur.ends_with(' ') {
            format!("{cur}{token} ")
        } else {
            format!("{cur} {token} ")
        };
        ed.update(cx, |e, cx| {
            e.set_value(next, w, cx);
            e.focus(w, cx);
        });
        self.mention_picker_open = false;
        cx.notify();
    }

    fn submit_composer(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let raw = self
            .editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        if raw.is_empty() {
            return;
        }
        if let Some(ed) = &self.editor {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        self.mention_picker_open = false;
        let text: SharedString = raw.into();

        if let Some((target_cid, _)) = self.replying_to.take()
            && let Some(parent) = self.comments.iter_mut().find(|c| c.id == target_cid)
        {
            let rid = SharedString::from(format!("r{}", self.next_seq));
            self.next_seq += 1;
            if !disabled_motion(cx) {
                self.entering_ids
                    .push((format!("rep-{target_cid}-{rid}").into(), false));
            }
            parent.replies.push(CommentReply::new(
                rid.clone(),
                self.current_user.clone(),
                tr(cx, "Just now", "刚刚"),
                text.clone(),
            ));
            if parent.resolved {
                parent.resolved = false;
            }
            cx.emit(CommentsPanelEvent::ReplyAdded {
                comment_id: target_cid,
                reply_id: rid,
                text,
            });
            cx.notify();
            return;
        }

        let cid = SharedString::from(format!("c{}", self.next_seq));
        self.next_seq += 1;
        if !disabled_motion(cx) {
            self.entering_ids.push((cid.clone(), false));
        }
        let new_comment = PanelComment::new(
            cid.clone(),
            self.current_user.clone(),
            tr(cx, "Just now", "刚刚"),
            text.clone(),
        );
        self.comments.insert(0, new_comment);
        if self.filter == CommentFilter::Resolved {
            self.filter = CommentFilter::Open;
            self.folding_out_ids.clear();
        }
        cx.emit(CommentsPanelEvent::CommentAdded { id: cid, text });
        cx.notify();
    }

    fn render_rich_body(&self, text: &str, resolved: bool, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        let spans = tokenize_comment_body(text);
        let mut wrap = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(px(3.5))
            .gap_y(px(2.));

        for sp in spans {
            match sp {
                RichSpan::SlashEdit => {
                    wrap = wrap.child(
                        div()
                            .h(px(18.))
                            .px(px(5.5))
                            .rounded(px(4.))
                            .bg(p.purple.opacity(0.14))
                            .flex()
                            .items_center()
                            .text_size(px(11.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.purple)
                            .child("/edit"),
                    );
                }
                RichSpan::Mention(handle) => {
                    wrap = wrap.child(
                        div()
                            .h(px(18.))
                            .px(px(5.))
                            .rounded(px(4.))
                            .bg(p.accent_soft)
                            .flex()
                            .items_center()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.blue)
                            .child(handle),
                    );
                }
                RichSpan::Word(w) => {
                    wrap = wrap.child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(19.5))
                            .text_color(if resolved { p.muted } else { p.ink })
                            .when(resolved, |d| d.line_through())
                            .child(w),
                    );
                }
            }
        }
        wrap
    }

    fn render_slash_action_row(
        &self,
        comment_id: &SharedString,
        reply_id: Option<&SharedString>,
        instruction: &str,
        applied: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let is_running = self
            .running_edits
            .iter()
            .any(|(c, r, _)| c == comment_id && r.as_ref() == reply_id);
        let key_suffix = match reply_id {
            Some(r) => format!("{comment_id}-{r}"),
            None => comment_id.to_string(),
        };

        let mut row = div().mt(px(7.)).flex().items_center().gap(px(8.));
        if is_running {
            row = row.child(
                div()
                    .h(px(26.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_svg("0 0 16 16", SVG_SPIN_ARC, 13., p.blue))
                    .child(pro_shimmer_text(
                        pro_eid(&self.id, format!("edit-shim-{key_suffix}")),
                        tr(cx, "Applying edit…", "正在应用修改…"),
                        p.muted,
                        p.strong,
                        12.,
                        FontWeight::MEDIUM,
                        true,
                    )),
            );
        } else if applied {
            let cid = comment_id.clone();
            let rid = reply_id.cloned();
            row = row
                .child(
                    div()
                        .h(px(26.))
                        .px(px(9.))
                        .rounded(px(6.))
                        .bg(p.green_soft)
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.green_ink)
                        .child(pro_check_draw(
                            pro_eid(&self.id, format!("edit-chk-{key_suffix}")),
                            true,
                            13.,
                            false,
                            p.green,
                        ))
                        .child(tr(cx, "Edit applied", "已应用修改")),
                )
                .child(
                    pro_tactile(
                        pro_eid(&self.id, format!("undo-edit-{key_suffix}")),
                        tr(cx, "Undo edit", "撤销修改"),
                        5.,
                        transparent_black(),
                        p.hover,
                        transparent_black(),
                        transparent_black(),
                        false,
                        div()
                            .h(px(24.))
                            .px(px(6.))
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .whitespace_nowrap()
                            .child(tr(cx, "Undo", "撤销")),
                        cx,
                        move |this, _, cx| {
                            this.undo_slash_edit(cid.clone(), rid.clone(), cx);
                        },
                    )
                    .flex_none(),
                );
        } else {
            let cid = comment_id.clone();
            let rid = reply_id.cloned();
            let instr: SharedString = instruction.to_owned().into();
            row = row.child(
                pro_tactile(
                    pro_eid(&self.id, format!("run-edit-{key_suffix}")),
                    tr(cx, "Run edit", "执行修改"),
                    6.,
                    p.card,
                    p.hover,
                    p.line_strong,
                    p.line_strong,
                    false,
                    div()
                        .h(px(27.))
                        .pl(px(8.))
                        .pr(px(10.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .whitespace_nowrap()
                        .child(pro_icon(ProIcon::Play, 12., p.blue))
                        .child(tr(cx, "Run edit", "执行 /edit")),
                    cx,
                    move |this, _, cx| {
                        this.run_slash_edit(cid.clone(), rid.clone(), instr.clone(), cx);
                    },
                )
                .flex_none()
                .shadow_xs(),
            );
        }
        row
    }
}

impl Render for CommentsPanel {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editor(w, cx);
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let measure_cell = self.measured_width.clone();
        let compact = width < 380.;
        let elapsed_ms = self.boot_at.elapsed().as_secs_f32() * 1000.0;

        // Settle running `/edit` commands after `680ms`.
        if !self.running_edits.is_empty() {
            let mut finished = Vec::new();
            self.running_edits.retain(|(cid, rid, started)| {
                if reduced || started.elapsed().as_millis() >= 680 {
                    finished.push((cid.clone(), rid.clone()));
                    false
                } else {
                    true
                }
            });
            if !self.running_edits.is_empty() && !reduced {
                w.request_animation_frame();
            }
            for (cid, rid) in finished {
                self.mark_edit_applied(&cid, rid.as_deref(), true);
                self.toast = Some((
                    tr(cx, "Applied /edit change", "已成功执行 /edit 修改").into(),
                    Instant::now(),
                ));
            }
        }

        if self.loading {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .p(px(16.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(420.))
                        .p(px(16.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(140., 18., 5., 0.2, cx))
                        .child(pro_skeleton_bar(260., 28., 6., 0.4, cx))
                        .child(pro_skeleton_bar(360., 68., 8., 0.6, cx))
                        .child(pro_skeleton_bar(360., 54., 8., 0.8, cx)),
                );
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .p(px(16.))
                .child(
                    div()
                        .w_full()
                        .max_w(px(420.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Thread,
                            tr(cx, "No comments yet", "暂无协作评论"),
                            tr(
                                cx,
                                "Highlight any section or type /edit to suggest an AI-assisted revision.",
                                "在下方评论框输入建议、@提及成员，或以 /edit 触发智能修改。",
                            ),
                            Some((
                                pro_eid(&self.id, "restore-comments"),
                                tr(cx, "Load sample feed", "加载示例评论").into(),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                );
        }

        // Header + Filter Tabs (`Open`, `Resolved`, `All` via `pro_pill_indicator`) + Sort dropdown.
        let (open_n, resolved_n, all_n) = self.counts();
        let tab_sensor = pro_selection_sensor();
        let tabs = [
            (CommentFilter::Open, open_n),
            (CommentFilter::Resolved, resolved_n),
            (CommentFilter::All, all_n),
        ];

        let mut tab_strip = div()
            .relative()
            .p(px(3.))
            .rounded(px(8.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "filter-pill"),
                self.filter.key(),
                tab_sensor.clone(),
                6.,
                p.card,
                Some(p.line),
                true,
            ));

        for (flt, cnt) in tabs {
            let selected = self.filter == flt;
            tab_strip = tab_strip.child(
                div()
                    .id(pro_eid(&self.id, format!("tab-{}", flt.key())))
                    .role(Role::Tab)
                    .aria_label(format!("{} ({cnt})", flt.label(cx)))
                    .aria_selected(selected)
                    .tab_index(0)
                    .relative()
                    .h(px(26.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .active(|s| s.top(px(0.75)))
                    .when(!selected, move |d| d.hover(move |s| s.text_color(p.ink)))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if selected { p.strong } else { p.text })
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_filter(flt, cx);
                    }))
                    .when(selected, |d| d.child(pro_mark_selected(&tab_sensor)))
                    .child(flt.label(cx))
                    .child(
                        div()
                            .min_w(px(16.))
                            .h(px(16.))
                            .px(px(4.))
                            .rounded(px(4.))
                            .bg(if selected {
                                p.chip
                            } else {
                                transparent_black()
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_swap_label(
                                pro_eid(&self.id, format!("tab-cnt-{}", flt.key())),
                                format!("{cnt}"),
                                10.5,
                                FontWeight::MEDIUM,
                                if selected { p.ink } else { p.faint },
                            )),
                    ),
            );
        }

        let sort_popover =
            pro_popover_presence(pro_eid(&self.id, "sort-pop"), self.sort_menu_open, {
                let cur_sort = self.sort;
                let base_id = self.id.clone();
                let entity = cx.entity().clone();
                let options = [
                    CommentSort::Newest,
                    CommentSort::Oldest,
                    CommentSort::EditsFirst,
                ]
                .map(|o| (o, o.label(cx)));
                move || {
                    let mut menu = div()
                        .mx(px(14.))
                        .mb(px(6.))
                        .p(px(4.))
                        .rounded(px(8.))
                        .bg(if p.dark { rgb(0x222226) } else { rgb(0xffffff) })
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_md()
                        .flex()
                        .flex_col()
                        .gap(px(2.));
                    for (si, (opt, opt_lbl)) in options.into_iter().enumerate() {
                        let active = opt == cur_sort;
                        let ent = entity.clone();
                        menu = menu.child(
                            div()
                                .id(pro_eid(&base_id, format!("sort-opt-{si}")))
                                .role(Role::ListBoxOption)
                                .aria_label(opt_lbl)
                                .aria_selected(active)
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_between()
                                .text_size(px(12.5))
                                .text_color(p.ink)
                                .hover(move |s| s.bg(p.hover))
                                .on_click(move |_, _, app| {
                                    ent.update(app, |this, cx| {
                                        this.sort = opt;
                                        this.sort_menu_open = false;
                                        cx.emit(CommentsPanelEvent::SortChanged(opt));
                                        cx.notify();
                                    });
                                })
                                .child(opt_lbl)
                                .child(pro_check_draw(
                                    pro_eid(&base_id, format!("sort-chk-{si}")),
                                    active,
                                    13.,
                                    false,
                                    p.blue,
                                )),
                        );
                    }
                    menu.into_any_element()
                }
            });

        if reduced {
            self.entering_ids.clear();
            self.folding_out_ids.clear();
        } else {
            self.folding_out_ids
                .retain(|(_, t)| t.elapsed().as_secs_f32() < 0.28);
            if !self.folding_out_ids.is_empty()
                || self.entering_ids.iter().any(|(_, ready)| !*ready)
            {
                w.request_animation_frame();
            }
        }

        // Filter and sort visible comments (keeping recently resolved/reopened threads during `pro_fold` collapse).
        let mut visible: Vec<PanelComment> = self
            .comments
            .iter()
            .filter(|c| {
                let in_filter = match self.filter {
                    CommentFilter::Open => !c.resolved,
                    CommentFilter::Resolved => c.resolved,
                    CommentFilter::All => true,
                };
                in_filter || self.folding_out_ids.iter().any(|(fid, _)| fid == &c.id)
            })
            .cloned()
            .collect();

        match self.sort {
            CommentSort::Newest => {}
            CommentSort::Oldest => visible.reverse(),
            CommentSort::EditsFirst => {
                visible.sort_by_key(|c| !c.has_slash_edit());
            }
        }

        let mut feed_list = div()
            .id(pro_eid(&self.id, "comments-scroll"))
            .max_h(px(296.))
            .overflow_y_scroll()
            .flex()
            .flex_col();

        if visible.is_empty() {
            feed_list = feed_list.child(
                div()
                    .py(px(28.))
                    .px(px(16.))
                    .text_size(px(12.5))
                    .text_color(p.muted)
                    .text_center()
                    .child(match self.filter {
                        CommentFilter::Open => {
                            tr(cx, "All threads resolved", "所有待处理评论均已解决")
                        }
                        CommentFilter::Resolved => {
                            tr(cx, "No resolved comments yet", "暂无已解决的评论")
                        }
                        CommentFilter::All => tr(cx, "No comments yet", "暂无评论"),
                    }),
            );
        } else {
            for (ci, item) in visible.into_iter().enumerate() {
                let cid = item.id.clone();
                let cid_res = cid.clone();
                let cid_rep = cid.clone();
                let author_rep = item.author.clone();
                let avatar = if item.is_ai {
                    pro_agent_avatar("claude", p.blue, 22., false, elapsed_ms, cx)
                } else {
                    pro_person_avatar(&item.author, 22., false, None, cx)
                };

                let mut thread_card = div()
                    .px(px(if compact { 12. } else { 15. }))
                    .py(px(10.))
                    .when(ci > 0, |d| d.border_t_1().border_color(p.line))
                    .when(item.resolved, |d| d.opacity(0.76))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .min_w_0()
                            .child(avatar)
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.strong)
                                    .child(item.author.clone()),
                            )
                            .when_some(item.anchor.clone(), |d, anc| {
                                d.child(
                                    div()
                                        .h(px(18.))
                                        .px(px(5.5))
                                        .rounded(px(4.))
                                        .bg(p.chip)
                                        .flex()
                                        .items_center()
                                        .text_size(px(10.5))
                                        .text_color(p.text)
                                        .child(anc),
                                )
                            })
                            .child(
                                div()
                                    .ml_auto()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(
                                        pro_tactile(
                                            pro_eid(&self.id, format!("reply-btn-{cid}")),
                                            tr(cx, "Reply", "回复"),
                                            4.,
                                            transparent_black(),
                                            p.hover,
                                            transparent_black(),
                                            transparent_black(),
                                            false,
                                            div()
                                                .h(px(20.))
                                                .px(px(5.))
                                                .flex()
                                                .items_center()
                                                .text_size(px(11.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.muted)
                                                .whitespace_nowrap()
                                                .child(tr(cx, "Reply", "回复")),
                                            cx,
                                            move |this, w, cx| {
                                                this.replying_to =
                                                    Some((cid_rep.clone(), author_rep.clone()));
                                                if let Some(ed) = &this.editor {
                                                    ed.update(cx, |e, cx| e.focus(w, cx));
                                                }
                                                cx.notify();
                                            },
                                        )
                                        .flex_none(),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(p.muted)
                                            .child(item.created_at.clone()),
                                    ),
                            )
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, format!("res-btn-{cid}")),
                                    if item.resolved {
                                        tr(cx, "Reopen comment", "重新打开评论")
                                    } else {
                                        tr(cx, "Resolve comment", "标记评论为已解决")
                                    },
                                    6.,
                                    if item.resolved {
                                        p.green_soft
                                    } else {
                                        transparent_black()
                                    },
                                    p.hover,
                                    transparent_black(),
                                    transparent_black(),
                                    item.resolved,
                                    div()
                                        .size(px(22.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_check_draw(
                                            pro_eid(&self.id, format!("res-chk-{cid}")),
                                            item.resolved,
                                            14.,
                                            false,
                                            p.green,
                                        )),
                                    cx,
                                    move |this, _, cx| {
                                        this.toggle_resolve(&cid_res, cx);
                                    },
                                )
                                .flex_none(),
                            ),
                    )
                    .child(
                        div()
                            .pl(px(30.))
                            .mt(px(3.))
                            .flex()
                            .flex_col()
                            .child(self.render_rich_body(&item.text, item.resolved, cx))
                            .when_some(extract_slash_edit(&item.text), |d, instr| {
                                d.child(self.render_slash_action_row(
                                    &cid,
                                    None,
                                    &instr,
                                    item.applied,
                                    cx,
                                ))
                            }),
                    );

                if !item.replies.is_empty() {
                    let mut replies_col = div()
                        .ml(px(11.))
                        .mt(px(7.))
                        .pl(px(19.))
                        .border_l_1()
                        .border_color(if p.dark {
                            hsla(0., 0., 1., 0.12)
                        } else {
                            hsla(0., 0., 0., 0.10)
                        })
                        .flex()
                        .flex_col()
                        .gap(px(8.));

                    for rep in &item.replies {
                        let rep_av = if rep.is_ai {
                            pro_agent_avatar("claude", p.blue, 18., false, elapsed_ms, cx)
                        } else {
                            pro_person_avatar(&rep.author, 18., false, None, cx)
                        };
                        let rep_key: SharedString = format!("rep-{cid}-{}", rep.id).into();
                        let rep_open = if let Some(entry) =
                            self.entering_ids.iter_mut().find(|(id, _)| *id == rep_key)
                        {
                            if entry.1 {
                                true
                            } else {
                                entry.1 = true;
                                false
                            }
                        } else {
                            true
                        };
                        let rep_node = div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(rep_av)
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(p.strong)
                                            .child(rep.author.clone()),
                                    )
                                    .child(
                                        div()
                                            .ml_auto()
                                            .text_size(px(11.))
                                            .text_color(p.muted)
                                            .child(rep.created_at.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .pl(px(24.))
                                    .mt(px(2.))
                                    .flex()
                                    .flex_col()
                                    .child(self.render_rich_body(&rep.text, item.resolved, cx))
                                    .when_some(extract_slash_edit(&rep.text), |d, instr| {
                                        d.child(self.render_slash_action_row(
                                            &cid,
                                            Some(&rep.id),
                                            &instr,
                                            rep.applied,
                                            cx,
                                        ))
                                    }),
                            )
                            .into_any_element();
                        replies_col = replies_col.child(pro_fold(
                            pro_eid(&self.id, format!("rep-fold-{cid}-{}", rep.id)),
                            rep_open,
                            60.,
                            8.,
                            move || rep_node,
                        ));
                    }
                    thread_card = thread_card.child(replies_col);
                }

                let is_folding_out = self.folding_out_ids.iter().any(|(fid, _)| fid == &cid);
                let cmt_open = if is_folding_out {
                    false
                } else if let Some(entry) = self.entering_ids.iter_mut().find(|(id, _)| *id == cid)
                {
                    if entry.1 {
                        true
                    } else {
                        entry.1 = true;
                        false
                    }
                } else {
                    true
                };
                let thread_el = thread_card.into_any_element();
                feed_list = feed_list.child(pro_fold(
                    pro_eid(&self.id, format!("cmt-fold-{cid}")),
                    cmt_open,
                    96.,
                    0.,
                    move || thread_el,
                ));
            }
            self.entering_ids.retain(|(_, ready)| !*ready);
        }

        // Composer: optional `"Replying to ..."` banner + `@mention` / `/edit` helpers + `Editor` + Send.
        let reply_banner = pro_fold(
            pro_eid(&self.id, "reply-target-fold"),
            self.replying_to.is_some(),
            28.,
            0.,
            {
                let target = self.replying_to.clone();
                let entity = cx.entity().clone();
                let base_id = self.id.clone();
                let prefix = tr(cx, "Replying to", "回复").to_owned();
                move || {
                    let Some((_, name)) = target else {
                        return div().into_any_element();
                    };
                    div()
                        .px(px(14.))
                        .py(px(5.))
                        .bg(p.subtle)
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(format!("{prefix} {name}"))
                        .child(
                            div()
                                .id(pro_eid(&base_id, "cancel-reply"))
                                .role(Role::Button)
                                .aria_label("Cancel reply")
                                .tab_index(0)
                                .size(px(18.))
                                .rounded(px(4.))
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_center()
                                .hover(move |s| s.bg(p.hover))
                                .on_click(move |_, _, app| {
                                    entity.update(app, |this, cx| {
                                        this.replying_to = None;
                                        cx.notify();
                                    });
                                })
                                .child(pro_icon(ProIcon::Close, 11., p.muted)),
                        )
                        .into_any_element()
                }
            },
        );

        let mention_pop = pro_popover_presence(
            pro_eid(&self.id, "mention-pop"),
            self.mention_picker_open,
            {
                let base_id = self.id.clone();
                let entity = cx.entity().clone();
                move || {
                    let handles = ["@Mara", "@Jonas", "@Lena", "@KenseAI"];
                    let mut row = div()
                        .px(px(12.))
                        .py(px(6.))
                        .bg(p.subtle)
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(5.));
                    for (hi, h) in handles.into_iter().enumerate() {
                        let ent = entity.clone();
                        row = row.child(
                            div()
                                .id(pro_eid(&base_id, format!("pick-m-{hi}")))
                                .role(Role::Button)
                                .aria_label(h)
                                .tab_index(0)
                                .h(px(22.))
                                .px(px(7.))
                                .rounded(px(5.))
                                .bg(p.accent_soft)
                                .cursor_pointer()
                                .active(|s| s.top(px(0.75)))
                                .hover(move |s| s.bg(p.blue.opacity(0.20)))
                                .flex()
                                .items_center()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.blue)
                                .on_click(move |_, w, app| {
                                    ent.update(app, |this, cx| {
                                        this.insert_composer_prefix(h, w, cx);
                                    });
                                })
                                .child(h),
                        );
                    }
                    row.into_any_element()
                }
            },
        );

        let can_send = self
            .editor
            .as_ref()
            .is_some_and(|e| !e.read(cx).value().trim().is_empty());
        let ed_click = self.editor.clone();

        let send_btn = if can_send {
            pro_tactile(
                pro_eid(&self.id, "send-comment-btn"),
                tr(cx, "Send comment", "发送评论"),
                6.,
                p.primary,
                if p.dark {
                    hsla(0., 0., 1., 0.88)
                } else {
                    hsla(0., 0., 0., 0.84)
                },
                transparent_black(),
                transparent_black(),
                false,
                div()
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::ArrowUp, 14., p.on_primary)),
                cx,
                |this, w, cx| this.submit_composer(w, cx),
            )
            .flex_none()
            .into_any_element()
        } else {
            div()
                .id(pro_eid(&self.id, "send-comment-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "Send comment", "发送评论"))
                .tab_index(-1)
                .size(px(28.))
                .flex_none()
                .rounded(px(6.))
                .bg(p.chip)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(ProIcon::ArrowUp, 14., p.faint))
                .into_any_element()
        };

        let composer_row = div()
            .px(px(12.))
            .py(px(8.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(6.))
            .child(pro_person_avatar(&self.current_user, 20., false, None, cx))
            .child(
                div()
                    .id(pro_eid(&self.id, "comment-input-hit"))
                    .flex_1()
                    .min_w_0()
                    .min_h(px(26.))
                    .flex()
                    .items_center()
                    .cursor_text()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_, _, w, cx| {
                            if let Some(ed) = &ed_click {
                                ed.read(cx).focus.clone().focus(w, cx);
                            }
                        }),
                    )
                    .when_some(self.editor.clone(), |d, ed| {
                        d.child(div().flex_1().min_w_0().h(px(22.)).child(ed))
                    }),
            )
            .child(
                pro_tactile(
                    pro_eid(&self.id, "quick-at-btn"),
                    tr(cx, "Insert @mention", "插入 @提及"),
                    5.,
                    if self.mention_picker_open {
                        p.accent_soft
                    } else {
                        p.chip
                    },
                    p.chip_hover,
                    transparent_black(),
                    p.line_strong,
                    self.mention_picker_open,
                    div()
                        .h(px(22.))
                        .px(px(5.5))
                        .flex()
                        .items_center()
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if self.mention_picker_open {
                            p.blue
                        } else {
                            p.muted
                        })
                        .whitespace_nowrap()
                        .child("@"),
                    cx,
                    |this, _, cx| {
                        this.mention_picker_open = !this.mention_picker_open;
                        cx.notify();
                    },
                )
                .flex_none(),
            )
            .child(
                pro_tactile(
                    pro_eid(&self.id, "quick-slash-edit"),
                    tr(cx, "Insert /edit command", "插入 /edit 指令"),
                    5.,
                    p.chip,
                    p.chip_hover,
                    transparent_black(),
                    p.line_strong,
                    false,
                    div()
                        .h(px(22.))
                        .px(px(6.))
                        .flex()
                        .items_center()
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.purple)
                        .whitespace_nowrap()
                        .child("/edit"),
                    cx,
                    |this, w, cx| {
                        this.insert_composer_prefix("/edit", w, cx);
                    },
                )
                .flex_none(),
            )
            .child(send_btn);

        let feed_state_key =
            SharedString::from(format!("feed-{}-{:?}", self.filter.key(), self.sort));

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(self.title.clone())
            .track_focus(&self.focus)
            .relative()
            .w_full()
            .min_w_0()
            .p(px(if compact { 12. } else { 16. }))
            .flex()
            .flex_col()
            .items_center()
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
                .w_full()
                .h(px(0.)),
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(396.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .shadow(pro_card_shadow(p.dark))
                    .flex()
                    .flex_col()
                    // Header
                    .child(
                        div()
                            .px(px(15.))
                            .pt(px(11.))
                            .pb(px(2.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(14.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.strong)
                                    .child(self.title.clone()),
                            )
                            .when(self.show_close, |d| {
                                d.child(pro_icon_btn(
                                    pro_eid(&self.id, "close-comments-btn"),
                                    tr(cx, "Close comments", "关闭评论面板"),
                                    ProIcon::Close,
                                    26.,
                                    false,
                                    cx,
                                    |this, _, cx| {
                                        this.toast = Some((
                                            tr(cx, "Close requested", "已触发关闭事件").into(),
                                            Instant::now(),
                                        ));
                                        cx.emit(CommentsPanelEvent::CloseRequested);
                                        cx.notify();
                                    },
                                ))
                            }),
                    )
                    // Filter bar + Sort button
                    .child(
                        div()
                            .px(px(15.))
                            .py(px(8.))
                            .border_b_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(tab_strip)
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, "sort-comments-btn"),
                                    tr(cx, "Sort comments", "评论排序"),
                                    6.,
                                    if self.sort_menu_open {
                                        p.hover
                                    } else {
                                        transparent_black()
                                    },
                                    p.hover,
                                    transparent_black(),
                                    transparent_black(),
                                    self.sort_menu_open,
                                    div()
                                        .size(px(28.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_svg("0 0 16 16", SVG_SORT_LINES, 15., p.muted)),
                                    cx,
                                    |this, _, cx| {
                                        this.sort_menu_open = !this.sort_menu_open;
                                        cx.notify();
                                    },
                                )
                                .flex_none(),
                            ),
                    )
                    .child(sort_popover)
                    .child(pro_view_transition(
                        pro_eid(&self.id, "feed-vtx"),
                        feed_state_key,
                        feed_list,
                    ))
                    .child(reply_banner)
                    .child(mention_pop)
                    .child(composer_row),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{RichSpan, default_comments, extract_slash_edit, tokenize_comment_body};

    #[test]
    fn parses_slash_edit_commands_and_mention_pills() {
        let instr = extract_slash_edit("  /Edit Rename customers to members for @Mara. ");
        assert_eq!(
            instr.as_deref(),
            Some("Rename customers to members for @Mara.")
        );
        assert!(extract_slash_edit("/editorial note").is_none());

        let spans = tokenize_comment_body("/edit Shorten FAQ for @Lena, please.");
        assert_eq!(spans[0], RichSpan::SlashEdit);
        assert!(spans.contains(&RichSpan::Mention("@Lena".to_owned())));
        assert!(spans.contains(&RichSpan::Word(",".to_owned())));
    }

    #[test]
    fn default_threads_cover_open_resolved_and_nested_slash_edits() {
        let comments = default_comments();
        assert_eq!(comments.len(), 4);
        assert_eq!(comments.iter().filter(|c| !c.resolved).count(), 3);
        assert_eq!(comments.iter().filter(|c| c.resolved).count(), 1);
        assert!(comments[0].has_slash_edit());
    }
}
