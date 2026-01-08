use crate::input::Action;
use gilrs::{Button, Event, EventType, Gilrs};
use iced::futures::sink::SinkExt;
use iced::Subscription;

pub fn gamepad_subscription() -> Subscription<Action> {
    Subscription::run(|| {
        iced::stream::channel(
            100,
            |mut output: iced::futures::channel::mpsc::Sender<Action>| async move {
                let mut gilrs = match Gilrs::new() {
                    Ok(g) => g,
                    Err(e) => {
                        tracing::error!("Failed to initialize Gilrs: {}", e);
                        return;
                    }
                };

                let mut axis_dir_x: i8 = 0;
                let mut axis_dir_y: i8 = 0;
                let deadzone = 0.6_f32;

                loop {
                    while let Some(Event { event, .. }) = gilrs.next_event() {
                        let action = match event {
                            EventType::ButtonPressed(Button::South, _) => Some(Action::Select),
                            EventType::ButtonPressed(Button::East, _) => Some(Action::Back),
                            EventType::ButtonPressed(Button::West, _) => Some(Action::ContextMenu),
                            EventType::ButtonPressed(Button::DPadUp, _) => Some(Action::Up),
                            EventType::ButtonPressed(Button::DPadDown, _) => Some(Action::Down),
                            EventType::ButtonPressed(Button::DPadLeft, _) => Some(Action::Left),
                            EventType::ButtonPressed(Button::DPadRight, _) => Some(Action::Right),
                            EventType::ButtonPressed(Button::LeftTrigger, _) => {
                                Some(Action::PrevCategory)
                            }
                            EventType::ButtonPressed(Button::RightTrigger, _) => {
                                Some(Action::NextCategory)
                            }
                            EventType::ButtonPressed(Button::LeftTrigger2, _) => {
                                Some(Action::PrevCategory)
                            }
                            EventType::ButtonPressed(Button::RightTrigger2, _) => {
                                Some(Action::NextCategory)
                            }
                            EventType::AxisChanged(gilrs::Axis::LeftStickX, value, _) => {
                                let new_dir = if value <= -deadzone {
                                    -1
                                } else if value >= deadzone {
                                    1
                                } else {
                                    0
                                };
                                if new_dir != axis_dir_x {
                                    axis_dir_x = new_dir;
                                    match axis_dir_x {
                                        -1 => Some(Action::Left),
                                        1 => Some(Action::Right),
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            }
                            EventType::AxisChanged(gilrs::Axis::LeftStickY, value, _) => {
                                let new_dir = if value <= -deadzone {
                                    1
                                } else if value >= deadzone {
                                    -1
                                } else {
                                    0
                                };
                                if new_dir != axis_dir_y {
                                    axis_dir_y = new_dir;
                                    match axis_dir_y {
                                        -1 => Some(Action::Up),
                                        1 => Some(Action::Down),
                                        _ => None,
                                    }
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        };

                        if let Some(act) = action {
                            let _ = output.send(act).await;
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            },
        )
    })
}
