//! WASM entry for the KUI Button gallery.
//!
//! ```bash
//! ./scripts/build-web.sh
//! python3 -m http.server -d apps/web/www 8080
//! ```

#[cfg(target_family = "wasm")]
mod web {
    use gpui::prelude::*;
    use gpui::{App, Bounds, WindowBounds, WindowHandle, WindowOptions, px, size};
    use kui::components::theme_switch::Transition;
    use kui::foundation::init;
    use kui::foundation::{AccentPalette, Theme, ThemeMode};
    use kui::primitives::control::{UiDirection, UiLanguage};
    use kui_story::{StoryBridge, StoryHost, ThemeBridge};
    use std::cell::{Cell, RefCell};
    use wasm_bindgen::prelude::wasm_bindgen;
    struct Preview {
        app: gpui::AsyncApp,
        window: WindowHandle<StoryHost>,
    }
    thread_local! { static PREVIEW: RefCell<Option<Preview>> = const { RefCell::new(None) }; }

    thread_local! { static ACCENT: Cell<AccentPalette> = Cell::default(); }

    #[wasm_bindgen]
    pub fn set_accent(light: u32, dark: u32) {
        let palette = AccentPalette {
            light: gpui::rgba(light).into(),
            dark: gpui::rgba(dark).into(),
        };
        if ACCENT.replace(palette) == palette {
            return;
        }
        PREVIEW.with_borrow(|preview| {
            if let Some(preview) = preview {
                preview.app.update(|cx| {
                    Theme::global_mut(cx).set_accent(palette);
                    cx.refresh_windows();
                });
            }
        });
    }

    #[wasm_bindgen]
    pub fn show_story(id: &str, chinese: bool) -> bool {
        PREVIEW.with_borrow(|preview| {
            let Some(preview) = preview else { return false };
            preview.app.update(|cx| {
                cx.set_global(if chinese {
                    UiLanguage::Chinese
                } else {
                    UiLanguage::English
                });
                cx.set_global(if host_direction() {
                    UiDirection::Rtl
                } else {
                    UiDirection::Ltr
                });
                preview
                    .window
                    .update(cx, |host, w, cx| host.show_story(id, w, cx))
                    .unwrap_or(false)
            })
        })
    }
    #[wasm_bindgen]
    pub fn install_input_font(bytes: Vec<u8>) -> bool {
        PREVIEW.with_borrow(|preview| {
            preview.as_ref().is_some_and(|preview| {
                preview.app.update(|cx| {
                    if cx.try_global::<kui_story::InputFontLoaded>().is_some() {
                        return true;
                    }
                    if cx.text_system().add_fonts(vec![bytes.into()]).is_err() {
                        return false;
                    }
                    cx.set_global(kui_story::InputFontLoaded);
                    cx.refresh_windows();
                    true
                })
            })
        })
    }
    fn appearance(dark: bool, reduced: bool, cx: &mut App) {
        let mode = if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        let theme = Theme::global_mut(cx);
        if theme.mode == mode && theme.reduced_motion == reduced {
            return;
        }
        theme.set_mode(mode);
        theme.reduced_motion = reduced;
        cx.refresh_windows();
    }
    #[wasm_bindgen]
    pub fn set_icon_intent(dark: bool) {
        use kui::components::theme_switch::ThemeIconIntent;
        PREVIEW.with_borrow(|preview| {
            if let Some(preview) = preview {
                preview.app.update(|cx| {
                    let mode = if dark {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    };
                    if cx
                        .try_global::<ThemeIconIntent>()
                        .is_none_or(|i| i.0 != mode)
                    {
                        cx.set_global(ThemeIconIntent(mode));
                        cx.refresh_windows();
                    }
                });
            }
        });
    }
    /// Synchronize platform geometry and request a presented host frame.
    #[wasm_bindgen]
    pub fn request_snapshot() {
        PREVIEW.with_borrow(|preview| {
            if let Some(preview) = preview {
                preview.app.update(|cx| {
                    let _ = preview.window.update(cx, |_, w, cx| w.bounds_changed(cx));
                });
            }
        });
    }

    #[wasm_bindgen]
    pub fn set_preferences(dark: bool, reduced: bool) {
        PREVIEW.with_borrow(|preview| {
            if let Some(preview) = preview {
                preview.app.update(|cx| appearance(dark, reduced, cx))
            }
        })
    }

    // The host owns browser integration; neither the story nor installed controls depend on JS.
    #[wasm_bindgen(inline_js = "
        export function compactHost(){try{return (window.frameElement?.ownerDocument.defaultView ?? window).matchMedia('(max-width:639px)').matches}catch{return innerWidth<640}}
        export function storedAppearance(){try{return ({light:0,dark:1})[localStorage.getItem('theme')] ?? 2}catch{return 2}}
        export function accountAppearance(mode){parent.postMessage({type:'kui:account-appearance',mode},location.origin)}
        export function previewStarted() { queueMicrotask(() => window.kuiPreviewStarted?.()); }
        export function reportThemeTarget(x,y,w,h,radius) { window.kuiThemeTarget?.(x,y,w,h,radius); }
        export function horizontalRegion(x,y,w,h,left,right) { window.kuiHorizontalScroll?.(x,y,w,h,left,right); }
        export function editorRegion(x,y,w,h,up,down) { window.kuiEditorScroll?.(x,y,w,h,up,down); }
        export function downloadCsv(name, csv) {
            const url = URL.createObjectURL(new Blob([csv], {type: 'text/csv;charset=utf-8'}));
            const link = document.createElement('a');
            link.href = url;
            link.download = name;
            document.body.append(link);
            link.click();
            link.remove();
            setTimeout(() => URL.revokeObjectURL(url), 30000);
        }
        export function reportFrame(height, dark) {
            window.kuiFramePainted?.(Math.ceil(height), dark);
        }
        export function previewVisible() { return !document.hidden && self.kuiStoryInViewport !== false && self.kuiStoryPageActive !== false; }
        export function leavePreview(back) {
            parent.postMessage({type:'kui:story-leave',back},location.origin);
        }
        export function buttonTheme(dark,mode,x,y,w,h) {
            if (window.kuiPointerActivated?.()) return;
            if (window !== parent) parent.postMessage({type:'kui:family-theme',dark,mode,x,y,w,h},location.origin);
        }
        // The embedding document owns writing direction; a preview never sets it.
        export function hostDirection() {
            const doc = window.frameElement?.ownerDocument ?? document;
            return doc.documentElement.dir === 'rtl' || doc.defaultView?.getComputedStyle(doc.documentElement).direction === 'rtl';
        }

    ")]
    extern "C" {
        #[wasm_bindgen(js_name=compactHost)]
        fn compact_host() -> bool;
        #[wasm_bindgen(js_name=storedAppearance)]
        fn stored_appearance() -> u8;
        #[wasm_bindgen(js_name=accountAppearance)]
        fn account_appearance(mode: &str);
        #[wasm_bindgen(js_name=previewStarted)]
        fn preview_started();
        #[wasm_bindgen(js_name=reportThemeTarget)]
        fn report_theme_target(x: f32, y: f32, w: f32, h: f32, radius: f32);
        #[wasm_bindgen(js_name=horizontalRegion)]
        fn horizontal_region(x: f32, y: f32, w: f32, h: f32, left: bool, right: bool);
        #[wasm_bindgen(js_name=editorRegion)]
        fn editor_region(x: f32, y: f32, w: f32, h: f32, up: bool, down: bool);
        #[wasm_bindgen(js_name=downloadCsv)]
        fn download_csv(name: &str, csv: &str);
        #[wasm_bindgen(js_name=reportFrame)]
        fn report_frame(height: f32, dark: bool);
        #[wasm_bindgen(js_name=buttonTheme)]
        fn button_theme(dark: bool, mode: &str, x: f32, y: f32, w: f32, h: f32);

        #[wasm_bindgen(js_name=hostDirection)]
        fn host_direction() -> bool;

        #[wasm_bindgen(js_name=previewVisible)]
        fn preview_visible() -> bool;
        #[wasm_bindgen(js_name=leavePreview)]
        fn leave_preview(back: bool);
    }

    fn run_gallery(id: String, dark: bool, reduced: bool, chinese: bool, embedded: bool) {
        gpui_platform::application().run(move |cx: &mut App| {
            // The web text system starts empty and cannot enumerate OS fonts.
            cx.text_system()
                .add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!(
                    "../www/fonts/NotoSans-Regular.ttf"
                ))])
                .expect("load bundled gallery font");
            cx.text_system()
                .add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!(
                    "../www/fonts/KUIStorySans.ttf"
                ))])
                .expect("load bilingual preview font");
            cx.set_global(if chinese {
                UiLanguage::Chinese
            } else {
                UiLanguage::English
            });
            cx.set_global(if host_direction() {
                UiDirection::Rtl
            } else {
                UiDirection::Ltr
            });
            cx.set_global(kui::components::data_grid::CsvDownload(download_csv));
            if embedded {
                cx.set_global(kui::primitives::editor::EditorScrollRegion(
                    |b, up, down| {
                        editor_region(
                            b.left().into(),
                            b.top().into(),
                            b.size.width.into(),
                            b.size.height.into(),
                            up,
                            down,
                        )
                    },
                ));
                cx.set_global(kui::primitives::menu::MenuScrollRegion(|b, up, down| {
                    editor_region(
                        b.left().into(),
                        b.top().into(),
                        b.size.width.into(),
                        b.size.height.into(),
                        up,
                        down,
                    )
                }));
                cx.set_global(kui::components::scroll_area::HorizontalScrollRegion(
                    |b, left, right| {
                        horizontal_region(
                            b.left().into(),
                            b.top().into(),
                            b.size.width.into(),
                            b.size.height.into(),
                            left,
                            right,
                        )
                    },
                ));
                cx.set_global(kui_story::AccountHost {
                    compact: || compact_host(),
                    current: || {
                        use kui::components::user_menu::Appearance::*;
                        match stored_appearance() {
                            0 => Light,
                            1 => Dark,
                            _ => System,
                        }
                    },
                    preference: |mode| {
                        use kui::components::user_menu::Appearance::*;
                        account_appearance(match mode {
                            Light => "light",
                            Dark => "dark",
                            System => "system",
                        })
                    },
                });
                cx.set_global(ThemeBridge(|r| {
                    button_theme(
                        r.next == ThemeMode::Dark,
                        match r.mode {
                            Transition::Reveal => "reveal",
                            Transition::Eclipse => "eclipse",
                            Transition::Split => "split",
                            Transition::Rise => "rise",
                        },
                        r.trigger.origin.x.into(),
                        r.trigger.origin.y.into(),
                        r.trigger.size.width.into(),
                        r.trigger.size.height.into(),
                    )
                }));
                cx.set_global(StoryBridge {
                    visible: preview_visible,
                    frame: |height, mode| report_frame(height, mode == ThemeMode::Dark),
                    leave: leave_preview,
                    theme_target: |b, radius| {
                        report_theme_target(
                            b.origin.x.into(),
                            b.origin.y.into(),
                            b.size.width.into(),
                            b.size.height.into(),
                            radius.into(),
                        )
                    },
                });
            }
            init(cx);
            Theme::global_mut(cx).set_accent(ACCENT.get());
            appearance(dark, reduced, cx);
            let bounds = Bounds::centered(None, size(px(960.), px(540.)), cx);
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        // An embedded preview can be visible without DOM focus.
                        // Let the browser schedule frames at display cadence.
                        inactive_frame_interval: None,
                        ..Default::default()
                    },
                    move |w, cx| cx.new(|cx| StoryHost::new(Some(&id), w, cx)),
                )
                .expect("open KUI web gallery");
            PREVIEW.set(Some(Preview {
                app: cx.to_async(),
                window,
            }));
            cx.activate(true);
            preview_started();
        });
    }

    #[wasm_bindgen]
    pub fn start(id: String, dark: bool, reduced: bool, chinese: bool, embedded: bool) {
        gpui_platform::web_init();
        run_gallery(id, dark, reduced, chinese, embedded);
    }
}

#[cfg(not(target_family = "wasm"))]
/// Host-target placeholder. Build with `--target wasm32-unknown-unknown`.
pub fn wasm_only() {
    let _ = kui_story::ButtonGallery::new();
}
