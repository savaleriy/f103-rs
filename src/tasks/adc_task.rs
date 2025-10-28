use defmt::*;
use embassy_executor::task;
use embassy_stm32::adc::Adc;
use embassy_stm32::{peripherals, Peri};
use embassy_time::Timer;

use heapless::Vec;

use crate::shared::SHARED_ADC_VALUE;

const NUM_SAMPLES: usize = 300;

#[task]
pub async fn measure_voltage(
    mut adc: Adc<'static, peripherals::ADC1>,
    mut pin: Peri<'static, peripherals::PA4>,
) {
    let vrefint_sample = calibrate_vrefint(&mut adc).await;
    info!("VREFINT calibration sample: {}", vrefint_sample);

    let to_ml = create_voltage_converter(vrefint_sample);

    loop {
        let mut samples = Vec::<u16, NUM_SAMPLES>::new();
        for _ in 0..NUM_SAMPLES {
            let _ = samples.push(adc.read(&mut pin).await);
            Timer::after_micros(1).await;
        }

        let avg = samples.iter().map(|&sample| sample as u32).sum::<u32>() / NUM_SAMPLES as u32;
        let avg_mv = to_ml(avg as u16);
        SHARED_ADC_VALUE.signal(avg_mv);
    }
}

async fn calibrate_vrefint(adc: &mut Adc<'static, peripherals::ADC1>) -> u16 {
    let mut vrefint = adc.enable_vref();
    let mut samples = Vec::<u16, NUM_SAMPLES>::new();
    for _ in 0..NUM_SAMPLES {
        let _ = samples.push(adc.read(&mut vrefint).await);
        Timer::after_micros(1).await;
    }
    let sum: u32 = samples.iter().map(|&sample| sample as u32).sum();
    (sum / NUM_SAMPLES as u32) as u16
}

fn create_voltage_converter(vrefint_sample: u16) -> impl Fn(u16) -> u32 {
    const VREFINT_MV: u32 = 1200;
    move |sample: u16| -> u32 { u32::from(sample) * VREFINT_MV / u32::from(vrefint_sample) }
}


