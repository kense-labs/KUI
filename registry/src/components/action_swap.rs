//! Switch between two labels after the host confirms a change.
use crate::primitives::{
    commit::{ActionCore, Commit, CommitControl, finish, render_action},
    family::Tone,
    glyph::Glyph,
};
use gpui::{prelude::*, *};
pub struct ActionSwap {
    core: ActionCore,
}
impl EventEmitter<Commit> for ActionSwap {}
impl CommitControl for ActionSwap {
    fn core(&mut self) -> &mut ActionCore {
        &mut self.core
    }
}
impl ActionSwap {
    pub fn new<S: Into<SharedString>>(id: impl Into<ElementId>, labels: [S; 4]) -> Self {
        use Glyph::*;
        let mut core = ActionCore::new(
            id.into(),
            labels.map(Into::into),
            [Plus, Spinner, Check, Alert],
        );
        core.reset = None;

        core.momentary = false;
        core.height = 36.;
        core.trailing = false;
        Self { core }
    }
    pub fn with_reset(mut self, delay: std::time::Duration) -> Self {
        self.core.reset = Some(delay);
        self.core.momentary = true;
        self
    }
    pub fn with_width(mut self, width: f32) -> Self {
        self.core.set_widths([width; 4]);
        self
    }
    pub fn with_phase_widths(mut self, widths: [f32; 4]) -> Self {
        self.core.set_widths(widths);
        self
    }
    /// Tone to settle on after the host acknowledges this action.
    pub fn with_done_tone(mut self, tone: Tone) -> Self {
        self.core.done_tone = tone;
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
impl Render for ActionSwap {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_action(self, w, cx)
    }
}
