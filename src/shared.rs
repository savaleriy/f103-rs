use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

// Shared async primitives
pub static SHARED_DUTY: Signal<ThreadModeRawMutex, u16> = Signal::new();
pub static SHARED_ADC_VALUE: Signal<ThreadModeRawMutex, u32> = Signal::new();
pub static SHARED_MESSAGE: Signal<ThreadModeRawMutex, u32> = Signal::new();

// Channel to change LED delay dynamically
pub static DELAY_CHANNEL: Channel<ThreadModeRawMutex, u64, 4> = Channel::new();


