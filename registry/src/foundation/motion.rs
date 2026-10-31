//! KUI motion presets. Always branch on [`crate::foundation::Theme::reduced_motion`].

#[derive(Clone, Copy, Debug)]
pub struct MotionPreset {
    pub duration_ms: u64,
    pub ease: &'static str,
}

#[derive(Clone, Debug)]
pub struct MotionTokens {
    pub snappy: MotionPreset,
    pub smooth: MotionPreset,
    pub morph: MotionPreset,
    pub instant: MotionPreset,
}

pub const MOTION: MotionTokens = MotionTokens {
    snappy: MotionPreset {
        duration_ms: 120,
        ease: "standard",
    },
    smooth: MotionPreset {
        duration_ms: 220,
        ease: "emphasized",
    },
    morph: MotionPreset {
        duration_ms: 280,
        ease: "emphasized",
    },
    instant: MotionPreset {
        duration_ms: 80,
        ease: "standard",
    },
};

impl MotionTokens {
    pub fn effective(&self, preset: MotionPreset, reduced_motion: bool) -> MotionPreset {
        if reduced_motion {
            MotionPreset {
                duration_ms: 0,
                ease: "linear",
            }
        } else {
            preset
        }
    }
}
