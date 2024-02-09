/*!
 * Game logic.
 */

use crate::{
    chart::{runtime::*, *},
    core::audio::{AudioSystem, SFX_TAP_A_INDEX},
};

use parking_lot::Mutex;

pub mod conductor;

pub struct GameState {
    /// For testing purposes.
    audio_system: Mutex<AudioSystem>,

    /// Current song information.
    ///
    /// Current song position in seconds.
    // current_song_position: f32,
    chart: Option<RuntimeChart>,
    current_note_index: usize,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            chart: None,
            audio_system: Mutex::new(AudioSystem::new().unwrap()),
            current_note_index: 0,
        }
    }

    pub fn update_current_music_position(&mut self, secs: f32) {
        // if let Some(chart) = &self.chart {
        //     while self.current_note_index < chart.notes.len() {
        //         if chart.notes[self.current_note_index].offset < secs {
        //             // log::debug!(
        //             //     "Note offset {} less than song position {}",
        //             //     self.chart.notes[self.current_note_index].offset,
        //             //     secs
        //             // );
        //             self.audio_system
        //                 .lock()
        //                 .play_sound_effect(SFX_TAP_A_INDEX)
        //                 .unwrap();

        //             self.current_note_index += 1;
        //         } else {
        //             break;
        //         }
        //     }
        // }
    }

    pub fn set_chart(&mut self, chart: RuntimeChart) {
        self.chart = Some(chart);
    }

    pub fn get_chart(&self) -> &RuntimeChart {
        // XXX: Remove this unwrap
        self.chart.as_ref().unwrap()
    }
}
