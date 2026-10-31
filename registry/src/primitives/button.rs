use gpui::ElementId;

/// Behavior contract for a KUI button-like control (no default chrome).
#[derive(Clone)]
pub struct ButtonBehavior {
    pub id: ElementId,
    pub disabled: bool,
    pub selected: bool,
}

impl ButtonBehavior {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            disabled: false,
            selected: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}
