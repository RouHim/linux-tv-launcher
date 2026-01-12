use gilrs::{Event, Gilrs, PowerInfo};
use iced::futures::stream;
use iced::Subscription;

#[derive(Debug, Clone, PartialEq)]
pub struct GamepadBattery {
    pub power_info: PowerInfo,
}

pub fn gamepad_battery_subscription() -> Subscription<Vec<GamepadBattery>> {
    struct State {
        gilrs: Option<Gilrs>,
    }

    Subscription::run(|| {
        stream::unfold(
            State {
                gilrs: Gilrs::new().ok(),
            },
            |mut state| async move {
                let mut batteries = Vec::new();

                if let Some(gilrs) = &mut state.gilrs {
                    // Process all pending events to update internal state of gamepads
                    // We need to drain events so that the internal state (including battery) is updated.
                    // We ignore the actual events as they are handled by the main input subscription.
                    loop {
                        // Explicitly type annotation to help compiler
                        let event: Option<Event> = gilrs.next_event();
                        if event.is_none() {
                            break;
                        }
                    }

                    // Collect battery info
                    for (_id, gamepad) in gilrs.gamepads() {
                        batteries.push(GamepadBattery {
                            power_info: gamepad.power_info(),
                        });
                    }
                } else {
                    // Try to initialize if it failed previously (e.g. no permissions initially)
                    state.gilrs = Gilrs::new().ok();
                }

                // Poll every minute
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;

                Some((batteries, state))
            },
        )
    })
}
