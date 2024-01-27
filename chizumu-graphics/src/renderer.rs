use std::sync::Arc;

use anyhow::Result;
use ash::vk;
use gpu_allocator::MemoryLocation;
use nalgebra::{Isometry3, Matrix4, Orthographic3, Perspective3, Point3, Vector3};

use crate::{
    gpu::{
        device::{Device, MAX_FRAMES},
        resource::{Buffer, BufferDescriptor},
    },
    hit::{HitObject, HitRenderer},
    lane::LaneRenderer,
};

#[derive(Clone, Copy)]
#[repr(C)]
struct SceneConstants {
    view_projection: Matrix4<f32>,
}

/// A high-level renderer that performs game-specific draws.
pub struct Renderer {
    device: Arc<Device>,
    scene_constants_buffer: Buffer,
    lane_renderer: LaneRenderer,
    hit_renderer: HitRenderer,
}

impl Renderer {
    pub fn new(device: Arc<Device>) -> Result<Self> {
        let scene_constants_buffer = device.create_buffer(BufferDescriptor {
            size: std::mem::size_of::<SceneConstants>() as u64,
            usage_flags: vk::BufferUsageFlags::UNIFORM_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;

        let lane_renderer = LaneRenderer::new(device.clone())?;
        lane_renderer.write_gpu_resources(&scene_constants_buffer)?;

        let mut hit_renderer = HitRenderer::new(device.clone())?;
        hit_renderer.write_gpu_resources(&scene_constants_buffer)?;

        for i in 0..60 {
            let z_start = i as f32 * 7.0;
            hit_renderer.add_hit_objects(vec![
                HitObject::new(0.25, -1.0, z_start + 1.5),
                HitObject::new(0.25, 1.0, z_start + 1.5),
                HitObject::new(0.25, 0.0, z_start + 2.0),
                HitObject::new(0.25, 0.3, z_start + 2.5),
                HitObject::new(0.25, -0.3, z_start + 2.5),
                HitObject::new(0.25, -0.6, z_start + 3.0),
                HitObject::new(0.25, -0.9, z_start + 3.0),
                HitObject::new(0.25, 0.75, z_start + 3.5),
                HitObject::new(0.25, -1.0, z_start + 4.5),
                HitObject::new(0.25, 1.0, z_start + 4.5),
                HitObject::new(0.25, 0.0, z_start + 5.0),
                HitObject::new(0.25, 0.3, z_start + 5.5),
                HitObject::new(0.25, -0.3, z_start + 5.5),
                HitObject::new(0.25, -0.6, z_start + 6.0),
                HitObject::new(0.25, -0.9, z_start + 6.0),
                HitObject::new(0.25, 0.75, z_start + 6.5),
            ]);
        }

        Ok(Self {
            device,
            lane_renderer,
            hit_renderer,
            scene_constants_buffer,
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
            .command_begin_rendering_swapchain(&commands, [0.0, 0.0, 0.0, 1.0]);

        self.lane_renderer
            .write_render_commands(&commands, self.device.current_frame());
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

    pub fn update(&mut self, dt: f32) -> Result<()> {
        self.hit_renderer.update()?;

        Ok(())
    }

    pub fn advance_hit_runner(&mut self, advance_amount: f32) {
        self.hit_renderer.advance_runner(advance_amount);
    }

    fn update_scene_constants(&self) -> Result<()> {
        // XXX: Need to find good parameters for this
        let eye = Point3::new(0.0, -1.1, 0.2);
        let target = Point3::new(0.0, 0.5, 2.4);

        let view = Isometry3::look_at_rh(&eye, &target, &Vector3::y());
        let projection = Perspective3::new(1920.0 / 1200.0, 3.14 / 3.0, 0.01, 1000.0);
        let view_projection = projection.into_inner()
            * view.to_homogeneous()
            // XXX: Use view and projection matrices that fit accordingly to the vulkan coord system.
            * Matrix4::new_nonuniform_scaling(&Vector3::new(-1.0, 1.0, 1.0));

        let scene_constants = SceneConstants { view_projection };
        self.scene_constants_buffer
            .write_data(std::slice::from_ref(&scene_constants))?;

        Ok(())
    }
}
