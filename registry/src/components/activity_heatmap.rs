//! DailyActivity with pointer and keyboard inspection.
use crate::primitives::plot::{InspectSample, Plot, PlotKind, Sample};
use gpui::{prelude::*, *};
pub struct DailyActivity {
    plot: Plot,
}
impl EventEmitter<InspectSample> for DailyActivity {}
impl DailyActivity {
    pub fn new(title: impl Into<SharedString>, samples: Vec<Sample>) -> Self {
        Self {
            plot: Plot::new(PlotKind::Calendar { first_weekday: 0 }, title, samples),
        }
    }
    /// Align the first sample with its weekday, where Sunday is zero.
    pub fn set_first_weekday(&mut self, weekday: usize, cx: &mut Context<Self>) {
        self.plot.set_calendar_start(weekday);
        cx.notify();
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
impl Render for DailyActivity {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.plot.render(w, cx, |v| &mut v.plot)
    }
}
