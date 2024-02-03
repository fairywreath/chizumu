use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use winit::{
    dpi,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    raw_window_handle::{HasRawWindowHandle, HasWindowHandle},
    window::{Window, WindowBuilder},
};

use chizumu_graphics::{gpu::device::Device, renderer::Renderer};

use crate::chart::parse::parse_chart_file;
use crate::game::conductor::Conductor;
use crate::game::GameState;
use crate::{core::audio::AudioSystem, core::input::InputHandler};

mod chart;
mod core;
mod game;

fn main() {
    let env = env_logger::Env::default()
        .filter_or("MY_LOG_LEVEL", "trace")
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(env);

    log::info!("Starting Chizumu...");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Chizumu")
        .with_inner_size(dpi::PhysicalSize::new(1920, 1200))
        .with_position(dpi::PhysicalPosition::new(100, 100))
        .build(&event_loop)
        .unwrap();

    let mut audio_system = AudioSystem::new().unwrap();
    let music_index = audio_system
        // .load_music_data("data/music/hitotoki_tokimeki.ogg")
        // .load_music_data("data/music/CELERITAS.ogg")
        .load_music_data("data/music/winddrums vs cosMo - Divine's or Deal_cut.ogg")
        .unwrap();

    let input_handler = InputHandler::new();
    let mut renderer = Renderer::new(Arc::new(Device::new(&window, &window).unwrap())).unwrap();

    let (chart_info, chart_timed) = parse_chart_file("data/charts/divine's_or_deal.czm").unwrap();

    let mut game_state = GameState::new();
    game_state.set_chart(chart_timed);
    renderer.add_hit_objects(&game_state.get_chart().create_hit_objects());

    let mut conductor = Conductor::new();
    conductor
        .start_music(&mut audio_system, music_index)
        .unwrap();

    let mut last_render_time = Instant::now();

    event_loop
        .run(move |event, eltw| {
            eltw.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        eltw.exit();
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        input_handler.handle_window_event(&event);
                    }
                    WindowEvent::Resized(_) => {
                        // XXX: Explicitly tell the swapchain(held by `Device`) to be recreated/resized?
                    }
                    WindowEvent::RedrawRequested => {
                        renderer.render().unwrap();
                    }
                    _ => (),
                },
                Event::AboutToWait => {
                    let now = Instant::now();
                    let dt = now - last_render_time;
                    last_render_time = now;

                    renderer.advance_hit_runner(dt.as_secs_f32() * 8.0);
                    renderer.update(dt.as_secs_f32()).unwrap();

                    game_state.update_current_music_position(
                        conductor.get_current_music_position().unwrap(),
                    );

                    window.request_redraw();
                }
                _ => (),
            }
        })
        .unwrap();
}
