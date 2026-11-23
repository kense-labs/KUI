//! Interactive stories for StepConsole, LogStream, and PacketDissector.

use gpui::{prelude::*, *};
use kui::{
    components::{
        log_stream::{LogEntry, LogLevel, LogSource, LogStream},
        packet_dissector::{
            FrameDirection, PacketDissector, PacketFrame, ProtocolField, ProtocolLayer,
        },
        step_console::{StepConsole, StepLogLevel, StepLogLine, StepStage, StepStatus},
    },
    primitives::control::UiLanguage,
};

pub const IDS: &[&str] = &["step-console", "log-stream", "packet-dissector"];

pub fn story(id: &str, window: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    match id {
        "step-console" => Some(cx.new(StepDemo::new).into()),
        "log-stream" => Some(cx.new(|cx| LogDemo::new(window, cx)).into()),
        "packet-dissector" => Some(cx.new(PacketDemo::new).into()),
        _ => None,
    }
}

struct StepDemo(Entity<StepConsole>);

impl StepDemo {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let stages = vec![
            StepStage::new(
                "preflight",
                lang.text("Preflight checks", "环境检查"),
                StepStatus::Succeeded,
            )
            .with_offset_ms(0.0)
            .with_duration_ms(180.0)
            .with_summary(lang.text(
                "Disk, socket paths and runtime image verified.",
                "磁盘、套接字路径与镜像校验完成。",
            ))
            .with_metric("image", "ok")
            .with_metric("disk", "14 GB"),
            StepStage::new(
                "boot",
                lang.text("Runtime boot", "运行时启动"),
                StepStatus::Succeeded,
            )
            .with_offset_ms(180.0)
            .with_duration_ms(1420.0)
            .with_summary(lang.text(
                "Worker process booted and published health endpoint.",
                "工作进程启动并发布健康端点。",
            ))
            .with_metric("pid", "4821")
            .with_metric("ready", "1.42s"),
            StepStage::new(
                "attach",
                lang.text("Client attach", "客户端连接"),
                StepStatus::Running,
            )
            .with_offset_ms(1600.0)
            .with_duration_ms(340.0)
            .with_summary(lang.text(
                "Handshake complete; streaming live diagnostics.",
                "握手完成，正在接收实时数据。",
            ))
            .with_detail(lang.text(
                "Active stage updates automatically when Auto-follow is enabled.",
                "开启自动跟随时会跟踪最新活动步骤。",
            )),
        ];
        let logs = vec![
            StepLogLine::new(
                "l1",
                "10:12:01.020",
                StepLogLevel::Info,
                "preflight",
                "image checksum verified",
            )
            .with_stage("preflight"),
            StepLogLine::new(
                "l2",
                "10:12:01.180",
                StepLogLevel::Info,
                "runtime",
                "worker process spawned pid=4821",
            )
            .with_stage("boot"),
            StepLogLine::new(
                "l3",
                "10:12:02.410",
                StepLogLevel::Warn,
                "runtime",
                "cold cache warmup took 620ms",
            )
            .with_stage("boot"),
            StepLogLine::new(
                "l4",
                "10:12:02.600",
                StepLogLevel::Info,
                "client",
                "session attached over local transport",
            )
            .with_stage("attach"),
            StepLogLine::new(
                "l5",
                "10:12:02.940",
                StepLogLevel::Info,
                "client",
                "subscribed to live event stream",
            )
            .with_stage("attach"),
        ];
        Self(cx.new(|_| {
            StepConsole::new(
                "story-step-console",
                lang.text("Pipeline execution", "流水线执行"),
                stages,
                logs,
            )
        }))
    }
}

impl Render for StepDemo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .p_6()
            .flex()
            .justify_center()
            .child(self.0.clone())
    }
}

struct LogDemo(Entity<LogStream>);

impl LogDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let sources = vec![
            LogSource::new("gateway", "gateway", "edge")
                .with_count(2)
                .with_bytes(1420),
            LogSource::new("worker", "worker", "runtime")
                .with_count(2)
                .with_bytes(3840),
            LogSource::new("storage", "storage", "data")
                .with_count(1)
                .with_bytes(890),
        ];
        let entries = vec![
            LogEntry::new(
                "e1",
                "10:15:01.120",
                LogLevel::Info,
                "gateway",
                "accepted session from 127.0.0.1",
            )
            .with_tag("http")
            .with_field("status", "200")
            .with_field("latency", "4.2ms"),
            LogEntry::new(
                "e2",
                "10:15:01.410",
                LogLevel::Debug,
                "worker",
                "loaded workspace config in 12ms",
            )
            .with_tag("init")
            .with_field("pid", "4821"),
            LogEntry::new(
                "e3",
                "10:15:02.005",
                LogLevel::Warn,
                "worker",
                "slow query threshold exceeded (86ms)",
            )
            .with_tag("query")
            .with_field("duration_ms", "86"),
            LogEntry::new(
                "e4",
                "10:15:02.640",
                LogLevel::Error,
                "storage",
                "retryable lock timeout on index flush",
            )
            .with_tag("wal")
            .with_field("attempt", "2"),
            LogEntry::new(
                "e5",
                "10:15:02.910",
                LogLevel::Info,
                "gateway",
                "health check passed",
            )
            .with_tag("probe")
            .with_field("status", "200"),
        ];
        Self(cx.new(|cx| {
            LogStream::new(
                "story-log-stream",
                lang.text("Live logs", "实时日志"),
                entries,
                window,
                cx,
            )
            .with_sources(sources)
        }))
    }
}

impl Render for LogDemo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .p_6()
            .flex()
            .justify_center()
            .child(self.0.clone())
    }
}

struct PacketDemo(Entity<PacketDissector>);

impl PacketDemo {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let frames = vec![
            PacketFrame::new(
                "f1",
                1,
                FrameDirection::Outbound,
                "HTTP",
                "POST /v1/messages",
            )
            .with_channel("local-rpc")
            .with_rtt_ms(18.4)
            .with_layers(vec![
                ProtocolLayer::new("Transport", "stream #4 · 64 bytes")
                    .with_range(0, 16)
                    .with_fields(vec![
                        ProtocolField::new("channel", "local-rpc"),
                        ProtocolField::new("direction", "outbound"),
                    ]),
                ProtocolLayer::new("HTTP/1.1", "POST /v1/messages")
                    .with_range(16, 64)
                    .with_fields(vec![
                        ProtocolField::new("method", "POST"),
                        ProtocolField::new("authorization", "[REDACTED]").redacted(),
                    ]),
            ])
            .with_payload(
                b"POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n{\"ok\":true}".to_vec(),
            ),
            PacketFrame::new(
                "f2",
                2,
                FrameDirection::Inbound,
                "HTTP",
                "200 OK (application/json)",
            )
            .with_channel("local-rpc")
            .with_rtt_ms(18.4)
            .with_layers(vec![ProtocolLayer::new("HTTP/1.1", "200 OK").with_fields(
                vec![
                    ProtocolField::new("status", "200"),
                    ProtocolField::new("content-type", "application/json"),
                ],
            )])
            .with_payload(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"id\":\"msg_1\"}"
                    .to_vec(),
            ),
            PacketFrame::new(
                "f3",
                3,
                FrameDirection::Inbound,
                "RPC",
                "504 gateway timeout",
            )
            .with_channel("worker-bus")
            .with_rtt_ms(2005.0)
            .with_error(true)
            .with_layers(vec![
                ProtocolLayer::new("RPC", "deadline exceeded")
                    .with_fields(vec![ProtocolField::new("code", "DEADLINE_EXCEEDED")]),
            ])
            .with_payload(b"{\"error\":\"deadline_exceeded\"}".to_vec()),
        ];
        Self(cx.new(|_| {
            PacketDissector::new(
                "story-packet-dissector",
                lang.text("Protocol frames", "协议帧分析"),
                frames,
            )
        }))
    }
}

impl Render for PacketDemo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .p_6()
            .flex()
            .justify_center()
            .child(self.0.clone())
    }
}
