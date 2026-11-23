//! Shared pure-Rust coordination bridge between `#kense-guide` and the 3D studio renderer (`solids.rs`).
//!
//! Projects 3D fusion host anchors onto viewport coordinates every frame so the companion bot can
//! dive directly into hovered `#capabilities` Bento sculptures and `#products` 3D hardware models,
//! retract their resting symbols, trigger an elastic 3D impact pop, and animate 3D KUI eyes in-scene.

use std::{cell::RefCell, rc::Rc};

pub(super) const SCENE_COUNT: usize = 7;

pub(super) fn parse_scene_idx(kind: &str) -> Option<usize> {
    match kind {
        "runtime" => Some(0),
        "ui" => Some(1),
        "agent" => Some(2),
        "clay-vault" => Some(3),
        "clay-shield" => Some(4),
        "clay-release" => Some(5),
        "clay-rust" => Some(6),
        _ => None,
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct HostProjection {
    pub ready: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum Phase {
    #[default]
    Idle,
    Diving(f64),
    Attached,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Snapshot {
    pub active_scene: Option<usize>,
    pub phase: Phase,
    pub look_x: f32,
    pub look_y: f32,
    pub blink: f32,
    pub impact_seq: u32,
}

struct FusionState {
    projections: [HostProjection; SCENE_COUNT],
    snap: Snapshot,
    waker: Option<Rc<dyn Fn()>>,
}

impl Default for FusionState {
    fn default() -> Self {
        Self {
            projections: [HostProjection::default(); SCENE_COUNT],
            snap: Snapshot {
                active_scene: None,
                phase: Phase::Idle,
                look_x: 0.0,
                look_y: 0.0,
                blink: 1.0,
                impact_seq: 0,
            },
            waker: None,
        }
    }
}

thread_local! {
    static STATE: RefCell<FusionState> = RefCell::new(FusionState::default());
}

fn wake() {
    let cb = STATE.with(|s| s.borrow().waker.clone());
    if let Some(f) = cb {
        f();
    }
}

pub(super) fn register_waker(f: impl Fn() + 'static) {
    STATE.with(|s| {
        s.borrow_mut().waker = Some(Rc::new(f));
    });
}

pub(super) fn update_projection(scene_idx: usize, proj: HostProjection) {
    if scene_idx < SCENE_COUNT {
        STATE.with(|s| {
            s.borrow_mut().projections[scene_idx] = proj;
        });
    }
}

pub(super) fn projection(scene_idx: usize) -> Option<HostProjection> {
    if scene_idx >= SCENE_COUNT {
        return None;
    }
    STATE.with(|s| {
        let p = s.borrow().projections[scene_idx];
        p.ready.then_some(p)
    })
}

pub(super) fn snapshot() -> Snapshot {
    STATE.with(|s| s.borrow().snap)
}

pub(super) fn set_diving(scene_idx: usize, progress: f64) {
    let changed = STATE.with(|s| {
        let mut st = s.borrow_mut();
        let next = Phase::Diving(progress.clamp(0.0, 1.0));
        if st.snap.active_scene != Some(scene_idx) || st.snap.phase != next {
            st.snap.active_scene = Some(scene_idx);
            st.snap.phase = next;
            true
        } else {
            false
        }
    });
    if changed {
        wake();
    }
}

pub(super) fn set_attached(scene_idx: usize) {
    let changed = STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.snap.active_scene != Some(scene_idx) || st.snap.phase != Phase::Attached {
            st.snap.active_scene = Some(scene_idx);
            st.snap.phase = Phase::Attached;
            st.snap.impact_seq = st.snap.impact_seq.wrapping_add(1);
            true
        } else {
            false
        }
    });
    if changed {
        wake();
    }
}

pub(super) fn set_gaze(look_x: f32, look_y: f32, blink: f32) {
    let changed = STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.snap.active_scene.is_none() {
            return false;
        }
        let lx = look_x.clamp(-1.0, 1.0);
        let ly = look_y.clamp(-1.0, 1.0);
        let bl = blink.clamp(0.1, 1.0);
        let diff =
            (st.snap.look_x - lx).abs() + (st.snap.look_y - ly).abs() + (st.snap.blink - bl).abs();
        if diff > 0.004 {
            st.snap.look_x = lx;
            st.snap.look_y = ly;
            st.snap.blink = bl;
            true
        } else {
            false
        }
    });
    if changed {
        wake();
    }
}

pub(super) fn clear() {
    let changed = STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.snap.active_scene.is_some() || st.snap.phase != Phase::Idle {
            st.snap.active_scene = None;
            st.snap.phase = Phase::Idle;
            true
        } else {
            false
        }
    });
    if changed {
        wake();
    }
}
