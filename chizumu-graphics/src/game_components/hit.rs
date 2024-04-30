/*! Game hit objects.
 */

use std::{mem::size_of, sync::Arc, usize::MAX};

use anyhow::Result;
use ash::vk;
use gpu_allocator::MemoryLocation;
use nalgebra::{Matrix4, Vector3, Vector4};

use crate::{
    game_components::HitObject,
    gpu::{
        command::CommandBuffer,
        device::{Device, MAX_FRAMES},
        resource::{
            Buffer, BufferDescriptor, DescriptorBindingBufferWrite, DescriptorBindingWrites,
            DescriptorSet, DescriptorSetDescriptor, DescriptorSetLayout,
            DescriptorSetLayoutDescriptor, Pipeline, PipelineDescriptor,
        },
        shader::{ShaderModuleDescriptor, ShaderStage},
    },
};

pub const TAP_Z_RANGE: f32 = 0.14;
const MAX_HIT_OBJECT_INSTANCE_COUNT: usize = 2048;

#[derive(Clone, Copy)]
#[repr(C)]
struct InstanceData {
    model: Matrix4<f32>,
    color: Vector4<f32>,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct RunnerData {
    model: Matrix4<f32>,
}

pub(crate) struct HitRenderer {
    // Drawn with instancing.
    buffer_position_hit_objects: Buffer,
    buffer_index_hit_objects: Buffer,
    buffer_instance_data_hit_objects: Buffer,

    buffer_uniform_runner_data: Buffer,

    current_first_instance: u32,
    current_instance_count: u32,

    hit_objects: Vec<HitObject>,
    hit_objects_instance_data: Vec<InstanceData>,

    runner_position: f32,

    descriptor_sets: [DescriptorSet; MAX_FRAMES],
    graphics_pipeline: Pipeline,

    device: Arc<Device>,
}

impl HitRenderer {
    pub(crate) fn new(device: Arc<Device>) -> Result<Self> {
        let buffer_position_hit_objects = device.create_buffer(BufferDescriptor {
            size: 8 * 3 * (size_of::<f32>() as u64),
            usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let buffer_index_hit_objects = device.create_buffer(BufferDescriptor {
            size: 36 * (size_of::<u16>() as u64),
            usage_flags: vk::BufferUsageFlags::INDEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let buffer_instance_data_hit_objects = device.create_buffer(BufferDescriptor {
            size: (MAX_HIT_OBJECT_INSTANCE_COUNT * size_of::<InstanceData>()) as u64,
            usage_flags: vk::BufferUsageFlags::STORAGE_BUFFER,
            // XXX: Find out how slow this is.
            memory_location: MemoryLocation::CpuToGpu,
        })?;

        let buffer_uniform_runner_data = device.create_buffer(BufferDescriptor {
            size: size_of::<RunnerData>() as _,
            usage_flags: vk::BufferUsageFlags::UNIFORM_BUFFER,
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
            buffer_position_hit_objects,
            buffer_index_hit_objects,
            buffer_instance_data_hit_objects,
            buffer_uniform_runner_data,
            descriptor_sets,
            graphics_pipeline,
            current_first_instance: 0,
            current_instance_count: 0,
            hit_objects: Vec::new(),
            hit_objects_instance_data: Vec::new(),
            runner_position: 0.0,
        })
    }

    pub(crate) fn update(&self) -> Result<()> {
        let runnner_data = RunnerData {
            model: Matrix4::new_translation(&Vector3::new(0.0, 0.0, -self.runner_position)),
        };
        self.buffer_uniform_runner_data
            .write_data(&[runnner_data])?;

        Ok(())
    }

    pub(crate) fn advance_runner(&mut self, advance_amount: f32) {
        self.runner_position += advance_amount;
    }

    pub(crate) fn get_runner_position(&self) -> f32 {
        self.runner_position
    }

    pub(crate) fn add_hit_objects(&mut self, hit_objects: &[HitObject]) {
        for object in hit_objects {
            let left_edge_x = -1.0;

            let instance_data = InstanceData {
                model: Matrix4::new_translation(&Vector3::new(
                    // object.x_offset + left_edge_x,
                    object.x_offset,
                    0.0,
                    object.z_offset,
                )) * Matrix4::new_nonuniform_scaling(&Vector3::new(
                    object.x_scale,
                    1.0,
                    1.0,
                )),
                color: Vector4::new(1.0, 0.0, 0.0, 1.0),
            };
            self.hit_objects_instance_data.push(instance_data);
            self.hit_objects.push(object.clone());
        }

        // XXX: More work required on deciding what is drawn per frame based on this data.
        // Need to properly decide when to write to SSBO.
        self.current_first_instance = 0;
        self.current_instance_count = self.hit_objects_instance_data.len() as _;
        self.buffer_instance_data_hit_objects
            .write_data(&self.hit_objects_instance_data)
            .unwrap();
    }

    pub(crate) fn write_render_commands(&self, command_buffer: &CommandBuffer, current_frame: u64) {
        // self.write_gpu_resources_hit_objects().unwrap();

        command_buffer.bind_graphics_pipeline(&self.graphics_pipeline);
        command_buffer.bind_descriptor_set_graphics(
            &self.descriptor_sets[current_frame as usize],
            &self.graphics_pipeline,
        );

        command_buffer.bind_vertex_buffers(0, &[&self.buffer_position_hit_objects], &[0]);
        command_buffer.bind_index_buffer(&self.buffer_index_hit_objects, 0);
        command_buffer.draw_indexed(
            36,
            self.current_instance_count,
            0,
            0,
            self.current_first_instance,
        )
    }

    pub(crate) fn write_gpu_resources(&self, buffer_uniform_scene: &Buffer) -> Result<()> {
        self.write_gpu_resources_hit_objects()?;

        // let eye = Point3::new(0.0, 0.0, 0.0);
        // let target = Point3::new(0.0, 0.4, 4.5);
        // let view = Isometry3::look_at_rh(&eye, &target, &Vector3::y());
        let runnner_data = RunnerData {
            // model: Matrix4::new_translation(&Vector3::new(0.0, 0.0, 10.0)),
            model: Matrix4::new_translation(&Vector3::new(0.0, 0.0, 0.0)),
        };
        self.buffer_uniform_runner_data
            .write_data(&[runnner_data])?;

        let descriptor_binding_writes = DescriptorBindingWrites {
            buffers: vec![
                DescriptorBindingBufferWrite {
                    buffer: buffer_uniform_scene,
                    binding_index: 0,
                },
                DescriptorBindingBufferWrite {
                    buffer: &self.buffer_uniform_runner_data,
                    binding_index: 1,
                },
                DescriptorBindingBufferWrite {
                    buffer: &self.buffer_instance_data_hit_objects,
                    binding_index: 2,
                },
            ],
        };
        for descriptor_set in &self.descriptor_sets {
            self.device
                .update_descriptor_set(descriptor_set, descriptor_binding_writes.clone())?;
        }

        Ok(())
    }

    fn write_gpu_resources_hit_objects(&self) -> Result<()> {
        let x_range = [0.0, 2.0];
        let y_range = [0.0, -0.08];
        let z_range = [0.0, TAP_Z_RANGE];

        let position_data: Vec<[f32; 3]> = vec![
            // Front face.
            [x_range[0], y_range[0], z_range[0]],
            [x_range[1], y_range[0], z_range[0]],
            [x_range[0], y_range[1], z_range[0]],
            [x_range[1], y_range[1], z_range[0]],
            // Back face.
            [x_range[0], y_range[0], z_range[1]],
            [x_range[1], y_range[0], z_range[1]],
            [x_range[0], y_range[1], z_range[1]],
            [x_range[1], y_range[1], z_range[1]],
        ];
        self.buffer_position_hit_objects
            .write_data(&position_data)?;

        let buffer_index_data: [u16; 36] = [
            0, 1, 2, 1, 2, 3, // Front face.
            4, 5, 6, 5, 6, 7, // Back face.
            4, 0, 6, 0, 6, 2, // Left face.
            1, 5, 3, 5, 3, 7, // Right face.
            2, 3, 6, 3, 6, 7, // Top face.
            0, 1, 4, 1, 4, 5, // Bottom face.
        ];
        self.buffer_index_hit_objects
            .write_data(&buffer_index_data)?;

        self.buffer_instance_data_hit_objects
            .write_data(&self.hit_objects_instance_data)?;

        Ok(())
    }

    fn create_descriptor_set_layout(device: &Device) -> Result<DescriptorSetLayout> {
        let descriptor = DescriptorSetLayoutDescriptor {
            bindings: vec![
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(2)
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .stage_flags(vk::ShaderStageFlags::VERTEX)
                    .build(),
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
            source_file_name: "shaders/hit.vs.glsl",
            shader_stage: ShaderStage::Vertex,
        })?;
        let fragment_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/hit.fs.glsl",
            shader_stage: ShaderStage::Fragment,
        })?;

        let vertex_input_attributes = vec![vk::VertexInputAttributeDescription::builder()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .build()];
        let vertex_input_bindings = vec![vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(12)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()];

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
