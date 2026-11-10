//! A bilingual ticket with a replaceable paper stub.
use gpui::{prelude::*, *};
use kui::{
    components::ticket_tear::{TicketTear, TicketTornChanged},
    foundation::surface_radius,
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
        glyph::{Glyph, icon},
    },
};

pub const IDS: &[&str] = &["ticket-tear"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

struct Example {
    ticket: Entity<TicketTear>,
    language: UiLanguage,
    _subscription: Subscription,
}

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let ticket = cx.new(|cx| {
            TicketTear::new(
                move |w, cx| body(language, w, cx).into_any_element(),
                move |_, _| stub(language).into_any_element(),
                cx,
            )
            .with_label(language.text("Tear off stub to check in", "撕下票根以签到"))
            .with_stub_width(px(96.))
        });
        let subscription = cx.subscribe(&ticket, |_, _, _: &TicketTornChanged, cx| cx.notify());
        Self {
            ticket,
            language,
            _subscription: subscription,
        }
    }
}

fn body(language: UiLanguage, w: &Window, cx: &App) -> impl IntoElement {
    let p = Paint::get(cx);
    let narrow = w.viewport_size().width < px(325.);
    let inset = px(if narrow { 16. } else { 24. });
    let rule = p.line.blend(p.secondary.opacity(0.22));
    let t = |en, zh| language.text(en, zh);
    let info = |label: &'static str, value: &'static str| {
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .text_color(p.secondary)
                    .child(label),
            )
            .child(div().text_size(px(14.)).line_height(px(21.)).child(value))
    };
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .px(inset)
        .py(px(20.))
        .text_color(p.ink)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(4.))
                .h(px(22.))
                .child(
                    div()
                        .w(px(22.))
                        .flex_none()
                        .text_size(px(19.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(p.secondary)
                        .child("K"),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .truncate()
                        .text_color(p.secondary)
                        .child(t("Oct 14, 2026", "2026年10月14日")),
                ),
        )
        .child(
            div()
                .mt(px(18.))
                .text_size(px(if narrow { 24. } else { 36. }))
                .line_height(px(39.6))
                .font_weight(FontWeight::MEDIUM)
                .truncate()
                .child("Kense ’26"),
        )
        .child(
            div()
                .mt(px(2.))
                .text_size(px(14.))
                .line_height(px(19.6))
                .truncate()
                .text_color(p.secondary)
                .child(t("Teatro Tivoli, Lisbon", "里斯本 · 蒂沃利剧院")),
        )
        .child(
            div()
                .mt(px(18.))
                .relative()
                .pt(px(13.))
                .flex()
                .gap(px(if narrow { 14. } else { 24. }))
                .child(
                    canvas(
                        |_, _, _| (),
                        move |bounds, _, w, _| {
                            let start = bounds.origin + point(px(0.), px(0.5));
                            let mut path =
                                PathBuilder::stroke(px(1.)).dash_array(&[px(2.), px(2.)]);
                            path.move_to(start);
                            path.line_to(start + point(bounds.size.width, px(0.)));
                            if let Ok(path) = path.build() {
                                w.paint_path(path, rule);
                            }
                        },
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(1.)),
                )
                .child(info(t("Guest", "来宾"), t("Avery Chen", "陈清禾")))
                .child(info(t("Doors", "入场"), "18:30")),
        )
}

fn stub(language: UiLanguage) -> impl IntoElement {
    let t = |en, zh| language.text(en, zh);
    div()
        .size_full()
        .pl(px(12.))
        .pr(px(8.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.8))
                .opacity(0.78)
                .mb(px(6.))
                .child(t("Admit one", "限一人入场")),
        )
        .child(
            div()
                .text_size(px(18.))
                .line_height(px(19.8))
                .font_weight(FontWeight::MEDIUM)
                .child(t("Row F", "F 排")),
        )
        .child(
            div()
                .text_size(px(18.))
                .line_height(px(19.8))
                .font_weight(FontWeight::MEDIUM)
                .child(t("Seat 12", "12 号")),
        )
}

impl Render for Example {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = f32::from(surface_radius(cx));
        let torn = self.ticket.read(cx).is_torn();
        let label = self.language.text("New ticket", "换一张票");
        let width = f32::from(window.viewport_size().width);
        let inset = 16. + 24. * ((width - 320.) / 70.).clamp(0., 1.);
        div()
            .w_full()
            .px(px(inset))
            .pb(px(72.))
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(400.))
                    .flex()
                    .flex_col()
                    .gap(px(24.))
                    .child(self.ticket.clone())
                    .child(
                        div().flex().justify_end().child(
                            face("fresh-ticket", label, 36., radius, Tone::Surface, cx)
                                .tab_stop(torn)
                                .gap(px(8.))
                                .px(px(12.))
                                .text_size(px(14.))
                                .opacity(if torn { 1. } else { 0.45 })
                                .child(icon(Glyph::Undo, 16., p.secondary))
                                .child(label)
                                .when(torn, |button| {
                                    button.on_click(cx.listener(|s, _, _, cx| {
                                        s.ticket
                                            .update(cx, |ticket, cx| ticket.set_torn(false, cx));
                                        cx.notify();
                                    }))
                                }),
                        ),
                    ),
            )
    }
}
