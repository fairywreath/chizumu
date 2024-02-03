use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use anyhow::{anyhow, Result};

use super::*;

static COMMENT_STR: &str = "//";

enum Tag {
    Resolution,
    StartingBpm,
    StartingMeasure,
    BpmChanges,
    MeasureChanges,
    PlayfieldChanges,
    Notes,
    MusicFilePath,
    MusicStartingOffset,
}

impl TryFrom<&str> for Tag {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "RESOLUTION" => Ok(Tag::Resolution),
            "STARTING_BPM" => Ok(Tag::StartingBpm),
            "STARTING_MEASURE" => Ok(Tag::StartingMeasure),
            "BPM_CHANGES" => Ok(Tag::BpmChanges),
            "MEASURE_CHANGES" => Ok(Tag::MeasureChanges),
            "PLAYFIELD_CHANGES" => Ok(Tag::PlayfieldChanges),
            "NOTES" => Ok(Tag::Notes),
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

fn parse_chart_file_to_chart_info(file_path: &str) -> Result<ChartInfo> {
    let lines = read_lines(file_path)?;

    let initial_chart_info = ChartInfo {
        resolution: 0,
        starting_bpm: 0,
        starting_measure: TimeSignature {
            num_beats: 0,
            note_value: 0,
        },
        bpm_changes: Vec::new(),
        measure_changes: Vec::new(),
        notes: Vec::new(),
        playfield_changes: Vec::new(),
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
                            Tag::Resolution => chart_info.resolution = subs[0].parse().unwrap(),
                            Tag::Notes => chart_info.notes.push(Note {
                                note_type: NoteType::try_from(subs[0]).unwrap(),
                                measure: subs[1].parse().unwrap(),
                                offset: subs[2].parse().unwrap(),
                                cell: subs[3].parse().unwrap(),
                                width: subs[4].parse().unwrap(),
                            }),
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

pub fn parse_chart_file(file_path: &str) -> Result<(ChartInfo, Chart)> {
    let chart_info = parse_chart_file_to_chart_info(file_path)?;
    let chart = chart_info.create_timed_chart()?;

    Ok((chart_info, chart))
}
