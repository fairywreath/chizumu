use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use anyhow::{anyhow, Result};

use super::{runtime::RuntimeChart, *};

const COMMENT_STR: &str = "//";

enum Tag {
    StartingBpm,
    StartingMeasure,
    Notes,
    Platforms,
    BpmChanges,
    MeasureChanges,
    PlayfieldChanges,
    MusicFilePath,
    MusicStartingOffset,
}

impl TryFrom<&str> for Tag {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "STARTING_BPM" => Ok(Tag::StartingBpm),
            "STARTING_MEASURE" => Ok(Tag::StartingMeasure),
            "BPM_CHANGES" => Ok(Tag::BpmChanges),
            "MEASURE_CHANGES" => Ok(Tag::MeasureChanges),
            "PLAYFIELD_CHANGES" => Ok(Tag::PlayfieldChanges),
            "NOTES" => Ok(Tag::Notes),
            "PLATFORMS" => Ok(Tag::Platforms),
            "MUSIC_FILE_PATH" => Ok(Tag::MusicFilePath),
            "MUSIC_STARTING_OFFSET" => Ok(Tag::MusicStartingOffset),
            _ => Err(anyhow!("Invalid string for Tag conversion: {}", s)),
        }
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

/// Starts from index 0 of `subs`.
fn parse_bezier_control_points(subs: &[&str]) -> Result<PlatformBezierControlPoint> {
    Ok(PlatformBezierControlPoint {
        music_position: MusicPosition::new(subs[0].parse()?, subs[1].parse()?),
        placement_offset: subs[2].parse()?,
    })
}

fn parse_is_left(val: &str) -> Result<bool> {
    if val == "l" {
        Ok(true)
    } else if val == "r" {
        Ok(false)
    } else {
        Err(anyhow!("Unrecognized `is left` token `{}`", val))
    }
}

/// Starts from index 1 of `subs`.
fn parse_common_platform_parameters(subs: &[&str]) -> Result<CommonPlatformParameters> {
    Ok(CommonPlatformParameters {
        start_music_position: MusicPosition::new(subs[1].parse()?, subs[2].parse()?),
        end_music_position: MusicPosition::new(subs[3].parse()?, subs[4].parse()?),
        start_placement_offset: subs[5].parse()?,
        end_placement_offset: subs[6].parse()?,
        start_width: subs[7].parse()?,
        end_width: subs[8].parse()?,
    })
}

fn parse_platform(subs: &[&str]) -> Result<Platform> {
    let platform_type = PlatformType::try_from(subs[0]).unwrap();
    let platform;

    match platform_type {
        // PlatformType::Static => {
        //     platform = Platform::Static(StaticPlatform {
        //         start_music_position: MusicPosition::new(subs[1].parse()?, subs[2].parse()?),
        //         width: subs[3].parse()?,
        //         placement_offset: subs[4].parse()?,
        //     })
        // }
        PlatformType::DynamicQuad => {
            platform = Platform::DynamicQuad(DynamicQuadPlatform {
                params: parse_common_platform_parameters(subs)?,
            })
        }
        PlatformType::DoubleSidedBezier => {
            platform = Platform::DoubleSidedBezier(DoubleSidedBezierPlatform {
                params: parse_common_platform_parameters(subs)?,
                left_side_control_points: (
                    parse_bezier_control_points(&subs[9..])?,
                    parse_bezier_control_points(&subs[12..])?,
                ),
                right_side_control_points: (
                    parse_bezier_control_points(&subs[15..])?,
                    parse_bezier_control_points(&subs[18..])?,
                ),
            })
        }
        PlatformType::DoubleSidedParallelBezier => {
            platform = Platform::DoubleSidedParallelBezier(DoubleSidedParallelBezierPlatform {
                params: parse_common_platform_parameters(subs)?,
                control_points: (
                    parse_bezier_control_points(&subs[9..])?,
                    parse_bezier_control_points(&subs[12..])?,
                ),
                width: subs[15].parse()?,
            })
        }
        PlatformType::SingleSidedBezier => {
            platform = Platform::SingleSidedBezier(SingleSideBezierPlatform {
                params: parse_common_platform_parameters(subs)?,
                control_points: (
                    parse_bezier_control_points(&subs[9..])?,
                    parse_bezier_control_points(&subs[12..])?,
                ),
                is_left: parse_is_left(&subs[15])?,
            })
        }
    };

    Ok(platform)
}

fn parse_chart_file_to_chart_info(file_path: &str) -> Result<ChartInfo> {
    let lines = read_lines(file_path)?;

    let initial_chart_info = ChartInfo {
        starting_bpm: 0,
        starting_measure: TimeSignature {
            num_beats: 0,
            note_value: 0,
        },
        bpm_changes: Vec::new(),
        measure_changes: Vec::new(),
        notes: Vec::new(),
        platforms: Vec::new(),
        playfield_speed_changes: Vec::new(),
        music_file_path: String::new(),
        music_starting_offset: 0.0,
    };

    // XXX: Properly handle `unwrap`s and progate error.
    let chart_info = lines
        .flatten()
        .fold((initial_chart_info, None::<Tag>), |acc, line| {
            let line = line.trim();

            let mut chart_info = acc.0;
            let mut current_tag = acc.1;

            if !line.is_empty() && !line.starts_with(COMMENT_STR) {
                if let Some(tag) = &current_tag {
                    if let Ok(new_tag) = Tag::try_from(line) {
                        current_tag = Some(new_tag);
                    } else {
                        let subs = line.split_whitespace().collect::<Vec<_>>();
                        match tag {
                            Tag::StartingBpm => chart_info.starting_bpm = subs[0].parse().unwrap(),
                            Tag::StartingMeasure => {
                                chart_info.starting_measure = TimeSignature {
                                    num_beats: subs[0].parse().unwrap(),
                                    note_value: subs[1].parse().unwrap(),
                                }
                            }
                            Tag::Platforms => {
                                chart_info.platforms.push(parse_platform(&subs).unwrap());
                            }
                            // Tag::Notes => chart_info.notes.push(Note {
                            //     note_type: NoteType::try_from(subs[0]).unwrap(),
                            //     music_position: MusicPosition::new(
                            //         subs[1].parse().unwrap(),
                            //         subs[2].parse().unwrap(),
                            //     ),
                            //     cell: subs[3].parse().unwrap(),
                            //     width: subs[4].parse().unwrap(),
                            // }),
                            Tag::MusicFilePath => {
                                chart_info.music_file_path = String::from(subs[0])
                            }
                            Tag::MusicStartingOffset => {
                                chart_info.music_starting_offset = subs[0].parse().unwrap()
                            }
                            _ => {
                                todo!()
                            }
                        }
                    }
                } else {
                    current_tag = Some(Tag::try_from(line).unwrap());
                }
            }

            (chart_info, current_tag)
        })
        .0;

    Ok(chart_info)
}

pub fn parse_chart_file(file_path: &str) -> Result<(ChartInfo, RuntimeChart)> {
    let chart_info = parse_chart_file_to_chart_info(file_path)?;
    let chart = chart_info.create_runtime_chart()?;

    Ok((chart_info, chart))
}
