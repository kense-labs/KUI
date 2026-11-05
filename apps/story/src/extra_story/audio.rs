//! Sample-driven audio stories. No device permissions are requested.
use gpui::{prelude::*, *};
use kui::components::{
    now_playing::{NowPlaying, PlayerEvent, Track},
    voice_orb::{VoiceOrb, VoiceOrbEvent, VoiceState},
    voice_recorder::{
        RecorderEvent, RecorderOperation, RecorderPhase, VoiceMessage, VoiceRecorder,
    },
};
use kui::primitives::{control::UiLanguage, family::Paint};
use std::{sync::Arc, time::Duration};
pub const IDS: &[&str] = &["voice-orb", "now-playing", "voice-recorder"];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| AudioStory::new(id, cx)).into())
}
enum Example {
    Orb(Entity<VoiceOrb>),
    Player(Entity<NowPlaying>),
    Recorder(Entity<VoiceRecorder>),
}
struct AudioStory {
    example: Example,
    language: UiLanguage,
    muted: bool,
    failure_seen: bool,
    recording_id: u64,
    _subs: Vec<Subscription>,
}
impl AudioStory {
    fn new(id: &str, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en, zh| language.text(en, zh);
        let mut subs = Vec::new();
        let example = match id {
            "voice-orb" => {
                let orb = cx.new(|cx| VoiceOrb::new(cx));
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                orb.update(cx, |view, cx| view.set_visible(visible, cx));
                let weak = orb.downgrade();
                cx.spawn(async move |_, cx| {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(200))
                            .await;
                        if weak
                            .update(cx, |view, cx| {
                                let visible = cx
                                    .try_global::<crate::StoryBridge>()
                                    .is_none_or(|bridge| (bridge.visible)());
                                view.set_visible(visible, cx);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                subs.push(
                    cx.subscribe(&orb, |s: &mut Self, view, event: &VoiceOrbEvent, cx| {
                        let t = |en, zh| s.language.text(en, zh);
                        match event {
                            VoiceOrbEvent::Start => view.update(cx, |v, cx| {
                                v.set_state(VoiceState::Listening, cx);
                                v.set_level(0.35, cx);
                                v.set_transcript(t("Sample voice session", "样例语音会话"), cx);
                            }),
                            VoiceOrbEvent::End => view.update(cx, |v, cx| {
                                v.set_state(VoiceState::Idle, cx);
                                v.set_transcript("", cx);
                            }),
                            VoiceOrbEvent::ToggleMute => {
                                s.muted = !s.muted;
                                view.update(cx, |v, cx| v.set_muted(s.muted, cx));
                            }
                        }
                    }),
                );
                Example::Orb(orb)
            }
            "now-playing" => {
                let cover: ImageSource = Arc::new(Image::from_bytes(
                    ImageFormat::Jpeg,
                    include_bytes!("../../assets/navigation/room-0.jpg").to_vec(),
                ))
                .into();
                let tracks = vec![
                    Track::new(
                        "a",
                        t("Morning light", "晨光"),
                        "Kense",
                        212.,
                        cover.clone(),
                    ),
                    Track::new("b", t("Quiet room", "安静的房间"), "Kense", 185., cover),
                ];
                let player = cx.new(|_| NowPlaying::new(t("Sample player", "样例播放器"), tracks));
                subs.push(
                    cx.subscribe(
                        &player,
                        |_: &mut Self, view, event: &PlayerEvent, cx| match event {
                            PlayerEvent::Play(value) => {
                                view.update(cx, |v, cx| v.set_playing(*value, cx))
                            }
                            PlayerEvent::Track(index) => {
                                view.update(cx, |v, cx| v.set_index(*index, cx))
                            }
                            PlayerEvent::Seek(value) => {
                                view.update(cx, |v, cx| v.set_position(*value, cx))
                            }
                            PlayerEvent::Expanded(_) => {}
                        },
                    ),
                );
                Example::Player(player)
            }
            "voice-recorder" => {
                let notes = vec![
                    VoiceMessage::new(
                        "one",
                        t("Alex", "小林"),
                        "9:38",
                        14.,
                        vec![0.2, 0.5, 0.8, 0.4, 0.7, 0.3, 0.6, 0.2, 0.5, 0.4],
                    ),
                    VoiceMessage::new(
                        "two",
                        t("You", "你"),
                        "9:40",
                        6.,
                        vec![0.4, 0.7, 0.2, 0.5, 0.8, 0.3],
                    )
                    .outgoing(true),
                ];
                let recorder = cx.new(|_| {
                    VoiceRecorder::new(t("Sample recorder", "样例录音")).with_messages(notes)
                });
                subs.push(cx.subscribe(
                    &recorder,
                    |s: &mut Self, view, event: &RecorderEvent, cx| {
                        let t = |en, zh| s.language.text(en, zh);
                        match event {
                            RecorderEvent::Start => {
                                let Some(request) = view.read(cx).request_id() else {
                                    return;
                                };
                                let Some(session) = view.read(cx).session_id() else {
                                    return;
                                };
                                s.recording_id += 1;
                                s.failure_seen = false;
                                view.update(cx, |v, cx| {
                                    v.set_phase_for(request, RecorderPhase::Recording, cx);
                                    v.set_audio_for(
                                        session,
                                        6.,
                                        0.,
                                        vec![0.2, 0.4, 0.7, 0.5, 0.8, 0.3, 0.6, 0.4, 0.2],
                                        false,
                                        cx,
                                    );
                                });
                            }
                            RecorderEvent::Stop => {
                                let Some(request) = view.read(cx).request_id() else {
                                    return;
                                };
                                view.update(cx, |v, cx| {
                                    v.set_phase_for(request, RecorderPhase::Review, cx);
                                });
                            }
                            RecorderEvent::Play(value) => {
                                let Some(session) = view.read(cx).session_id() else {
                                    return;
                                };
                                view.update(cx, |v, cx| {
                                    v.set_audio_for(
                                        session,
                                        6.,
                                        0.,
                                        vec![0.2, 0.4, 0.7, 0.5, 0.8, 0.3, 0.6, 0.4, 0.2],
                                        *value,
                                        cx,
                                    );
                                });
                            }
                            RecorderEvent::Seek(value) => {
                                let Some(session) = view.read(cx).session_id() else {
                                    return;
                                };
                                view.update(cx, |v, cx| {
                                    v.set_audio_for(
                                        session,
                                        6.,
                                        *value,
                                        vec![0.2, 0.4, 0.7, 0.5, 0.8, 0.3, 0.6, 0.4, 0.2],
                                        false,
                                        cx,
                                    );
                                });
                            }
                            RecorderEvent::Send if !s.failure_seen => {
                                let Some(request) = view.read(cx).request_id() else {
                                    return;
                                };
                                s.failure_seen = true;
                                view.update(cx, |v, cx| {
                                    v.set_error_for(
                                        request,
                                        t(
                                            "Sample send failed; your recording is preserved.",
                                            "样例发送失败，录音草稿已保留。",
                                        ),
                                        cx,
                                    );
                                });
                            }
                            RecorderEvent::Send | RecorderEvent::Retry => {
                                let operation = view.read(cx).failed_operation();
                                let Some(request) = view.read(cx).request_id() else {
                                    return;
                                };
                                if *event == RecorderEvent::Retry {
                                    if operation == Some(RecorderOperation::Start) {
                                        view.update(cx, |v, cx| {
                                            v.set_phase_for(request, RecorderPhase::Recording, cx);
                                        });
                                        return;
                                    }
                                    if operation == Some(RecorderOperation::Stop) {
                                        view.update(cx, |v, cx| {
                                            v.set_phase_for(request, RecorderPhase::Review, cx);
                                        });
                                        return;
                                    }
                                }
                                let message_id = format!("sample-{}", s.recording_id);
                                view.update(cx, |v, cx| {
                                    v.append_message_for(
                                        request,
                                        VoiceMessage::new(
                                            message_id,
                                            t("You", "你"),
                                            t("Just now", "刚刚"),
                                            6.,
                                            vec![0.2, 0.4, 0.7, 0.5, 0.8, 0.3, 0.6, 0.4, 0.2],
                                        )
                                        .outgoing(true),
                                        cx,
                                    );
                                    v.set_phase_for(request, RecorderPhase::Idle, cx);
                                });
                            }
                            RecorderEvent::Discard => {}
                            RecorderEvent::PlayMessage(index) => view.update(cx, |v, cx| {
                                let next = if v.playing_message() == Some(*index) {
                                    None
                                } else {
                                    Some(*index)
                                };
                                v.set_message_playing(next, cx);
                            }),
                        }
                    },
                ));
                Example::Recorder(recorder)
            }
            _ => unreachable!(),
        };
        Self {
            example,
            language,
            muted: false,
            failure_seen: false,
            recording_id: 0,
            _subs: subs,
        }
    }
}
impl Render for AudioStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let view: AnyView = match &self.example {
            Example::Orb(v) => v.clone().into(),
            Example::Player(v) => v.clone().into(),
            Example::Recorder(v) => v.clone().into(),
        };
        div()
            .size_full()
            .min_h(px(540.))
            .p_4()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .gap_4()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_sm()
                    .text_color(p.secondary)
                    .child(self.language.text(
                        "Sample audio — no microphone requested",
                        "样例音频，不会申请麦克风",
                    )),
            )
            .child(div().max_w_full().child(view))
    }
}
