use std::time::Instant;

use anyhow::Result;

use crate::core::audio::AudioSystem;

use kira::sound::static_sound::StaticSoundHandle;

pub struct Conductor {
    current_music_handle: Option<StaticSoundHandle>,
    manual_timer: Option<Instant>,
}

impl Conductor {
    pub fn new() -> Self {
        Self {
            current_music_handle: None,
            manual_timer: None,
        }
    }

    pub fn start_music(
        &mut self,
        audio_system: &mut AudioSystem,
        music_index: usize,
    ) -> Result<()> {
        self.current_music_handle = Some(audio_system.play_music(music_index)?);
        // self.manual_timer = Some(Instant::now());

        Ok(())
    }

    pub fn get_current_music_position(&self) -> Option<f32> {
        self.current_music_handle
            .as_ref()
            .map(|sound_handle| sound_handle.position() as f32)

        // self.manual_timer
        //     .as_ref()
        //     .map(|timer| timer.elapsed().as_secs_f32())
    }
}
