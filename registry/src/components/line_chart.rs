//! LineChart with pointer and keyboard inspection.
use crate::primitives::plot::{InspectSample, Plot, PlotKind, Sample};
use gpui::{prelude::*, *};
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
