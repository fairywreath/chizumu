use std::{
    collections::HashMap,
    ffi::CString,
    mem::{align_of, size_of_val},
    sync::Arc,
};

use anyhow::Result;
use ash::vk;
use gpu_allocator::{
    vulkan::{Allocation, AllocationCreateDesc, AllocationScheme},
    MemoryLocation,
};

use super::{device::Device, shader::ShaderModule, DeviceShared};

pub struct BufferDescriptor {
    pub size: u64,
    pub usage_flags: vk::BufferUsageFlags,
    pub memory_location: MemoryLocation,
}

pub struct Buffer {
    pub(crate) raw: vk::Buffer,
    size: u64,
    allocation: Option<Allocation>,
    device: Arc<Device>,
}

/// Buffer that is pending for actual vulkan destruction.
/// This structure should not hold the actual `Device` resource to prevent circular referencing.
pub(crate) struct PendingDestructionBuffer {
    raw: vk::Buffer,
    allocation: Allocation,
    // Add other info such as frame submission index as required....
}

impl Buffer {
    /// Writes to a GPU<->GPU buffer. Returns error if buffer is not writable from the CPU.
    pub fn write_data<T: Copy>(&self, data: &[T]) -> Result<()> {
        unsafe {
            let data_ptr = self
                .allocation
                .as_ref()
                .unwrap()
                .mapped_ptr()
                .unwrap()
                .as_ptr();

            let mut align =
                ash::util::Align::new(data_ptr, align_of::<T>() as _, size_of_val(data) as _);
            align.copy_from_slice(data);
        };

        Ok(())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let allocation = self.allocation.take().unwrap();
        self.device.schedule_buffer_destruction(self, allocation);
    }
}

pub struct PipelineDescriptor {
    /// vkPipelineLayoutCreateInfo information. Descriptor binding layout is required.
    pub descriptor_set_layouts: Vec<Arc<DescriptorSetLayout>>,

    /// vkPipelineCreateInfo information.
    pub shader_modules: Vec<ShaderModule>,
    pub vertex_input_attributes: Vec<vk::VertexInputAttributeDescription>,
    pub vertex_input_bindings: Vec<vk::VertexInputBindingDescription>,
    pub primitive_topology: vk::PrimitiveTopology,
    pub viewport_scissor_extent: vk::Extent2D,
    pub color_blend_attachments: Vec<vk::PipelineColorBlendAttachmentState>, // Should be equal to the number of color attachments.
    pub depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo,
    pub rasterization_state: vk::PipelineRasterizationStateCreateInfo,

    /// Required for dynamic rendering.
    pub color_attachment_formats: Vec<vk::Format>,
    pub depth_attachment_format: vk::Format,
}

pub struct Pipeline {
    pub(crate) raw: vk::Pipeline,
    pub(crate) raw_layout: vk::PipelineLayout,

    /// XXX: Do we need to hold onto the descriptor set layouts after the pipelin layout is created?
    _descriptor_set_layouts: Vec<Arc<DescriptorSetLayout>>,
    device: Arc<DeviceShared>,
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            // XXX: Need to wait for command buffer execution completion.
            self.device.raw.destroy_pipeline(self.raw, None);
            self.device
                .raw
                .destroy_pipeline_layout(self.raw_layout, None);
        }
    }
}

/// Small wrapper around `vkDescriptorPool`.
pub(crate) struct DescriptorPool {
    raw: vk::DescriptorPool,
    device: Arc<DeviceShared>,
}

impl DescriptorPool {
    pub(crate) fn new(
        device: Arc<DeviceShared>,
        desc: vk::DescriptorPoolCreateInfo,
    ) -> Result<Self> {
        let raw = unsafe { device.raw.create_descriptor_pool(&desc, None)? };

        Ok(Self { raw, device })
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_descriptor_pool(self.raw, None);
        }
    }
}

pub struct DescriptorSetLayoutDescriptor {
    pub bindings: Vec<vk::DescriptorSetLayoutBinding>,
    pub flags: vk::DescriptorSetLayoutCreateFlags,
}

pub struct DescriptorSetLayout {
    raw: vk::DescriptorSetLayout,
    bindings: Vec<vk::DescriptorSetLayoutBinding>,
    bindings_map: HashMap<u32, vk::DescriptorSetLayoutBinding>,
    device: Arc<DeviceShared>,
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .raw
                .destroy_descriptor_set_layout(self.raw, None);
        }
    }
}

#[derive(Clone)]
pub struct DescriptorSetDescriptor {
    pub layout: Arc<DescriptorSetLayout>,
}

pub struct DescriptorSet {
    pub(crate) raw: vk::DescriptorSet,

    /// Do not need to hold the pool object itself as the global pool is tied to `Device`,
    /// and when `Device` is dropped this descriptor set object cannot be used anymore anyways.
    ///
    /// XXX: Need to hold onto the resource bindings as well(eg. buffers and images)?
    layout: Arc<DescriptorSetLayout>,
    device: Arc<DeviceShared>,
}

/// XXX: The descriptor set is tehcnically responsible for keeping its bounded reosurces valid.
/// Maybe hold a strong reference to the bounded resources as well?
#[derive(Clone)]
pub struct DescriptorBindingBufferWrite<'a> {
    pub buffer: &'a Buffer,
    pub binding_index: u32,
}

#[derive(Clone)]
pub struct DescriptorBindingWrites<'a> {
    pub buffers: Vec<DescriptorBindingBufferWrite<'a>>,
}

impl Device {
    pub fn create_buffer(self: &Arc<Self>, desc: BufferDescriptor) -> Result<Buffer> {
        let create_info = vk::BufferCreateInfo::builder().size(desc.size).usage(
            desc.usage_flags
                | vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::TRANSFER_DST,
        );

        let raw;
        let requirements;
        unsafe {
            raw = self.shared.raw.create_buffer(&create_info, None)?;
            requirements = self.shared.raw.get_buffer_memory_requirements(raw);
        }

        let allocation = self
            .shared
            .allocator
            .lock()
            .allocate(&AllocationCreateDesc {
                name: "buffer",
                requirements,
                location: desc.memory_location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })?;

        unsafe {
            self.shared
                .raw
                .bind_buffer_memory(raw, allocation.memory(), allocation.offset())?;
        }

        Ok(Buffer {
            device: self.clone(),
            raw,
            size: desc.size,
            allocation: Some(allocation),
        })
    }

    /// Schedules/queues a buffer for destruction. `buffer` should no longer be used after this is called
    /// but it is passed in as a reference so this can be called inside `drop`.
    fn schedule_buffer_destruction(&self, buffer: &Buffer, allocation: Allocation) {
        self.resource_hub
            .lock()
            .pending_destruction_buffers
            .push(PendingDestructionBuffer {
                raw: buffer.raw,
                allocation,
            })
    }

    /// Destroys and deallocate buffer GPU resources.
    pub(crate) fn destroy_buffer(&self, buffer: PendingDestructionBuffer) -> Result<()> {
        unsafe {
            self.shared.raw.destroy_buffer(buffer.raw, None);
            self.shared.allocator.lock().free(buffer.allocation)?;
        }

        Ok(())
    }

    pub fn create_pipeline(&self, desc: PipelineDescriptor) -> Result<Pipeline> {
        let descriptor_set_layouts = desc
            .descriptor_set_layouts
            .iter()
            .map(|layout| layout.raw)
            .collect::<Vec<_>>();
        let pipeline_layout_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layouts);
        let pipeline_layout = unsafe {
            self.shared
                .raw
                .create_pipeline_layout(&pipeline_layout_info, None)?
        };

        let shader_entry_point_name = CString::new("main").unwrap();
        let shader_stages = desc
            .shader_modules
            .iter()
            .map(|shader_module| {
                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(shader_module.stage.to_vulkan_shader_stage_flag())
                    .module(shader_module.raw)
                    .name(&shader_entry_point_name)
                    .build()
            })
            .collect::<Vec<_>>();

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&desc.vertex_input_attributes)
            .vertex_binding_descriptions(&desc.vertex_input_bindings);

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(desc.primitive_topology)
            .primitive_restart_enable(false);

        let viewports = [vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(desc.viewport_scissor_extent.width as f32)
            .height(desc.viewport_scissor_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build()];
        let scissors = [vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(desc.viewport_scissor_extent)
            .build()];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        // Individual color blend attachments needs color write mask to be RGBA(?).
        // Need one color blend attachment state for each color attachement(render target).
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&desc.color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false)
            .min_sample_shading(1.0);

        let mut pipeline_rendering_info = vk::PipelineRenderingCreateInfo::builder()
            .view_mask(0)
            .color_attachment_formats(&desc.color_attachment_formats)
            .depth_attachment_format(desc.depth_attachment_format)
            .stencil_attachment_format(vk::Format::UNDEFINED);

        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .color_blend_state(&color_blend_state)
            .depth_stencil_state(&desc.depth_stencil_state)
            .multisample_state(&multisample_state)
            .rasterization_state(&desc.rasterization_state)
            .layout(pipeline_layout)
            .push_next(&mut pipeline_rendering_info)
            .build();

        let raw = unsafe {
            self.shared
                .raw
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_create_info),
                    None,
                )
                .map_err(|e| e.1)?[0]
        };

        Ok(Pipeline {
            raw,
            raw_layout: pipeline_layout,
            _descriptor_set_layouts: desc.descriptor_set_layouts,
            device: self.shared.clone(),
        })
    }

    pub fn create_descriptor_set_layout(
        &self,
        desc: DescriptorSetLayoutDescriptor,
    ) -> Result<DescriptorSetLayout> {
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&desc.bindings)
            .flags(desc.flags);
        let raw = unsafe {
            self.shared
                .raw
                .create_descriptor_set_layout(&create_info, None)?
        };

        let bindings_map = desc
            .bindings
            .iter()
            .cloned()
            .map(|binding| (binding.binding, binding))
            .collect();

        Ok(DescriptorSetLayout {
            raw,
            bindings: desc.bindings,
            bindings_map,
            device: self.shared.clone(),
        })
    }

    pub fn create_descriptor_set(&self, desc: DescriptorSetDescriptor) -> Result<DescriptorSet> {
        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.global_descriptor_pool.raw)
            .set_layouts(std::slice::from_ref(&desc.layout.raw));
        let raws = unsafe { self.shared.raw.allocate_descriptor_sets(&allocate_info)? };

        Ok(DescriptorSet {
            raw: raws[0],
            layout: desc.layout,
            device: self.shared.clone(),
        })
    }

    /// Binds descriptor set with resource writes.
    pub fn update_descriptor_set(
        &self,
        descriptor_set: &DescriptorSet,
        writes: DescriptorBindingWrites,
    ) -> Result<()> {
        let mut vulkan_write_descriptors = Vec::new();

        // Image/buffer descriptor write infos need to be valid when calling vkUpdateDescriptorSets.
        let mut descriptor_buffer_infos = Vec::<vk::DescriptorBufferInfo>::new();

        for buffer_write in &writes.buffers {
            if let Some(binding) = descriptor_set
                .layout
                .bindings_map
                .get(&buffer_write.binding_index)
            {
                assert_eq!(
                    binding.binding, buffer_write.binding_index,
                    "Descriptor set layout binding index and buffer write binding do not match."
                );

                let mut vulkan_write_descriptor = vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set.raw)
                    .dst_binding(binding.binding)
                    .dst_array_element(0)
                    .descriptor_type(binding.descriptor_type);

                match binding.descriptor_type {
                    vk::DescriptorType::UNIFORM_BUFFER | vk::DescriptorType::STORAGE_BUFFER => {
                        let vulkan_buffer_info = vk::DescriptorBufferInfo::builder()
                            .offset(0)
                            .range(buffer_write.buffer.size as u64)
                            .buffer(buffer_write.buffer.raw)
                            .build();
                        descriptor_buffer_infos.push(vulkan_buffer_info);

                        // 1 buffer info for the whole descriptor write element.
                        vulkan_write_descriptor = vulkan_write_descriptor.buffer_info(
                            std::slice::from_ref(descriptor_buffer_infos.last().unwrap()),
                        );

                        vulkan_write_descriptors.push(vulkan_write_descriptor.build());
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Cannot handle descriptor type {:#?}",
                            binding.descriptor_type
                        ));
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Binding index {} on descriptor buffer write is invalid!",
                    buffer_write.binding_index
                ));
            }
        }

        unsafe {
            self.shared
                .raw
                .update_descriptor_sets(&vulkan_write_descriptors, &[]);
        }

        Ok(())
    }
}
