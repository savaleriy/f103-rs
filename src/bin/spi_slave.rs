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
    info!("Ping Pong Slave Starting...");

    let p = embassy_stm32::init(Default::default());

    // Configure SPI as Slave
    let mut spi_config = Config::default();
    spi_config.frequency = Hertz(1_000_000);

    // SPI1 as slave: SCK=PA5, MOSI=PA7, MISO=PA6, NSS=PA4
    let mut spi = Spi::new_slave(
        p.SPI1, p.PA5, p.PA7, p.PA6, p.DMA1_CH3, p.DMA1_CH2, spi_config,
    );

    // LED on PC13 - indicates who has the "ball"
    let mut led = Output::new(p.PC13, Level::Low, Speed::Low);

    // Game state
    let mut ball_in_my_court = false; // Slave starts without the ball
    let mut round_count = 0u32;

    // Message definitions (must match master)
    const PING_MSG: u8 = 0xAA;
    const PONG_MSG: u8 = 0x55;
    const ACK_MSG: u8 = 0xCC;

    info!("Slave ready! Waiting for ping pong game to start...");

    loop {
        if ball_in_my_court {
            // I have the ball - send it back after a delay
            info!("Round {}: Slave sending PONG", round_count);

            // Wait a bit to make the game visible
            Timer::after(Duration::from_millis(500)).await;

            // Send PONG response
            match spi.write(&[PONG_MSG]).await {
                Ok(()) => {
                    info!("Slave sent PONG");

                    // Wait for acknowledgment
                    let mut rx_buf = [0u8];
                    match spi.read(&mut rx_buf).await {
                        Ok(()) => {
                            if rx_buf[0] == ACK_MSG {
                                info!("Master acknowledged PONG");
                                ball_in_my_court = false; // Ball goes back to master
                                led.set_low(); // Turn off LED - master has the ball
                            }
                        }
                        Err(e) => {
                            info!("SPI read error: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    info!("SPI write error: {:?}", e);
                }
            }
        } else {
            // Waiting for master to send the ball
            let mut rx_buf = [0u8];
            match spi.read(&mut rx_buf).await {
                Ok(()) => {
                    if rx_buf[0] == PING_MSG {
                        info!("Round {}: Slave received PING", round_count);

                        // Acknowledge receipt
                        let _ = spi.write(&[ACK_MSG]).await;

                        ball_in_my_court = true; // Now I have the ball
                        led.set_high(); // Turn on LED - I have the ball
                    }
                }
                Err(e) => {
                    // No data or error - continue waiting
                    Timer::after(Duration::from_millis(10)).await;
                }
            }
        }

        round_count += 1;
    }
}
