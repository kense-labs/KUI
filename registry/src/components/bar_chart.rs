//! BarChart with pointer and keyboard inspection.
use crate::primitives::plot::{InspectSample, Plot, PlotKind, Sample};
use gpui::{prelude::*, *};
/// `BarChart` component.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::bar_chart::BarChart;
/// use kui::primitives::plot::Sample;
///
/// let control = cx.new(|_| BarChart::new("Activity", vec![Sample::new("Mon", 12.0), Sample::new("Tue", 18.0)]));
/// # }
/// ```
pub struct BarChart {
    plot: Plot,
}
impl EventEmitter<InspectSample> for BarChart {}
impl BarChart {
    pub fn new(title: impl Into<SharedString>, samples: Vec<Sample>) -> Self {
        Self {
            plot: Plot::new(PlotKind::Bar, title, samples),
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
impl Render for BarChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.plot.render(w, cx, |v| &mut v.plot)
    }
}
