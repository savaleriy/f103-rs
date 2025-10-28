#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{AfioRemap, Level, Output, OutputType, Speed};
use embassy_stm32::time::khz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::Ch1;
use embassy_stm32::{peripherals, Peri};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

/// PWM Generation Example for STM32F103
///
/// This example generates PWM signals on PA8 (TIM1_CH1) with varying duty cycles
/// LED on PC13 blinks to show the system is running
///
/// Hardware connections:
/// - PA8: PWM output (TIM1_CH1)
/// - PC13: Status LED (built-in LED on most STM32F103 boards)

#[embassy_executor::task]
async fn blinky(led: Peri<'static, peripherals::PC13>) {
    let mut led = Output::new(led, Level::High, Speed::Low);

    loop {
        info!("LED: high");
        led.set_high();
        Timer::after_millis(500).await;

        info!("LED: low");
        led.set_low();
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("PWM Generation Example Starting!");

    // Spawn the LED blinky task
    spawner.spawn(unwrap!(blinky(p.PC13)));

    // Configure PWM pin - PA8 is TIM1_CH1 on STM32F103
    let pwm_pin: PwmPin<'_, peripherals::TIM1, Ch1, AfioRemap<0>> = 
        PwmPin::new(p.PA8, OutputType::PushPull);

    // Create PWM instance with 1kHz frequency
    let mut pwm = SimplePwm::new(
        p.TIM1,
        Some(pwm_pin), // Channel 1
        None,          // Channel 2
        None,          // Channel 3
        None,          // Channel 4
        khz(1),        // 1kHz frequency
        Default::default(),
    );

    // Get channel 1 and enable it
    let mut ch1 = pwm.ch1();
    ch1.enable();

    info!("PWM enabled on PA8 at 1kHz");

    // PWM duty cycle patterns
    let mut duty_cycle = 0u16;
    let max_duty = ch1.max_duty_cycle();
    let mut direction = 1i16; // 1 for increasing, -1 for decreasing

    loop {
        // Set the duty cycle
        ch1.set_duty_cycle(duty_cycle);

        info!(
            "PWM duty cycle: {}%",
            (duty_cycle as u32 * 100) / max_duty as u32
        );

        // Update duty cycle for next iteration
        if direction > 0 {
            duty_cycle += 50;
            if duty_cycle >= max_duty {
                duty_cycle = max_duty;
                direction = -1;
            }
        } else {
            if duty_cycle >= 50 {
                duty_cycle -= 50;
            } else {
                duty_cycle = 0;
                direction = 1;
            }
        }

        // Wait before next update
        Timer::after_millis(100).await;
    }
}
