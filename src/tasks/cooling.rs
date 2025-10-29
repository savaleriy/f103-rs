use defmt::*;
use embassy_executor::task;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::{peripherals, Peri};
use embassy_time::Timer;

use crate::shared::{CoolingState, COOLING_CHANNEL, COOLING_STATUS, CURRENT_SPEED, SPEED_CHANNEL};

#[task]
pub async fn cooling_controller(cooling_pin: Peri<'static, peripherals::PB2>) {
    let mut cooling_output = Output::new(cooling_pin, Level::Low, Speed::Low);
    let mut current_state = CoolingState::Off;
    let mut current_speed = 0u16;

    loop {
        // Check for cooling state commands
        if let Ok(command) = COOLING_CHANNEL.try_receive() {
            match command {
                CoolingState::On => {
                    cooling_output.set_high();
                    current_state = CoolingState::On;
                    info!("Cooling turned ON");
                }
                CoolingState::Off => {
                    cooling_output.set_low();
                    current_state = CoolingState::Off;
                    current_speed = 0;
                    info!("Cooling turned OFF, speed reset to {}", current_speed);
                }
            }
            COOLING_STATUS.signal(current_state);
        }

        // Check for speed commands
        if let Ok(speed) = SPEED_CHANNEL.try_receive() {
            current_speed = speed;
            if current_state == CoolingState::On {
                info!("Cooling speed set to {}", current_speed);
                // In a real implementation, you might control PWM duty cycle here
                // For now, we just store the speed value
            }
            CURRENT_SPEED.signal(current_speed);
        }

        Timer::after_millis(10).await;
    }
}
