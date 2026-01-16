use crate::input::Action;
use gilrs::{Button, Event, EventType, Gilrs, PowerInfo};
use iced::futures::sink::SinkExt;
use iced::Subscription;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const BATTERY_CHECK_INTERVAL: Duration = Duration::from_secs(5);
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
                                let is_keyboard = name.to_lowercase().contains("keyboard");
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
