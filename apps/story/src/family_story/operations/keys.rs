//! In-memory credential adapter for the component gallery.
use super::Demo;
use gpui::{prelude::*, *};
use kui::components::provider_keys::*;
use kui::primitives::{clipboard, ops_surface::tr};
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

pub(super) fn demo(w: &mut Window, cx: &mut Context<Demo>) -> AnyView {
    let entries = [
        (
            "anthropic",
            "Anthropic",
            "sk-ant-",
            "4f2a",
            tr(cx, "Used 2 min ago", "2 分钟前使用"),
        ),
        (
            "openai",
            "OpenAI",
            "sk-proj-",
            "9c1e",
            tr(cx, "Used 14 min ago", "14 分钟前使用"),
        ),
        (
            "google",
            "Google AI",
            "AIza",
            "Qk7w",
            tr(cx, "Used yesterday", "昨天使用"),
        ),
        (
            "mistral",
            "Mistral",
            "",
            "h7Qd",
            tr(cx, "Rejected 3 days ago", "3 天前被拒绝"),
        ),
        ("xai", "xAI", "xai-", "", ""),
    ];
    let providers = entries
        .iter()
        .map(|(id, name, prefix, last, when)| Provider {
            id: (*id).into(),
            name: (*name).into(),
            prefix: (*prefix).into(),
            masked: if last.is_empty() {
                String::new()
            } else {
                format!("{prefix}••••{last}")
            },
            valid: *id != "mistral" && !last.is_empty(),
            last_used: (*when).into(),
        })
        .collect();
    let vault = Rc::new(RefCell::new(
        entries
            .into_iter()
            .filter(|e| !e.3.is_empty())
            .map(|(id, _, prefix, last, _)| {
                (id.to_string(), format!("{prefix}sample-workspace-{last}"))
            })
            .collect::<HashMap<_, _>>(),
    ));
    let panel = cx.new(|cx| {
        CredentialPanel::new("provider-keys", providers, w, cx)
            .with_max_width(522.)
            .with_demo_inputs()
    });
    let keys = vault.clone();
    cx.subscribe_in(&panel, w, move |_, e, a: &KeyAction, w, cx| match a {
        KeyAction::Test {
            provider,
            secret,
            ticket,
        } => {
            let (id, key, n) = (provider.clone(), secret.clone(), *ticket);
            let store = keys.clone();
            let weak = e.downgrade();
            cx.spawn_in(w, async move |_, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(650))
                    .await;
                let _ = cx.update(|w, cx| {
                    let _ = weak.update(cx, |s, cx| {
                        if !s.awaiting(n) {
                            return;
                        }
                        let mask = if key.to_ascii_lowercase().ends_with("invalid") {
                            None
                        } else {
                            let prefix = s
                                .providers
                                .iter()
                                .find(|p| p.id == id)
                                .map(|p| p.prefix.as_str())
                                .unwrap_or_default();
                            let last = key
                                .chars()
                                .skip(key.chars().count().saturating_sub(4))
                                .collect::<String>();
                            store.borrow_mut().insert(id, key);
                            Some(format!("{prefix}••••{last}"))
                        };
                        s.tested(n, mask, w, cx)
                    });
                });
            })
            .detach()
        }
        KeyAction::Reveal { provider, ticket } => {
            if let Some(k) = keys.borrow().get(provider) {
                e.update(cx, |s, cx| {
                    s.revealed(*ticket, provider.clone(), k.clone(), cx)
                })
            }
        }
        KeyAction::Copy(provider) => {
            if let Some(key) = keys.borrow().get(provider) {
                let write = clipboard::write(key, cx);
                let weak = e.downgrade();
                let id = provider.clone();
                cx.spawn(async move |_, cx| {
                    let success = write.await;
                    let _ = weak.update(cx, |s, cx| s.copy_finished(id, success, cx));
                })
                .detach()
            }
        }
        KeyAction::Remove(id) => {
            keys.borrow_mut().remove(id);
        }
        KeyAction::Restore(_) => {}
    })
    .detach();
    cx.subscribe_in(&panel, w, move |_, e, a: &RetestKey, _, cx| {
        let n = a.ticket;
        let ok = vault
            .borrow()
            .get(&a.provider)
            .is_some_and(|key| key != "sample-workspace-h7Qd");
        super::later(e, 650, move |s, cx| s.retested(n, ok, cx), cx);
    })
    .detach();
    panel.into()
}
