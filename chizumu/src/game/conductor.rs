use std::time::Instant;

use anyhow::Result;

use crate::core::audio::AudioSystem;

use kira::sound::static_sound::StaticSoundHandle;

pub(crate) struct Conductor {
    current_music_handle: Option<StaticSoundHandle>,
    // XXX: Include manual timer for fine grain music position tracking (Kira has a minimum delta song position).
    // manual_timer: Instant,
}

impl Conductor {
    pub(crate) fn new() -> Self {
        Self {
            current_music_handle: None,
        }
    }

    pub(crate) fn start_music(
        &mut self,
        audio_system: &mut AudioSystem,
        music_index: usize,
    ) -> Result<()> {
        self.current_music_handle = Some(audio_system.play_music(music_index)?);
        Ok(())
    }

    pub(crate) fn get_current_music_position(&self) -> Option<f32> {
        self.current_music_handle
            .as_ref()
            .map(|sound_handle| sound_handle.position() as f32)
    }
}
