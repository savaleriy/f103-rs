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

use embassy_stm32::adc::Adc;
use embassy_stm32::peripherals::ADC1;
use embassy_stm32::{adc, bind_interrupts};

use {defmt_rtt as _, panic_probe as _};

// PWM Generation and Calibration Example for STM32F103
//
// - PA8: PWM output (TIM1_CH1) to the cooler
// - PC13: Status LED
// - PA4: ADC input reading a tach/voltage proportional to RPM
//
// Flow:
// 1) Stop fan and measure ADC (zero RPM baseline)
// 2) Run fan at 100% and measure ADC (max RPM baseline)
// 3) Sweep several duty percentages and estimate RPM via linear interpolation

bind_interrupts!(struct Irqs {
    ADC1_2 => adc::InterruptHandler<ADC1>;
});

#[embassy_executor::task]
async fn blinky(led: Peri<'static, peripherals::PC13>) {
    let mut led = Output::new(led, Level::High, Speed::Low);
    loop {
        led.set_high();
        Timer::after_millis(300).await;
        led.set_low();
        Timer::after_millis(300).await;
    }
}

#[derive(Debug, Clone, Copy)]
struct CoolerCalibration {
    min_duty_percent: u16,   // Minimum duty that reliably starts the fan
    max_duty_percent: u16,   // Typically 100%
    min_rpm: u16,            // Estimated RPM at min duty (tune after tests)
    max_rpm: u16,            // Datasheet value, e.g. 3400 RPM
    adc_zero_rpm: u16,       // Measured ADC when stopped
    adc_max_rpm: u16,        // Measured ADC at max speed
}

impl Default for CoolerCalibration {
    fn default() -> Self {
        Self {
            min_duty_percent: 20,
            max_duty_percent: 100,
            min_rpm: 800,
            max_rpm: 3400,
            adc_zero_rpm: 0,
            adc_max_rpm: 4095,
        }
    }
}

impl CoolerCalibration {
    fn rpm_from_adc(&self, adc: u16) -> u16 {
        if adc <= self.adc_zero_rpm {
            return 0;
        }
        if adc >= self.adc_max_rpm {
            return self.max_rpm;
        }
        let adc_range = (self.adc_max_rpm - self.adc_zero_rpm) as u32;
        if adc_range == 0 {
            return 0;
        }
        let rpm_range = (self.max_rpm - self.min_rpm) as u32;
        let adc_offset = (adc - self.adc_zero_rpm) as u32;
        (self.min_rpm as u32 + adc_offset * rpm_range / adc_range) as u16
    }
}

fn percent_to_duty(max_duty: u16, percent: u16) -> u16 {
    ((max_duty as u32 * percent as u32) / 100) as u16
}

async fn measure_adc_avg<const N: usize>(adc: &mut Adc<'static, peripherals::ADC1>, pin: &mut Peri<'static, peripherals::PA4>) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i < N {
        sum += adc.read(pin).await as u32;
        i += 1;
        Timer::after_micros(50).await;
    }
    (sum / N as u32) as u16
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Cooler Calibration Example Starting");

    // Status LED
    spawner.spawn(unwrap!(blinky(p.PC13)));

    // PWM on PA8 (TIM1_CH1)
    let pwm_pin: PwmPin<'_, peripherals::TIM1, Ch1, AfioRemap<0>> = PwmPin::new(p.PA8, OutputType::PushPull);
    let mut pwm = SimplePwm::new(
        p.TIM1,
        Some(pwm_pin),
        None,
        None,
        None,
        khz(1),
        Default::default(),
    );
    let mut ch1 = pwm.ch1();
    ch1.enable();
    let max_duty = ch1.max_duty_cycle();

    // ADC on PA4
    let mut adc = Adc::new(p.ADC1);
    let mut adc_pin = p.PA4;

    let mut calib = CoolerCalibration::default();

    // Step 1: Measure ADC at 0% (fan stopped)
    ch1.set_duty_cycle(0);
    info!("Waiting for fan to stop...");
    Timer::after_millis(2000).await;
    let adc_zero = measure_adc_avg::<64>(&mut adc, &mut adc_pin).await;
    info!("ADC (stopped) = {}", adc_zero);

    // Step 2: Measure ADC at 100% (max speed)
    ch1.set_duty_cycle(percent_to_duty(max_duty, 100));
    info!("Driving fan at 100% for max speed measurement...");
    Timer::after_millis(3000).await;
    let adc_max = measure_adc_avg::<64>(&mut adc, &mut adc_pin).await;
    info!("ADC (max) = {}", adc_max);

    // Save calibration
    calib.adc_zero_rpm = adc_zero;
    calib.adc_max_rpm = adc_max;
    info!("Calibration captured: zero={}, max={}", calib.adc_zero_rpm, calib.adc_max_rpm);

    // Step 3: Sweep duty and log estimated RPM
    let sweep = [20u16, 40, 60, 80, 100];
    for &pct in &sweep {
        let duty = percent_to_duty(max_duty, pct);
        ch1.set_duty_cycle(duty);
        info!("Set duty to {}%", pct);
        Timer::after_millis(2000).await; // settle

        let adc_avg = measure_adc_avg::<64>(&mut adc, &mut adc_pin).await;
        let rpm_est = calib.rpm_from_adc(adc_avg);
        info!("Duty: {}%, ADC: {}, Estimated RPM: {}", pct, adc_avg, rpm_est);
    }

    info!("Calibration sweep complete. Holding at 50%...");
    ch1.set_duty_cycle(percent_to_duty(max_duty, 50));

    // Idle loop to keep example running and show live estimate
    loop {
        let adc_now = measure_adc_avg::<32>(&mut adc, &mut adc_pin).await;
        let rpm_now = calib.rpm_from_adc(adc_now);
        info!("Hold 50% - ADC: {}, RPM: {}", adc_now, rpm_now);
        Timer::after_millis(1000).await;
    }
}