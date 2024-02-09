use std::{mem::size_of, sync::Arc};

use anyhow::{anyhow, Result};
use ash::vk;
use gpu_allocator::MemoryLocation;
use nalgebra::{Matrix4, Vector2, Vector3, Vector4};

use crate::{
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
    mesh::plane::Plane,
};

use super::PlatformObject;

const MAX_TOTAL_VERTICES_PER_PLATFORM_BUFFER: u64 = 4096;
const MAX_TOTAL_INDICES_PER_PLATFORM_BUFFER: u64 = 8192;
const MAX_PLATFORM_INSTANCES: u64 = 4096;

const QUAD_PLATFORM_VERTEX_COUNT: u32 = 4;
const CURVE_SIDED_PLATFORM_VERTEX_COUNT: u32 = 82;

#[derive(Clone)]
struct GlobalPlatformParameters {
    z_range: Vector2<f32>,
    base_color: Vector4<f32>,
}

struct DrawRange {
    index_offset: u32,
    index_count: u32,
    first_platform_index: usize,
    last_platform_index: usize,
}

impl DrawRange {
    fn new() -> Self {
        Self {
            index_offset: 0,
            index_count: 0,
            first_platform_index: 0,
            last_platform_index: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct PlatformInstanceGpuData {
    _model: Matrix4<f32>,
}

impl PlatformInstanceGpuData {
    fn new(_model: Matrix4<f32>) -> Self {
        Self { _model }
    }
}

/// Renders platforms of a single type, i.e. platform objects with the same number of vertices/indices per instance.
/// Does not own the instance data SSBO, a single global data SSBO is used, where the upper 16 bits of gl_InstanceIndex.
struct SingleMeshTypePlatformRenderer {
    max_vertices: u64,
    max_indices: u64,
    max_platform_instances: u64,
    vertex_count_per_instance: u32,

    draw_storage_buffer_offset: u64,
    draw_range: DrawRange,
    platforms: Vec<PlatformObject>,
    global_parameters: GlobalPlatformParameters,

    buffer_positions: Buffer,
    buffer_indices: Buffer,
    device: Arc<Device>,
}

impl SingleMeshTypePlatformRenderer {
    fn new(
        device: Arc<Device>,
        global_parameters: GlobalPlatformParameters,
        max_vertices: u64,
        max_indices: u64,
        max_platform_instances: u64,
        vertex_count_per_instance: u32,
        draw_storage_buffer_offset: u64,
    ) -> Result<Self> {
        let buffer_positions = device.create_buffer(BufferDescriptor {
            size: max_vertices * size_of::<Vector3<f32>>() as u64,
            usage_flags: vk::BufferUsageFlags::VERTEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;
        let buffer_indices = device.create_buffer(BufferDescriptor {
            size: max_indices * size_of::<u16>() as u64,
            usage_flags: vk::BufferUsageFlags::INDEX_BUFFER,
            memory_location: MemoryLocation::CpuToGpu,
        })?;

        Ok(Self {
            max_vertices,
            max_indices,
            max_platform_instances,
            vertex_count_per_instance,
            platforms: Vec::new(),
            draw_range: DrawRange::new(),
            draw_storage_buffer_offset,
            buffer_positions,
            buffer_indices,
            device,
            global_parameters,
        })
    }

    pub(crate) fn write_render_commands(&self, command_buffer: &CommandBuffer, current_frame: u64) {
        command_buffer.bind_vertex_buffers(0, &[&self.buffer_positions], &[0]);
        command_buffer.bind_index_buffer(&self.buffer_indices, 0);

        // Encode `first_instance` to contain parameters to calculate index to global instance SSBO in shader.
        let storage_buffer_offset = (self.draw_storage_buffer_offset & 0xFFFF) as u32;
        let vertices_per_instance = (self.vertex_count_per_instance & 0xFFFF) as u32;
        let first_instance_encoded = (storage_buffer_offset << 16) | vertices_per_instance;

        command_buffer.draw_indexed(
            self.draw_range.index_count,
            1,
            self.draw_range.index_offset,
            0,
            first_instance_encoded,
        );
    }

    /// Returns platform instances GPU data to be set in the global SSBO.
    fn set_platforms_objects(
        &mut self,
        platforms: Vec<PlatformObject>,
    ) -> Result<Vec<PlatformInstanceGpuData>> {
        println!("Calling set platforms with len {}", platforms.len());

        let (vertex_positions, indices, platform_instances_data, _) = platforms.iter().fold(
            (Vec::new(), Vec::new(), Vec::new(), 0),
            |(
                mut acc_vertex_positions,
                mut acc_indices,
                mut platform_instances_data,
                mut current_index_offset,
            ),
             platform| {
                match platform {
                    PlatformObject::DynamicPlane(dynamic_plane) => {
                        println!(
                            "acc vertex poss len {}",
                            dynamic_plane.plane_mesh.vertices.len()
                        );

                        assert!(
                            dynamic_plane.plane_mesh.vertices.len()
                                == self.vertex_count_per_instance as _
                        );

                        acc_vertex_positions.extend_from_slice(&dynamic_plane.plane_mesh.vertices);
                        acc_indices.extend(
                            dynamic_plane
                                .plane_mesh
                                .indices
                                .iter()
                                .map(|i| i + current_index_offset)
                                .collect::<Vec<_>>(),
                        );
                        current_index_offset += dynamic_plane.plane_mesh.vertices.len() as i16;

                        platform_instances_data.push(PlatformInstanceGpuData::new(
                            Matrix4::new_translation(&Vector3::new(
                                0.0,
                                0.0,
                                dynamic_plane.runner_position_start,
                            )),
                        ));
                    }
                }
                (
                    acc_vertex_positions,
                    acc_indices,
                    platform_instances_data,
                    current_index_offset,
                )
            },
        );

        assert!(vertex_positions.len() <= self.max_vertices as _);
        assert!(indices.len() <= self.max_indices as _);
        assert!(platform_instances_data.len() <= self.max_platform_instances as _);

        log::info!(
            "Wrtiing vertices len {} to plat mesh type renderer, platforms len {}",
            vertex_positions.len(),
            platforms.len(),
        );
        self.buffer_positions.write_data(&vertex_positions)?;
        self.buffer_indices.write_data(&indices)?;

        self.platforms = platforms;
        Ok(platform_instances_data)
    }

    fn update_draw_range(&mut self, current_runner_position: f32) {
        // Add new platforms to the draw range.
        while self.draw_range.last_platform_index < self.platforms.len() as _ {
            let platform = &*&self.platforms[self.draw_range.last_platform_index as usize];
            match platform {
                PlatformObject::DynamicPlane(dynamic_plane) => {
                    if dynamic_plane.runner_position_start
                        < current_runner_position + self.global_parameters.z_range[1]
                    {
                        self.draw_range.last_platform_index += 1;
                        self.draw_range.index_count +=
                            dynamic_plane.plane_mesh.indices.len() as u32;
                    } else {
                        break;
                    }
                }
            }
        }

        // Remove passed platforms from draw range.
        while self.draw_range.first_platform_index < self.platforms.len() as _ {
            let platform = &*&self.platforms[self.draw_range.first_platform_index as usize];
            match platform {
                PlatformObject::DynamicPlane(dynamic_plane) => {
                    let additional_offset = 4.0; // Additional z axis offset to make sure platform is fully passed.
                    if dynamic_plane.runner_position_end + additional_offset
                        < current_runner_position
                    {
                        self.draw_range.first_platform_index += 1;
                        self.draw_range.index_offset +=
                            dynamic_plane.plane_mesh.indices.len() as u32;
                        self.draw_range.index_count -=
                            dynamic_plane.plane_mesh.indices.len() as u32;
                    } else {
                        break;
                    }
                }
            }
        }
    }
}

pub(crate) struct PlatformRenderer {
    global_parameters: GlobalPlatformParameters,

    quad_platform_ssbo_offset: u64,
    quad_renderer: SingleMeshTypePlatformRenderer,

    curve_sided_platform_ssbo_offset: u64,
    curve_sided_plane_renderer: SingleMeshTypePlatformRenderer,

    /// Global SSBO to contain per-object data for all platforms.
    buffer_storage_global: Buffer,

    descriptor_sets: [DescriptorSet; MAX_FRAMES],
    graphics_pipeline: Pipeline,
    device: Arc<Device>,
}

impl PlatformRenderer {
    pub(crate) fn new(device: Arc<Device>) -> Result<Self> {
        let global_parameters = GlobalPlatformParameters {
            z_range: Vector2::new(-1.0, 20.0),
            base_color: Vector4::new(0.3, 0.2, 0.8, 1.0),
        };

        let max_platform_instances = MAX_PLATFORM_INSTANCES;
        let buffer_storage_global = device.create_buffer(BufferDescriptor {
            size: max_platform_instances * size_of::<PlatformInstanceGpuData>() as u64,
            usage_flags: vk::BufferUsageFlags::STORAGE_BUFFER,
            // XXX TODO: Use GPU only mememory and do proper async transfers.
            memory_location: MemoryLocation::CpuToGpu,
        })?;

        // Currently we have two mesh types.
        let num_mesh_types = 2;
        let max_platform_instances_per_mesh_type = max_platform_instances / num_mesh_types;
        let max_vertices_per_mesh_type = MAX_TOTAL_VERTICES_PER_PLATFORM_BUFFER;
        let max_indices_per_mesh_type = MAX_TOTAL_INDICES_PER_PLATFORM_BUFFER;

        let quad_platform_object_vertex_count = QUAD_PLATFORM_VERTEX_COUNT;
        let quad_platform_ssbo_offset = 0;
        let quad_renderer = SingleMeshTypePlatformRenderer::new(
            device.clone(),
            global_parameters.clone(),
            max_vertices_per_mesh_type,
            max_indices_per_mesh_type,
            max_platform_instances_per_mesh_type,
            quad_platform_object_vertex_count,
            quad_platform_ssbo_offset,
        )?;

        // XXX TODO: Have thie configurable by the user.
        let curve_sided_platform_object_vertex_count = CURVE_SIDED_PLATFORM_VERTEX_COUNT;
        let curve_sided_platform_ssbo_offset = 1 * max_platform_instances_per_mesh_type;
        let curve_sided_plane_renderer = SingleMeshTypePlatformRenderer::new(
            device.clone(),
            global_parameters.clone(),
            max_vertices_per_mesh_type,
            max_indices_per_mesh_type,
            max_platform_instances_per_mesh_type,
            curve_sided_platform_object_vertex_count,
            curve_sided_platform_ssbo_offset,
        )?;

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

        // let platforms = Vec::new();

        // let complex_plane = Plane::one_sided_cubic_bezier(
        //     Vector2::new(-0.0, 2.0),
        //     Vector2::new(-0.0, 8.0),
        //     (Vector2::new(-1.0, 4.0), Vector2::new(-1.0, 6.0)),
        //     Vector2::new(1.0, 2.0),
        //     Vector2::new(1.0, 8.0),
        //     20,
        // );
        // log::debug!(
        //     "Bezier plane num vertices {}, num indices {}",
        //     complex_plane.vertices.len(),
        //     complex_plane.indices.len()
        // );
        // println!("Indicies {:?}", &complex_plane.indices);

        // let plane2 = Plane::two_sided_parallel_cubic_bezier(
        //     Vector2::new(0.5, 10.0),
        //     Vector2::new(0.5, 17.0),
        //     (Vector2::new(-1.0, 12.0), Vector2::new(-1.0, 15.0)),
        //     0.5,
        //     40,
        // );
        // log::debug!(
        //     "Bezier plane num vertices {}, num indices {}",
        //     plane2.vertices.len(),
        //     plane2.indices.len()
        // );

        // buffer_positions_platforms.write_data(&complex_plane.vertices)?;
        // buffer_indices_platforms.write_data(&complex_plane.indices)?;
        // // buffer_positions_platforms.write_data(&plane2.vertices)?;
        // // buffer_indices_platforms.write_data(&plane2.indices)?;

        // let plane3 = Plane::two_sided_cubic_bezier(
        //     Vector2::new(-0.5, 5.0),
        //     Vector2::new(-0.5, 10.0),
        //     (Vector2::new(-1.0, 6.5), Vector2::new(-1.0, 8.5)),
        //     Vector2::new(0.5, 5.0),
        //     Vector2::new(0.5, 10.0),
        //     (Vector2::new(1.0, 6.5), Vector2::new(1.0, 8.5)),
        //     40,
        // );
        // log::debug!(
        //     "Bezier plane num vertices {}, num indices {}",
        //     plane3.vertices.len(),
        //     plane3.indices.len()
        // );
        // // buffer_positions_platforms.write_data(&plane3.vertices)?;
        // // buffer_indices_platforms.write_data(&plane3.indices)?;

        Ok(Self {
            global_parameters,
            buffer_storage_global,
            quad_platform_ssbo_offset,
            quad_renderer,
            curve_sided_platform_ssbo_offset,
            curve_sided_plane_renderer,
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

        self.quad_renderer
            .write_render_commands(command_buffer, current_frame);
        self.curve_sided_plane_renderer
            .write_render_commands(command_buffer, current_frame);
    }

    pub(crate) fn update_with_runner_position(&mut self, runner_position: f32) {
        self.quad_renderer.update_draw_range(runner_position);
        self.curve_sided_plane_renderer
            .update_draw_range(runner_position);
    }

    pub(crate) fn write_initital_gpu_resources(&self, scene_uniform_buffer: &Buffer) -> Result<()> {
        let descriptor_binding_writes = DescriptorBindingWrites {
            buffers: vec![
                DescriptorBindingBufferWrite {
                    buffer: scene_uniform_buffer,
                    binding_index: 0,
                },
                DescriptorBindingBufferWrite {
                    buffer: &self.buffer_storage_global,
                    binding_index: 1,
                },
            ],
        };
        for descriptor_set in &self.descriptor_sets {
            self.device
                .update_descriptor_set(descriptor_set, descriptor_binding_writes.clone())?;
        }

        Ok(())
    }

    pub(crate) fn set_platforms_objects(&mut self, platforms: Vec<PlatformObject>) -> Result<()> {
        // Make sure the meshes are valid quads or curbed/bezier planes with the correct amount of indices.
        for p in &platforms {
            match p {
                PlatformObject::DynamicPlane(plane) => {
                    if plane.plane_mesh.vertices.len() != QUAD_PLATFORM_VERTEX_COUNT as _
                        && plane.plane_mesh.vertices.len() != CURVE_SIDED_PLATFORM_VERTEX_COUNT as _
                    {
                        return Err(anyhow!("Incorrect platform mesh type."));
                    }
                }
            }
        }

        let quad_platforms = platforms
            .iter()
            .filter(|p| match p {
                PlatformObject::DynamicPlane(plane) => {
                    plane.plane_mesh.vertices.len() == QUAD_PLATFORM_VERTEX_COUNT as _
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        let quad_platforms_instances_data =
            self.quad_renderer.set_platforms_objects(quad_platforms)?;
        self.buffer_storage_global.write_data_with_value_offset(
            &quad_platforms_instances_data,
            self.quad_platform_ssbo_offset,
        )?;

        let curve_sided_platforms = platforms
            .iter()
            .filter(|p| match p {
                PlatformObject::DynamicPlane(plane) => {
                    plane.plane_mesh.vertices.len() == CURVE_SIDED_PLATFORM_VERTEX_COUNT as _
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        let curve_sided_platforms_instances_data = self
            .curve_sided_plane_renderer
            .set_platforms_objects(curve_sided_platforms)?;
        self.buffer_storage_global.write_data_with_value_offset(
            &curve_sided_platforms_instances_data,
            self.curve_sided_platform_ssbo_offset,
        )?;

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
            source_file_name: "shaders/platform.vs.glsl",
            shader_stage: ShaderStage::Vertex,
        })?;
        let fragment_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            source_file_name: "shaders/platform.fs.glsl",
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
            .blend_enable(true)
            .color_blend_op(vk::BlendOp::ADD)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
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
