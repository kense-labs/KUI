//! Deterministic sample transcript for the streaming reasoning control.
use super::{Demo, DemoAction, demo_chunks};
use gpui::{prelude::*, *};
use kui::{components::reasoning_block::*, primitives::control::UiLanguage};
use std::{cell::Cell, rc::Rc, time::Duration};

type Steps = [(&'static str, &'static str); 4];

fn sample(lang: UiLanguage) -> Steps {
    let t = |en, zh| lang.text(en, zh);
    [
        (
            t("Inspecting the receipt worker", "检查收据任务"),
            t(
                "The worker stores each invoice before sending its receipt. I’m checking which part can run a second time when the same event arrives again.",
                "收据任务先保存账单，再发送邮件。我要核对同一事件再次送达时，哪些步骤会被重复执行。",
            ),
        ),
        (
            t("Comparing the delivery times", "核对投递时间"),
            t(
                "The second delivery arrived while the first receipt was still rendering. Both requests carry the same event ID, even though they reached different workers.",
                "第二次投递发生在第一份收据还在渲染时。两次请求被不同进程接收，但它们携带的事件标识相同。",
            ),
        ),
        (
            t("Following the duplicate", "追踪重复记录"),
            t(
                "Each worker made a fresh dispatch key. The mail queue accepted both jobs because it had no stable identifier that connected them to the original invoice.",
                "两个进程各自生成了发送标识。邮件队列无法据此识别同一张账单，所以两个任务都被接收了。",
            ),
        ),
        (
            t("Preparing the change", "准备修复方案"),
            t(
                "Reserve the event ID in the invoice transaction, acknowledge the delivery, and let the queue render the receipt. A repeated event will reuse the existing record.",
                "在账单事务中记录事件标识，先确认投递，再由队列渲染收据。相同事件再次到达时，就能复用已有记录。",
            ),
        ),
    ]
}
fn stream(
    view: &Entity<ExplanationNote>,
    steps: Steps,
    epoch: Rc<Cell<u64>>,
    cx: &mut Context<Demo>,
) {
    let run = epoch.get().wrapping_add(1);
    epoch.set(run);
    view.update(cx, |s, cx| {
        s.notes.clear();
        s.elapsed = 0;
        s.set_state(NoteState::Live, cx)
    });
    let weak = view.downgrade();
    cx.spawn(async move |_, cx| {
        let batches = steps
            .into_iter()
            .flat_map(|(heading, body)| {
                demo_chunks(body)
                    .into_iter()
                    .enumerate()
                    .map(move |(i, text)| (heading, i == 0, text))
            })
            .collect::<Vec<_>>();
        let count = batches.len() as u64;
        let mut clock = 0u64;
        let mut next = 0;
        loop {
            // Sleep without drawing. Hidden and background previews accumulate no playback time.
            let target = if next < batches.len() {
                300 + 3650 * (next as u64) / count
            } else if next == batches.len() {
                4100
            } else {
                10600
            };
            let wait = (target - clock).min(180);
            cx.background_executor()
                .timer(Duration::from_millis(wait))
                .await;
            if epoch.get() != run {
                break;
            }
            let Ok(advance) = weak.update(cx, |s, cx| {
                let visible = cx
                    .try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)());
                s.set_visible(visible, cx);
                visible && !(s.state == NoteState::Finished && s.is_expanded())
            }) else {
                break;
            };
            if !advance {
                continue;
            }
            clock += wait;
            if clock < target {
                continue;
            }
            if let Some((heading, first, text)) = batches.get(next) {
                let _ = weak.update(cx, |s, cx| {
                    if *first {
                        s.notes.push(((*heading).into(), text.clone()));
                    } else if let Some((_, b)) = s.notes.last_mut() {
                        b.clone_from(text);
                    }
                    s.elapsed = (clock / 1000) as u32;
                    cx.notify();
                });
                next += 1;
            } else if next == batches.len() {
                let _ = weak.update(cx, |s, cx| {
                    s.elapsed = 4;
                    s.set_state(NoteState::Finished, cx);
                });
                next += 1;
            } else {
                let _ = weak.update(cx, |s, cx| {
                    s.notes.clear();
                    s.elapsed = 0;
                    s.set_state(NoteState::Live, cx);
                });
                next = 0;
                clock = 0;
            }
        }
    })
    .detach();
}
pub(super) fn demo(
    id: &str,
    l: UiLanguage,
    options: &mut Vec<(&'static str, DemoAction)>,
    w: &mut Window,
    cx: &mut Context<Demo>,
) -> AnyView {
    let view = cx.new(|_| ExplanationNote::new(id.to_owned(), vec![]));
    let steps = sample(l);
    let epoch = Rc::new(Cell::new(0u64));
    for (mode, title) in [
        (NoteState::Live, l.text("Live", "进行中")),
        (NoteState::Finished, l.text("Finished", "已完成")),
        (NoteState::Interrupted, l.text("Interrupted", "已中断")),
    ] {
        let view = view.clone();
        let epoch = epoch.clone();
        options.push((
            title,
            Rc::new(move |_, cx| {
                if mode == NoteState::Live {
                    stream(&view, steps, epoch.clone(), cx);
                    return;
                }
                epoch.set(epoch.get().wrapping_add(1));
                view.update(cx, |s, cx| {
                    s.notes = steps
                        .iter()
                        .take(if mode == NoteState::Finished { 4 } else { 2 })
                        .map(|(h, b)| (h.to_string(), b.to_string()))
                        .collect();
                    s.elapsed = if mode == NoteState::Finished { 7 } else { 4 };
                    s.set_state(mode, cx);
                    if mode == NoteState::Finished {
                        s.set_expanded(true, cx)
                    } else {
                        s.error = Some(
                            l.text(
                                "The connection was lost while receiving the next step.",
                                "接收下一步内容时连接中断。",
                            )
                            .into(),
                        );
                    }
                })
            }),
        ));
    }
    cx.subscribe_in(&view, w, {
        let epoch = epoch.clone();
        move |_, _, _: &StopRequested, _, _| epoch.set(epoch.get().wrapping_add(1))
    })
    .detach();
    cx.subscribe_in(&view, w, {
        let epoch = epoch.clone();
        move |d, view, _: &RetryRequested, _, cx| {
            d.selected = 0;
            stream(view, steps, epoch.clone(), cx);
            cx.notify()
        }
    })
    .detach();
    stream(&view, steps, epoch, cx);
    view.into()
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::super::{
        build,
        tests::{click, frame},
    };
    use super::{ExplanationNote, NoteState, parse_transcript};
    use gpui::TestAppContext;

    #[test]
    fn disclosure_stop_and_retry_preserve_the_current_stream() {
        let mut app = TestAppContext::single();
        app.update(kui::foundation::init);
        let (demo, w) = app.add_window_view(|w, cx| build("reasoning-block", w, cx));
        let view = w.update(|_, cx| {
            demo.read(cx)
                .view
                .clone()
                .downcast::<ExplanationNote>()
                .unwrap()
        });
        frame(w, 800);
        w.update(|_, cx| {
            let s = view.read(cx);
            assert_eq!(s.state, NoteState::Live);
            assert!(!s.notes.is_empty());
            assert!(!s.is_expanded());
        });
        click(w, "Stop");
        assert!(
            !w.update(|_, cx| view.read(cx).is_expanded()),
            "Stop must not also activate the disclosure"
        );
        let stopped = w.update(|_, cx| view.read(cx).notes.clone());
        frame(w, 1000);
        w.update(|_, cx| {
            assert_eq!(view.read(cx).state, NoteState::Stopped);
            assert_eq!(view.read(cx).notes, stopped)
        });
        click(w, "Finished");
        frame(w, 330);
        w.update(|_, cx| {
            assert!(view.read(cx).is_expanded());
            assert_eq!(view.read(cx).notes.len(), 4)
        });
        assert!(w.debug_bounds("Copy reasoning").is_some());
        click(w, "Thought for 7s");
        frame(w, 65);
        click(w, "Thought for 7s");
        frame(w, 370);
        assert!(w.update(|_, cx| view.read(cx).is_expanded()));
        click(w, "Interrupted");
        frame(w, 340);
        click(w, "Retry");
        frame(w, 600);
        w.update(|_, cx| {
            assert_eq!(view.read(cx).state, NoteState::Live);
            assert!(!view.read(cx).is_expanded());
            assert!(view.read(cx).notes.len() < 4);
        });
    }
    #[test]
    fn natural_completion_stays_open_until_the_reader_folds_it() {
        thread_local! {static VISIBLE: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };}
        let mut app = TestAppContext::single();
        app.update(|cx| {
            kui::foundation::init(cx);
            cx.set_global(crate::StoryBridge {
                visible: || VISIBLE.get(),
                min_height: || 0.,
                frame: |_, _| {},
                leave: |_| {},
                theme_target: |_, _| {},
            });
        });
        let (demo, w) = app.add_window_view(|w, cx| build("reasoning-block", w, cx));
        let view = w.update(|_, cx| {
            demo.read(cx)
                .view
                .clone()
                .downcast::<ExplanationNote>()
                .unwrap()
        });
        for _ in 0..25 {
            frame(w, 20);
        }
        let snapshot = w.update(|_, cx| view.read(cx).notes.clone());
        assert!(!snapshot.is_empty());
        VISIBLE.set(false);
        for _ in 0..48 {
            frame(w, 180);
        }
        assert_eq!(w.update(|_, cx| view.read(cx).notes.clone()), snapshot);
        VISIBLE.set(true);
        for _ in 0..260 {
            frame(w, 20);
            if w.update(|_, cx| view.read(cx).state) == NoteState::Finished {
                break;
            }
        }
        w.update(|_, cx| {
            let s = view.read(cx);
            assert_eq!(s.state, NoteState::Finished);
            assert_eq!(s.elapsed, 4);
            assert!(s.is_expanded());
            assert_eq!(s.notes.len(), 4);
        });
        for _ in 0..340 {
            frame(w, 20);
        }
        assert!(w.debug_bounds("Copy reasoning").is_some());
        click(w, "Thought for 4s");
        w.update(|_, cx| view.update(cx, |s, cx| s.set_state(NoteState::Finished, cx)));
        assert!(!w.update(|_, cx| view.read(cx).is_expanded()));
        for _ in 0..340 {
            frame(w, 20);
        }
        assert_eq!(w.update(|_, cx| view.read(cx).state), NoteState::Live);
    }
    #[test]
    fn only_overflowing_live_text_has_a_top_fade() {
        for (mode, reduced) in [kui::ThemeMode::Dark, kui::ThemeMode::Light]
            .into_iter()
            .flat_map(|m| [false, true].map(|r| (m, r)))
        {
            let mut app = TestAppContext::single();
            app.update(|cx| {
                kui::foundation::init(cx);
                let theme = kui::Theme::global_mut(cx);
                theme.set_mode(mode);
                theme.reduced_motion = reduced;
            });
            let (view, w) = app.add_window_view(|_, _| {
                ExplanationNote::new("notes", vec![("".into(), "Short.".into())])
            });
            frame(w, 20);
            frame(w, 20);
            assert!(w.debug_bounds("reasoning-live-fade").is_none());
            w.update(|_, cx| {
                view.update(cx, |s, cx| {
                    s.set_transcript("A line.\nAnother line.\nThe latest line.", cx)
                })
            });
            for _ in 0..5 {
                frame(w, 20);
            }
            let preview = w
                .debug_bounds("reasoning-live-preview")
                .expect("two-line preview");
            let fade = w
                .debug_bounds("reasoning-live-fade")
                .expect("overflow fades at the top");
            assert_eq!(fade.origin, preview.origin);
            assert_eq!(fade.size.width, preview.size.width);
            assert_eq!(fade.size.height, gpui::px(16.));
            assert_eq!(preview.size.height, gpui::px(40.));
            w.update(|_, cx| view.update(cx, |s, cx| s.set_expanded(true, cx)));
            frame(w, 1300);
            frame(w, 20);
            assert!(w.debug_bounds("reasoning-live-fade").is_none());
        }
    }
    #[test]
    fn parsing_a_partial_heading_keeps_markup_out_of_the_live_preview() {
        assert_eq!(
            parse_transcript("## One\nFirst\n\n**Tw"),
            vec![("One".into(), "First".into())]
        );
        assert_eq!(
            parse_transcript("## One\nFirst\n**Two**\nSecond"),
            vec![
                ("One".into(), "First".into()),
                ("Two".into(), "Second".into())
            ]
        );
    }
}
