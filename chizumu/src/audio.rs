use anyhow::Result;
use kira::{
    manager::{backend::cpal::CpalBackend, AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
};

pub const SFX_TAP_A_INDEX: usize = 0;
pub const SFX_TAP_B_INDEX: usize = 1;

pub struct AudioSystem {
    audio_manager: AudioManager,

    sound_data_effects: Vec<StaticSoundData>,
    sound_data_music: Vec<StaticSoundData>,
}

impl AudioSystem {
    pub fn new() -> Result<Self> {
        let audio_manager = AudioManager::<CpalBackend>::new(AudioManagerSettings::default())?;

        let mut sound_data_effects = Vec::new();
        sound_data_effects.push(StaticSoundData::from_file(
            "data/Sound Effects/Arcaea/arc.wav",
            StaticSoundSettings::new(),
        )?);
        sound_data_effects.push(StaticSoundData::from_file(
            "data/Sound Effects/Idolmaster Stella Stage/se_rhythm#1 (RHY_TAP).wav",
            StaticSoundSettings::new(),
        )?);

        Ok(Self {
            audio_manager,
            sound_data_effects,
            sound_data_music: Vec::new(),
        })
    }

    /// Returns index to loaded music
    pub fn load_music_data(&mut self, music_file_path: &str) -> Result<usize> {
        let data = StaticSoundData::from_file(music_file_path, StaticSoundSettings::new())?;
        self.sound_data_music.push(data);
        Ok(self.sound_data_music.len() - 1)
    }

    pub fn play_sound_effect(&mut self, sound_effect_index: usize) -> Result<()> {
        self.audio_manager
            .play(self.sound_data_effects[sound_effect_index].clone())?;
        Ok(())
    }

    pub fn play_music(&mut self, music_index: usize) -> Result<StaticSoundHandle> {
        let sound_handle = self
            .audio_manager
            .play(self.sound_data_music[music_index].clone())?;

        Ok(sound_handle)
    }
}
