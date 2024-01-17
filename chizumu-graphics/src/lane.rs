use std::{mem::size_of, sync::Arc, usize::MAX};

use anyhow::Result;
use ash::vk;
use gpu_allocator::MemoryLocation;
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

pub struct Lane {
    /// GPU resources for a simple 2D plane.
    position_buffer: Buffer,
    color_buffer: Buffer,
    index_buffer: Buffer,

    descriptor_set_layout: Arc<DescriptorSetLayout>,
    descriptor_sets: [DescriptorSet; MAX_FRAMES],
    graphics_pipeline: Pipeline,

    device: Arc<Device>,
}

impl Lane {
    pub fn new(device: Arc<Device>) -> Result<Self> {
        let position_buffer_desc = BufferDescriptor {
            size: 4 * 3 * (size_of::<f32>() as u64), // 4 Vector3's
            usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        };
        let position_buffer = device.create_buffer(position_buffer_desc)?;

        let color_buffer_desc = BufferDescriptor {
            size: 4 * 4 * (size_of::<f32>() as u64), // 4 Vector4's
            usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        };
        let color_buffer = device.create_buffer(color_buffer_desc)?;

        let index_buffer_desc = BufferDescriptor {
            size: 6 * (size_of::<u16>() as u64),
            usage_flags: vk::BufferUsageFlags::INDEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        };
        let index_buffer = device.create_buffer(index_buffer_desc)?;

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

        Ok(Self {
            device,
            position_buffer,
            color_buffer,
            index_buffer,
            descriptor_set_layout,
            descriptor_sets,
            graphics_pipeline,
        })
    }

    pub fn write_render_commands(&self, command_buffer: &CommandBuffer, current_frame: u64) {
        command_buffer.bind_graphics_pipeline(&self.graphics_pipeline);
        command_buffer.bind_descriptor_set_graphics(
            &self.descriptor_sets[current_frame as usize],
            &self.graphics_pipeline,
        );

        command_buffer.bind_vertex_buffers(
            0,
            &[&self.position_buffer, &self.color_buffer],
            &[0, 0],
        );
        command_buffer.bind_index_buffer(&self.index_buffer, 0);

        command_buffer.draw_indexed(6, 1, 0, 0, 0);
    }

    pub fn update_gpu_resources(&self, scene_uniform_buffer: &Buffer) -> Result<()> {
        let position_data: Vec<[f32; 3]> = vec![
            [-1.0, 0.0, -20.0],
            [1.0, 0.0, -20.0],
            [-1.0, 0.0, 20.0],
            [1.0, 0.0, 20.0],
        ];
        self.position_buffer.write_data(&position_data)?;

        let color = [0.05, 0.12, 0.1, 1.0];
        let color_data: Vec<[f32; 4]> = vec![color, color, color, color];
        self.color_buffer.write_data(&color_data)?;

        let index_data: Vec<u16> = vec![0, 1, 2, 1, 2, 3];
        self.index_buffer.write_data(&index_data)?;

        let descriptor_binding_writes = DescriptorBindingWrites {
            buffers: vec![DescriptorBindingBufferWrite {
                buffer: scene_uniform_buffer,
                binding_index: 0,
            }],
        };
        for descriptor_set in &self.descriptor_sets {
            self.device
                .update_descriptor_set(descriptor_set, descriptor_binding_writes.clone())?;
        }

        Ok(())
    }

    fn create_descriptor_set_layout(device: &Device) -> Result<DescriptorSetLayout> {
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
        device: &Device,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
    ) -> Result<Pipeline> {
        let vertex_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/simple.vert.glsl",
            shader_stage: ShaderStage::Vertex,
        })?;
        let fragment_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/simple.frag.glsl",
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
