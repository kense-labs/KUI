//! Live key style, remapping and searchable shortcut reference.
use gpui::{prelude::*, *};
use kui::components::{
    segmented_control::{FormOption, SegmentedControl, SelectionChanged},
    shortcut_recorder::{ShortcutChanged, ShortcutPlatform, ShortcutRecorder},
};
use kui::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::Paint,
    form_chrome::{col, glyph, hint, row, tr},
    glyph::{Glyph, icon},
};
const NAMES: [(&str, &str); 9] = [
    ("Open command menu", "打开命令菜单"),
    ("Create issue", "创建问题"),
    ("Toggle sidebar", "切换侧栏"),
    ("Go to inbox", "前往收件箱"),
    ("Go to projects", "前往项目"),
    ("Search everything", "全局搜索"),
    ("Bold", "粗体"),
    ("Insert link", "插入链接"),
    ("Save draft", "保存草稿"),
];
const DEFAULTS: [&str; 9] = [
    "Mod+K",
    "Alt+N",
    "Mod+.",
    "Alt+1",
    "Alt+2",
    "Shift+Mod+F",
    "Mod+B",
    "Shift+Mod+L",
    "Mod+S",
];
pub(super) struct ShortcutDemo {
    fields: Vec<Entity<ShortcutRecorder>>,
    platform: ShortcutPlatform,
    chooser: Entity<SegmentedControl>,
    values: Vec<String>,
    search: Entity<Editor>,
    anchor: FocusHandle,
    opened: bool,
    held: Modifiers,
    pressed_key: Option<Keystroke>,
    _subs: Vec<Subscription>,
}
impl ShortcutDemo {
    pub(super) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let platform = ShortcutPlatform::detected();
        let chooser = cx.new(|cx| {
            SegmentedControl::new(
                tr(cx, "Key style", "键帽样式"),
                vec![
                    FormOption::new("mac", "Mac"),
                    FormOption::new("windows", "Windows"),
                ],
                w,
                cx,
            )
            .with_selected(&[if platform == ShortcutPlatform::Mac {
                "mac"
            } else {
                "windows"
            }])
        });
        let search = cx.new(|cx| {
            let mut e = Editor::new(
                tr(cx, "Search shortcuts", "搜索快捷键"),
                InputMode::Search,
                w,
                cx,
            );
            e.set_placeholder(tr(cx, "Search shortcuts", "搜索快捷键"));
            e
        });
        let mut subs = vec![];
        let mut fields = vec![];
        for (i, (en, zh)) in NAMES[..3].iter().enumerate() {
            let field =
                cx.new(|cx| ShortcutRecorder::new(tr(cx, en, zh), Some(DEFAULTS[i].into()), cx));
            subs.push(
                cx.subscribe(&field, move |s: &mut Self, _, ev: &ShortcutChanged, cx| {
                    let next = ev.0.clone().unwrap_or_default();
                    s.opened = false;
                    for j in 0..s.values.len() {
                        if i != j && s.platform.matches(&s.values[j], &next) {
                            s.values[j].clear();
                            if let Some(old) = s.fields.get(j) {
                                old.update(cx, |f, cx| f.set_value(None, cx));
                            }
                        }
                    }
                    s.values[i] = next;
                    s.bind(cx);
                    cx.notify();
                }),
            );
            fields.push(field);
        }
        subs.push(
            cx.subscribe(&chooser, |s: &mut Self, _, event: &SelectionChanged, cx| {
                s.platform = if event.0.first().is_some_and(|k| k.as_ref() == "mac") {
                    ShortcutPlatform::Mac
                } else {
                    ShortcutPlatform::Windows
                };
                for field in &s.fields {
                    field.update(cx, |f, cx| f.set_platform(s.platform, cx));
                }
                s.opened = false;
                s.held = Modifiers::default();
                s.pressed_key = None;
                cx.notify()
            }),
        );
        subs.push(
            cx.subscribe_in(&search, w, |s: &mut Self, e, ev: &EditEvent, w, cx| {
                if *ev == EditEvent::Escape {
                    e.update(cx, |f, cx| f.set_value("", w, cx));
                }
                s.opened = false;
                cx.notify()
            }),
        );
        subs.push(cx.observe_window_activation(w, |s: &mut Self, w, cx| {
            if !w.is_window_active() {
                s.held = Modifiers::default();
                s.pressed_key = None;
                cx.notify()
            }
        }));
        let demo = Self {
            fields,
            platform,
            chooser,
            values: DEFAULTS.map(str::to_owned).to_vec(),
            search,
            anchor: cx.focus_handle().tab_stop(true),
            opened: false,
            held: Modifiers::default(),
            pressed_key: None,
            _subs: subs,
        };
        demo.bind(cx);
        demo
    }
    fn bind(&self, cx: &mut Context<Self>) {
        for (i, field) in self.fields.iter().enumerate() {
            let bindings = self
                .values
                .iter()
                .enumerate()
                .filter(|(j, k)| i != *j && !k.is_empty())
                .map(|(j, k)| (k.clone(), tr(cx, NAMES[j].0, NAMES[j].1).into()))
                .collect();
            field.update(cx, |f, _| f.set_bindings(bindings));
        }
    }
}
impl Render for ShortcutDemo {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let narrow = w.viewport_size().width < px(425.);
        let inset = px(if narrow { 16. } else { 24. });
        let header = row()
            .justify_between()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(px(18.))
                    .child(tr(cx, "Shortcuts", "快捷键")),
            )
            .child(div().w(px(148.)).flex_none().child(self.chooser.clone()));
        let mut form = col().p(inset).gap(px(16.)).child(header);
        for field in &self.fields {
            form = form.child(field.clone())
        }
        let binding = &self.values[0];
        let mut demo = row()
            .id("test-shortcut")
            .debug_selector(|| "Test shortcut".into())
            .track_focus(&self.anchor)
            .tab_index(0)
            .flex_wrap()
            .gap(px(4.))
            .min_h(px(30.))
            .mt(px(2.))
            .text_size(px(12.))
            .line_height(px(17.))
            .text_color(p.secondary)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, _, w, cx| s.anchor.focus(w, cx)),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if s.platform.matches_keystroke(&s.values[0], &e.keystroke) {
                    s.opened = true;
                    w.prevent_default();
                    cx.stop_propagation();
                    cx.notify()
                } else if e.keystroke.key == "escape" {
                    s.opened = false;
                    cx.notify()
                }
            }))
            .rounded(kui::foundation::surface_radius(cx))
            .focus_visible(move |s| s.bg(p.hover));
        demo = if self.opened {
            demo.child(tr(cx, "Command menu opened", "命令菜单已打开"))
        } else if binding.is_empty() {
            demo.child(tr(
                cx,
                "Record a shortcut to try it here",
                "先录制快捷键，即可在此试用",
            ))
        } else {
            demo.child(tr(cx, "Press", "按"))
                .child(self.platform.keycaps(binding, true, self.held, cx))
                .child(tr(
                    cx,
                    "here to open the command menu",
                    "在这里打开命令菜单",
                ))
        };
        form = form.child(demo);
        let q = self.search.read(cx).value().trim().to_lowercase();
        let mut search = row()
            .h(px(36.))
            .gap(px(8.))
            .pl(px(12.))
            .pr(px(5.))
            .border_1()
            .border_color(p.line)
            .rounded(kui::foundation::surface_radius(cx))
            .bg(p.panel)
            .child(icon(Glyph::Search, 15., p.faint))
            .child(div().flex_1().h_full().min_w_0().child(self.search.clone()));
        if !q.is_empty() {
            search = search.child(
                glyph(
                    "clear-search",
                    tr(cx, "Clear search", "清空搜索"),
                    Glyph::Close,
                    cx,
                )
                .size(px(24.))
                .on_click(cx.listener(|s, _, w, cx| {
                    s.search.update(cx, |e, cx| {
                        e.set_value("", w, cx);
                        e.focus(w, cx)
                    })
                })),
            )
        }
        let mut list = col()
            .border_t_1()
            .border_color(p.line)
            .bg(p.panel.blend(p.stage.opacity(0.30)))
            .p(inset)
            .gap(px(0.))
            .child(search);
        let mut any = false;
        for (group, (en, zh)) in [
            ("Workspace", "工作空间"),
            ("Navigation", "导航"),
            ("Editing", "编辑"),
        ]
        .into_iter()
        .enumerate()
        {
            let mut first = true;
            for ((name_en, name_zh), key) in NAMES
                .iter()
                .copied()
                .zip(&self.values)
                .skip(3 * group)
                .take(3)
            {
                let name = tr(cx, name_en, name_zh);
                let spoken = self.platform.spoken(key);
                if !format!("{en} {zh} {name} {name_en} {key} {spoken}")
                    .to_lowercase()
                    .contains(&q)
                {
                    continue;
                }
                if first {
                    list = list.child(hint(tr(cx, en, zh), cx).mt(px(12.)).mb(px(4.)))
                }
                let matched = self
                    .pressed_key
                    .as_ref()
                    .is_some_and(|held| self.platform.matches_keystroke(key, held));
                let mut r = row()
                    .justify_between()
                    .min_h(px(36.))
                    .gap(px(8.))
                    .text_color(p.secondary)
                    .opacity(if self.platform.contains_modifiers(key, self.held) {
                        1.
                    } else {
                        0.38
                    })
                    .when(!first && !matched, |r| r.border_t_1().border_color(p.line))
                    .when(matched, |r| {
                        r.rounded(kui::foundation::surface_radius(cx))
                            .bg(p.hover)
                            .text_color(p.ink)
                    })
                    .child(div().flex_1().min_w_0().truncate().child(name));
                if key.is_empty() {
                    r = r.child(hint(tr(cx, "Unassigned", "未设置"), cx))
                } else {
                    r = r.child(self.platform.keycaps(key, true, self.held, cx))
                }
                list = list.child(r);
                any = true;
                first = false;
            }
        }
        if !any {
            list = list
                .child(hint(tr(cx, "No matching shortcuts", "没有匹配的快捷键"), cx).py(px(16.)))
        }
        col()
            .id("shortcuts-demo")
            .w_full()
            .max_w(px(522.))
            .gap_0()
            .border_1()
            .border_color(p.line)
            .rounded(kui::foundation::surface_radius(cx))
            .overflow_hidden()
            .capture_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                if !e.is_held {
                    s.pressed_key = Some(e.keystroke.clone());
                    cx.notify()
                }
            }))
            .capture_key_up(cx.listener(|s, e: &KeyUpEvent, _, cx| {
                if s.pressed_key
                    .as_ref()
                    .is_some_and(|k| k.key == e.keystroke.key)
                {
                    s.pressed_key = None;
                    cx.notify()
                }
            }))
            .on_modifiers_changed(cx.listener(|s, e: &ModifiersChangedEvent, _, cx| {
                if s.held != e.modifiers {
                    s.held = e.modifiers;
                    if s.pressed_key
                        .as_ref()
                        .is_some_and(|k| k.modifiers != s.held)
                    {
                        s.pressed_key = None;
                    }
                    cx.notify()
                }
            }))
            .bg(p.panel)
            .text_color(p.ink)
            .text_size(px(14.))
            .line_height(px(20.))
            .child(form)
            .child(list)
    }
}
