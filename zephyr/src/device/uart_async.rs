//! Rust wrapper for Zephyr UART Async driver.

// # u_Note: This driver uses Zephyr's async UART API, which requires DMA as far as I'm aware. Eventually, we'll probably want to add wrappers for the other non-async Zephyr UART drivers as well. It would probably be best to rename this to uart_async.rs or something similar, and create totally separate drivers for the other APIs.

use core::cell::UnsafeCell;

/// Number of stop bits.
/// (Note: This is just the Rust version of `uart_config_stop_bits` from `#include <zephyr/drivers/uart.h>`)
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum UartConfigStopBits {
    /// 0.5 stop bit
    UART_CFG_STOP_BITS_0_5 = 0,
    /// 1 stop bit
    UART_CFG_STOP_BITS_1,
    /// 1.5 stop bits
    UART_CFG_STOP_BITS_1_5,
    /// 2 stop bits
    UART_CFG_STOP_BITS_2,
}

/// Parity modes.
/// (Note: This is just the Rust equivalent of `uart_config_parity` from `#include <zephyr/drivers/uart.h>`)
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum UartConfigParity {
    /// No parity.
    UART_CFG_PARITY_NONE = 0,
    /// Odd parity.
    UART_CFG_PARITY_ODD,
    /// Even parity.
    UART_CFG_PARITY_EVEN,
    /// Mark parity.
    UART_CFG_PARITY_MARK,
    /// Space parity.
    UART_CFG_PARITY_SPACE,
}

/// Number of data bits.
/// (This is just the Rust equivalent of `uart_config_data_bits` from `#include <zephyr/drivers/uart.h>`)
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum UartConfigDataBits {
    /// 5 data bits
    UART_CFG_DATA_BITS_5 = 0,
    /// 6 data bits
    UART_CFG_DATA_BITS_6,
    /// 7 data bits
    UART_CFG_DATA_BITS_7,
    /// 8 data bits
    UART_CFG_DATA_BITS_8,
    /// 9 data bits
    UART_CFG_DATA_BITS_9,
}

/// Hardware flow control options.
/// With flow control set to none, any operations related to flow control signals can be managed by user with uart_line_ctrl functions. In other cases, flow control is managed by hardware/driver.  
///
/// (Note: This is just the Rust equivalent of `uart_config_flow_control` from `#include <zephyr/drivers/uart.h>`)
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum UartConfigFlowControl {
    /// No flow control.
    UART_CFG_FLOW_CTRL_NONE = 0,
    /// RTS/CTS flow control.
    UART_CFG_FLOW_CTRL_RTS_CTS,
    /// DTR/DSR flow control.
    UART_CFG_FLOW_CTRL_DTR_DSR,
    /// RS485 flow control.
    UART_CFG_FLOW_CTRL_RS485,
}

// Guy used to hold the memory areas and waker for `Uart`
// Each instance of `Uart` gets their own unique `UartStatic`
// u_Note: This whole area is modelled after what's currently in gpio.rs.
const RX_DMA_BUFFER_SIZE: usize = 32;
const TX_DMA_BUFFER_SIZE: usize = 32;
const RX_RINGBUFFER_SIZE: usize = 256;
const TX_RINGBUFFER_SIZE: usize = 256;
const UART_RX_TIMEOUT: i32 = 1000;
pub(crate) struct UartStatic {
    // RX Stuff
    rx_dma_buffer: [UnsafeCell<[u8; RX_DMA_BUFFER_SIZE]>; 2], // 2 for double buffering
    rx_next_dma_buffer:   core::sync::atomic::AtomicUsize, // Index of rx_dma_buffer (either 0 or 1) corresponding to the NEXT buffer to use for double buffering
    rx_ringbuffer: UnsafeCell<heapless::spsc::Queue<u8, RX_RINGBUFFER_SIZE>>,
    rx_waker: embassy_sync::waitqueue::AtomicWaker,

    // TX Stuff
    tx_dma_buffer: [UnsafeCell<[u8; TX_DMA_BUFFER_SIZE]>; 1], // We don't need to double buffer for TX
    tx_ringbuffer: UnsafeCell<heapless::spsc::Queue<u8, TX_RINGBUFFER_SIZE>>,
    tx_waker: embassy_sync::waitqueue::AtomicWaker,
    tx_running: core::sync::atomic::AtomicBool, // Tracks whether or not a uart_tx() is currently in flight
}
unsafe impl Sync for UartStatic {}

impl UartStatic {
    pub(crate) const fn new() -> Self {
        Self {
            rx_dma_buffer:   [const { UnsafeCell::new([0; RX_DMA_BUFFER_SIZE]) }; 2],
            rx_next_dma_buffer:   core::sync::atomic::AtomicUsize::new(1),
            rx_ringbuffer:  UnsafeCell::new(heapless::spsc::Queue::new()),
            rx_waker: embassy_sync::waitqueue::AtomicWaker::new(),

            tx_dma_buffer:   [const { UnsafeCell::new([0; TX_DMA_BUFFER_SIZE]) }; 1],
            tx_ringbuffer:  UnsafeCell::new(heapless::spsc::Queue::new()),
            tx_waker: embassy_sync::waitqueue::AtomicWaker::new(),
            tx_running: core::sync::atomic::AtomicBool::new(false),
        }
    }
}

// Function to "kick" the TX dispatcher.
// In other words, this function gets the TX dispatcher to transfer bytes into the outgoing DMA buffer so they can actually get sent.
//
// Idempotent: if `tx_running == true` when called, returns immediately and the in-flight transmission will pick up any newly-enqueued bytes when it completes. Otherwise, claims the engine (via CAS on `tx_running`), drains the ringbuffer into the DMA buffer, calls `uart_tx()`, and either leaves the engine running (success) or releases the engine (drained empty / `uart_tx()` errored).
//
// Callers: `write()` after enqueueing bytes, and `uart_callback()` (after manually clearing `tx_running` to release the engine claim held by the previous transmission).
//
// SAFETY: There's never a race over the buffers since this function always checks tx_running
unsafe fn kick_tx(device: *const crate::raw::device, state: &UartStatic) {
    use core::sync::atomic::Ordering;

    // Check if TX is already running. If it is, return.
    if state.tx_running
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    // Dequeue bytes from the TX ringbuffer and start adding them to the (currently empty) outgoing TX DMA buffer.
    let mut bytes_written: usize = 0; // Tracks how many bytes we write to the TX DMA buffer, corresponding to the `len` parameter in uart_tx().
    let ringbuffer = &mut *state.tx_ringbuffer.get();
    let slice: &mut [u8] = unsafe { &mut *state.tx_dma_buffer[0].get() };

    // Loop until either the ringbuffer is empty, or the outgoing TX DMA buffer is completely full
    while bytes_written < slice.len() {
        // Deqeue bytes from ringbuffer and place them in the outgoing TX DMA buffer
        if let Some(byte) = ringbuffer.dequeue() {
            slice[bytes_written] = byte;
            bytes_written += 1;
        } else {
            break; // Ringbuffer is empty, so we're done
        }
    }

    const SYS_FOREVER_US: i32 = -1; // This is how Zephyr defines this macro. crate::raw doesn't create a constant for the macro though, so we have to define it here. u_Note: if we end up using more of these macros it might be good to set up a dedicated module for them somewhere just so we can keep them all in one place and easily cross-reference them with the Zephyr headers. Or, maybe just have the codegen generate these macros if that's possible

    // If any bytes were actually written, call uart_tx() to send them
    if bytes_written > 0 {
        if let Err(e) = crate::error::to_result_void(
            // If any bytes were written, call uart_tx() to send them off
            crate::raw::uart_tx(device, slice.as_ptr(), bytes_written, SYS_FOREVER_US)
        ) {
            match e.0 {
                crate::raw::ENOTSUP => log::error!("uart_tx() returned -ENOTSUP: API is not enabled."),
                crate::raw::EBUSY => log::error!("uart_tx() returned -EBUSY: There is already an ongoing transfer"),
                _ => log::error!("uart_tx() failed with Zephyr errno {}", e),
            }
            // uart_tx() failed, so no TX_DONE/TX_ABORTED will fire to release the engine. Release it ourselves so the next kick_tx() can retry.
            state.tx_running.store(false, Ordering::Release);
        }
    } else {
        // Ringbuffer was empty. nothing to send, so release the engine. The next write() will re-claim via kick_tx().
        state.tx_running.store(false, Ordering::Release);
    }

    // Wake any TX writers parked on a full ringbuffer (or waiting to re-claim a released engine). Placed last so that it covers all three exit paths (chained successfully / drained empty / errored).
    state.tx_waker.wake();
}

// Uart callback. This function signiature has been taken from the uart_callback_set() docs.
unsafe extern "C" fn uart_callback(_device: *const crate::raw::device, event: *mut crate::raw::uart_event, user_data: *mut core::ffi::c_void) {
    use core::sync::atomic::Ordering;

    // Reborrow the UartStatic state and uart event
    let state = &*(user_data as *const UartStatic);
    let event = &*event;

    match event.type_ {

        // Buffer is no longer used by UART driver.
        crate::raw::uart_event_type_UART_RX_BUF_RELEASED => {
            // Nothing to do here. We already drained everything on RDY, and the
            // next BUF_REQUEST will give this buffer back to the driver.
        }

        // Driver requests next buffer for continuous reception.
        crate::raw::uart_event_type_UART_RX_BUF_REQUEST => {
            // Give the driver the OTHER buffer
            let index = state.rx_next_dma_buffer.fetch_xor(1, Ordering::AcqRel) & 1; // XOR to toggle the bit
            let buffer = state.rx_dma_buffer[index].get() as *mut u8;
            
            // Call uart_rx_buf_rsp() to provide the next recieve buffer.
            // We're not gonna return an error code here since if this function fails,
            // RX will eventually fire UART_RX_DISABLED and we'll recover there. But, we'll still print out the specific error codes here
            // for convenience.
            if let Err(e) = crate::error::to_result_void(
                crate::raw::uart_rx_buf_rsp(_device, buffer, RX_DMA_BUFFER_SIZE)
            ) {
                match e.0 {
                    crate::raw::EBUSY => log::error!("uart_rx_buf_rsp() returned -EBUSY: Next buffer already set."),
                    crate::raw::ENOTSUP => log::error!("uart_rx_buf_rsp() returned -ENOTSUP: API is not enabled."),
                    crate::raw::EACCES => log::error!("uart_rx_buf_rsp() returned -EACCES: Receiver is already disabled (function called too late?)."),
                    _ => log::error!("uart_rx_buf_rsp() failed with Zephyr errno {}", e),
                }
                // again, not going to return anything here. This is just for logging
            }
        }

        // RX has been disabled and can be reenabled
        crate::raw::uart_event_type_UART_RX_DISABLED => {
            // RX shut down (we missed a BUF_REQUEST, or someone called
            // uart_rx_disable). Restart from buffer 0.
            state.rx_next_dma_buffer.store(1, Ordering::Release);
            let buffer = state.rx_dma_buffer[0].get() as *mut u8;

            // Call uart_rx_enable() to restart uart. 
            // We don't need to explicitly handle an error
            // here, since if uart_rx_enable() fails, we'll just hit uart_event_type_UART_RX_DISABLED the
            // next time around (since UART will still be disabled). We're still going to log the specific
            // error codes though for conveinience.
            if let Err(e) = crate::error::to_result_void(
                crate::raw::uart_rx_enable(_device, buffer, RX_DMA_BUFFER_SIZE, UART_RX_TIMEOUT,)
            ) {
                match e.0 {
                    crate::raw::EBUSY => log::error!("uart_rx_enable() returned -EBUSY: RX already in progress."),
                    crate::raw::ENOTSUP => log::error!("uart_rx_enable() returned -ENOTSUP: API is not enabled."),
                    _ => log::error!("uart_rx_enable() failed with Zephyr errno {}", e),
                }
                // again, not going to return anything here. This is just for logging
            }
        }

        // Received data is ready for processing.
        crate::raw::uart_event_type_UART_RX_RDY => {

            // Bytes landed in one of the RX DMA buffers; drain into the ringbuffer.
            let rx = event.data.rx.as_ref();
            let slice = core::slice::from_raw_parts(rx.buf.add(rx.offset), rx.len);

            // SPSC: this callback is the only producer.
            let ringbuffer = &mut *state.rx_ringbuffer.get();
            let mut dropped: usize = 0; // Counter to count how many bytes were dropped
            for &byte in slice {
                // Enque bytes.
                // If the queue is full, .enqueue will return an error, and we increment a counter.
                if ringbuffer.enqueue(byte).is_err() {
                    dropped += 1;
                }
            }

            // Print out a warning if bytes were dropped.
            if dropped > 0 { log::warn!("Uart RX ringbuffer is full, dropped {} bytes.", dropped); }

            state.rx_waker.wake();
        }

        // RX has stopped due to external event.
        crate::raw::uart_event_type_UART_RX_STOPPED => {
            // Don't need to do anything specific here, since we will end up at uart_event_type_UART_RX_DISABLED the next time around.
        }

        // Transmitting aborted due to timeout or uart_tx_abort call
        crate::raw::uart_event_type_UART_TX_ABORTED => {
            // A previous uart_tx() got aborted (timeout or uart_tx_abort()). The DMA buffer is free to reuse, so treat this like TX_DONE and keep the chain going.
            let tx = event.data.tx.as_ref();
            log::warn!("uart_tx() aborted after sending {} bytes; remainder of that submission is lost.", tx.len);

            state.tx_running.store(false, Ordering::Release); // This callback being triggered means that TX isn't running anymore, so update that state.
            kick_tx(_device, state); // Then immediately kick TX again
        }

        // Whole TX buffer was transmitted, so we can fill it up with outgoing stuff again.
        crate::raw::uart_event_type_UART_TX_DONE => {
            state.tx_running.store(false, Ordering::Release); // This callback being triggered means that TX isn't running anymore, so update that state.
            kick_tx(_device, state); // Then immediately kick TX again
        }

        // Unknown event type?
        _ => {
            log::warn!("uart_callback() has recieved an unknown event type. Weird! (event: {})", event.type_);
        }
    }
}

/// A UART peripheral.
/// (This is a wrapper around the `struct device` in Zephyr that represents a UART controller. This driver utilizes Zephyr's async UART API.)
/// 
/// # Using This Struct From The Devicetree:
/// 
/// Unlike externally-wired devices (e.g. a sensor), a UART is usually
/// an on-chip peripheral that is already declared in the board's `.dts` file with the
/// vendor-specific compatible (`nxp,lpc-usart`, `nordic,nrf-uarte`, `raspberrypi,pico-uart`,
/// etc.). So, you normally don't add a new devicetree node yourself to use this driver. You 
/// should just be able to reference the one your board already provides by its label (e.g. `uart0`, `flexcomm0`, `usart1`).
/// 
/// For example, if my chip had a UART peripheral called `uart0` in the devicetree, I'd retrieve an instance of it like:
/// ```rust
/// let mut uart: zephyr::device::uart::Uart = zephyr::devicetree::labels::uart0::get_instance().unwrap();
/// ```
/// 
/// You'll also want to enable serial and the Zephyr UART async API in your prj.conf:
/// ```kconfig
/// CONFIG_SERIAL=y
/// CONFIG_UART_ASYNC_API=y
/// ```
/// 
/// If needed, you can also disable Zephyr's UART logging if those messages would collide with your traffic:
/// ```kconfig
/// CONFIG_UART_CONSOLE=n
/// CONFIG_LOG_BACKEND_UART=n
/// ```
/// 
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

        // Register the UART callback via uart_callback_set()
        if let Err(e) = crate::error::to_result_void(
            crate::raw::uart_callback_set(device, Some(uart_callback), data as *const UartStatic as *mut core::ffi::c_void),
        ) {
            match e.0 {
                crate::raw::ENOSYS => log::error!("uart_callback_set() returned -ENOSYS: not supported by the device."),
                crate::raw::ENOTSUP => log::error!("uart_callback_set() returned -ENOTSUP: API not enabled."),
                _ => log::error!("uart_callback_set() failed with Zephyr errno {}", e),
            }
            return None;
        }

        // Enable incoming uart via uart_rx_enable()
        if let Err(e) = crate::error::to_result_void(
            crate::raw::uart_rx_enable(device, data.rx_dma_buffer[0].get() as *mut u8, RX_DMA_BUFFER_SIZE, UART_RX_TIMEOUT)
        ) {
            match e.0 {
                crate::raw::ENOTSUP => log::error!("uart_rx_enable() returned -ENOTSUP: API not enabled."),
                crate::raw::EBUSY => log::error!("uart_rx_enable() returned -EBUSY: RX already in progress."),
                _ => log::error!("uart_rx_enable() failed with Zephyr errno {}", e),
            }
            return None;
        }

        Some(Uart { device, data })
    }

    /// Lets you get the current uart config.
    pub(crate) fn config_get(&self) -> crate::error::Result<crate::raw::uart_config> {
        let mut config = crate::raw::uart_config::default();

        // Call uart_config_get()
        // SAFETY: `self.device` is valid for the lifetime of the object, and `&mut config` is a valid exclusive pointer of the type uart_config_get() expects.
        unsafe {
            if let Err(e) = crate::error::to_result_void(
                crate::raw::uart_config_get(self.device, &mut config),
            ) {
                match e.0 {
                    crate::raw::ENOSYS => log::error!("uart_config_get() returned -ENOSYS: driver does not support getting current configuration."),
                    crate::raw::ENOTSUP => log::error!("uart_config_get() returned -ENOTSUP: API is not enabled."),
                    _ => log::error!("uart_config_get() failed with Zephyr errno {}", e),
                }
                return Err(e);
            }
        }

        Ok(config)
    }

    /// Reconfigure the UART at runtime.
    pub fn configure(
        &self,
        baudrate: u32,
        parity: UartConfigParity,
        stop_bits: UartConfigStopBits,
        data_bits: UartConfigDataBits,
        flow_ctrl: UartConfigFlowControl,
    ) -> crate::error::Result<()> {
        let config = crate::raw::uart_config {
            baudrate,
            parity: parity as u8,
            stop_bits: stop_bits as u8,
            data_bits: data_bits as u8,
            flow_ctrl: flow_ctrl as u8,
        };

        // SAFETY: `self.device` is valid for the lifetime of `self`, and `&config` is a valid read-only pointer to an initialized `uart_config` that the C side only reads from.
        if let Err(e) = crate::error::to_result_void(unsafe {
            crate::raw::uart_configure(self.device, &config)
        }) {
            match e.0 {
                crate::raw::ENOSYS => log::error!("uart_configure() returned -ENOSYS: configuration is not supported by device or driver does not support setting configuration in runtime."),
                crate::raw::ENOTSUP => log::error!("uart_configure() returned -ENOTSUP: API is likely not enabled"),
                _ => log::error!("uart_configure() failed with Zephyr errno {}", e),
            }
            return Err(e);
        }

        Ok(())
    }
}

impl embedded_io_async::ErrorType for Uart {
    type Error = embedded_io_async::ErrorKind;
}

impl embedded_io_async::Read for Uart {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {

        if buf.is_empty() {
            return Ok(0);
        }

        core::future::poll_fn(|cx| {
            // SAFETY: exclusive access on the consumer side, producer only touches it from the UART callback via enqueue().
            let ringbuffer = unsafe { &mut *self.data.rx_ringbuffer.get() };

            // Drain the ringbuffer.
            // Basically just continuously move bytes from our ringbuffer into the user's `buf` until
            // either our ringbuffer is completely empty or the user's `buf` is completely full.
            let mut n = 0;
            while n < buf.len() {
                match ringbuffer.dequeue() {
                    Some(byte) => { buf[n] = byte; n += 1; }
                    None => break,
                }
            }
            if n > 0 {return core::task::Poll::Ready(Ok(n));} // Return success, indicating how many bytes we drained. However, if we didn't drain any bytes (meaning the ringbuffer was empty), fall through to the below case.

            // If we get here, we our ringbuffer was empty so we didn't drain anything.
            // So, register the waker so that whenever there ARE bytes to drain, this task gets woken up to finish its job.
            self.data.rx_waker.register(cx.waker()); // We have to register this waker before the final check (i.e., before we know if we will return core::task::Poll::Pending or not) because the ISR might place a byte in the empty buffer WHILE we are doing the check.
            match ringbuffer.dequeue() {
                Some(byte) => {
                    buf[0] = byte;
                    core::task::Poll::Ready(Ok(1))
                }
                None => core::task::Poll::Pending,
            }
        })
        .await
    }
}

impl embedded_io_async::Write for Uart {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {

        if buf.is_empty() {
            return Ok(0);
        }

        let n = core::future::poll_fn(|cx| {
            // SAFETY: exclusive access on the producer side
            let ringbuffer = unsafe { &mut *self.data.tx_ringbuffer.get() };

            // Enqueue as many bytes from `buf` as will fit.
            // Basically just continuously move bytes from the user's `buf` into our ringbuffer until either our ringbuffer is completely full or the user's `buf` has been completely consumed.
            let mut n = 0;
            while n < buf.len() {
                if ringbuffer.enqueue(buf[n]).is_err() { break; }
                n += 1;
            }
            if n > 0 { return core::task::Poll::Ready(n); } // Return success, indicating how many bytes we enqueued. However, if we didn't enqueue any bytes (meaning the ringbuffer was full), fall through to the below case.

            // If we get here, our ringbuffer was full so we couldn't enqueue anything. So, register the waker so that whenever there IS space to enqueue, this task gets woken up to finish its job.
            self.data.tx_waker.register(cx.waker()); // We have to register this waker before the final check (i.e., before we know if we will return core::task::Poll::Pending or not) because the ISR might drain a byte from the full buffer WHILE we are doing the check.
            match ringbuffer.enqueue(buf[0]) {
                Ok(()) => core::task::Poll::Ready(1),
                Err(_) => core::task::Poll::Pending,
            }
        })
        .await;

        // Kick the TX engine. No-op if the callback chain is already running; otherwise claims the engine and starts a uart_tx() with whatever we just enqueued.
        // SAFETY: We're the entry point for kick_tx(). If there's already an active TX chain going, kick_tx() won't do anything and we can just wait for that chain to run its course.
        unsafe { kick_tx(self.device, self.data); }

        Ok(n)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {

        core::future::poll_fn(|cx| {
            use core::sync::atomic::Ordering;

            // SAFETY: exclusive consumer-side access to the ringbuffer's "is empty" view via &mut self. 
            // The callback, who's the producer of "empty" as we understand it, only changes len() downward (never upward).
            let ringbuffer = unsafe { &*self.data.tx_ringbuffer.get() };

            // If nothing's pending to be sent and tx_running is already false, then we're already flushed
            if ringbuffer.len() == 0 && !self.data.tx_running.load(Ordering::Acquire) {
                return core::task::Poll::Ready(Ok(()));
            }

            // We need to re-check before returning. So, do literally the exact same thing we do above, but
            // register the waker first just in case a wake fires as we are actively doing the check
            self.data.tx_waker.register(cx.waker());
            if ringbuffer.len() == 0 && !self.data.tx_running.load(Ordering::Acquire) {
                core::task::Poll::Ready(Ok(()))
            } else {
                core::task::Poll::Pending
            }
        })
        .await
    }
}