#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::usart::{BufferedUart, Config};
use embassy_stm32::{bind_interrupts, peripherals, usart, Peri};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Timer};
use embedded_io_async::{Read, Write};

use alloc::vec::Vec;
use heapless::String;

use scpi::{cmd_nquery, cmd_qonly, error::Error, tree::prelude::*, Branch, Leaf, Root};

use embedded_alloc::TlsfHeap as Heap;
#[global_allocator]
static HEAP: Heap = Heap::empty();

extern crate alloc;

use {defmt_rtt as _, panic_probe as _};

// Backing memory for the global allocator (TLSF)
// Adjust size as needed depending on expected dynamic allocations
#[link_section = ".uninit"]
static mut HEAP_MEM: [u8; 8 * 1024] = [0; 8 * 1024];

bind_interrupts!(struct Irqs {
    USART1 => usart::BufferedInterruptHandler<peripherals::USART1>;
});

// Static buffers for UART (fixed lifetime issues)
static mut TX_BUF: [u8; 256] = [0; 256];
static mut RX_BUF: [u8; 256] = [0; 256];

// Channel to send messages from RX to TX
static TX_MESSAGE_CHANNEL: Channel<ThreadModeRawMutex, heapless::String<64>, 4> = Channel::new();
static LED_CHANNEL: Channel<ThreadModeRawMutex, LedState, 4> = Channel::new();

// Example of SIMPLE DEVICE
struct MyDevice;
impl Device for MyDevice {
    fn handle_error(&mut self, err: Error) {
        info!("Error");
    }
}

struct IdnCommand;
impl Command<MyDevice> for IdnCommand {
    // Allow only queries
    cmd_qonly!();

    // Called when a query is made
    fn query(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
        mut resp: ResponseUnit,
    ) -> scpi::error::Result<()> {
        resp.data(b"STM32F103 Embassy!".as_slice()).finish()
    }
}

struct LedToggleCommand;
impl Command<MyDevice> for LedToggleCommand {
    cmd_nquery!();
    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI : TOGGLE");
        let _ = LED_CHANNEL.try_send(LedState::Toggle);
        Ok(())
    }
}

struct LedOnCommand;
impl Command<MyDevice> for LedOnCommand {
    cmd_nquery!();
    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI : ON");
        let _ = LED_CHANNEL.try_send(LedState::On);
        Ok(())
    }
}

struct LedOffCommand;
impl Command<MyDevice> for LedOffCommand {
    cmd_nquery!();
    fn event(
        &self,
        _device: &mut MyDevice,
        _context: &mut Context,
        _params: Parameters,
    ) -> Result<(), Error> {
        info!("SCPI : OFF");
        let _ = LED_CHANNEL.try_send(LedState::Off);
        Ok(())
    }
}

// Basic commands
// *IDN?      -> STM32F103 Embassy!
// LED:TOGGle -> Toggle led
// LED:ON     -> On led
// LED:OFF    -> Off led

const MYTREE: Node<MyDevice> = Root![
    Leaf!(b"*IDN" => &IdnCommand),
    Branch![b"LED";
        Leaf!(default b"TOGGle" => &LedToggleCommand),
        Leaf!(default b"ON" => &LedOnCommand),
        Leaf!(default b"OFF" => &LedOffCommand)
    ]
];

#[derive(Debug, Clone, Copy)]
enum LedState {
    Toggle,
    On,
    Off,
}

#[embassy_executor::task]
async fn blinky(led: Peri<'static, peripherals::PC13>) {
    let mut led = Output::new(led, Level::High, Speed::Low);

    loop {
        match LED_CHANNEL.receive().await {
            LedState::Toggle => {
                led.toggle();
                info!("Toggle LED");
            }
            LedState::On => {
                led.set_low();
                info!("LED On");
            }
            LedState::Off => {
                led.set_high();
                info!("LED Off");
            }
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}

#[embassy_executor::task]
async fn rx_task(
    mut rx: embassy_stm32::usart::BufferedUartRx<'static>,
    tx_sender: Sender<'static, ThreadModeRawMutex, heapless::String<64>, 4>,
) {
    let mut buf = [0u8; 64];
    let mut pos = 0;

    info!("RX task started");

    let mut device: MyDevice = MyDevice;

    loop {
        let mut byte = [0u8; 1];
        if let Ok(()) = rx.read_exact(&mut byte).await {
            let b = byte[0];
            info!("Get message");
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

                        // Run SCPI command tree
                        info!("Received");

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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize allocator before any use of heap (Vec, Box, etc.)
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

    spawner.spawn(rx_task(rx, tx_sender).unwrap());
    spawner.spawn(tx_task(tx, tx_receiver).unwrap());
    spawner.spawn(blinky(p.PC13).unwrap());

    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}
