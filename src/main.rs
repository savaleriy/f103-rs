#![no_std]
#![no_main]

/// basic
use defmt::*;
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

/// GPIO
use embassy_stm32::gpio::{AfioRemap, Level, Output, OutputType, Speed};
use embassy_stm32::time::khz;

/// Timer
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::Ch1;
use embassy_stm32::{peripherals, Peri};
use embassy_time::Timer;

/// async premitives
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

/// Adc
use embassy_stm32::adc::Adc;
use embassy_stm32::peripherals::ADC1;
use embassy_stm32::{adc, bind_interrupts};

bind_interrupts!(struct Irqs {
    ADC1_2 => adc::InterruptHandler<ADC1>;
});

/// For Heapless Vectors
use embedded_alloc::TlsfHeap as Heap;
#[global_allocator]
static HEAP: Heap = Heap::empty();
extern crate alloc;
use heapless::Vec;

static SHARED_DUTY: Signal<ThreadModeRawMutex, u16> = Signal::new();
static SHARED_ADC_VALUE: Signal<ThreadModeRawMutex, u32> = Signal::new();
static SHARED_MESSAGE: Signal<ThreadModeRawMutex, u32> = Signal::new();

/// Blinking routine
/// Change delay via channel

static DELAY_CHANNEL: Channel<ThreadModeRawMutex, u64, 4> = Channel::new();

#[embassy_executor::task]
async fn blinky(led: Peri<'static, peripherals::PC13>, initial_delay: u64) {
    let mut led = Output::new(led, Level::High, Speed::Low);
    let mut current_delay = initial_delay;

    loop {
        // Check for new delay without blocking
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

/// Cooling routine
/// TODO
#[embassy_executor::task]
async fn change_duty_cycle(mut pwm: SimplePwm<'static, peripherals::TIM1>) {
    let mut ch1 = pwm.ch1();
    ch1.enable();

    loop {
        let duty_cycle = SHARED_DUTY.wait().await;
        ch1.set_duty_cycle(duty_cycle);
        info!("PWM duty cycle {}", duty_cycle);
        Timer::after_millis(100).await;
    }
}

/// Power routine
/// We be able to change what power supply are we using

#[derive(Debug, Clone, Copy, PartialEq)]
enum PowerState {
    DCDC,
    ACDC,
    OFF,
}

impl PowerState {
    const ACDC_THRESHOLD: u32 = 760;

    fn from_voltage(voltage: u32) -> Self {
        if voltage > Self::ACDC_THRESHOLD {
            PowerState::ACDC
        } else {
            PowerState::DCDC
        }
    }

    fn determine_state(message: u32, voltage: u32) -> Self {
        match message {
            1 => PowerState::DCDC,
            2 => PowerState::ACDC,
            3 => PowerState::OFF,
            _ => Self::from_voltage(voltage),
        }
    }

    fn get_led_delay(&self) -> u64 {
        match self {
            PowerState::ACDC => 500, // every 500 ms
            PowerState::DCDC => 100, // every 100 ms
            PowerState::OFF => 1000, // every 1 sec
        }
    }

    fn set_pins(&self, acdc_pin: &mut Output<'_>, dcdc_pin: &mut Output<'_>) {
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

#[embassy_executor::task]
async fn change_power_source(
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

        // Only update if state changed
        if previous_state != Some(state) {
            state.set_pins(&mut acdc_output, &mut dcdc_output);

            // Send new LED delay based on state
            let led_delay = state.get_led_delay();
            if DELAY_CHANNEL.try_send(led_delay).is_err() {
                warn!("Failed to send LED delay update");
            }

            previous_state = Some(state);
        }

        Timer::after_millis(delay as u64).await;
    }
}

/// ADC Routine
/// Measure voltage and send it to shared variable

const NUM_SAMPLES: usize = 300;

#[embassy_executor::task]
async fn measure_voltage(
    mut adc: Adc<'static, peripherals::ADC1>,
    mut pin: Peri<'static, peripherals::PA4>,
) {
    // Calibrate VREFINT
    let vrefint_sample = calibrate_vrefint(&mut adc).await;
    info!("VREFINT calibration sample: {}", vrefint_sample);

    let to_ml = create_voltage_converter(vrefint_sample);

    loop {
        let mut samples = Vec::<u16, NUM_SAMPLES>::new();
        for _ in 0..NUM_SAMPLES {
            let _ = samples.push(adc.read(&mut pin).await);
            Timer::after_micros(1).await;
        }

        // Calculate average using iterator methods
        let avg = samples.iter().map(|&sample| sample as u32).sum::<u32>() / NUM_SAMPLES as u32;

        let avg_mv = to_ml(avg as u16);

        // info!("MicroVoltage {}", avg_mv);
        SHARED_ADC_VALUE.signal(avg_mv);
    }
}

async fn calibrate_vrefint(adc: &mut Adc<'static, peripherals::ADC1>) -> u16 {
    let mut vrefint = adc.enable_vref();

    // Create a vector to store the samples
    let mut samples = Vec::<u16, NUM_SAMPLES>::new();

    // Collect samples sequentially (not concurrently)
    for _ in 0..NUM_SAMPLES {
        let _ = samples.push(adc.read(&mut vrefint).await);
        Timer::after_micros(1).await;
    }

    // Calculate average
    let sum: u32 = samples.iter().map(|&sample| sample as u32).sum();

    (sum / NUM_SAMPLES as u32) as u16
}

fn create_voltage_converter(vrefint_sample: u16) -> impl Fn(u16) -> u32 {
    // Based on datasheet it is typicly 1200 mV
    const VREFINT_MV: u32 = 1200; // mV

    move |sample: u16| -> u32 { u32::from(sample) * VREFINT_MV / u32::from(vrefint_sample) }
}

/// Entry point  
///

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("PWM Generation Example Starting!");

    // Spawn the LED blinky task
    spawner.spawn(unwrap!(blinky(p.PC13, 10)));

    // Configure PWM pin - PA8 is TIM1_CH1 on STM32F103
    let pwm_pin: PwmPin<'_, peripherals::TIM1, Ch1, AfioRemap<0>> =
        PwmPin::new(p.PA8, OutputType::PushPull);

    // Create PWM instance with 1kHz frequency
    let pwm = SimplePwm::new(
        p.TIM1,
        Some(pwm_pin), // Channel 1
        None,          // Channel 2
        None,          // Channel 3
        None,          // Channel 4
        khz(1),        // 1kHz frequency
        Default::default(),
    );

    spawner.spawn(change_duty_cycle(pwm).unwrap());
    // setup
    SHARED_DUTY.signal(50);

    let adc = Adc::new(p.ADC1);
    let pin = p.PA4;

    spawner.spawn(measure_voltage(adc, pin).unwrap());

    spawner.spawn(change_power_source(p.PB0, p.PB1, 100).unwrap());

    loop {
        // This is simple test
        // In future we get commands from other MCU by uart not it is just for test
        for i in 0..4 {
            SHARED_MESSAGE.signal(i);
            info!("Send state {}", i);
            Timer::after_millis(1000).await;
        }
    }
}
