#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::can::frame::Envelope;
use embassy_stm32::can::{
    filter, Can, Fifo, Frame, Id, Rx0InterruptHandler, Rx1InterruptHandler, SceInterruptHandler,
    StandardId, TxInterruptHandler,
};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::CAN;
use embassy_stm32::{bind_interrupts, Config};
use embassy_time::{with_timeout, Duration, TimeoutError, Timer};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USB_LP_CAN1_RX0 => Rx0InterruptHandler<CAN>;
    CAN1_RX1 => Rx1InterruptHandler<CAN>;
    CAN1_SCE => SceInterruptHandler<CAN>;
    USB_HP_CAN1_TX => TxInterruptHandler<CAN>;
});

fn handle_frame(env: Envelope, read_mode: &str) {
    match env.frame.id() {
        Id::Extended(id) => {
            defmt::println!(
                "{} Extended Frame id={:x} {:02x}",
                read_mode,
                id.as_raw(),
                env.frame.data()
            );
        }
        Id::Standard(id) => {
            defmt::println!(
                "{} Standard Frame id={:x} {:02x}",
                read_mode,
                id.as_raw(),
                env.frame.data()
            );
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Config::default());

    // Set alternate pin mapping to B8/B9
    embassy_stm32::pac::AFIO
        .mapr()
        .modify(|w| w.set_can1_remap(2));

    // Initialize the LED pin (PC13 is common on bluepill)
    let mut led = Output::new(p.PC13, Level::Low, Speed::Low); // Start with LED off

    static RX_BUF: StaticCell<embassy_stm32::can::RxBuf<10>> = StaticCell::new();
    static TX_BUF: StaticCell<embassy_stm32::can::TxBuf<10>> = StaticCell::new();

    let mut can = Can::new(p.CAN, p.PB8, p.PB9, Irqs);
    info!("CAN init");

    can.modify_filters()
        .enable_bank(0, Fifo::Fifo0, filter::Mask32::accept_all());

    can.modify_config()
        .set_loopback(false)
        .set_silent(false)
        .set_bitrate(250_000);

    can.enable().await;

    let (mut tx, mut rx) = can.split();
    let mut rx = rx.buffered(RX_BUF.init(embassy_stm32::can::RxBuf::<10>::new()));
    let mut tx = tx.buffered(TX_BUF.init(embassy_stm32::can::TxBuf::<10>::new()));

    let mut i: u8 = 0;
    // Optional: Send an initial message to confirm setup
    let initial_frame =
        Frame::new_data(unwrap!(StandardId::new(0x123)), &[0xDE, 0xAD, 0xBE, 0xEF]).unwrap();
    tx.write(&initial_frame).await;
    info!("Initial message sent.");

    loop {
        // Attempt to read a CAN frame with a specific timeout (e.g., 500ms)
        // This uses embassy_time's with_timeout function [[7]]
        match with_timeout(Duration::from_millis(500), rx.read()).await {
            Ok(Ok(envelope)) => {
                // Message received successfully within the timeout
                led.set_high(); // Turn LED on
                handle_frame(envelope, "Received");
                info!("Message received, LED ON");
                // Optional: Turn LED off after a short delay
                // Timer::after(Duration::from_millis(200)).await;
                // led.set_low();
            }
            Ok(Err(e)) => {
                // An error occurred during the read operation (not a timeout)
                led.set_low(); // Ensure LED is off
                info!("CAN read error: {}, LED OFF", e);
            }
            Err(_) => {
                // Err from with_timeout indicates a TimeoutError [[8]]
                // No message was received within the timeout period
                led.set_low(); // Turn LED off
                info!("No message received within timeout, LED OFF");
            }
        }

        // Optional: Send a periodic message
        let tx_frame = Frame::new_data(
            unwrap!(StandardId::new(0x200 + (i as u16))),
            &[i, i + 1, i + 2],
        )
        .unwrap();
        tx.write(&tx_frame).await;
        i = i.wrapping_add(1);
    }
}
