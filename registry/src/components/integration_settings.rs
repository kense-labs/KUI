//! Connected SaaS integration settings card with API key masking, URL validation, and dirty bar.
//!
//! Combines a brand header (`pro_app_tile` + `pro_switch`), foldable configuration sections
//! (`pro_fold`), a masked/revealable API key [`Editor`] with copy and regeneration, live domain
//! validation, a searchable timezone popover, and an animated `Discard` / `Save changes` footer.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_check_draw, pro_eid, pro_empty_state,
        pro_fold, pro_icon_btn, pro_popover_presence, pro_presence, pro_skeleton_bar,
        pro_swap_label, pro_switch, pro_tactile, pro_toast_banner, tr,
    },
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// Editable values managed inside [`IntegrationSettings`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrationValues {
    /// Whether live synchronization for this integration is enabled.
    pub enabled: bool,
    /// Secret or publishable API key for the connected service.
    pub api_key: SharedString,
    /// Primary store or workspace domain (without scheme, e.g. `"lumen-studio.com"`).
    pub site_url: SharedString,
    /// Selected reporting/sync time zone label.
    pub time_zone: SharedString,
}

impl Default for IntegrationValues {
    fn default() -> Self {
        Self {
            enabled: true,
            api_key: "sk_live_9f8a4c2d1e7b6035a9c4e210".into(),
            site_url: "lumen-studio.com".into(),
            time_zone: "(GMT +01:00) Berlin".into(),
        }
    }
}

/// Validate a bare host/domain or path (`"lumen-studio.com"`, `"api.kense.dev/v1"`).
pub fn is_valid_site_address(raw: &str) -> bool {
    let trimmed = raw
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    if trimmed.is_empty() || trimmed.contains(' ') {
        return false;
    }
    let host = trimmed.split('/').next().unwrap_or("");
    if host.len() < 4 || host.starts_with('.') || host.ends_with('.') || host.contains("..") {
        return false;
    }
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let tld = labels.last().copied().unwrap_or("");
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    labels.iter().all(|lbl| {
        !lbl.is_empty()
            && !lbl.starts_with('-')
            && !lbl.ends_with('-')
            && lbl.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

/// Typed events emitted by [`IntegrationSettings`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrationSettingsEvent {
    /// Master integration switch toggled.
    Toggled(bool),
    /// User generated a new API key.
    KeyGenerated(SharedString),
    /// User copied the API key to the clipboard.
    KeyCopied(SharedString),
    /// User confirmed and saved dirty changes.
    Saved(IntegrationValues),
    /// User discarded unsaved edits back to the last saved state.
    Discarded(IntegrationValues),
}

/// Connected SaaS service settings card with live validation and dirty-state confirmation bar.
pub struct IntegrationSettings {
    id: ElementId,
    provider: SharedString,
    name: SharedString,
    name_zh: SharedString,
    count: Option<u32>,
    meta: SharedString,
    meta_zh: SharedString,
    site_hint: SharedString,
    site_hint_zh: SharedString,
    time_zones: Vec<SharedString>,
    saved: IntegrationValues,
    draft: IntegrationValues,
    key_revealed: bool,
    key_just_generated: bool,
    copied_at: Option<Instant>,
    regenerated_at: Option<Instant>,
    gen_seq: u32,
    tz_open: bool,
    tz_query: String,
    key_editor: Option<Entity<Editor>>,
    url_editor: Option<Entity<Editor>>,
    tz_search_editor: Option<Entity<Editor>>,
    loading: bool,
    empty: bool,
    width_cell: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    focus: FocusHandle,
}

impl EventEmitter<IntegrationSettingsEvent> for IntegrationSettings {}

impl IntegrationSettings {
    /// Construct an [`IntegrationSettings`] card with default connected SaaS values.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let defaults = IntegrationValues::default();
        Self {
            id: id.into(),
            provider: "stripe".into(),
            name: "Stripe store".into(),
            name_zh: "Stripe 商业同步".into(),
            count: Some(128),
            meta: "Connected on March 4, 2026 at 09:12".into(),
            meta_zh: "已于 2026年3月4日 09:12 连接".into(),
            site_hint: "Your store's primary domain from Settings › Domains".into(),
            site_hint_zh: "请填写设置 › 域名管理中的主站点域名".into(),
            time_zones: default_time_zones(),
            saved: defaults.clone(),
            draft: defaults,
            key_revealed: false,
            key_just_generated: false,
            copied_at: None,
            regenerated_at: None,
            gen_seq: 1,
            tz_open: false,
            tz_query: String::new(),
            key_editor: None,
            url_editor: None,
            tz_search_editor: None,
            loading: false,
            empty: false,
            width_cell: Rc::new(Cell::new(560.)),
            toast: None,
            focus: cx.focus_handle(),
        }
    }

    /// Set the provider icon identifier rendered via [`pro_app_tile`] (e.g. `"stripe"`, `"slack"`, `"github"`, `"hubspot"`).
    pub fn with_provider(mut self, provider: impl Into<SharedString>) -> Self {
        self.provider = provider.into();
        self
    }

    /// Set the connected integration display name.
    pub fn with_name(mut self, name: impl Into<SharedString>) -> Self {
        let n = name.into();
        self.name_zh = n.clone();
        self.name = n;
        self
    }

    /// Set or clear the numeric item badge beside the integration name.
    pub fn with_count(mut self, count: Option<u32>) -> Self {
        self.count = count;
        self
    }

    /// Customize the connection status subtitle shown when enabled.
    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        let m = meta.into();
        self.meta_zh = m.clone();
        self.meta = m;
        self
    }

    /// Customize the helper text beneath the site address input.
    pub fn with_site_hint(mut self, hint: impl Into<SharedString>) -> Self {
        let h = hint.into();
        self.site_hint_zh = h.clone();
        self.site_hint = h;
        self
    }

    /// Configure initial saved & draft [`IntegrationValues`].
    pub fn with_values(mut self, values: IntegrationValues) -> Self {
        self.saved = values.clone();
        self.draft = values;
        self
    }

    /// Override the selectable timezone list.
    pub fn with_time_zones(
        mut self,
        zones: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.time_zones = zones.into_iter().map(Into::into).collect();
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return last committed [`IntegrationValues`].
    pub fn saved_values(&self) -> &IntegrationValues {
        &self.saved
    }

    /// Return current editable [`IntegrationValues`].
    pub fn draft_values(&self) -> &IntegrationValues {
        &self.draft
    }

    /// Return whether there are unsaved modifications.
    pub fn is_dirty(&self) -> bool {
        self.draft != self.saved
    }

    /// Return whether the current draft site URL is syntactically valid.
    pub fn is_url_valid(&self) -> bool {
        is_valid_site_address(&self.draft.site_url)
    }

    /// Return whether changes can currently be saved (`is_dirty()` and valid URL when enabled).
    pub fn can_save(&self) -> bool {
        self.is_dirty() && (!self.draft.enabled || self.is_url_valid())
    }

    /// Toggle the master enabled switch.
    pub fn toggle_enabled(&mut self, cx: &mut Context<Self>) {
        self.draft.enabled = !self.draft.enabled;
        if !self.draft.enabled {
            self.tz_open = false;
        }
        cx.emit(IntegrationSettingsEvent::Toggled(self.draft.enabled));
        cx.notify();
    }

    /// Toggle API key mask/reveal mode.
    pub fn toggle_key_reveal(&mut self, cx: &mut Context<Self>) {
        self.key_revealed = !self.key_revealed;
        let rev = self.key_revealed;
        if let Some(ed) = &self.key_editor {
            ed.update(cx, |e, cx| {
                e.revealed = rev;
                cx.notify();
            });
        }
        cx.notify();
    }

    /// Generate a fresh deterministic-prefix API key and reveal it for copying.
    pub fn generate_new_key(&mut self, w: Option<&mut Window>, cx: &mut Context<Self>) {
        self.gen_seq = self.gen_seq.wrapping_add(1);
        let stamp = (u64::from(self.gen_seq)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let new_key =
            SharedString::from(format!("sk_live_{stamp:016x}{:08x}", self.gen_seq * 7919));
        self.draft.api_key = new_key.clone();
        self.key_revealed = true;
        self.key_just_generated = true;
        self.regenerated_at = Some(Instant::now());
        if let Some(ed) = &self.key_editor {
            ed.update(cx, |e, cx| {
                e.revealed = true;
                if let Some(win) = w {
                    e.set_value(new_key.to_string(), win, cx);
                }
                cx.notify();
            });
        }
        cx.emit(IntegrationSettingsEvent::KeyGenerated(new_key));
        cx.notify();
    }

    /// Copy the current draft API key to the system clipboard.
    pub fn copy_api_key(&mut self, cx: &mut Context<Self>) {
        let key = self.draft.api_key.clone();
        cx.write_to_clipboard(ClipboardItem::new_string(key.to_string()));
        self.copied_at = Some(Instant::now());
        self.toast = Some((
            SharedString::from(tr(cx, "API key copied to clipboard", "API 密钥已复制")),
            Instant::now(),
        ));
        cx.emit(IntegrationSettingsEvent::KeyCopied(key));
        cx.notify();
    }

    /// Programmatically update the draft site URL.
    pub fn set_site_url(
        &mut self,
        url: impl Into<SharedString>,
        w: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        let clean = url
            .into()
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .to_string();
        self.draft.site_url = SharedString::from(clean.clone());
        if let Some(win) = w
            && let Some(ed) = &self.url_editor
        {
            ed.update(cx, |e, cx| e.set_value(clean, win, cx));
        }
        cx.notify();
    }

    /// Select a time zone and close the timezone popover.
    pub fn select_time_zone(&mut self, zone: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.draft.time_zone = zone.into();
        self.tz_open = false;
        cx.notify();
    }

    /// Discard unsaved edits and revert to [`Self::saved_values`].
    pub fn discard_changes(&mut self, w: Option<&mut Window>, cx: &mut Context<Self>) -> bool {
        if !self.is_dirty() {
            return false;
        }
        self.draft = self.saved.clone();
        self.key_just_generated = false;
        self.tz_open = false;
        if let Some(win) = w {
            let saved_key = self.saved.api_key.to_string();
            let saved_url = self.saved.site_url.to_string();
            if let Some(ke) = &self.key_editor {
                ke.update(cx, |e, cx| e.set_value(saved_key, win, cx));
            }
            if let Some(ue) = &self.url_editor {
                ue.update(cx, |e, cx| e.set_value(saved_url, win, cx));
            }
        }
        cx.emit(IntegrationSettingsEvent::Discarded(self.saved.clone()));
        cx.notify();
        true
    }

    /// Validate and commit unsaved changes.
    pub fn save_changes(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.can_save() {
            return false;
        }
        self.saved = self.draft.clone();
        self.key_just_generated = false;
        self.key_revealed = false;
        if let Some(ke) = &self.key_editor {
            ke.update(cx, |e, cx| {
                e.revealed = false;
                cx.notify();
            });
        }
        self.toast = Some((
            SharedString::from(tr(cx, "Integration settings saved", "集成设置已保存")),
            Instant::now(),
        ));
        cx.emit(IntegrationSettingsEvent::Saved(self.saved.clone()));
        cx.notify();
        true
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.key_editor.is_none() {
            let init_key = self.draft.api_key.to_string();
            let revealed = self.key_revealed;
            let ke = cx.new(|cx| {
                let mut e = Editor::new("API key", InputMode::Password, w, cx);
                e.set_placeholder("sk_live_...");
                e.set_value(init_key, w, cx);
                e.revealed = revealed;
                e.font_size = 12.5;
                e
            });
            cx.subscribe_in(&ke, w, |this, ed_ent, ev: &EditEvent, w, cx| {
                if matches!(
                    ev,
                    EditEvent::Changed | EditEvent::Submitted | EditEvent::Blurred
                ) {
                    let next = ed_ent.read(cx).value().trim().to_string();
                    this.draft.api_key = SharedString::from(next);
                    if matches!(ev, EditEvent::Submitted) {
                        this.focus.focus(w, cx);
                    }
                    cx.notify();
                }
            })
            .detach();
            self.key_editor = Some(ke);
        }

        if self.url_editor.is_none() {
            let init_url = self.draft.site_url.to_string();
            let ue = cx.new(|cx| {
                let mut e = Editor::new("Site address", InputMode::Single, w, cx);
                e.set_placeholder("yourstore.com");
                e.set_value(init_url, w, cx);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ue, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    let raw = ed_ent.read(cx).value().to_string();
                    let stripped = raw
                        .trim_start_matches("https://")
                        .trim_start_matches("http://")
                        .to_string();
                    this.draft.site_url = SharedString::from(stripped);
                    cx.notify();
                }
                EditEvent::Submitted | EditEvent::Blurred => {
                    let clean = this.draft.site_url.trim().to_string();
                    this.draft.site_url = SharedString::from(clean.clone());
                    ed_ent.update(cx, |e, cx| e.set_value(clean, w, cx));
                    if matches!(ev, EditEvent::Submitted) {
                        this.focus.focus(w, cx);
                    }
                    cx.notify();
                }
                EditEvent::Escape => {
                    let restore = this.saved.site_url.to_string();
                    this.draft.site_url = this.saved.site_url.clone();
                    ed_ent.update(cx, |e, cx| e.set_value(restore, w, cx));
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.url_editor = Some(ue);
        }

        if self.tz_search_editor.is_none() {
            let se = cx.new(|cx| {
                let mut e = Editor::new("Search time zones", InputMode::Search, w, cx);
                e.set_placeholder("Search time zones...");
                e.font_size = 12.5;
                e
            });
            cx.subscribe_in(&se, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    this.tz_query = ed_ent.read(cx).value().to_string();
                    cx.notify();
                }
                EditEvent::Escape => {
                    this.tz_open = false;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.tz_search_editor = Some(se);
        }
    }
}

impl Render for IntegrationSettings {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let width_cell = self.width_cell.clone();
        let narrow = width_cell.get() < 420.;

        let sensor = canvas(
            move |bounds, _, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 10. {
                    width_cell.set(bw);
                }
            },
            |_, _, _, _| (),
        )
        .w_full()
        .h(px(0.));

        if self.loading {
            let phase = if disabled_motion(cx) { 0.3 } else { 0.5 };
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(560.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(40., 40., 8., phase, cx))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(pro_skeleton_bar(160., 15., 4., phase, cx))
                                .child(pro_skeleton_bar(240., 12., 4., phase, cx)),
                        ),
                )
                .child(pro_skeleton_bar(520., 36., 8., phase, cx))
                .child(pro_skeleton_bar(520., 36., 8., phase, cx))
                .child(pro_skeleton_bar(520., 36., 8., phase, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(560.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Link,
                    tr(cx, "No integration connected", "尚未连接外部服务"),
                    tr(
                        cx,
                        "Connect a commerce, analytics, or billing service to configure sync credentials.",
                        "连接电商、分析或计费服务后即可在此配置同步密钥与域名。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-connect"),
                        SharedString::from(tr(cx, "Connect Stripe store", "连接示例店铺")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let recently_copied = self
            .copied_at
            .is_some_and(|t0| t0.elapsed().as_secs_f32() < 1.8);
        let recently_regenerated = self
            .regenerated_at
            .is_some_and(|t0| t0.elapsed().as_secs_f32() < 1.8);
        if (recently_copied || recently_regenerated) && !disabled_motion(cx) {
            w.request_animation_frame();
        }

        let display_name = tr(cx, self.name.as_ref(), self.name_zh.as_ref()).to_string();
        let meta_line = if self.draft.enabled {
            tr(cx, self.meta.as_ref(), self.meta_zh.as_ref()).to_string()
        } else {
            tr(
                cx,
                "Sync paused — webhooks and scheduled imports are stopped",
                "同步已暂停 — Webhook 与定时导入已挂起",
            )
            .to_string()
        };

        let header = div()
            .w_full()
            .p(if narrow { px(14.) } else { px(16.) })
            .flex()
            .items_center()
            .gap(px(12.))
            .child(pro_app_tile(&self.provider, 40., cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .min_w_0()
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(display_name.clone()),
                            )
                            .when_some(self.count, |d, cnt| {
                                d.child(
                                    div()
                                        .h(px(18.))
                                        .px(px(5.5))
                                        .flex_none()
                                        .rounded(px(5.))
                                        .bg(p.chip)
                                        .flex()
                                        .items_center()
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.text)
                                        .child(cnt.to_string()),
                                )
                            }),
                    )
                    .child(div().min_w_0().truncate().child(pro_swap_label(
                        pro_eid(&self.id, "hdr-meta"),
                        meta_line,
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    ))),
            )
            .child(pro_switch(
                pro_eid(&self.id, "master-switch"),
                format!("{display_name} sync"),
                self.draft.enabled,
                cx,
                |this, _, cx| {
                    this.toggle_enabled(cx);
                },
            ));

        // Build foldable settings sections
        let key_ed_el = self
            .key_editor
            .as_ref()
            .map(|e| {
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(22.))
                    .child(e.clone())
                    .into_any_element()
            })
            .unwrap_or_else(|| div().flex_1().into_any_element());

        let reveal_btn = pro_icon_btn(
            pro_eid(&self.id, "key-reveal"),
            tr(
                cx,
                if self.key_revealed {
                    "Hide API key"
                } else {
                    "Reveal API key"
                },
                if self.key_revealed {
                    "隐藏 API 密钥"
                } else {
                    "显示 API 密钥"
                },
            ),
            ProIcon::Eye,
            28.,
            self.key_revealed,
            cx,
            |this, _, cx| {
                this.toggle_key_reveal(cx);
            },
        )
        .flex_none();

        let copy_lbl = if recently_copied {
            tr(cx, "Copied", "已复制")
        } else {
            tr(cx, "Copy", "复制")
        };
        let copy_icon_slot = div()
            .relative()
            .size(px(14.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .opacity(if recently_copied { 0.0 } else { 1.0 })
                    .child(pro_icon(ProIcon::Copy, 13.5, p.muted)),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .opacity(if recently_copied { 1.0 } else { 0.0 })
                    .child(pro_check_draw(
                        pro_eid(&self.id, "key-copy-chk"),
                        recently_copied,
                        14.,
                        false,
                        p.green,
                    )),
            );

        let copy_btn = pro_tactile(
            pro_eid(&self.id, "key-copy"),
            tr(cx, "Copy API key", "复制 API 密钥"),
            6.,
            if recently_copied {
                p.green_soft
            } else {
                transparent_black()
            },
            p.hover,
            transparent_black(),
            transparent_black(),
            recently_copied,
            div()
                .h(px(28.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(5.))
                .whitespace_nowrap()
                .child(copy_icon_slot)
                .child(pro_swap_label(
                    pro_eid(&self.id, "key-copy-lbl"),
                    copy_lbl,
                    12.,
                    FontWeight::MEDIUM,
                    if recently_copied { p.green_ink } else { p.text },
                )),
            cx,
            |this, _, cx| {
                this.copy_api_key(cx);
            },
        )
        .flex_none();

        let regen_lbl = if recently_regenerated {
            tr(cx, "Regenerated", "已重新生成")
        } else {
            tr(cx, "Regenerate", "重新生成")
        };
        let regen_icon_slot = div()
            .relative()
            .size(px(14.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .opacity(if recently_regenerated { 0.0 } else { 1.0 })
                    .child(pro_icon(ProIcon::KeyLock, 13.5, p.muted)),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .opacity(if recently_regenerated { 1.0 } else { 0.0 })
                    .child(pro_check_draw(
                        pro_eid(&self.id, "key-regen-chk"),
                        recently_regenerated,
                        14.,
                        false,
                        p.green,
                    )),
            );

        let regen_btn = pro_tactile(
            pro_eid(&self.id, "key-regen"),
            tr(cx, "Generate a new API key", "生成新的 API 密钥"),
            6.,
            if recently_regenerated {
                p.green_soft
            } else {
                p.card
            },
            p.hover,
            if recently_regenerated {
                p.green.opacity(0.38)
            } else {
                p.line
            },
            p.line_strong,
            recently_regenerated,
            div()
                .h(px(28.))
                .px(px(10.))
                .flex()
                .items_center()
                .gap(px(5.))
                .whitespace_nowrap()
                .child(regen_icon_slot)
                .child(pro_swap_label(
                    pro_eid(&self.id, "key-regen-lbl"),
                    regen_lbl,
                    12.5,
                    FontWeight::MEDIUM,
                    if recently_regenerated {
                        p.green_ink
                    } else {
                        p.strong
                    },
                )),
            cx,
            |this, w, cx| {
                this.generate_new_key(Some(w), cx);
            },
        )
        .flex_none();

        let new_key_note = SharedString::from(tr(
            cx,
            "New key generated — copy it now and save changes to activate.",
            "已生成新密钥——请立即复制并点击下方保存以生效。",
        ));
        let new_key_color = p.green_ink;
        let key_section = div()
            .w_full()
            .p(if narrow { px(14.) } else { px(16.) })
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(tr(cx, "API key", "API 访问密钥")),
            )
            .child(
                div()
                    .w_full()
                    .h(px(36.))
                    .pl(px(10.))
                    .pr(px(4.))
                    .rounded(px(8.))
                    .bg(if self.key_just_generated {
                        p.green_soft
                    } else {
                        p.subtle
                    })
                    .border_1()
                    .border_color(if self.key_just_generated {
                        p.green.opacity(0.38)
                    } else {
                        p.line
                    })
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .child(key_ed_el)
                    .child(reveal_btn)
                    .child(copy_btn)
                    .child(regen_btn),
            )
            .child(pro_fold(
                pro_eid(&self.id, "new-key-fold"),
                self.key_just_generated,
                18.,
                6.,
                move || {
                    div()
                        .text_size(px(12.))
                        .text_color(new_key_color)
                        .child(new_key_note)
                        .into_any_element()
                },
            ));

        let url_valid = self.is_url_valid();
        let url_ed_el = self
            .url_editor
            .as_ref()
            .map(|e| {
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(22.))
                    .child(e.clone())
                    .into_any_element()
            })
            .unwrap_or_else(|| div().flex_1().into_any_element());

        let url_hint_text = if url_valid {
            tr(cx, self.site_hint.as_ref(), self.site_hint_zh.as_ref()).to_string()
        } else {
            tr(
                cx,
                "Enter a valid domain name such as lumen-studio.com (no spaces or protocol).",
                "请输入合法的站点域名，例如 lumen-studio.com（不含空格）。",
            )
            .to_string()
        };

        let url_section = div()
            .w_full()
            .p(if narrow { px(14.) } else { px(16.) })
            .border_t_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Site address", "站点域名地址")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if url_valid { p.green_ink } else { p.red_ink })
                            .child(pro_icon(
                                if url_valid {
                                    ProIcon::Check
                                } else {
                                    ProIcon::AlertTriangle
                                },
                                12.5,
                                if url_valid { p.green } else { p.red },
                            ))
                            .child(if url_valid {
                                tr(cx, "Verified format", "域名格式有效")
                            } else {
                                tr(cx, "Invalid domain", "无效域名")
                            }),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .h(px(34.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .bg(if url_valid { p.card } else { p.red_soft })
                    .border_1()
                    .border_color(if url_valid {
                        p.line
                    } else {
                        p.red.opacity(0.45)
                    })
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(13.))
                            .text_color(p.faint)
                            .child("https://"),
                    )
                    .child(url_ed_el)
                    .child(pro_icon(
                        if url_valid {
                            ProIcon::Check
                        } else {
                            ProIcon::AlertTriangle
                        },
                        15.,
                        if url_valid { p.green } else { p.red },
                    )),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(if url_valid { p.muted } else { p.red_ink })
                    .child(url_hint_text),
            );

        let tz_open = self.tz_open;
        let query_lower = self.tz_query.trim().to_ascii_lowercase();
        let filtered_zones: Vec<SharedString> = self
            .time_zones
            .iter()
            .filter(|z| {
                query_lower.is_empty() || z.as_ref().to_ascii_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect();

        let tz_search_row = self
            .tz_search_editor
            .as_ref()
            .map(|se| {
                div()
                    .w_full()
                    .h(px(30.))
                    .px(px(8.))
                    .mb(px(4.))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_icon(ProIcon::Search, 13.5, p.muted))
                    .child(div().flex_1().min_w_0().h(px(20.)).child(se.clone()))
                    .into_any_element()
            })
            .unwrap_or_else(|| div().into_any_element());

        let mut tz_list = div()
            .id(pro_eid(&self.id, "tz-scroll"))
            .max_h(px(184.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(1.));

        if filtered_zones.is_empty() {
            tz_list = tz_list.child(
                div()
                    .py(px(10.))
                    .px(px(8.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(tr(cx, "No matching time zones", "无匹配的时区")),
            );
        } else {
            for (z_idx, zone) in filtered_zones.iter().enumerate() {
                let selected = zone == &self.draft.time_zone;
                let z_clone = zone.clone();
                tz_list = tz_list.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("tz-opt-{z_idx}")))
                        .role(Role::Button)
                        .aria_label(zone.clone())
                        .tab_index(0)
                        .min_h(px(30.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(12.5))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .child(div().min_w_0().truncate().child(zone.clone()))
                        .child(pro_check_draw(
                            pro_eid(&self.id, format!("tz-chk-{z_idx}")),
                            selected,
                            13.5,
                            false,
                            p.blue,
                        )),
                    cx,
                    move |this, _, cx| {
                        this.select_time_zone(z_clone.clone(), cx);
                    },
                ));
            }
        }

        let tz_menu = div()
            .w_full()
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .flex_col()
            .child(tz_search_row)
            .child(tz_list);

        let tz_section = div()
            .w_full()
            .p(if narrow { px(14.) } else { px(16.) })
            .border_t_1()
            .border_color(p.line)
            .relative()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(tr(cx, "Time zone", "同步时区")),
            )
            .child(pro_tactile(
                pro_eid(&self.id, "tz-btn"),
                format!(
                    "{}: {}",
                    tr(cx, "Time zone", "同步时区"),
                    self.draft.time_zone
                ),
                8.,
                p.card,
                p.card,
                if tz_open { p.line_strong } else { p.line },
                p.line_strong,
                tz_open,
                div()
                    .w_full()
                    .h(px(34.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .child(pro_icon(ProIcon::Clock, 15., p.muted))
                            .child(div().min_w_0().truncate().child(pro_swap_label(
                                pro_eid(&self.id, "tz-cur-lbl"),
                                self.draft.time_zone.clone(),
                                13.,
                                FontWeight::NORMAL,
                                p.ink,
                            ))),
                    )
                    .child(pro_icon(
                        if tz_open {
                            ProIcon::ChevronUp
                        } else {
                            ProIcon::ChevronDown
                        },
                        15.,
                        p.muted,
                    )),
                cx,
                |this, w, cx| {
                    this.tz_open = !this.tz_open;
                    if this.tz_open
                        && let Some(se) = &this.tz_search_editor
                    {
                        se.clone().update(cx, |e, cx| e.focus(w, cx));
                    }
                    cx.notify();
                },
            ))
            .child(pro_popover_presence(
                pro_eid(&self.id, "tz-pres"),
                tz_open,
                move || {
                    div()
                        .occlude()
                        .absolute()
                        .bottom(px(58.))
                        .left(px(16.))
                        .right(px(16.))
                        .child(tz_menu)
                        .into_any_element()
                },
            ));

        let fold_body = pro_fold(
            pro_eid(&self.id, "settings-fold"),
            self.draft.enabled,
            318.,
            0.,
            move || {
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(key_section)
                    .child(url_section)
                    .child(tz_section)
                    .into_any_element()
            },
        );

        let dirty = self.is_dirty();
        let can_save = self.can_save();
        let dirty_note = if !can_save && self.draft.enabled {
            tr(
                cx,
                "Fix invalid site address to save",
                "请先修正站点域名后再保存",
            )
        } else {
            tr(cx, "Unsaved changes", "有未保存的更改")
        };

        let discard_btn = pro_btn(
            pro_eid(&self.id, "dirty-discard"),
            tr(cx, "Discard", "放弃"),
            None,
            ProBtnStyle::Secondary,
            true,
            cx,
            |this, w, cx| {
                this.discard_changes(Some(w), cx);
            },
        );

        let save_btn = if can_save {
            pro_btn(
                pro_eid(&self.id, "dirty-save"),
                tr(cx, "Save changes", "保存更改"),
                None,
                ProBtnStyle::Primary,
                true,
                cx,
                |this, _, cx| {
                    this.save_changes(cx);
                },
            )
            .into_any_element()
        } else {
            div()
                .id(pro_eid(&self.id, "dirty-save"))
                .role(Role::Button)
                .aria_label(tr(cx, "Save changes", "保存更改"))
                .tab_index(-1)
                .h(px(28.))
                .px(px(12.))
                .rounded(px(6.))
                .bg(p.primary)
                .opacity(0.40)
                .flex()
                .items_center()
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.on_primary)
                .child(tr(cx, "Save changes", "保存更改"))
                .into_any_element()
        };

        let note_color = if !can_save && self.draft.enabled {
            p.red_ink
        } else {
            p.text
        };

        let save_bar = pro_presence(
            pro_eid(&self.id, "dirty-pres"),
            dirty,
            0.22,
            0.18,
            move |amt| {
                div()
                    .w_full()
                    .h(px(48. * amt))
                    .opacity(amt)
                    .overflow_hidden()
                    .border_t_1()
                    .border_color(p.line)
                    .bg(p.subtle)
                    .px(if narrow { px(12.) } else { px(16.) })
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .text_color(note_color)
                            .child(dirty_note),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(discard_btn)
                            .child(save_btn),
                    )
                    .into_any_element()
            },
        );

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.key == "escape" && this.tz_open {
                    this.tz_open = false;
                    cx.notify();
                }
            }))
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .child(sensor)
            .child(
                div()
                    .w_full()
                    .max_w(px(560.))
                    .mx_auto()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_xs())
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(fold_body)
                    .child(save_bar),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

fn default_time_zones() -> Vec<SharedString> {
    vec![
        "(GMT +00:00) UTC".into(),
        "(GMT +00:00) London".into(),
        "(GMT +01:00) Berlin".into(),
        "(GMT -05:00) New York".into(),
        "(GMT -08:00) San Francisco".into(),
        "(GMT +08:00) Shanghai".into(),
        "(GMT +08:00) Singapore".into(),
        "(GMT +09:00) Tokyo".into(),
        "(GMT +10:00) Sydney".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::{IntegrationSettings, is_valid_site_address};
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn site_address_validator_accepts_domains_and_rejects_malformed_hosts() {
        assert!(is_valid_site_address("lumen-studio.com"));
        assert!(is_valid_site_address("https://shop.kense.dev/webhooks"));
        assert!(!is_valid_site_address(""));
        assert!(!is_valid_site_address("localhost"));
        assert!(!is_valid_site_address("bad domain.com"));
        assert!(!is_valid_site_address("-invalid.com"));
    }

    #[gpui::test]
    fn editing_fields_triggers_dirty_bar_validation_discard_and_save(cx: &mut TestAppContext) {
        let card = cx.new(|cx| IntegrationSettings::new("test-integration-settings", cx));

        card.update(cx, |s, cx| {
            assert!(!s.is_dirty());
            assert!(s.is_url_valid());

            // Invalid URL marks card dirty, blocks save, allows discard
            s.set_site_url("not a domain", None, cx);
            assert!(s.is_dirty());
            assert!(!s.is_url_valid());
            assert!(!s.can_save());

            assert!(s.discard_changes(None, cx));
            assert!(!s.is_dirty());
            assert_eq!(s.draft_values().site_url.as_ref(), "lumen-studio.com");

            // Regenerate key + pick timezone + valid URL -> save commits cleanly
            s.generate_new_key(None, cx);
            s.select_time_zone("(GMT +09:00) Tokyo", cx);
            s.set_site_url("store.kense.dev", None, cx);
            assert!(s.can_save());
            assert!(s.save_changes(cx));
            assert!(!s.is_dirty());
            assert_eq!(s.saved_values().site_url.as_ref(), "store.kense.dev");
            assert_eq!(s.saved_values().time_zone.as_ref(), "(GMT +09:00) Tokyo");
        });
    }
}
