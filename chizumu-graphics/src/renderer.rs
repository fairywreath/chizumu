use std::sync::Arc;

use anyhow::Result;
use ash::vk;
use gpu_allocator::MemoryLocation;
use nalgebra::{Isometry3, Matrix4, Orthographic3, Perspective3, Point3, Vector2, Vector3};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::{
    game_components::{
        hit::HitRenderer,
        lane::{self, LaneRenderer},
        platform::PlatformRenderer,
        DynamicPlanePlatform, HitObject, PlatformObject,
    },
    gpu::{
        device::{Device, MAX_FRAMES},
        resource::{Buffer, BufferDescriptor},
    },
    line::LineRenderer,
    mesh::plane::Plane,
    HIT_AREA_Z_START,
};

#[derive(Clone, Copy)]
#[repr(C)]
struct SceneConstantsGpuData {
    view_projection: Matrix4<f32>,
    viewport: Vector2<u32>,
    _pad0: Vector2<u32>,
    runner: Matrix4<f32>,
}

/// A high-level renderer that performs game-specific draws.
pub struct Renderer {
    device: Arc<Device>,
    scene_constants_buffer: Buffer,

    runner_position: f32,

    platform_renderer: PlatformRenderer,
    hit_renderer: HitRenderer,
    // lane_renderer: LaneRenderer,
    // line_renderer: LineRenderer,
}

impl Renderer {
    pub fn new(
        window_handle: &dyn HasRawWindowHandle,
        display_handle: &dyn HasRawDisplayHandle,
    ) -> Result<Self> {
        let device = Arc::new(Device::new(window_handle, display_handle)?);

        let scene_constants_buffer = device.create_buffer(BufferDescriptor {
            size: std::mem::size_of::<SceneConstantsGpuData>() as u64,
            usage_flags: vk::BufferUsageFlags::UNIFORM_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;

        let platform_renderer = PlatformRenderer::new(device.clone())?;
        platform_renderer.write_initital_gpu_resources(&scene_constants_buffer)?;

        let hit_renderer = HitRenderer::new(device.clone())?;
        hit_renderer.write_gpu_resources(&scene_constants_buffer)?;

        Ok(Self {
            device,
            scene_constants_buffer,
            runner_position: 0.0,
            platform_renderer,
            // lane_renderer,
            hit_renderer,
            // line_renderer,
        })
    }

    pub fn render(&self) -> Result<()> {
        self.update_scene_constants()?;

        self.device.frame_begin()?;

        let commands = self.device.get_current_command_buffer()?;
        commands.begin()?;
        self.device
            .command_transition_swapchain_image_layout_to_color_attachment(&commands);
        self.device
            .command_begin_rendering_swapchain(&commands, [1.0, 1.0, 1.0, 1.0]);

        self.platform_renderer
            .write_render_commands(&commands, self.device.current_frame());

        // self.lane_renderer
        //     .write_render_commands(&commands, self.device.current_frame());

        // self.line_renderer
        //     .write_render_commands(&commands, self.device.current_frame());

        self.hit_renderer
            .write_render_commands(&commands, self.device.current_frame());

        commands.end_rendering();
        self.device
            .command_transition_swapchain_image_layout_to_present(&commands);
        commands.end()?;

        self.device.queue_submit_commands_graphics(commands)?;
        self.device.swapchain_present()?;

        Ok(())
    }

    pub fn update(&mut self, frame_dt: f32, runner_dp: f32) -> Result<()> {
        self.hit_renderer.update()?;
        self.hit_renderer.advance_runner(runner_dp);

        self.runner_position += runner_dp;

        self.platform_renderer
            .update_with_runner_position(self.runner_position);

        Ok(())
    }

    fn update_scene_constants(&self) -> Result<()> {
        // XXX TODO: Need to find good parameters for this
        let eye = Point3::new(0.0, -1.54, 0.2);
        let target = Point3::new(0.0, 0.7, 3.0);

        let view = Isometry3::look_at_rh(&eye, &target, &Vector3::y());
        let projection = Perspective3::new(1920.0 / 1200.0, 3.14 / 3.0, 0.01, 1000.0);
        let view_projection = projection.into_inner()
            * view.to_homogeneous()
            // XXX: Use view and projection matrices that fit accordingly to the vulkan coord system. (?)
            * Matrix4::new_nonuniform_scaling(&Vector3::new(-1.0, 1.0, 1.0));

        let scene_constants = SceneConstantsGpuData {
            view_projection,
            viewport: Vector2::new(1920, 1200),
            runner: Matrix4::new_translation(&Vector3::new(0.0, 0.0, -self.runner_position)),
            _pad0: Vector2::identity(),
        };
        self.scene_constants_buffer
            .write_data(std::slice::from_ref(&scene_constants))?;

        Ok(())
    }

    pub fn set_platform_objects(&mut self, platform_objects: Vec<PlatformObject>) -> Result<()> {
        self.platform_renderer
            .set_platforms_objects(platform_objects)?;

        Ok(())
    }

    pub fn add_hit_objects(&mut self, hit_objects: &[HitObject]) {
        self.hit_renderer.add_hit_objects(hit_objects);
    }
}
