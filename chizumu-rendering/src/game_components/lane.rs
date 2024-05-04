use std::{mem::size_of, sync::Arc, usize::MAX};

use anyhow::Result;
use chizumu_gpu::{
    ash::vk,
    command::CommandBuffer,
    device::{Device, MAX_FRAMES},
    gpu_allocator::MemoryLocation,
    resource::{
        Buffer, BufferDescriptor, DescriptorBindingBufferWrite, DescriptorBindingWrites,
        DescriptorSet, DescriptorSetDescriptor, DescriptorSetLayout, DescriptorSetLayoutDescriptor,
        Pipeline, PipelineDescriptor,
    },
    shader::{ShaderModuleDescriptor, ShaderStage},
    types::{DescriptorSetLayoutBinding, PipelineDepthStencilState, PipelineRasterizationState},
};
use nalgebra::{Matrix4, Vector2, Vector3, Vector4};

use crate::{
    game_components::hit::TAP_Z_RANGE,
    line::{LineData, LineRenderer},
    HIT_AREA_Z_START,
};

const MAX_SIMPLE_PLANES: usize = 24;

#[derive(Clone, Copy)]
struct SimplePlaneData {
    vertices: [Vector4<f32>; 4], // Use Vector4 for padding purposes.
    model: Matrix4<f32>,
    color: Vector4<f32>,
}

pub struct LaneRenderer {
    buffer_storage_simple_planes: Buffer,
    simple_planes: Vec<SimplePlaneData>,

    descriptor_sets: [DescriptorSet; MAX_FRAMES],
    graphics_pipeline: Pipeline,
    device: Arc<Device>,
}

impl LaneRenderer {
    pub fn new(device: Arc<Device>) -> Result<Self> {
        let buffer_storage_simple_planes = device.create_buffer(BufferDescriptor {
            size: (MAX_SIMPLE_PLANES * size_of::<SimplePlaneData>()) as u64,
            usage_flags: vk::BufferUsageFlags::STORAGE_BUFFER,
            // XXX: Find out how slow this is.
            memory_location: MemoryLocation::CpuToGpu,
        })?;

        let descriptor_set_layout = Arc::new(Self::create_descriptor_set_layout(&device)?);
        let graphics_pipeline =
            Self::create_graphics_pipeline(&device, descriptor_set_layout.clone())?;

        let descriptor_set_desc = DescriptorSetDescriptor {
            layout: descriptor_set_layout.clone(),
        };
        let descriptor_sets = [
            device.create_descriptor_set(descriptor_set_desc.clone())?,
            device.create_descriptor_set(descriptor_set_desc.clone())?,
        ];

        let mut simple_planes = vec![
            SimplePlaneData {
                vertices: [
                    Vector4::new(-1.0, 0.0, 0.85, 1.0),
                    Vector4::new(-0.5, 0.0, 0.85, 1.0),
                    Vector4::new(0.5, 0.0, 3.0, 1.0),
                    Vector4::new(1.0, 0.0, 3.0, 1.0),
                ],
                model: Matrix4::identity(),
                color: Vector4::new(0.0, 1.0, 0.0, 0.5),
            },
            SimplePlaneData {
                vertices: [
                    Vector4::new(0.5, 0.0, 3.0, 1.0),
                    Vector4::new(1.0, 0.0, 3.0, 1.0),
                    Vector4::new(0.5, 0.0, 6.0, 1.0),
                    Vector4::new(1.0, 0.0, 6.0, 1.0),
                ],
                model: Matrix4::identity(),
                color: Vector4::new(0.0, 1.0, 0.0, 0.5),
            },
            SimplePlaneData {
                vertices: [
                    Vector4::new(-1.0, 0.0, 6.0, 1.0),
                    Vector4::new(1.0, 0.0, 6.0, 1.0),
                    Vector4::new(-1.0, 0.0, 7.0, 1.0),
                    Vector4::new(1.0, 0.0, 7.0, 1.0),
                ],
                model: Matrix4::identity(),
                color: Vector4::new(0.0, 1.0, 0.0, 0.5),
            },
            SimplePlaneData {
                vertices: [
                    Vector4::new(-0.5, 0.0, 7.0, 1.0),
                    Vector4::new(-1.0, 0.0, 7.0, 1.0),
                    Vector4::new(-0.5, 0.0, 15.0, 1.0),
                    Vector4::new(-1.0, 0.0, 15.0, 1.0),
                ],
                model: Matrix4::identity(),
                color: Vector4::new(0.0, 1.0, 0.0, 0.5),
            },
        ];

        Ok(Self {
            buffer_storage_simple_planes,
            simple_planes,
            descriptor_sets,
            graphics_pipeline,
            device,
        })
    }

    pub(crate) fn write_render_commands(&self, command_buffer: &CommandBuffer, current_frame: u64) {
        command_buffer.bind_graphics_pipeline(&self.graphics_pipeline);
        command_buffer.bind_descriptor_set_graphics(
            &self.descriptor_sets[current_frame as usize],
            &self.graphics_pipeline,
        );

        command_buffer.draw(self.simple_planes.len() as u32 * 6, 1, 0, 0);
    }

    pub(crate) fn write_gpu_resources(&self, buffer_uniform_scene: &Buffer) -> Result<()> {
        let descriptor_binding_writes = DescriptorBindingWrites {
            buffers: vec![
                DescriptorBindingBufferWrite {
                    buffer: buffer_uniform_scene,
                    binding_index: 0,
                },
                DescriptorBindingBufferWrite {
                    buffer: &self.buffer_storage_simple_planes,
                    binding_index: 1,
                },
            ],
        };
        for descriptor_set in &self.descriptor_sets {
            self.device
                .update_descriptor_set(descriptor_set, descriptor_binding_writes.clone())?;
        }

        self.buffer_storage_simple_planes
            .write_data(&self.simple_planes)?;

        Ok(())
    }

    fn create_descriptor_set_layout(device: &Device) -> Result<DescriptorSetLayout> {
        let descriptor = DescriptorSetLayoutDescriptor {
            bindings: vec![
                DescriptorSetLayoutBinding::new()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
                DescriptorSetLayoutBinding::new()
                    .binding(1)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX),
            ],
            flags: vk::DescriptorSetLayoutCreateFlags::empty(),
        };

        device.create_descriptor_set_layout(descriptor)
    }

    fn create_graphics_pipeline(
        device: &Arc<Device>,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
    ) -> Result<Pipeline> {
        let vertex_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/platform.vs.glsl",
            shader_stage: ShaderStage::Vertex,
        })?;
        let fragment_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/platform.fs.glsl",
            shader_stage: ShaderStage::Fragment,
        })?;

        // Only 1 render target.
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .color_blend_op(vk::BlendOp::ADD)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

        let rasterization_state = PipelineRasterizationState::new()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::empty());

        let pipeline_descriptor = PipelineDescriptor {
            descriptor_set_layouts: vec![descriptor_set_layout],
            shader_modules: vec![vertex_shader_module, fragment_shader_module],
            vertex_input_attributes: Vec::new(),
            vertex_input_bindings: Vec::new(),
            viewport_scissor_extent: device.swapchain_extent(),
            primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            color_blend_attachments: vec![color_blend_attachment],
            depth_stencil_state: PipelineDepthStencilState::new(),
            rasterization_state,
            color_attachment_formats: vec![device.swapchain_color_format()],
            depth_attachment_format: vk::Format::UNDEFINED,
        };

        device.create_pipeline(pipeline_descriptor)
    }
}
