#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{Config, Spi};
use embassy_stm32::time::Hertz;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Ping Pong Master Starting...");

    let p = embassy_stm32::init(Default::default());

    // Configure SPI as Master
    let mut spi_config = Config::default();
    spi_config.frequency = Hertz(1_000_000); // 1 MHz

    // SPI1: SCK=PA5, MOSI=PA7, MISO=PA6
    let mut spi = Spi::new(
        p.SPI1, p.PA5, p.PA7, p.PA6, p.DMA1_CH3, p.DMA1_CH2, spi_config,
    );

    // Chip Select (optional but good practice)
    let mut cs = Output::new(p.PA4, Level::High, Speed::Low);

    // LED on PC13 - indicates who has the "ball"
    let mut led = Output::new(p.PC13, Level::Low, Speed::Low);

    // Game state
    let mut ball_in_my_court = true; // Master starts with the ball
    let mut round_count = 0u32;

    // Ping/Pong messages
    const PING_MSG: u8 = 0xAA; // Master -> Slave
    const PONG_MSG: u8 = 0x55; // Slave -> Master
    const ACK_MSG: u8 = 0xCC; // Acknowledgment

    info!("Master ready! Starting ping pong game...");
    led.set_high(); // Master starts with the ball

    loop {
        if ball_in_my_court {
            // It's my turn to send the ball
            info!("Round {}: Master sending PING", round_count);

            let mut tx_buf = [PING_MSG];
            let mut rx_buf = [0u8];

            cs.set_low();
            match spi.transfer(&mut rx_buf, &tx_buf).await {
                Ok(()) => {
                    if rx_buf[0] == ACK_MSG {
                        info!("Slave acknowledged PING");

                        // Wait a bit to make the game visible
                        Timer::after(Duration::from_millis(500)).await;

                        // Now wait for the pong response
                        cs.set_low();
                        match spi.transfer(&mut rx_buf, &[0u8]).await {
                            Ok(()) => {
                                if rx_buf[0] == PONG_MSG {
                                    info!("Master received PONG");
                                    ball_in_my_court = false; // Ball goes to slave
                                    led.set_low(); // Turn off LED - slave has the ball

                                    // Acknowledge receipt
                                    cs.set_low();
                                    let _ = spi.write(&[ACK_MSG]).await;
                                }
                            }
                            Err(e) => {
                                info!("SPI receive error: {:?}", e);
                            }
                        }
                        cs.set_high();
                    }
                }
                Err(e) => {
                    info!("SPI transfer error: {:?}", e);
                }
            }
            cs.set_high();
        } else {
            // Waiting for slave to send the ball back
            Timer::after(Duration::from_millis(100)).await;
        }

        round_count += 1;
        Timer::after(Duration::from_millis(100)).await; // Brief pause between rounds
    }
}
