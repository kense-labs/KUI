//! Photo actions with local state and host-owned export.
use gpui::{prelude::*, *};
use kui::{
    components::orbit_menu::{
        OrbitAction, OrbitIcon, OrbitInvoked, OrbitMenu, OrbitOpenChanged, OrbitTone,
    },
    foundation::{Theme, ThemeMode, surface_radius},
    primitives::{
        control::UiLanguage,
        family::{Paint, Slide, Tone, face},
        glyph::{Glyph, icon},
        presentation::{column, row},
    },
};
use std::sync::Arc;

pub const IDS: &[&str] = &["orbit-menu"];
const PHOTO: &[u8] = include_bytes!("../../assets/navigation/as17-147-22465.jpg");
const FILENAME: &str = "apollo-17-lunar-ridges.jpg";

/// Browser integration for the bundled photo example.
pub struct OrbitPhotoHost {
    pub download: fn(&str, &[u8]),
    pub gesture_region: fn(Bounds<Pixels>),
}
impl Global for OrbitPhotoHost {}

pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Example::new(w, cx)).into())
}

#[derive(Default)]
struct PhotoState {
    liked: bool,
    album: bool,
    shared: bool,
    deleted: bool,
}

fn actions(lang: UiLanguage, state: &PhotoState) -> Vec<OrbitAction> {
    let t = |en, zh| lang.text(en, zh);
    vec![
        OrbitAction::new(
            "like",
            if state.liked {
                t("Unlike", "取消点赞")
            } else {
                t("Like", "点赞")
            },
            Glyph::Heart,
        )
        .with_done(if state.liked {
            t("Like removed", "已取消点赞")
        } else {
            t("Liked", "已点赞")
        }),
        OrbitAction::new(
            "album",
            if state.album {
                t("Remove from album", "从相册移除")
            } else {
                t("Add to album", "添加到相册")
            },
            OrbitIcon::Album,
        )
        .with_done(if state.album {
            t("Removed from album", "已从相册移除")
        } else {
            t("Added to album", "已添加到相册")
        }),
        OrbitAction::new(
            "share",
            t("Share with Ryan", "与 Ryan 分享"),
            OrbitIcon::Share,
        )
        .with_done(t("Shared", "已分享")),
        OrbitAction::new("download", t("Download", "下载"), Glyph::Download)
            .with_done(t("Download ready", "下载已就绪")),
        OrbitAction::new("delete", t("Delete", "删除"), Glyph::Trash)
            .with_done(t("Deleted", "已删除"))
            .with_tone(OrbitTone::Danger),
    ]
}

struct Example {
    menu: Entity<OrbitMenu>,
    state: PhotoState,
    image: ImageSource,
    menu_progress: Slide,
    notice: SharedString,
    changed: bool,
    reset_focus: FocusHandle,
    _subscriptions: [Subscription; 2],
}

impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let state = PhotoState::default();
        let menu = cx.new(|cx| {
            OrbitMenu::new(actions(lang, &state), w, cx)
                .with_id("photo-actions")
                .with_label(lang.text("Photo actions", "照片操作"))
        });
        let subscriptions = [
            cx.subscribe(&menu, |s, _, e: &OrbitInvoked, cx| s.invoke(&e.0, cx)),
            cx.subscribe(&menu, |s, _, e: &OrbitOpenChanged, cx| {
                s.menu_progress.duration(f32::from(e.0), 0.24, cx);
                cx.notify();
            }),
        ];
        Self {
            menu,
            state,
            image: Arc::new(Image::from_bytes(ImageFormat::Jpeg, PHOTO.to_vec())).into(),
            menu_progress: Slide::new(0.),
            notice: lang
                .text(
                    "Press and drag to pick an action. A tap keeps the arc open.",
                    "按住并拖向操作，或点按以保留菜单。",
                )
                .into(),
            changed: false,
            reset_focus: cx.focus_handle().tab_stop(true),
            _subscriptions: subscriptions,
        }
    }

    fn invoke(&mut self, action: &str, cx: &mut Context<Self>) {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        self.notice = match action {
            "like" => {
                self.state.liked = !self.state.liked;
                if self.state.liked {
                    t("Liked the photo, 129 likes now", "已点赞，现在有 129 个赞")
                } else {
                    t("Like removed, 128 likes now", "已取消点赞，现在有 128 个赞")
                }
            }
            "album" => {
                self.state.album = !self.state.album;
                if self.state.album {
                    t("Added to the album", "已添加到相册")
                } else {
                    t("Removed from the album", "已从相册移除")
                }
            }
            "share" => {
                self.state.shared = true;
                t("Share recorded in the example", "已在演示中记录分享")
            }
            "download" => {
                self.download(cx);
                t("Photo export requested", "已请求下载照片")
            }
            "delete" => {
                self.state.deleted = true;
                t(
                    "Photo removed from the example. Reset to restore it.",
                    "照片已从演示中移除，重置即可恢复。",
                )
            }
            _ => return,
        }
        .into();
        self.changed = true;
        self.menu.update(cx, |menu, cx| {
            menu.set_actions(actions(lang, &self.state), cx);
            menu.set_disabled(self.state.deleted, cx);
        });
        cx.notify();
    }

    fn download(&mut self, cx: &mut Context<Self>) {
        if let Some(host) = cx.try_global::<OrbitPhotoHost>() {
            (host.download)(FILENAME, PHOTO);
            return;
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let directory = std::env::current_dir().unwrap_or_else(|_| ".".into());
            let chosen = cx.prompt_for_new_path(&directory, Some(FILENAME));
            cx.spawn(async move |view, cx| {
                let failure = match chosen.await {
                    Ok(Ok(Some(path))) => cx
                        .background_executor()
                        .spawn(async move { std::fs::write(path, PHOTO) })
                        .await
                        .is_err(),
                    Ok(Ok(None)) => return,
                    _ => true,
                };
                let _ = view.update(cx, |s, cx| {
                    let lang = UiLanguage::current(cx);
                    s.notice = if failure {
                        lang.text("Could not save the photo", "无法保存照片")
                    } else {
                        lang.text("Photo saved", "照片已保存")
                    }
                    .into();
                    cx.notify();
                });
            })
            .detach();
        }
    }

    fn reset(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let lang = UiLanguage::current(cx);
        self.state = PhotoState::default();
        self.changed = false;
        self.notice = lang.text("Photo restored", "照片已恢复").into();
        self.menu.update(cx, |menu, cx| {
            menu.reset(cx);
            menu.set_actions(actions(lang, &self.state), cx);
            menu.set_disabled(false, cx);
            menu.focus_trigger(w, cx);
        });
        cx.notify();
    }
}

impl Render for Example {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let p = Paint::get(cx);
        let radius = surface_radius(cx);
        let width = (f32::from(w.viewport_size().width) - 12.).clamp(1., 522.);
        let narrow = width < 400.;
        let hero_height = if narrow { 170. } else { 200. };
        let base = (width - 32.).clamp(1., 360.);
        let open = self.menu_progress.get(w, cx).clamp(0., 1.);
        let scale = 1. - open * 0.03;
        let stage_height = if narrow { 420. } else { 440. };
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let card_bg = if dark { rgb(0x191919) } else { rgb(0xffffff) };
        let stage_bg = if dark { rgb(0x242424) } else { rgb(0xf8f8f8) };
        let mut stage = column()
            .id("orbit-photo-stage")
            .debug_selector(|| "orbit-photo-stage".into())
            .w_full()
            .h(px(stage_height))
            .relative()
            .items_center()
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(stage_bg)
            .overflow_hidden();
        if self.state.deleted {
            stage = stage.child(
                column()
                    .absolute()
                    .inset_0()
                    .items_center()
                    .justify_center()
                    .gap(px(10.))
                    .text_color(p.secondary)
                    .child(icon(Glyph::Trash, 28., p.faint))
                    .child(t("Photo removed", "照片已移除")),
            );
        } else {
            let likes = if self.state.liked { 129 } else { 128 };
            let count = if lang == UiLanguage::Chinese {
                format!("{likes} 个赞")
            } else {
                format!("{likes} likes")
            };
            let note: SharedString = if self.state.album {
                format!("{count} · {}", t("In the album", "已在相册中")).into()
            } else if self.state.shared {
                format!("{count} · {}", t("Shared in the example", "演示已分享")).into()
            } else if !self.state.liked {
                format!("{count}, {}", t("not in an album", "未添加到相册")).into()
            } else {
                count.into()
            };
            stage = stage.child(
                column()
                    .id("orbit-photo-card")
                    .debug_selector(|| "orbit-photo-card".into())
                    .aria_label(t("Lunar ridges", "月球山脊"))
                    .w(px(base * scale))
                    .h(px((hero_height + 105.) * scale))
                    .max_w_full()
                    .mt(px(
                        if narrow { 15. } else { 24. } + (1. - scale) * hero_height / 2.
                    ))
                    .rounded(radius)
                    .border_1()
                    .border_color(p.line)
                    .shadow(vec![BoxShadow {
                        color: black().opacity(if dark { 0.22 } else { 0.065 }),
                        offset: point(px(0.), px(if dark { 8. } else { 6. })),
                        blur_radius: px(if dark { 22. } else { 18. }),
                        spread_radius: px(0.),
                        inset: false,
                    }])
                    .overflow_hidden()
                    .bg(card_bg)
                    .opacity(1. - open * 0.45)
                    .child(
                        img(self.image.clone())
                            .w_full()
                            .h(px(hero_height * scale))
                            .flex_none()
                            .rounded_t(radius - px(1.))
                            .object_fit(ObjectFit::Cover),
                    )
                    .child(
                        column()
                            .px(px(16. * scale))
                            .py(px(13. * scale))
                            .gap(px(13. * scale))
                            .child(
                                row()
                                    .gap(px(10. * scale))
                                    .child(
                                        div().size(px(28. * scale)).flex_none().rounded_full().bg(
                                            linear_gradient(
                                                130.,
                                                linear_color_stop(rgb(0xc6be84), 0.),
                                                linear_color_stop(rgb(0x9873cc), 1.),
                                            ),
                                        ),
                                    )
                                    .child(
                                        column()
                                            .min_w_0()
                                            .gap(px(1.))
                                            .child(
                                                div()
                                                    .truncate()
                                                    .text_size(px(16. * scale))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(t("Lunar ridges", "月球山脊")),
                                            )
                                            .child(
                                                div()
                                                    .truncate()
                                                    .text_color(p.secondary)
                                                    .text_size(px(12. * scale))
                                                    .child(t(
                                                        "NASA, Apollo 17 archive",
                                                        "NASA · 阿波罗 17 号档案",
                                                    )),
                                            ),
                                    ),
                            )
                            .child(
                                row()
                                    .text_size(px(12. * scale))
                                    .gap(px(5.))
                                    .text_color(p.faint)
                                    .when(self.state.liked, |line| {
                                        line.child(icon(Glyph::Heart, 12. * scale, p.faint))
                                    })
                                    .child(div().min_w_0().truncate().child(note)),
                            ),
                    ),
            );
        }
        let inactive = self.state.deleted;
        let orbit = row()
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(30.))
            .justify_center()
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(host) = cx.try_global::<OrbitPhotoHost>() {
                    let bounds = if inactive {
                        Bounds::default()
                    } else {
                        bounds.first().copied().unwrap_or_default()
                    };
                    (host.gesture_region)(bounds);
                }
            })
            .child(self.menu.clone());
        stage = stage.child(orbit);
        let reset = face(
            "orbit-reset",
            t("Reset photo", "重置照片"),
            36.,
            18.,
            Tone::Surface,
            cx,
        )
        .track_focus(&self.reset_focus)
        .tab_stop(self.changed)
        .px(px(12.))
        .border_1()
        .border_color(p.line)
        .opacity(if self.changed { 1. } else { 0.38 })
        .child(icon(Glyph::Undo, 16., p.secondary))
        .child(t("Reset photo", "重置照片"))
        .when(self.changed, |b| {
            b.on_click(cx.listener(|s, _, w, cx| s.reset(w, cx)))
        });
        column()
            .w_full()
            .max_w(px(534.))
            .px(px(6.))
            .gap(px(14.))
            .items_center()
            .child(stage)
            .child(row().w_full().child(reset))
            .child(
                div()
                    .id("orbit-photo-status")
                    .role(Role::Status)
                    .aria_label(self.notice.clone())
                    .absolute()
                    .size(px(0.)),
            )
    }
}
