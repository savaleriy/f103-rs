use defmt::*;
use embassy_executor::task;
use embassy_stm32::peripherals;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_time::Timer;

use crate::shared::SHARED_DUTY;

#[task]
pub async fn change_duty_cycle(mut pwm: SimplePwm<'static, peripherals::TIM1>) {
    let mut ch1 = pwm.ch1();
    ch1.enable();

    loop {
        let duty_cycle = SHARED_DUTY.wait().await;
        ch1.set_duty_cycle(duty_cycle);
        info!("PWM duty cycle {}", duty_cycle);
        Timer::after_millis(100).await;
    }
}
