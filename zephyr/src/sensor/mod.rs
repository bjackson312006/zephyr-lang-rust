//! Generic sensor driver framework
//!
//! Provides traits and types for building Zephyr sensor drivers in Rust.

pub mod types;
pub use types::*;

/// Trait that all sensor drivers must implement
pub trait SensorDriver {
    /// Initialize the driver
    ///
    /// # Parameters
    /// * `dev` - Pointer to the device structure
    /// * `device_ready` - Whether the device is ready
    fn init(&mut self, dev: *const crate::raw::device, device_ready: bool) -> SensorResult<()>;

    /// Set sensor attribute (optional)
    ///
    /// # Parameters
    /// * `_dev` - Pointer to the device structure
    /// * `_channel` - The sensor channel
    /// * `_attr` - The attribute ID to set
    /// * `_value` - The value to set
    fn attr_set(
        &mut self,
        _dev: *const crate::raw::device,
        _channel: SensorChannel,
        _attr: i32,
        _value: &SensorValue,
    ) -> SensorResult<()> {
        Err(SensorError::NotSupported)
    }

    /// Get sensor attribute (optional)
    ///
    /// # Parameters
    /// * `_dev` - Pointer to the device structure
    /// * `_channel` - The sensor channel
    /// * `_attr` - The attribute ID to get
    fn attr_get(
        &self,
        _dev: *const crate::raw::device,
        _channel: SensorChannel,
        _attr: i32,
    ) -> SensorResult<SensorValue> {
        Err(SensorError::NotSupported)
    }

    /// Set trigger configuration (optional)
    ///
    /// # Parameters
    /// * `_dev` - Pointer to the device structure
    /// * `_trig` - Pointer to the trigger configuration
    /// * `_handler` - Trigger handler callback
    fn trigger_set(
        &mut self,
        _dev: *const crate::raw::device,
        _trig: *const crate::raw::sensor_trigger,
        _handler: crate::raw::sensor_trigger_handler_t,
    ) -> SensorResult<()> {
        Err(SensorError::NotSupported)
    }

    /// Fetch sample(s) from the sensor
    ///
    /// # Parameters
    /// * `dev` - Pointer to the device structure
    /// * `channel` - The sensor channel to fetch from
    fn sample_fetch(
        &mut self,
        dev: *const crate::raw::device,
        channel: SensorChannel,
    ) -> SensorResult<()>;

    /// Fetch sample(s) from a specific sensor channel (optional)
    ///
    /// # Parameters
    /// * `_dev` - Pointer to the device structure
    /// * `_channel` - The sensor channel to fetch from
    fn sample_fetch_chan(
        &mut self,
        _dev: *const crate::raw::device,
        _channel: SensorChannel,
    ) -> SensorResult<()> {
        Err(SensorError::NotSupported)
    }

    /// Get channel value (after fetch)
    ///
    /// # Parameters
    /// * `channel` - The sensor channel to read from
    fn channel_get(&self, channel: SensorChannel) -> SensorResult<SensorValue>;
}

/// Macro to generate FFI exports for a sensor driver
///
/// This macro generates all the necessary FFI glue code to export a Rust
/// sensor driver to C code. It creates:
/// - A static driver_api vtable
/// - FFI wrapper functions for all sensor operations
/// - Proper unsafe handling at the FFI boundary
///
/// # Parameters
///
/// * `driver` - The type implementing `SensorDriver` trait. Must have a `const fn new()` method.
/// * `prefix` - The prefix for generated C symbols
///
/// # Usage
///
/// ```rust,no_run
/// # use zephyr::sensor::{SensorDriver, SensorChannel, SensorResult, SensorValue};
/// # struct MyDriver;
/// # impl MyDriver { const fn new() -> Self { Self } }
/// # impl SensorDriver for MyDriver {
/// #     fn init(&mut self, _dev: *const zephyr::raw::device, _ready: bool) -> SensorResult<()> {
/// #         Ok(())
/// #     }
/// #     fn attr_set(&mut self, _dev: *const zephyr::raw::device, _ch: SensorChannel, _attr: i32, _val: &SensorValue) -> SensorResult<()> {
/// #         Ok(())
/// #     }
/// #     fn attr_get(&self, _dev: *const zephyr::raw::device, _ch: SensorChannel, _attr: i32) -> SensorResult<SensorValue> {
/// #         Ok(SensorValue::default())
/// #     }
/// #     fn trigger_set(&mut self, _dev: *const zephyr::raw::device, _trig: *const zephyr::raw::sensor_trigger, _handler: zephyr::raw::sensor_trigger_handler_t) -> SensorResult<()> {
/// #         Ok(())
/// #     }
/// #     fn sample_fetch(&mut self, _dev: *const zephyr::raw::device, _ch: SensorChannel) -> SensorResult<()> {
/// #         Ok(())
/// #     }
/// #     fn sample_fetch_chan(&mut self, _dev: *const zephyr::raw::device, _ch: SensorChannel) -> SensorResult<()> {
/// #         Ok(())
/// #     }
/// #     fn channel_get(&self, _ch: SensorChannel) -> SensorResult<SensorValue> {
/// #         Ok(SensorValue::default())
/// #     }
/// # }
/// zephyr::sensor_ffi_exports!(
///     driver: MyDriver,
///     prefix: my_sensor_rs
/// );
/// ```
///
/// # Generated C Symbols
///
/// This generates the following C-callable symbols:
/// - `<prefix>_driver_api` - The sensor API vtable (static)
/// - `<prefix>_init` - Init function
/// - `__<prefix>_attr_set` - Attribute set wrapper
/// - `__<prefix>_attr_get` - Attribute get wrapper
/// - `__<prefix>_trigger_set` - Trigger set wrapper
/// - `__<prefix>_sample_fetch` - Sample fetch wrapper
/// - `__<prefix>_channel_get` - Channel get wrapper
/// - `__<prefix>_get_decoder` - Not implemented
/// - `__<prefix>_submit` - Not implemented
///
/// # Safety
///
/// The generated code properly handles the FFI boundary and converts between
/// Rust and C types. The driver instance is stored in a static mutable variable
/// and must be initialized with a `const fn new()` constructor.
#[macro_export]
macro_rules! sensor_ffi_exports {
    (
        driver: $driver_type:ty,
        prefix: $prefix:ident
    ) => {
        use core::ffi::c_int;
        use $crate::sensor::{SensorChannel, SensorDriver, SensorValue};

        paste::paste! {
            // Global driver instance
            static mut DRIVER_INSTANCE: $driver_type = <$driver_type>::new();

            /// Driver API vtable
            #[unsafe(no_mangle)]
            pub static [<$prefix _driver_api>]: $crate::raw::sensor_driver_api =
                $crate::raw::sensor_driver_api {
                    attr_set: Some([<__ $prefix _attr_set>]),
                    attr_get: Some([<__ $prefix _attr_get>]),
                    trigger_set: Some([<__ $prefix _trigger_set>]),
                    sample_fetch: Some([<__ $prefix _sample_fetch>]),
                    channel_get: Some([<__ $prefix _channel_get>]),
                    get_decoder: None,
                    submit: None,
                };

            /// Init function
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn [<$prefix _init>](
                dev: *const $crate::raw::device
            ) -> c_int {
                let device_ready = !dev.is_null() && $crate::raw::device_is_ready(dev);

                match DRIVER_INSTANCE.init(dev, device_ready) {
                    Ok(()) => 0,
                    Err(e) => e.to_errno(),
                }
            }

            /// Attr set FFI wrapper
            #[unsafe(no_mangle)]
            unsafe extern "C" fn [<__ $prefix _attr_set>](
                dev: *const $crate::raw::device,
                chan: u32,
                attr: u32,
                val: *const $crate::raw::sensor_value,
            ) -> c_int {
                let value = SensorValue::new((*val).val1, (*val).val2);
                match DRIVER_INSTANCE.attr_set(
                    dev,
                    SensorChannel::from(chan as i32),
                    attr as i32,
                    &value,
                ) {
                    Ok(()) => 0,
                    Err(e) => e.to_errno(),
                }
            }

            /// Attr get FFI wrapper
            #[unsafe(no_mangle)]
            unsafe extern "C" fn [<__ $prefix _attr_get>](
                dev: *const $crate::raw::device,
                chan: u32,
                attr: u32,
                val: *mut $crate::raw::sensor_value,
            ) -> c_int {
                match DRIVER_INSTANCE.attr_get(
                    dev,
                    SensorChannel::from(chan as i32),
                    attr as i32,
                ) {
                    Ok(result) => {
                        (*val).val1 = result.val1;
                        (*val).val2 = result.val2;
                        0
                    }
                    Err(e) => e.to_errno(),
                }
            }

            /// Trigger set FFI wrapper
            #[unsafe(no_mangle)]
            unsafe extern "C" fn [<__ $prefix _trigger_set>](
                dev: *const $crate::raw::device,
                trig: *const $crate::raw::sensor_trigger,
                handler: $crate::raw::sensor_trigger_handler_t,
            ) -> c_int {
                match DRIVER_INSTANCE.trigger_set(dev, trig, handler) {
                    Ok(()) => 0,
                    Err(e) => e.to_errno(),
                }
            }

            /// Sample fetch FFI wrapper
            #[unsafe(no_mangle)]
            unsafe extern "C" fn [<__ $prefix _sample_fetch>](
                dev: *const $crate::raw::device,
                chan: u32,
            ) -> c_int {
                match DRIVER_INSTANCE.sample_fetch(dev, SensorChannel::from(chan as i32)) {
                    Ok(()) => 0,
                    Err(e) => e.to_errno(),
                }
            }

            /// Channel get FFI wrapper
            #[unsafe(no_mangle)]
            unsafe extern "C" fn [<__ $prefix _channel_get>](
                dev: *const $crate::raw::device,
                chan: u32,
                val: *mut $crate::raw::sensor_value,
            ) -> c_int {
                match DRIVER_INSTANCE.channel_get(SensorChannel::from(chan as i32)) {
                    Ok(result) => {
                        (*val).val1 = result.val1;
                        (*val).val2 = result.val2;
                        0
                    }
                    Err(e) => e.to_errno(),
                }
            }
        }
    };
}

pub use sensor_ffi_exports;
