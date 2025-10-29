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
use embassy_stm32::usart::{BufferedUart, Config};
use embassy_stm32::peripherals::{ADC1, USART1};
use embassy_stm32::{adc, usart, bind_interrupts};


use embedded_alloc::TlsfHeap as Heap;
#[global_allocator]
static HEAP: Heap = Heap::empty();
// Backing memory for the global allocator (TLSF)
// Adjust size as needed depending on expected dynamic allocations
#[link_section = ".uninit"]
static mut HEAP_MEM: [u8; 8 * 1024] = [0; 8 * 1024];


extern crate alloc;

mod device;
mod shared;
mod tasks {
    pub mod adc_task;
    pub mod blinky;
    pub mod cooling;
    pub mod led;
    pub mod power;
    pub mod pwm;
    pub mod rx_tx;
}

use shared::{SHARED_DUTY, SHARED_MESSAGE, TX_MESSAGE_CHANNEL};
use tasks::{
    adc_task::measure_voltage, blinky::blinky, cooling::cooling_controller, led::led_controller,
    power::change_power_source, pwm::change_duty_cycle, rx_tx::{rx_task, tx_task}
};

bind_interrupts!(struct Irqs {
    ADC1_2 => adc::InterruptHandler<ADC1>;
    USART1 => usart::BufferedInterruptHandler<USART1>;
});


// Static buffers for UART (fixed lifetime issues)
static mut TX_BUF: [u8; 256] = [0; 256];
static mut RX_BUF: [u8; 256] = [0; 256];

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    unsafe {
        HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_MEM.len());
    }

    let p = embassy_stm32::init(Default::default());

    let mut config = Config::default();
    config.baudrate = 9600;

    // Initialize buffered UART with static buffers
    let usart = unsafe {
        BufferedUart::new(
            p.USART1,
            p.PA10,      // RX
            p.PA9,       // TX
            &mut TX_BUF, // TX Buffer (static)
            &mut RX_BUF, // RX Buffer (static)
            Irqs,        // Interrupt Handler
            config,
        )
        .unwrap()
    };

    let (tx, rx) = usart.split();

    // Get senders/receivers
    let tx_sender = TX_MESSAGE_CHANNEL.sender();
    let tx_receiver = TX_MESSAGE_CHANNEL.receiver();


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

    // Blink Task
    spawner.spawn((blinky(p.PC13, 10)).unwrap());
    // PWM task
    spawner.spawn(change_duty_cycle(pwm).unwrap());
    SHARED_DUTY.signal(50);
    // ADC Task
    let adc = Adc::new(p.ADC1);
    let pin = p.PA4;
    spawner.spawn(measure_voltage(adc, pin).unwrap());
    // Power Task
    spawner.spawn(change_power_source(p.PB0, p.PB1, 100).unwrap());
    // LED controller task (using PA5)
    spawner.spawn(led_controller(p.PA5).unwrap());
    // Cooling controller task (using PB2)
    spawner.spawn(cooling_controller(p.PB2).unwrap());
    // USART Task
    spawner.spawn(rx_task(rx, tx_sender).unwrap());
    spawner.spawn(tx_task(tx, tx_receiver).unwrap());
    
    loop {
        // Simple test
        // for i in 0..4 {
        //     SHARED_MESSAGE.signal(i);
            Timer::after_millis(1000).await;
        // }
    }
}
