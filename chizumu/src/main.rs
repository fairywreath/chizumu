use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use winit::{
    dpi,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    raw_window_handle::{HasRawWindowHandle, HasWindowHandle},
    window::{Window, WindowBuilder},
};

use chizumu_graphics::gpu::device::Device;

use crate::{audio::AudioSystem, input::InputHandler};

mod audio;
mod input;

fn render(device: &Arc<Device>) -> Result<()> {
    device.frame_begin()?;

    let commands = device.get_current_command_buffer()?;
    commands.begin()?;
    device.command_transition_swapchain_image_layout_to_color_attachment(&commands);
    device.command_begin_rendering_swapchain(&commands);
    commands.end_rendering();
    device.command_transition_swapchain_image_layout_to_present(&commands);
    commands.end()?;

    device.queue_submit_commands_graphics(commands)?;
    device.swapchain_present()?;

    Ok(())
}

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

    // let mut audio_system = AudioSystem::new().unwrap();
    // let music_index = audio_system
    //     .load_music_data("data/Music/CELERITAS.ogg")
    //     // .load_music_data("data/Music/hitotoki_tokimeki.ogg")
    //     .unwrap();
    // audio_system.play_music(music_index).unwrap();

    let input_handler = InputHandler::new();
    let device = Arc::new(Device::new(&window, &window).unwrap());

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
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
                _ => (),
            },
            Event::AboutToWait => {
                render(&device).unwrap();
            }
            _ => (),
        })
        .unwrap();
}
