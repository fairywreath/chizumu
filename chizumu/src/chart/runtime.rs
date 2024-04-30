use anyhow::Result;
use nalgebra::Vector2;

use crate::chart::MusicPositionable;

use super::{ChartInfo, MusicPosition, Platform};

use chizumu_graphics::{
    game_components::{HitObject, PlatformObject, CURVE_SIDED_PLATFORM_BEZIER_SUBDIVISONS},
    mesh::plane::Plane,
    HIT_AREA_Z_START,
};

struct RuntimePlatform {
    platform: Platform,
    start_music_position: f32,
    end_music_position: f32,
}

impl RuntimePlatform {
    fn new(platform: Platform, start_music_position: f32, end_music_position: f32) -> Self {
        Self {
            platform,
            start_music_position,
            end_music_position,
        }
    }
}

pub struct RuntimeNote {
    /// Offset in seconds from the start of the piece.
    pub offset: f32,
    pub cell: u32,
    pub width: u32,
}

impl RuntimeNote {
    pub fn new(offset: f32, cell: u32, width: u32) -> Self {
        Self {
            offset,
            cell,
            width,
        }
    }
}

/// Structure used by the main game logic during run time.
pub struct RuntimeChart {
    notes: Vec<RuntimeNote>,
    platforms: Vec<RuntimePlatform>,

    pub chart_info: ChartInfo,
}

impl RuntimeChart {
    pub fn create_hit_objects(&self) -> Vec<HitObject> {
        let play_field_speed = 7.0; // z-axis movement per second.
        let num_lanes = 10.0; // Number of individual lanes.

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

    /// `runner_speed` - distance covered by runner per second.
    pub fn create_platform_objects(&self, runner_speed: f32) -> Vec<PlatformObject> {
        self.platforms
            .iter()
            .map(|p| {
                let start_runner_position = p.start_music_position * runner_speed;
                let end_runner_position = p.end_music_position * runner_speed;
                let z_length = end_runner_position - start_runner_position;
                let z_offset = HIT_AREA_Z_START;
                let bezier_subdivisions = CURVE_SIDED_PLATFORM_BEZIER_SUBDIVISONS as _;

                let plane_mesh = match &p.platform {
                    Platform::DynamicQuad(platform) => {
                        let params = &platform.params;
                        Plane::quad(
                            Vector2::new(params.start_placement_offset, z_offset),
                            Vector2::new(
                                params.start_placement_offset + params.start_width,
                                z_offset,
                            ),
                            Vector2::new(params.end_placement_offset, z_offset + z_length),
                            Vector2::new(
                                params.end_placement_offset + params.end_width,
                                z_offset + z_length,
                            ),
                        )
                    }
                    Platform::DoubleSidedBezier(platform) => {
                        let params = &platform.params;
                        // XXX TODO: Make utility function for bezier 2d coord conversion these.
                        // XXX TODO: Make utiity function for music positition seconds to runner position.
                        let left_side_control_points = &platform.left_side_control_points;
                        let left_side_control_points_z = (
                            self.chart_info.music_position_to_seconds(
                                &platform.left_side_control_points.0.music_position,
                            ) * runner_speed
                                - start_runner_position
                                + z_offset,
                            self.chart_info.music_position_to_seconds(
                                &platform.left_side_control_points.1.music_position,
                            ) * runner_speed
                                - start_runner_position
                                + z_offset,
                        );
                        let left_side_control_points_2d = (
                            Vector2::new(
                                left_side_control_points.0.placement_offset,
                                left_side_control_points_z.0,
                            ),
                            Vector2::new(
                                left_side_control_points.1.placement_offset,
                                left_side_control_points_z.1,
                            ),
                        );

                        let right_side_control_points = &platform.right_side_control_points;
                        let right_side_control_points_z = (
                            self.chart_info.music_position_to_seconds(
                                &platform.right_side_control_points.0.music_position,
                            ) * runner_speed
                                - start_runner_position
                                + z_offset,
                            self.chart_info.music_position_to_seconds(
                                &platform.right_side_control_points.1.music_position,
                            ) * runner_speed
                                - start_runner_position
                                + z_offset,
                        );
                        let right_side_control_points_2d = (
                            Vector2::new(
                                right_side_control_points.0.placement_offset,
                                right_side_control_points_z.0,
                            ),
                            Vector2::new(
                                right_side_control_points.1.placement_offset,
                                right_side_control_points_z.1,
                            ),
                        );

                        Plane::double_sided_cubic_bezier(
                            Vector2::new(params.start_placement_offset, z_offset),
                            Vector2::new(params.end_placement_offset, z_offset + z_length),
                            left_side_control_points_2d,
                            Vector2::new(
                                params.start_placement_offset + params.start_width,
                                z_offset,
                            ),
                            Vector2::new(
                                params.end_placement_offset + params.end_width,
                                z_offset + z_length,
                            ),
                            right_side_control_points_2d,
                            bezier_subdivisions,
                        )
                    }
                    Platform::DoubleSidedParallelBezier(platform) => {
                        let params = &platform.params;

                        let control_points = &platform.control_points;
                        let control_points_z = (
                            self.chart_info.music_position_to_seconds(
                                &platform.control_points.0.music_position,
                            ) * runner_speed
                                - start_runner_position
                                + z_offset,
                            self.chart_info.music_position_to_seconds(
                                &platform.control_points.1.music_position,
                            ) * runner_speed
                                - start_runner_position
                                + z_offset,
                        );
                        let control_points_2d = (
                            Vector2::new(control_points.0.placement_offset, control_points_z.0),
                            Vector2::new(control_points.1.placement_offset, control_points_z.1),
                        );

                        Plane::double_sided_parallel_cubic_bezier(
                            Vector2::new(params.start_placement_offset, z_offset),
                            Vector2::new(params.end_placement_offset, z_offset + z_length),
                            control_points_2d,
                            platform.width,
                            bezier_subdivisions,
                        )
                    }
                    Platform::SingleSidedBezier(platform) => {
                        let params = &platform.params;

                        let control_points = &platform.control_points;
                        let control_points_z = (
                            self.chart_info.music_position_to_seconds(
                                &platform.control_points.0.music_position,
                            ) * runner_speed
                                - start_runner_position
                                + z_offset,
                            self.chart_info.music_position_to_seconds(
                                &platform.control_points.1.music_position,
                            ) * runner_speed
                                - start_runner_position
                                + z_offset,
                        );
                        let control_points_2d = (
                            Vector2::new(control_points.0.placement_offset, control_points_z.0),
                            Vector2::new(control_points.1.placement_offset, control_points_z.1),
                        );

                        let (v0, v1, v2, v3) = {
                            let v0 = Vector2::new(params.start_placement_offset, z_offset);
                            let v1 = Vector2::new(params.end_placement_offset, z_offset + z_length);
                            let v2 = Vector2::new(
                                params.start_placement_offset + params.start_width,
                                z_offset,
                            );
                            let v3 = Vector2::new(
                                params.end_placement_offset + params.end_width,
                                z_offset + z_length,
                            );

                            if platform.is_left {
                                (v0, v1, v2, v3)
                            } else {
                                (v2, v3, v0, v1)
                            }
                        };

                        // XXX TODO: Properly support single sided (less triangles) bezier planes in renderer.
                        // Plane::single_sided_cubic_bezier(
                        //     v0,
                        //     v1,
                        //     control_points_2d,
                        //     v2,
                        //     v3,
                        //     bezier_subdivisions,
                        // )
                        Plane::double_sided_cubic_bezier(
                            v0,
                            v1,
                            control_points_2d,
                            v2,
                            v3,
                            (v2, v3),
                            bezier_subdivisions,
                        )
                    }
                };

                PlatformObject::new_dynamic_plane(
                    start_runner_position,
                    end_runner_position,
                    plane_mesh,
                )
            })
            .collect::<Vec<_>>()
    }
}

impl ChartInfo {
    fn music_position_to_seconds(&self, music_position: &MusicPosition) -> f32 {
        let seconds_per_minute = 60.0;
        let time_per_measure = seconds_per_minute
            / (self.starting_bpm as f32 / self.starting_measure.num_beats as f32);
        self.music_starting_offset
            + ((music_position.measure as f32 * time_per_measure)
                + (time_per_measure * music_position.offset))
    }

    pub fn create_runtime_chart(self) -> Result<RuntimeChart> {
        log::debug!("{:#?}", self);

        let platforms = self
            .platforms
            .iter()
            .map(|p| RuntimePlatform {
                platform: p.clone(),
                start_music_position: self.music_position_to_seconds(&p.start_music_position()),
                end_music_position: self.music_position_to_seconds(&p.end_music_position()),
            })
            .collect::<Vec<_>>();

        let mut notes = Vec::new();
        for note in &self.notes {
            notes.push(RuntimeNote::new(
                self.music_position_to_seconds(&note.music_position),
                note.cell,
                note.width,
            ))
        }

        let chart = RuntimeChart {
            notes,
            platforms,
            chart_info: self,
        };
        Ok(chart)
    }
}
