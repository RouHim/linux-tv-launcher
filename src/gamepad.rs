use crate::input::Action;
use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Envelope, Replay, Ticks};
use gilrs::{Axis, Button, Event, EventType, Gamepad, GamepadId, Gilrs, MappingSource, PowerInfo};
use iced::futures::sink::SinkExt;
use iced::Subscription;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::error;

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const BATTERY_CHECK_INTERVAL: Duration = Duration::from_secs(5);
const REPEAT_DELAY: Duration = Duration::from_millis(400);
const REPEAT_INTERVAL: Duration = Duration::from_millis(100);
const DEADZONE: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq)]
enum GamepadInput {
    Press(Action),
    Release(Action),
}

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
                        error!("Failed to initialize Gilrs: {}", e);
                        return;
                    }
                };

                let mut axis_states: HashMap<GamepadId, AxisState> = HashMap::new();
                let mut last_battery_check = Instant::now();
                // Force an initial battery check immediately
                let mut current_battery_interval = Duration::ZERO;

                // Store active vibration effects to keep them alive while playing
                let mut active_effects: Vec<(gilrs::ff::Effect, Instant)> = Vec::new();
                let mut current_repeater: Option<(Action, Instant, Instant)> = None;

                loop {
                    // Clean up finished effects
                    active_effects.retain(|(_, expires_at)| *expires_at > Instant::now());

                    // 1. Process all available events (non-blocking)
                    while let Some(Event { id, event, .. }) = gilrs.next_event() {
                        match event {
                            EventType::Connected => {
                                trigger_connection_haptics(&mut gilrs, id, &mut active_effects);
                            }
                            EventType::Disconnected => {
                                axis_states.remove(&id);
                                continue;
                            }
                            _ => {}
                        }

                        let state = axis_states.entry(id).or_insert_with(AxisState::new);
                        if let Some(input) = process_event(event, state) {
                            match input {
                                GamepadInput::Press(action) => {
                                    let _ = output.send(GamepadEvent::Input(action)).await;
                                    if is_nav_action(action) {
                                        current_repeater =
                                            Some((action, Instant::now(), Instant::now()));
                                    }
                                }
                                GamepadInput::Release(action) => {
                                    if let Some((curr_action, _, _)) = current_repeater {
                                        if curr_action == action {
                                            current_repeater = None;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Handle Repeats
                    if let Some((action, start_time, last_emit)) = &mut current_repeater {
                        let now = Instant::now();
                        if now.duration_since(*start_time) >= REPEAT_DELAY
                            && now.duration_since(*last_emit) >= REPEAT_INTERVAL
                        {
                            let _ = output.send(GamepadEvent::Input(*action)).await;
                            *last_emit = now;
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

fn trigger_connection_haptics(
    gilrs: &mut Gilrs,
    connected_id: GamepadId,
    active_effects: &mut Vec<(gilrs::ff::Effect, Instant)>,
) {
    let gamepad = gilrs.gamepad(connected_id);
    if is_likely_keyboard(&gamepad) {
        return;
    }

    // Determine player number based on sorted IDs of valid gamepads
    let mut gamepads: Vec<_> = gilrs
        .gamepads()
        .filter(|(_, gp)| !is_likely_keyboard(gp))
        .map(|(id, _)| id)
        .collect();
    gamepads.sort_by_key(|id| usize::from(*id));

    if let Some(idx) = gamepads.iter().position(|&x| x == connected_id) {
        let player_number = idx + 1;

        // Vibrate 'player_number' times
        // Pulse 200ms, Interval 400ms

        for i in 0..player_number {
            let start_delay_ms = (i as u64) * 400;
            let start_delay = Ticks::from_ms(start_delay_ms as u32);
            let duration = Ticks::from_ms(200);

            // Attempt to create and play effect
            // We use a Strong rumble for notification
            let effect_result = EffectBuilder::new()
                .add_effect(BaseEffect {
                    kind: BaseEffectType::Strong { magnitude: 0xC000 }, // ~75% strength
                    scheduling: Replay {
                        play_for: duration,
                        with_delay: start_delay,
                        ..Default::default()
                    },
                    envelope: Envelope::default(),
                })
                .gamepads(&[connected_id])
                .finish(gilrs);

            if let Ok(effect) = effect_result {
                if effect.play().is_ok() {
                    let expires_at = Instant::now()
                        + Duration::from_millis(start_delay_ms)
                        + Duration::from_millis(200)
                        + Duration::from_millis(100);
                    active_effects.push((effect, expires_at));
                }
            }
        }
    }
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

fn process_event(event: EventType, state: &mut AxisState) -> Option<GamepadInput> {
    match event {
        EventType::ButtonPressed(Button::South, _) => Some(GamepadInput::Press(Action::Select)),
        EventType::ButtonPressed(Button::East, _) => Some(GamepadInput::Press(Action::Back)),
        EventType::ButtonPressed(Button::West, _) => Some(GamepadInput::Press(Action::ContextMenu)),
        EventType::ButtonPressed(Button::North, _) => Some(GamepadInput::Press(Action::AddApp)),
        EventType::ButtonPressed(Button::DPadUp, _) => Some(GamepadInput::Press(Action::Up)),
        EventType::ButtonPressed(Button::DPadDown, _) => Some(GamepadInput::Press(Action::Down)),
        EventType::ButtonPressed(Button::DPadLeft, _) => Some(GamepadInput::Press(Action::Left)),
        EventType::ButtonPressed(Button::DPadRight, _) => Some(GamepadInput::Press(Action::Right)),
        EventType::ButtonPressed(Button::LeftTrigger, _) => {
            Some(GamepadInput::Press(Action::PrevCategory))
        }
        EventType::ButtonPressed(Button::RightTrigger, _) => {
            Some(GamepadInput::Press(Action::NextCategory))
        }
        EventType::ButtonPressed(Button::LeftTrigger2, _) => {
            Some(GamepadInput::Press(Action::PrevCategory))
        }
        EventType::ButtonPressed(Button::RightTrigger2, _) => {
            Some(GamepadInput::Press(Action::NextCategory))
        }
        EventType::ButtonPressed(Button::Select, _) => Some(GamepadInput::Press(Action::ShowHelp)),

        // Released events for navigation buttons
        EventType::ButtonReleased(Button::DPadUp, _) => Some(GamepadInput::Release(Action::Up)),
        EventType::ButtonReleased(Button::DPadDown, _) => Some(GamepadInput::Release(Action::Down)),
        EventType::ButtonReleased(Button::DPadLeft, _) => Some(GamepadInput::Release(Action::Left)),
        EventType::ButtonReleased(Button::DPadRight, _) => {
            Some(GamepadInput::Release(Action::Right))
        }
        EventType::ButtonReleased(Button::LeftTrigger, _) => {
            Some(GamepadInput::Release(Action::PrevCategory))
        }
        EventType::ButtonReleased(Button::RightTrigger, _) => {
            Some(GamepadInput::Release(Action::NextCategory))
        }
        EventType::ButtonReleased(Button::LeftTrigger2, _) => {
            Some(GamepadInput::Release(Action::PrevCategory))
        }
        EventType::ButtonReleased(Button::RightTrigger2, _) => {
            Some(GamepadInput::Release(Action::NextCategory))
        }

        EventType::AxisChanged(gilrs::Axis::LeftStickX, value, _) => {
            let new_dir = map_axis_value(value);
            if new_dir != state.dir_x {
                let old_dir = state.dir_x;
                state.dir_x = new_dir;
                match new_dir {
                    -1 => Some(GamepadInput::Press(Action::Left)),
                    1 => Some(GamepadInput::Press(Action::Right)),
                    0 => match old_dir {
                        -1 => Some(GamepadInput::Release(Action::Left)),
                        1 => Some(GamepadInput::Release(Action::Right)),
                        _ => None,
                    },
                    _ => None,
                }
            } else {
                None
            }
        }
        EventType::AxisChanged(gilrs::Axis::LeftStickY, value, _) => {
            let new_dir = map_axis_value(value);
            if new_dir != state.dir_y {
                let old_dir = state.dir_y;
                state.dir_y = new_dir;
                match new_dir {
                    -1 => Some(GamepadInput::Press(Action::Up)),
                    1 => Some(GamepadInput::Press(Action::Down)),
                    0 => match old_dir {
                        -1 => Some(GamepadInput::Release(Action::Up)),
                        1 => Some(GamepadInput::Release(Action::Down)),
                        _ => None,
                    },
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_nav_action(action: Action) -> bool {
    matches!(
        action,
        Action::Up
            | Action::Down
            | Action::Left
            | Action::Right
            | Action::NextCategory
            | Action::PrevCategory
    )
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
