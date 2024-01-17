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
    lane::Lane,
};

#[derive(Clone, Copy)]
#[repr(C)]
struct SceneConstants {
    view_projection: Matrix4<f32>,
}

pub struct Renderer {
    device: Arc<Device>,
    scene_constants_buffer: Buffer,
    lane: Lane,
}

impl Renderer {
    pub fn new(device: Arc<Device>) -> Result<Self> {
        let scene_constants_buffer = device.create_buffer(BufferDescriptor {
            size: std::mem::size_of::<SceneConstants>() as u64,
            usage_flags: vk::BufferUsageFlags::UNIFORM_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;

        let lane = Lane::new(device.clone())?;
        lane.update_gpu_resources(&scene_constants_buffer)?;

        Ok(Self {
            device,
            lane,
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
            .command_begin_rendering_swapchain(&commands, [1.0, 1.0, 1.0, 1.0]);

        self.lane
            .write_render_commands(&commands, self.device.current_frame());

        commands.end_rendering();
        self.device
            .command_transition_swapchain_image_layout_to_present(&commands);
        commands.end()?;

        self.device.queue_submit_commands_graphics(commands)?;
        self.device.swapchain_present()?;

        Ok(())
    }

    fn update_scene_constants(&self) -> Result<()> {
        // XXX: Need to find good parameters for this
        let eye = Point3::new(0.0, -1.3, -3.0);
        let target = Point3::new(0.0, -0.0, 5.0);

        let view = Isometry3::look_at_rh(&eye, &target, &Vector3::y());
        let projection = Perspective3::new(1920.0 / 1200.0, 3.14 / 6.0, 0.001, 1000.0);
        let view_projection = projection.into_inner() * view.to_homogeneous();

        let scene_constants = SceneConstants { view_projection };
        self.scene_constants_buffer
            .write_data(std::slice::from_ref(&scene_constants))?;

        Ok(())
    }
}
