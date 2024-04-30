use anyhow::Result;
use kira::{
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
    tween::Tween,
};

pub(crate) struct AudioSystem {
    audio_manager: AudioManager,

    sound_data_effects: Vec<StaticSoundData>,
    sound_data_music: Vec<StaticSoundData>,
}

impl AudioSystem {
    pub(crate) fn new() -> Result<Self> {
        let audio_manager = AudioManager::<CpalBackend>::new(AudioManagerSettings::default())?;

        let mut sound_data_effects = Vec::new();
        sound_data_effects.push(StaticSoundData::from_file(
            "assets/sound_effects/Arcaea/arc.wav",
            StaticSoundSettings::new().volume(0.3),
        )?);

        Ok(Self {
            audio_manager,
            sound_data_effects,
            sound_data_music: Vec::new(),
        })
    }

    pub(crate) fn play_sound_effect(&mut self, sound_effect_index: usize) -> Result<()> {
        self.audio_manager
            .play(self.sound_data_effects[sound_effect_index].clone())?;
        Ok(())
    }

    /// Returns index to loaded music
    pub(crate) fn load_music_data(&mut self, music_file_path: &str) -> Result<usize> {
        let data =
            StaticSoundData::from_file(music_file_path, StaticSoundSettings::new().volume(0.1))?;
        self.sound_data_music.push(data);
        Ok(self.sound_data_music.len() - 1)
    }

    pub fn play_music(&mut self, music_index: usize) -> Result<StaticSoundHandle> {
        let sound_handle = self
            .audio_manager
            .play(self.sound_data_music[music_index].clone())?;

        Ok(sound_handle)
    }
}

impl Drop for AudioSystem {
    fn drop(&mut self) {
        self.audio_manager
            .pause(Tween {
                ..Default::default()
            })
            .unwrap();
    }
}
