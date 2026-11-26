//! Git commit feed on a vertical blue trail with agent pixel marks, merge branch curves, and expandable details.
//!
//! Renders human and autonomous agent commits with tinted checkmarks, backtick file/branch chips,
//! colored change-kind tags (`feat`, `fix`, `refactor`, `test`, `release`, etc.), foldable merge
//! side curves via [`PathBuilder`], a live `"Committing"` shimmer + upward spark, and `pro_fold`
//! commit inspection with hash copy and diff line totals.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_agent_avatar, pro_card_shadow, pro_check_draw, pro_ease, pro_eid,
        pro_empty_state, pro_fold, pro_person_avatar, pro_popover_presence, pro_rolling_number,
        pro_shimmer_text, pro_skeleton_bar, pro_swap_label, pro_tactile, tr,
    },
    primitives::{
        family::{PhaseClock, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{
    collections::{HashMap, HashSet},
    f32::consts::TAU,
};
use web_time::Instant;

const ROW_H: f32 = 32.0;
const SUB_ROW_H: f32 = 24.0;
const MAIN_RAIL_X: f32 = 17.0;
const SUB_RAIL_X: f32 = 29.0;
const LOOP_SECS: f32 = 2.40;

const REF_FILE_SVG: &str = r#"<path d="M8 7L16 7M8 11L12 11M13 21.5V18C13 16.3431 14.3431 15 16 15H19.5M20 13.5V9C20 5.68629 18.3137 3 14 3H10C5.68629 3 4 5.68629 4 9V15C4 18.3137 5.68629 21 10 21H13.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const REF_BRANCH_SVG: &str = r#"<path d="M7 19H13C16.5 19 19 17.5 19 13V10M5 7V17" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"></path><circle cx="5" cy="5" r="2.2" stroke="currentColor" stroke-width="1.8"></circle><circle cx="19" cy="5" r="2.2" stroke="currentColor" stroke-width="1.8"></circle><circle cx="5" cy="19" r="2.2" stroke="currentColor" stroke-width="1.8"></circle>"#;

/// Semantic commit change category rendered as a soft tinted pill before the timestamp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitKind {
    Feat,
    Fix,
    Refactor,
    Test,
    Perf,
    Docs,
    Chore,
    Release,
    Merge,
}

impl CommitKind {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Feat => tr(cx, "Feature", "功能"),
            Self::Fix => tr(cx, "Fix", "修复"),
            Self::Refactor => tr(cx, "Refactor", "重构"),
            Self::Test => tr(cx, "Tests", "测试"),
            Self::Perf => tr(cx, "Perf", "性能"),
            Self::Docs => tr(cx, "Docs", "文档"),
            Self::Chore => tr(cx, "Chore", "杂项"),
            Self::Release => tr(cx, "Release", "发布"),
            Self::Merge => tr(cx, "Merge", "合并"),
        }
    }

    fn color(self, dark: bool) -> Hsla {
        match (self, dark) {
            (Self::Feat, false) => rgb(0x12a150).into(),
            (Self::Feat, true) => rgb(0x3ccb7f).into(),
            (Self::Fix, false) => rgb(0xe8730c).into(),
            (Self::Fix, true) => rgb(0xffa04d).into(),
            (Self::Refactor, false) => rgb(0x7c4dff).into(),
            (Self::Refactor, true) => rgb(0xa78bfa).into(),
            (Self::Test, false) => rgb(0xb8860b).into(),
            (Self::Test, true) => rgb(0xe3b341).into(),
            (Self::Perf, false) => rgb(0x0e9f8e).into(),
            (Self::Perf, true) => rgb(0x2dd4bf).into(),
            (Self::Docs, false) => rgb(0x0a84ff).into(),
            (Self::Docs, true) => rgb(0x5aa9ff).into(),
            (Self::Chore, false) => hsla(0., 0., 0., 0.52),
            (Self::Chore, true) => hsla(0., 0., 1., 0.58),
            (Self::Release, false) => rgb(0xe0388f).into(),
            (Self::Release, true) => rgb(0xf472b6).into(),
            (Self::Merge, false) => rgb(0x5b5bd6).into(),
            (Self::Merge, true) => rgb(0x9b9bff).into(),
        }
    }
}

/// Commit author or co-author (person initials avatar or autonomous agent 3x3 pixel mark).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitAuthor {
    /// Display name (`"Mara Lind"`, `"Claude"`, `"Kense Coder"`).
    pub name: SharedString,
    /// When `true`, renders [`pro_agent_avatar`] instead of [`pro_person_avatar`].
    pub agent: bool,
    /// Optional custom hex color (`0xRRGGBB`) for the agent pixel mark.
    pub color_hex: Option<u32>,
}

impl CommitAuthor {
    /// Create a human [`CommitAuthor`].
    pub fn person(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            agent: false,
            color_hex: None,
        }
    }

    /// Create an autonomous agent [`CommitAuthor`] rendered with its deterministic 3x3 pixel mark.
    pub fn agent(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            agent: true,
            color_hex: None,
        }
    }

    /// Override the agent pixel mark RGB hex color (`0x0190fe`, etc.).
    pub fn with_color_hex(mut self, hex: u32) -> Self {
        self.color_hex = Some(hex);
        self
    }
}

/// One commit (or merge commit containing brought-in branch commits) in [`CommitTrail`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrailCommit {
    /// Short or full commit hash (`"a41c7e2b"`).
    pub id: SharedString,
    /// Imperative commit summary; wrap file or branch names in backticks (`"Fix rounding in \`proration.ts\`"`).
    pub message: SharedString,
    /// Optional Chinese translation of `message`.
    pub message_zh: Option<SharedString>,
    /// Primary commit author.
    pub author: CommitAuthor,
    /// Optional co-author (`"with Claude"`).
    pub co_author: Option<CommitAuthor>,
    /// Compact relative timestamp shown in the row (`"26m"`, `"2h"`).
    pub time: SharedString,
    /// Full timestamp shown when the commit detail drawer is expanded.
    pub full_date: Option<SharedString>,
    /// Optional release/version tag pill shown after the message (`"v3.1"`).
    pub tag: Option<SharedString>,
    /// Optional semantic change kind (`Feat`, `Fix`, `Refactor`, `Test`, `Release`, etc.).
    pub kind: Option<CommitKind>,
    /// Number of modified files shown when expanded.
    pub files: Option<u32>,
    /// Lines added (`+N`) shown when expanded.
    pub added: Option<u32>,
    /// Lines removed (`−N`) shown when expanded.
    pub removed: Option<u32>,
    /// Extended multi-line commit body shown when expanded.
    pub body: Option<SharedString>,
    /// Commits brought in by a merge commit (rendered along a side S-curve when unfolded).
    pub commits: Vec<TrailCommit>,
    /// When `true`, the commit is actively being written (`"Committing"` shimmer + spinning arc).
    pub pending: bool,
}

impl TrailCommit {
    /// Create a [`TrailCommit`] with `id`, `message`, and `author`.
    pub fn new(
        id: impl Into<SharedString>,
        message: impl Into<SharedString>,
        author: CommitAuthor,
    ) -> Self {
        Self {
            id: id.into(),
            message: message.into(),
            message_zh: None,
            author,
            co_author: None,
            time: "now".into(),
            full_date: None,
            tag: None,
            kind: None,
            files: None,
            added: None,
            removed: None,
            body: None,
            commits: Vec::new(),
            pending: false,
        }
    }

    /// Set a localized Chinese commit message (backtick chips still parse identically).
    pub fn with_message_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.message_zh = Some(zh.into());
        self
    }

    /// Attach a co-author (`"Tomás Reyes with Claude"`).
    pub fn with_co_author(mut self, co: CommitAuthor) -> Self {
        self.co_author = Some(co);
        self
    }

    /// Set the compact relative timestamp (`"26m"`, `"38m"`).
    pub fn with_time(mut self, time: impl Into<SharedString>) -> Self {
        self.time = time.into();
        self
    }

    /// Set the full date/time string shown inside the expanded commit card.
    pub fn with_full_date(mut self, date: impl Into<SharedString>) -> Self {
        self.full_date = Some(date.into());
        self
    }

    /// Attach a quiet version/release tag chip after the message (`"v3.1"`).
    pub fn with_tag(mut self, tag: impl Into<SharedString>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Set the colored change-kind tag (`Feat`, `Fix`, `Refactor`, `Test`, `Release`, etc.).
    pub fn with_kind(mut self, kind: CommitKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Set file and diff line counts (`files`, `added`, `removed`).
    pub fn with_stats(mut self, files: u32, added: u32, removed: u32) -> Self {
        self.files = Some(files);
        self.added = Some(added);
        self.removed = Some(removed);
        self
    }

    /// Attach an extended commit body paragraph shown when expanded.
    pub fn with_body(mut self, body: impl Into<SharedString>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Attach merged branch commits (makes this commit a foldable merge node).
    pub fn with_merged_commits(mut self, commits: Vec<TrailCommit>) -> Self {
        self.commits = commits;
        self
    }

    /// Mark this commit as actively being written (`pending = true`).
    pub fn with_pending(mut self, pending: bool) -> Self {
        self.pending = pending;
        self
    }

    fn display_message<'a>(&'a self, cx: &App) -> &'a str {
        if let Some(zh) = &self.message_zh {
            tr(cx, self.message.as_ref(), zh.as_ref())
        } else {
            self.message.as_ref()
        }
    }
}

/// Parsed segment of a commit message (plain prose vs backtick-delimited file/branch chip).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MessageToken {
    Text(String),
    RefChip { name: String, is_file: bool },
}

pub(crate) fn parse_message_tokens(msg: &str) -> Vec<MessageToken> {
    let mut out = Vec::new();
    let mut rest = msg;
    while let Some(open) = rest.find('`') {
        let before = &rest[..open];
        if !before.is_empty() {
            out.push(MessageToken::Text(before.to_string()));
        }
        let after_open = &rest[open + 1..];
        if let Some(close) = after_open.find('`') {
            let token = &after_open[..close];
            if !token.is_empty() {
                let is_file = token.contains('.');
                out.push(MessageToken::RefChip {
                    name: token.to_string(),
                    is_file,
                });
            }
            rest = &after_open[close + 1..];
        } else {
            out.push(MessageToken::Text(rest[open..].to_string()));
            return out;
        }
    }
    if !rest.is_empty() {
        out.push(MessageToken::Text(rest.to_string()));
    }
    out
}

/// Typed events emitted by [`CommitTrail`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommitTrailEvent {
    /// Active branch switched via the header menu.
    BranchChanged(SharedString),
    /// Commit detail card opened or closed (`None` when closed).
    OpenCommitChanged(Option<SharedString>),
    /// Merge node expanded or folded (`expanded` is the new state).
    ToggleMerge {
        commit_id: SharedString,
        expanded: bool,
    },
    /// Commit hash copied to the clipboard.
    CopiedHash(SharedString),
}

impl EventEmitter<CommitTrailEvent> for CommitTrail {}

fn sample_merge_wire(x_main: f32, x_sub: f32, total_h: f32) -> Vec<(f32, f32)> {
    // Starts at bottom of merge commit's 16px marker (`y = -8` relative to substep box top),
    // bends out to `x_sub`, runs down past all merged sub-commits, and bends back to `x_main`
    // at `y = total_h + 8` (top of next main commit's 16px marker).
    let y0 = -8.0;
    let y_in = 6.0;
    let y_out_start = (total_h - 6.0).max(y_in + 2.0);
    let y_out_end = total_h + 8.0;
    let mut pts = Vec::with_capacity(46);

    for i in 0..=14 {
        let u = i as f32 / 14.0;
        let inv = 1.0 - u;
        let px_x = inv * inv * inv * x_main
            + 3.0 * inv * inv * u * x_main
            + 3.0 * inv * u * u * x_sub
            + u * u * u * x_sub;
        let py_y = inv * inv * inv * y0
            + 3.0 * inv * inv * u * (y0 + 8.0)
            + 3.0 * inv * u * u * (y_in - 8.0)
            + u * u * u * y_in;
        pts.push((px_x, py_y));
    }
    for i in 1..=14 {
        let u = i as f32 / 14.0;
        pts.push((x_sub, y_in + (y_out_start - y_in) * u));
    }
    for i in 1..=14 {
        let u = i as f32 / 14.0;
        let inv = 1.0 - u;
        let px_x = inv * inv * inv * x_sub
            + 3.0 * inv * inv * u * x_sub
            + 3.0 * inv * u * u * x_main
            + u * u * u * x_main;
        let py_y = inv * inv * inv * y_out_start
            + 3.0 * inv * inv * u * (y_out_start + 8.0)
            + 3.0 * inv * u * u * (y_out_end - 8.0)
            + u * u * u * y_out_end;
        pts.push((px_x, py_y));
    }
    pts
}

fn default_main_commits() -> Vec<TrailCommit> {
    let claude = CommitAuthor::agent("Claude");
    vec![
        TrailCommit::new(
            "f9c81a20",
            "Retry failed webhooks in `queue.ts`",
            claude.clone(),
        )
        .with_message_zh("在 `queue.ts` 中重试失败的 Webhook")
        .with_pending(true),
        TrailCommit::new(
            "a41c7e2b",
            "Bump version in `package.json`",
            CommitAuthor::person("Mara Lind"),
        )
        .with_message_zh("在 `package.json` 中升级版本号")
        .with_kind(CommitKind::Release)
        .with_time("26m")
        .with_full_date("Today, 14:34 UTC")
        .with_stats(2, 14, 4)
        .with_body("Prepare 3.1 release artifacts and update lockfile metadata."),
        TrailCommit::new(
            "9be20f11",
            "Merge `invoice-pdf` into `main`",
            CommitAuthor::person("Tomás Reyes"),
        )
        .with_message_zh("将 `invoice-pdf` 合并至 `main`")
        .with_time("38m")
        .with_full_date("Today, 14:22 UTC")
        .with_merged_commits(vec![
            TrailCommit::new(
                "d7a90c4e",
                "Render line items as a PDF table",
                claude.clone(),
            )
            .with_message_zh("将账单明细渲染为 PDF 表格")
            .with_time("41m")
            .with_full_date("Today, 14:19 UTC")
            .with_stats(4, 286, 22),
            TrailCommit::new(
                "53f1b8a0",
                "Embed fonts in invoice exports",
                CommitAuthor::person("Tomás Reyes"),
            )
            .with_message_zh("在发票导出文件中内嵌字体")
            .with_co_author(claude.clone())
            .with_time("43m")
            .with_full_date("Today, 14:17 UTC")
            .with_stats(2, 94, 8),
            TrailCommit::new(
                "8c4e12d9",
                "Add the PDF route in `routes.ts`",
                claude.clone(),
            )
            .with_message_zh("在 `routes.ts` 中新增 PDF 导出路由")
            .with_time("46m")
            .with_full_date("Today, 14:14 UTC")
            .with_stats(3, 68, 5),
        ]),
        TrailCommit::new(
            "e02c6d97",
            "Fix rounding in `proration.ts`",
            CommitAuthor::person("Kai Brandt"),
        )
        .with_message_zh("修复 `proration.ts` 中的按比例计费舍入误差")
        .with_co_author(claude.clone())
        .with_kind(CommitKind::Fix)
        .with_time("44m")
        .with_full_date("Today, 14:16 UTC")
        .with_stats(2, 18, 9)
        .with_body("Switch mid-cycle seat proration to banker's rounding."),
        TrailCommit::new(
            "3b7d91f4",
            "Merge `tax-rates` into `main`",
            CommitAuthor::person("Yuki Sato"),
        )
        .with_message_zh("将 `tax-rates` 合并至 `main`")
        .with_time("48m")
        .with_full_date("Today, 14:12 UTC")
        .with_merged_commits(vec![
            TrailCommit::new("7a11e02c", "Sync EU VAT rate table", claude.clone())
                .with_message_zh("同步欧盟增值税税率表")
                .with_kind(CommitKind::Feat)
                .with_time("51m")
                .with_stats(2, 112, 19),
            TrailCommit::new(
                "19c4b08e",
                "Add VAT regression tests",
                CommitAuthor::person("Yuki Sato"),
            )
            .with_message_zh("新增增值税计算回归测试")
            .with_kind(CommitKind::Test)
            .with_time("55m")
            .with_stats(1, 64, 0),
        ]),
    ]
}

fn branch_commits_preset(branch: &str) -> Vec<TrailCommit> {
    let claude = CommitAuthor::agent("Claude");
    match branch {
        "invoice-pdf" => vec![
            TrailCommit::new(
                "d7a90c4e",
                "Render line items as a PDF table",
                claude.clone(),
            )
            .with_message_zh("将账单明细渲染为 PDF 表格")
            .with_kind(CommitKind::Feat)
            .with_time("41m")
            .with_stats(4, 286, 22),
            TrailCommit::new(
                "53f1b8a0",
                "Embed fonts in invoice exports",
                CommitAuthor::person("Tomás Reyes"),
            )
            .with_message_zh("在发票导出文件中内嵌字体")
            .with_co_author(claude.clone())
            .with_kind(CommitKind::Perf)
            .with_time("43m")
            .with_stats(2, 94, 8),
            TrailCommit::new("8c4e12d9", "Add the PDF route in `routes.ts`", claude)
                .with_message_zh("在 `routes.ts` 中新增 PDF 导出路由")
                .with_kind(CommitKind::Feat)
                .with_time("46m")
                .with_stats(3, 68, 5),
        ],
        "tax-rates" => vec![
            TrailCommit::new("7a11e02c", "Sync EU VAT rate table", claude)
                .with_message_zh("同步欧盟增值税税率表")
                .with_kind(CommitKind::Feat)
                .with_time("51m")
                .with_stats(2, 112, 19),
            TrailCommit::new(
                "19c4b08e",
                "Add VAT regression tests",
                CommitAuthor::person("Yuki Sato"),
            )
            .with_message_zh("新增增值税计算回归测试")
            .with_kind(CommitKind::Test)
            .with_time("55m")
            .with_stats(1, 64, 0),
        ],
        _ => default_main_commits(),
    }
}

fn default_refs() -> HashMap<SharedString, SharedString> {
    let mut m = HashMap::new();
    m.insert("queue.ts".into(), "src/webhooks/queue.ts".into());
    m.insert(
        "package.json".into(),
        "Workspace root manifest · v3.1.0".into(),
    );
    m.insert(
        "invoice-pdf".into(),
        "3 commits · merged by Tomás Reyes".into(),
    );
    m.insert("main".into(), "Default production branch".into());
    m.insert("routes.ts".into(), "src/api/invoices/routes.ts".into());
    m.insert("proration.ts".into(), "src/billing/proration.ts".into());
    m.insert("tax-rates".into(), "2 commits · merged by Yuki Sato".into());
    m
}

/// Git commit history feed on a vertical blue trail with merge folding and commit detail drawers.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::commit_trail::{CommitTrail, CommitTrailEvent};
///
/// let history = cx.new(|cx| {
///     CommitTrail::new("main-commits", cx)
///         .with_branches(["main", "invoice-pdf", "tax-rates"])
///         .with_has_more(true)
/// });
/// cx.subscribe(&history, |_, ev: &CommitTrailEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct CommitTrail {
    id: ElementId,
    branches: Vec<SharedString>,
    active_branch: SharedString,
    custom_commits: bool,
    commits: Vec<TrailCommit>,
    refs: HashMap<SharedString, SharedString>,
    open_commit_id: Option<SharedString>,
    expanded_merges: HashSet<SharedString>,
    branch_menu_open: bool,
    hovered_row: Option<SharedString>,
    pressed_row: Option<SharedString>,
    hovered_ref: Option<(SharedString, SharedString, bool)>,
    copied_hash: Option<(SharedString, Instant)>,
    copy_clock: PhaseClock,
    has_more: bool,
    max_height: Option<f32>,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl CommitTrail {
    /// Create a [`CommitTrail`] on `"main"` with 9 landed commits, 1 live `"Committing"` head, and `"invoice-pdf"` unfolded.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let mut expanded_merges = HashSet::new();
        expanded_merges.insert("9be20f11".into());
        Self {
            id: id.into(),
            branches: vec!["main".into(), "invoice-pdf".into(), "tax-rates".into()],
            active_branch: "main".into(),
            custom_commits: false,
            commits: default_main_commits(),
            refs: default_refs(),
            open_commit_id: None,
            expanded_merges,
            branch_menu_open: false,
            hovered_row: None,
            pressed_row: None,
            hovered_ref: None,
            copied_hash: None,
            copy_clock: PhaseClock::default(),
            has_more: true,
            max_height: None,
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Supply custom commits (newest first).
    pub fn with_commits(mut self, commits: Vec<TrailCommit>) -> Self {
        self.commits = commits;
        self.custom_commits = true;
        self
    }

    /// Configure selectable branch names in the header menu.
    pub fn with_branches<I, S>(mut self, branches: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        let list: Vec<SharedString> = branches.into_iter().map(Into::into).collect();
        if let Some(first) = list.first() {
            self.active_branch = first.clone();
        }
        self.branches = list;
        self
    }

    /// Set the initially selected branch name.
    pub fn with_branch(mut self, branch: impl Into<SharedString>) -> Self {
        let b: SharedString = branch.into();
        if !self.custom_commits {
            self.commits = branch_commits_preset(b.as_ref());
        }
        self.active_branch = b;
        self
    }

    /// Set which commit id starts expanded with its author, hash, and diff stats open.
    pub fn with_open_commit(mut self, commit_id: Option<SharedString>) -> Self {
        self.open_commit_id = commit_id;
        self
    }

    /// Set which merge commit ids start with their brought-in commits unfolded.
    pub fn with_expanded_merges<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.expanded_merges = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Provide tooltip descriptions for backtick-wrapped file or branch names.
    pub fn with_refs(mut self, refs: HashMap<SharedString, SharedString>) -> Self {
        self.refs = refs;
        self
    }

    /// Show or hide the fading continuation tail below the oldest commit (`true` by default).
    pub fn with_has_more(mut self, has_more: bool) -> Self {
        self.has_more = has_more;
        self
    }

    /// Cap the list viewport height in pixels and scroll past it.
    pub fn with_max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// Render skeleton placeholder rows.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the `"No commits yet"` empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Count all landed (`!pending`) commits, including commits brought in by merges.
    pub fn landed_commit_count(&self) -> usize {
        self.commits
            .iter()
            .filter(|c| !c.pending)
            .map(|c| 1 + c.commits.len())
            .sum()
    }

    /// Select a branch in the header switcher.
    pub fn select_branch(&mut self, branch: &str, cx: &mut Context<Self>) {
        let next: SharedString = branch.to_string().into();
        self.active_branch = next.clone();
        self.branch_menu_open = false;
        self.open_commit_id = None;
        if !self.custom_commits {
            self.commits = branch_commits_preset(branch);
        }
        cx.emit(CommitTrailEvent::BranchChanged(next));
        cx.notify();
    }

    /// Toggle a merge node's brought-in commits or open/close a regular commit's detail card.
    pub fn activate_commit(&mut self, commit_id: &str, is_merge: bool, cx: &mut Context<Self>) {
        let key: SharedString = commit_id.to_string().into();
        if is_merge {
            let expanded = if self.expanded_merges.contains(&key) {
                self.expanded_merges.remove(&key);
                false
            } else {
                self.expanded_merges.insert(key.clone());
                true
            };
            cx.emit(CommitTrailEvent::ToggleMerge {
                commit_id: key,
                expanded,
            });
        } else {
            if self.open_commit_id.as_ref() == Some(&key) {
                self.open_commit_id = None;
            } else {
                self.open_commit_id = Some(key);
            }
            cx.emit(CommitTrailEvent::OpenCommitChanged(
                self.open_commit_id.clone(),
            ));
        }
        cx.notify();
    }
}

fn commit_agent_avatar_seed(name: &str) -> &'static str {
    let s = name.to_ascii_lowercase();
    if s.contains("claude")
        || s.contains("opus")
        || s.contains("sonnet")
        || s.contains("haiku")
        || s.contains("anthropic")
    {
        "claude"
    } else if s.contains("gemini") {
        "gemini"
    } else if s.contains("deepseek") {
        "deepseek"
    } else if s.contains("mistral") || s.contains("codestral") {
        "mistral"
    } else if s.contains("qwen") {
        "qwen"
    } else if s.contains("llama") {
        "llama"
    } else if s.contains("grok") {
        "grok"
    } else if s.contains("gpt")
        || s.contains("openai")
        || s.contains("codex")
        || s.contains("coder")
    {
        "gpt"
    } else {
        "claude"
    }
}

fn render_author_inline(
    author: &CommitAuthor,
    elapsed_ms: f32,
    default_agent_c: Hsla,
    text_c: Hsla,
    cx: &App,
) -> AnyElement {
    let avatar = if author.agent {
        let c = author
            .color_hex
            .map(|h| Hsla::from(rgb(h)))
            .unwrap_or(default_agent_c);
        pro_agent_avatar(
            commit_agent_avatar_seed(author.name.as_ref()),
            c,
            18.,
            false,
            elapsed_ms,
            cx,
        )
    } else {
        pro_person_avatar(author.name.as_ref(), 18., false, None, cx)
    };
    div()
        .flex()
        .items_center()
        .gap(px(5.))
        .child(avatar)
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(text_c)
                .child(author.name.clone()),
        )
        .into_any_element()
}

impl CommitTrail {
    #[allow(clippy::too_many_arguments)]
    fn render_message_tokens(
        &self,
        row_key: &str,
        msg: &str,
        msg_color: Hsla,
        is_sub: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let tokens = parse_message_tokens(msg);
        let fs = if is_sub { 12.0 } else { 13.0 };
        let chip_fs = if is_sub { 10.5 } else { 11.0 };
        let mut line = div()
            .min_w_0()
            .flex_shrink(1.0)
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(px(3.))
            .text_size(px(fs))
            .line_height(px(if is_sub { 16. } else { 20. }))
            .text_color(msg_color);

        for (t_idx, tok) in tokens.into_iter().enumerate() {
            match tok {
                MessageToken::Text(txt) => {
                    let trimmed = txt;
                    line = line.child(div().whitespace_nowrap().child(trimmed));
                }
                MessageToken::RefChip { name, is_file } => {
                    let detail = self.refs.get(name.as_str()).cloned().unwrap_or_else(|| {
                        if is_file {
                            tr(cx, "Source file", "源码文件").into()
                        } else {
                            tr(cx, "Git branch", "Git 分支").into()
                        }
                    });
                    let ref_name: SharedString = name.clone().into();
                    let ref_name_hov = ref_name.clone();
                    let icon_svg = if is_file {
                        REF_FILE_SVG
                    } else {
                        REF_BRANCH_SVG
                    };
                    let chip_border = if p.dark {
                        hsla(0., 0., 1., 0.08)
                    } else {
                        hsla(0., 0., 0., 0.06)
                    };
                    let hover_bg = p.chip_hover;
                    line = line.child(
                        div()
                            .id(pro_eid(&self.id, format!("ref-{row_key}-{t_idx}-{name}")))
                            .h(px(if is_sub { 17. } else { 19. }))
                            .pl(px(4.))
                            .pr(px(5.))
                            .flex_none()
                            .rounded(px(5.))
                            .bg(p.hover)
                            .border_1()
                            .border_color(chip_border)
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .font_family(crate::components::pro_chrome::pro_mono_name())
                            .text_size(px(chip_fs))
                            .text_color(p.text)
                            .hover(move |s| s.bg(hover_bg))
                            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                if *inside {
                                    this.hovered_ref =
                                        Some((ref_name_hov.clone(), detail.clone(), is_file));
                                } else if this
                                    .hovered_ref
                                    .as_ref()
                                    .is_some_and(|(n, _, _)| n == &ref_name_hov)
                                {
                                    this.hovered_ref = None;
                                }
                                cx.notify();
                            }))
                            .child(pro_svg("0 0 24 24", icon_svg, 11., p.faint))
                            .child(ref_name),
                    );
                }
            }
        }
        line.into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_commit_detail(
        &self,
        commit: &TrailCommit,
        nested: bool,
        has_rail_below: bool,
        rail_color: Hsla,
        elapsed_ms: f32,
        copy_drawn: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_open = self.open_commit_id.as_ref() == Some(&commit.id);
        let cid = commit.id.clone();
        let author = commit.author.clone();
        let co_author = commit.co_author.clone();
        let full_date = commit
            .full_date
            .clone()
            .unwrap_or_else(|| commit.time.clone());
        let files = commit.files;
        let added = commit.added;
        let removed = commit.removed;
        let body = commit.body.clone();
        let est_h = if body.is_some() { 82.0 } else { 58.0 };
        let is_copied = self
            .copied_hash
            .as_ref()
            .is_some_and(|(h, t0)| h == &cid && t0.elapsed().as_secs_f32() < 1.8);
        let default_agent_c: Hsla = if p.dark {
            rgba(0x4d8dffd9).into()
        } else {
            rgba(0x0060e2c7).into()
        };
        let green_c: Hsla = if p.dark {
            rgb(0x3ccb7f).into()
        } else {
            rgb(0x12a150).into()
        };
        let red_c: Hsla = if p.dark {
            rgb(0xff6b5f).into()
        } else {
            rgb(0xf04438).into()
        };
        let text_c = p.text;
        let muted_c = p.muted;
        let faint_c = p.faint;
        let hover_c = p.hover;
        let with_word = tr(cx, "with", "协同").to_string();
        let files_suffix = if tr(cx, "en", "zh") == "zh" {
            " 个文件"
        } else if files == Some(1) {
            " file"
        } else {
            " files"
        };
        let author_el = render_author_inline(&author, elapsed_ms, default_agent_c, text_c, cx);
        let co_el = co_author
            .as_ref()
            .map(|co| render_author_inline(co, elapsed_ms, default_agent_c, text_c, cx));
        let ent = cx.entity().clone();
        let copy_hash = cid.clone();
        let short_hash: SharedString = cid.chars().take(8).collect::<String>().into();
        let det_base_id = self.id.clone();

        pro_fold(
            pro_eid(&self.id, format!("cmt-det-{}", commit.id)),
            is_open,
            est_h,
            0.0,
            move || {
                let mut line1 = div()
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(muted_c)
                    .child(author_el);
                if let Some(co) = co_el {
                    line1 = line1
                        .child(div().text_color(faint_c).child(with_word))
                        .child(co);
                }
                line1 = line1
                    .child(div().text_color(faint_c).child("·"))
                    .child(div().truncate().child(full_date));

                let copy_cid = copy_hash.clone();
                let copy_btn = div()
                    .id(SharedString::from(format!("copy-hash-{copy_hash}")))
                    .role(Role::Button)
                    .aria_label("Copy commit hash")
                    .size(px(22.))
                    .rounded(px(5.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_c))
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        cx.write_to_clipboard(ClipboardItem::new_string(copy_hash.to_string()));
                        ent.update(cx, |this, cx| {
                            this.copied_hash = Some((copy_hash.clone(), Instant::now()));
                            this.copy_clock.restart(cx);
                            cx.emit(CommitTrailEvent::CopiedHash(copy_hash.clone()));
                            cx.notify();
                        });
                    })
                    .child(if is_copied {
                        pro_check_draw(
                            pro_eid(&det_base_id, format!("copy-chk-{copy_cid}")),
                            copy_drawn,
                            12.5,
                            false,
                            green_c,
                        )
                    } else {
                        pro_icon(ProIcon::Copy, 12.5, faint_c)
                    });

                let mut line2 = div()
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(muted_c)
                    .child(
                        div()
                            .font_family(crate::components::pro_chrome::pro_mono_name())
                            .text_color(text_c)
                            .child(short_hash),
                    )
                    .child(copy_btn);

                if let Some(fc) = files {
                    line2 = line2.child(div().text_color(faint_c).child("·")).child(
                        pro_rolling_number(
                            pro_eid(&det_base_id, format!("det-files-{copy_cid}")),
                            fc as f64,
                            0,
                            "",
                            files_suffix,
                            12.,
                            FontWeight::NORMAL,
                            muted_c,
                        ),
                    );
                }
                if added.is_some() || removed.is_some() {
                    let a = added.unwrap_or(0);
                    let r = removed.unwrap_or(0);
                    line2 = line2.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .ml(px(2.))
                            .child(pro_rolling_number(
                                pro_eid(&det_base_id, format!("det-add-{copy_cid}")),
                                a as f64,
                                0,
                                "+",
                                "",
                                12.,
                                FontWeight::MEDIUM,
                                green_c,
                            ))
                            .child(pro_rolling_number(
                                pro_eid(&det_base_id, format!("det-rem-{copy_cid}")),
                                r as f64,
                                0,
                                "−",
                                "",
                                12.,
                                FontWeight::MEDIUM,
                                red_c,
                            )),
                    );
                }

                div()
                    .w_full()
                    .relative()
                    .pl(px(if nested { 46. } else { 40. }))
                    .pr(px(10.))
                    .pt(px(2.))
                    .pb(px(9.))
                    .when(has_rail_below && !nested, |d| {
                        d.child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(px(MAIN_RAIL_X - 0.75))
                                .w(px(1.5))
                                .bg(rail_color),
                        )
                    })
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(line1)
                    .child(line2)
                    .when_some(body, |d, b| {
                        d.child(
                            div()
                                .mt(px(3.))
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(text_c)
                                .child(b),
                        )
                    })
                    .into_any_element()
            },
        )
    }
}

impl Render for CommitTrail {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);

        let any_pending = self.commits.iter().any(|c| c.pending);
        let copy_active = self
            .copied_hash
            .as_ref()
            .is_some_and(|(_, t0)| t0.elapsed().as_secs_f32() < 1.85);
        let elapsed_secs = self.started_at.elapsed().as_secs_f32();
        if ((any_pending || self.loading) && !reduced) || copy_active {
            window.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let accent: Hsla = if p.dark {
            rgb(0x4fb0fe).into()
        } else {
            rgb(0x0190fe).into()
        };
        let wash: Hsla = if p.dark {
            accent.opacity(0.18)
        } else {
            accent.opacity(0.12)
        };
        let rail_line: Hsla = if p.dark {
            hsla(0., 0., 1., 0.12)
        } else {
            hsla(0., 0., 0., 0.10)
        };

        let landed_count = if self.empty {
            0
        } else {
            self.landed_commit_count()
        };
        let count_suffix = if tr(cx, "en", "zh") == "zh" {
            " 次提交"
        } else if landed_count == 1 {
            " commit"
        } else {
            " commits"
        };
        let copy_drawn = reduced || self.copy_clock.time(0.26, window, cx) > 0.012;

        let can_switch_branch = self.branches.len() >= 2;
        let menu_open = self.branch_menu_open;
        let branch_btn = if can_switch_branch {
            let br_aria = SharedString::from(format!(
                "{}: {}",
                tr(cx, "Switch branch", "切换分支"),
                self.active_branch
            ));
            let br_inner = div()
                .h(px(24.))
                .px(px(5.))
                .flex()
                .items_center()
                .gap(px(4.))
                .child(pro_swap_label(
                    pro_eid(&self.id, "branch-lbl-swap"),
                    self.active_branch.clone(),
                    13.,
                    FontWeight::MEDIUM,
                    p.strong,
                ))
                .child(pro_icon(
                    if menu_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    12.,
                    p.faint,
                ));
            pro_tactile(
                pro_eid(&self.id, "branch-btn"),
                br_aria,
                6.,
                if menu_open {
                    p.chip
                } else {
                    transparent_black()
                },
                p.hover,
                transparent_black(),
                transparent_black(),
                menu_open,
                br_inner,
                cx,
                |this, _, cx| {
                    this.branch_menu_open = !this.branch_menu_open;
                    cx.notify();
                },
            )
            .ml(px(-4.))
            .flex_none()
            .into_any_element()
        } else {
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .child(self.active_branch.clone())
                .into_any_element()
        };

        // Floating tooltip pill in header when hovering any backtick ref chip!
        let ref_tip: Option<AnyElement> =
            self.hovered_ref.as_ref().map(|(rname, rdet, is_file)| {
                let tip_bg: Hsla = if p.dark {
                    rgb(0x323236).into()
                } else {
                    rgb(0x18181b).into()
                };
                div()
                    .h(px(22.))
                    .px(px(7.))
                    .rounded(px(6.))
                    .bg(tip_bg)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(11.))
                    .text_color(rgb(0xffffff))
                    .child(pro_svg(
                        "0 0 24 24",
                        if *is_file {
                            REF_FILE_SVG
                        } else {
                            REF_BRANCH_SVG
                        },
                        11.,
                        hsla(0., 0., 1., 0.62),
                    ))
                    .child(
                        div()
                            .font_family(crate::components::pro_chrome::pro_mono_name())
                            .font_weight(FontWeight::MEDIUM)
                            .child(rname.clone()),
                    )
                    .child(
                        div()
                            .truncate()
                            .max_w(px(170.))
                            .text_color(hsla(0., 0., 1., 0.65))
                            .child(rdet.clone()),
                    )
                    .into_any_element()
            });

        let header = div()
            .h(px(36.))
            .px(px(8.))
            .relative()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .child(branch_btn),
            )
            .when_some(ref_tip, |d, tip| d.child(tip))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "landed-cnt"),
                        landed_count as f64,
                        0,
                        "",
                        count_suffix,
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            );

        if self.loading {
            let phase = if reduced {
                0.5
            } else {
                (elapsed_secs * 0.9).fract()
            };
            let mut skel = div().pt(px(4.)).pb(px(6.)).flex().flex_col().gap(px(4.));
            for i in 0..4usize {
                skel = skel.child(
                    div()
                        .h(px(ROW_H))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(16., 16., 8., phase + i as f32 * 0.1, cx))
                        .child(pro_skeleton_bar(
                            170. + (i as f32 * 18.),
                            12.,
                            4.,
                            phase + i as f32 * 0.1,
                            cx,
                        ))
                        .child(div().ml_auto().child(pro_skeleton_bar(
                            34.,
                            11.,
                            4.,
                            phase + i as f32 * 0.1,
                            cx,
                        ))),
                );
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(440.))
                        .pt(px(4.))
                        .px(px(8.))
                        .pb(px(10.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .when(!p.dark, |d| d.shadow_sm())
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(skel),
                )
                .into_any_element();
        }

        if self.empty || self.commits.is_empty() {
            let empty_el = pro_empty_state(
                ProIcon::GitBranch,
                tr(cx, "No commits yet", "暂无提交记录"),
                tr(
                    cx,
                    "Commits by teammates and agents appear along this branch trail.",
                    "团队成员与智能体提交的记录将按时间呈现在此分支轨迹上。",
                ),
                None,
                cx,
            );
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(440.))
                        .pt(px(4.))
                        .px(px(8.))
                        .pb(px(10.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .when(!p.dark, |d| d.shadow_sm())
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(empty_el),
                )
                .into_any_element();
        }

        let spin_angle = if reduced {
            -std::f32::consts::FRAC_PI_4
        } else {
            (elapsed_secs / 0.90).fract() * TAU - std::f32::consts::FRAC_PI_2
        };
        let loop_phase = if reduced {
            0.0
        } else {
            (elapsed_secs / LOOP_SECS).fract()
        };
        let elapsed_ms = elapsed_secs * 1000.0;

        let any_detail_open = self.open_commit_id.is_some();
        let n_commits = self.commits.len();
        let mut list_col = div()
            .id(pro_eid(&self.id, "cmt-list"))
            .role(Role::List)
            .aria_label(format!("Commits – {}", self.active_branch))
            .pt(px(2.))
            .when(self.has_more, |d| d.pb(px(14.)))
            .flex()
            .flex_col();

        for idx in 0..n_commits {
            let cmt = self.commits[idx].clone();
            let cid = cmt.id.clone();
            let is_merge = !cmt.commits.is_empty();
            let merge_open = is_merge && self.expanded_merges.contains(&cid);
            let is_open = self.open_commit_id.as_ref() == Some(&cid);
            let has_prev = idx > 0;
            let has_next = idx + 1 < n_commits;
            let prev_merge_open = idx > 0
                && !self.commits[idx - 1].commits.is_empty()
                && self.expanded_merges.contains(&self.commits[idx - 1].id);
            let prev_landed = idx > 0 && !self.commits[idx - 1].pending;
            let cur_landed = !cmt.pending;
            let next_landed = idx + 1 < n_commits && !self.commits[idx + 1].pending;

            let top_seg_c = if prev_landed && cur_landed {
                accent
            } else {
                rail_line
            };
            let bot_seg_c = if cur_landed && next_landed {
                accent
            } else {
                rail_line
            };

            let is_hovered = self.hovered_row.as_ref() == Some(&cid);
            let dim_other = any_detail_open && !is_open && !cmt.pending;

            let marker_el: AnyElement = if cmt.pending {
                // Spinning arc + arrival ring when upward pulse lands!
                let show_spark = has_next && next_landed && !reduced;
                canvas(
                    |_, _, _| (),
                    move |bounds, _, win, _| {
                        let c = bounds.center();
                        let r = 6.25;
                        let steps = 28usize;
                        let mut track = PathBuilder::stroke(px(1.55));
                        for i in 0..=steps {
                            let a = (i as f32 / steps as f32) * TAU;
                            let pt = point(c.x + px(r * a.cos()), c.y + px(r * a.sin()));
                            if i == 0 {
                                track.move_to(pt);
                            } else {
                                track.line_to(pt);
                            }
                        }
                        if let Ok(path) = track.build() {
                            win.paint_path(path, wash);
                        }
                        let sweep = TAU * 0.30;
                        let mut arc = PathBuilder::stroke(px(1.65));
                        for i in 0..=12 {
                            let a = spin_angle + (i as f32 / 12.0) * sweep;
                            let pt = point(c.x + px(r * a.cos()), c.y + px(r * a.sin()));
                            if i == 0 {
                                arc.move_to(pt);
                            } else {
                                arc.line_to(pt);
                            }
                        }
                        if let Ok(path) = arc.build() {
                            win.paint_path(path, accent);
                        }
                        if show_spark && (0.56..=0.92).contains(&loop_phase) {
                            let rt = (loop_phase - 0.56) / 0.36;
                            let rr = 6.25 + 4.5 * pro_ease(rt);
                            let alpha = (1.0 - rt) * 0.55;
                            let mut ring = PathBuilder::stroke(px(1.35));
                            for i in 0..=20 {
                                let a = (i as f32 / 20.0) * TAU;
                                let pt = point(c.x + px(rr * a.cos()), c.y + px(rr * a.sin()));
                                if i == 0 {
                                    ring.move_to(pt);
                                } else {
                                    ring.line_to(pt);
                                }
                            }
                            if let Ok(path) = ring.build() {
                                win.paint_path(path, accent.opacity(alpha));
                            }
                        }
                    },
                )
                .size(px(16.))
                .flex_none()
                .into_any_element()
            } else {
                div()
                    .size(px(16.))
                    .rounded_full()
                    .bg(wash)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("cmt-chk-{cid}")),
                        true,
                        13.5,
                        false,
                        accent,
                    ))
                    .into_any_element()
            };

            let msg_color = if cmt.pending || is_open {
                p.ink
            } else {
                p.text
            };
            let msg_str = cmt.display_message(cx).to_string();
            let msg_el =
                self.render_message_tokens(cid.as_ref(), &msg_str, msg_color, false, &p, cx);

            let tag_el: Option<AnyElement> = cmt.tag.as_ref().map(|tg| {
                div()
                    .h(px(18.))
                    .px(px(6.))
                    .ml(px(6.))
                    .flex_none()
                    .rounded(px(5.))
                    .bg(p.hover)
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text)
                    .flex()
                    .items_center()
                    .child(tg.clone())
                    .into_any_element()
            });

            let merge_chev: Option<AnyElement> = if is_merge {
                Some(
                    div()
                        .ml(px(4.))
                        .flex_none()
                        .opacity(if is_hovered || merge_open { 0.85 } else { 0.0 })
                        .child(pro_icon(
                            if merge_open {
                                ProIcon::ChevronDown
                            } else {
                                ProIcon::ChevronRight
                            },
                            12.,
                            p.faint,
                        ))
                        .into_any_element(),
                )
            } else {
                None
            };

            let kind_el: Option<AnyElement> = cmt.kind.map(|k| {
                let kc = k.color(p.dark);
                div()
                    .h(px(20.))
                    .px(px(7.))
                    .flex_none()
                    .rounded(px(6.))
                    .bg(kc.opacity(0.13))
                    .text_color(kc)
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .child(k.label(cx))
                    .into_any_element()
            });

            let right_time: AnyElement = if cmt.pending {
                pro_shimmer_text(
                    pro_eid(&self.id, format!("cmt-shim-{cid}")),
                    tr(cx, "Committing", "正在提交"),
                    accent,
                    crate::components::pro_chrome::pro_mix_color(accent, card_bg, 0.65),
                    12.,
                    FontWeight::MEDIUM,
                    true,
                )
            } else {
                div()
                    .w(px(32.))
                    .text_right()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.faint)
                    .child(cmt.time.clone())
                    .into_any_element()
            };

            let hov_id = cid.clone();
            let press_id = cid.clone();
            let act_id = cid.clone();
            let key_id = cid.clone();
            let interactive = !cmt.pending;
            let is_pressed = self.pressed_row.as_ref() == Some(&cid);

            let mut row = div()
                .id(pro_eid(&self.id, format!("cmt-row-{cid}")))
                .role(if interactive {
                    Role::Button
                } else {
                    Role::ListItem
                })
                .aria_label(format!("{} – {}", msg_str, cmt.author.name))
                .when(interactive, |d| d.tab_index(0).cursor_pointer())
                .w_full()
                .h(px(ROW_H))
                .px(px(8.))
                .rounded(px(6.))
                .relative()
                .top(px(if is_pressed { 0.5 } else { 0.0 }))
                .flex()
                .items_center()
                .bg(if interactive && is_pressed {
                    p.chip
                } else if (interactive && is_hovered) || is_open {
                    p.hover
                } else {
                    transparent_black()
                })
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    let next = if *inside { Some(hov_id.clone()) } else { None };
                    if !*inside && this.pressed_row.as_ref() == Some(&hov_id) {
                        this.pressed_row = None;
                    }
                    if this.hovered_row != next {
                        this.hovered_row = next;
                        cx.notify();
                    }
                }));

            if interactive {
                row = row
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.pressed_row = Some(press_id.clone());
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            if this.pressed_row.take().is_some() {
                                cx.notify();
                            }
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            if this.pressed_row.take().is_some() {
                                cx.notify();
                            }
                        }),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.activate_commit(act_id.as_ref(), is_merge, cx);
                    }))
                    .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                        match ev.keystroke.key.as_str() {
                            "enter" | "space" | " " => {
                                this.activate_commit(key_id.as_ref(), is_merge, cx);
                                cx.stop_propagation();
                            }
                            "escape" => {
                                if this.open_commit_id.as_ref() == Some(&key_id) {
                                    this.open_commit_id = None;
                                    cx.stop_propagation();
                                    cx.notify();
                                } else if is_merge && this.expanded_merges.contains(&key_id) {
                                    this.expanded_merges.remove(&key_id);
                                    cx.stop_propagation();
                                    cx.notify();
                                }
                            }
                            _ => {}
                        }
                    }));
            }

            // Top rail segment
            if has_prev && !prev_merge_open {
                row = row.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(MAIN_RAIL_X - 0.75))
                        .w(px(1.5))
                        .h(px((ROW_H - 16.0) * 0.5))
                        .bg(top_seg_c),
                );
            }
            // Bottom rail segment (or upward fading spark when this commit is `pending`!)
            if has_next && !merge_open {
                row = row.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left(px(MAIN_RAIL_X - 0.75))
                        .w(px(1.5))
                        .h(px((ROW_H - 16.0) * 0.5))
                        .bg(bot_seg_c),
                );
                if cmt.pending && next_landed && !reduced {
                    // Upward spark running from `y = 16` (top of next commit's check) up to `y = 0` (bottom of spinning marker)
                    row = row.child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, win, _| {
                                let x = f32::from(bounds.left()) + MAIN_RAIL_X;
                                let y_bottom = f32::from(bounds.bottom()) + 8.0;
                                let y_top = f32::from(bounds.bottom()) - 8.0;
                                let segs = 16usize;
                                let u_head =
                                    pro_ease((loop_phase / 0.62).clamp(0., 1.)) * (segs as f32);
                                let tail = 7.0;
                                for i in 0..segs {
                                    let t0 = i as f32 / segs as f32;
                                    let t1 = (i + 1) as f32 / segs as f32;
                                    let pos = i as f32 + 0.5;
                                    let behind = u_head - pos;
                                    if (-0.8..=tail).contains(&behind) {
                                        let alpha = if behind < 0.0 {
                                            (1.0 + behind / 0.8).clamp(0., 1.)
                                        } else {
                                            let f = (1.0 - behind / tail).clamp(0., 1.);
                                            f * f
                                        };
                                        let mut seg = PathBuilder::stroke(px(1.65));
                                        seg.move_to(point(
                                            px(x),
                                            px(y_bottom + (y_top - y_bottom) * t0),
                                        ));
                                        seg.line_to(point(
                                            px(x),
                                            px(y_bottom + (y_top - y_bottom) * t1),
                                        ));
                                        if let Ok(path) = seg.build() {
                                            win.paint_path(path, accent.opacity(alpha));
                                        }
                                    }
                                }
                            },
                        )
                        .absolute()
                        .inset_0(),
                    );
                }
            } else if !has_next && self.has_more && !merge_open && cur_landed {
                // Fading continuation tail below the oldest commit (`hasMore = true`)
                row = row.child(
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, win, _| {
                            let x = f32::from(bounds.left()) + MAIN_RAIL_X;
                            let y0 = f32::from(bounds.bottom()) - 8.0;
                            let y1 = f32::from(bounds.bottom()) + 14.0;
                            let steps = 12usize;
                            for i in 0..steps {
                                let u0 = i as f32 / steps as f32;
                                let u1 = (i + 1) as f32 / steps as f32;
                                let fade = (1.0 - u0).powi(2);
                                let mut seg = PathBuilder::stroke(px(1.5));
                                seg.move_to(point(px(x), px(y0 + (y1 - y0) * u0)));
                                seg.line_to(point(px(x), px(y0 + (y1 - y0) * u1)));
                                if let Ok(path) = seg.build() {
                                    win.paint_path(path, accent.opacity(fade));
                                }
                            }
                        },
                    )
                    .absolute()
                    .inset_0(),
                );
            }

            let content_alpha = if dim_other { 0.42 } else { 1.0 };
            row = row
                .child(
                    div()
                        .w(px(18.))
                        .h(px(16.))
                        .mr(px(14.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(marker_el),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .opacity(content_alpha)
                        .flex()
                        .items_center()
                        .child(msg_el)
                        .when_some(tag_el, |d, tg| d.child(tg))
                        .when_some(merge_chev, |d, ch| d.child(ch))
                        .child(
                            div()
                                .ml_auto()
                                .pl(px(10.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .when_some(kind_el, |d, k| d.child(k))
                                .child(right_time),
                        ),
                );

            let mut item_wrap = div().w_full().relative().flex().flex_col().child(row);

            if !is_merge && !cmt.pending {
                item_wrap = item_wrap.child(self.render_commit_detail(
                    &cmt,
                    false,
                    has_next || self.has_more,
                    if next_landed || self.has_more {
                        accent
                    } else {
                        rail_line
                    },
                    elapsed_ms,
                    copy_drawn,
                    &p,
                    cx,
                ));
            }

            if is_merge {
                // Pre-render brought-in commits + side S-curve wire inside `pro_fold`
                let sub_count = cmt.commits.len();
                let est_sub_h = (sub_count as f32) * SUB_ROW_H + 16.0;
                let mut sub_rows_vec: Vec<AnyElement> = Vec::with_capacity(sub_count);
                for sub in &cmt.commits {
                    let scid = sub.id.clone();
                    let sub_open = self.open_commit_id.as_ref() == Some(&scid);
                    let sub_hov = self.hovered_row.as_ref() == Some(&scid);
                    let sub_dim = any_detail_open && !sub_open;
                    let sub_msg_c = if sub_open { p.ink } else { p.muted };
                    let sub_msg_str = sub.display_message(cx).to_string();
                    let sub_tokens = self.render_message_tokens(
                        scid.as_ref(),
                        &sub_msg_str,
                        sub_msg_c,
                        true,
                        &p,
                        cx,
                    );
                    let sub_kind_el: Option<AnyElement> = sub.kind.map(|k| {
                        let kc = k.color(p.dark);
                        div()
                            .h(px(18.))
                            .px(px(6.))
                            .flex_none()
                            .rounded(px(5.))
                            .bg(kc.opacity(0.13))
                            .text_color(kc)
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(k.label(cx))
                            .into_any_element()
                    });

                    let s_hov_id = scid.clone();
                    let s_act_id = scid.clone();
                    let sub_row = div()
                        .id(pro_eid(&self.id, format!("sub-cmt-{scid}")))
                        .role(Role::Button)
                        .aria_label(format!("{} – {}", sub_msg_str, sub.author.name))
                        .tab_index(0)
                        .w_full()
                        .h(px(SUB_ROW_H))
                        .pl(px(SUB_RAIL_X - 2.5))
                        .pr(px(8.))
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .gap(px(14.))
                        .cursor_pointer()
                        .bg(if sub_hov || sub_open {
                            p.hover
                        } else {
                            transparent_black()
                        })
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            let next = if *inside {
                                Some(s_hov_id.clone())
                            } else {
                                None
                            };
                            if this.hovered_row != next {
                                this.hovered_row = next;
                                cx.notify();
                            }
                        }))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.activate_commit(s_act_id.as_ref(), false, cx);
                        }))
                        .child(div().size(px(5.)).flex_none().rounded_full().bg(accent))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .opacity(if sub_dim { 0.42 } else { 1.0 })
                                .flex()
                                .items_center()
                                .child(sub_tokens)
                                .child(
                                    div()
                                        .ml_auto()
                                        .pl(px(8.))
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .when_some(sub_kind_el, |d, k| d.child(k))
                                        .child(
                                            div()
                                                .w(px(30.))
                                                .text_right()
                                                .text_size(px(11.))
                                                .line_height(px(16.))
                                                .text_color(p.faint)
                                                .child(sub.time.clone()),
                                        ),
                                ),
                        );

                    let sub_detail = self.render_commit_detail(
                        sub, true, false, accent, elapsed_ms, copy_drawn, &p, cx,
                    );
                    sub_rows_vec.push(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .child(sub_row)
                            .child(sub_detail)
                            .into_any_element(),
                    );
                }

                item_wrap = item_wrap.child(pro_fold(
                    pro_eid(&self.id, format!("mrg-fold-{cid}")),
                    merge_open,
                    est_sub_h,
                    0.0,
                    move || {
                        let wire = canvas(
                            |_, _, _| (),
                            move |bounds, _, win, _| {
                                let ox = f32::from(bounds.left());
                                let oy = f32::from(bounds.top());
                                let bh = f32::from(bounds.size.height);
                                let pts = sample_merge_wire(MAIN_RAIL_X, SUB_RAIL_X, bh);
                                if pts.len() >= 2 {
                                    let mut pb = PathBuilder::stroke(px(1.55));
                                    for (k, &(x, y)) in pts.iter().enumerate() {
                                        let pt = point(px(ox + x), px(oy + y));
                                        if k == 0 {
                                            pb.move_to(pt);
                                        } else {
                                            pb.line_to(pt);
                                        }
                                    }
                                    if let Ok(path) = pb.build() {
                                        win.paint_path(path, accent);
                                    }
                                }
                            },
                        )
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .w(px(40.));

                        div()
                            .w_full()
                            .relative()
                            .pt(px(2.))
                            .pb(px(12.))
                            .flex()
                            .flex_col()
                            .child(wire)
                            .children(sub_rows_vec)
                            .into_any_element()
                    },
                ));
            }

            list_col = list_col.child(item_wrap);
        }

        let scroll_wrap = if let Some(max_h) = self.max_height {
            div()
                .id(pro_eid(&self.id, "scroll-vp"))
                .w_full()
                .max_h(px(max_h))
                .overflow_y_scroll()
                .child(list_col)
                .into_any_element()
        } else {
            list_col.into_any_element()
        };

        // Branch switcher popover menu (`pro_popover_presence`)
        let active_br = self.active_branch.clone();
        let branch_rows: Vec<AnyElement> = self
            .branches
            .iter()
            .enumerate()
            .map(|(bi, br)| {
                let selected = br == &active_br;
                let br_name = br.clone();
                let inner = div()
                    .w_full()
                    .h(px(30.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .child(div().flex_1().truncate().child(br.clone()))
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("br-chk-{bi}")),
                        selected,
                        13.,
                        false,
                        accent,
                    ));
                pro_tactile(
                    SharedString::from(format!("br-opt-{bi}-{br_name}")),
                    br.clone(),
                    6.,
                    if selected {
                        p.select
                    } else {
                        transparent_black()
                    },
                    p.hover,
                    transparent_black(),
                    transparent_black(),
                    selected,
                    inner,
                    cx,
                    move |this, _, cx| {
                        this.select_branch(br_name.as_ref(), cx);
                    },
                )
                .w_full()
                .into_any_element()
            })
            .collect();
        let menu_layer = div()
            .absolute()
            .top(px(38.))
            .left(px(12.))
            .child(pro_popover_presence(
                pro_eid(&self.id, "branch-menu"),
                menu_open,
                move || {
                    div()
                        .id("cmt-branch-popup")
                        .role(Role::Menu)
                        .min_w(px(184.))
                        .p(px(4.))
                        .rounded(px(11.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_md()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .children(branch_rows)
                        .into_any_element()
                },
            ));

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(440.))
                    .relative()
                    .pt(px(4.))
                    .px(px(8.))
                    .pb(px(10.))
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow(pro_card_shadow(p.dark))
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(scroll_wrap)
                    .child(menu_layer),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{CommitTrail, MessageToken, parse_message_tokens};
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn backtick_parser_distinguishes_files_and_branches() {
        let tokens = parse_message_tokens("Merge `invoice-pdf` into `main` via `routes.ts`");
        assert_eq!(
            tokens,
            vec![
                MessageToken::Text("Merge ".into()),
                MessageToken::RefChip {
                    name: "invoice-pdf".into(),
                    is_file: false,
                },
                MessageToken::Text(" into ".into()),
                MessageToken::RefChip {
                    name: "main".into(),
                    is_file: false,
                },
                MessageToken::Text(" via ".into()),
                MessageToken::RefChip {
                    name: "routes.ts".into(),
                    is_file: true,
                },
            ]
        );
    }

    #[gpui::test]
    fn commit_trail_counts_landed_commits_folds_merges_and_switches_branches(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let trail = cx.new(|cx| CommitTrail::new("test-commit-trail", cx));

        trail.update(cx, |v, cx| {
            // 9 landed commits on `main` (excluding the top `pending` commit)
            assert_eq!(v.landed_commit_count(), 9);
            assert!(v.expanded_merges.contains("9be20f11"));

            // Fold `9be20f11` closed
            v.activate_commit("9be20f11", true, cx);
            assert!(!v.expanded_merges.contains("9be20f11"));

            // Open commit detail on `e02c6d97`
            v.activate_commit("e02c6d97", false, cx);
            assert_eq!(v.open_commit_id.as_deref(), Some("e02c6d97"));

            // Switch branch to `invoice-pdf`
            v.select_branch("invoice-pdf", cx);
            assert_eq!(v.landed_commit_count(), 3);
            assert_eq!(v.open_commit_id, None);
        });
    }
}
