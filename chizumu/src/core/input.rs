use std::{collections::HashMap, hash::Hash};

use parking_lot::{Mutex, RwLock};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    keyboard::{KeyCode as WinitKeyCode, PhysicalKey},
};

use super::audio::*;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
enum KeyCode {
    Escape,
    Left,
    Up,
    Right,
    Down,
    Any,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Space,
    Comma,
}

impl From<&PhysicalKey> for KeyCode {
    fn from(key: &PhysicalKey) -> Self {
        match key {
            PhysicalKey::Code(WinitKeyCode::KeyA) => KeyCode::A,
            PhysicalKey::Code(WinitKeyCode::KeyB) => KeyCode::B,
            PhysicalKey::Code(WinitKeyCode::KeyC) => KeyCode::C,
            PhysicalKey::Code(WinitKeyCode::KeyD) => KeyCode::D,
            PhysicalKey::Code(WinitKeyCode::KeyE) => KeyCode::E,
            PhysicalKey::Code(WinitKeyCode::KeyF) => KeyCode::F,
            PhysicalKey::Code(WinitKeyCode::KeyG) => KeyCode::G,
            PhysicalKey::Code(WinitKeyCode::KeyH) => KeyCode::H,
            PhysicalKey::Code(WinitKeyCode::KeyI) => KeyCode::I,
            PhysicalKey::Code(WinitKeyCode::KeyJ) => KeyCode::J,
            PhysicalKey::Code(WinitKeyCode::KeyK) => KeyCode::K,
            PhysicalKey::Code(WinitKeyCode::KeyL) => KeyCode::L,
            PhysicalKey::Code(WinitKeyCode::KeyM) => KeyCode::M,
            PhysicalKey::Code(WinitKeyCode::KeyN) => KeyCode::N,
            PhysicalKey::Code(WinitKeyCode::KeyO) => KeyCode::O,
            PhysicalKey::Code(WinitKeyCode::KeyP) => KeyCode::P,
            PhysicalKey::Code(WinitKeyCode::KeyQ) => KeyCode::Q,
            PhysicalKey::Code(WinitKeyCode::KeyR) => KeyCode::R,
            PhysicalKey::Code(WinitKeyCode::KeyS) => KeyCode::S,
            PhysicalKey::Code(WinitKeyCode::KeyT) => KeyCode::T,
            PhysicalKey::Code(WinitKeyCode::KeyU) => KeyCode::U,
            PhysicalKey::Code(WinitKeyCode::KeyV) => KeyCode::V,
            PhysicalKey::Code(WinitKeyCode::KeyW) => KeyCode::W,
            PhysicalKey::Code(WinitKeyCode::KeyX) => KeyCode::X,
            PhysicalKey::Code(WinitKeyCode::KeyY) => KeyCode::Y,
            PhysicalKey::Code(WinitKeyCode::KeyZ) => KeyCode::Z,
            PhysicalKey::Code(WinitKeyCode::Comma) => KeyCode::Comma,
            PhysicalKey::Code(WinitKeyCode::Space) => KeyCode::Space,
            _ => KeyCode::Any,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
enum RhythmControlInput {
    Tap1,
    Tap2,
    Tap3,
    Tap4,
    TapMove1,
    TapMove2,
    TapWide,
}

enum RhythmControlInputState {
    Pressed,
    Unpressed,
}

struct RhythmControlState {
    states: HashMap<RhythmControlInput, RhythmControlInputState>,
}

impl RhythmControlState {
    fn new() -> Self {
        let mut states = HashMap::new();

        states.insert(RhythmControlInput::Tap1, RhythmControlInputState::Unpressed);
        states.insert(RhythmControlInput::Tap2, RhythmControlInputState::Unpressed);
        states.insert(RhythmControlInput::Tap3, RhythmControlInputState::Unpressed);
        states.insert(RhythmControlInput::Tap4, RhythmControlInputState::Unpressed);
        states.insert(
            RhythmControlInput::TapMove1,
            RhythmControlInputState::Unpressed,
        );
        states.insert(
            RhythmControlInput::TapMove2,
            RhythmControlInputState::Unpressed,
        );
        states.insert(
            RhythmControlInput::TapWide,
            RhythmControlInputState::Unpressed,
        );

        Self { states }
    }
}

pub(crate) struct RhythmControlInputHandler {
    rhythm_control_keybindings: HashMap<KeyCode, RhythmControlInput>,

    /// XXX: Maybe an RwLock is better here.
    rhythm_control_state: Mutex<RhythmControlState>,

    /// XXX: Use an existing audio system to properly mix with music sound(?)
    audio_system: Mutex<AudioSystem>,
}

impl RhythmControlInputHandler {
    pub(crate) fn new() -> Self {
        let mut rhythm_control_keybindings = HashMap::new();

        rhythm_control_keybindings.insert(KeyCode::Q, RhythmControlInput::Tap1);
        rhythm_control_keybindings.insert(KeyCode::W, RhythmControlInput::Tap2);
        rhythm_control_keybindings.insert(KeyCode::E, RhythmControlInput::Tap3);
        rhythm_control_keybindings.insert(KeyCode::R, RhythmControlInput::Tap4);
        rhythm_control_keybindings.insert(KeyCode::Space, RhythmControlInput::TapWide);

        Self {
            rhythm_control_keybindings,
            rhythm_control_state: Mutex::new(RhythmControlState::new()),
            audio_system: Mutex::new(AudioSystem::new().unwrap()),
        }
    }

    pub(crate) fn handle_window_event(&self, window_event: &WindowEvent) {
        match window_event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(KeyCode::from(&event.physical_key), event.state)
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.handle_mouse_input(&button, *state)
            }
            _ => {}
        }
    }

    fn handle_keyboard_input(&self, keycode: KeyCode, state: ElementState) {
        if let Some(control_input) = self.rhythm_control_keybindings.get(&keycode) {
            self.update_rhythm_control_state(*control_input, state);
        }
    }

    fn handle_mouse_input(&self, button: &MouseButton, state: ElementState) {
        let control_input = match button {
            MouseButton::Left => Some(RhythmControlInput::TapMove1),
            MouseButton::Right => Some(RhythmControlInput::TapMove2),
            _ => None,
        };

        if let Some(control_input) = control_input {
            self.update_rhythm_control_state(control_input, state);
        }
    }

    fn update_rhythm_control_state(&self, control_input: RhythmControlInput, state: ElementState) {
        let mut control_state = self.rhythm_control_state.lock();
        match state {
            ElementState::Pressed => {
                if let Some(input_state) = control_state.states.get(&control_input) {
                    match input_state {
                        RhythmControlInputState::Pressed => {
                            // Held on pressed.
                        }
                        RhythmControlInputState::Unpressed => {
                            // Unpressed -> pressed.
                            self.play_tap_sound(control_input);
                            control_state
                                .states
                                .insert(control_input, RhythmControlInputState::Pressed);
                        }
                    }
                }
            }
            ElementState::Released => {
                control_state
                    .states
                    .insert(control_input, RhythmControlInputState::Unpressed);
            }
        }
    }

    /// XXX: Figure out the best way to play these tap sounds as fast as possible, want minimum latency between press -> sound.
    fn play_tap_sound(&self, rhythm_control: RhythmControlInput) {
        match rhythm_control {
            _ => {
                self.audio_system.lock().play_sound_effect(0).unwrap();
            }
        }
    }
}
