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
        glyph::{Glyph, icon},
    },
};
use std::{cell::Cell, rc::Rc, time::Duration};
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
    dictation_blocked: bool,
    speech_fail_next: bool,
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
                            if s.dictation_blocked {
                                view.update(cx, |v, cx| {
                                    v.set_error_for(
                                        session,
                                        s.language.text(
                                            "Microphone is blocked in this sample.",
                                            "此样例中的麦克风权限已禁用。",
                                        ),
                                        cx,
                                    )
                                });
                                return;
                            }
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
                            let spacer = if position > 0
                                && s.draft[..position].ends_with(|c: char| !c.is_whitespace())
                                && s.language != UiLanguage::Chinese
                            {
                                " "
                            } else {
                                ""
                            };
                            s.draft.insert_str(position, &format!("{spacer}{text}"));
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
                            "Your flight to Lisbon leaves Thursday at 7:40 from Zurich, gate A54. Check-in closes 40 minutes before departure, so plan to reach the airport by 6:15. Lisbon looks warm, with a light breeze off the river. Your hotel holds the room until midnight.",
                            "您飞往里斯本的航班将于周四 7:40 从苏黎世起飞，登机口 A54。值机在起飞前 40 分钟结束，建议 6:15 到机场。里斯本天气温暖，河边有微风。酒店将为您保留房间至午夜。",
                        ),
                    )
                    .with_voices(voices)
                    .with_model("Claude Sonnet 5.5")
                    .with_transport(22.)
                });
                let position = Rc::new(Cell::new(0f32));
                let speed = Rc::new(Cell::new(1f32));
                let sub = cx.subscribe(
                    &control,
                    move |s: &mut Self, view, event: &SpeechEvent, cx| match event {
                        SpeechEvent::Start | SpeechEvent::Retry | SpeechEvent::Resume => {
                            let Some(session) = view.read(cx).session_id() else {
                                return;
                            };
                            let operation = view.read(cx).operation_id();
                            if *event != SpeechEvent::Resume || position.get() >= 22. {
                                position.set(0.);
                            }
                            let position = position.clone();
                            let speed = speed.clone();
                            let fail = std::mem::take(&mut s.speech_fail_next);
                            let language = s.language;
                            if fail {
                                cx.notify();
                            }
                            let weak = view.downgrade();
                            cx.spawn(async move |_, cx| {
                                cx.background_executor()
                                    .timer(Duration::from_millis(260))
                                    .await;
                                let start = weak.update(cx, |v, cx| {
                                    if v.operation_id() != operation
                                        || v.session_id() != Some(session)
                                    {
                                        return false;
                                    }
                                    if fail {
                                        v.set_error_for(
                                            session,
                                            language.text(
                                                "Sample speech request failed. Try again.",
                                                "样例朗读请求失败，请重试。",
                                            ),
                                            cx,
                                        );
                                        return false;
                                    }
                                    v.set_phase_for_operation(
                                        session,
                                        operation,
                                        SpeechPhase::Playing,
                                        cx,
                                    )
                                });
                                if !start.unwrap_or(false) {
                                    return;
                                }
                                loop {
                                    cx.background_executor()
                                        .timer(Duration::from_millis(150))
                                        .await;
                                    let active = weak.update(cx, |v, cx| {
                                        if v.operation_id() != operation
                                            || v.session_id() != Some(session)
                                        {
                                            return false;
                                        }
                                        if cx
                                            .try_global::<crate::StoryBridge>()
                                            .is_some_and(|bridge| !(bridge.visible)())
                                        {
                                            return true;
                                        }
                                        let next = (position.get() + 0.15 * speed.get()).min(22.);
                                        position.set(next);
                                        v.set_progress_for(session, None, next, 22., cx);
                                        if next == 22. {
                                            v.set_phase_for_operation(
                                                session,
                                                operation,
                                                SpeechPhase::Finished,
                                                cx,
                                            );
                                        }
                                        next < 22.
                                    });
                                    if !active.unwrap_or(false) {
                                        break;
                                    }
                                }
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
                        SpeechEvent::Stop => {
                            position.set(0.);
                            view.update(cx, |v, cx| v.set_progress(None, 0., 22., cx));
                        }
                        SpeechEvent::Seek(next) => {
                            let Some(session) = view.read(cx).session_id() else {
                                return;
                            };
                            view.update(cx, |v, cx| {
                                position.set(*next);
                                v.set_progress_for(session, None, *next, 22., cx);
                            });
                        }
                        SpeechEvent::Speed(next) => {
                            speed.set(*next);
                            view.update(cx, |v, cx| {
                                v.set_speed(*next, cx);
                            });
                        }
                        SpeechEvent::Voice(id) => {
                            view.update(cx, |v, cx| {
                                v.set_voice(id, cx);
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
            draft: String::new(),
            original_anchor: 0,
            dictation_session: None,
            dictation_blocked: false,
            speech_fail_next: false,
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
        let root = div()
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
            let lang = self.language;
            let move_label = lang.text("Move insertion position", "移动插入位置");
            let mic = div()
                .flex()
                .gap(px(4.))
                .children([false, true].map(|blocked| {
                    let label = if blocked {
                        lang.text("Mic blocked", "麦克风禁用")
                    } else {
                        lang.text("Mic allowed", "麦克风可用")
                    };
                    face(
                        ("dictation-permission", usize::from(blocked)),
                        label,
                        34.,
                        kui::foundation::surface_radius(cx).into(),
                        if self.dictation_blocked == blocked {
                            Tone::Surface
                        } else {
                            Tone::Clear
                        },
                        cx,
                    )
                    .px_3()
                    .aria_toggled((self.dictation_blocked == blocked).into())
                    .child(label)
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.dictation_blocked = blocked;
                        cx.notify();
                    }))
                }));
            return root
                .child(
                    div()
                        .w(px(490.))
                        .max_w_full()
                        .p(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .bg(p.panel)
                        .rounded(kui::foundation::surface_radius(cx))
                        .border_1()
                        .border_color(p.line)
                        .child(
                            div()
                                .min_h(px(32.))
                                .px(px(4.))
                                .text_color(if self.draft.is_empty() {
                                    p.secondary
                                } else {
                                    p.ink
                                })
                                .child(if self.draft.is_empty() {
                                    lang.text("Draft a reply to Priya", "起草给 Priya 的回复")
                                        .to_string()
                                } else {
                                    self.draft.clone()
                                }),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_end()
                                .justify_between()
                                .gap(px(4.))
                                .child(
                                    face(
                                        "move-caret",
                                        move_label,
                                        32.,
                                        kui::foundation::surface_radius(cx).into(),
                                        Tone::Clear,
                                        cx,
                                    )
                                    .px_2()
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(move_label)
                                    .on_click(cx.listener(
                                        move |_, _, _, cx| {
                                            control.update(cx, |v, cx| {
                                                if let Some(session) = v.session_id() {
                                                    v.set_anchor_changed_for(session, true, cx);
                                                }
                                            })
                                        },
                                    )),
                                )
                                .child(
                                    div().flex().items_end().gap(px(6.)).child(view).child(
                                        face(
                                            "sample-draft-clear",
                                            lang.text("Clear sample draft", "清除样例草稿"),
                                            32.,
                                            kui::foundation::surface_radius(cx).into(),
                                            Tone::Clear,
                                            cx,
                                        )
                                        .size(px(32.))
                                        .child(icon(Glyph::Close, 14., p.secondary))
                                        .on_click(
                                            cx.listener(|s, _, _, cx| {
                                                if s.dictation_session.is_none() {
                                                    s.draft.clear();
                                                    cx.notify();
                                                }
                                            }),
                                        ),
                                    ),
                                ),
                        ),
                )
                .child(mic);
        }
        root.child(div().max_w_full().child(view)).when(
            matches!(self.example, Example::Speech(_)),
            |root| {
                let label = self
                    .language
                    .text("Fail the next speech request", "模拟下次朗读失败");
                root.child(
                    face(
                        "speech-fail-next",
                        label,
                        32.,
                        kui::foundation::surface_radius(cx).into(),
                        Tone::Clear,
                        cx,
                    )
                    .aria_toggled(self.speech_fail_next.into())
                    .child(if self.speech_fail_next { "☑" } else { "☐" })
                    .child(label)
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.speech_fail_next = !s.speech_fail_next;
                        cx.notify();
                    })),
                )
            },
        )
    }
}
