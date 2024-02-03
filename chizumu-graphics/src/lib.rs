pub mod gpu;
pub mod hit;
pub mod renderer;

mod lane;
mod line;

/// "Bottom" z-axis start offset of the hit area.
pub const HIT_AREA_Z_START: f32 = 0.85;
