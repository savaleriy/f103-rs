#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::Adc;
use embassy_stm32::peripherals::ADC1;
use embassy_stm32::{adc, bind_interrupts};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC1_2 => adc::InterruptHandler<ADC1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("ADC Voltage Measurement Started!");

    let mut adc = Adc::new(p.ADC1);
    let mut pin = p.PA4;

    // Enable and read VREFINT for accurate voltage reference
    let mut vrefint = adc.enable_vref();
    
    // Take multiple VREFINT readings for stability
    let mut vrefint_sum: u32 = 0;
    for _ in 0..10 {
        let sample = adc.read(&mut vrefint).await;
        vrefint_sum += sample as u32;
        Timer::after_micros(10).await;
    }
    let vrefint_sample = (vrefint_sum / 10) as u16;
    info!("VREFINT calibration sample: {}", vrefint_sample);

    let convert_to_millivolts = |sample: u16| -> u32 {
        // From STM32 datasheet - typical VREFINT value is 1.2V
        const VREFINT_MV: u32 = 1200; // mV
        
        // Convert ADC reading to millivolts using VREFINT as reference
        (u32::from(sample) * VREFINT_MV / u32::from(vrefint_sample))
    };

    let mut moving_avg_buffer = [0u16; 8];
    let mut moving_avg_index = 0;
    let mut moving_avg_initialized = false;

    let num_samples = 300;
    // Filter state variable    // Main measurement loop
    loop {
        // Take multiple samples for filtering
        let mut samples = [0u16; 300];
        
        // Read raw samples
        for i in 0..num_samples {
            samples[i] = adc.read(&mut pin).await;
            Timer::after_micros(1).await;
        }

        // 1. Simple Moving Average Filter
        let mut sum: u32 = 0;
        for &sample in &samples {
            sum += sample as u32;
        }
        let moving_avg = (sum / num_samples as u32) as u16;

        // 2. Running Moving Average Filter (more efficient)
        moving_avg_buffer[moving_avg_index] = samples[num_samples - 1]; // Use last sample
        moving_avg_index = (moving_avg_index + 1) % moving_avg_buffer.len();
        
        let mut running_avg_sum: u32 = 0;
        let buffer_samples = if moving_avg_initialized {
            moving_avg_buffer.len()
        } else {
            moving_avg_index + 1
        };

        let recent_samples = [samples[num_samples - 3], samples[num_samples - 2], samples[num_samples - 1]];
        let median_value = median_of_three(recent_samples[0], recent_samples[1], recent_samples[2]);

        // Convert all filtered values to millivolts
        let moving_avg_mv = convert_to_millivolts(moving_avg);
        let median_mv = convert_to_millivolts(median_value);

        info!("Simple Avg: {} mV, Median: {} mV", 
              moving_avg_mv,  median_mv);
        
        Timer::after_millis(500).await;
    }
}

/// Helper function for median filter
fn median_of_three(a: u16, b: u16, c: u16) -> u16 {
    if a <= b {
        if b <= c {
            b
        } else if a <= c {
            c
        } else {
            a
        }
    } else {
        if a <= c {
            a
        } else if b <= c {
            c
        } else {
            b
        }
    }
}
