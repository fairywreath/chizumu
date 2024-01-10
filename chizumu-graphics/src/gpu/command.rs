use std::sync::Arc;

use anyhow::Result;
use ash::vk;

use super::device::Device;

pub struct CommandBuffer {
    pub(crate) raw: vk::CommandBuffer,
}

impl Device {}
