//! Find and activate a team without leaving the current workspace.
use crate::primitives::brand_mark::brand_mark;
use crate::primitives::ops_surface::*;
use crate::primitives::{
    editor::{EditEvent, Editor, EditorScrollRegion, InputMode},
    family::{Paint, PhaseClock, disabled_motion, standard_ease},
    glyph::{Glyph, icon, transformed},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
const ROW_HEIGHT: f32 = 48.;
const VISIBLE_ROWS: f32 = 6.;

pub struct Workspace {
    pub id: String,
    pub name: String,
    pub plan: String,
    pub members: u32,
    pub role: String,
}
pub enum WorkspaceEvent {
    Switch(String),
    Create(String),
    Join(String),
}
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Search,
    Create,
    Join,
}
pub struct WorkspacePicker {
    id: SharedString,
    pub teams: Vec<Workspace>,
    pub current: usize,
    open: bool,
    mode: Mode,
    cursor: usize,
    scroll: ScrollHandle,
    search: Entity<Editor>,
    name: Entity<Editor>,
    invite: Entity<Editor>,
    route: PhaseClock,
    list_height: Rc<Cell<Pixels>>,
    _subscriptions: Vec<Subscription>,
    trigger: FocusHandle,
}
impl EventEmitter<WorkspaceEvent> for WorkspacePicker {}
impl WorkspacePicker {
    pub fn new(
        id: impl Into<SharedString>,
        teams: Vec<Workspace>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = editor(
            tr(cx, "Find a workspace", "搜索工作区").into(),
            InputMode::Combobox,
            w,
            cx,
        );
        let name = editor(
            tr(cx, "Workspace name", "工作区名称").into(),
            InputMode::Single,
            w,
            cx,
        );
        let invite = editor(
            tr(cx, "Invite code or link", "邀请码或邀请链接").into(),
            InputMode::Single,
            w,
            cx,
        );
        name.update(cx, |e, cx| {
            e.set_placeholder(tr(cx, "e.g. Northwind Labs", "例如，松林工作室"))
        });
        invite.update(cx, |e, cx| {
            e.set_placeholder(tr(cx, "Paste a code or invite link", "粘贴邀请码或链接"))
        });
        let subscriptions = [
            (Mode::Search, &search),
            (Mode::Create, &name),
            (Mode::Join, &invite),
        ]
        .into_iter()
        .map(|(mode, field)| {
            cx.subscribe_in(field, w, move |s: &mut Self, _, ev: &EditEvent, w, cx| {
                if !s.open || s.mode != mode {
                    return;
                }
                match ev {
                    EditEvent::Changed if mode == Mode::Search => {
                        s.cursor = 0;
                        s.scroll.set_offset(Point::default());
                    }
                    EditEvent::Submitted => s.submit(w, cx),
                    EditEvent::Escape => s.escape(w, cx),
                    EditEvent::Navigate(d) if mode == Mode::Search => {
                        let n = s.matches(cx).len();
                        let total = n + 2;
                        let next = if *d {
                            (s.cursor + 1) % total
                        } else {
                            (s.cursor + total - 1) % total
                        };
                        s.highlight(next, n);
                    }
                    _ => {}
                }
                cx.notify()
            })
        })
        .collect();
        Self {
            id: id.into(),
            teams,
            current: 0,
            open: true,
            mode: Mode::Search,
            cursor: 0,
            scroll: ScrollHandle::new(),
            search,
            name,
            invite,
            route: PhaseClock::default(),
            list_height: Rc::new(Cell::new(px(0.))),
            _subscriptions: subscriptions,
            trigger: cx.focus_handle().tab_stop(true),
        }
    }
    fn matches(&self, cx: &App) -> Vec<usize> {
        let q = self.search.read(cx).value().to_lowercase();
        self.teams
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                [&t.name, &t.plan, &t.role]
                    .iter()
                    .any(|s| s.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect()
    }
    fn highlight(&mut self, cursor: usize, rows: usize) {
        self.cursor = cursor;
        if rows != 0 {
            self.scroll.scroll_to_item(cursor.min(rows - 1));
        }
    }
    fn close(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.mode = Mode::Search;
        self.cursor = 0;
        self.scroll.set_offset(Point::default());
        for editor in [&self.search, &self.name, &self.invite] {
            replace(editor, "", w, cx);
        }
        self.route = PhaseClock::default();
        self.trigger.focus(w, cx);
        cx.notify()
    }
    fn field(&self) -> &Entity<Editor> {
        match self.mode {
            Mode::Search => &self.search,
            Mode::Create => &self.name,
            Mode::Join => &self.invite,
        }
    }
    fn escape(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.mode != Mode::Search {
            self.form(Mode::Search, w, cx);
        } else if !self.search.read(cx).value().is_empty() {
            replace(&self.search, "", w, cx);
            self.cursor = 0;
            self.search.read(cx).focus.clone().focus(w, cx);
        } else {
            self.close(w, cx);
        }
    }
    fn pick(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(t) = self.teams.get(i) {
            self.current = i;
            cx.emit(WorkspaceEvent::Switch(t.id.clone()));
            self.close(w, cx)
        }
    }
    fn submit(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        let v = self.field().read(cx).value().trim().to_owned();
        match self.mode {
            Mode::Search => {
                let matches = self.matches(cx);
                if let Some(&i) = matches.get(self.cursor) {
                    self.pick(i, w, cx)
                } else if self.cursor == matches.len() {
                    self.form(Mode::Create, w, cx);
                    if matches.is_empty() && !v.is_empty() {
                        replace(&self.name, &v, w, cx)
                    }
                } else {
                    self.form(Mode::Join, w, cx)
                }
            }
            Mode::Create | Mode::Join if !v.is_empty() => {
                cx.emit(if self.mode == Mode::Create {
                    WorkspaceEvent::Create(v)
                } else {
                    WorkspaceEvent::Join(v)
                });
                self.close(w, cx)
            }
            _ => {}
        }
    }
    fn form(&mut self, m: Mode, w: &mut Window, cx: &mut Context<Self>) {
        if self.mode != Mode::Search {
            replace(self.field(), "", w, cx);
        }
        self.mode = m;
        self.route.restart(cx);
        self.field().read(cx).focus.clone().focus(w, cx);
        cx.notify()
    }
}
fn workspace_mark(name: &str, i: usize, cx: &App) -> Div {
    if let Some(mark) = brand_mark(name, 16., cx) {
        div()
            .size(px(28.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .bg(Paint::get(cx).stage)
            .child(mark)
    } else {
        avatar(name, i, cx)
    }
}

impl Render for WorkspacePicker {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::Theme::global(cx).tokens.radius.md;
        let team = self.teams.get(self.current);
        let name = team
            .map(|t| t.name.clone())
            .unwrap_or_else(|| tr(cx, "Workspaces", "工作区").into());
        let mut r = column()
            .id(self.id.clone())
            .w_full()
            .max_w(px(360.))
            .text_color(p.ink)
            .child(
                item("workspace", name.clone(), cx, |s, w, cx| {
                    if s.open {
                        s.close(w, cx)
                    } else {
                        s.open = true;
                        s.search.read(cx).focus.clone().focus(w, cx);
                        cx.notify()
                    }
                })
                .track_focus(&self.trigger)
                .rounded(radius)
                .aria_expanded(self.open)
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(workspace_mark(&name, self.current, cx).rounded(radius))
                .child(
                    column()
                        .flex_1()
                        .gap_0()
                        .child(swap_label("workspace-name", name, w, cx))
                        .when_some(team, |el, t| {
                            el.child(note(
                                format!(
                                    "{} · {} {}",
                                    t.plan,
                                    t.members,
                                    tr(cx, "members", "名成员")
                                ),
                                cx,
                            ))
                        }),
                )
                .child(icon(Glyph::Down, 14., p.secondary)),
            );
        if (!self.open || self.mode != Mode::Search)
            && let Some(host) = cx.try_global::<EditorScrollRegion>()
        {
            (host.0)(Bounds::default(), false, false);
        }
        let layer = Fold::get("workspace-layer", self.open, 360., 0.18, w, cx);
        if !layer.mounted() {
            return r;
        }
        let tokens = &crate::foundation::Theme::global(cx).tokens;
        let padding = tokens.space.two;
        let gap = tokens.space.three;
        let edge = px(1.);
        let chrome_height = px(36. + 1. + 32.) + gap * 3. + (padding + edge) * 2.;
        // Reserve the trigger and, on narrow windows, a second footer action line.
        let window = w.viewport_size();
        let wrapped_footer = if window.width < px(300.) {
            px(40.)
        } else {
            px(0.)
        };
        let list_viewport = (window.height - chrome_height - px(64.) - wrapped_footer)
            .clamp(px(ROW_HEIGHT), px(ROW_HEIGHT * VISIBLE_ROWS));
        let active = self.field().clone();
        let focus = active.read(cx).focus.clone();
        let focused = focus.is_focused(w);
        let field = if self.mode == Mode::Search {
            row()
                .w_full()
                .h(px(36.))
                .flex_none()
                .px(tokens.space.two)
                .border_b_1()
                .border_color(if focused { p.ink } else { p.line })
                .child(icon(Glyph::Search, 16., p.secondary))
                .child(div().h_full().min_w_0().flex_1().child(active.clone()))
        } else {
            input(&active, cx)
                .rounded(radius)
                .border_color(if focused { p.ink } else { p.line })
        }
        .cursor(CursorStyle::IBeam)
        .on_mouse_down(MouseButton::Left, move |_, w, cx| focus.focus(w, cx));
        let age = self.route.age(cx);
        let progress = standard_ease((age / 0.16).clamp(0., 1.));
        if age < 0.16 {
            w.request_animation_frame();
        }
        let direction = if self.mode == Mode::Search { -1. } else { 1. };
        let offset = if disabled_motion(cx) {
            0.
        } else {
            direction * 6. * (1. - progress)
        };
        let mut body = column()
            .gap(gap)
            .w_full()
            .min_w_0()
            .flex_1()
            .relative()
            .left(px(offset))
            .opacity(progress);
        if self.mode == Mode::Search {
            body = body.child(field);
        } else {
            let heading = if self.mode == Mode::Create {
                tr(cx, "Create workspace", "创建工作区")
            } else {
                tr(cx, "Join workspace", "加入工作区")
            };
            body = body
                .child(
                    row()
                        .child(
                            item(
                                "back",
                                tr(cx, "Back to workspaces", "返回工作区列表"),
                                cx,
                                |s, w, cx| s.form(Mode::Search, w, cx),
                            )
                            .rounded(radius)
                            .size(px(30.))
                            .p_0()
                            .justify_center()
                            .child(transformed(
                                Glyph::Arrow,
                                15.,
                                p.secondary,
                                std::f32::consts::PI,
                                1.,
                                (0., 0.),
                                1.,
                            )),
                        )
                        .child(title(heading)),
                )
                .child(divider(cx));
            if self.mode == Mode::Create {
                let draft = active.read(cx).value().trim();
                let name = if draft.is_empty() {
                    tr(cx, "Untitled workspace", "未命名工作区")
                } else {
                    draft
                };
                body = body.child(
                    row()
                        .child(
                            avatar(name, self.teams.len(), cx)
                                .rounded(radius)
                                .size(px(36.)),
                        )
                        .child(
                            column()
                                .min_w_0()
                                .flex_1()
                                .gap_0()
                                .child(div().truncate().child(name.to_owned()))
                                .child(note(
                                    tr(
                                        cx,
                                        "Team plan · You are the owner",
                                        "Team 方案 · 你是所有者",
                                    ),
                                    cx,
                                )),
                        ),
                );
            }
            let label = if self.mode == Mode::Create {
                tr(cx, "Workspace name", "工作区名称")
            } else {
                tr(cx, "Invite code or link", "邀请码或邀请链接")
            };
            body = body.child(column().gap(px(6.)).child(title(label)).child(field));
            if self.mode == Mode::Join {
                body = body.child(note(
                    tr(
                        cx,
                        "Use the code or link from your workspace invitation.",
                        "粘贴工作区邀请中提供的邀请码或链接。",
                    ),
                    cx,
                ));
            }
        }
        let menu = panel(&"teams".into(), 360., cx)
            .p(padding)
            .rounded(radius)
            .min_h(chrome_height + list_viewport)
            .capture_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if s.open
                    && s.mode == Mode::Search
                    && s.search.read(cx).focus.is_focused(w)
                    && s.search.read(cx).value().is_empty()
                    && matches!(e.keystroke.key.as_str(), "home" | "end")
                {
                    let n = s.matches(cx).len();
                    s.highlight(if e.keystroke.key == "home" { 0 } else { n + 1 }, n);
                    w.prevent_default();
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if !s.open {
                    return;
                }
                if e.keystroke.key == "escape" {
                    s.escape(w, cx);
                    cx.stop_propagation();
                } else if s.mode == Mode::Search
                    && (e.keystroke.modifiers.platform || e.keystroke.modifiers.control)
                    && let Ok(n) = e.keystroke.key.parse::<usize>()
                    && (1..=9).contains(&n)
                    && n <= s.teams.len()
                {
                    s.pick(n - 1, w, cx);
                    cx.stop_propagation()
                }
            }));
        if self.mode == Mode::Search {
            let matches = self.matches(cx);
            let n = matches.len();
            let mut list = column()
                .id("teams-list")
                .role(Role::ListBox)
                .aria_label(tr(cx, "Workspaces", "工作区"))
                .debug_selector(|| "workspace-list-viewport".into())
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .on_scroll(cx.listener(|_, _, _, cx| cx.notify()))
                .gap_0();
            for (pos, i) in matches.into_iter().enumerate() {
                let t = &self.teams[i];
                list = list.child(
                    item(("team", i), t.name.clone(), cx, move |s, w, cx| {
                        s.pick(i, w, cx)
                    })
                    .rounded(radius)
                    .role(Role::ListBoxOption)
                    .aria_selected(i == self.current)
                    .h(px(ROW_HEIGHT))
                    .flex_none()
                    .py_0()
                    .when(pos == self.cursor, |e| e.bg(p.stage))
                    .child(workspace_mark(&t.name, i, cx).rounded(radius))
                    .child(
                        column()
                            .flex_1()
                            .gap_0()
                            .child(title(t.name.clone()))
                            .child(note(t.plan.clone(), cx)),
                    )
                    .child(badge(t.role.clone(), cx).rounded(radius))
                    .when(i == self.current, |r| {
                        r.child(icon(Glyph::Check, 14., p.ink))
                    }),
                )
            }
            let scroll = self.scroll.clone();
            let region = canvas(
                |_, _, _| (),
                move |bounds, _, _, cx| {
                    if let Some(host) = cx.try_global::<EditorScrollRegion>() {
                        let top = -scroll.offset().y;
                        (host.0)(bounds, top > px(0.1), top + px(0.1) < scroll.max_offset().y);
                    }
                },
            )
            .absolute()
            .inset_0();
            body = body
                .child(
                    div()
                        .relative()
                        .w_full()
                        .h(list_viewport)
                        .flex_none()
                        .child(list)
                        .when(self.open, |v| v.child(region)),
                )
                .child(divider(cx))
                .child(
                    row()
                        .flex_wrap()
                        .child(
                            btn(
                                "create",
                                tr(cx, "Create workspace", "创建工作区"),
                                false,
                                cx,
                                |s, w, cx| s.form(Mode::Create, w, cx),
                            )
                            .rounded(radius)
                            .when(self.cursor == n, |b| b.bg(p.stage)),
                        )
                        .child(
                            btn(
                                "join",
                                tr(cx, "Join with invite", "使用邀请加入"),
                                false,
                                cx,
                                |s, w, cx| s.form(Mode::Join, w, cx),
                            )
                            .rounded(radius)
                            .when(self.cursor == n + 1, |b| b.bg(p.stage)),
                        ),
                )
        } else {
            let ready = !active.read(cx).value().trim().is_empty();
            let submit = if self.mode == Mode::Create {
                tr(cx, "Create workspace", "创建工作区")
            } else {
                tr(cx, "Join workspace", "加入工作区")
            };
            body = body.child(div().flex_1()).child(divider(cx)).child(
                row()
                    .justify_end()
                    .flex_wrap()
                    .child(
                        btn(
                            "cancel",
                            tr(cx, "Cancel", "取消"),
                            false,
                            cx,
                            |s, w, cx| s.form(Mode::Search, w, cx),
                        )
                        .rounded(radius),
                    )
                    .child(
                        btn("submit", submit, true, cx, |s, w, cx| s.submit(w, cx))
                            .rounded(radius)
                            .when(!ready, |v| v.opacity(0.45).cursor_default().tab_index(-1)),
                    ),
            );
        }
        let panel = if self.mode == Mode::Search {
            let height = self.list_height.clone();
            div()
                .w_full()
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(menu) = bounds.first() {
                        height.set(menu.size.height);
                    }
                })
                .child(menu.child(body))
        } else {
            // On a narrow host the two footer commands can wrap. Keep the actual
            // list footprint so entering a form never moves the trigger.
            div()
                .w_full()
                .child(menu.min_h(self.list_height.get()).child(body))
        };
        r = r.child(layer.wrap(8., panel));
        r
    }
}
