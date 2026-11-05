//! Sample speech and dictation interactions. No microphone or network is used.
use gpui::{prelude::*, *};
use kui::{
    components::{
        dictation_button::{DictationButton, DictationEvent, DictationPhase, InsertionTarget},
        speech_player::{SpeechEvent, SpeechPhase, SpeechPlayer, SpeechVoice},
    },
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
    },
};
use std::time::Duration;
pub const IDS: &[&str] = &["dictation-button", "speech-player"];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Demo::new(id, cx)).into())
}
enum Example {
    Dictation(Entity<DictationButton>),
    Speech(Entity<SpeechPlayer>),
}
struct Demo {
    example: Example,
    language: UiLanguage,
    draft: String,
    original_anchor: usize,
    dictation_session: Option<u64>,
    _sub: Subscription,
}
impl Demo {
    fn new(id: &str, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let t = |en, zh| language.text(en, zh);
        let (example, sub) = match id {
            "dictation-button" => {
                let control = cx.new(|_| DictationButton::new());
                let sub = cx.subscribe(
                    &control,
                    |s: &mut Self, view, event: &DictationEvent, cx| match event {
                        DictationEvent::Start | DictationEvent::Retry => {
                            let Some(session) = view.read(cx).session_id() else {
                                return;
                            };
                            s.dictation_session = Some(session);
                            s.original_anchor = s.draft.len();
                            view.update(cx, |v, cx| {
                                v.set_phase_for(session, DictationPhase::Listening, cx);
                                v.set_transcript_for(
                                    session,
                                    s.language.text("A thoughtful interface", "让界面更好用"),
                                    s.language.text(" feels natural", "，自然顺手"),
                                    cx,
                                );
                                v.set_level_for(session, 0.45, 3, cx);
                            });
                        }
                        DictationEvent::Finalize => {
                            let Some(session) = s.dictation_session else {
                                return;
                            };
                            let text = s.language.text(
                                "A thoughtful interface feels natural",
                                "让界面更好用，自然顺手",
                            );
                            view.update(cx, |v, cx| {
                                v.finish_recognition_for(session, text, cx);
                            });
                        }
                        DictationEvent::Cancel => {
                            s.dictation_session = None;
                        }
                        DictationEvent::Insert { text, target } => {
                            let Some(session) = s.dictation_session else {
                                return;
                            };
                            if view.read(cx).session_id() != Some(session) {
                                return;
                            }
                            let Some(insertion) = view.read(cx).insertion_id() else {
                                return;
                            };
                            let position = match target {
                                InsertionTarget::Original => s.original_anchor.min(s.draft.len()),
                                InsertionTarget::Current => 0,
                            };
                            s.draft.insert_str(position, text);
                            view.update(cx, |v, cx| {
                                v.confirm_insert_for(session, insertion, cx);
                            });
                            s.dictation_session = None;
                            cx.notify();
                        }
                    },
                );
                (Example::Dictation(control), sub)
            }
            "speech-player" => {
                let voices = vec![
                    SpeechVoice::new("one", t("Ava", "小安")),
                    SpeechVoice::new("two", t("Leo", "小乐")),
                ];
                let control = cx.new(|_| {
                    SpeechPlayer::new(
                        t("Sample read aloud", "样例朗读"),
                        t(
                            "A thoughtful interface makes room for what matters.",
                            "好的界面，让重要的事情自然发生。",
                        ),
                    )
                    .with_voices(voices)
                });
                let sub =
                    cx.subscribe(
                        &control,
                        |_: &mut Self, view, event: &SpeechEvent, cx| match event {
                            SpeechEvent::Start | SpeechEvent::Retry | SpeechEvent::Resume => {
                                let Some(session) = view.read(cx).session_id() else {
                                    return;
                                };
                                let operation = view.read(cx).operation_id();
                                let weak = view.downgrade();
                                cx.spawn(async move |_, cx| {
                                    cx.background_executor()
                                        .timer(Duration::from_millis(700))
                                        .await;
                                    let _ = weak.update(cx, |v, cx| {
                                        if v.set_phase_for_operation(
                                            session,
                                            operation,
                                            SpeechPhase::Playing,
                                            cx,
                                        ) {
                                            v.set_progress_for(session, Some(0), 3., 18., cx);
                                        }
                                    });
                                })
                                .detach();
                            }
                            SpeechEvent::Pause => {
                                let Some(session) = view.read(cx).session_id() else {
                                    return;
                                };
                                let operation = view.read(cx).operation_id();
                                view.update(cx, |v, cx| {
                                    v.set_phase_for_operation(
                                        session,
                                        operation,
                                        SpeechPhase::Paused,
                                        cx,
                                    );
                                });
                            }
                            SpeechEvent::Stop => {}
                            SpeechEvent::Seek(position) => {
                                let Some(session) = view.read(cx).session_id() else {
                                    return;
                                };
                                view.update(cx, |v, cx| {
                                    v.set_progress_for(session, Some(0), *position, 18., cx);
                                });
                            }
                            SpeechEvent::Speed(speed) => {
                                let Some(session) = view.read(cx).session_id() else {
                                    return;
                                };
                                view.update(cx, |v, cx| {
                                    v.set_speed_for(session, *speed, cx);
                                });
                            }
                            SpeechEvent::Voice(id) => {
                                let Some(session) = view.read(cx).session_id() else {
                                    return;
                                };
                                view.update(cx, |v, cx| {
                                    v.set_voice_for(session, id, cx);
                                });
                            }
                        },
                    );
                (Example::Speech(control), sub)
            }
            _ => unreachable!(),
        };
        Self {
            example,
            language,
            draft: t("Notes: ", "笔记：").to_string(),
            original_anchor: 0,
            dictation_session: None,
            _sub: sub,
        }
    }
}
impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let view: AnyView = match &self.example {
            Example::Dictation(v) => v.clone().into(),
            Example::Speech(v) => v.clone().into(),
        };
        let mut root = div()
            .size_full()
            .min_h(px(480.))
            .p_5()
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
                        "Sample only — no microphone or network",
                        "样例演示，不会申请麦克风或连接网络",
                    )),
            );
        if let Example::Dictation(control) = &self.example {
            let control = control.clone();
            root = root
                .child(
                    div()
                        .w(px(360.))
                        .max_w_full()
                        .p_3()
                        .rounded(kui::foundation::surface_radius(cx))
                        .border_1()
                        .border_color(p.line)
                        .child(if self.draft.is_empty() {
                            self.language.text("Draft", "草稿").to_string()
                        } else {
                            self.draft.clone()
                        }),
                )
                .child(
                    face(
                        "move-caret",
                        self.language
                            .text("Move insertion position", "移动插入位置"),
                        36.,
                        kui::foundation::surface_radius(cx).into(),
                        Tone::Surface,
                        cx,
                    )
                    .px_3()
                    .child(
                        self.language
                            .text("Move insertion position", "移动插入位置"),
                    )
                    .on_click(cx.listener(move |_, _, _, cx| {
                        control.update(cx, |v, cx| {
                            if let Some(session) = v.session_id() {
                                v.set_anchor_changed_for(session, true, cx);
                            }
                        })
                    })),
                );
        }
        root.child(div().max_w_full().child(view))
    }
}
