use defmt::*;
use embassy_executor::task;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::{peripherals, Peri};
use embassy_time::Timer;

use crate::shared::{DELAY_CHANNEL, SHARED_ADC_VALUE, SHARED_MESSAGE};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerState {
    DCDC,
    ACDC,
    OFF,
}

impl PowerState {
    pub const ACDC_THRESHOLD: u32 = 760;

    fn from_voltage(voltage: u32) -> Self {
        if voltage > Self::ACDC_THRESHOLD {
            PowerState::ACDC
        } else {
            PowerState::DCDC
        }
    }

    pub fn determine_state(message: u32, voltage: u32) -> Self {
        match message {
            1 => PowerState::DCDC,
            2 => PowerState::ACDC,
            3 => PowerState::OFF,
            _ => Self::from_voltage(voltage),
        }
    }

    pub fn get_led_delay(&self) -> u64 {
        match self {
            PowerState::ACDC => 500,
            PowerState::DCDC => 100,
            PowerState::OFF => 1000,
        }
    }

    pub fn set_pins(&self, acdc_pin: &mut Output<'_>, dcdc_pin: &mut Output<'_>) {
        match self {
            PowerState::ACDC => {
                info!("ACDC");
                acdc_pin.set_low();
                dcdc_pin.set_high();
            }
            PowerState::DCDC => {
                info!("DCDC");
                acdc_pin.set_high();
                dcdc_pin.set_low();
            }
            PowerState::OFF => {
                info!("OFF");
                acdc_pin.set_low();
                dcdc_pin.set_low();
            }
        }
    }
}

#[task]
pub async fn change_power_source(
    acdc_pin: Peri<'static, peripherals::PB0>,
    dcdc_pin: Peri<'static, peripherals::PB1>,
    delay: i32,
) {
    let mut acdc_output = Output::new(acdc_pin, Level::Low, Speed::Low);
    let mut dcdc_output = Output::new(dcdc_pin, Level::Low, Speed::Low);
    let mut previous_state: Option<PowerState> = None;

    loop {
        let voltage = SHARED_ADC_VALUE.wait().await;
        let message = SHARED_MESSAGE.wait().await;
        info!("Get voltage {}", voltage);

        let state = PowerState::determine_state(message, voltage);

        if previous_state != Some(state) {
            state.set_pins(&mut acdc_output, &mut dcdc_output);

            let led_delay = state.get_led_delay();
            let _ = DELAY_CHANNEL.try_send(led_delay);

            previous_state = Some(state);
        }

        Timer::after_millis(delay as u64).await;
    }
}


