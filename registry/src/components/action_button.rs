//! Commit an action and acknowledge its result.
use crate::primitives::{
    commit::{ActionCore, Commit, CommitControl, finish, render_action},
    family::Tone,
    glyph::Glyph,
};
use gpui::{prelude::*, *};
pub struct ActionButton {
    core: ActionCore,
}
impl EventEmitter<Commit> for ActionButton {}
impl CommitControl for ActionButton {
    fn core(&mut self) -> &mut ActionCore {
        &mut self.core
    }
}
impl ActionButton {
    pub fn new<S: Into<SharedString>>(id: impl Into<ElementId>, labels: [S; 4]) -> Self {
        use Glyph::*;
        let mut core = ActionCore::new(
            id.into(),
            labels.map(Into::into),
            [Arrow, Spinner, Check, Alert],
        );
        core.trailing = true;
        Self { core }
    }
    pub fn with_width(mut self, width: f32) -> Self {
        self.core.set_widths([width; 4]);
        self
    }
    pub fn with_phase_widths(mut self, widths: [f32; 4]) -> Self {
        self.core.set_widths(widths);
        self
    }
    pub fn with_tone(mut self, tone: Tone) -> Self {
        self.core.tone = tone;
        self
    }
    pub fn with_icons(mut self, marks: [Glyph; 4]) -> Self {
        self.core.set_marks(marks);
        self
    }
    pub fn complete(&mut self, ticket: u64, ok: bool, cx: &mut Context<Self>) {
        finish(self, ticket, ok, cx)
    }
}
impl Render for ActionButton {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_action(self, w, cx)
    }
}
