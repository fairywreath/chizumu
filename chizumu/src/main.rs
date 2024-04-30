use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use winit::{
    dpi,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use chizumu_graphics::renderer::Renderer;

use crate::chart::parse::parse_chart_file;
use crate::chart::runtime;
use crate::game::conductor::Conductor;
use crate::game::GameState;
use crate::{core::audio::AudioSystem, core::input::RhythmControlInputHandler};

mod chart;
mod core;
mod game;

fn main() {
    let env = env_logger::Env::default()
        .filter_or("MY_LOG_LEVEL", "trace")
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(env);

    log::info!("Starting Chizumu...");

    // Initialize window.
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Chizumu")
        .with_inner_size(dpi::PhysicalSize::new(1920, 1200))
        .with_position(dpi::PhysicalPosition::new(100, 100))
        .build(&event_loop)
        .unwrap();

    // Initialize renderer.
    let mut renderer = Renderer::new(&window, &window).unwrap();

    // Initialize audio system.
    let mut audio_system = AudioSystem::new().unwrap();

    // Initialize rhythm control (game) input handler.
    let input_handler = RhythmControlInputHandler::new();

    // Parse chart file.
    let runtime_chart = parse_chart_file("assets/charts/lateral_arc_of_flame.czm").unwrap();
    let runner_speed = 7.0;

    // Create renderer resources based on the parsed chart.
    renderer
        .set_platform_objects(runtime_chart.create_platform_objects(runner_speed))
        .unwrap();
    renderer.add_hit_objects(&runtime_chart.create_hit_objects());

    // Load chart music.
    let music_index = audio_system
        .load_music_data(&runtime_chart.chart_info.music_file_path)
        .unwrap();

    // Initialize game/player state.
    let mut game_state = GameState::new();
    game_state.set_chart(runtime_chart);

    // Connductor keeps track of the current music position.
    let mut conductor = Conductor::new();

    let mut last_music_position = 0.0;
    let mut last_frame_time = Instant::now();

    // Start the music.
    conductor
        .start_music(&mut audio_system, music_index)
        .unwrap();

    event_loop
        .run(move |event, eltw| {
            eltw.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => {
                    input_handler.handle_window_event(&event);
                    match event {
                        WindowEvent::CloseRequested => {
                            eltw.exit();
                        }
                        WindowEvent::Resized(_) => {
                            // XXX: Explicitly tell the swapchain(held by `Device`) to be recreated/resized.
                        }
                        WindowEvent::RedrawRequested => {
                            renderer.render().unwrap();
                        }
                        _ => (),
                    }
                }
                Event::AboutToWait => {
                    let now = Instant::now();
                    let frame_dt = now - last_frame_time;
                    last_frame_time = now;

                    let current_music_position = conductor.get_current_music_position().unwrap();
                    let music_dt = current_music_position - last_music_position;
                    last_music_position = current_music_position;

                    renderer
                        .update(frame_dt.as_secs_f32(), music_dt * runner_speed)
                        .unwrap();

                    game_state.update_current_music_position(current_music_position);

                    window.request_redraw();
                }
                _ => (),
            }
        })
        .unwrap();
}
