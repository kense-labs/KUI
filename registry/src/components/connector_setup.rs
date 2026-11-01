//! Connect an external service and track the setup state.
use crate::primitives::family::Paint;
use crate::primitives::ops_surface::*;
use gpui::{prelude::*, *};
#[derive(Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Offline,
    Authorizing,
    Syncing(f32),
    Connected,
    Failed,
}
pub struct Service {
    pub id: String,
    pub name: String,
    pub description: String,
    pub scopes: Vec<String>,
    pub state: ConnectionState,
}
pub enum ConnectionAction {
    Connect { service: String, ticket: u64 },
    Disconnect(String),
}
pub struct ConnectionWizard {
    id: SharedString,
    pub services: Vec<Service>,
    open: Option<usize>,
    confirm: Option<usize>,
    tickets: std::collections::HashMap<String, u64>,
    serial: u64,
}
impl EventEmitter<ConnectionAction> for ConnectionWizard {}
impl ConnectionWizard {
    pub fn new(id: impl Into<SharedString>, services: Vec<Service>) -> Self {
        Self {
            id: id.into(),
            services,
            open: None,
            confirm: None,
            tickets: Default::default(),
            serial: 0,
        }
    }
    fn connect(&mut self, i: usize, cx: &mut Context<Self>) {
        let s = &mut self.services[i];
        if !matches!(s.state, ConnectionState::Offline | ConnectionState::Failed) {
            return;
        }
        self.serial += 1;
        self.tickets.insert(s.id.clone(), self.serial);
        s.state = ConnectionState::Authorizing;
        cx.emit(ConnectionAction::Connect {
            service: s.id.clone(),
            ticket: self.serial,
        });
        cx.notify()
    }
    pub fn progress(
        &mut self,
        id: &str,
        ticket: u64,
        state: ConnectionState,
        cx: &mut Context<Self>,
    ) {
        if self.tickets.get(id) != Some(&ticket) {
            return;
        }
        if let Some(s) = self.services.iter_mut().find(|s| s.id == id) {
            s.state = state
        }
        if matches!(
            state,
            ConnectionState::Connected | ConnectionState::Offline | ConnectionState::Failed
        ) {
            self.tickets.remove(id);
        }
        cx.notify()
    }
}
impl Render for ConnectionWizard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let n = self
            .services
            .iter()
            .filter(|s| s.state == ConnectionState::Connected)
            .count();
        let mut r = panel(&self.id, 522., cx)
            .child(title(tr(cx, "Connect your tools", "连接你的工具")))
            .child(note(
                format!(
                    "{n} / {} · {}",
                    self.services.len(),
                    tr(cx, "connected", "已连接")
                ),
                cx,
            ))
            .child(bar(
                n as f32 / self.services.len().max(1) as f32,
                p.success,
                cx,
            ));
        for (i, s) in self.services.iter().enumerate() {
            let label = match s.state {
                ConnectionState::Offline => tr(cx, "Connect", "连接"),
                ConnectionState::Connected => tr(cx, "Connected", "已连接"),
                ConnectionState::Authorizing => tr(cx, "Authorizing…", "授权中…"),
                ConnectionState::Syncing(_) => tr(cx, "Syncing…", "同步中…"),
                ConnectionState::Failed => tr(cx, "Retry", "重试"),
            };
            r = r.child(divider(cx)).child(
                row()
                    .child(avatar(&s.name, i, cx))
                    .child(
                        column()
                            .flex_1()
                            .gap_0()
                            .child(title(s.name.clone()))
                            .child(note(s.description.clone(), cx)),
                    )
                    .child(btn(("connect", i), label, false, cx, move |v, _, cx| {
                        if v.services[i].state == ConnectionState::Connected {
                            v.open = if v.open == Some(i) { None } else { Some(i) };
                            v.confirm = None;
                            cx.notify()
                        } else {
                            v.connect(i, cx)
                        }
                    })),
            );
            if let ConnectionState::Syncing(n) = s.state {
                r = r.child(
                    bar(
                        animated_value(("sync-progress", i), n, 0.16, w, cx),
                        p.success,
                        cx,
                    )
                    .h(px(2.)),
                )
            }
            let details = Fold::get(
                ("service-details", i),
                self.open == Some(i) && s.state == ConnectionState::Connected,
                67.,
                0.24,
                w,
                cx,
            );
            if details.mounted() {
                let content = column().child(note(s.scopes.join(" · "), cx)).child(
                    row()
                        .child(btn(
                            ("disconnect", i),
                            if self.confirm == Some(i) {
                                tr(cx, "Confirm disconnect", "确认断开")
                            } else {
                                tr(cx, "Disconnect", "断开连接")
                            },
                            false,
                            cx,
                            move |v, _, cx| {
                                if v.confirm == Some(i) {
                                    let s = &mut v.services[i];
                                    s.state = ConnectionState::Offline;
                                    cx.emit(ConnectionAction::Disconnect(s.id.clone()));
                                    v.open = None;
                                    v.confirm = None
                                } else {
                                    v.confirm = Some(i)
                                }
                                cx.notify()
                            },
                        ))
                        .when(self.confirm == Some(i), |el| {
                            el.child(btn(
                                "cancel",
                                tr(cx, "Cancel", "取消"),
                                false,
                                cx,
                                |v, _, cx| {
                                    v.confirm = None;
                                    cx.notify()
                                },
                            ))
                        }),
                );
                r = r.child(details.wrap(12., content));
            }
        }
        r
    }
}
