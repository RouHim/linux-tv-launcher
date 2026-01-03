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

                loop {
                    while let Some(Event { event, .. }) = gilrs.next_event() {
                        let action = match event {
                            EventType::ButtonPressed(Button::South, _) => Some(Action::Select),
                            EventType::ButtonPressed(Button::East, _) => Some(Action::Back),
                            EventType::ButtonPressed(Button::DPadUp, _) => Some(Action::Up),
                            EventType::ButtonPressed(Button::DPadDown, _) => Some(Action::Down),
                            EventType::ButtonPressed(Button::DPadLeft, _) => Some(Action::Left),
                            EventType::ButtonPressed(Button::DPadRight, _) => Some(Action::Right),
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
