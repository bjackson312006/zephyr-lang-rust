//! Zephyr macros

use proc_macro::TokenStream;

mod task;
mod driver;

/// Declares a Zephyr thread (or pool of threads) that can be spawned.
///
/// There are some restrictions on this:
/// - All arguments to the function must be Send.
/// - The function must not use generics.
/// - The optional `pool_size` attribute must be 1 or greater.
/// - The `stack_size` must be specified, and will set the size of the pre-defined stack for _each_
///   task in the pool.
///
/// ## Examples
///
/// Declaring a task with a simple argument:
///
/// ```rust
/// #[zephyr::thread(stack_size = 1024)]
/// fn mytask(arg: u32) {
///     // Function body.
/// }
/// ```
///
/// The result will be a function `mytask` that takes this argument, and returns a `ReadyThread`.  A
/// simple use case is to call `.start()` on this, to start the Zephyr thread.
///
/// Threads can be reused after they have exited.  Calling the `mytask` function before the thread
/// has exited will result in a panic.  The `RunningThread`'s `join` method can be used to wait for
/// thread termination.
#[proc_macro_attribute]
pub fn thread(args: TokenStream, item: TokenStream) -> TokenStream {
    task::run(args.into(), item.into()).into()
}

/// Declares a Zephyr device driver.
///
/// This macro generates the boilerplate code needed to implement a Zephyr device driver in Rust,
/// replacing the C macro pattern (e.g., `DT_INST_FOREACH_STATUS_OKAY`) used in traditional drivers.
///
/// ## Arguments
///
/// - `compatible`: The devicetree compatible string for this driver (e.g., "ti,tmp11x-rs")
/// - `subsystem`: The driver subsystem (e.g., sensor, i2c, gpio)
///
/// ## Example
///
/// ```rust
/// use zephyr::driver::{SensorDriver, SensorChannel, SensorValue, Device};
///
/// #[zephyr::driver(compatible = "ti,tmp11x-rs", subsystem = sensor)]
/// pub struct Tmp11xDriver {
///     sample: u16,
///     id: u16,
/// }
///
/// impl SensorDriver for Tmp11xDriver {
///     fn sample_fetch(&mut self, chan: SensorChannel) -> Result<(), i32> {
///         // Fetch sample from sensor
///         Ok(())
///     }
///
///     fn channel_get(&self, chan: SensorChannel) -> Result<SensorValue, i32> {
///         // Return sensor value
///         Ok(SensorValue { val1: 25, val2: 0 })
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn driver(args: TokenStream, item: TokenStream) -> TokenStream {
    driver::driver_impl(args.into(), item.into()).into()
}
