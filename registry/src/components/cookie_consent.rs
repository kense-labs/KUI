//! Host-managed consent choices with a reopenable preferences panel.
use crate::{
    components::switch::Switch,
    primitives::{
        family::{Paint, Slide, Tone, disabled_motion, face},
        form_chrome::tr,
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};

#[derive(Clone)]
pub struct ConsentCategory {
    pub id: SharedString,
    pub label: SharedString,
    pub description: SharedString,
    pub required: bool,
    pub default_enabled: bool,
}
impl ConsentCategory {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: "".into(),
            required: false,
            default_enabled: false,
        }
    }
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
    pub fn with_default_enabled(mut self, enabled: bool) -> Self {
        self.default_enabled = enabled;
        self
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsentChoice {
    pub id: SharedString,
    pub enabled: bool,
}
/// The host is responsible for saving this decision and applying it to integrations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsentChanged(pub Vec<ConsentChoice>);
#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Banner,
    Preferences,
    Closed,
}
/// Consent control. Optional categories are off unless explicitly configured.
pub struct CookieConsent {
    categories: Vec<ConsentCategory>,
    switches: Vec<Entity<Switch>>,
    subscriptions: Vec<Subscription>,
    draft: Vec<bool>,
    saved: Option<Vec<ConsentChoice>>,
    panel: Panel,
    reveal: Slide,
    shape: Slide,
    fold: Slide,
    close_epoch: u64,
    focus: FocusHandle,
}
impl EventEmitter<ConsentChanged> for CookieConsent {}
impl CookieConsent {
    pub fn new(
        categories: Vec<ConsentCategory>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut switches = Vec::new();
        let mut subscriptions = Vec::new();
        let mut draft = Vec::new();
        for (i, cat) in categories.iter().enumerate() {
            let initial = cat.required || cat.default_enabled;
            let switch = cx.new(|cx| {
                Switch::new(("consent-category", i), "", window, cx)
                    .with_checked(initial)
                    .with_disabled(cat.required)
            });
            subscriptions.push(cx.subscribe_in(
                &switch,
                window,
                move |s: &mut Self, _, checked: &bool, _, cx| {
                    if !s.categories[i].required {
                        s.draft[i] = *checked;
                        cx.notify();
                    }
                },
            ));
            switches.push(switch);
            draft.push(initial);
        }
        Self {
            categories,
            switches,
            subscriptions,
            draft,
            saved: None,
            panel: Panel::Banner,
            reveal: Slide::snappy(1.),
            shape: Slide::snappy(0.),
            fold: Slide::snappy(1.),
            close_epoch: 0,
            focus: cx.focus_handle(),
        }
    }
    /// Reset back to the initial unconfirmed banner.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.close_epoch = self.close_epoch.wrapping_add(1);
        for (i, cat) in self.categories.iter().enumerate() {
            let initial = cat.required || cat.default_enabled;
            self.draft[i] = initial;
            self.switches[i].update(cx, |s, cx| s.set_checked(initial, cx));
        }
        self.saved = None;
        self.panel = Panel::Banner;
        self.fold.duration(1., 0.46, cx);
        self.shape.duration(0., 0.46, cx);
        self.reveal.reset(0.);
        self.reveal.duration(1., 0.22, cx);
        cx.notify();
    }
    /// Restore a host-supplied decision without emitting a change.
    pub fn restore(&mut self, choices: &[ConsentChoice], cx: &mut Context<Self>) {
        for (i, cat) in self.categories.iter().enumerate() {
            let enabled = cat.required
                || choices
                    .iter()
                    .find(|v| v.id == cat.id)
                    .is_some_and(|v| v.enabled);
            self.draft[i] = enabled;
            self.switches[i].update(cx, |s, cx| s.set_checked(enabled, cx));
        }
        self.saved = Some(self.choices());
        self.panel = Panel::Closed;
        self.fold.reset(0.);
        cx.notify();
    }
    pub fn choices(&self) -> Vec<ConsentChoice> {
        self.categories
            .iter()
            .enumerate()
            .map(|(i, c)| ConsentChoice {
                id: c.id.clone(),
                enabled: c.required || self.draft[i],
            })
            .collect()
    }
    pub fn is_decided(&self) -> bool {
        self.saved.is_some()
    }
    pub fn open_preferences(&mut self, cx: &mut Context<Self>) {
        self.close_epoch = self.close_epoch.wrapping_add(1);
        self.panel = Panel::Preferences;
        self.fold.duration(1., 0.46, cx);
        self.reveal.reset(0.);
        self.reveal.duration(1., 0.22, cx);
        self.shape.duration(1., 0.46, cx);
        cx.notify();
    }
    /// Discard unsaved changes and return to the banner or reopen button.
    pub fn close_preferences(&mut self, cx: &mut Context<Self>) {
        if self.panel != Panel::Preferences {
            return;
        }
        for (i, category) in self.categories.iter().enumerate() {
            let enabled = category.required
                || self
                    .saved
                    .as_ref()
                    .map_or(category.default_enabled, |saved| {
                        saved
                            .iter()
                            .find(|choice| choice.id == category.id)
                            .is_some_and(|choice| choice.enabled)
                    });
            self.draft[i] = enabled;
            self.switches[i].update(cx, |switch, cx| switch.set_checked(enabled, cx));
        }
        self.panel = if self.saved.is_some() {
            Panel::Closed
        } else {
            Panel::Banner
        };
        let shrink = self.saved.is_some();
        self.shape
            .duration(0., if shrink { 0.38 } else { 0.46 }, cx);
        if shrink {
            self.fold.duration(0., 0.38, cx);
        }
        cx.notify();
    }
    fn commit(&mut self, value: Option<bool>, cx: &mut Context<Self>) {
        if let Some(value) = value {
            for (i, c) in self.categories.iter().enumerate() {
                self.draft[i] = c.required || value;
                self.switches[i].update(cx, |s, cx| s.set_checked(self.draft[i], cx));
            }
        }
        let choice = self.choices();
        self.saved = Some(choice.clone());
        cx.emit(ConsentChanged(choice));
        self.close_epoch = self.close_epoch.wrapping_add(1);
        let epoch = self.close_epoch;
        if disabled_motion(cx) {
            self.fold.reset(0.);
            self.panel = Panel::Closed;
        } else {
            self.fold.duration(0., 0.38, cx);
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(380))
                    .await;
                let _ = this.update(cx, |s, cx| {
                    if s.close_epoch == epoch {
                        s.panel = Panel::Closed;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }
}
impl Render for CookieConsent {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let button = |id: &'static str, label: &'static str, primary: bool, cx: &App| {
            face(
                id,
                label,
                36.,
                radius.into(),
                if primary { Tone::Primary } else { Tone::Clear },
                cx,
            )
            .px(px(12.))
            .child(label)
        };
        let shape = self.shape.get(w, cx).clamp(0., 1.);
        let fold = self.fold.get(w, cx).clamp(0., 1.);
        let mut root = div()
            .id("cookie-consent")
            .w(px(44. + (616. - shape * 240.) * fold))
            .max_w_full()
            .track_focus(&self.focus)
            .tab_index(-1)
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape" && s.panel == Panel::Preferences {
                    s.close_preferences(cx);
                    s.focus.focus(w, cx);
                    cx.stop_propagation();
                    w.prevent_default();
                }
            }));
        if self.panel == Panel::Closed {
            let label = tr(cx, "Cookie settings", "Cookie 设置");
            return root.child(
                face("consent-reopen", label, 44., 22., Tone::Clear, cx)
                    .size(px(44.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .hover(move |s| s.bg(p.hover))
                    .child(icon(Glyph::Settings, 18., p.ink))
                    .on_click(cx.listener(|s, _, w, cx| {
                        s.open_preferences(cx);
                        s.focus.focus(w, cx);
                    })),
            );
        }
        let opacity = self.reveal.get(w, cx).clamp(0., 1.);
        let heading = if self.panel == Panel::Preferences {
            tr(cx, "Cookie preferences", "Cookie 偏好设置")
        } else {
            tr(cx, "Your privacy", "隐私设置")
        };
        let body = tr(
            cx,
            "Choose how optional cookies are used. Essential cookies are always enabled.",
            "请选择可选 Cookie 的使用方式。必要 Cookie 始终启用。",
        );
        let mut card = div()
            .flex()
            .flex_col()
            .gap(px(if self.panel == Panel::Preferences {
                12.
            } else {
                16.
            }))
            .w_full()
            .pt(px(if self.panel == Panel::Preferences {
                18.
            } else {
                10.
            }))
            .pr(px(if self.panel == Panel::Preferences {
                18.
            } else {
                10.
            }))
            .pb(px(if self.panel == Panel::Preferences {
                18.
            } else {
                10.
            }))
            .pl(px(18.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .opacity(opacity)
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(heading),
                    )
                    .when(self.panel == Panel::Preferences, |title| {
                        title.child(
                            face(
                                "consent-close",
                                tr(cx, "Close preferences", "关闭偏好设置"),
                                32.,
                                radius.into(),
                                Tone::Clear,
                                cx,
                            )
                            .size(px(32.))
                            .hover(move |s| s.bg(p.hover))
                            .child(icon(Glyph::Close, 15., p.secondary))
                            .on_click(cx.listener(|s, _, w, cx| {
                                s.close_preferences(cx);
                                s.focus.focus(w, cx);
                            })),
                        )
                    }),
            )
            .child(
                div()
                    .text_size(px(13.))
                    .line_height(px(19.))
                    .text_color(p.secondary)
                    .child(body),
            );
        if self.panel == Panel::Preferences {
            let mut list = div().flex().flex_col().max_h(px(300.)).mx(px(-6.));
            for (i, category) in self.categories.iter().enumerate() {
                let right = if category.required {
                    div()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(tr(cx, "Always on", "始终启用"))
                        .into_any_element()
                } else {
                    self.switches[i].clone().into_any_element()
                };
                list = list.child(
                    div()
                        .py(px(10.))
                        .px(px(6.))
                        .when(i > 0, |r| r.border_t_1().border_color(p.line))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(16.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(category.label.clone()),
                                )
                                .when(!category.description.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.faint)
                                            .child(category.description.clone()),
                                    )
                                }),
                        )
                        .child(right),
                );
            }
            card = card.child(list);
        }
        let save = self.panel == Panel::Preferences;
        let primary_label = if save {
            tr(cx, "Save choices", "保存选择")
        } else {
            tr(cx, "Accept all", "全部接受")
        };
        let secondary_label = tr(cx, "Reject optional", "拒绝可选项");
        let settings_label = tr(cx, "Customize", "自定义");
        let actions = div()
            .flex()
            .flex_wrap()
            .gap(px(6.))
            .when(save, |a| a.pt(px(8.)).border_t_1().border_color(p.line))
            .child(
                button("consent-primary", primary_label, true, cx).on_click(cx.listener(
                    move |s, _, w, cx| {
                        s.commit(if save { None } else { Some(true) }, cx);
                        s.focus.focus(w, cx);
                    },
                )),
            )
            .child(
                button("consent-reject", secondary_label, false, cx)
                    .border_1()
                    .border_color(p.line)
                    .hover(move |s| s.bg(p.hover))
                    .on_click(cx.listener(|s, _, w, cx| {
                        s.commit(Some(false), cx);
                        s.focus.focus(w, cx);
                    })),
            )
            .when(save, |a| {
                a.child(
                    button(
                        "consent-accept-all",
                        tr(cx, "Accept all", "全部接受"),
                        false,
                        cx,
                    )
                    .border_1()
                    .border_color(p.line)
                    .hover(move |s| s.bg(p.hover))
                    .on_click(cx.listener(|s, _, w, cx| {
                        s.commit(Some(true), cx);
                        s.focus.focus(w, cx);
                    })),
                )
            })
            .when(!save, |a| {
                a.child(
                    button("consent-settings", settings_label, false, cx)
                        .hover(move |s| s.bg(p.hover))
                        .on_click(cx.listener(|s, _, w, cx| {
                            s.open_preferences(cx);
                            s.focus.focus(w, cx);
                        })),
                )
            });
        let height_limit = if save {
            220. + shape * (self.categories.len() as f32 * 70. + 120.)
        } else {
            300.
        };
        root = root
            .max_h(px(height_limit * fold))
            .overflow_hidden()
            .child(card.child(actions));
        let _ = &self.subscriptions;
        root
    }
}
