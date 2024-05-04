use crate::mesh::plane::Plane;

pub(crate) mod hit;
pub(crate) mod lane;
pub(crate) mod platform;

pub const CURVE_SIDED_PLATFORM_BEZIER_SUBDIVISONS: u32 = 40;

#[derive(Clone)]
pub struct HitObject {
    /// The object's scale compared to the lane, 1.0 is max.
    pub x_scale: f32,
    /// Position of the object along the lane's width from -1.0 to 1.0(?).
    pub x_offset: f32,
    /// Position of the object along the lane, higher values mean the object
    /// is deep into the lane/track and will appear later.
    pub z_offset: f32,
}

impl HitObject {
    pub fn new(x_scale: f32, x_offset: f32, z_offset: f32) -> Self {
        Self {
            x_scale,
            x_offset,
            z_offset,
        }
    }
}

#[derive(Clone)]
pub struct DynamicPlanePlatform {
    runner_position_start: f32,
    runner_position_end: f32,
    plane_mesh: Plane,
}

#[derive(Clone)]
pub enum PlatformObject {
    DynamicPlane(DynamicPlanePlatform),
}

impl PlatformObject {
    pub fn new_dynamic_plane(
        runner_position_start: f32,
        runner_position_end: f32,
        plane_mesh: Plane,
    ) -> Self {
        Self::DynamicPlane(DynamicPlanePlatform {
            runner_position_start,
            runner_position_end,
            plane_mesh,
        })
    }
}
