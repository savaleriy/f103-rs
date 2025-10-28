use defmt::*;
use embassy_executor::task;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::{peripherals, Peri};
use embassy_time::Timer;

use crate::shared::DELAY_CHANNEL;

#[task]
pub async fn blinky(led: Peri<'static, peripherals::PC13>, initial_delay: u64) {
    let mut led = Output::new(led, Level::High, Speed::Low);
    let mut current_delay = initial_delay;

    loop {
        if let Ok(new_delay) = DELAY_CHANNEL.try_receive() {
            current_delay = new_delay;
            info!("LED delay updated to {} ms", current_delay);
        }

        led.set_high();
        Timer::after_millis(current_delay).await;
        led.set_low();
        Timer::after_millis(current_delay).await;
    }
}


