use std::thread;
use std::time::Duration;

use anyhow::Result;
use winit::{
    dpi,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{audio::AudioSystem, input::InputHandler};

mod audio;
mod input;

fn main() {
    println!("Starting Chizumu...");

    let event_loop = EventLoop::new().unwrap();
    let _window = WindowBuilder::new()
        .with_title("Chizumu")
        .with_inner_size(dpi::PhysicalSize::new(1920, 1200))
        .with_position(dpi::PhysicalPosition::new(100, 100))
        .build(&event_loop)
        .unwrap();

    let mut audio_system = AudioSystem::new().unwrap();
    let music_index = audio_system
        // .load_music_data("data/Music/CELERITAS.ogg")
        .load_music_data("data/Music/hitotoki_tokimeki.ogg")
        .unwrap();
    audio_system.play_music(music_index).unwrap();

    let input_handler = InputHandler::new();

    event_loop
        .run(move |event, elwt| {
            if let Event::WindowEvent { event, .. } = event {
                match event {
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
                    _ => (),
                }
            };
        })
        .unwrap();
}
