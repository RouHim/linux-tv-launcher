use crate::input::Action;
use gilrs::{Axis, Button, Event, EventType, Gamepad, Gilrs, MappingSource, PowerInfo};
use iced::futures::sink::SinkExt;
use iced::Subscription;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const BATTERY_CHECK_INTERVAL: Duration = Duration::from_mins(1  );
const DEADZONE: f32 = 0.6;

#[derive(Debug, Clone, PartialEq)]
pub struct GamepadInfo {
    pub power_info: PowerInfo,
    pub name: String,
    pub is_keyboard: bool,
}

#[derive(Debug, Clone)]
pub enum GamepadEvent {
    Input(Action),
    Battery(Vec<GamepadInfo>),
}

/// Device capabilities extracted from Gilrs for pure logic classification
struct GamepadCapabilities {
    is_sdl_mapped: bool,
    has_left_stick: bool,
    has_dpad: bool,
    has_face_buttons: bool,
    name: String,
}

impl GamepadCapabilities {
    fn from_gamepad(gp: &Gamepad) -> Self {
        let name = gp.name().to_string();

        let has_left_stick =
            gp.axis_code(Axis::LeftStickX).is_some() && gp.axis_code(Axis::LeftStickY).is_some();

        let has_dpad =
            gp.button_code(Button::DPadUp).is_some() && gp.button_code(Button::DPadDown).is_some();

        // South (A) is the minimum viable face button for a gamepad
        let has_face_buttons = gp.button_code(Button::South).is_some();

        Self {
            is_sdl_mapped: gp.mapping_source() == MappingSource::SdlMappings,
            has_left_stick,
            has_dpad,
            has_face_buttons,
            name,
        }
    }
}

struct AxisState {
    dir_x: i8,
    dir_y: i8,
}

impl AxisState {
    fn new() -> Self {
        Self { dir_x: 0, dir_y: 0 }
    }
}

pub fn gamepad_subscription() -> Subscription<GamepadEvent> {
    Subscription::run(|| {
        iced::stream::channel(
            100,
            |mut output: iced::futures::channel::mpsc::Sender<GamepadEvent>| async move {
                let mut gilrs = match Gilrs::new() {
                    Ok(g) => g,
                    Err(e) => {
                        eprintln!("Failed to initialize Gilrs: {}", e);
                        return;
                    }
                };

                let mut axis_state = AxisState::new();
                let mut last_battery_check = Instant::now();
                // Force an initial battery check immediately
                let mut current_battery_interval = Duration::ZERO;

                loop {
                    // 1. Process all available events (non-blocking)
                    while let Some(Event { event, .. }) = gilrs.next_event() {
                        if let Some(action) = process_event(event, &mut axis_state) {
                            let _ = output.send(GamepadEvent::Input(action)).await;
                        }
                    }

                    // 2. Periodic Battery Check
                    if last_battery_check.elapsed() >= current_battery_interval {
                        let batteries = gilrs
                            .gamepads()
                            .map(|(_, gp)| {
                                let name = gp.name().to_string();
                                let is_keyboard = is_likely_keyboard(&gp);
                                GamepadInfo {
                                    power_info: gp.power_info(),
                                    name,
                                    is_keyboard,
                                }
                            })
                            .collect();

                        let _ = output.send(GamepadEvent::Battery(batteries)).await;

                        last_battery_check = Instant::now();
                        current_battery_interval = BATTERY_CHECK_INTERVAL;
                    }

                    // 3. Yield to avoid busy loop
                    tokio::time::sleep(POLL_INTERVAL).await;
                }
            },
        )
    })
}

fn is_likely_keyboard(gp: &Gamepad) -> bool {
    let caps = GamepadCapabilities::from_gamepad(gp);
    classify_as_keyboard(&caps)
}

fn classify_as_keyboard(caps: &GamepadCapabilities) -> bool {
    // 1. If explicitly SDL mapped, it's a gamepad.
    if caps.is_sdl_mapped {
        return false;
    }

    // 2. If it lacks basic gamepad controls, it's likely a keyboard/other.
    // A functional gamepad for our UI needs at least navigation (Stick OR DPad) AND a Select button (South/A).
    let has_navigation = caps.has_left_stick || caps.has_dpad;
    let functional_gamepad = has_navigation && caps.has_face_buttons;

    if !functional_gamepad {
        return true;
    }

    // 3. Fallback: If it looks like a gamepad but claims to be a keyboard via name
    // This handles edge cases like "Gaming Keypads" that might want to be treated as keyboards visually?
    // Or maybe we trust the capability check more.
    // For now, let's keep the name check as a secondary filter for "Driver" mapped devices.
    let lower_name = caps.name.to_lowercase();
    if lower_name.contains("keyboard")
        || lower_name.contains("system control")
        || lower_name.contains("consumer control")
    {
        return true;
    }

    false
}

fn map_axis_value(value: f32) -> i8 {
    if value <= -DEADZONE {
        -1
    } else if value >= DEADZONE {
        1
    } else {
        0
    }
}

fn process_event(event: EventType, state: &mut AxisState) -> Option<Action> {
    match event {
        EventType::ButtonPressed(Button::South, _) => Some(Action::Select),
        EventType::ButtonPressed(Button::East, _) => Some(Action::Back),
        EventType::ButtonPressed(Button::West, _) => Some(Action::ContextMenu),
        EventType::ButtonPressed(Button::DPadUp, _) => Some(Action::Up),
        EventType::ButtonPressed(Button::DPadDown, _) => Some(Action::Down),
        EventType::ButtonPressed(Button::DPadLeft, _) => Some(Action::Left),
        EventType::ButtonPressed(Button::DPadRight, _) => Some(Action::Right),
        EventType::ButtonPressed(Button::LeftTrigger, _) => Some(Action::PrevCategory),
        EventType::ButtonPressed(Button::RightTrigger, _) => Some(Action::NextCategory),
        EventType::ButtonPressed(Button::LeftTrigger2, _) => Some(Action::PrevCategory),
        EventType::ButtonPressed(Button::RightTrigger2, _) => Some(Action::NextCategory),
        EventType::ButtonPressed(Button::Select, _) => Some(Action::ShowHelp),
        EventType::AxisChanged(gilrs::Axis::LeftStickX, value, _) => {
            let new_dir = map_axis_value(value);
            if new_dir != state.dir_x {
                state.dir_x = new_dir;
                match new_dir {
                    -1 => Some(Action::Left),
                    1 => Some(Action::Right),
                    _ => None,
                }
            } else {
                None
            }
        }
        EventType::AxisChanged(gilrs::Axis::LeftStickY, value, _) => {
            let new_dir = if value <= -DEADZONE {
                1
            } else if value >= DEADZONE {
                -1
            } else {
                0
            };
            if new_dir != state.dir_y {
                state.dir_y = new_dir;
                match new_dir {
                    -1 => Some(Action::Up),
                    1 => Some(Action::Down),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_as_keyboard_logic() {
        // Case 1: Xbox Controller (SDL Mapped) -> Gamepad (False)
        let xbox = GamepadCapabilities {
            is_sdl_mapped: true,
            has_left_stick: true,
            has_dpad: true,
            has_face_buttons: true,
            name: "Xbox 360 Controller".to_string(),
        };
        assert!(!classify_as_keyboard(&xbox), "Xbox should be a gamepad");

        // Case 2: Keychron (Driver Mapped, No Stick/Dpad) -> Keyboard (True)
        let keychron = GamepadCapabilities {
            is_sdl_mapped: false,
            has_left_stick: false,
            has_dpad: false,
            has_face_buttons: false, // Often only has 1-2 buttons mapped weirdly
            name: "Keychron Q3 Pro System Control".to_string(),
        };
        assert!(
            classify_as_keyboard(&keychron),
            "Keychron should be a keyboard"
        );

        // Case 3: Generic Gamepad (Driver Mapped, but functional) -> Gamepad (False)
        let generic_gamepad = GamepadCapabilities {
            is_sdl_mapped: false, // Driver mapped
            has_left_stick: true,
            has_dpad: true,
            has_face_buttons: true,
            name: "Generic USB Gamepad".to_string(),
        };
        assert!(
            !classify_as_keyboard(&generic_gamepad),
            "Generic functional gamepad should be a gamepad"
        );

        // Case 4: Device with name "Keyboard" but FULL gamepad capabilities (Driver mapped)
        // Current logic: If name contains "keyboard", we fallback to True.
        let gaming_keyboard = GamepadCapabilities {
            is_sdl_mapped: false,
            has_left_stick: true,
            has_dpad: false,
            has_face_buttons: true,
            name: "Wooting Keyboard".to_string(),
        };
        assert!(
            classify_as_keyboard(&gaming_keyboard),
            "Named keyboard should be detected as keyboard even if functional"
        );

        // Case 5: Broken/Partial Device (No Face buttons) -> Keyboard (True)
        let broken_device = GamepadCapabilities {
            is_sdl_mapped: false,
            has_left_stick: true,
            has_dpad: true,
            has_face_buttons: false, // Missing 'A' button
            name: "Unknown Device".to_string(),
        };
        assert!(
            classify_as_keyboard(&broken_device),
            "Device without face buttons is not a usable gamepad"
        );
    }
}
