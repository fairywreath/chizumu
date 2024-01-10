use std::sync::Arc;

use anyhow::Result;
use ash::vk;
use parking_lot::{Mutex, RwLock};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use super::{
    DeviceShared, Instance, Queue, Semaphore, SemaphoreType, Surface, Swapchain,
    QUEUE_FAMILY_INDEX_GRAPHICS,
};

pub const MAX_FRAMES: usize = 2;

struct FrameCounters {
    current: u64,
    previous: u64,
    absolute: u64,
}

/// Structure that describes the functionality of a logical device and contains all the necessary resources
/// for rendering, including window/surface resources.
///
/// Additionally handles frame  synchronization logic.
pub struct Device {
    /// Frame synchronization device resources.
    ///
    /// Wait on this semaphore when presenting.
    semaphores_render_complete: [Semaphore; 2],
    /// Signal semaphore when acquiring swapchain image, wait when submitting graphics command buffer work.
    semaphore_swapchain_image_acquired: Semaphore,
    /// Timeline semaphore for general purpose rendering work. Only one semaphore required for (potentially) multiple frames in flight.
    semaphore_graphics_frame: Semaphore,

    frame_counters: RwLock<FrameCounters>,

    /// Same HW queue for both graphics and present work.
    queue_graphics_present: Queue,

    swapchain: Swapchain,
    pub(crate) shared: Arc<DeviceShared>,
}

impl Device {
    pub fn new(
        window_handle: &dyn HasRawWindowHandle,
        display_handle: &dyn HasRawDisplayHandle,
    ) -> Result<Self> {
        let instance = Instance::new(display_handle)?;
        let surface = Surface::new(&instance, window_handle, display_handle)?;
        let shared = Arc::new(DeviceShared::new(instance, surface)?);
        let swapchain = Swapchain::new(shared.clone(), vk::PresentModeKHR::FIFO, 1920, 1200)?;

        // Always get index at queue 0 since only 1 queue is used per family.
        let queue_graphics_present_family_index =
            shared.queue_families[QUEUE_FAMILY_INDEX_GRAPHICS].index;
        let queue_graphics_present = unsafe {
            shared
                .raw
                .get_device_queue(queue_graphics_present_family_index, 0)
        };
        let queue_graphics_present = Queue::new_from_vulkan_handle(
            shared.raw.clone(),
            queue_graphics_present,
            queue_graphics_present_family_index,
        );
        log::info!(
            "Graphics/Present Queue family index: {}",
            queue_graphics_present_family_index
        );

        let semaphores_render_complete = [
            Semaphore::new(shared.clone(), SemaphoreType::Binary)?,
            Semaphore::new(shared.clone(), SemaphoreType::Binary)?,
        ];
        let semaphore_swapchain_image_acquired =
            Semaphore::new(shared.clone(), SemaphoreType::Binary)?;
        let semaphore_graphics_frame = Semaphore::new(shared.clone(), SemaphoreType::Timeline)?;

        Ok(Self {
            shared,
            swapchain,
            queue_graphics_present,
            semaphore_graphics_frame,
            semaphore_swapchain_image_acquired,
            semaphores_render_complete,
            frame_counters: RwLock::new(FrameCounters {
                current: 0,
                previous: 0,
                absolute: 0,
            }),
        })
    }

    fn frame_counters_advance(&self) {
        let mut counters = self.frame_counters.write();
        counters.previous = counters.current;
        counters.current = (counters.current + 1) % (MAX_FRAMES as u64);
        counters.absolute += 1;
    }

    /// Returns the timeline semaphore value needed to be waited on before beggining a frame.
    /// A "frame" shares GPU resources.
    fn frame_semaphore_graphics_wait_value(&self) -> u64 {
        self.frame_counters.read().absolute - (MAX_FRAMES as u64 - 1)
    }

    pub fn frame_begin(&mut self) -> Result<()> {
        if self.frame_counters.read().absolute >= MAX_FRAMES as u64 {
            let graphics_wait_value = self.frame_semaphore_graphics_wait_value();

            let wait_values = [graphics_wait_value];
            let semaphores = [self.semaphore_graphics_frame.raw];

            let wait_info = vk::SemaphoreWaitInfo::builder()
                .semaphores(&semaphores)
                .values(&wait_values);

            unsafe { self.shared.raw.wait_semaphores(&wait_info, u64::MAX)? };
        }

        self.swapchain
            .acquire_next_image(self.semaphore_swapchain_image_acquired.raw)?;

        Ok(())
    }

    pub fn swapchain_present(&self) -> Result<()> {
        self.swapchain.queue_present(
            self.queue_graphics_present.raw,
            &[self.semaphores_render_complete[self.swapchain.image_index as usize].raw],
        )?;
        self.frame_counters_advance();

        Ok(())
    }
}
