//! Rust wrapper for Zephyr UART Interupt-driven driver.

use core::cell::UnsafeCell;

// Static info for a `Uart` instance.
// Each instance of `Uart` gets their own unique `UartStatic`
// u_Note: This whole area is modelled after what's currently in gpio.rs.
const RX_RINGBUFFER_SIZE: usize = 256;
const TX_RINGBUFFER_SIZE: usize = 256;
const UART_RX_TIMEOUT: i32 = 1000;
pub(crate) struct UartStatic {
    // RX Stuff
    rx_ringbuffer: UnsafeCell<heapless::spsc::Queue<u8, RX_RINGBUFFER_SIZE>>,
    rx_waker: embassy_sync::waitqueue::AtomicWaker,

    // TX Stuff
    tx_ringbuffer: UnsafeCell<heapless::spsc::Queue<u8, TX_RINGBUFFER_SIZE>>,
    tx_waker: embassy_sync::waitqueue::AtomicWaker,
    tx_running: core::sync::atomic::AtomicBool, // Tracks whether or not a uart_tx() is currently in flight
}
unsafe impl Sync for UartStatic {}

impl UartStatic {
    pub(crate) const fn new() -> Self {
        Self {
            rx_ringbuffer:  UnsafeCell::new(heapless::spsc::Queue::new()),
            rx_waker: embassy_sync::waitqueue::AtomicWaker::new(),

            tx_ringbuffer:  UnsafeCell::new(heapless::spsc::Queue::new()),
            tx_waker: embassy_sync::waitqueue::AtomicWaker::new(),
        }
    }
}

// Uart callback. This function signiature has been taken from the uart_callback_set() docs.
unsafe extern "C" fn uart_callback(device: *const crate::raw::device, user_data: *mut core::ffi::c_void) {
    use core::sync::atomic::Ordering;
    use crate::error::{to_result, Error};

    /// Macro to return from the callback if we encounter an `Err()`.
    macro_rules! check {
        ($expr:expr) => {
            match $expr {
                Ok(value) => value,
                Err(_) => { return; }
            }
        };
    }

    /// Helper to check if an IRQ is pending
    fn is_irq_pending(device: *const crate::raw::device) -> Result<bool, ()> {
        match to_result(
            // SAFETY: `device` is a valid UART device pointer for the duration of this call.
            unsafe { crate::raw::uart_irq_is_pending(device) }
        ) {
            Ok(1) => Ok(true),  // An IRQ is pending.
            Ok(0) => Ok(false), // In IRQ is not pending.
            Err(Error(crate::raw::ENOSYS)) => {
                log::error!("uart_irq_is_pending() failed with ENOSYS: This function is not implemented.");
                return Err(());
            },
            Err(Error(crate::raw::ENOTSUP)) => {
                log::error!("uart_irq_is_pending() failed with ENOTSUP: This API is not enabled.");
                return Err(());
            },
            _ => {
                log::error!("uart_irq_is_pending() failed with Zephyr errno {}.", e);
                return Err(());
            }
        }
    }

    /// Helper to check if RX is ready
    fn is_rx_ready(device: *const crate::raw::device) -> Result<bool, ()> {
        match to_result(
            // SAFETY: `device` is a valid UART device pointer for the duration of this call.
            unsafe { crate::raw::uart_irq_rx_ready(device) }
        ) {
            Ok(1) => Ok(true),  // A received char is ready.
            Ok(0) => Ok(false), // A received char is not ready.
            Err(Error(crate::raw::ENOSYS)) => {
                log::error!("uart_irq_rx_ready() failed with ENOSYS: This function is not implemented.");
                return Err(());
            },
            Err(Error(crate::raw::ENOTSUP)) => {
                log::error!("uart_irq_rx_ready() failed with ENOTSUP: This API is not enabled.");
                return Err(());
            },
            _ => {
                log::error!("uart_irq_rx_ready() failed with Zephyr errno {}.", e);
                return Err(());
            }
        }
    }
    
    /// Helper to read from the FIFO. Reads back a single character (returned here as a Option<u8>).
    fn fifo_read(device: *const crate::raw::device) -> Result<Option<u8>, ()> {
        let mut character: u8 = 0;
        match to_result(
            // SAFETY:
            // `character` is large enough to hold one byte. If `uart_fifo_read()` wrutes nire than that
            // into `charater` (which it shouldn't per its contract), then we print out an error and exit the callback.
            unsafe { crate::raw::uart_fifo_read(device, &mut character, 1) }
        ) {
            Ok(1) => Ok(Some(character)),
            Ok(0) => Ok(None),
            Ok(_) => {
                log::error!("uart_fifo_read() failed: `uart_fifo_read()` somehow returned more bytes than we requested. This is probably not good, and might have resulted in a memory overwrite.");
                return Err(());
            }
            Err(Error(crate::raw::ENOSYS)) => {
                log::error!("uart_fifo_read() failed with -ENOSYS: This function is not implemented.");
                return Err(());
            },
            Err(Error(crate::raw::ENOTSUP)) => {
                log::error!("uart_fifo_read() failed with -ENOTSUP: This API is not enabled.");
                return Err(());
            },
            Err(e) => {
                log::error!("uart_fifo_read() failed with Zephyr errno {}.", e);
                return Err(());
            }
        }
    }

    /// Helper to check if TX is ready. Returns `None` if the device isn't ready to write a new byte. Otherwise, returns a `usize` indicating the minimum number of bytes that can be written in a single call to uart_fifo_fill if the device is ready.
    fn is_tx_ready(device: *const crate::raw::device) -> Result<Option<usize>, ()> {
        match to_result(
            // SAFETY: `device` is a valid UART device pointer for the duration of this call.
            unsafe { crate::raw::uart_irq_tx_ready(device) }
        ) {
            Ok(0) => Ok(None),
            Ok(n) => Ok(Some(n as usize)),
            Err(Error(crate::raw::ENOSYS)) => {
                log::error!("uart_irq_tx_ready() failed with -ENOSYS: This function is not implemented.");
                return Err(());
            },
            Err(Error(crate::raw::ENOTSUP)) => {
                log::error!("uart_irq_tx_ready() failed with -ENOTSUP: This API is not enabled.");
                return Err(());
            },
            Err(e) => {
                log::error!("uart_irq_tx_ready() failed with Zephyr errno {}.", e);
                return Err(());
            }
        }
    }

    /// Helper to write a single character into the TX FIFO. Returns `true` if the byte was written, and `false` if it wasn't.
    /// 
    /// Note: `bool` is basically just an error code here, but I wanted to separate the "did it accept our character" indicator from the
    /// actual Zephyr errors that cause the whole callback to return via `check!()`. This is probably not the most idiomatic
    /// but since this function's still in FFI land who cares
    fn fifo_fill(device: *const crate::raw::device, character: u8) -> Result<bool, ()> {
        match to_result(
            // SAFETY: `device` is a valid UART device pointer for the duration of this call.
            unsafe { crate::raw::uart_fifo_fill(device, &character, 1) }
        ) {
            Ok(1) => Ok(true),
            Ok(0) => Ok(false),
            Ok(_) => {
                log::error!("uart_fifo_fill() somehow wrote in more bytes than we requested. That should not be possible.");
                return Err(());
            },
            Err(Error(crate::raw::ENOSYS)) => {
                log::error!("uart_fifo_fill() failed with -ENOSYS: This function is not implemented.");
                return Err(());
            },
            Err(Error(crate::raw::ENOTSUP)) => {
                log::error!("uart_fifo_fill() failed with -ENOTSUP: This API is not enabled.");
                return Err(());
            },
            Err(e) => {
                log::error!("uart_fifo_fill() failed with Zephyr errno {}.", e);
                return Err(());
            }
        }
    }

    // Reborrow the UartStatic state and uart event
    let state = &*(user_data as *const UartStatic);

    // Start processing interrupts in the ISR.
    // According to Zephyr docs you need to call this as the first thing in the ISR before doing other stuff
    crate::raw::uart_irq_update(device);

    // Loop until there's no pending IRQs left to process
    loop {
        // If no IRQ is pending (or there's an error) we can exit out of the callback
        if !check!(is_irq_pending(device)) { return; }

        // Handle RX
        while check!(is_rx_ready(device)) {
            match check!(fifo_read(device)) {
                Some(byte) => {
                    // Enqueue the byte into the ringbuffer
                    let ringbuffer = &mut *state.rx_ringbuffer.get();
                    if ringbuffer.enqueue(byte).is_err() {
                        log::warn!("Uart RX ringbuffer is full! Dropped a byte.");
                    }
                }
                None => break, // The FIFO is drained at this point so we're done and can break out of the while loop.
            }
        }
        state.rx_waker.wake();

        // Handle TX
        if check!(is_tx_ready(device)).is_some() {
            let ringbuffer = &mut *state.tx_ringbuffer.get();
            loop {
                let Some(&byte) = ringbuffer.peek() else { break }; // Nothing left
                
                // Fill one byte and check if it was accepted
                if check!(fifo_fill(device, byte)) {
                    // it was accepted so we don't need that byte anymore
                    if ringbuffer.dequeue()
                    .is_none() {
                        // if `None` was returned, then nothing got dequeued? This shouldn't be possible due to the `break` if
                        // nothing was returned from .peek() but print out an error just in case
                        log::error!("ringbuffer.dequeue() failed: Returned `None`. This shouldn't be possible at this point?");
                        return;
                    }
                } else {
                    // if fifo_fill() returns `false` then our byte wasn't accepted, meaning the fifo is full. So, we're going to break
                    // and then wait for the next TX ISR to try again
                    break;
                }
            }

            // If we get here then we've drained the loop. So, stop the TX IRQ (or it will keep firing forever)
            // Important: we need to re-enable the TX IRQ in the write() function after we enqueue stuff to the ringbuffer
            if ringbuffer.len() == 0 {
                // SAFETY: `device` is a valid UART device pointer for the duration of this call.
                unsafe { crate::raw::uart_irq_tx_disable(device); }
            }
            state.tx_waker.wake();
        }
    }
}

/// A UART peripheral.
/// # u_Note: Put a better doc here eventually
pub struct Uart {
    device: *const crate::raw::device,    // The underlying device itself.
    pub(crate) data: &'static UartStatic, // Our associated data, used for callbacks.
}

impl Uart {
    /// Constructor, used by the devicetree generated code.
    pub(crate) unsafe fn new(
        unique: &crate::device::Unique,
        data: &'static UartStatic,
        device: *const crate::raw::device,
    ) -> Option<Uart> {
        // Make sure this instance doesn't already exist.
        if !unique.once() { return None; }

        // Register the UART callback via uart_irq_callback_user_data_set()
        if let Err(e) = crate::error::to_result_void(
            crate::raw::uart_irq_callback_user_data_set(device, Some(uart_callback), data as *const UartStatic as *mut core::ffi::c_void),
        ) {
            match e.0 {
                crate::raw::ENOSYS => log::error!("uart_irq_callback_user_data_set() returned -ENOSYS: not supported by the device."),
                crate::raw::ENOTSUP => log::error!("uart_irq_callback_user_data_set() returned -ENOTSUP: API not enabled."),
                _ => log::error!("uart_irq_callback_user_data_set() failed with Zephyr errno {}", e),
            }
            return None;
        }

        // Enable RX interrupts (not TX yet though)
        crate::raw::uart_irq_rx_enable(device);

        Some(Uart { device, data })
    }
}