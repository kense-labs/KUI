//! An editable example board with host-owned tools, order and comment count.
use gpui::{prelude::*, *};
use kui::{
    components::dock::{Dock, DockIcon, DockItem, DockReordered, DockSelected},
    foundation::{Theme, ThemeMode, surface_radius},
    primitives::{
        control::UiLanguage,
        editor::{Editor, InputMode},
        family::{Paint, PhaseClock, Tone, ease, face},
        glyph::{Glyph, icon},
        presentation::{column, hint, row},
    },
};
use std::{cell::Cell, rc::Rc, sync::Arc};

pub const IDS: &[&str] = &["dock"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Board::new).into())
}

fn tools(lang: UiLanguage) -> Vec<DockItem> {
    let t = |en, zh| lang.text(en, zh);
    vec![
        DockItem::new("move", t("Move", "移动"), DockIcon::Pointer).with_shortcut("V"),
        DockItem::new("hand", t("Hand", "抓手"), DockIcon::Hand).with_shortcut("H"),
        DockItem::new("shapes", t("Shapes", "形状"), DockIcon::Rectangle).with_items(vec![
            DockItem::new("rectangle", t("Rectangle", "矩形"), DockIcon::Rectangle)
                .with_shortcut("R"),
            DockItem::new("ellipse", t("Ellipse", "椭圆"), DockIcon::Ellipse).with_shortcut("O"),
            DockItem::new("triangle", t("Triangle", "三角形"), DockIcon::Triangle),
            DockItem::new("star", t("Star", "星形"), DockIcon::Star),
        ]),
        DockItem::new("text", t("Text", "文本"), DockIcon::Text).with_shortcut("T"),
        DockItem::new("note", t("Sticky note", "便签"), DockIcon::Note).with_shortcut("N"),
        DockItem::new(
            "comment",
            t("Comment", "评论"),
            DockIcon::Glyph(Glyph::Comment),
        )
        .with_shortcut("C")
        .with_badge(2),
    ]
}

struct Pin {
    anchor: PinAnchor,
    y: Pixels,
    label: SharedString,
    visible: bool,
}

#[derive(Clone, Copy)]
enum PinAnchor {
    LeadPhoto,
    SidePhoto,
    Canvas(Pixels),
}

fn initial_pins(lang: UiLanguage) -> Vec<Pin> {
    vec![
        Pin {
            anchor: PinAnchor::LeadPhoto,
            y: px(45.),
            label: lang
                .text("Lead the board with this one?", "把这张放在开头？")
                .into(),
            visible: true,
        },
        Pin {
            anchor: PinAnchor::SidePhoto,
            y: px(54.),
            label: lang.text("Try a warmer tone", "试试暖一点的色调").into(),
            visible: true,
        },
    ]
}

#[derive(Clone, Copy)]
enum BoardShape {
    Rectangle,
    Ellipse,
    Triangle,
    Star,
}

impl BoardShape {
    fn size(self) -> Size<Pixels> {
        let (width, height) = match self {
            Self::Rectangle => (96., 64.),
            Self::Ellipse => (76., 76.),
            Self::Triangle => (84., 74.),
            Self::Star => (80., 77.),
        };
        size(px(width), px(height))
    }

    fn paint(
        self,
        bounds: Bounds<Pixels>,
        progress: f32,
        dark: bool,
        radius: Pixels,
        w: &mut Window,
    ) {
        let (outline, dark_fill, light_fill) = match self {
            Self::Rectangle => (0x2f8adc, 0x22303f, 0xd3e6f9),
            Self::Ellipse => (0x23ac74, 0x23362c, 0xd6eddf),
            Self::Triangle => (0xe48233, 0x412f23, 0xfce4d5),
            Self::Star => (0xdca81c, 0x3e3523, 0xf8ecd5),
        };
        let outline = rgb(outline).opacity(progress);
        let fill = rgb(if dark { dark_fill } else { light_fill }).opacity(progress);
        let scale = 0.86 + progress * 0.14;
        let inset = 0.25 * scale;
        let area = size(bounds.size.width * scale, bounds.size.height * scale);
        let origin = bounds.center() - point(area.width / 2., area.height / 2.);
        let stroke = px(1.5 * scale);
        if matches!(self, Self::Rectangle | Self::Ellipse) {
            let rect = Bounds::new(
                origin + point(px(inset), px(inset)),
                size(area.width - px(2. * inset), area.height - px(2. * inset)),
            );
            let corners = match self {
                Self::Ellipse => rect.size.width / 2.,
                _ => radius * scale,
            };
            w.paint_quad(quad(
                rect,
                corners,
                fill,
                stroke,
                outline,
                BorderStyle::Solid,
            ));
            return;
        }

        let star = std::array::from_fn::<_, 10, _>(|i| {
            let angle = std::f32::consts::PI * (i as f32 / 5. - 0.5);
            let r = if i % 2 == 0 { 41. } else { 20.6 };
            (40. + r * angle.cos(), 41. + r * angle.sin())
        });
        let triangle = [(42., -3.), (86., 73.5), (-2., 73.5)];
        let (vertices, cut): (&[(f32, f32)], f32) = match self {
            Self::Triangle => (&triangle, 9.5),
            _ => (&star, 3.),
        };
        let at = |(x, y): (f32, f32)| origin + point(px(x * scale), px(y * scale));
        let toward = |(x, y): (f32, f32), (a, b): (f32, f32)| {
            let ratio = cut / (a - x).hypot(b - y);
            at((x + (a - x) * ratio, y + (b - y) * ratio))
        };
        let path = |mut p: PathBuilder| {
            for (i, &vertex) in vertices.iter().enumerate() {
                let before = toward(vertex, vertices[(i + vertices.len() - 1) % vertices.len()]);
                let after = toward(vertex, vertices[(i + 1) % vertices.len()]);
                if i == 0 {
                    p.move_to(before);
                } else {
                    p.line_to(before);
                }
                p.curve_to(after, at(vertex));
            }
            p.close();
            p.build().ok()
        };
        if let Some(path) = path(PathBuilder::fill()) {
            w.paint_path(path, fill);
        }
        if let Some(path) = path(PathBuilder::stroke(stroke)) {
            w.paint_path(path, outline);
        }
    }
}

enum Mark {
    Shape(BoardShape, PhaseClock),
    Text(Entity<Editor>),
    Note(Entity<Editor>),
}

struct Placed {
    at: Point<Pixels>,
    mark: Mark,
}

#[derive(Clone, Copy)]
enum BoardDrag {
    Pan(Point<Pixels>),
    Photo(usize, Point<Pixels>),
}

struct Board {
    dock: Entity<Dock>,
    items: Vec<DockItem>,
    selected: SharedString,
    selection_label: SharedString,
    images: [ImageSource; 2],
    pins: Vec<Pin>,
    placed: Vec<Placed>,
    photo_offsets: [Point<Pixels>; 2],
    pan: Point<Pixels>,
    drag: Option<(BoardDrag, Point<Pixels>)>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    focus: FocusHandle,
    lang: UiLanguage,
    dirty: bool,
    _subscriptions: [Subscription; 2],
}

impl Board {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let items = tools(lang);
        let dock = cx.new(|cx| {
            Dock::new(items.clone(), cx)
                .with_label(lang.text("Board tools", "画板工具"))
                .with_reorderable(true)
                .with_value("move", cx)
        });
        let subscriptions = [
            cx.subscribe(&dock, |s, _, e: &DockSelected, cx| {
                s.select(e.0.clone(), cx)
            }),
            cx.subscribe(&dock, |s, _, e: &DockReordered, cx| {
                s.items = e.items.clone();
                s.dirty = true;
                s.sync(cx);
            }),
        ];
        let picture =
            |bytes: &[u8]| Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec())).into();
        Self {
            dock,
            items,
            selected: "move".into(),
            selection_label: lang.text("Move", "移动").into(),
            images: [
                picture(include_bytes!("../../assets/cover-flow/crescent.jpg")),
                picture(include_bytes!("../../assets/places/coast.jpg")),
            ],
            pins: initial_pins(lang),
            placed: vec![],
            photo_offsets: [Point::default(); 2],
            pan: Point::default(),
            drag: None,
            bounds: Rc::default(),
            focus: cx.focus_handle().tab_stop(true),
            lang,
            dirty: false,
            _subscriptions: subscriptions,
        }
    }

    fn select(&mut self, key: SharedString, cx: &mut Context<Self>) {
        if let Some(tool) = self
            .items
            .iter()
            .flat_map(|i| std::iter::once(i).chain(i.items.iter()))
            .find(|i| i.id == key)
        {
            self.selection_label = tool.label.clone();
            self.dirty |= self.selected != key;
            self.selected = key;
            self.dock.update(cx, |dock, cx| {
                dock.set_value(Some(self.selected.clone()), cx)
            });
            cx.notify();
        }
    }

    fn sync(&mut self, cx: &mut Context<Self>) {
        if let Some(comment) = self.items.iter_mut().find(|i| i.id.as_ref() == "comment") {
            comment.badge = self.pins.iter().filter(|pin| pin.visible).count() as u32;
        }
        self.dock
            .update(cx, |dock, cx| dock.set_items(self.items.clone(), cx));
        cx.notify();
    }

    fn reset(&mut self, cx: &mut Context<Self>) {
        self.items = tools(self.lang);
        self.pins = initial_pins(self.lang);
        self.placed.clear();
        self.pan = Point::default();
        self.photo_offsets = [Point::default(); 2];
        self.drag = None;
        self.sync(cx);
        self.select("move".into(), cx);
        self.dirty = false;
        cx.notify();
    }

    fn place(&mut self, position: Point<Pixels>, w: &mut Window, cx: &mut Context<Self>) {
        let at = position - self.bounds.get().origin - self.pan;
        if self.selected.as_ref() == "comment" {
            self.pins.push(Pin {
                anchor: PinAnchor::Canvas(at.x),
                y: at.y,
                label: self.lang.text("New comment", "新评论").into(),
                visible: true,
            });
            self.dirty = true;
            self.sync(cx);
            return;
        }
        let shape = |shape| {
            let mut entrance = PhaseClock::default();
            entrance.restart(cx);
            Mark::Shape(shape, entrance)
        };
        let mark = match self.selected.as_ref() {
            "rectangle" => shape(BoardShape::Rectangle),
            "ellipse" => shape(BoardShape::Ellipse),
            "triangle" => shape(BoardShape::Triangle),
            "star" => shape(BoardShape::Star),
            "text" | "note" => {
                let note = self.selected.as_ref() == "note";
                let editor = cx.new(|cx| {
                    let mut editor = Editor::new(
                        self.lang.text("Board text", "画板文字"),
                        if note {
                            InputMode::Multi
                        } else {
                            InputMode::Single
                        },
                        w,
                        cx,
                    );
                    editor.set_placeholder(self.lang.text("Type here…", "输入文字…"));
                    editor
                });
                let focus = editor.read(cx).focus.clone();
                cx.defer_in(w, move |_, w, cx| {
                    focus.focus(w, cx);
                    w.request_virtual_keyboard();
                });
                if note {
                    Mark::Note(editor)
                } else {
                    Mark::Text(editor)
                }
            }
            _ => return,
        };
        self.placed.push(Placed { at, mark });
        self.dirty = true;
        cx.notify();
    }

    fn shortcut(&mut self, event: &KeyDownEvent, w: &Window, cx: &mut Context<Self>) {
        let editing = self.placed.iter().any(|mark| match &mark.mark {
            Mark::Text(editor) | Mark::Note(editor) => editor.read(cx).focus.is_focused(w),
            Mark::Shape(..) => false,
        });
        if editing || event.keystroke.modifiers.modified() || event.is_held {
            return;
        }
        let key = event.keystroke.key.to_ascii_uppercase();
        let selected = self
            .items
            .iter()
            .flat_map(|item| std::iter::once(item).chain(item.items.iter()))
            .find(|item| item.shortcut.as_deref() == Some(key.as_str()))
            .map(|i| i.id.clone());
        if let Some(selected) = selected {
            self.select(selected, cx);
            cx.stop_propagation();
        }
    }
}

impl Render for Board {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let width = self.bounds.get().size.width;
        let compact = if width == px(0.) {
            w.viewport_size().width
        } else {
            width
        } < px(420.);
        let lang = self.lang;
        let radius = surface_radius(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let caption = [
            lang.text("Lake Crescent, morning", "清晨的新月湖"),
            lang.text("Atlantic coast, dusk", "暮色中的大西洋海岸"),
        ];
        let mut scene = div()
            .id("dock-board-scene")
            .relative()
            .size_full()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| {
                    s.focus.focus(w, cx);
                    if s.selected.as_ref() == "hand" {
                        s.drag = Some((BoardDrag::Pan(s.pan), e.position));
                    }
                }),
            )
            .on_click(cx.listener(|s, e: &ClickEvent, w, cx| s.place(e.position(), w, cx)));
        for (i, image) in self.images.iter().enumerate() {
            let width = if i == 0 { 0.26 } else { 0.34 };
            let height = if i == 0 { 171. } else { 120. };
            let offset = self.photo_offsets[i] + self.pan;
            scene = scene.child(
                column()
                    .id(("dock-photo", i))
                    .absolute()
                    .left(relative(if i == 0 { 0.08 } else { 0.56 }))
                    .top(px(if i == 0 { 53. } else { 70. }))
                    .ml(offset.x)
                    .mt(offset.y)
                    .w(relative(width))
                    .gap(px(5.))
                    .child(
                        img(image.clone())
                            .w_full()
                            .h(px(if compact { height * 0.75 } else { height }))
                            .object_fit(ObjectFit::Cover)
                            .rounded(radius),
                    )
                    .child(hint(caption[i], p))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |s, e: &MouseDownEvent, w, cx| {
                            if s.selected.as_ref() == "move" {
                                s.focus.focus(w, cx);
                                s.drag =
                                    Some((BoardDrag::Photo(i, s.photo_offsets[i]), e.position));
                                cx.stop_propagation();
                            }
                        }),
                    ),
            );
        }
        for placed in &self.placed {
            let at = placed.at + self.pan;
            let mark = match &placed.mark {
                Mark::Shape(shape, entrance) => {
                    let shape = *shape;
                    let size = shape.size();
                    let progress = ease(entrance.time(0.2, w, cx) / 0.2);
                    scene = scene.child(
                        canvas(
                            |_, _, _| (),
                            move |b, _, w, _| shape.paint(b, progress, dark, radius, w),
                        )
                        .absolute()
                        .left(at.x - size.width / 2.)
                        .top(at.y - size.height / 2.)
                        .w(size.width)
                        .h(size.height),
                    );
                    continue;
                }
                Mark::Text(editor) => row()
                    .w(px(120.))
                    .px(px(6.))
                    .h(px(28.))
                    .bg(p.panel.opacity(0.8))
                    .rounded(radius)
                    .child(editor.clone()),
                Mark::Note(editor) => row()
                    .items_start()
                    .size(px(104.))
                    .p(px(8.))
                    .bg(p.panel.blend(rgb(0xd2a755).opacity(0.25).into()))
                    .rounded(radius)
                    .child(editor.clone()),
            };
            let origin = at - point(px(32.), px(32.));
            scene = scene.child(mark.absolute().left(origin.x).top(origin.y));
        }
        for (i, pin) in self.pins.iter().enumerate().filter(|(_, pin)| pin.visible) {
            let button = face(
                ("dock-pin", i),
                pin.label.clone(),
                30.,
                radius.into(),
                Tone::Surface,
                cx,
            )
            .absolute()
            .top(pin.y + self.pan.y)
            .border_1()
            .border_color(p.line)
            .gap(px(7.))
            .text_size(px(12.))
            .font_weight(FontWeight::NORMAL)
            .shadow_sm();
            let button = match pin.anchor {
                PinAnchor::LeadPhoto => button
                    .left(relative(0.306))
                    .ml(self.pan.x + self.photo_offsets[0].x)
                    .mt(self.photo_offsets[0].y),
                PinAnchor::SidePhoto => button
                    .right(relative(0.117))
                    .mr(-self.pan.x - self.photo_offsets[1].x)
                    .mt(self.photo_offsets[1].y)
                    .flex_row_reverse(),
                PinAnchor::Canvas(x) => button.left(x + self.pan.x),
            };
            let marker = match pin.anchor {
                PinAnchor::LeadPhoto => Some((0xddbd6a, 0x8c77cc)),
                PinAnchor::SidePhoto => Some((0xbc6ec7, 0x69c99c)),
                PinAnchor::Canvas(_) => None,
            };
            scene =
                scene.child(
                    button
                        .px(px(3.))
                        .child(row().size(px(22.)).justify_center().rounded_full().map(
                            |marker_view| match marker {
                                Some((a, b)) => marker_view.bg(linear_gradient(
                                    135.,
                                    linear_color_stop(rgb(a), 0.),
                                    linear_color_stop(rgb(b), 1.),
                                )),
                                None => marker_view.child(icon(Glyph::Comment, 12., p.faint)),
                            },
                        ))
                        .when(!compact, |button| {
                            button.child(div().px(px(3.)).child(pin.label.clone()))
                        })
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.pins[i].visible = false;
                            s.dirty = true;
                            s.sync(cx);
                            cx.stop_propagation();
                        })),
                );
        }
        let bounds = self.bounds.clone();
        let movement = cx.listener(|s, event: &MouseMoveEvent, _, cx| {
            if let Some((drag, at)) = s.drag {
                let delta = event.position - at;
                if delta.x.abs() + delta.y.abs() > px(1.) {
                    match drag {
                        BoardDrag::Pan(start) => s.pan = start + delta,
                        BoardDrag::Photo(i, start) => s.photo_offsets[i] = start + delta,
                    }
                    s.dirty = true;
                    cx.notify();
                }
            }
        });
        let up = cx.listener(|s, _: &MouseUpEvent, _, _| {
            s.drag = None;
        });
        let dots = canvas(
            move |b, w, _| {
                if bounds.replace(b).size.width != b.size.width {
                    w.request_animation_frame();
                }
            },
            move |b, _, w, _| {
                w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                    if phase.bubble() {
                        movement(e, w, cx);
                    }
                });
                w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                    if phase.bubble() && e.button == MouseButton::Left {
                        up(e, w, cx);
                    }
                });
                for y in (12..f32::from(b.size.height).ceil() as i32).step_by(22) {
                    for x in (12..f32::from(b.size.width).ceil() as i32).step_by(22) {
                        w.paint_quad(
                            fill(
                                Bounds::new(
                                    b.origin + point(px(x as f32), px(y as f32)),
                                    size(px(2.), px(2.)),
                                ),
                                p.ink.opacity(0.085),
                            )
                            .corner_radii(px(1.)),
                        );
                    }
                }
            },
        )
        .absolute()
        .inset_0();
        let board = div()
            .id("dock-board")
            .track_focus(&self.focus)
            .relative()
            .w(px(522.))
            .max_w_full()
            .h(px(446.))
            .rounded(radius)
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .on_key_down(cx.listener(|s, event: &KeyDownEvent, w, cx| s.shortcut(event, w, cx)))
            .child(dots)
            .child(scene)
            .when(self.dirty, |board| {
                board.child(
                    face(
                        "dock-board-reset",
                        lang.text("Reset board", "重置画板"),
                        32.,
                        radius.into(),
                        Tone::Surface,
                        cx,
                    )
                    .absolute()
                    .right(px(12.))
                    .top(px(12.))
                    .w(px(32.))
                    .border_1()
                    .border_color(p.line)
                    .child(icon(Glyph::Undo, 16., p.ink))
                    .on_click(cx.listener(|s, _, _, cx| s.reset(cx)))
                    .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                        if !e.is_held && matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            s.reset(cx);
                            cx.stop_propagation();
                        }
                    })),
                )
            })
            .child(
                row()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom(px(26.))
                    .justify_center()
                    .child(self.dock.clone()),
            );
        let status = if lang == UiLanguage::Chinese {
            format!("当前工具：{}", self.selection_label)
        } else {
            format!("{} is active", self.selection_label)
        };
        column()
            .w_full()
            .max_w(px(540.))
            .px(px(6.))
            .gap(px(8.))
            .items_center()
            .child(board)
            .child(
                div()
                    .id("dock-status")
                    .role(Role::Status)
                    .absolute()
                    .size(px(1.))
                    .overflow_hidden()
                    .child(hint(status, p)),
            )
    }
}
