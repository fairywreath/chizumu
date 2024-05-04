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
use nalgebra::{Matrix4, Vector3, Vector4};

use flo_curves::bezier;
use flo_curves::*;

const MAX_LINES: usize = 1024;

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct LineData {
    point_a: Vector3<f32>,
    _pad0: f32,
    point_b: Vector3<f32>,
    thickness: f32,
    color: Vector4<f32>,

    model: Matrix4<f32>,
}

impl LineData {
    pub(crate) fn new(
        point_a: Vector3<f32>,
        point_b: Vector3<f32>,
        thickness: f32,
        color: Vector4<f32>,
    ) -> Self {
        Self {
            point_a,
            point_b,
            thickness,
            color,
            _pad0: 0.0,
            model: Matrix4::identity(),
        }
    }
}

pub struct LineRenderer {
    buffer_storage_line_data: Buffer,
    line_data: Vec<LineData>,
    descriptor_sets: [DescriptorSet; MAX_FRAMES],
    graphics_pipeline: Pipeline,
    device: Arc<Device>,
}

impl LineRenderer {
    pub fn new(device: Arc<Device>) -> Result<Self> {
        let buffer_storage_line_data = device.create_buffer(BufferDescriptor {
            size: (MAX_LINES * size_of::<LineData>()) as u64,
            usage_flags: vk::BufferUsageFlags::STORAGE_BUFFER,
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

        let mut line_data = vec![];

        // Test flo curves.
        let curve = bezier::Curve::from_points(
            Coord2(0.0, 1.0),
            (Coord2(-1.9, 3.0), Coord2(0.9, 5.0)),
            Coord2(0.0, 7.0),
        );

        let precision = 1000;
        let step = 0.01;
        let curve_points = (0..precision)
            .step_by((step * precision as f32) as _)
            .map(|t| curve.point_at_pos(t as f64 / precision as f64))
            .collect::<Vec<_>>();

        for (i, point) in curve_points.iter().enumerate() {
            if i == 0 {
                continue;
            }

            let prev_point = Vector3::new(
                curve_points[i - 1].0 as f32,
                0.0,
                curve_points[i - 1].1 as f32,
            );
            let curr_point = Vector3::new(curve_points[i].0 as f32, 0.0, curve_points[i].1 as f32);

            line_data.push(LineData::new(
                prev_point,
                curr_point,
                2.0,
                Vector4::new(1.0, 1.0, 1.0, 1.0),
            ));
        }

        // log::debug!("Bezier curve points len {}", curve_points.len());

        Ok(Self {
            line_data,
            buffer_storage_line_data,
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

        command_buffer.draw(self.line_data.len() as u32 * 6, 1, 0, 0);
    }

    pub(crate) fn write_gpu_resources(&self, buffer_uniform_scene: &Buffer) -> Result<()> {
        let descriptor_binding_writes = DescriptorBindingWrites {
            buffers: vec![
                DescriptorBindingBufferWrite {
                    buffer: buffer_uniform_scene,
                    binding_index: 0,
                },
                DescriptorBindingBufferWrite {
                    buffer: &self.buffer_storage_line_data,
                    binding_index: 1,
                },
            ],
        };
        for descriptor_set in &self.descriptor_sets {
            self.device
                .update_descriptor_set(descriptor_set, descriptor_binding_writes.clone())?;
        }

        self.write_gpu_resources_line_data()?;

        Ok(())
    }

    // XXX: Properly use channels.
    pub(crate) fn add_lines(&mut self, lines: &[LineData]) {
        self.line_data.extend_from_slice(lines);

        // XXX: Handle this more gracefully.
        self.write_gpu_resources_line_data().unwrap();
    }

    fn write_gpu_resources_line_data(&self) -> Result<()> {
        self.buffer_storage_line_data.write_data(&self.line_data)?;

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
            source_file_name: "shaders/line.vs.glsl",
            shader_stage: ShaderStage::Vertex,
        })?;
        let fragment_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/line.fs.glsl",
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
