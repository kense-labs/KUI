//! Multi-source filterable log explorer with search, severity chips, and detail view.

use crate::{
    components::switch::Switch,
    foundation::{Theme, surface_radius},
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::Paint,
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};

/// Severity level for a [`LogEntry`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn tag(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// Source inventory item shown in the [`LogStream`] sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogSource {
    pub id: SharedString,
    pub label: SharedString,
    pub layer: SharedString,
    pub count: usize,
    pub bytes: u64,
}

impl LogSource {
    /// Create a log source descriptor.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        layer: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            layer: layer.into(),
            count: 0,
            bytes: 0,
        }
    }

    /// Set the observed line count for this source.
    pub fn with_count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// Set the observed byte volume for this source.
    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes = bytes;
        self
    }
}

/// Structured log row displayed in [`LogStream`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub id: SharedString,
    pub timestamp: SharedString,
    pub level: LogLevel,
    pub source: SharedString,
    pub tag: Option<SharedString>,
    pub message: SharedString,
    pub fields: Vec<(SharedString, SharedString)>,
}

impl LogEntry {
    /// Create a log entry.
    pub fn new(
        id: impl Into<SharedString>,
        timestamp: impl Into<SharedString>,
        level: LogLevel,
        source: impl Into<SharedString>,
        message: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            timestamp: timestamp.into(),
            level,
            source: source.into(),
            tag: None,
            message: message.into(),
            fields: Vec::new(),
        }
    }

    /// Attach a phase or category tag badge.
    pub fn with_tag(mut self, tag: impl Into<SharedString>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Attach a structured key-value attribute shown in the detail panel.
    pub fn with_field(
        mut self,
        key: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        self.fields.push((key.into(), value.into()));
        self
    }
}

/// Emitted when the user selects a log row to inspect its structured fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntrySelected(pub SharedString);

/// Multi-pane log viewer with source directory, live search, severity filters, and detail pane.
pub struct LogStream {
    id: ElementId,
    title: SharedString,
    sources: Vec<LogSource>,
    entries: Vec<LogEntry>,
    query_editor: Entity<Editor>,
    follow_switch: Entity<Switch>,
    query: String,
    level_filter: Option<LogLevel>,
    source_filter: Option<SharedString>,
    selected_entry: Option<SharedString>,
    stream_scroll: ScrollHandle,
    _sub: Subscription,
}

impl EventEmitter<LogEntrySelected> for LogStream {}

impl LogStream {
    /// Create a multi-source log stream explorer.
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        entries: Vec<LogEntry>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let id = id.into();
        let lang = UiLanguage::current(cx);
        let query_editor = cx.new(|cx| {
            let mut e = Editor::new(
                lang.text("Search logs", "搜索日志"),
                InputMode::Search,
                window,
                cx,
            );
            e.set_placeholder(lang.text("Filter by keyword or source…", "按关键词或来源筛选…"));
            e
        });
        let follow_switch = cx.new(|cx| {
            Switch::new(
                (id.clone(), "follow"),
                lang.text("Follow tail", "跟随末尾"),
                window,
                cx,
            )
            .with_checked(true)
        });
        let sub = cx.subscribe(&query_editor, |this, ed, ev: &EditEvent, cx| {
            if matches!(ev, EditEvent::Changed | EditEvent::Escape) {
                this.query = ed.read(cx).value().to_owned();
                cx.notify();
            }
        });
        Self {
            id,
            title: title.into(),
            sources: Vec::new(),
            entries,
            query_editor,
            follow_switch,
            query: String::new(),
            level_filter: None,
            source_filter: None,
            selected_entry: None,
            stream_scroll: ScrollHandle::new(),
            _sub: sub,
        }
    }

    /// Attach a source inventory list displayed in the left sidebar.
    pub fn with_sources(mut self, sources: Vec<LogSource>) -> Self {
        self.sources = sources;
        self
    }

    /// Replace all log entries.
    pub fn set_entries(&mut self, entries: Vec<LogEntry>, cx: &mut Context<Self>) {
        self.entries = entries;
        cx.notify();
    }

    /// Append a live log entry.
    pub fn push_entry(&mut self, entry: LogEntry, cx: &mut Context<Self>) {
        self.entries.push(entry);
        cx.notify();
    }

    /// Replace the source inventory metadata.
    pub fn set_sources(&mut self, sources: Vec<LogSource>, cx: &mut Context<Self>) {
        self.sources = sources;
        cx.notify();
    }

    /// Filter rows by severity level (`None` shows all levels).
    pub fn set_level_filter(&mut self, level: Option<LogLevel>, cx: &mut Context<Self>) {
        self.level_filter = level;
        cx.notify();
    }

    /// Filter rows by source identifier (`None` shows all sources).
    pub fn set_source_filter(&mut self, source: Option<SharedString>, cx: &mut Context<Self>) {
        self.source_filter = source;
        cx.notify();
    }

    /// Return the currently selected entry identifier.
    pub fn selected_entry_id(&self) -> Option<&str> {
        self.selected_entry.as_deref()
    }

    /// Return the number of entries matching the active filters.
    pub fn filtered_count(&self) -> usize {
        self.filtered_entries().len()
    }

    fn filtered_entries(&self) -> Vec<&LogEntry> {
        let q = self.query.trim().to_ascii_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                if let Some(level) = self.level_filter
                    && entry.level != level
                {
                    return false;
                }
                if let Some(src) = &self.source_filter
                    && &entry.source != src
                {
                    return false;
                }
                if !q.is_empty() {
                    let match_msg = entry.message.to_ascii_lowercase().contains(&q);
                    let match_src = entry.source.to_ascii_lowercase().contains(&q);
                    let match_tag = entry
                        .tag
                        .as_ref()
                        .is_some_and(|t| t.to_ascii_lowercase().contains(&q));
                    if !match_msg && !match_src && !match_tag {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    fn level_color(level: LogLevel, cx: &App) -> Hsla {
        let c = &Theme::global(cx).tokens.colors;
        match level {
            LogLevel::Error => c.danger,
            LogLevel::Warn => c.warning,
            LogLevel::Info => c.accent,
            LogLevel::Debug => c.text_muted,
        }
    }

    fn format_bytes(bytes: u64) -> String {
        if bytes >= 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{bytes} B")
        }
    }
}

impl Render for LogStream {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).clone();
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let radius = surface_radius(cx);

        let matched: Vec<LogEntry> = self.filtered_entries().into_iter().cloned().collect();
        let selected = self
            .selected_entry
            .as_deref()
            .and_then(|id| self.entries.iter().find(|e| e.id.as_ref() == id))
            .cloned();

        // Level filter bar
        let levels: [(Option<LogLevel>, &str); 5] = [
            (None, lang.text("All", "全部")),
            (Some(LogLevel::Error), "ERROR"),
            (Some(LogLevel::Warn), "WARN"),
            (Some(LogLevel::Info), "INFO"),
            (Some(LogLevel::Debug), "DEBUG"),
        ];
        let mut level_bar = div().flex().items_center().gap_1();
        for (idx, (lvl, lbl)) in levels.into_iter().enumerate() {
            let active = self.level_filter == lvl;
            level_bar = level_bar.child(
                div()
                    .id((self.id.clone(), SharedString::from(format!("lvl-{idx}"))))
                    .cursor_pointer()
                    .px_2()
                    .py_1()
                    .rounded(radius)
                    .border_1()
                    .border_color(if active {
                        t.tokens.colors.accent
                    } else {
                        t.tokens.colors.border
                    })
                    .bg(if active {
                        t.tokens.colors.accent
                    } else {
                        t.tokens.colors.surface
                    })
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if active {
                        t.tokens.colors.accent_foreground
                    } else {
                        t.tokens.colors.text_secondary
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_level_filter(lvl, cx);
                    }))
                    .child(lbl),
            );
        }

        // Optional source sidebar
        let source_sidebar = (!self.sources.is_empty()).then(|| {
            let mut list = div()
                .w(px(192.))
                .flex_none()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .id((self.id.clone(), "src-all"))
                        .cursor_pointer()
                        .px_2()
                        .py_1p5()
                        .rounded(radius)
                        .border_1()
                        .border_color(if self.source_filter.is_none() {
                            t.tokens.colors.accent
                        } else {
                            t.tokens.colors.border
                        })
                        .bg(if self.source_filter.is_none() {
                            t.tokens.colors.accent_subtle
                        } else {
                            t.tokens.colors.surface
                        })
                        .flex()
                        .justify_between()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.tokens.colors.foreground)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.set_source_filter(None, cx);
                        }))
                        .child(lang.text("All sources", "全部来源"))
                        .child(self.entries.len().to_string()),
                );

            for (idx, src) in self.sources.iter().enumerate() {
                let sid = src.id.clone();
                let active = self.source_filter.as_ref() == Some(&src.id);
                list = list.child(
                    div()
                        .id((self.id.clone(), SharedString::from(format!("src-{idx}"))))
                        .cursor_pointer()
                        .px_2()
                        .py_1p5()
                        .rounded(radius)
                        .border_1()
                        .border_color(if active {
                            t.tokens.colors.accent
                        } else {
                            t.tokens.colors.border
                        })
                        .bg(if active {
                            t.tokens.colors.accent_subtle
                        } else {
                            t.tokens.colors.surface
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_source_filter(Some(sid.clone()), cx);
                        }))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t.tokens.colors.foreground)
                                .child(src.label.clone())
                                .child(src.count.to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(t.tokens.colors.text_muted)
                                .child(format!(
                                    "{} · {}",
                                    src.layer,
                                    Self::format_bytes(src.bytes)
                                )),
                        ),
                );
            }
            list
        });

        // Main stream list
        let mut rows = div()
            .id((self.id.clone(), "rows"))
            .flex_1()
            .min_w(px(320.))
            .flex()
            .flex_col()
            .gap(px(2.))
            .p_2p5()
            .rounded(radius)
            .bg(t.tokens.colors.surface_muted)
            .border_1()
            .border_color(t.tokens.colors.border)
            .overflow_y_scroll()
            .track_scroll(&self.stream_scroll)
            .min_h(px(240.))
            .max_h(px(360.));

        if matched.is_empty() {
            rows = rows.child(
                div()
                    .p_4()
                    .text_size(px(12.))
                    .text_color(t.tokens.colors.text_muted)
                    .child(lang.text("No matching log entries.", "暂无匹配日志。")),
            );
        } else {
            for (idx, entry) in matched.iter().enumerate() {
                let eid = entry.id.clone();
                let is_sel = self.selected_entry.as_ref() == Some(&entry.id);
                let lc = Self::level_color(entry.level, cx);
                rows = rows.child(
                    div()
                        .id((self.id.clone(), SharedString::from(format!("row-{idx}"))))
                        .cursor_pointer()
                        .px_2()
                        .py(px(3.))
                        .rounded(px(4.))
                        .bg(if is_sel {
                            t.tokens.colors.accent_subtle
                        } else {
                            transparent_black()
                        })
                        .hover(|s| s.bg(t.tokens.colors.surface))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.selected_entry = Some(eid.clone());
                            cx.emit(LogEntrySelected(eid.clone()));
                            cx.notify();
                        }))
                        .flex()
                        .items_start()
                        .gap_2()
                        .text_size(px(12.))
                        .child(
                            div()
                                .flex_none()
                                .w(px(72.))
                                .text_color(t.tokens.colors.text_muted)
                                .child(entry.timestamp.clone()),
                        )
                        .child(
                            div()
                                .flex_none()
                                .w(px(42.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(lc)
                                .child(entry.level.tag()),
                        )
                        .child(
                            div()
                                .flex_none()
                                .w(px(96.))
                                .truncate()
                                .text_color(t.tokens.colors.text_secondary)
                                .child(entry.source.clone()),
                        )
                        .when_some(entry.tag.clone(), |d, tag| {
                            d.child(
                                div()
                                    .flex_none()
                                    .px_1p5()
                                    .py(px(1.))
                                    .rounded(px(4.))
                                    .bg(t.tokens.colors.surface)
                                    .border_1()
                                    .border_color(t.tokens.colors.border)
                                    .text_size(px(10.))
                                    .text_color(t.tokens.colors.text_secondary)
                                    .child(tag),
                            )
                        })
                        .child(
                            div()
                                .flex_1()
                                .text_color(t.tokens.colors.foreground)
                                .child(entry.message.clone()),
                        ),
                );
            }
        }

        // Selected entry structured detail panel
        let detail_pane = selected.map(|entry| {
            let mut fields = div().flex().flex_col().gap_1();
            for (k, v) in entry.fields {
                fields = fields.child(
                    div()
                        .flex()
                        .justify_between()
                        .gap_2()
                        .text_size(px(11.))
                        .child(div().text_color(t.tokens.colors.text_secondary).child(k))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t.tokens.colors.foreground)
                                .child(v),
                        ),
                );
            }
            div()
                .w(px(240.))
                .flex_none()
                .p_3()
                .rounded(radius)
                .bg(t.tokens.colors.surface_muted)
                .border_1()
                .border_color(t.tokens.colors.border)
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.tokens.colors.foreground)
                        .child(lang.text("Entry details", "日志详情")),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(t.tokens.colors.text_secondary)
                        .child(format!(
                            "{} · {} · {}",
                            entry.timestamp,
                            entry.level.tag(),
                            entry.source
                        )),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(t.tokens.colors.foreground)
                        .child(entry.message),
                )
                .child(fields)
        });

        div()
            .id(self.id.clone())
            .w_full()
            .max_w(px(940.))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .rounded(radius)
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(icon(Glyph::FileText, 15., t.tokens.colors.foreground))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(t.tokens.colors.foreground)
                                    .child(format!(
                                        "{} ({}/{})",
                                        self.title,
                                        matched.len(),
                                        self.entries.len()
                                    )),
                            ),
                    )
                    .child(self.follow_switch.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        div()
                            .w(px(260.))
                            .px_2()
                            .py_1()
                            .rounded(radius)
                            .border_1()
                            .border_color(t.tokens.colors.border)
                            .bg(t.tokens.colors.surface)
                            .child(self.query_editor.clone()),
                    )
                    .child(level_bar),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .items_start()
                    .children(source_sidebar)
                    .child(rows)
                    .children(detail_pane),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{LogEntry, LogLevel, LogStream};
    use gpui::TestAppContext;

    #[test]
    fn format_bytes_scales_units() {
        assert_eq!(LogStream::format_bytes(512), "512 B");
        assert_eq!(LogStream::format_bytes(2048), "2.0 KB");
        assert_eq!(LogStream::format_bytes(3 * 1024 * 1024), "3.0 MB");
    }

    #[gpui::test]
    fn filters_entries_by_level_and_source(cx: &mut TestAppContext) {
        cx.update(crate::foundation::init);
        let (view, _) = cx.add_window_view(|w, cx| {
            LogStream::new(
                "logs-test",
                "Logs",
                vec![
                    LogEntry::new("1", "10:00:01", LogLevel::Info, "api", "request ok"),
                    LogEntry::new("2", "10:00:02", LogLevel::Error, "db", "timeout"),
                    LogEntry::new("3", "10:00:03", LogLevel::Warn, "api", "retry"),
                ],
                w,
                cx,
            )
        });
        view.update(cx, |v, cx| {
            assert_eq!(v.filtered_count(), 3);
            v.set_level_filter(Some(LogLevel::Error), cx);
            assert_eq!(v.filtered_count(), 1);
            v.set_level_filter(None, cx);
            v.set_source_filter(Some("api".into()), cx);
            assert_eq!(v.filtered_count(), 2);
        });
    }
}
