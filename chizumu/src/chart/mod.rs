/*!
 * Chart writing and parsing.
 * Implementation based on Chunithm's format.
 */
use anyhow::{anyhow, Result};

use chizumu_graphics::{hit::HitObject, HIT_AREA_Z_START};

pub mod parse;

#[derive(Debug)]
pub struct ChartInfo {
    /// Chart mapping information.
    resolution: u32,
    starting_bpm: u32,
    starting_measure: TimeSignature,
    bpm_changes: Vec<BpmChange>,
    measure_changes: Vec<MeasureChange>,
    notes: Vec<Note>,

    /// Cosmetic/visual information.
    playfield_changes: Vec<PlayfieldChange>,

    pub music_file_path: String,
    /// Starting offset in seconds before the first measure.
    music_starting_offset: f32,
}

#[derive(Debug)]
enum NoteType {
    TAP,
}

impl TryFrom<&str> for NoteType {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "TAP" => Ok(NoteType::TAP),
            _ => Err(anyhow!("Invalid string for NoteType conversion: {}", s)),
        }
    }
}

#[derive(Debug)]
struct Note {
    note_type: NoteType,

    /// The global measure index in which the current note starts at.
    measure: u32,

    /// Offset from the current measure scaled by the resolution.
    offset: u32,

    /// Cell gives the position of the leftmost cell of the note, while width gives the number of
    /// cells the note covers.
    cell: u32,
    width: u32,
}

#[derive(Debug)]
struct TimeSignature {
    /// Top value/numerator.
    num_beats: u32,
    /// Bottom value/denomintaor.
    note_value: u32,
}

#[derive(Debug)]
struct MeasureChange {
    /// The global measure and offset in which the change takes place.
    /// The specific time of this change depends on the last measure/time siganuture + bpm values.
    measure: u32,
    offset: u32,

    time_signature: TimeSignature,
}

#[derive(Debug)]
struct BpmChange {
    /// The global measure and offset in which the change takes place.
    /// The specific time of this change depends on the last measure/time siganuture + bpm values.
    measure: u32,
    offset: u32,

    bpm: u32,
}

/// Purely cosmetic playfield change.
#[derive(Debug)]
struct PlayfieldChange {
    /// The global measure and offset in which the change takes place.
    /// The specific time of this change depends on the last measure/time siganuture + bpm values.
    measure: u32,
    offset: u32,

    /// In seconds.
    duration: f32,
    mutiplier: f32,
}

pub struct TimedNote {
    /// Offset in seconds from the start of the piece.
    pub offset: f32,
    pub cell: u32,
    pub width: u32,
}

impl TimedNote {
    pub fn new(offset: f32, cell: u32, width: u32) -> Self {
        Self {
            offset,
            cell,
            width,
        }
    }
}

/// Structure used by the main game logic during run time.
pub struct Chart {
    pub notes: Vec<TimedNote>,
}

impl Chart {
    pub fn create_hit_objects(&self) -> Vec<HitObject> {
        let play_field_speed = 8.0; // z-axis movement per second.
        let num_lanes = 8.0; // Number of individual lanes.

        let lane_scale = 1.0 / num_lanes; // Scale amount for one individual lane.
        let lane_left_edge_offset = -1.0; // X axis offset for leftmost lane.

        let base_width = 2.0;
        let lane_width = base_width / num_lanes;

        self.notes
            .iter()
            .map(|note| HitObject {
                x_scale: lane_scale * note.width as f32,
                x_offset: lane_left_edge_offset + (note.cell as f32 * lane_width),
                z_offset: (play_field_speed * note.offset) + HIT_AREA_Z_START,
            })
            .collect::<Vec<_>>()
    }
}

impl ChartInfo {
    pub fn create_timed_chart(&self) -> Result<Chart> {
        // log::debug!("{:#?}", self);

        // A BPM is the number of quarter notes in a minute(60 sconds).
        let seconds_per_minute = 60.0;
        let time_per_measure = seconds_per_minute
            / (self.starting_bpm as f32 / self.starting_measure.num_beats as f32);

        let mut notes = Vec::new();

        for note in &self.notes {
            notes.push(TimedNote::new(
                self.music_starting_offset
                    + ((note.measure as f32 * time_per_measure)
                        + (time_per_measure * (note.offset as f32 / self.resolution as f32))),
                note.cell,
                note.width,
            ))
        }

        log::debug!("Timed notes length: {}", notes.len());

        let chart = Chart { notes };

        Ok(chart)
    }
}
