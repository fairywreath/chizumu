/*!
 * Chart writing and parsing.
 * Implementation loosely based on Chunithm's format.
 */

use chizumu_graphics::hit::HitObject;

mod parse;

struct ChartInfo {
    /// Chart mapping information.
    resolution: u32,
    starting_bpm: u32,
    starting_measure: TimeSignature,
    notes: Vec<Note>,
    measure_changes: Vec<MeasureChange>,
    bpm_changes: Vec<BpmChange>,

    /// Cosmetic/visual information.
    playfield_changes: Vec<PlayfieldChange>,

    music_file_path: String,
    /// Starting offset in seconds before the first measure.
    music_staring_offset: f32,
}

enum NoteType {
    TAP,
}

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

struct TimeSignature {
    /// Top value/numerator.
    num_beats: u32,
    /// Bottom value/denomintaor.
    note_value: u32,
}

struct MeasureChange {
    /// The global measure and offset in which the change takes place.
    /// The specific time of this change depends on the last measure/time siganuture + bpm values.
    measure: u32,
    offset: u32,

    time_signature: TimeSignature,
}

struct BpmChange {
    /// The global measure and offset in which the change takes place.
    /// The specific time of this change depends on the last measure/time siganuture + bpm values.
    measure: u32,
    offset: u32,

    bpm: u32,
}

/// Purely cosmetic playfield change.
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
    offset: f32,

    cell: u32,
    width: u32,

    /// Structure to be submitted to the renderer.
    hit_object: Option<HitObject>,
}

/// Structure used by the main game logic during run time.
pub struct Chart {
    notes: Vec<TimedNote>,
}
