use std::{collections::HashMap, sync::Mutex};

use winit::{
    event::WindowEvent,
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
pub enum RhythmControl {
    TapBottom1,
    TapBottom2,
    TapBottom3,
    TapBottom4,
    TapBottom5,
    TapBottom6,
    TapBottom7,
    TapBottom8,
    TapTop1,
    TapTop2,
    TapTop3,
    TapTop4,
    TapTop5,
    TapTop6,
    TapTop7,
    TapTop8,
    SwitchLane,
    TapAvatarLeft,
    TapAvatarMiddle,
    TapAvatarRight,
}

pub struct InputHandler {
    rhythm_control_keybindings: HashMap<KeyCode, RhythmControl>,

    /// XXX: Use an existing audio system to properly mix with music sound(?)
    audio_system: Mutex<AudioSystem>,
}

impl InputHandler {
    pub fn new() -> Self {
        let mut rhythm_control_keybindings = HashMap::new();

        rhythm_control_keybindings.insert(KeyCode::Z, RhythmControl::TapBottom1);
        rhythm_control_keybindings.insert(KeyCode::X, RhythmControl::TapBottom2);
        rhythm_control_keybindings.insert(KeyCode::C, RhythmControl::TapBottom3);
        rhythm_control_keybindings.insert(KeyCode::V, RhythmControl::TapBottom4);
        rhythm_control_keybindings.insert(KeyCode::B, RhythmControl::TapBottom5);
        rhythm_control_keybindings.insert(KeyCode::N, RhythmControl::TapBottom6);
        rhythm_control_keybindings.insert(KeyCode::M, RhythmControl::TapBottom7);
        rhythm_control_keybindings.insert(KeyCode::Comma, RhythmControl::TapBottom8);

        rhythm_control_keybindings.insert(KeyCode::A, RhythmControl::TapTop1);
        rhythm_control_keybindings.insert(KeyCode::S, RhythmControl::TapTop2);
        rhythm_control_keybindings.insert(KeyCode::D, RhythmControl::TapTop3);
        rhythm_control_keybindings.insert(KeyCode::F, RhythmControl::TapTop4);
        rhythm_control_keybindings.insert(KeyCode::G, RhythmControl::TapTop5);
        rhythm_control_keybindings.insert(KeyCode::H, RhythmControl::TapTop6);
        rhythm_control_keybindings.insert(KeyCode::J, RhythmControl::TapTop7);
        rhythm_control_keybindings.insert(KeyCode::K, RhythmControl::TapTop8);

        rhythm_control_keybindings.insert(KeyCode::Space, RhythmControl::SwitchLane);

        rhythm_control_keybindings.insert(KeyCode::Q, RhythmControl::TapAvatarLeft);
        rhythm_control_keybindings.insert(KeyCode::W, RhythmControl::TapAvatarMiddle);
        rhythm_control_keybindings.insert(KeyCode::E, RhythmControl::TapAvatarRight);

        Self {
            rhythm_control_keybindings,
            audio_system: Mutex::new(AudioSystem::new().unwrap()),
        }
    }

    pub fn handle_window_event(&self, window_event: &WindowEvent) {
        match window_event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(KeyCode::from(&event.physical_key))
            }
            _ => {}
        }
    }

    fn handle_keyboard_input(&self, keycode: KeyCode) {
        if let Some(rhythm_control) = self.rhythm_control_keybindings.get(&keycode) {
            self.update_rhythm_control(rhythm_control);
        }
    }

    fn update_rhythm_control(&self, rhythm_control: &RhythmControl) {
        match rhythm_control {
            RhythmControl::SwitchLane => {
                self.audio_system
                    .lock()
                    .unwrap()
                    .play_sound_effect(SFX_TAP_B_INDEX)
                    .unwrap();
            }
            _ => {
                self.audio_system
                    .lock()
                    .unwrap()
                    .play_sound_effect(SFX_TAP_A_INDEX)
                    .unwrap();
            }
        }
    }
}
