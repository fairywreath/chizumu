use std::{mem::size_of, sync::Arc, usize::MAX};

use anyhow::Result;
use ash::vk;
use gpu_allocator::MemoryLocation;
use nalgebra::{Vector2, Vector3, Vector4};

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

struct LaneParameters {
    /// Lane base position.
    x_range: Vector2<f32>,
    y_range: Vector2<f32>,
    z_range: Vector2<f32>,
    num_primary_lanes: usize,
    lane_separator_width: f32,
    color_base: Vector4<f32>,
    color_separator: Vector4<f32>,
}

pub struct LaneRenderer {
    /// XXX: Combine everything in GPU and optimize draws.
    ///
    /// GPU resources for base.
    buffer_position: Buffer,
    buffer_color: Buffer,
    buffer_index: Buffer,
    /// GPU resources for base markings and overlay, eg. lane separators.
    buffer_position_overlay: Buffer,
    buffer_color_overlay: Buffer,
    buffer_index_overlay: Buffer,

    parameters: LaneParameters,
    num_separators: usize,
    descriptor_sets: [DescriptorSet; MAX_FRAMES],
    graphics_pipeline: Pipeline,
    device: Arc<Device>,
}

impl LaneRenderer {
    pub fn new(device: Arc<Device>) -> Result<Self> {
        let parameters = LaneParameters {
            x_range: Vector2::new(-1.0, 1.0),
            y_range: Vector2::new(0.0, 0.0), // base is a 2D plane
            z_range: Vector2::new(-1.0, 20.0),
            num_primary_lanes: 8,
            lane_separator_width: 0.003,
            color_base: Vector4::new(0.3, 0.2, 0.8, 1.0),
            color_separator: Vector4::new(0.8, 0.8, 0.8, 1.0),
        };

        let buffer_position = device.create_buffer(BufferDescriptor {
            size: 4 * 3 * (size_of::<f32>() as u64), // 4 Vector3's
            usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let buffer_color = device.create_buffer(BufferDescriptor {
            size: 4 * 4 * (size_of::<f32>() as u64), // 4 Vector4's
            usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let buffer_index = device.create_buffer(BufferDescriptor {
            size: 6 * (size_of::<u16>() as u64),
            usage_flags: vk::BufferUsageFlags::INDEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;

        let num_separators = (parameters.num_primary_lanes + 1) as u64;
        let buffer_position_overlay = device.create_buffer(BufferDescriptor {
            size: 4 * 3 * (size_of::<f32>() as u64) * num_separators,
            usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let buffer_color_overlay = device.create_buffer(BufferDescriptor {
            size: 4 * 4 * (size_of::<f32>() as u64) * num_separators,
            usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let buffer_index_overlay = device.create_buffer(BufferDescriptor {
            size: 6 * (size_of::<u16>() as u64) * num_separators,
            usage_flags: vk::BufferUsageFlags::INDEX_BUFFER,
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

        Ok(Self {
            device,
            buffer_position,
            buffer_color,
            buffer_index,
            descriptor_sets,
            graphics_pipeline,
            parameters,
            buffer_position_overlay,
            buffer_index_overlay,
            buffer_color_overlay,
            num_separators: num_separators as _,
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
            &[&self.buffer_position, &self.buffer_color],
            &[0, 0],
        );
        command_buffer.bind_index_buffer(&self.buffer_index, 0);
        command_buffer.draw_indexed(6, 1, 0, 0, 0);

        command_buffer.bind_vertex_buffers(
            0,
            &[&self.buffer_position_overlay, &self.buffer_color_overlay],
            &[0, 0],
        );
        command_buffer.bind_index_buffer(&self.buffer_index_overlay, 0);
        command_buffer.draw_indexed(6 * (self.num_separators as u32), 1, 0, 0, 0);
    }

    pub fn write_gpu_resources(&self, scene_uniform_buffer: &Buffer) -> Result<()> {
        self.write_gpu_resources_base()?;
        self.write_gpu_resources_overlay()?;

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

    fn write_gpu_resources_base(&self) -> Result<()> {
        let position_data = [
            [self.parameters.x_range[0], 0.0, self.parameters.z_range[0]],
            [self.parameters.x_range[1], 0.0, self.parameters.z_range[0]],
            [self.parameters.x_range[0], 0.0, self.parameters.z_range[1]],
            [self.parameters.x_range[1], 0.0, self.parameters.z_range[1]],
        ];
        self.buffer_position.write_data(&position_data)?;

        let buffer_color_data = [self.parameters.color_base.clone(); 4];
        self.buffer_color.write_data(&buffer_color_data)?;

        let buffer_index_data: [u16; 6] = [0, 1, 2, 1, 2, 3];
        self.buffer_index.write_data(&buffer_index_data)?;

        Ok(())
    }

    fn write_gpu_resources_overlay(&self) -> Result<()> {
        let primary_lane_width = (self.parameters.x_range[0] - self.parameters.x_range[1]).abs()
            / self.parameters.num_primary_lanes as f32;

        let mut buffer_position_overlay_data = Vec::with_capacity(4 * self.num_separators as usize);
        for i in 0..self.num_separators {
            let lane_center_x = -1.0 + (i as f32 * primary_lane_width);
            // The separator is a thin line drawn as a quad/plane.
            for j in 0..2 {
                buffer_position_overlay_data.push([
                    lane_center_x - self.parameters.lane_separator_width,
                    0.0,
                    self.parameters.z_range[j],
                ]);
                buffer_position_overlay_data.push([
                    lane_center_x + self.parameters.lane_separator_width,
                    0.0,
                    self.parameters.z_range[j],
                ]);
            }
        }
        self.buffer_position_overlay
            .write_data(&buffer_position_overlay_data)?;

        let buffer_color_overlay_data =
            vec![self.parameters.color_separator.clone(); 4 * self.num_separators];
        self.buffer_color_overlay
            .write_data(&buffer_color_overlay_data)?;

        let mut buffer_index_overlay_data = Vec::<u16>::with_capacity(6 * self.num_separators);
        for i in 0..self.num_separators as u16 {
            let current_base_index = i * 4;
            buffer_index_overlay_data.extend([
                current_base_index,
                current_base_index + 1,
                current_base_index + 2,
                current_base_index + 1,
                current_base_index + 2,
                current_base_index + 3,
            ]);
        }
        self.buffer_index_overlay
            .write_data(&buffer_index_overlay_data)?;

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
        device: &Arc<Device>,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
    ) -> Result<Pipeline> {
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
