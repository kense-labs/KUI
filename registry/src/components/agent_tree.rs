//! Hierarchical orchestrator and sub-agent tree with rolled-up branch costs and folding.
//!
//! Displays who started whom across an autonomous run: every node shows live status, elapsed time,
//! own spend, and rolled-up branch cost, with branches that fold cleanly to a single summary line.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_connector_pulse,
        pro_eid, pro_empty_state, pro_fold, pro_modal_appear, pro_rolling_number, pro_skeleton_bar,
        pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        control::activate,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use web_time::Instant;

const SPIN_ARC_SVG: &str = r#"<circle cx="12" cy="12" r="8.5" stroke="currentColor" stroke-opacity="0.25" stroke-width="2.5"/><path d="M12 3.5A8.5 8.5 0 0 1 20.5 12" stroke="currentColor" stroke-linecap="round" stroke-width="2.5"/>"#;

/// Lifecycle state of an agent node in [`AgentTree`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeStatus {
    /// Queued and awaiting execution; card and connector render muted.
    Queued,
    /// Actively executing; elapsed clock ticks live.
    #[default]
    Running,
    /// Waiting on operator confirmation (`"Needs you"`).
    Waiting,
    /// Completed cleanly.
    Done,
    /// Terminated with an error.
    Failed,
    /// Stopped or cancelled; card and connector render muted.
    Stopped,
}

impl TreeStatus {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Queued => tr(cx, "Queued", "排队中"),
            Self::Running => tr(cx, "Running", "运行中"),
            Self::Waiting => tr(cx, "Needs you", "待确认"),
            Self::Done => tr(cx, "Done", "已完成"),
            Self::Failed => tr(cx, "Failed", "失败"),
            Self::Stopped => tr(cx, "Stopped", "已停止"),
        }
    }

    fn is_muted(self) -> bool {
        matches!(self, Self::Queued | Self::Stopped)
    }
}

/// One orchestrator or delegated sub-agent node inside [`AgentTree`].
#[derive(Clone, Debug, PartialEq)]
pub struct TreeAgent {
    /// Stable node identifier used for folding, connectors, and callbacks.
    pub id: SharedString,
    /// Display name shown on the card header.
    pub name: SharedString,
    /// Model label shown beneath `name` (for example `"Opus 5.5"`).
    pub model: SharedString,
    /// Task summary (up to two lines on the card).
    pub task: SharedString,
    /// Current lifecycle status.
    pub status: TreeStatus,
    /// Tool names shown as overlapping `16x16` tiles ahead of `task`.
    pub tools: Vec<SharedString>,
    /// Own spend in USD (branch nodes display the sum of `cost` across the entire subtree).
    pub cost: f64,
    /// Base elapsed duration in seconds.
    pub elapsed_secs: u64,
    /// Approval question when `status == TreeStatus::Waiting`.
    pub ask: Option<SharedString>,
    /// Failure reason when `status == TreeStatus::Failed`.
    pub error: Option<SharedString>,
    /// Delegated child sub-agents started by this node.
    pub children: Vec<TreeAgent>,
}

impl TreeAgent {
    /// Construct a [`TreeAgent`] with `id`, `name`, `task`, and `status`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        task: impl Into<SharedString>,
        status: TreeStatus,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: SharedString::default(),
            task: task.into(),
            status,
            tools: Vec::new(),
            cost: 0.08,
            elapsed_secs: 45,
            ask: None,
            error: None,
            children: Vec::new(),
        }
    }

    /// Set the model label displayed beneath `name`.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = model.into();
        self
    }

    /// Attach overlapping tool tiles displayed ahead of `task`.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.tools = tools.into_iter().map(Into::into).collect();
        self
    }

    /// Set this node's own USD spend and base elapsed seconds.
    pub fn with_usage(mut self, cost: f64, elapsed_secs: u64) -> Self {
        self.cost = if cost.is_finite() && cost >= 0.0 {
            cost
        } else {
            0.0
        };
        self.elapsed_secs = elapsed_secs;
        self
    }

    /// Set the approval prompt shown when `status == TreeStatus::Waiting`.
    pub fn with_ask(mut self, ask: impl Into<SharedString>) -> Self {
        self.ask = Some(ask.into());
        self
    }

    /// Set the error detail shown when `status == TreeStatus::Failed`.
    pub fn with_error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Attach child sub-agents spawned by this node.
    pub fn with_children(mut self, children: Vec<TreeAgent>) -> Self {
        self.children = children;
        self
    }
}

/// Resolution action for waiting or failed nodes in [`AgentTree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeResolveAction {
    /// Approve a waiting node so it continues execution.
    Approve,
    /// Reject a waiting node and stop its branch.
    Reject,
    /// Spawn a clean retry run for a failed node.
    Retry,
    /// Dismiss a failed node error.
    Dismiss,
}

/// Typed events emitted by [`AgentTree`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentTreeEvent {
    /// User clicked an agent card to inspect its run details.
    Selected {
        /// Selected node identifier.
        agent_id: SharedString,
    },
    /// User folded or unfolded a branch.
    CollapsedChanged {
        /// Sorted list of currently collapsed branch ids.
        collapsed: Vec<SharedString>,
    },
    /// Operator resolved a waiting or failed node.
    Resolved {
        /// Target node identifier.
        agent_id: SharedString,
        /// Chosen resolution action.
        action: TreeResolveAction,
    },
}

impl EventEmitter<AgentTreeEvent> for AgentTree {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeaderFilterPopover {
    Waiting,
    Failed,
}

/// Hierarchical orchestrator and sub-agent tree with rolled-up branch costs and branch folding.
///
/// # Examples
///
/// ```ignore
/// use kui::components::agent_tree::AgentTree;
///
/// let tree = cx.new(|cx| AgentTree::new("agent-tree", cx));
/// ```
pub struct AgentTree {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    root: Option<TreeAgent>,
    custom_data: bool,
    collapsed: HashSet<SharedString>,
    selected_id: Option<SharedString>,
    header_pop: Option<HeaderFilterPopover>,
    spawn_counter: usize,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentTree {
    /// Create a live [`AgentTree`] initialized with a 7-node pricing launch orchestration run.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: None,
            root: Some(default_tree_root()),
            custom_data: false,
            collapsed: HashSet::new(),
            selected_id: None,
            header_pop: None,
            spawn_counter: 0,
            toast: None,
            loading: false,
            empty: false,
            boot: Instant::now(),
            measured_width: Rc::new(Cell::new(780.0)),
        }
    }

    /// Override the header title (defaults to `"Agents"` / `"智能体层级树"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replace the hierarchy with a custom root [`TreeAgent`].
    pub fn with_root(mut self, root: TreeAgent) -> Self {
        self.root = Some(root);
        self.custom_data = true;
        self
    }

    /// Configure which branch ids start folded.
    pub fn with_default_collapsed(
        mut self,
        ids: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.collapsed = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Render skeleton placeholders when `loading` is `true`.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state when `empty` is `true`.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the total rolled-up USD cost of the entire tree.
    pub fn total_cost(&self) -> f64 {
        self.root.as_ref().map(rollup_cost).unwrap_or(0.0)
    }

    /// Check whether `branch_id` is currently folded.
    pub fn is_branch_collapsed(&self, branch_id: &str) -> bool {
        self.collapsed.contains(branch_id)
    }

    /// Fold or unfold `branch_id` and emit [`AgentTreeEvent::CollapsedChanged`].
    pub fn toggle_branch(&mut self, branch_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let bid = branch_id.into();
        if !self.collapsed.remove(&bid) {
            self.collapsed.insert(bid);
        }
        let mut sorted: Vec<SharedString> = self.collapsed.iter().cloned().collect();
        sorted.sort();
        cx.emit(AgentTreeEvent::CollapsedChanged { collapsed: sorted });
        cx.notify();
    }

    /// Resolve a waiting or failed node and roll up the updated cost across its ancestors.
    pub fn resolve_agent(
        &mut self,
        agent_id: impl Into<SharedString>,
        action: TreeResolveAction,
        cx: &mut Context<Self>,
    ) {
        let target = agent_id.into();
        let mut name = target.clone();
        if let Some(root) = &mut self.root
            && let Some(node) = find_tree_mut(root, &target)
        {
            name = node.name.clone();
            match action {
                TreeResolveAction::Approve => {
                    node.status = TreeStatus::Running;
                    node.ask = None;
                    node.cost += 0.06;
                }
                TreeResolveAction::Reject => {
                    node.status = TreeStatus::Stopped;
                    node.ask = None;
                }
                TreeResolveAction::Retry => {
                    node.status = TreeStatus::Running;
                    node.error = None;
                    node.cost += 0.05;
                }
                TreeResolveAction::Dismiss => {
                    node.status = TreeStatus::Stopped;
                    node.error = None;
                }
            }
        }
        self.header_pop = None;
        self.selected_id = None;
        let msg = match action {
            TreeResolveAction::Approve => format!("{} {name}", tr(cx, "Approved", "已批准")),
            TreeResolveAction::Reject => format!("{} {name}", tr(cx, "Rejected", "已拒绝")),
            TreeResolveAction::Retry => format!("{} {name}", tr(cx, "Retrying", "正在重试")),
            TreeResolveAction::Dismiss => format!("{} {name}", tr(cx, "Dismissed", "已忽略")),
        };
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(AgentTreeEvent::Resolved {
            agent_id: target,
            action,
        });
        cx.notify();
    }

    /// Simulate an incremental sub-agent step spend so rolled-up branch numbers animate upward.
    pub fn add_step_spend(&mut self, cx: &mut Context<Self>) {
        self.spawn_counter = self.spawn_counter.wrapping_add(1);
        let Some(root) = &mut self.root else {
            return;
        };
        if self.spawn_counter % 2 == 1 {
            if let Some(sc) = find_tree_mut(root, &"scraper".into()) {
                sc.cost += 0.07;
            } else {
                root.cost += 0.07;
            }
        } else if let Some(cd) = find_tree_mut(root, &"coder".into()) {
            let new_id = format!("sub-test-{}", self.spawn_counter);
            if cd.children.len() < 3 {
                cd.children.push(
                    TreeAgent::new(
                        new_id,
                        "Test verifier",
                        "Verify responsive checkout layout",
                        TreeStatus::Running,
                    )
                    .with_model("Haiku 4.5")
                    .with_tools(["GitHub"])
                    .with_usage(0.09, 12),
                );
            } else {
                cd.cost += 0.08;
            }
        }
        cx.notify();
    }

    /// Toggle skeleton loading placeholders at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether skeleton loading is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.empty || self.root.is_none()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_node_card(
        &self,
        node: &TreeAgent,
        full_width: bool,
        sensor: Rc<Cell<Option<Bounds<Pixels>>>>,
        live_secs: u64,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let nid = node.id.clone();
        let has_children = !node.children.is_empty();
        let is_folded = self.collapsed.contains(&nid);
        let desc_count = count_tree_all(node).saturating_sub(1);
        let branch_total = rollup_cost(node);
        let own_spend = node.cost;
        let elapsed = if node.status == TreeStatus::Running {
            node.elapsed_secs + live_secs
        } else {
            node.elapsed_secs
        };

        let disp_name = if self.custom_data {
            node.name.clone()
        } else {
            tr_tree_name(&node.id, &node.name, cx).into()
        };
        let disp_task = if self.custom_data {
            node.task.clone()
        } else {
            tr_tree_task(&node.task, cx).into()
        };

        let provider_key = infer_provider_key(&node.model);
        let avatar_el = if !node.model.is_empty()
            && let Some(art) = pro_app_artwork(provider_key, 13., cx)
        {
            div()
                .size(px(22.))
                .flex_none()
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .justify_center()
                .child(art)
                .into_any_element()
        } else {
            pro_agent_avatar(
                &node.id,
                if node.status == TreeStatus::Waiting {
                    p.amber
                } else if node.status == TreeStatus::Failed {
                    p.red
                } else {
                    p.blue
                },
                22.,
                node.status == TreeStatus::Running,
                (live_secs as f32) * 1000.0,
                cx,
            )
        };

        let status_pill = render_tree_status(node.status, p, cx);

        let mut task_row = div()
            .w_full()
            .min_h(px(32.))
            .flex()
            .items_start()
            .gap(px(6.))
            .min_w_0();
        if !node.tools.is_empty() {
            let shown = node.tools.len().min(3);
            let mut strip = div().flex_none().mt(px(1.)).flex().items_center();
            for (ti, tool) in node.tools.iter().take(shown).enumerate() {
                strip = strip.child(
                    div()
                        .when(ti > 0, |d| d.ml(px(-4.)))
                        .child(pro_app_tile(tool, 16., cx)),
                );
            }
            task_row = task_row.child(strip);
        }
        task_row = task_row.child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(p.text)
                .child(disp_task),
        );

        let fold_btn = if has_children {
            let fid = nid.clone();
            let hov_bg = p.chip_hover;
            let focus_c = p.focus;
            let fold_aria = if is_folded {
                format!("{} {desc_count} {}", tr(cx, "Show", "展开"), disp_name)
            } else {
                format!(
                    "{} {}",
                    tr(cx, "Fold sub-agents of", "折叠子智能体："),
                    disp_name
                )
            };
            Some(
                div()
                    .id(pro_eid(&self.id, format!("fold-btn-{nid}")))
                    .role(Role::Button)
                    .aria_label(fold_aria)
                    .tab_index(0)
                    .h(px(20.))
                    .pl(px(6.))
                    .pr(px(4.))
                    .rounded(px(5.))
                    .bg(if is_folded {
                        p.chip
                    } else {
                        transparent_black()
                    })
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .cursor_pointer()
                    .hover(move |s| s.bg(hov_bg))
                    .focus_visible(move |s| s.border_1().border_color(focus_c))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.toggle_branch(fid.clone(), cx);
                    }))
                    .child(format!("{desc_count}"))
                    .child(pro_icon(
                        if is_folded {
                            ProIcon::ChevronRight
                        } else {
                            ProIcon::ChevronDown
                        },
                        12.,
                        p.muted,
                    )),
            )
        } else {
            None
        };

        let hover_border = p.line_strong;
        let focus_c = p.focus;
        let click_id = nid.clone();

        let card = activate(
            div()
                .id(pro_eid(&self.id, format!("tnode-{nid}")))
                .role(Role::TreeItem)
                .aria_label(format!("{disp_name}: {}", node.status.label(cx)))
                .tab_index(0)
                .when(full_width, |d| d.w_full())
                .when(!full_width, |d| d.w(px(220.)))
                .relative()
                .opacity(if node.status.is_muted() { 0.60 } else { 1.0 })
                .rounded(px(8.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(10.))
                .pb(px(8.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .cursor_pointer()
                .hover(move |s| s.border_color(hover_border))
                .focus_visible(move |s| s.border_color(focus_c))
                .child(
                    canvas(
                        move |bounds, _, _| {
                            sensor.set(Some(bounds));
                        },
                        |_, _, _, _| (),
                    )
                    .absolute()
                    .inset_0(),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .min_w_0()
                        .child(avatar_el)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(16.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(disp_name),
                                )
                                .when(!node.model.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .text_size(px(11.))
                                            .line_height(px(14.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(node.model.clone()),
                                    )
                                }),
                        )
                        .child(status_pill),
                )
                .child(task_row)
                .child(
                    div()
                        .w_full()
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(4.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(3.))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, format!("tcost-{nid}")),
                                    branch_total,
                                    2,
                                    "$",
                                    "",
                                    11.,
                                    FontWeight::MEDIUM,
                                    p.ink,
                                ))
                                .when(has_children, |d| {
                                    d.child(
                                        div()
                                            .text_size(px(10.5))
                                            .text_color(p.faint)
                                            .child(format!("(${own_spend:.2})")),
                                    )
                                })
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(format!("· {}", format_elapsed(elapsed))),
                                ),
                        )
                        .children(fold_btn),
                ),
            cx,
            move |this, _, cx| {
                let target = click_id.clone();
                this.header_pop = None;
                this.selected_id = Some(target.clone());
                cx.emit(AgentTreeEvent::Selected { agent_id: target });
                cx.notify();
            },
        );

        if has_children && is_folded {
            let sub_cost: f64 = node.children.iter().map(rollup_cost).sum();
            let sub_run = count_descendants_by(node, |n| n.status == TreeStatus::Running);
            let sub_wait = count_descendants_by(node, |n| n.status == TreeStatus::Waiting);
            let unfold_id = nid.clone();
            let hov_chip = p.chip_hover;
            let summary_line = activate(
                div()
                    .id(pro_eid(&self.id, format!("folded-line-{nid}")))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Unfold branch", "展开已折叠分支"))
                    .when(full_width, |d| d.w_full())
                    .when(!full_width, |d| d.w(px(220.)))
                    .h(px(26.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .text_size(px(11.))
                    .text_color(p.muted)
                    .cursor_pointer()
                    .hover(move |s| s.bg(hov_chip))
                    .child(div().truncate().child(if sub_wait > 0 {
                        format!(
                            "{desc_count} {} · {sub_wait} {}",
                            tr(cx, "folded", "个已折叠"),
                            tr(cx, "needs you", "待确认")
                        )
                    } else if sub_run > 0 {
                        format!(
                            "{desc_count} {} · {sub_run} {}",
                            tr(cx, "folded", "个已折叠"),
                            tr(cx, "running", "运行中")
                        )
                    } else {
                        format!(
                            "{desc_count} {}",
                            tr(cx, "sub-agents folded", "个子智能体已折叠")
                        )
                    }))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(format!("${sub_cost:.2}")),
                    ),
                cx,
                move |this, _, cx| {
                    this.toggle_branch(unfold_id.clone(), cx);
                },
            );
            div()
                .when(full_width, |d| d.w_full())
                .when(!full_width, |d| d.w(px(220.)))
                .flex()
                .flex_col()
                .gap(px(5.))
                .child(card)
                .child(summary_line)
                .into_any_element()
        } else {
            card.into_any_element()
        }
    }

    fn render_node_modal(
        &self,
        node: &TreeAgent,
        live_secs: u64,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let nid = node.id.clone();
        let disp_name = if self.custom_data {
            node.name.clone()
        } else {
            tr_tree_name(&node.id, &node.name, cx).into()
        };
        let branch_total = rollup_cost(node);
        let desc_count = count_tree_all(node).saturating_sub(1);
        let elapsed = if node.status == TreeStatus::Running {
            node.elapsed_secs + live_secs
        } else {
            node.elapsed_secs
        };

        let hover_bg = p.chip_hover;
        let close_btn = activate(
            div()
                .id(pro_eid(&self.id, "close-tmodal"))
                .role(Role::Button)
                .aria_label(tr(cx, "Close details", "关闭详情"))
                .size(px(24.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg))
                .child(pro_icon(ProIcon::Close, 13., p.muted)),
            cx,
            |this, w, cx| {
                this.selected_id = None;
                this.focus.focus(w, cx);
                cx.notify();
            },
        );

        let mut dialog = div()
            .id(pro_eid(&self.id, "tnode-dialog"))
            .role(Role::Dialog)
            .aria_label(disp_name.clone())
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(330.))
            .max_w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .min_w_0()
                            .child(pro_agent_avatar(&node.id, p.blue, 24., false, 0., cx))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(13.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(disp_name),
                                    )
                                    .when(!node.model.is_empty(), |d| {
                                        d.child(
                                            div()
                                                .text_size(px(11.5))
                                                .text_color(p.muted)
                                                .child(node.model.clone()),
                                        )
                                    }),
                            ),
                    )
                    .child(close_btn),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .line_height(px(18.))
                    .text_color(p.text)
                    .child(node.task.clone()),
            )
            .child(
                div()
                    .w_full()
                    .py(px(8.))
                    .border_t_1()
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(stat_col(
                        tr(cx, "Own cost", "自身花费"),
                        format!("${:.2}", node.cost),
                        p,
                    ))
                    .child(stat_col(
                        tr(cx, "Branch total", "分支总计"),
                        format!("${branch_total:.2}"),
                        p,
                    ))
                    .child(stat_col(
                        tr(cx, "Elapsed", "耗时"),
                        format_elapsed(elapsed),
                        p,
                    )),
            );

        if desc_count > 0 {
            let fold_target = nid.clone();
            let folded = self.collapsed.contains(&nid);
            dialog = dialog.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(format!(
                        "{desc_count} {}",
                        tr(cx, "sub-agents in branch", "个分支子智能体")
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "modal-fold-toggle"),
                        if folded {
                            tr(cx, "Unfold branch", "展开分支")
                        } else {
                            tr(cx, "Fold branch", "折叠分支")
                        },
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.toggle_branch(fold_target.clone(), cx);
                        },
                    )),
            );
        }

        if node.status == TreeStatus::Waiting
            && let Some(ask) = &node.ask
        {
            let a_rej = nid.clone();
            let a_app = nid.clone();
            dialog = dialog
                .child(
                    div()
                        .p(px(8.))
                        .rounded(px(7.))
                        .bg(p.amber_soft)
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .text_color(p.amber_ink)
                        .child(ask.clone()),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_end()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(&self.id, "tmodal-rej"),
                            tr(cx, "Reject", "拒绝"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_agent(a_rej.clone(), TreeResolveAction::Reject, cx);
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, "tmodal-app"),
                            tr(cx, "Approve", "批准"),
                            Some(ProIcon::Check),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_agent(a_app.clone(), TreeResolveAction::Approve, cx);
                            },
                        )),
                );
        }

        if node.status == TreeStatus::Failed
            && let Some(err) = &node.error
        {
            let a_dis = nid.clone();
            let a_ret = nid.clone();
            dialog = dialog
                .child(
                    div()
                        .p(px(8.))
                        .rounded(px(7.))
                        .bg(p.red_soft)
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .text_color(p.red_ink)
                        .child(err.clone()),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_end()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(&self.id, "tmodal-dis"),
                            tr(cx, "Dismiss", "忽略"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_agent(a_dis.clone(), TreeResolveAction::Dismiss, cx);
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, "tmodal-ret"),
                            tr(cx, "Retry", "重试"),
                            Some(ProIcon::Refresh),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_agent(a_ret.clone(), TreeResolveAction::Retry, cx);
                            },
                        )),
                );
        }

        div()
            .id(pro_eid(&self.id, "tmodal-scrim"))
            .absolute()
            .inset_0()
            .on_click(cx.listener(|this, _, w, cx| {
                this.selected_id = None;
                this.focus.focus(w, cx);
                cx.notify();
            }))
            .child(pro_modal_appear(
                pro_eid(&self.id, "tmodal-app-wrap"),
                nid,
                if p.dark { 0.48 } else { 0.22 },
                dialog,
            ))
            .into_any_element()
    }
}

impl Render for AgentTree {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_secs = self.boot.elapsed().as_secs();

        let width_cell = self.measured_width.clone();
        let width_sensor = canvas(
            move |bounds, _, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 20.0 {
                    width_cell.set(bw);
                }
            },
            |_, _, _, _| (),
        )
        .absolute()
        .left_0()
        .right_0()
        .top_0()
        .h(px(1.));

        if self.loading {
            let phase = (self.boot.elapsed().as_secs_f32() / 1.2).fract();
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(width_sensor)
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_between()
                        .child(pro_skeleton_bar(140., 18., 5., phase, cx))
                        .child(pro_skeleton_bar(190., 16., 5., phase, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .child(pro_skeleton_bar(220., 108., 8., phase, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(220., 108., 8., phase, cx))
                        .child(pro_skeleton_bar(220., 108., 8., phase, cx))
                        .child(pro_skeleton_bar(220., 108., 8., phase, cx)),
                )
                .into_any_element();
        }

        let Some(root_node) = self.root.clone() else {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(width_sensor)
                .child(pro_empty_state(
                    ProIcon::GitBranch,
                    tr(cx, "No agent run active", "暂无运行中的智能体树"),
                    tr(
                        cx,
                        "Start an orchestrator run to trace delegated sub-agents and rolled-up branch costs.",
                        "启动主控编排任务后，即可按分支追踪子智能体耗时与汇总花费。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        };

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(width_sensor)
                .child(pro_empty_state(
                    ProIcon::GitBranch,
                    tr(cx, "No agent run active", "暂无运行中的智能体树"),
                    tr(
                        cx,
                        "Start an orchestrator run to trace delegated sub-agents and rolled-up branch costs.",
                        "启动主控编排任务后，即可按分支追踪子智能体耗时与汇总花费。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let total_agents = count_tree_all(&root_node);
        let running_count = count_tree_by(&root_node, |n| n.status == TreeStatus::Running);
        let waiting_nodes = collect_tree_by(&root_node, |n| n.status == TreeStatus::Waiting);
        let failed_nodes = collect_tree_by(&root_node, |n| n.status == TreeStatus::Failed);
        let total_cost = rollup_cost(&root_node);

        if running_count > 0 && !reduced {
            w.request_animation_frame();
        }

        let container_w = self.measured_width.get();
        let wide = container_w >= 680.0 && root_node.children.len() <= 3;

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Agents", "智能体层级树").into());

        let wait_btn = if !waiting_nodes.is_empty() {
            let active = self.header_pop == Some(HeaderFilterPopover::Waiting);
            Some(activate(
                div()
                    .id(pro_eid(&self.id, "hdr-wait-btn"))
                    .role(Role::Button)
                    .h(px(22.))
                    .px(px(7.))
                    .rounded(px(5.))
                    .bg(p.amber_soft)
                    .text_color(p.amber_ink)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .cursor_pointer()
                    .child(format!(
                        "{} {}",
                        waiting_nodes.len(),
                        tr(cx, "needs you", "待确认")
                    ))
                    .child(pro_icon(
                        if active {
                            ProIcon::ChevronUp
                        } else {
                            ProIcon::ChevronDown
                        },
                        11.,
                        p.amber_ink,
                    )),
                cx,
                |this, _, cx| {
                    this.header_pop = if this.header_pop == Some(HeaderFilterPopover::Waiting) {
                        None
                    } else {
                        Some(HeaderFilterPopover::Waiting)
                    };
                    cx.notify();
                },
            ))
        } else {
            None
        };

        let fail_btn = if !failed_nodes.is_empty() {
            let active = self.header_pop == Some(HeaderFilterPopover::Failed);
            Some(activate(
                div()
                    .id(pro_eid(&self.id, "hdr-fail-btn"))
                    .role(Role::Button)
                    .h(px(22.))
                    .px(px(7.))
                    .rounded(px(5.))
                    .bg(p.red_soft)
                    .text_color(p.red_ink)
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .cursor_pointer()
                    .child(format!(
                        "{} {}",
                        failed_nodes.len(),
                        tr(cx, "failed", "已失败")
                    ))
                    .child(pro_icon(
                        if active {
                            ProIcon::ChevronUp
                        } else {
                            ProIcon::ChevronDown
                        },
                        11.,
                        p.red_ink,
                    )),
                cx,
                |this, _, cx| {
                    this.header_pop = if this.header_pop == Some(HeaderFilterPopover::Failed) {
                        None
                    } else {
                        Some(HeaderFilterPopover::Failed)
                    };
                    cx.notify();
                },
            ))
        } else {
            None
        };

        let header = div()
            .w_full()
            .mb(px(16.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(pro_icon(ProIcon::Sparkles, 14., p.blue))
                    .child(
                        div()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(title_str.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(pro_btn(
                        pro_eid(&self.id, "step-spend-btn"),
                        tr(cx, "+ Step spend", "+ 模拟计费"),
                        Some(ProIcon::Coins),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.add_step_spend(cx);
                        },
                    ))
                    .child(format!("{total_agents} {}", tr(cx, "agents", "个智能体")))
                    .when(running_count > 0, |d| {
                        d.child(
                            div()
                                .text_color(p.blue)
                                .child(format!("{running_count} {}", tr(cx, "running", "运行中"))),
                        )
                    })
                    .children(wait_btn)
                    .children(fail_btn)
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "tree-total-cost"),
                        total_cost,
                        2,
                        "$",
                        "",
                        12.,
                        FontWeight::MEDIUM,
                        p.ink,
                    )),
            );

        // Build card layout and connector sensors
        let mut sensors: HashMap<SharedString, Rc<Cell<Option<Bounds<Pixels>>>>> = HashMap::new();
        let mut edges: Vec<(SharedString, SharedString, bool, bool)> = Vec::new();

        let root_sensor = Rc::new(Cell::new(None));
        sensors.insert(root_node.id.clone(), root_sensor.clone());
        let root_folded = self.collapsed.contains(&root_node.id);

        let root_card = self.render_node_card(&root_node, !wide, root_sensor, elapsed_secs, &p, cx);

        let mut l1_cols: Vec<AnyElement> = Vec::new();
        if !root_folded {
            for l1 in &root_node.children {
                let s1 = Rc::new(Cell::new(None));
                sensors.insert(l1.id.clone(), s1.clone());
                edges.push((
                    root_node.id.clone(),
                    l1.id.clone(),
                    wide,
                    l1.status.is_muted(),
                ));
                let l1_card = self.render_node_card(l1, !wide, s1, elapsed_secs, &p, cx);
                let l1_folded = self.collapsed.contains(&l1.id);

                let mut l2_elements: Vec<AnyElement> = Vec::new();
                if !l1_folded {
                    for l2 in &l1.children {
                        let s2 = Rc::new(Cell::new(None));
                        sensors.insert(l2.id.clone(), s2.clone());
                        edges.push((l1.id.clone(), l2.id.clone(), false, l2.status.is_muted()));
                        l2_elements.push(self.render_node_card(
                            l2,
                            !wide,
                            s2,
                            elapsed_secs,
                            &p,
                            cx,
                        ));
                    }
                }

                let l2_fold = pro_fold(
                    pro_eid(&self.id, format!("l1-sub-fold-{}", l1.id)),
                    !l1_folded && !l1.children.is_empty(),
                    220.,
                    0.,
                    move || {
                        div()
                            .pl(px(30.))
                            .pt(px(12.))
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .children(l2_elements)
                            .into_any_element()
                    },
                );

                let col_w = if l1.children.is_empty() || l1_folded {
                    220.
                } else {
                    250.
                };
                l1_cols.push(
                    div()
                        .when(wide, |d| d.w(px(col_w)).flex_none())
                        .when(!wide, |d| d.w_full())
                        .flex()
                        .flex_col()
                        .child(l1_card)
                        .child(l2_fold)
                        .into_any_element(),
                );
            }
        }

        let root_children_fold = pro_fold(
            pro_eid(&self.id, "root-children-fold"),
            !root_folded && !root_node.children.is_empty(),
            360.,
            0.,
            move || {
                div()
                    .w_full()
                    .when(wide, |d| {
                        d.mt(px(38.))
                            .flex()
                            .items_start()
                            .justify_center()
                            .gap(px(10.))
                    })
                    .when(!wide, |d| {
                        d.mt(px(12.)).pl(px(28.)).flex().flex_col().gap(px(12.))
                    })
                    .children(l1_cols)
                    .into_any_element()
            },
        );

        let link_blue: Hsla = if p.dark {
            p.blue.opacity(0.76)
        } else {
            rgb(0x83c7fe).into()
        };
        let link_muted = p.line_strong;
        let pulse_blue = p.blue;
        let card_bg = p.card;
        let pulse_t = if running_count > 0 && !reduced {
            Some((self.boot.elapsed().as_secs_f32() / 2.4).fract())
        } else {
            None
        };

        let links_canvas = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                let mut dots: Vec<Point<Pixels>> = Vec::new();
                for (from_id, to_id, s_bend, muted) in &edges {
                    let (Some(b0), Some(b1)) = (
                        sensors.get(from_id).and_then(|c| c.get()),
                        sensors.get(to_id).and_then(|c| c.get()),
                    ) else {
                        continue;
                    };
                    let col = if *muted { link_muted } else { link_blue };
                    if *s_bend {
                        let p0 = point((b0.left() + b0.right()) * 0.5, b0.bottom());
                        let p1 = point((b1.left() + b1.right()) * 0.5, b1.top());
                        pro_connector_pulse(
                            p0,
                            p1,
                            8.0,
                            true,
                            col,
                            pulse_blue,
                            if *muted { None } else { pulse_t },
                            1.5,
                            window,
                        );
                        if !dots
                            .iter()
                            .any(|d| (d.x - p0.x).abs() < px(1.) && (d.y - p0.y).abs() < px(1.))
                        {
                            dots.push(p0);
                        }
                    } else {
                        let x0 = f32::from(b0.left()) + 16.0;
                        let y0 = f32::from(b0.bottom());
                        let x1 = f32::from(b1.left());
                        let y1 = (f32::from(b1.top()) + f32::from(b1.bottom())) * 0.5;
                        paint_tree_elbow(x0, y0, x1, y1, 5.5, col, 1.5, window);
                        let anchor = point(px(x0), px(y0));
                        if !dots.iter().any(|d| {
                            (d.x - anchor.x).abs() < px(1.) && (d.y - anchor.y).abs() < px(1.)
                        }) {
                            dots.push(anchor);
                        }
                    }
                }
                for pt in dots {
                    paint_ring_dot(pt, 3.25, card_bg, link_blue, 1.5, window);
                }
            },
        )
        .absolute()
        .inset_0();

        // Optional top-right header resolution popover
        let header_popover = self.header_pop.map(|kind| {
            let items = match kind {
                HeaderFilterPopover::Waiting => &waiting_nodes,
                HeaderFilterPopover::Failed => &failed_nodes,
            };
            let mut pop = div()
                .id(pro_eid(&self.id, "hdr-pop"))
                .role(Role::Dialog)
                .on_click(|_, _, cx| cx.stop_propagation())
                .absolute()
                .top(px(42.))
                .right(px(12.))
                .w(px(308.))
                .max_w(px((container_w - 24.).max(220.)))
                .p(px(8.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(8.));

            for item in items {
                let iid = item.id.clone();
                let detail = item
                    .ask
                    .clone()
                    .or_else(|| item.error.clone())
                    .unwrap_or_else(|| item.task.clone());
                let actions = if kind == HeaderFilterPopover::Waiting {
                    let r1 = iid.clone();
                    let r2 = iid.clone();
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(&self.id, format!("hpop-rej-{iid}")),
                            tr(cx, "Reject", "拒绝"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_agent(r1.clone(), TreeResolveAction::Reject, cx);
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, format!("hpop-app-{iid}")),
                            tr(cx, "Approve", "批准"),
                            Some(ProIcon::Check),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_agent(r2.clone(), TreeResolveAction::Approve, cx);
                            },
                        ))
                } else {
                    let r1 = iid.clone();
                    let r2 = iid.clone();
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(&self.id, format!("hpop-dis-{iid}")),
                            tr(cx, "Dismiss", "忽略"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_agent(r1.clone(), TreeResolveAction::Dismiss, cx);
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(&self.id, format!("hpop-ret-{iid}")),
                            tr(cx, "Retry", "重试"),
                            Some(ProIcon::Refresh),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_agent(r2.clone(), TreeResolveAction::Retry, cx);
                            },
                        ))
                };

                pop = pop.child(
                    div()
                        .p(px(6.))
                        .rounded(px(7.))
                        .bg(p.subtle)
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(item.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(17.))
                                .text_color(p.text)
                                .child(detail),
                        )
                        .child(actions),
                );
            }
            pop
        });

        let modal_el = self.selected_id.clone().and_then(|sid| {
            let found = find_tree(&root_node, &sid)?;
            Some(self.render_node_modal(found, elapsed_secs, &p, cx))
        });

        let sr_announce =
            format!("{total_agents} agents, {running_count} running, ${total_cost:.2}");

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(title_str.clone())
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "escape" {
                    this.header_pop = None;
                    this.selected_id = None;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
            }))
            .on_click(cx.listener(|this, _, _, cx| {
                if this.header_pop.is_some() {
                    this.header_pop = None;
                    cx.notify();
                }
            }))
            .w_full()
            .p(px(12.))
            .relative()
            .flex()
            .flex_col()
            .child(width_sensor)
            .child(header)
            .child(
                div()
                    .id(pro_eid(&self.id, "tree-stage"))
                    .role(Role::Tree)
                    .aria_label(title_str)
                    .w_full()
                    .relative()
                    .flex()
                    .flex_col()
                    .when(wide, |d| d.items_center())
                    .child(links_canvas)
                    .child(
                        div()
                            .when(
                                wide && !root_folded && !root_node.children.is_empty(),
                                |d| d.mr(px(28.)),
                            )
                            .when(!wide, |d| d.w_full())
                            .child(root_card),
                    )
                    .child(root_children_fold),
            )
            .children(header_popover)
            .children(modal_el)
            .child(pro_toast_banner(self.toast.clone()))
            .child(
                div()
                    .id(pro_eid(&self.id, "sr-tree"))
                    .role(Role::Status)
                    .aria_label(sr_announce)
                    .absolute()
                    .size_0(),
            )
            .into_any_element()
    }
}

fn render_tree_status(status: TreeStatus, p: &ProPalette, cx: &App) -> Div {
    let (bg, fg) = match status {
        TreeStatus::Running => (p.accent_soft, p.blue),
        TreeStatus::Waiting => (p.amber_soft, p.amber_ink),
        TreeStatus::Done => (p.green_soft, p.green_ink),
        TreeStatus::Failed => (p.red_soft, p.red_ink),
        TreeStatus::Queued | TreeStatus::Stopped => (p.chip, p.muted),
    };
    div()
        .h(px(18.))
        .px(px(6.))
        .flex_none()
        .rounded(px(4.))
        .bg(bg)
        .text_color(fg)
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .flex()
        .items_center()
        .gap(px(4.))
        .when(status == TreeStatus::Running, |d| {
            d.child(pro_svg("0 0 24 24", SPIN_ARC_SVG, 11.5, fg))
        })
        .when(status == TreeStatus::Done, |d| {
            d.child(pro_icon(ProIcon::Check, 11., fg))
        })
        .when(status == TreeStatus::Waiting, |d| {
            d.child(pro_icon(ProIcon::Clock, 11., fg))
        })
        .when(status == TreeStatus::Failed, |d| {
            d.child(pro_icon(ProIcon::AlertTriangle, 11., fg))
        })
        .child(status.label(cx))
}

fn stat_col(label: &str, value: String, p: &ProPalette) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(11.))
                .text_color(p.muted)
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .truncate()
                .child(value),
        )
}

#[allow(clippy::too_many_arguments)]
fn paint_tree_elbow(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    radius: f32,
    color: Hsla,
    stroke_w: f32,
    window: &mut Window,
) {
    let dy = y1 - y0;
    let dx = x1 - x0;
    let r = radius.min(dy.abs() * 0.5).min(dx.abs() * 0.5).max(0.0);
    let mut pb = PathBuilder::stroke(px(stroke_w));
    pb.move_to(point(px(x0), px(y0)));
    if r < 1.0 {
        pb.line_to(point(px(x0), px(y1)));
        pb.line_to(point(px(x1), px(y1)));
    } else {
        let sy = dy.signum();
        let sx = dx.signum();
        let b = (x0, y1 - sy * r);
        let c = (x0, y1);
        let d = (x0 + sx * r, y1);
        pb.line_to(point(px(b.0), px(b.1)));
        for k in 1..=6 {
            let u = k as f32 / 6.0;
            let inv = 1.0 - u;
            pb.line_to(point(
                px(inv * inv * b.0 + 2.0 * inv * u * c.0 + u * u * d.0),
                px(inv * inv * b.1 + 2.0 * inv * u * c.1 + u * u * d.1),
            ));
        }
        pb.line_to(point(px(x1), px(y1)));
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

fn paint_ring_dot(
    center: Point<Pixels>,
    r: f32,
    fill_color: Hsla,
    stroke_color: Hsla,
    stroke_w: f32,
    window: &mut Window,
) {
    let outer = r + stroke_w * 0.5;
    window.paint_quad(
        fill(
            Bounds::new(
                point(center.x - px(outer), center.y - px(outer)),
                size(px(outer * 2.0), px(outer * 2.0)),
            ),
            stroke_color,
        )
        .corner_radii(px(outer)),
    );
    let inner = (outer - stroke_w).max(1.0);
    window.paint_quad(
        fill(
            Bounds::new(
                point(center.x - px(inner), center.y - px(inner)),
                size(px(inner * 2.0), px(inner * 2.0)),
            ),
            fill_color,
        )
        .corner_radii(px(inner)),
    );
}

fn rollup_cost(node: &TreeAgent) -> f64 {
    let own = if node.cost.is_finite() && node.cost >= 0.0 {
        node.cost
    } else {
        0.0
    };
    own + node.children.iter().map(rollup_cost).sum::<f64>()
}

fn count_tree_all(node: &TreeAgent) -> usize {
    1 + node.children.iter().map(count_tree_all).sum::<usize>()
}

fn count_tree_by(node: &TreeAgent, pred: impl Fn(&TreeAgent) -> bool + Copy) -> usize {
    usize::from(pred(node))
        + node
            .children
            .iter()
            .map(|c| count_tree_by(c, pred))
            .sum::<usize>()
}

fn count_descendants_by(node: &TreeAgent, pred: impl Fn(&TreeAgent) -> bool + Copy) -> usize {
    node.children.iter().map(|c| count_tree_by(c, pred)).sum()
}

fn collect_tree_by(node: &TreeAgent, pred: impl Fn(&TreeAgent) -> bool + Copy) -> Vec<TreeAgent> {
    let mut out = Vec::new();
    if pred(node) {
        out.push(node.clone());
    }
    for ch in &node.children {
        out.extend(collect_tree_by(ch, pred));
    }
    out
}

fn find_tree<'a>(node: &'a TreeAgent, id: &SharedString) -> Option<&'a TreeAgent> {
    if &node.id == id {
        return Some(node);
    }
    for ch in &node.children {
        if let Some(found) = find_tree(ch, id) {
            return Some(found);
        }
    }
    None
}

fn find_tree_mut<'a>(node: &'a mut TreeAgent, id: &SharedString) -> Option<&'a mut TreeAgent> {
    if &node.id == id {
        return Some(node);
    }
    for ch in &mut node.children {
        if let Some(found) = find_tree_mut(ch, id) {
            return Some(found);
        }
    }
    None
}

fn infer_provider_key(model: &str) -> &'static str {
    let lower = model.to_ascii_lowercase();
    if lower.contains("opus")
        || lower.contains("sonnet")
        || lower.contains("haiku")
        || lower.contains("claude")
    {
        "claude"
    } else if lower.contains("gemini") {
        "gemini"
    } else {
        "openai"
    }
}

fn format_elapsed(secs: u64) -> String {
    if secs >= 3_600 {
        format!("{}h {:02}m", secs / 3_600, (secs % 3_600) / 60)
    } else if secs >= 60 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    }
}

fn tr_tree_name<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    match id {
        "orchestrator" => tr(cx, "Orchestrator", "主控编排器"),
        "researcher" => tr(cx, "Researcher", "竞品调研员"),
        "scraper" => tr(cx, "Scraper", "网页抓取器"),
        "analyst" => tr(cx, "Analyst", "数据分析师"),
        "writer" => tr(cx, "Writer", "文案撰写员"),
        "coder" => tr(cx, "Coder", "前端编码员"),
        "checkout-sub" => tr(cx, "Stripe verifier", "支付校验器"),
        _ => fallback,
    }
}

fn tr_tree_task<'a>(en: &'a str, cx: &App) -> &'a str {
    match en {
        "Launch the new pricing page by Friday" => tr(
            cx,
            "Launch the new pricing page by Friday",
            "周五前完成新版定价页上线",
        ),
        "Compare the pricing of five competitors" => tr(
            cx,
            "Compare the pricing of five competitors",
            "对比五家竞品的定价档位",
        ),
        "Scrape public pricing tables & annual tiers" => tr(
            cx,
            "Scrape public pricing tables & annual tiers",
            "抓取公开定价表与年付折扣",
        ),
        "Synthesize seat-tier elasticity table" => tr(
            cx,
            "Synthesize seat-tier elasticity table",
            "整理席位阶梯价格对比矩阵",
        ),
        "Write the copy for the three plans" => tr(
            cx,
            "Write the copy for the three plans",
            "撰写三档套餐标题与权益文案",
        ),
        "Build the page and wire up checkout" => tr(
            cx,
            "Build the page and wire up checkout",
            "开发定价页面并接入结账流程",
        ),
        "Verify Stripe webhook & annual coupon IDs" => tr(
            cx,
            "Verify Stripe webhook & annual coupon IDs",
            "校验 Stripe 回调与年付优惠码",
        ),
        _ => en,
    }
}

fn default_tree_root() -> TreeAgent {
    TreeAgent::new(
        "orchestrator",
        "Orchestrator",
        "Launch the new pricing page by Friday",
        TreeStatus::Running,
    )
    .with_model("Opus 5.5")
    .with_tools(["Slack", "Notion", "GitHub"])
    .with_usage(0.14, 105)
    .with_children(vec![
        TreeAgent::new(
            "researcher",
            "Researcher",
            "Compare the pricing of five competitors",
            TreeStatus::Running,
        )
        .with_model("Sonnet 5.5")
        .with_tools(["Notion", "HubSpot"])
        .with_usage(0.12, 92)
        .with_children(vec![
            TreeAgent::new(
                "scraper",
                "Scraper",
                "Scrape public pricing tables & annual tiers",
                TreeStatus::Done,
            )
            .with_model("Haiku 4.5")
            .with_tools(["GitHub"])
            .with_usage(0.09, 48),
            TreeAgent::new(
                "analyst",
                "Analyst",
                "Synthesize seat-tier elasticity table",
                TreeStatus::Running,
            )
            .with_model("Gemini 3.8 Pro")
            .with_tools(["Notion"])
            .with_usage(0.11, 64),
        ]),
        TreeAgent::new(
            "writer",
            "Writer",
            "Write the copy for the three plans",
            TreeStatus::Waiting,
        )
        .with_model("Opus 5.5")
        .with_tools(["Notion", "Figma"])
        .with_usage(0.16, 78)
        .with_ask(
            "Approve enterprise SLA guarantee line (\"99.99% uptime backed by service credits\") before publishing?",
        ),
        TreeAgent::new(
            "coder",
            "Coder",
            "Build the page and wire up checkout",
            TreeStatus::Running,
        )
        .with_model("GPT-6 Sol")
        .with_tools(["GitHub", "Figma", "Stripe"])
        .with_usage(0.19, 86)
        .with_children(vec![
            TreeAgent::new(
                "checkout-sub",
                "Stripe verifier",
                "Verify Stripe webhook & annual coupon IDs",
                TreeStatus::Running,
            )
            .with_model("Sonnet 5.5")
            .with_tools(["Stripe"])
            .with_usage(0.08, 39),
        ]),
    ])
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{
        AgentTree, TreeResolveAction, TreeStatus, count_tree_all, default_tree_root, rollup_cost,
    };
    use gpui::TestAppContext;

    #[test]
    fn rollup_cost_accumulates_own_and_descendant_spend_accurately() {
        let root = default_tree_root();
        assert_eq!(count_tree_all(&root), 7);
        // 0.14 + (0.12 + 0.09 + 0.11) + 0.16 + (0.19 + 0.08) = 0.89
        assert!((rollup_cost(&root) - 0.89).abs() < 1e-6);
        assert!((rollup_cost(&root.children[0]) - 0.32).abs() < 1e-6);
        assert!((rollup_cost(&root.children[2]) - 0.27).abs() < 1e-6);
    }

    #[test]
    fn folding_branches_and_resolving_gates_mutate_tree_cleanly() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let tree = app.new(|cx| AgentTree::new("tree-test", cx));
        tree.update(&mut app, |t, cx| {
            assert!(!t.is_branch_collapsed("researcher"));
            t.toggle_branch("researcher", cx);
            assert!(t.is_branch_collapsed("researcher"));
            t.toggle_branch("researcher", cx);
            assert!(!t.is_branch_collapsed("researcher"));

            let before = t.total_cost();
            t.resolve_agent("writer", TreeResolveAction::Approve, cx);
            assert!(t.total_cost() > before);
            let writer = &t.root.as_ref().expect("root present").children[1];
            assert_eq!(writer.status, TreeStatus::Running);
        });
    }
}
