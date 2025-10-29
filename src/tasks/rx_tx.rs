use defmt::*;
use embassy_executor::task;

use embassy_time::{Duration, Timer};

use heapless::{String};

extern crate alloc;
use alloc::vec::Vec;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Receiver, Sender};
use embedded_io_async::{Read, Write};
use scpi::tree::prelude::Context;

use crate::device::device::{MyDevice, MYTREE};


const LOG_LEVEL: &str = "[USART]";

#[task]
pub async fn rx_task(
    mut rx: embassy_stm32::usart::BufferedUartRx<'static>,
    tx_sender: Sender<'static, ThreadModeRawMutex, heapless::String<64>, 4>,
) {
    let mut buf = [0u8; 64];
    let mut pos = 0;

    info!("{}: RX task started", LOG_LEVEL);

    let mut device: MyDevice = MyDevice;

    loop {
        let mut byte = [0u8; 1];
        if let Ok(()) = rx.read_exact(&mut byte).await {
            let b = byte[0];
            info!("{}: Get message", LOG_LEVEL);
            if pos >= buf.len() {
                info!("{}: RX buffer overflow, resetting", LOG_LEVEL);
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

                        // Run SCPI command tree
                        info!("{}: Received", LOG_LEVEL);

                        let command = received_string;

                        let mut context = Context::default();
                        let mut response: Vec<u8> = Vec::new();

                        let res = MYTREE.run(
                            &command.as_bytes(),
                            &mut device,
                            &mut context,
                            &mut response,
                        );

                        match res {
                            Ok(()) => {
                                // Forward response to TX task (ASCII-only for this demo)
                                let mut out = String::<64>::new();
                                for &b in response.as_slice() {
                                    if out.push(b as char).is_err() {
                                        break;
                                    }
                                }
                                let _ = tx_sender.try_send(out);
                            }
                            Err(e) => {
                                warn!("SCPI run error");
                                let mut out = String::<64>::new();
                                let _ = out.push_str("ERR\r\n");
                                let _ = tx_sender.try_send(out);
                            }
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

#[task]
pub async fn tx_task(
    mut tx: embassy_stm32::usart::BufferedUartTx<'static>,
    rx: Receiver<'static, ThreadModeRawMutex, heapless::String<64>, 4>,
) {
    info!("{}: TX task started", LOG_LEVEL);
    loop {
        let msg = rx.receive().await;

        // Convert heapless string to bytes and send
        if let Err(e) = tx.write_all(msg.as_bytes()).await {
            error!("{} : TX write error: {:?}", LOG_LEVEL, e);
        }

        if let Err(e) = tx.flush().await {
            error!("{} : TX flush error: {:?}", LOG_LEVEL, e);
        }
        info!("{} : Sended", LOG_LEVEL);
        Timer::after(Duration::from_millis(100)).await;
    }
}