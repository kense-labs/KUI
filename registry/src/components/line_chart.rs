//! LineChart with pointer and keyboard inspection.
use crate::primitives::plot::{InspectSample, Plot, PlotKind, Sample};
use gpui::{prelude::*, *};
/// `LineChart` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::line_chart::LineChart;
/// use kui::primitives::plot::Sample;
///
/// let control = cx.new(|_| LineChart::new("Activity", vec![Sample::new("Mon", 12.0), Sample::new("Tue", 18.0)]));
/// # }
/// ```
pub struct LineChart {
    plot: Plot,
}
impl EventEmitter<InspectSample> for LineChart {}
impl LineChart {
    pub fn new(title: impl Into<SharedString>, samples: Vec<Sample>) -> Self {
        Self {
            plot: Plot::new(PlotKind::Line, title, samples),
        }
    }
    /// Place host-owned range or filter controls beside the chart title.
    pub fn with_header(mut self, header: impl Into<AnyView>) -> Self {
        self.plot.header = Some(header.into());
        self
    }
    /// Describe the comparison window shown beside the percentage delta.
    pub fn with_comparison_note(mut self, note: impl Into<SharedString>) -> Self {
        self.plot.comparison_note = Some(note.into());
        self
    }
    /// Update the comparison window description beside the percentage delta.
    pub fn set_comparison_note(&mut self, note: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.plot.comparison_note = Some(note.into());
        cx.notify();
    }
    pub fn set_samples(&mut self, samples: Vec<Sample>, cx: &mut Context<Self>) {
        self.plot.set_data(samples, cx);
        cx.notify()
    }
}
impl Render for LineChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.plot.render(w, cx, |v| &mut v.plot)
    }
}
