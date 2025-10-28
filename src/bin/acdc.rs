#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::adc::Adc;
use embassy_stm32::peripherals::ADC1;
use embassy_stm32::usart::{BufferedUart, Config};
use embassy_stm32::{adc, bind_interrupts, peripherals, usart};
use embassy_time::{Duration, Timer};
use embedded_io_async::{Read, Write};
use heapless::String;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC1_2 => adc::InterruptHandler<ADC1>;
});

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};

use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USART1 => usart::BufferedInterruptHandler<peripherals::USART1>;
});

// Static buffers for UART (fixed lifetime issues)
static mut TX_BUF: [u8; 256] = [0; 256];
static mut RX_BUF: [u8; 256] = [0; 256];

// Channel to send messages from RX to TX
static TX_MESSAGE_CHANNEL: Channel<ThreadModeRawMutex, heapless::String<64>, 4> = Channel::new();

// Channel for LED control
static LED_CHANNEL: Channel<ThreadModeRawMutex, LedState, 4> = Channel::new();

#[derive(Debug, Clone, Copy)]
enum LedState {
    Toggle,
    On,
    Off,
}

#[embassy_executor::task]
async fn toggle_led(mut led: Output<'static>) {
    loop {
        match LED_CHANNEL.receive().await {
            LedState::Toggle => {
                led.toggle();
                info!("Toggle LED");
            }
            LedState::On => {
                led.set_high();
                info!("LED On");
            }
            LedState::Off => {
                led.set_low();
                info!("LED Off");
            }
        }
    }
}

#[embassy_executor::task]
async fn rx_task(
    mut rx: embassy_stm32::usart::BufferedUartRx<'static>,
    tx_sender: Sender<'static, ThreadModeRawMutex, heapless::String<64>, 4>,
    led_sender: Sender<'static, ThreadModeRawMutex, LedState, 4>,
) {
    let mut buf = [0u8; 64];
    let mut pos = 0;

    info!("RX task started");

    loop {
        let mut byte = [0u8; 1];
        if let Ok(()) = rx.read_exact(&mut byte).await {
            let b = byte[0];

            if pos >= buf.len() {
                info!("RX buffer overflow, resetting");
                pos = 0;
                continue;
            }

            buf[pos] = b;
            pos += 1;

            if b == b'\n' || b == b'\r' {
                if pos > 1 {
                    let message_len = if buf[pos - 1] == b'\n' || buf[pos - 1] == b'\r' {
                        pos - 1
                    } else {
                        pos
                    };

                    if message_len > 0 {
                        let mut received_string = String::<64>::new();

                        for &byte in &buf[..message_len] {
                            if received_string.push(byte as char).is_err() {
                                warn!("String conversion overflow");
                                break;
                            }
                        }

                        info!("Received");

                        let mut echo_string = String::<64>::new();
                        echo_string.push_str(&received_string).unwrap();
                        echo_string.push_str("\r\n").unwrap();

                        tx_sender.send(echo_string).await;

                        let command = received_string;

                        if command == "led on" {
                            led_sender.send(LedState::On).await;
                        } else if command == "led off" {
                            led_sender.send(LedState::Off).await;
                        } else if command == "led toggle" || command == "toggle" {
                            led_sender.send(LedState::Toggle).await;
                        }
                    }
                }
                pos = 0; // reset buffer
            }
        } else {
            // Handle read error
            Timer::after(Duration::from_millis(100)).await;
        }
    }
}

#[embassy_executor::task]
async fn tx_task(
    mut tx: embassy_stm32::usart::BufferedUartTx<'static>,
    rx: Receiver<'static, ThreadModeRawMutex, heapless::String<64>, 4>,
) {
    info!("TX task started");
    loop {
        let msg = rx.receive().await;

        // Convert heapless string to bytes and send
        if let Err(e) = tx.write_all(msg.as_bytes()).await {
            error!("TX write error: {:?}", e);
        }

        if let Err(e) = tx.flush().await {
            error!("TX flush error: {:?}", e);
        }
        info!("Sended");
        Timer::after(Duration::from_millis(100)).await;
    }
}


#![no_std]
#![no_main]

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Hello World!");


}


#[embassy_executor::main]
async fn main(_spawner: Spawner) {
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

    let (mut tx, mut rx) = usart.split();

    let led = Output::new(p.PC13, Level::High, Speed::Low);

    // Get senders/receivers
    let tx_sender = TX_MESSAGE_CHANNEL.sender();
    let tx_receiver = TX_MESSAGE_CHANNEL.receiver();
    let led_sender = LED_CHANNEL.sender();

    _spawner.spawn(rx_task(rx, tx_sender, led_sender).unwrap());
    // let _ = tx.write_all(b"led toggle\r\n").await;
    _spawner.spawn(tx_task(tx, tx_receiver).unwrap());
    _spawner.spawn(toggle_led(led).unwrap());

    // ADC routine 
    let mut adc = Adc::new(p.ADC1);
    let mut pin = p.PA4;
    // acdc dcdc controling channel states
    let mut dcdc = 0;
    let mut acdc = 0;
    // controling pin's 
    let mut dcdc_pin = p.PB0;
    let mut dcdc_pin = p.PB1;

    let mut vrefint = adc.enable_vref();
    let vrefint_sample = adc.read(&mut vrefint).await;

    let num = 30;

    let convert_to_millivolts = |sample| {
        // From http://www.st.com/resource/en/datasheet/CD00161566.pdf
        // 5.3.4 Embedded reference voltage
        const VREFINT_MV: u32 = 1200; // mV

        (u32::from(sample) * VREFINT_MV / u32::from(vrefint_sample)) as u16
    };

    // adc -> mV
    loop {
        let mut sum = 0;
        for i in 0..num{
            let v = adc.read(&mut pin).await;
            sum += v;
        }
        let a_sum = sum / num;
        info!("--> {} - {} mV", a_sum, convert_to_millivolts(a_sum));
        Timer::after(Duration::from_secs(10)).await;
    }
}
