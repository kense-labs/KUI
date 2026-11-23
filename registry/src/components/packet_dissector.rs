//! Layered protocol frame inspector with redaction markers and hex/ASCII dump.

use crate::{
    foundation::{Theme, surface_radius},
    primitives::{
        control::UiLanguage,
        family::Paint,
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::collections::BTreeSet;

/// Direction of a captured network or IPC frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameDirection {
    #[default]
    Outbound,
    Inbound,
}

impl FrameDirection {
    fn symbol(self) -> &'static str {
        match self {
            Self::Outbound => "TX →",
            Self::Inbound => "RX ←",
        }
    }
}

/// Single key-value field within a decoded protocol layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolField {
    pub key: SharedString,
    pub value: SharedString,
    pub redacted: bool,
}

impl ProtocolField {
    /// Create a decoded protocol field.
    pub fn new(key: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            redacted: false,
        }
    }

    /// Mark this field as redacted so sensitive values receive warning emphasis.
    pub fn redacted(mut self) -> Self {
        self.redacted = true;
        self
    }
}

/// One protocol layer decoded from a captured frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolLayer {
    pub name: SharedString,
    pub summary: SharedString,
    pub byte_range: Option<(usize, usize)>,
    pub fields: Vec<ProtocolField>,
}

impl ProtocolLayer {
    /// Create a decoded protocol layer.
    pub fn new(name: impl Into<SharedString>, summary: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            summary: summary.into(),
            byte_range: None,
            fields: Vec::new(),
        }
    }

    /// Attach the byte slice range `[start, end)` covered by this layer.
    pub fn with_range(mut self, start: usize, end: usize) -> Self {
        self.byte_range = Some((start, end.max(start)));
        self
    }

    /// Set the decoded fields belonging to this layer.
    pub fn with_fields(mut self, fields: Vec<ProtocolField>) -> Self {
        self.fields = fields;
        self
    }
}

/// Captured network or IPC frame displayed in [`PacketDissector`].
#[derive(Clone, Debug, PartialEq)]
pub struct PacketFrame {
    pub id: SharedString,
    pub index: usize,
    pub direction: FrameDirection,
    pub protocol: SharedString,
    pub channel: SharedString,
    pub summary: SharedString,
    pub wire_bytes: u64,
    pub rtt_ms: Option<f64>,
    pub is_error: bool,
    pub layers: Vec<ProtocolLayer>,
    pub payload: Vec<u8>,
}

impl PacketFrame {
    /// Create a captured frame entry.
    pub fn new(
        id: impl Into<SharedString>,
        index: usize,
        direction: FrameDirection,
        protocol: impl Into<SharedString>,
        summary: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            index,
            direction,
            protocol: protocol.into(),
            channel: SharedString::default(),
            summary: summary.into(),
            wire_bytes: 0,
            rtt_ms: None,
            is_error: false,
            layers: Vec::new(),
            payload: Vec::new(),
        }
    }

    /// Set the channel or endpoint name.
    pub fn with_channel(mut self, channel: impl Into<SharedString>) -> Self {
        self.channel = channel.into();
        self
    }

    /// Set the total wire size in bytes.
    pub fn with_wire_bytes(mut self, bytes: u64) -> Self {
        self.wire_bytes = bytes;
        self
    }

    /// Attach a measured round-trip time in milliseconds.
    pub fn with_rtt_ms(mut self, rtt_ms: f64) -> Self {
        if rtt_ms.is_finite() {
            self.rtt_ms = Some(rtt_ms.max(0.0));
        }
        self
    }

    /// Mark whether this frame represents an error response or transport failure.
    pub fn with_error(mut self, is_error: bool) -> Self {
        self.is_error = is_error;
        self
    }

    /// Attach decoded protocol layers.
    pub fn with_layers(mut self, layers: Vec<ProtocolLayer>) -> Self {
        self.layers = layers;
        self
    }

    /// Attach raw (already redacted) payload bytes for hex/ASCII inspection.
    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        if self.wire_bytes == 0 {
            self.wire_bytes = payload.len() as u64;
        }
        self.payload = payload;
        self
    }
}

/// Emitted when the user selects a frame in [`PacketDissector`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameSelected(pub SharedString);

/// Split frame list, protocol layer tree, and hex/ASCII wire inspector.
pub struct PacketDissector {
    id: ElementId,
    title: SharedString,
    frames: Vec<PacketFrame>,
    selected_frame: Option<SharedString>,
    protocol_filter: Option<SharedString>,
    errors_only: bool,
    frame_scroll: ScrollHandle,
}

impl EventEmitter<FrameSelected> for PacketDissector {}

impl PacketDissector {
    /// Create a protocol frame inspector with initial captured frames.
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        frames: Vec<PacketFrame>,
    ) -> Self {
        let selected_frame = frames.first().map(|f| f.id.clone());
        Self {
            id: id.into(),
            title: title.into(),
            frames,
            selected_frame,
            protocol_filter: None,
            errors_only: false,
            frame_scroll: ScrollHandle::new(),
        }
    }

    /// Replace all captured frames.
    pub fn set_frames(&mut self, frames: Vec<PacketFrame>, cx: &mut Context<Self>) {
        self.frames = frames;
        if self
            .selected_frame
            .as_ref()
            .is_none_or(|id| !self.frames.iter().any(|f| &f.id == id))
        {
            self.selected_frame = self.frames.first().map(|f| f.id.clone());
        }
        cx.notify();
    }

    /// Append a live captured frame.
    pub fn push_frame(&mut self, frame: PacketFrame, cx: &mut Context<Self>) {
        if self.selected_frame.is_none() {
            self.selected_frame = Some(frame.id.clone());
        }
        self.frames.push(frame);
        cx.notify();
    }

    /// Select a frame by identifier.
    pub fn select_frame(&mut self, frame_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let next = frame_id.into();
        if self.frames.iter().any(|f| f.id == next) {
            self.selected_frame = Some(next.clone());
            cx.emit(FrameSelected(next));
            cx.notify();
        }
    }

    /// Filter frames by protocol name (`None` shows all protocols).
    pub fn set_protocol_filter(&mut self, protocol: Option<SharedString>, cx: &mut Context<Self>) {
        self.protocol_filter = protocol;
        cx.notify();
    }

    /// Toggle error-only frame filtering.
    pub fn set_errors_only(&mut self, errors_only: bool, cx: &mut Context<Self>) {
        self.errors_only = errors_only;
        cx.notify();
    }

    /// Return the currently selected frame identifier.
    pub fn selected_frame_id(&self) -> Option<&str> {
        self.selected_frame.as_deref()
    }

    /// Return the frames matching the active protocol and error filters.
    pub fn filtered_frames(&self) -> Vec<&PacketFrame> {
        self.frames
            .iter()
            .filter(|f| {
                let err_ok = !self.errors_only || f.is_error;
                let proto_ok = self
                    .protocol_filter
                    .as_ref()
                    .is_none_or(|p| f.protocol.eq_ignore_ascii_case(p));
                err_ok && proto_ok
            })
            .collect()
    }

    /// Format payload bytes as canonical 16-byte hex + printable ASCII rows.
    pub fn format_hex_dump(bytes: &[u8], max_bytes: usize) -> String {
        if bytes.is_empty() {
            return String::new();
        }
        let len = bytes.len().min(max_bytes);
        let mut out = String::new();
        for (row_idx, chunk) in bytes[..len].chunks(16).enumerate() {
            let offset = row_idx * 16;
            out.push_str(&format!("{offset:04x}  "));
            for i in 0..16 {
                if let Some(&b) = chunk.get(i) {
                    out.push_str(&format!("{b:02x} "));
                } else {
                    out.push_str("   ");
                }
                if i == 7 {
                    out.push(' ');
                }
            }
            out.push_str(" |");
            for &b in chunk {
                out.push(if (0x20..=0x7e).contains(&b) {
                    b as char
                } else {
                    '.'
                });
            }
            out.push_str("|\n");
        }
        if bytes.len() > max_bytes {
            out.push_str(&format!("… +{} bytes", bytes.len() - max_bytes));
        }
        out
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

impl Render for PacketDissector {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).clone();
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let radius = surface_radius(cx);

        let protocols: Vec<SharedString> = self
            .frames
            .iter()
            .map(|f| f.protocol.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let filtered: Vec<PacketFrame> = self.filtered_frames().into_iter().cloned().collect();
        let selected = self
            .selected_frame
            .as_deref()
            .and_then(|id| filtered.iter().find(|f| f.id.as_ref() == id))
            .or_else(|| filtered.first())
            .cloned();

        // Filter strip
        let mut filter_bar = div().flex().items_center().flex_wrap().gap_1p5().child(
            div()
                .id((self.id.clone(), "proto-all"))
                .cursor_pointer()
                .px_2p5()
                .py_1()
                .rounded(radius)
                .border_1()
                .border_color(if self.protocol_filter.is_none() {
                    t.tokens.colors.accent
                } else {
                    t.tokens.colors.border
                })
                .bg(if self.protocol_filter.is_none() {
                    t.tokens.colors.accent
                } else {
                    t.tokens.colors.surface
                })
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if self.protocol_filter.is_none() {
                    t.tokens.colors.accent_foreground
                } else {
                    t.tokens.colors.text_secondary
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.set_protocol_filter(None, cx);
                }))
                .child(format!(
                    "{} ({})",
                    lang.text("All", "全部"),
                    self.frames.len()
                )),
        );

        for (idx, proto) in protocols.into_iter().enumerate() {
            let p_clone = proto.clone();
            let active = self.protocol_filter.as_ref() == Some(&proto);
            filter_bar = filter_bar.child(
                div()
                    .id((self.id.clone(), SharedString::from(format!("proto-{idx}"))))
                    .cursor_pointer()
                    .px_2p5()
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
                        this.set_protocol_filter(Some(p_clone.clone()), cx);
                    }))
                    .child(proto),
            );
        }

        filter_bar = filter_bar.child(
            div()
                .id((self.id.clone(), "errors-only"))
                .cursor_pointer()
                .px_2p5()
                .py_1()
                .rounded(radius)
                .border_1()
                .border_color(if self.errors_only {
                    t.tokens.colors.danger
                } else {
                    t.tokens.colors.border
                })
                .bg(if self.errors_only {
                    t.tokens.colors.danger.opacity(0.14)
                } else {
                    t.tokens.colors.surface
                })
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if self.errors_only {
                    t.tokens.colors.danger
                } else {
                    t.tokens.colors.text_secondary
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.set_errors_only(!this.errors_only, cx);
                }))
                .child(lang.text("Errors only", "仅异常帧")),
        );

        // Left frame list
        let mut list = div()
            .id((self.id.clone(), "frames"))
            .flex_1()
            .min_w(px(340.))
            .flex()
            .flex_col()
            .gap_1()
            .overflow_y_scroll()
            .track_scroll(&self.frame_scroll)
            .max_h(px(340.));

        for (idx, frame) in filtered.iter().enumerate() {
            let fid = frame.id.clone();
            let is_sel = selected.as_ref().is_some_and(|s| s.id == frame.id);
            list = list.child(
                div()
                    .id((self.id.clone(), SharedString::from(format!("frame-{idx}"))))
                    .cursor_pointer()
                    .px_2p5()
                    .py_1p5()
                    .rounded(radius)
                    .border_1()
                    .border_color(if is_sel {
                        t.tokens.colors.accent
                    } else if frame.is_error {
                        t.tokens.colors.danger.opacity(0.45)
                    } else {
                        t.tokens.colors.border
                    })
                    .bg(if is_sel {
                        t.tokens.colors.accent_subtle
                    } else {
                        t.tokens.colors.surface
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_frame(fid.clone(), cx);
                    }))
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(12.))
                    .child(
                        div()
                            .w(px(32.))
                            .text_color(t.tokens.colors.text_muted)
                            .child(format!("#{}", frame.index)),
                    )
                    .child(
                        div()
                            .w(px(40.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t.tokens.colors.accent)
                            .child(frame.direction.symbol()),
                    )
                    .child(
                        div()
                            .w(px(62.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if frame.is_error {
                                t.tokens.colors.danger
                            } else {
                                t.tokens.colors.foreground
                            })
                            .child(frame.protocol.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_color(t.tokens.colors.foreground)
                            .child(frame.summary.clone()),
                    )
                    .child(
                        div()
                            .w(px(56.))
                            .text_right()
                            .text_color(t.tokens.colors.text_secondary)
                            .child(Self::format_bytes(frame.wire_bytes)),
                    )
                    .when_some(frame.rtt_ms, |d, rtt| {
                        d.child(
                            div()
                                .w(px(48.))
                                .text_right()
                                .text_color(t.tokens.colors.text_muted)
                                .child(format!("{rtt:.1}ms")),
                        )
                    }),
            );
        }

        // Right layer tree + hex dump
        let inspector = selected.map(|frame| {
            let mut layers_col = div().flex().flex_col().gap_2();
            for layer in frame.layers {
                let mut fields_col = div().flex().flex_col().gap(px(3.));
                for f in layer.fields {
                    fields_col = fields_col.child(
                        div()
                            .flex()
                            .justify_between()
                            .gap_2()
                            .text_size(px(11.))
                            .child(
                                div()
                                    .text_color(t.tokens.colors.text_secondary)
                                    .child(f.key),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if f.redacted {
                                        t.tokens.colors.warning
                                    } else {
                                        t.tokens.colors.foreground
                                    })
                                    .child(f.value),
                            ),
                    );
                }
                let range_label = layer
                    .byte_range
                    .map(|(s, e)| format!(" [{s}..{e}]"))
                    .unwrap_or_default();
                layers_col = layers_col.child(
                    div()
                        .p_2p5()
                        .rounded(radius)
                        .bg(t.tokens.colors.surface_muted)
                        .border_1()
                        .border_color(t.tokens.colors.border)
                        .flex()
                        .flex_col()
                        .gap_1p5()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t.tokens.colors.foreground)
                                .child(format!(
                                    "▸ {}{range_label} — {}",
                                    layer.name, layer.summary
                                )),
                        )
                        .child(fields_col),
                );
            }

            let hex = Self::format_hex_dump(&frame.payload, 256);
            div()
                .w(px(390.))
                .flex_none()
                .flex()
                .flex_col()
                .gap_2()
                .child(layers_col)
                .when(!hex.is_empty(), |d| {
                    d.child(
                        div()
                            .p_2p5()
                            .rounded(radius)
                            .bg(t.tokens.colors.surface_muted)
                            .border_1()
                            .border_color(t.tokens.colors.border)
                            .text_size(px(11.))
                            .text_color(t.tokens.colors.foreground)
                            .child(hex),
                    )
                })
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
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(icon(Glyph::Link, 15., t.tokens.colors.foreground))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(t.tokens.colors.foreground)
                                    .child(self.title.clone()),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(t.tokens.colors.text_muted)
                            .child(format!(
                                "{} / {} {}",
                                filtered.len(),
                                self.frames.len(),
                                lang.text("frames", "帧")
                            )),
                    ),
            )
            .child(filter_bar)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .items_start()
                    .child(list)
                    .children(inspector),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameDirection, PacketDissector, PacketFrame};

    #[test]
    fn formats_hex_and_ascii_with_truncation() {
        let dump = PacketDissector::format_hex_dump(b"GET /health HTTP/1.1", 16);
        assert!(dump.contains("0000  47 45 54 20"));
        assert!(dump.contains("|GET /health HTTP|"));
        assert!(dump.contains("+4 bytes"));
    }

    #[test]
    fn filters_by_protocol_and_error_flag() {
        let frames = vec![
            PacketFrame::new("f1", 1, FrameDirection::Outbound, "HTTP", "GET /models"),
            PacketFrame::new("f2", 2, FrameDirection::Inbound, "RPC", "tool.run").with_error(true),
        ];
        let mut d = PacketDissector::new("pkt", "Frames", frames);
        assert_eq!(d.filtered_frames().len(), 2);
        d.protocol_filter = Some("rpc".into());
        assert_eq!(d.filtered_frames().len(), 1);
        d.protocol_filter = None;
        d.errors_only = true;
        assert_eq!(d.filtered_frames().len(), 1);
        assert_eq!(d.filtered_frames()[0].id.as_ref(), "f2");
    }
}
