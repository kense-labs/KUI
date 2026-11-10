//! Interactive Git history and merge-strategy previews.
use crate::{
    foundation::{Theme, ThemeMode},
    primitives::{
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face, standard_ease},
        glyph::{Glyph, icon},
        presentation::{SERIES, tr},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashSet, f32::consts::PI, rc::Rc, time::Duration};
use web_time::Instant;

const ROW: f32 = 44.;
const PAD: f32 = 6.;
const AUTOPLAY_SECONDS: f32 = 4.;
// The first three lane inks are the observed light and dark reference colors
// converted to sRGB for GPUI. Additional lanes use the KUI series palette.
const LIGHT_LANES: [u32; 3] = [0x1f7dcf, 0x7128a5, 0x009e84];
const DARK_LANES: [u32; 3] = [0x55adff, 0x9c37be, 0x00a861];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BranchStrategy {
    #[default]
    Open,
    Merge,
    Rebase,
    Squash,
}

impl BranchStrategy {
    const ALL: [Self; 4] = [Self::Open, Self::Merge, Self::Rebase, Self::Squash];
    const CYCLE: [Self; 4] = [Self::Open, Self::Rebase, Self::Merge, Self::Squash];

    fn name(self, cx: &App) -> &'static str {
        match self {
            Self::Open => tr(cx, "Before", "合并前"),
            Self::Merge => tr(cx, "Merge", "合并"),
            Self::Rebase => tr(cx, "Rebase", "变基"),
            Self::Squash => tr(cx, "Squash", "压缩"),
        }
    }

    fn duration(self) -> f32 {
        match self {
            Self::Open => 0.64,
            Self::Merge => 0.60,
            Self::Rebase => 0.94,
            Self::Squash => 0.68,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchAuthor {
    pub name: SharedString,
    pub avatar: Option<SharedString>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchCommit {
    pub id: SharedString,
    pub hash: SharedString,
    pub message: SharedString,
    pub body: Option<SharedString>,
    pub branch: SharedString,
    pub parents: Vec<SharedString>,
    pub author: BranchAuthor,
    pub date: SharedString,
    pub additions: Option<u32>,
    pub deletions: Option<u32>,
    pub files: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchLane {
    pub name: SharedString,
    pub color: Option<Hsla>,
}

impl BranchLane {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            color: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchPullRequest {
    pub number: u32,
    pub title: SharedString,
    pub branch: SharedString,
}

#[derive(Clone, Debug)]
pub struct BranchGraphOptions {
    /// Newest first. Unknown parents are outside the displayed history.
    pub commits: Vec<BranchCommit>,
    /// The first lane is the destination of the pull request.
    pub branches: Vec<BranchLane>,
    pub pull_request: BranchPullRequest,
    pub strategy: Option<BranchStrategy>,
    pub default_strategy: BranchStrategy,
    pub strategies: Vec<BranchStrategy>,
    pub title: Option<SharedString>,
    pub autoplay: bool,
    pub speed: f32,
    pub paused: bool,
    pub accent: Option<Hsla>,
    pub label: Option<SharedString>,
}

impl Default for BranchGraphOptions {
    fn default() -> Self {
        Self {
            commits: example_commits(),
            branches: ["main", "feat/checkout", "fix/rounding"]
                .map(BranchLane::new)
                .to_vec(),
            pull_request: BranchPullRequest {
                number: 482,
                title: "Checkout redesign".into(),
                branch: "feat/checkout".into(),
            },
            strategy: None,
            default_strategy: BranchStrategy::Open,
            strategies: BranchStrategy::ALL.to_vec(),
            title: None,
            autoplay: true,
            speed: 1.,
            paused: false,
            accent: None,
            label: None,
        }
    }
}

fn example_commits() -> Vec<BranchCommit> {
    let rows = [
        (
            "f3",
            "9f2c1ab",
            "Save cards for returning customers",
            "Keeps a customer reference on the first order so saved cards appear next time.",
            1,
            &["f2"][..],
            0,
            "2 hours ago",
            184,
            32,
            6,
        ),
        (
            "m4",
            "4e81d07",
            "Bump stripe-js to 4.2",
            "Updates the checkout SDK and refreshes the lockfile.",
            0,
            &["m3"][..],
            1,
            "5 hours ago",
            12,
            12,
            2,
        ),
        (
            "f2",
            "c7a3e52",
            "Add Apple Pay button",
            "Shows Apple Pay where supported and falls back to the card form.",
            1,
            &["f1"][..],
            0,
            "Yesterday",
            96,
            8,
            4,
        ),
        (
            "m3",
            "2d9b6f4",
            "Merge branch fix/rounding",
            "Brings the tax calculation fix into checkout.",
            0,
            &["m2", "x1"][..],
            2,
            "Yesterday",
            18,
            6,
            3,
        ),
        (
            "x1",
            "a15e3c9",
            "Round tax per line item",
            "Rounds each line before calculating the order total.",
            2,
            &["m2"][..],
            2,
            "2 days ago",
            18,
            6,
            3,
        ),
        (
            "f1",
            "6b04d8e",
            "Move payment into its own step",
            "Separates payment selection from order review.",
            1,
            &["m2"][..],
            0,
            "2 days ago",
            241,
            117,
            9,
        ),
        (
            "m2",
            "e3c72a0",
            "Add order summary",
            "Shows the subtotal, delivery cost and tax before paying.",
            0,
            &["m1"][..],
            3,
            "3 days ago",
            132,
            14,
            5,
        ),
        (
            "m1",
            "0f5d9b3",
            "Ship checkout v1",
            "Adds the first complete checkout flow.",
            0,
            &[][..],
            3,
            "4 days ago",
            410,
            0,
            12,
        ),
    ];
    let branches = ["main", "feat/checkout", "fix/rounding"];
    let names = [
        "Chloe Nguyen",
        "Daniel Kim",
        "Sofia Ramirez",
        "Marcus Johnson",
    ];
    rows.into_iter()
        .map(
            |(
                id,
                hash,
                message,
                body,
                lane,
                parents,
                author,
                date,
                additions,
                deletions,
                files,
            )| {
                BranchCommit {
                    id: id.into(),
                    hash: hash.into(),
                    message: message.into(),
                    body: Some(body.into()),
                    branch: branches[lane].into(),
                    parents: parents.iter().map(|s| (*s).into()).collect(),
                    author: BranchAuthor {
                        name: names[author].into(),
                        avatar: None,
                    },
                    date: date.into(),
                    additions: Some(additions),
                    deletions: Some(deletions),
                    files: Some(files),
                }
            },
        )
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BranchStrategyChange {
    pub strategy: BranchStrategy,
}

#[derive(Clone)]
struct History {
    rows: Vec<Rc<BranchCommit>>,
    tags: Vec<(SharedString, SharedString)>,
}

impl History {
    fn row(&self, id: &SharedString) -> Option<&Rc<BranchCommit>> {
        self.rows.iter().find(|row| &row.id == id)
    }

    fn tip(&self, lane: &str) -> Option<&Rc<BranchCommit>> {
        self.rows.iter().find(|row| row.branch.as_ref() == lane)
    }

    fn ancestors(&self, tip: &SharedString) -> HashSet<SharedString> {
        let mut visited = HashSet::new();
        let mut pending = vec![tip.clone()];
        while let Some(id) = pending.pop() {
            if visited.insert(id.clone())
                && let Some(row) = self.row(&id)
            {
                pending.extend(row.parents.iter().cloned());
            }
        }
        visited
    }

    fn retag(&mut self, name: &SharedString, id: &SharedString) {
        self.tags.retain(|(lane, _)| lane != name);
        self.tags.push((name.clone(), id.clone()));
    }

    fn tags(&self, id: &SharedString) -> impl Iterator<Item = &SharedString> {
        self.tags
            .iter()
            .filter_map(move |(name, tip)| (tip == id).then_some(name))
    }
}

struct GraphData {
    layouts: [History; 4],
    lanes: Vec<BranchLane>,
    ahead: usize,
    behind: usize,
}

fn digest(parts: &[&str]) -> u32 {
    parts
        .iter()
        .flat_map(|part| part.bytes().chain([0]))
        .fold(0x811c9dc5_u32, |hash, byte| {
            (hash ^ u32::from(byte)).wrapping_mul(0x01000193)
        })
}

fn preview_hash(parts: &[&str]) -> SharedString {
    format!("{:07x}", digest(parts) & 0x0fff_ffff).into()
}

impl GraphData {
    fn new(options: &BranchGraphOptions) -> Self {
        let mut lanes = Vec::new();
        let mut seen = HashSet::new();
        for lane in &options.branches {
            if seen.insert(lane.name.clone()) {
                lanes.push(lane.clone());
            }
        }
        if lanes.is_empty()
            && options
                .commits
                .iter()
                .any(|row| row.branch.as_ref() == "main")
        {
            lanes.push(BranchLane::new("main"));
            seen.insert("main".into());
        }
        for row in &options.commits {
            if seen.insert(row.branch.clone()) {
                lanes.push(BranchLane::new(row.branch.clone()));
            }
        }
        let mut ids = HashSet::new();
        let mut open = History {
            rows: options
                .commits
                .iter()
                .filter(|row| ids.insert(row.id.clone()))
                .cloned()
                .map(Rc::new)
                .collect(),
            tags: Vec::new(),
        };
        // Merged side lanes remain drawn, but their closed tips carry no tag.
        let merged: HashSet<_> = open
            .rows
            .iter()
            .flat_map(|row| row.parents.iter().skip(1).cloned())
            .collect();
        for lane in &lanes {
            if let Some(tip) = open.tip(&lane.name)
                && !merged.contains(&tip.id)
            {
                open.tags.push((lane.name.clone(), tip.id.clone()));
            }
        }
        let mut data = Self {
            layouts: std::array::from_fn(|_| open.clone()),
            lanes,
            ahead: 0,
            behind: 0,
        };
        let Some(base) = data.lanes.first().map(|lane| &lane.name) else {
            return data;
        };
        let pr = &options.pull_request;
        if &pr.branch == base {
            return data;
        }
        let (Some(base_tip), Some(feature_tip)) = (open.tip(base), open.tip(&pr.branch)) else {
            return data;
        };
        let base_history = open.ancestors(&base_tip.id);
        let feature_history = open.ancestors(&feature_tip.id);
        let feature: Vec<_> = open
            .rows
            .iter()
            .filter(|row| {
                row.branch == pr.branch
                    && feature_history.contains(&row.id)
                    && !base_history.contains(&row.id)
            })
            .collect();
        let feature_ids: HashSet<_> = feature.iter().map(|row| row.id.clone()).collect();
        data.ahead = feature.len();
        data.behind = open
            .rows
            .iter()
            .filter(|row| base_history.contains(&row.id) && !feature_history.contains(&row.id))
            .count();
        if feature.is_empty() {
            return data;
        }

        let total = |field: fn(&BranchCommit) -> Option<u32>| {
            feature
                .iter()
                .filter_map(|row| field(row))
                .reduce(u32::saturating_add)
        };
        for strategy in [BranchStrategy::Merge, BranchStrategy::Squash] {
            let squash = strategy == BranchStrategy::Squash;
            let kind = if squash { "squash" } else { "merge" };
            let number = pr.number.to_string();
            let mut id = format!("preview-{kind}-{number}");
            while ids.contains(id.as_str()) {
                id.push('_');
            }
            let id: SharedString = id.into();
            let commit = Rc::new(BranchCommit {
                id: id.clone(),
                hash: preview_hash(&[kind, &number, &base_tip.hash, &feature_tip.hash]),
                message: if squash {
                    format!("{} (#{number})", pr.title).into()
                } else {
                    format!("Merge pull request #{number} from {}", pr.branch).into()
                },
                body: None,
                branch: base.clone(),
                parents: if squash {
                    vec![base_tip.id.clone()]
                } else {
                    vec![base_tip.id.clone(), feature_tip.id.clone()]
                },
                author: feature_tip.author.clone(),
                date: "Just now".into(),
                additions: total(|row| row.additions),
                deletions: total(|row| row.deletions),
                files: total(|row| row.files),
            });
            let mut target = open.clone();
            if squash {
                target.rows.retain(|row| !feature_ids.contains(&row.id));
                target.tags.retain(|(_, tip)| !feature_ids.contains(tip));
            }
            target.rows.insert(0, commit);
            target.retag(base, &id);
            data.layouts[strategy as usize] = target;
        }

        let mut target = open.clone();
        target.rows.retain(|row| !feature_ids.contains(&row.id));
        let mut parent = base_tip.id.clone();
        let mut parent_hash = base_tip.hash.clone();
        for old in feature.iter().rev() {
            let mut row = old.as_ref().clone();
            row.hash = preview_hash(&["rebase", &row.hash, &parent_hash]);
            row.branch = base.clone();
            row.parents = vec![parent];
            parent = row.id.clone();
            parent_hash = row.hash.clone();
            target.rows.insert(0, Rc::new(row));
        }
        target.retag(base, &parent);
        target.retag(&pr.branch, &parent);
        data.layouts[BranchStrategy::Rebase as usize] = target;
        data
    }

    fn layout(&self, strategy: BranchStrategy) -> &History {
        &self.layouts[strategy as usize]
    }

    fn lane(&self, name: &str) -> usize {
        self.lanes
            .iter()
            .position(|lane| lane.name.as_ref() == name)
            .unwrap_or(0)
    }
}

#[derive(Clone)]
struct Pose {
    row: Rc<BranchCommit>,
    y: f32,
    lane: f32,
    alpha: f32,
    lift: f32,
}

impl GraphData {
    fn poses(&self, strategy: BranchStrategy) -> Vec<Pose> {
        self.layout(strategy)
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| Pose {
                row: row.clone(),
                y: PAD + ROW * (i as f32 + 0.5),
                lane: self.lane(&row.branch) as f32,
                alpha: 1.,
                lift: 0.,
            })
            .collect()
    }

    fn height(&self, strategy: BranchStrategy) -> f32 {
        let rows = self
            .layout(BranchStrategy::Open)
            .rows
            .len()
            .max(self.layout(strategy).rows.len());
        (rows.max(1) as f32 * ROW) + 2. * PAD
    }
}

#[derive(Clone)]
struct Edge {
    parent: SharedString,
    child: SharedString,
    secondary: bool,
    alpha: f32,
}

impl Edge {
    fn same(&self, other: &Self) -> bool {
        self.parent == other.parent
            && self.child == other.child
            && self.secondary == other.secondary
    }
}

#[derive(Clone)]
struct Frame {
    rows: Vec<Pose>,
    edges: Vec<Edge>,
    height: f32,
    entrance: bool,
}

impl GraphData {
    fn frame(&self, strategy: BranchStrategy) -> Frame {
        let history = self.layout(strategy);
        let mut edges = Vec::new();
        for row in &history.rows {
            for (i, parent) in row.parents.iter().enumerate() {
                if history.row(parent).is_some() && parent != &row.id {
                    edges.push(Edge {
                        parent: parent.clone(),
                        child: row.id.clone(),
                        secondary: i > 0,
                        alpha: 1.,
                    });
                }
            }
        }
        Frame {
            rows: self.poses(strategy),
            edges,
            height: self.height(strategy),
            entrance: false,
        }
    }
}

struct Flight {
    from: Frame,
    to: Frame,
    time: f32,
    duration: f32,
    target: BranchStrategy,
    entrance: bool,
    clock: PhaseClock,
    age: f32,
}

impl Flight {
    fn advance(&mut self, visible: bool, speed: f32, cx: &App) {
        if !visible {
            self.clock = PhaseClock::default();
            return;
        }
        if !self.clock.age(cx).is_finite() {
            self.clock.restart(cx);
            self.age = 0.;
        }
        let age = self.clock.age(cx);
        self.time += (age - self.age).max(0.) * speed;
        self.age = age;
    }

    fn sample(&self) -> Frame {
        let t = (self.time / self.duration).clamp(0., 1.);
        let blend = standard_ease(t);
        let mut rows = Vec::with_capacity(self.from.rows.len() + self.to.rows.len());
        for (i, end) in self.to.rows.iter().enumerate() {
            let prior = self.from.rows.iter().find(|p| p.row.id == end.row.id);
            let seed = self
                .from
                .rows
                .iter()
                .find(|p| p.row.id.starts_with("preview-squash"));
            let start = prior.cloned().unwrap_or_else(|| Pose {
                alpha: 0.,
                y: seed.map_or(end.y - 9., |p| p.y),
                lane: seed.map_or(end.lane, |p| p.lane),
                ..end.clone()
            });
            let rewritten = prior.is_some() && start.row.hash != end.row.hash;
            let rebasing = self.target == BranchStrategy::Rebase && rewritten;
            let mix = if self.entrance {
                let delay =
                    ((self.to.rows.len() - 1 - i) as f32 / self.to.rows.len().max(1) as f32) * 0.76;
                standard_ease(((t - delay) / 0.24).clamp(0., 1.))
            } else if rebasing {
                let delay = (i as f32 * 0.075).min(0.37);
                standard_ease(((t - delay) / (0.9 - delay)).clamp(0., 1.))
            } else if prior.is_none()
                && matches!(self.target, BranchStrategy::Merge | BranchStrategy::Squash)
            {
                standard_ease(((t - 0.26) / 0.74).clamp(0., 1.))
            } else {
                blend
            };
            rows.push(Pose {
                row: if rewritten && mix < 0.70 {
                    start.row
                } else {
                    end.row.clone()
                },
                y: start.y + (end.y - start.y) * mix,
                lane: start.lane + (end.lane - start.lane) * mix,
                alpha: start.alpha + (end.alpha - start.alpha) * mix,
                lift: start.lift * (1. - mix) + if rebasing { (PI * mix).sin() * 14. } else { 0. },
            });
        }
        for start in &self.from.rows {
            if self.to.rows.iter().any(|end| end.row.id == start.row.id) {
                continue;
            }
            let squash = self.target == BranchStrategy::Squash;
            let y = if squash {
                PAD + ROW * 0.5
            } else {
                start.y - 8.
            };
            rows.push(Pose {
                y: start.y + (y - start.y) * blend,
                lane: if squash {
                    start.lane * (1. - blend)
                } else {
                    start.lane
                },
                alpha: start.alpha * (1. - blend),
                lift: start.lift * (1. - blend)
                    + if squash { (PI * blend).sin() * 10. } else { 0. },
                ..start.clone()
            });
        }
        let mut edges = Vec::with_capacity(self.from.edges.len() + self.to.edges.len());
        for end in &self.to.edges {
            let alpha = self
                .from
                .edges
                .iter()
                .find(|edge| edge.same(end))
                .map_or(0., |edge| edge.alpha);
            edges.push(Edge {
                alpha: alpha + (end.alpha - alpha) * blend,
                ..end.clone()
            });
        }
        for start in &self.from.edges {
            if !self.to.edges.iter().any(|edge| edge.same(start)) {
                edges.push(Edge {
                    alpha: start.alpha * (1. - blend),
                    ..start.clone()
                });
            }
        }
        Frame {
            rows,
            edges,
            // Hold the clip while rows move. Release the extra space only after
            // the last row lands so an embedding iframe never resizes per frame.
            height: self.from.height.max(self.to.height),
            entrance: self.entrance,
        }
    }
}

type Refine = Box<dyn Fn(Div) -> Div>;
type Visibility = Box<dyn Fn(&App) -> bool>;

pub struct BranchGraph {
    options: BranchGraphOptions,
    data: GraphData,
    strategy: BranchStrategy,
    frames: [Frame; 4],
    flight: Option<Flight>,
    flight_wake: Option<Task<()>>,
    last_flight_paint: Option<Instant>,
    autoplay_stopped: bool,
    hovering: bool,
    hovered: Option<SharedString>,
    pinned: Option<SharedString>,
    keyboard: Option<SharedString>,
    announcement: Option<SharedString>,
    wake: Option<(Instant, Task<()>)>,
    remaining: f32,
    focus: FocusHandle,
    strategy_focus: [FocusHandle; 4],
    blur: Option<Subscription>,
    width: Rc<Cell<Pixels>>,
    highlight: Slide,
    highlight_initialized: bool,
    id: ElementId,
    visibility: Option<Visibility>,
    style: Option<Refine>,
}

impl EventEmitter<BranchStrategyChange> for BranchGraph {}

impl BranchGraph {
    pub fn new(mut options: BranchGraphOptions, cx: &mut Context<Self>) -> Self {
        let mut seen = HashSet::new();
        options.strategies.retain(|strategy| seen.insert(*strategy));
        options.speed = if options.speed.is_finite() {
            options.speed.clamp(0., 32.)
        } else {
            1.
        };
        let strategy = options.strategy.unwrap_or(options.default_strategy);
        let data = GraphData::new(&options);
        let frames = BranchStrategy::ALL.map(|strategy| data.frame(strategy));
        let to = frames[strategy as usize].clone();
        let mut from = to.clone();
        for pose in &mut from.rows {
            pose.alpha = 0.;
        }
        Self {
            options,
            data,
            strategy,
            frames,
            flight: Some(Flight {
                from,
                to,
                time: 0.,
                duration: 0.86,
                target: strategy,
                entrance: true,
                clock: PhaseClock::default(),
                age: 0.,
            }),
            flight_wake: None,
            last_flight_paint: None,
            autoplay_stopped: false,
            hovering: false,
            hovered: None,
            pinned: None,
            keyboard: None,
            announcement: None,
            wake: None,
            remaining: AUTOPLAY_SECONDS,
            focus: cx.focus_handle().tab_stop(true),
            strategy_focus: std::array::from_fn(|_| cx.focus_handle()),
            blur: None,
            width: Rc::default(),
            highlight: Slide::snappy(0.),
            highlight_initialized: false,
            id: "branch-graph".into(),
            visibility: None,
            style: None,
        }
    }

    pub fn strategy(&self) -> BranchStrategy {
        self.strategy
    }

    pub fn with_id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_style(mut self, style: impl Fn(Div) -> Div + 'static) -> Self {
        self.style = Some(Box::new(style));
        self
    }

    pub fn with_visibility(mut self, visible: impl Fn(&App) -> bool + 'static) -> Self {
        self.visibility = Some(Box::new(visible));
        self
    }

    /// Feed back a controlled selection without emitting another request.
    pub fn set_strategy(&mut self, strategy: BranchStrategy, cx: &mut Context<Self>) {
        self.options.strategy = Some(strategy);
        self.transition(strategy, cx);
        cx.notify();
    }

    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        self.options.paused = paused;
        cx.notify();
    }

    fn current(&self) -> Frame {
        self.flight.as_ref().map_or_else(
            || self.frames[self.strategy as usize].clone(),
            Flight::sample,
        )
    }

    fn transition(&mut self, strategy: BranchStrategy, cx: &mut Context<Self>) {
        if strategy == self.strategy {
            return;
        }
        let from = self.current();
        self.strategy = strategy;
        self.flight = Some(Flight {
            from,
            to: self.frames[strategy as usize].clone(),
            time: 0.,
            duration: strategy.duration(),
            target: strategy,
            entrance: false,
            clock: PhaseClock::default(),
            age: 0.,
        });
        let history = self.data.layout(strategy);
        for id in [&mut self.hovered, &mut self.pinned, &mut self.keyboard] {
            if id.as_ref().is_some_and(|id| history.row(id).is_none()) {
                *id = None;
            }
        }
        self.remaining = AUTOPLAY_SECONDS;
        cx.notify();
    }

    fn request_flight_frame(&mut self, w: &Window, cx: &mut Context<Self>) {
        let view = cx.weak_entity();
        w.on_next_frame(move |_, cx| {
            let _ = view.update(cx, |_, cx| cx.notify());
        });
        self.last_flight_paint = Some(cx.background_executor().now());
        if self.flight_wake.is_none() {
            // A newly activated embedded surface can lose its first frame wake.
            // The timer only requests a redraw if no recent paint has occurred.
            let interval = Duration::from_millis(34);
            let timer = cx.background_executor().timer(interval);
            self.flight_wake = Some(cx.spawn(async move |view, cx| {
                timer.await;
                let _ = view.update(cx, |view, cx| {
                    view.flight_wake = None;
                    let now = cx.background_executor().now();
                    if view.flight.is_some()
                        && view
                            .last_flight_paint
                            .is_none_or(|paint| now.saturating_duration_since(paint) >= interval)
                    {
                        cx.notify();
                    }
                });
            }));
        }
    }

    fn interact(&mut self) {
        self.autoplay_stopped = true;
        self.wake = None;
    }

    fn choose(&mut self, next: BranchStrategy, cx: &mut Context<Self>) {
        self.interact();
        if next != self.strategy {
            if self.options.strategy.is_none() {
                self.transition(next, cx);
            }
            self.announcement = Some(self.summary(next, cx).into());
            cx.emit(BranchStrategyChange { strategy: next });
        }
        cx.notify();
    }

    fn summary(&self, strategy: BranchStrategy, cx: &App) -> String {
        let pr = &self.options.pull_request;
        let base = self
            .data
            .lanes
            .first()
            .map_or("main", |lane| lane.name.as_ref());
        let a = self.data.ahead;
        let b = self.data.behind;
        let zh = crate::primitives::control::UiLanguage::current(cx)
            == crate::primitives::control::UiLanguage::Chinese;
        match (strategy, zh) {
            (BranchStrategy::Open, false) => format!(
                "{} is {a} commits ahead of {base} and {b} behind.",
                pr.branch
            ),
            (BranchStrategy::Open, true) => {
                format!("{} 比 {base} 领先 {a} 次提交，落后 {b} 次。", pr.branch)
            }
            (BranchStrategy::Merge, false) => format!(
                "A merge commit brings {a} commits into {base} and keeps both lines of history."
            ),
            (BranchStrategy::Merge, true) => {
                format!("{a} 次提交经合并纳入 {base}，两条历史都保留。")
            }
            (BranchStrategy::Rebase, false) => {
                format!("{a} commits replayed on top of {base} with new hashes.")
            }
            (BranchStrategy::Rebase, true) => {
                format!("{a} 次提交在 {base} 上重新应用，并生成新哈希。")
            }
            (BranchStrategy::Squash, false) => {
                format!("{a} commits squashed into one commit on {base}.")
            }
            (BranchStrategy::Squash, true) => format!("{a} 次提交在 {base} 上压缩为一次提交。"),
        }
    }

    fn autoplay(&mut self, visible: bool, reduced: bool, cx: &mut Context<Self>) {
        let ready = visible
            && !reduced
            && self.options.autoplay
            && !self.options.paused
            && self.options.strategy.is_none()
            && !self.autoplay_stopped
            && !self.hovering
            && self.options.speed > 0.
            && self.options.strategies.len() > 1
            && self.flight.is_none()
            && !self.options.commits.is_empty();
        let now = cx.background_executor().now();
        if !ready {
            if let Some((at, _)) = self.wake.take() {
                self.remaining =
                    at.saturating_duration_since(now).as_secs_f32() * self.options.speed;
            }
            return;
        }
        if self.wake.is_some() {
            return;
        }
        let cycle = BranchStrategy::CYCLE;
        let index = cycle
            .iter()
            .position(|strategy| *strategy == self.strategy)
            .unwrap_or(0);
        let Some(next) = (1..cycle.len())
            .map(|offset| cycle[(index + offset) % cycle.len()])
            .find(|strategy| self.options.strategies.contains(strategy))
        else {
            return;
        };
        let delay = Duration::from_secs_f32((self.remaining / self.options.speed).clamp(0., 30.));
        let timer = cx.background_executor().timer(delay);
        self.wake = Some((
            now + delay,
            cx.spawn(async move |view, cx| {
                timer.await;
                let _ = view.update(cx, |view, cx| {
                    view.wake = None;
                    view.transition(next, cx);
                });
            }),
        ));
    }

    fn close_detail(&mut self, cx: &mut Context<Self>) {
        let pointer = self.hovered.take();
        let touch = self.pinned.take();
        let keyboard = self.keyboard.take();
        if pointer.is_some() || touch.is_some() || keyboard.is_some() {
            cx.notify();
        }
    }

    fn list_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.interact();
        let history = self.data.layout(self.strategy);
        let n = history.rows.len();
        if n == 0 {
            return;
        }
        let index = self
            .keyboard
            .as_ref()
            .and_then(|id| history.rows.iter().position(|row| &row.id == id));
        let next = match event.keystroke.key.as_str() {
            "up" => index.map_or(n - 1, |i| i.saturating_sub(1)),
            "down" => index.map_or(0, |i| (i + 1).min(n - 1)),
            "home" => 0,
            "end" => n - 1,
            "escape" => {
                self.close_detail(cx);
                window.prevent_default();
                cx.stop_propagation();
                return;
            }
            _ => return,
        };
        self.keyboard = Some(history.rows[next].id.clone());
        self.pinned = None;
        self.hovered = None;
        window.prevent_default();
        cx.stop_propagation();
        cx.notify();
    }
}

fn lane_x(lane: f32, inset: f32, lanes: usize) -> f32 {
    let spacing = if lanes > 3 {
        ((inset - 34.) / (lanes - 1) as f32).clamp(0., 18.)
    } else {
        18.
    };
    22. + lane * spacing
}

fn edge_curve(a: Point<Pixels>, z: Point<Pixels>, secondary: bool) -> [Point<Pixels>; 4] {
    let delta = z.y - a.y;
    let dx = (z.x - a.x).abs();
    // A new lane bends at the parent. A merge rejoins at the child.
    let span = (px(13.2) + dx * (11.44 / 18.)).min(delta.abs());
    let rise = if delta < px(0.) { -span } else { span };
    let start = point(a.x, if secondary { z.y - rise } else { a.y });
    let end = point(z.x, if secondary { z.y } else { a.y + rise });
    [
        start,
        point(a.x, start.y + rise * 0.56),
        point(z.x, end.y - rise * 0.56),
        end,
    ]
}

fn circle(w: &mut Window, center: Point<Pixels>, radius: f32, color: Hsla) {
    let r = px(radius);
    let bounds = Bounds::new(center - point(r, r), size(r * 2., r * 2.));
    w.paint_quad(fill(bounds, color).corner_radii(r));
}

struct Drawing {
    poses: Rc<Vec<Pose>>,
    edges: Vec<Edge>,
    entrance: bool,
    colors: Vec<Hsla>,
    selected: Option<SharedString>,
    surface: Hsla,
    inset: f32,
}

impl Drawing {
    fn paint(&self, bounds: Bounds<Pixels>, w: &mut Window) {
        let xy = |p: &Pose| {
            bounds.origin
                + point(
                    px(lane_x(p.lane, self.inset, self.colors.len())),
                    px(p.y - p.lift),
                )
        };
        let tint = |p: &Pose| {
            let lane = p.lane.max(0.);
            let low = lane.floor() as usize;
            let first = self
                .colors
                .get(low)
                .copied()
                .unwrap_or_else(|| rgb(SERIES[0]).into());
            self.colors
                .get(low + 1)
                .map_or(first, |next| first.blend(next.opacity(lane.fract())))
        };
        let selected_lane = self
            .selected
            .as_ref()
            .and_then(|id| self.poses.iter().find(|p| &p.row.id == id))
            .map(|p| p.lane.round());
        let dim = |lane: f32| {
            if selected_lane.is_none_or(|selected| lane.round() == selected) {
                1.
            } else {
                0.34
            }
        };
        for edge in &self.edges {
            let (Some(child), Some(parent)) = (
                self.poses.iter().find(|p| p.row.id == edge.child),
                self.poses.iter().find(|p| p.row.id == edge.parent),
            ) else {
                continue;
            };
            let lane = if edge.secondary { parent } else { child };
            let reveal = if self.entrance {
                (child.alpha * 1.12 + 0.12).clamp(0., 1.)
            } else {
                1.
            };
            let opacity = if self.entrance {
                parent.alpha
            } else {
                parent.alpha.min(child.alpha)
            };
            let alpha = opacity * edge.alpha * dim(lane.lane);
            if alpha < 0.015 {
                continue;
            }
            let a = xy(parent);
            let z = xy(child);
            let [start, ctrl_a, ctrl_b, end] = edge_curve(a, z, edge.secondary);
            let mut path = PathBuilder::stroke(px(2.));
            path.move_to(a);
            if (a.x - z.x).abs() < px(0.01) {
                path.line_to(a + (z - a) * reveal);
            } else if reveal >= 1. {
                path.line_to(start);
                path.cubic_bezier_to(end, ctrl_a, ctrl_b);
                path.line_to(z);
            } else {
                let up_to = (reveal * 24.).ceil() as usize;
                let mix = |a: Point<Pixels>, b: Point<Pixels>, t: f32| a + (b - a) * t;
                let span = (z.y - a.y).abs().max(px(0.01));
                let prefix = (start.y - a.y).abs() / span;
                let curve = (end.y - start.y).abs() / span;
                for i in 1..=up_to {
                    let t = (i as f32 / 24.).min(reveal);
                    let value = if t < prefix {
                        mix(a, start, t / prefix)
                    } else if t < prefix + curve {
                        let t = (t - prefix) / curve;
                        mix(
                            mix(mix(start, ctrl_a, t), mix(ctrl_a, ctrl_b, t), t),
                            mix(mix(ctrl_a, ctrl_b, t), mix(ctrl_b, end, t), t),
                            t,
                        )
                    } else {
                        mix(
                            end,
                            z,
                            (t - prefix - curve) / (1. - prefix - curve).max(0.0001),
                        )
                    };
                    path.line_to(value);
                }
            }
            if let Ok(path) = path.build() {
                w.paint_path(path, tint(lane).opacity(alpha));
            }
        }
        for p in self.poses.iter().filter(|p| p.alpha > 0.025) {
            let at = xy(p);
            let color = tint(p).opacity(p.alpha * dim(p.lane));
            let focused = self.selected.as_ref() == Some(&p.row.id);
            if focused || p.lift > 0.5 {
                circle(w, at, 10., color.opacity(0.17));
            }
            circle(w, at, 8., self.surface.opacity(p.alpha));
            circle(w, at, 5., color);
            if p.row.parents.len() > 1 {
                circle(w, at, 3.2, self.surface.opacity(p.alpha));
            }
        }
    }
}

fn branch_mark(color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |b, _, w, _| {
            let at = |x: f32, y: f32| b.origin + point(b.size.width * x, b.size.height * y);
            let mut path = PathBuilder::stroke(px(1.));
            path.move_to(at(0.28, 0.76));
            path.line_to(at(0.28, 0.16));
            path.move_to(at(0.28, 0.59));
            path.curve_to(at(0.72, 0.30), at(0.3, 0.35));
            if let Ok(path) = path.build() {
                w.paint_path(path, color);
            }
            circle(w, at(0.28, 0.76), 2.1, color);
            circle(w, at(0.72, 0.30), 2.1, color);
        },
    )
    .size(px(14.))
    .flex_none()
}

fn date(row: &BranchCommit, cx: &App) -> SharedString {
    if row.id.starts_with("preview-") && row.date.as_ref() == "Just now" {
        return tr(cx, "Just now", "刚刚").into();
    }
    row.date.clone()
}

fn commit_message(row: &BranchCommit, cx: &App) -> SharedString {
    if crate::primitives::control::UiLanguage::current(cx)
        == crate::primitives::control::UiLanguage::Chinese
        && row.id.starts_with("preview-merge-")
        && let Some((number, branch)) = row
            .message
            .strip_prefix("Merge pull request #")
            .and_then(|message| message.split_once(" from "))
    {
        return format!("合并来自 {branch} 的请求 #{number}").into();
    }
    row.message.clone()
}

fn clipped(value: impl Into<SharedString>) -> Div {
    div()
        .min_w(px(0.))
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .child(value.into())
}

fn avatar(author: &BranchAuthor, diameter: f32) -> Div {
    let index = digest(&[&author.name]) as usize % SERIES.len();
    let a: Hsla = rgb(SERIES[index]).into();
    let b: Hsla = rgb(SERIES[(index + 3) % SERIES.len()]).into();
    div()
        .size(px(diameter))
        .flex_none()
        .rounded_full()
        .overflow_hidden()
        .bg(linear_gradient(
            125.,
            linear_color_stop(a, 0.),
            linear_color_stop(b, 1.),
        ))
        .when_some(author.avatar.clone(), |e, avatar| {
            e.child(img(avatar).size_full())
        })
}

fn stats(row: &BranchCommit, p: Paint, cx: &App) -> Div {
    let mut line = div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap(px(7.))
        .text_size(px(11.))
        .text_color(p.secondary);
    line = line
        .child(icon(Glyph::Link, 12., p.faint))
        .child(row.hash.clone());
    if let Some(added) = row.additions {
        line = line.child(div().text_color(p.success).child(format!("+{added}")));
    }
    if let Some(removed) = row.deletions {
        line = line.child(div().text_color(p.danger).child(format!("−{removed}")));
    }
    if let (Some(added), Some(removed)) = (row.additions, row.deletions)
        && added.saturating_add(removed) > 0
    {
        let green =
            ((added as f64 / (f64::from(added) + f64::from(removed))) * 5.).round() as usize;
        line = line.child(div().flex().gap(px(2.)).children((0..5).map(|i| {
            div()
                .size(px(6.))
                .rounded(px(1.))
                .bg(if i < green { p.success } else { p.danger })
        })));
    }
    if let Some(files) = row.files {
        line = line
            .child(div().flex_1().min_w(px(10.)))
            .child(format!("{files} {}", tr(cx, "files", "个文件")));
    }
    line
}

fn accessible_commit(row: &BranchCommit, tags: &[&SharedString], cx: &App) -> SharedString {
    let mut label = format!(
        "{}. {} · {} · {}",
        commit_message(row, cx),
        row.hash,
        row.author.name,
        date(row, cx)
    );
    for (count, en, zh) in [
        (row.additions, "additions", "新增"),
        (row.deletions, "deletions", "删除"),
        (row.files, "files", "文件"),
    ] {
        if let Some(count) = count {
            label.push_str(&format!(" · {count} {}", tr(cx, en, zh)));
        }
    }
    for name in tags {
        label.push_str(&format!(" · {name}"));
    }
    label.into()
}

impl BranchGraph {
    fn header(&mut self, narrow: bool, width: f32, p: Paint, cx: &mut Context<Self>) -> Div {
        let radius = Theme::global(cx).tokens.radius.md;
        let options = &self.options;
        let heading = options
            .title
            .as_ref()
            .unwrap_or(&options.pull_request.title)
            .clone();
        let name = tr(cx, "Merge strategy preview", "合并策略预览");
        let n = options.strategies.len();
        let available = if narrow { (width - 32.).max(96.) } else { 284. };
        let button_width = (available - 6.) / n.max(1) as f32;
        let selected = options
            .strategies
            .iter()
            .position(|item| *item == self.strategy);
        let focus_index = selected.unwrap_or(0);
        if !self.highlight_initialized || disabled_motion(cx) || options.speed == 0. {
            self.highlight.reset(focus_index as f32);
            self.highlight_initialized = true;
        } else {
            self.highlight.set(focus_index as f32, cx);
        }
        let progress = self.highlight.value(cx);
        let thumb = div()
            .absolute()
            .top(px(3.))
            .left(px(3. + progress * button_width))
            .w(px(button_width))
            .h(px(32.))
            .rounded(radius)
            .bg(p.panel)
            .border_1()
            .border_color(p.line);
        div()
            .min_h(px(46.))
            .w_full()
            .px(px(7.))
            .gap(px(8.))
            .flex()
            .when(narrow, |e| e.flex_col().items_start())
            .when(!narrow, |e| e.items_center().justify_between())
            .when(!heading.is_empty(), |e| {
                e.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .max_w_full()
                        .min_w(px(0.))
                        .child(branch_mark(p.secondary))
                        .child(
                            clipped(heading)
                                .text_size(px(16.))
                                .font_weight(FontWeight::MEDIUM),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_color(p.secondary)
                                .text_size(px(13.))
                                .child(format!("#{}", options.pull_request.number)),
                        ),
                )
            })
            .when(n > 1, |e| {
                e.child(
                    div()
                        .id("branch-strategies")
                        .role(Role::RadioGroup)
                        .aria_label(name)
                        .relative()
                        .flex()
                        .w(px(available))
                        .h(px(38.))
                        .flex_none()
                        .p(px(3.))
                        .rounded(radius)
                        .bg(p.stage)
                        .when(selected.is_some(), |e| e.child(thumb))
                        .children(options.strategies.iter().enumerate().map(|(i, strategy)| {
                            let strategy = *strategy;
                            face(
                                ("branch-strategy", i),
                                strategy.name(cx),
                                32.,
                                f32::from(radius),
                                Tone::Clear,
                                cx,
                            )
                            .track_focus(&self.strategy_focus[strategy as usize])
                            .tab_stop(i == focus_index)
                            .role(Role::RadioButton)
                            .aria_toggled((selected == Some(i)).into())
                            .w(px(button_width))
                            .text_size(px(13.))
                            .font_weight(FontWeight::NORMAL)
                            .text_color(if selected == Some(i) {
                                p.ink
                            } else {
                                p.secondary
                            })
                            .hover(move |e| e.text_color(p.ink))
                            .child(strategy.name(cx))
                            .on_click(cx.listener(move |s, _, _, cx| s.choose(strategy, cx)))
                            .on_key_down(cx.listener(
                                move |s, event: &KeyDownEvent, w, cx| {
                                    let n = s.options.strategies.len();
                                    let next = match event.keystroke.key.as_str() {
                                        "left" | "up" => (i + n - 1) % n,
                                        "right" | "down" => (i + 1) % n,
                                        "home" => 0,
                                        "end" => n - 1,
                                        _ => return,
                                    };
                                    let strategy = s.options.strategies[next];
                                    s.choose(strategy, cx);
                                    s.strategy_focus[strategy as usize].focus(w, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                },
                            ))
                        })),
                )
            })
    }

    fn detail(&self, pose: &Pose, width: f32, height: f32, inset: f32, p: Paint, cx: &App) -> Div {
        let row = &pose.row;
        let radius = Theme::global(cx).tokens.radius.md;
        let w = (width - 16.).clamp(80., 336.);
        let left = (inset - 6.).min((width - w - 8.).max(4.));
        let top = if pose.y + 184. < height {
            pose.y + 18.
        } else {
            (pose.y - 174.).max(4.)
        };
        div()
            .absolute()
            .left(px(left))
            .top(px(top))
            .w(px(w))
            .max_h(px(172.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .p(px(12.))
            .overflow_hidden()
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .shadow_lg()
            .text_size(px(13.))
            .line_height(px(17.))
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child(commit_message(row, cx)),
            )
            .when_some(row.body.clone(), |e, body| {
                e.child(
                    div()
                        .max_h(px(53.))
                        .overflow_hidden()
                        .text_color(p.secondary)
                        .child(body),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(11.))
                    .text_color(p.secondary)
                    .child(avatar(&row.author, 16.))
                    .child(clipped(row.author.name.clone()))
                    .child("·")
                    .child(clipped(date(row, cx))),
            )
            .child(
                div()
                    .pt(px(6.))
                    .border_t_1()
                    .border_color(p.line)
                    .child(stats(row, p, cx)),
            )
    }
}

impl Render for BranchGraph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.blur.is_none() {
            self.blur = Some(cx.on_blur(&self.focus, w, |s, _, cx| {
                s.keyboard = None;
                cx.notify();
            }));
        }
        let p = Paint::get(cx);
        let theme = Theme::global(cx);
        let radius = theme.tokens.radius.md;
        let theme_accent = theme.accent.for_mode(theme.mode);
        let palette = match theme.mode {
            ThemeMode::Light => LIGHT_LANES,
            ThemeMode::Dark => DARK_LANES,
        };
        let reduced = disabled_motion(cx) || self.options.speed == 0.;
        let visible =
            w.is_visible() && (self.hovering || self.visibility.as_ref().is_none_or(|f| f(cx)));
        if let Some(flight) = &mut self.flight {
            flight.advance(visible, self.options.speed, cx);
            if reduced || flight.time >= flight.duration {
                self.flight = None;
            }
        }
        if visible && self.flight.is_some() {
            self.request_flight_frame(w, cx);
        } else {
            self.flight_wake = None;
            self.last_flight_paint = None;
        }
        self.autoplay(visible, reduced, cx);
        let width = if self.width.get() > px(0.) {
            f32::from(self.width.get())
        } else {
            f32::from(w.viewport_size().width).clamp(180., 760.)
        };
        let narrow = width < 560.;
        let Frame {
            rows,
            edges,
            height,
            entrance,
        } = self.current();
        let poses = Rc::new(rows);
        let selected = self
            .pinned
            .as_ref()
            .or(self.keyboard.as_ref())
            .or(self.hovered.as_ref())
            .cloned();
        let selected_pose = selected
            .as_ref()
            .and_then(|id| poses.iter().find(|p| &p.row.id == id));
        let inset = (26. + 18. * self.data.lanes.len().saturating_sub(1) as f32)
            .max(if narrow { 68. } else { 80. })
            .min((width * 0.37).max(68.));
        let lane_colors: Vec<Hsla> = self
            .data
            .lanes
            .iter()
            .enumerate()
            .map(|(i, lane)| {
                let fallback: Hsla =
                    rgb(palette.get(i).copied().unwrap_or(SERIES[i % SERIES.len()])).into();
                let color = if i == 0 && !theme_accent.is_transparent() {
                    theme_accent
                } else {
                    fallback
                };
                lane.color
                    .or(if i == 0 { self.options.accent } else { None })
                    .unwrap_or(color)
            })
            .collect();
        let accent = lane_colors.first().copied().unwrap_or(p.bright);
        let header = self.header(narrow, width, p, cx);
        let ink_for = |name: &str| {
            lane_colors
                .get(self.data.lane(name))
                .copied()
                .unwrap_or(accent)
        };
        let list_width = (width - 18.).max(120.);
        let width_cell = self.width.clone();
        let current = self.data.layout(self.strategy);
        let description = self.summary(self.strategy, cx);
        let list_name = format!(
            "{} {description}",
            tr(cx, "Commits, newest first.", "提交，从新到旧。")
        );
        let mut board = div()
            .id("branch-commits")
            .debug_selector(|| "branch-commits".into())
            .role(Role::ListBox)
            .aria_label(list_name)
            .track_focus(&self.focus)
            .tab_stop(!current.rows.is_empty())
            .relative()
            .w_full()
            .h(px(height))
            .flex_none()
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .focus_visible(move |e| e.border_color(p.focus))
            .overflow_hidden()
            .on_mouse_down_out(cx.listener(|s, _, _, cx| s.close_detail(cx)))
            .on_key_down(cx.listener(Self::list_key));
        for pose in poses
            .iter()
            .filter(|pose| pose.alpha > 0.012 || current.row(&pose.row.id).is_some())
        {
            let row = &pose.row;
            let interactive = current.row(&row.id).is_some();
            let tags: Vec<_> = current.tags(&row.id).collect();
            let key = row.id.clone();
            let is_active = selected.as_ref() == Some(&key);
            let dim = if selected_pose.is_some_and(|active| (active.lane - pose.lane).abs() >= 0.5)
            {
                0.48
            } else {
                1.
            };
            let left = if pose.lift > 0.5 {
                inset + pose.lift * 0.6
            } else {
                inset
            };
            let bg = if is_active {
                p.hover
            } else {
                p.panel.opacity((pose.lift / 14.).clamp(0., 1.))
            };
            let stacked_tags = width < 400. && !tags.is_empty();
            let message = div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .when(stacked_tags, |e| e.flex_col().items_start().gap(px(0.)))
                .when(!stacked_tags, |e| e.items_center().gap(px(7.)))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .min_w(px(0.))
                        .max_w_full()
                        .gap(px(7.))
                        .when(row.parents.len() > 1, |e| e.child(branch_mark(p.secondary)))
                        .child(clipped(commit_message(row, cx))),
                )
                .when(!tags.is_empty(), |e| {
                    e.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .min_w(px(0.))
                            .max_w_full()
                            .children(tags.iter().map(|name| {
                                let ink = ink_for(name);
                                div()
                                    .max_w(px(if narrow { 106. } else { 124. }))
                                    .min_w(px(0.))
                                    .px(px(5.))
                                    .rounded(px(4.))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .bg(ink.opacity(0.12))
                                    .text_size(px(11.))
                                    .line_height(px(16.))
                                    .text_color(p.ink)
                                    .child(div().size(px(5.)).rounded_full().flex_none().bg(ink))
                                    .child(clipped((*name).clone()))
                            })),
                    )
                });
            let body = div()
                .absolute()
                .top(px(pose.y - pose.lift - ROW * 0.5))
                .left(px(4.))
                .right(px(4.))
                .h(px(ROW))
                .pl(px(left - 4.))
                .pr(px(12.))
                .flex()
                .items_center()
                .gap(px(12.))
                .min_w(px(0.))
                .rounded(radius)
                .bg(bg)
                .opacity(pose.alpha * dim)
                .text_size(px(14.))
                .line_height(px(20.))
                .text_color(p.ink)
                .child(message)
                .when(width >= 560., |e| e.child(avatar(&row.author, 20.)))
                .when(width >= 400., |e| {
                    e.child(
                        clipped(row.hash.clone())
                            .w(px(64.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.secondary.blend(accent.opacity(pose.lift / 22.))),
                    )
                })
                .when(width >= 720., |e| {
                    e.child(
                        clipped(date(row, cx))
                            .w(px(88.))
                            .flex_none()
                            .text_right()
                            .text_size(px(12.))
                            .text_color(p.secondary),
                    )
                });
            board = if interactive {
                let click = key.clone();
                let keyboard_active = self.keyboard.as_ref() == Some(&key);
                board.child(
                    body.id(SharedString::from(format!("branch-commit-{key}")))
                        .debug_selector({
                            let key = key.clone();
                            move || format!("branch-commit-{key}")
                        })
                        .role(Role::ListBoxOption)
                        .aria_label(accessible_commit(row, &tags, cx))
                        .aria_selected(is_active)
                        .when(keyboard_active, |e| e.aria_active_descendant())
                        .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                            if *over {
                                s.hovered = Some(key.clone());
                            } else if s.hovered.as_ref() == Some(&key) {
                                s.hovered = None;
                            }
                            cx.notify();
                        }))
                        .on_click(cx.listener(move |s, event: &ClickEvent, w, cx| {
                            s.interact();
                            if matches!(event, ClickEvent::Touch(_)) {
                                s.pinned = if s.pinned.as_ref() == Some(&click) {
                                    None
                                } else {
                                    Some(click.clone())
                                };
                                s.keyboard = None;
                            } else {
                                s.focus.focus(w, cx);
                            }
                            cx.notify();
                        })),
                )
            } else {
                board.child(body)
            };
        }
        let drawing = Drawing {
            poses: poses.clone(),
            edges,
            entrance,
            colors: lane_colors,
            selected,
            surface: p.panel,
            inset,
        };
        let board = board
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, w, _| drawing.paint(bounds, w),
                )
                .absolute()
                .inset_0()
                .size_full(),
            )
            .when_some(selected_pose, |e, pose| {
                e.child(self.detail(pose, list_width, height, inset, p, cx))
            })
            .when(current.rows.is_empty(), |e| {
                e.child(
                    div()
                        .flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .text_color(p.secondary)
                        .child(tr(cx, "No commits to show", "暂无提交")),
                )
            });
        let id = self.id.clone();
        let options = &self.options;
        let label = options.label.clone().unwrap_or_else(|| {
            format!(
                "{} · {} · {}",
                options.pull_request.title,
                tr(cx, "Commit graph", "提交图"),
                self.data
                    .lanes
                    .iter()
                    .map(|lane| lane.name.as_ref())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .into()
        });
        let root = div()
            .w_full()
            .max_w(px(760.))
            .min_w(px(0.))
            .p(px(8.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel);
        let root = if let Some(style) = &self.style {
            style(root)
        } else {
            root
        };
        root.on_children_prepainted(move |children, w, _| {
            if let Some(board) = children.get(1) {
                let width = board.size.width + px(18.);
                if width != width_cell.replace(width) {
                    w.request_animation_frame();
                }
            }
        })
        .id(id)
        .role(Role::Region)
        .aria_label(label)
        .on_hover(cx.listener(|s, over: &bool, _, cx| {
            s.hovering = *over;
            cx.notify();
        }))
        .on_key_down(cx.listener(|s, _: &KeyDownEvent, _, cx| {
            s.interact();
            cx.notify();
        }))
        .child(header)
        .child(board)
        .child(
            div()
                .min_h(px(32.))
                .px(px(7.))
                .flex()
                .items_center()
                .text_size(px(13.))
                .line_height(px(18.))
                .text_color(p.secondary)
                .child(
                    div()
                        .w_full()
                        .min_w(px(0.))
                        .whitespace_normal()
                        .child(description),
                ),
        )
        .when_some(self.announcement.clone(), |e, announcement| {
            e.child(
                div()
                    .id("branch-announcement")
                    .absolute()
                    .size(px(1.))
                    .overflow_hidden()
                    .opacity(0.)
                    .role(Role::Status)
                    .aria_label(announcement),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BranchGraph, BranchGraphOptions, BranchStrategy, GraphData, edge_curve};
    use gpui::{TestAppContext, point, px, size};

    #[test]
    fn branch_and_merge_bend_at_opposite_ends_of_the_edge() {
        let parent = point(px(22.), px(292.));
        let child = point(px(40.), px(248.));
        let [start, _, _, end] = edge_curve(parent, child, false);
        assert_eq!(start, parent);
        assert!((f32::from(end.y) - 267.36).abs() < 0.01);
        let [start, _, _, end] = edge_curve(parent, child, true);
        assert!((f32::from(start.y) - 272.64).abs() < 0.01);
        assert_eq!(end, child);
    }

    #[test]
    fn previews_keep_history_and_rewrite_only_the_pull_request() {
        let options = BranchGraphOptions::default();
        let data = GraphData::new(&options);
        assert_eq!((data.ahead, data.behind), (3, 3));
        let before = data.layout(BranchStrategy::Open);
        let merged = data.layout(BranchStrategy::Merge);
        assert_eq!(merged.rows.len(), 9);
        assert_eq!(
            merged.rows[0]
                .parents
                .iter()
                .map(|p| p.as_ref())
                .collect::<Vec<_>>(),
            ["m4", "f3"]
        );
        assert_eq!(merged.rows[0].additions, Some(521));
        assert_eq!(merged.rows[0].deletions, Some(157));
        assert_eq!(merged.rows[0].files, Some(19));
        let rebased = data.layout(BranchStrategy::Rebase);
        assert_eq!(rebased.rows.len(), 8);
        for (index, (id, parent)) in [("f3", "f2"), ("f2", "f1"), ("f1", "m4")]
            .into_iter()
            .enumerate()
        {
            let row = &rebased.rows[index];
            assert_eq!(&*row.id, id);
            assert_eq!(&*row.parents[0], parent);
            assert_eq!(&*row.branch, "main");
            assert_ne!(row.hash, before.row(&row.id).unwrap().hash);
        }
        assert_eq!(rebased.tags(&rebased.rows[0].id).count(), 2);
        let squashed = data.layout(BranchStrategy::Squash);
        assert_eq!(squashed.rows.len(), 6);
        assert_eq!(&*squashed.rows[0].parents[0], "m4");
        assert_eq!(squashed.rows[0].additions, Some(521));
        assert_eq!(options.commits[0].hash, before.rows[0].hash);
    }

    #[test]
    fn retarget_uses_the_live_rows_and_edges_without_a_jump() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (graph, window) =
            app.add_window_view(|_, cx| BranchGraph::new(BranchGraphOptions::default(), cx));
        window.update(|_, cx| {
            graph.update(cx, |s, cx| {
                s.flight = None;
                s.transition(BranchStrategy::Rebase, cx);
                s.flight.as_mut().unwrap().time = 0.37;
                let moving = s.current();
                assert!(moving.rows.iter().any(|p| p.lift > 0.));
                s.transition(BranchStrategy::Squash, cx);
                let next = s.current();
                assert_eq!(moving.height, next.height);
                for pose in &moving.rows {
                    let continued = next.rows.iter().find(|p| p.row.id == pose.row.id).unwrap();
                    assert_eq!(
                        (pose.y, pose.lane, pose.alpha, pose.lift),
                        (continued.y, continued.lane, continued.alpha, continued.lift)
                    );
                }
                for edge in &moving.edges {
                    let continued = next.edges.iter().find(|e| e.same(edge)).unwrap();
                    assert_eq!(edge.alpha, continued.alpha);
                }
            });
        });
    }

    #[test]
    fn closing_a_merge_releases_the_extra_row_only_after_landing() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (graph, window) = app.add_window_view(|_, cx| {
            BranchGraph::new(
                BranchGraphOptions {
                    default_strategy: BranchStrategy::Merge,
                    ..Default::default()
                },
                cx,
            )
        });
        window.update(|_, cx| {
            graph.update(cx, |s, cx| {
                s.flight = None;
                let merge_height = s.current().height;
                s.transition(BranchStrategy::Open, cx);
                s.flight.as_mut().unwrap().time = 0.31;
                assert_eq!(s.current().height, merge_height);
                s.flight = None;
                assert!(s.current().height < merge_height);
            });
        });
    }

    #[test]
    fn keyboard_moves_between_strategies_and_keeps_commit_focus() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (graph, window) = app.add_window_view(|_, cx| {
            BranchGraph::new(
                BranchGraphOptions {
                    speed: 0.,
                    ..Default::default()
                },
                cx,
            )
        });
        window.simulate_resize(size(px(320.), px(800.)));
        for _ in 0..2 {
            window.update(|w, cx| {
                w.simulate_next_frame(cx);
                w.refresh();
            });
            window.run_until_parked();
        }
        let board = window.debug_bounds("branch-commits").unwrap();
        assert!(board.right() <= px(320.));
        window.update(|w, cx| {
            let focus = graph.read(cx).strategy_focus[0].clone();
            focus.focus(w, cx);
        });
        window.simulate_keystrokes("right");
        assert_eq!(
            window.update(|_, cx| graph.read(cx).strategy()),
            BranchStrategy::Merge
        );
        window.update(|w, cx| {
            let focus = graph.read(cx).focus.clone();
            focus.focus(w, cx);
        });
        window.simulate_keystrokes("end");
        assert_eq!(
            window.update(|_, cx| graph.read(cx).keyboard.clone().unwrap().to_string()),
            "m1"
        );
        window.simulate_keystrokes("escape");
        assert!(window.update(|_, cx| graph.read(cx).keyboard.is_none()));
        assert!(window.update(|w, cx| graph.read(cx).focus.is_focused(w)));
    }

    #[test]
    fn controlled_selection_leaves_history_until_the_host_applies_it() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (graph, window) = app.add_window_view(|_, cx| {
            BranchGraph::new(
                BranchGraphOptions {
                    strategy: Some(BranchStrategy::Open),
                    ..Default::default()
                },
                cx,
            )
        });
        window.update(|_, cx| {
            graph.update(cx, |s, cx| {
                s.choose(BranchStrategy::Rebase, cx);
                assert_eq!(s.strategy(), BranchStrategy::Open);
                assert!(s.autoplay_stopped);
                assert!(s.announcement.is_some());
                s.set_strategy(BranchStrategy::Rebase, cx);
                assert_eq!(s.strategy(), BranchStrategy::Rebase);
            });
        });
    }
}
