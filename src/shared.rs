use defmt::Format;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;


// Power control types
#[derive(Debug, Clone, Copy, PartialEq, Format)]
pub enum PowerState {
    DCDC,
    ACDC,
    OFF,
}

#[derive(Debug, Clone, Copy, PartialEq, Format)]
pub enum LedState {
    On,
    Off,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Format)]
pub enum CoolingState {
    On,
    Off,
}

// Shared async primitives
pub static SHARED_DUTY: Signal<ThreadModeRawMutex, u16> = Signal::new();
pub static SHARED_ADC_VALUE: Signal<ThreadModeRawMutex, u32> = Signal::new();
pub static SHARED_MESSAGE: Signal<ThreadModeRawMutex, u32> = Signal::new();

// Device control channels
pub static LED_CHANNEL: Channel<ThreadModeRawMutex, LedState, 4> = Channel::new();
pub static POWER_CHANNEL: Channel<ThreadModeRawMutex, PowerState, 4> = Channel::new();
pub static COOLING_CHANNEL: Channel<ThreadModeRawMutex, CoolingState, 4> = Channel::new();
pub static SPEED_CHANNEL: Channel<ThreadModeRawMutex, u16, 4> = Channel::new();

// Channel to change LED delay dynamically
pub static DELAY_CHANNEL: Channel<ThreadModeRawMutex, u64, 4> = Channel::new();

// Device status signals
pub static POWER_STATUS: Signal<ThreadModeRawMutex, PowerState> = Signal::new();
pub static LED_STATUS: Signal<ThreadModeRawMutex, bool> = Signal::new();
pub static COOLING_STATUS: Signal<ThreadModeRawMutex, CoolingState> = Signal::new();
pub static CURRENT_SPEED: Signal<ThreadModeRawMutex, u16> = Signal::new();

// Channel to send messages from RX to TX
pub static TX_MESSAGE_CHANNEL: Channel<ThreadModeRawMutex, heapless::String<64>, 4> = Channel::new();