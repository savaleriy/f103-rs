use defmt::*;
use embassy_executor::task;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::{peripherals, Peri};
use embassy_time::Timer;

use crate::shared::{LedState, LED_CHANNEL, LED_STATUS};

#[task]
pub async fn led_controller(led: Peri<'static, peripherals::PA5>) {
    let mut led_output = Output::new(led, Level::Low, Speed::Low);
    let mut current_state = false; // LED is off initially

    loop {
        if let Ok(command) = LED_CHANNEL.try_receive() {
            match command {
                LedState::On => {
                    led_output.set_high(); // Turn LED on
                    current_state = true;
                    info!("LED turned ON");
                }
                LedState::Off => {
                    led_output.set_low(); // Turn LED off
                    current_state = false;
                    info!("LED turned OFF");
                }
                LedState::Toggle => {
                    if current_state {
                        led_output.set_low();
                        current_state = false;
                        info!("LED toggled OFF");
                    } else {
                        led_output.set_high();
                        current_state = true;
                        info!("LED toggled ON");
                    }
                }
            }
            LED_STATUS.signal(current_state);
        }

        Timer::after_millis(10).await;
    }
}
