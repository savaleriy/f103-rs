#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{AfioRemap, OutputType};
use embassy_stm32::peripherals;
use embassy_stm32::time::khz;
use embassy_stm32::timer::simple_pwm::PwmPin;
use embassy_stm32::timer::Ch1;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

use embassy_stm32::adc::Adc;
use embassy_stm32::peripherals::ADC1;
use embassy_stm32::{adc, bind_interrupts};

mod shared;
mod tasks {
    pub mod adc_task;
    pub mod blinky;
    pub mod power;
    pub mod pwm;
}

use shared::{SHARED_DUTY, SHARED_MESSAGE};
use tasks::{
    adc_task::measure_voltage, blinky::blinky, power::change_power_source, pwm::change_duty_cycle,
};

bind_interrupts!(struct Irqs {
    ADC1_2 => adc::InterruptHandler<ADC1>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    spawner.spawn(unwrap!(blinky(p.PC13, 10)));

    let pwm_pin: PwmPin<'_, peripherals::TIM1, Ch1, AfioRemap<0>> =
        PwmPin::new(p.PA8, OutputType::PushPull);
    let pwm = embassy_stm32::timer::simple_pwm::SimplePwm::new(
        p.TIM1,
        Some(pwm_pin),
        None,
        None,
        None,
        khz(1),
        Default::default(),
    );
    spawner.spawn(change_duty_cycle(pwm).unwrap());
    SHARED_DUTY.signal(50);

    let adc = Adc::new(p.ADC1);
    let pin = p.PA4;
    spawner.spawn(measure_voltage(adc, pin).unwrap());

    spawner.spawn(change_power_source(p.PB0, p.PB1, 100).unwrap());

    loop {
        // Simple test
        for i in 0..4 {
            SHARED_MESSAGE.signal(i);
            Timer::after_millis(1000).await;
        }
    }
}
