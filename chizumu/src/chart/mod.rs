/*!
 * Chart writing and parsing.
 * Implementation based on Chunithm's format.
 */
use anyhow::{anyhow, Result};

pub mod parse;
pub mod runtime;

#[derive(Debug, Clone)]
struct MusicPosition {
    measure: u32,
    offset: f32,
}

impl MusicPosition {
    fn new(measure: u32, offset: f32) -> Self {
        Self { measure, offset }
    }
}

#[derive(Debug, Clone)]
pub struct ChartInfo {
    /// Chart mapping information.
    starting_bpm: u32,
    starting_measure: TimeSignature,
    bpm_changes: Vec<BpmChange>,
    measure_changes: Vec<MeasureChange>,
    notes: Vec<Note>,
    platforms: Vec<Platform>,

    playfield_speed_changes: Vec<PlayfieldSpeedChange>,

    pub music_file_path: String,
    /// Starting offset in seconds before the first measure.
    music_starting_offset: f32,
}

#[derive(Debug, Clone, Copy)]
enum NoteInputType {
    Tap1,
    Tap2,
    Tap3,
    Tap4,
    TapMove1,
    TapMove2,
    TapWidth,
}

impl TryFrom<&str> for NoteInputType {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "T1" => Ok(NoteInputType::Tap1),
            "T2" => Ok(NoteInputType::Tap2),
            "T3" => Ok(NoteInputType::Tap3),
            "T4" => Ok(NoteInputType::Tap4),
            "TM1" => Ok(NoteInputType::TapMove1),
            "TM2" => Ok(NoteInputType::TapMove2),
            "TW" => Ok(NoteInputType::TapWidth),
            _ => Err(anyhow!(
                "Invalid string for NoteInputType conversion: {}",
                s
            )),
        }
    }
}

#[derive(Debug, Clone)]
struct Note {
    music_position: MusicPosition,

    note_type: NoteInputType,

    /// Cell gives the position of the leftmost cell of the note, while width gives the number of
    /// cells the note covers.
    cell: u32,
    width: u32,
}

#[derive(Debug, Clone)]
struct TimeSignature {
    /// Top value/numerator.
    num_beats: u32,
    /// Bottom value/denomintaor.
    note_value: u32,
}

#[derive(Debug, Clone)]
struct MeasureChange {
    /// The global measure and offset in which the change takes place.
    /// The specific time of this change depends on the last measure/time siganuture + bpm values.
    music_position: MusicPosition,

    time_signature: TimeSignature,
}

#[derive(Debug, Clone)]
struct BpmChange {
    /// The global measure and offset in which the change takes place.
    /// The specific time of this change depends on the last measure/time siganuture + bpm values.
    music_position: MusicPosition,

    bpm: u32,
}

/// Purely cosmetic playfield change.
#[derive(Debug, Clone)]
struct PlayfieldSpeedChange {
    /// The global measure and offset in which the change takes place.
    /// The specific time of this change depends on the last measure/time siganuture + bpm values.
    music_position: MusicPosition,

    /// In seconds.
    duration: f32,
    mutiplier: f32,
}

#[derive(Debug, Clone)]
struct CommonPlatformParameters {
    start_music_position: MusicPosition,
    end_music_position: MusicPosition,
    start_placement_offset: f32,
    end_placement_offset: f32,
    start_width: f32,
    end_width: f32,
}

#[derive(Debug, Clone)]
struct DynamicQuadPlatform {
    params: CommonPlatformParameters,
}

pub trait MusicPositionable {
    fn start_music_position(&self) -> MusicPosition;
    fn end_music_position(&self) -> MusicPosition;
}

#[derive(Debug, Clone)]
struct StaticPlatform {
    start_music_position: MusicPosition,
    placement_offset: f32,
    width: f32,
}

#[derive(Debug, Clone)]
struct PlatformBezierControlPoint {
    music_position: MusicPosition,
    placement_offset: f32, // X-axis placement.
}

#[derive(Debug, Clone)]
struct DoubleSidedBezierPlatform {
    params: CommonPlatformParameters,
    left_side_control_points: (PlatformBezierControlPoint, PlatformBezierControlPoint),
    right_side_control_points: (PlatformBezierControlPoint, PlatformBezierControlPoint),
}

/// Parallel bezier control points.
#[derive(Debug, Clone)]
struct DoubleSidedParallelBezierPlatform {
    params: CommonPlatformParameters,
    control_points: (PlatformBezierControlPoint, PlatformBezierControlPoint),
    width: f32,
}

#[derive(Debug, Clone)]
struct SingleSideBezierPlatform {
    params: CommonPlatformParameters,
    control_points: (PlatformBezierControlPoint, PlatformBezierControlPoint),
    is_left: bool, // Whether the left or right side is the curved side.
}

#[derive(Debug, Clone)]
enum Platform {
    // Static(StaticPlatform),
    DynamicQuad(DynamicQuadPlatform),
    DoubleSidedBezier(DoubleSidedBezierPlatform),
    DoubleSidedParallelBezier(DoubleSidedParallelBezierPlatform),
    SingleSidedBezier(SingleSideBezierPlatform),
}

impl MusicPositionable for Platform {
    fn start_music_position(&self) -> MusicPosition {
        match self {
            Self::DynamicQuad(platform) => platform.params.start_music_position.clone(),
            Self::DoubleSidedBezier(platform) => platform.params.start_music_position.clone(),
            Self::DoubleSidedParallelBezier(platform) => {
                platform.params.start_music_position.clone()
            }
            Self::SingleSidedBezier(platform) => platform.params.start_music_position.clone(),
        }
    }

    fn end_music_position(&self) -> MusicPosition {
        match self {
            Self::DynamicQuad(platform) => platform.params.end_music_position.clone(),
            Self::DoubleSidedBezier(platform) => platform.params.end_music_position.clone(),
            Self::DoubleSidedParallelBezier(platform) => platform.params.end_music_position.clone(),
            Self::SingleSidedBezier(platform) => platform.params.end_music_position.clone(),
        }
    }
}

#[derive(Debug, Clone)]
enum PlatformType {
    // XXX TODO: Properly support static/non moving platforms(ie. long moving platforms that do not change)
    // Static,
    DynamicQuad,
    DoubleSidedBezier,
    DoubleSidedParallelBezier,
    SingleSidedBezier,
}

impl TryFrom<&str> for PlatformType {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            // "STATIC" => Ok(PlatformType::Static),
            "DQ" => Ok(PlatformType::DynamicQuad),
            "DSB" => Ok(PlatformType::DoubleSidedBezier),
            "DSPB" => Ok(PlatformType::DoubleSidedParallelBezier),
            "SSB" => Ok(PlatformType::SingleSidedBezier),
            _ => Err(anyhow!("Invalid string for PlatformType conversion: {}", s)),
        }
    }
}
