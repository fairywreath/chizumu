use std::{mem::size_of, sync::Arc, usize::MAX};

use anyhow::Result;
use ash::vk;
use nalgebra::{Vector3, Vector4};

use crate::gpu::{
    command::CommandBuffer,
    device::{Device, MAX_FRAMES},
    resource::{
        Buffer, BufferDescriptor, DescriptorBindingBufferWrite, DescriptorBindingWrites,
        DescriptorSet, DescriptorSetDescriptor, DescriptorSetLayout, DescriptorSetLayoutDescriptor,
        Pipeline, PipelineDescriptor,
    },
    shader::{ShaderModuleDescriptor, ShaderStage},
};

const MAX_LINES: usize = 1024;
pub(crate) struct Line {
    point_a: Vector3<f32>,
    point_b: Vector3<f32>,
    thickness: f32,
    color: Vector4<f32>,
}

// impl Line {
//     fn new(point_a: Vector3<f32>, point_b: Vector3<f32>, thickness: f32) -> Self {
//         Self {
//             point_a,
//             point_b,
//             thickness,
//         }
//     }
// }

// struct LineDrawData {
//     line_: Vec<Line>,
// }

/// Can do vkCmdDrawInstancedIndirect for this one - for n lines we need n draw counts, 2 instances for each line(for the triangle) (?)
pub struct LineRenderer {
    // lines: Vec<Line>,

    // num_lines_to_draw:
    buffer_line_positions: Buffer,
    buffer_indices: Buffer,
    buffer_storage_line_data: Buffer,
    buffer_draw_indirect_command: Buffer,
    descriptor_sets: [DescriptorSet; MAX_FRAMES],
    graphics_pipeline: Pipeline,
    device: Arc<Device>,
}

impl LineRenderer {
    pub fn new(device: Arc<Device>) -> Self {
        todo!()
    }

    fn create_descriptor_set_layout(device: &Device) -> Result<DescriptorSetLayout> {
        todo!();
        let descriptor = DescriptorSetLayoutDescriptor {
            bindings: vec![vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build()],
            flags: vk::DescriptorSetLayoutCreateFlags::empty(),
        };

        device.create_descriptor_set_layout(descriptor)
    }

    fn create_graphics_pipeline(
        device: &Arc<Device>,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
    ) -> Result<Pipeline> {
        todo!();

        let vertex_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/lane.vs.glsl",
            shader_stage: ShaderStage::Vertex,
        })?;
        let fragment_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/lane.fs.glsl",
            shader_stage: ShaderStage::Fragment,
        })?;

        let vertex_input_attributes = vec![
            vk::VertexInputAttributeDescription::builder()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .build(),
            vk::VertexInputAttributeDescription::builder()
                .location(1)
                .binding(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .build(),
        ];
        let vertex_input_bindings = vec![
            vk::VertexInputBindingDescription::builder()
                .binding(0)
                .stride(12)
                .input_rate(vk::VertexInputRate::VERTEX)
                .build(),
            vk::VertexInputBindingDescription::builder()
                .binding(1)
                .stride(16)
                .input_rate(vk::VertexInputRate::VERTEX)
                .build(),
        ];

        // Only 1 render target.
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .build();

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::empty())
            .build();

        let pipeline_descriptor = PipelineDescriptor {
            descriptor_set_layouts: vec![descriptor_set_layout],
            shader_modules: vec![vertex_shader_module, fragment_shader_module],
            vertex_input_attributes,
            vertex_input_bindings,
            viewport_scissor_extent: device.swapchain_extent(),
            primitive_topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            color_blend_attachments: vec![color_blend_attachment],
            depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo::builder().build(),
            rasterization_state,
            color_attachment_formats: vec![device.swapchain_color_format()],
            depth_attachment_format: vk::Format::UNDEFINED,
        };

        device.create_pipeline(pipeline_descriptor)
    }
}
